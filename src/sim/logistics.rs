use super::*;
use crate::{
    goods::{Good, Pile},
    *,
};

use bevy_ecs::prelude::*;

#[derive(Component, Debug)]
pub struct MoveTask {
    pub goal: IVec3,
    pub distance: i32,
}

impl MoveTask {
    pub fn new(goal: IVec3) -> MoveTask {
        Self { goal, distance: 0 }
    }
}

/// Path to move along, in reverse order
#[derive(Component)]
pub struct MovePath(VecDeque<IVec3>);

// Assumes reservations have already been made
#[derive(Component)]
pub struct PickupTask {
    pub from: Entity,
    pub stack: Stack,
    pub max_stack: f32,
}

#[derive(Component)]
pub struct DeliverTask {
    pub to: Entity,
}

// TODO: Storage piles, which don't request resources but allow storing resources that need to be relocated, e.g. lumber piles, piles from deconstruction or from relocating other storage piles.
/// Pile that actively requests goods.
#[derive(Component, Default, Debug)]
pub struct InPile {
    /// Goods requested that are not covered by current stock or incomming orders
    pub requested: Goods,
    /// Gets reset after delivery of priority good
    pub priority: Option<Good>,
}

/// Pile that makes goods available.
#[derive(Component, Default, Debug)]
pub struct OutPile {
    /// Goods available, not including goods present but promised for another delivery
    pub available: Goods,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct PickupReady;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct DeliverReady;

pub fn pickup(
    mut commands: Commands,
    pos: Query<&Pos>,
    mut out_piles: Query<(&mut Pile, &mut OutPile)>,
    mut pickup: Query<
        (Entity, &mut Villager, &mut PickupTask, Has<PickupReady>),
        Without<MoveTask>,
    >,
) {
    for (entity, mut villager, mut task, pickup_ready) in &mut pickup {
        if !pickup_ready {
            commands.entity(entity).insert((
                MoveTask {
                    goal: pos.get(task.from).unwrap().block(),
                    distance: out_piles.get(task.from).unwrap().0.interact_distance,
                },
                PickupReady,
            ));
        } else if villager.carry.is_none() {
            let (mut pile, mut out_pile) = out_piles.get_mut(task.from).unwrap();
            // If more goods have been deposited since the task was set, take them too
            let missing = task.max_stack - task.stack.amount;
            let extra = out_pile.available.remove_up_to(Stack {
                kind: task.stack.kind,
                amount: missing,
            });
            task.stack.amount += extra.amount;
            pile.remove(task.stack);
            villager.carry = Some(task.stack);
            commands
                .entity(entity)
                .remove::<(PickupTask, PickupReady)>();
        }
    }
}

pub fn deliver(
    mut commands: Commands,
    pos: Query<&Pos>,
    mut piles: Query<(&mut Pile, Option<&mut InPile>, Option<&mut OutPile>)>,
    mut deliver: Query<
        (Entity, &mut Villager, &DeliverTask, Has<DeliverReady>),
        (Without<MoveTask>, Without<PickupTask>),
    >,
) {
    for (entity, mut villager, task, deliver_ready) in &mut deliver {
        let Some(stack) = villager.carry else {
            commands.entity(entity).remove::<DeliverTask>();
            return;
        };
        if !deliver_ready {
            commands.entity(entity).insert((
                MoveTask {
                    goal: pos.get(task.to).unwrap().block(),
                    distance: piles.get(task.to).unwrap().0.interact_distance,
                },
                DeliverReady,
            ));
        } else {
            let (mut pile, in_pile, out_pile) = piles.get_mut(task.to).unwrap();
            pile.add(stack);

            if let Some(mut in_pile) = in_pile {
                if in_pile.priority == Some(stack.kind) {
                    in_pile.priority = None
                }
            }
            if let Some(mut out_pile) = out_pile {
                out_pile.available.add(stack)
            };
            villager.carry = None;
            commands
                .entity(entity)
                .remove::<(DeliverTask, DeliverReady)>();
        }
    }
}

// TODO: Smooth this out
pub fn walk(
    mut commands: Commands,
    level: Res<Level>,
    mut query: Query<(Entity, &mut Pos, &MoveTask, Option<&mut MovePath>), With<Villager>>,
) {
    for (entity, mut pos, goal, path) in &mut query {
        if let Some(mut path) = path {
            const BLOCKS_PER_TICK: f32 = 0.16;
            let mut next_node = *path.0.front().unwrap();
            let diff = (next_node.as_vec3() - pos.0).truncate();
            if diff.length() < BLOCKS_PER_TICK {
                path.0.pop_front();
                if let Some(&next) = path.0.front() {
                    next_node = next;
                } else {
                    commands.entity(entity).remove::<(MoveTask, MovePath)>();
                }
            }
            let diff = (next_node.as_vec3() - pos.0).truncate();
            pos.0 += (diff.normalize_or_zero() * BLOCKS_PER_TICK).extend(0.);
            set_walk_height(&level, &mut pos);
        } else {
            let path = pathfind(&level, pos.block(), goal.goal, goal.distance);
            commands.entity(entity).insert(MovePath(path));
        }
    }
}

fn set_walk_height(level: &Level, pos: &mut Vec3) {
    let size = 0.35;
    let mut height = 0f32;
    for off in [vec2(1., 1.), vec2(-1., 1.), vec2(1., -1.), vec2(-1., -1.)] {
        let mut block_pos = (*pos + off.extend(0.) * size).block();
        if !level(block_pos - IVec3::Z).solid() {
            block_pos.z -= 1
        }
        if level(block_pos).solid() {
            block_pos.z += 1
        }
        height = height.max(
            block_pos.z as f32
                - match level(block_pos - ivec3(0, 0, 1)) {
                    Slab(_, Bottom) => 0.5,
                    // TODO: Stairs
                    _ => 0.,
                },
        );
    }
    pos.z = height;
}
