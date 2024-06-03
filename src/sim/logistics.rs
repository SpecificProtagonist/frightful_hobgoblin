use super::*;
use crate::{goods::Good, pathfind::PathingNode, *};

use bevy_ecs::prelude::*;

#[derive(Component, Debug, Clone, Copy)]
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
#[derive(Component, Debug)]
pub struct MovePath {
    steps: VecDeque<PathingNode>,
    vertical: bool,
}

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

#[derive(Component, Debug, Clone, Deref, DerefMut)]
pub struct Pile {
    #[deref]
    pub goods: Goods,
    pub interact_distance: i32,
    /// If this is some, clear unblock the area and despawn this pile if it empties
    pub despawn_when_empty: Option<Rect>,
}

impl Pile {
    pub fn new(goods: Goods) -> Self {
        Self {
            goods,
            interact_distance: 1,
            despawn_when_empty: None,
        }
    }
}

impl Default for Pile {
    fn default() -> Self {
        Self {
            goods: default(),
            interact_distance: 1,
            despawn_when_empty: None,
        }
    }
}

/// Pile that doesn't request goods but can be used to store them
/// TODO: note what capacity it has
#[derive(Component, Default)]
pub struct StoragePile;

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
    mut level: ResMut<Level>,
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
                good: task.stack.good,
                amount: missing,
            });
            task.stack.amount += extra.amount;
            pile.remove(task.stack);
            if let Some(area) = pile.despawn_when_empty {
                if pile.iter().all(|(_, &amount)| amount <= 0.) {
                    (level.blocked)(area, Free);
                    commands.entity(task.from).despawn();
                }
            }
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
                if in_pile.priority == Some(stack.good) {
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
    mut replay: ResMut<Replay>,
    level: Res<Level>,
    mut query: Query<(
        Entity,
        &Id,
        &mut Pos,
        &MoveTask,
        Option<&InBoat>,
        Option<&mut MovePath>,
    )>,
) {
    for (entity, id, mut pos, goal, in_boat, path) in &mut query {
        if let Some(mut path) = path {
            const WALK_PER_TICK: f32 = 0.16;
            const BOATING_PER_TICK: f32 = 0.2;
            const CLIMB_PER_TICK: f32 = 0.09;
            let Some(mut next_node) = path.steps.front().copied() else {
                commands.entity(entity).remove::<(MoveTask, MovePath)>();
                continue;
            };
            let diff = (next_node.pos.as_vec3() - pos.0).truncate();
            if path.vertical {
                // Climbing
                if if next_node.pos.z as f32 > pos.0.z {
                    pos.0.z += CLIMB_PER_TICK;
                    pos.0.z > next_node.pos.z as f32
                } else {
                    pos.0.z -= CLIMB_PER_TICK;
                    pos.0.z < next_node.pos.z as f32
                } {
                    path.steps.pop_front();
                    if let Some(&next) = path.steps.front() {
                        path.vertical = (next.pos - next_node.pos).truncate() == IVec2::ZERO;
                    }
                }
            } else {
                let speed;
                if next_node.boat {
                    speed = BOATING_PER_TICK;
                    if in_boat.is_none() {
                        let boat_id = Id::default();
                        commands.entity(entity).insert(InBoat(boat_id));
                        let biome = level.biome[pos.block().truncate()];
                        replay.command(format!(
                            "summon boat {} {} {} {{{}, Invulnerable:1, Type:\"{}\"}}",
                            pos.x,
                            pos.z,
                            pos.y,
                            boat_id.snbt(),
                            biome.default_tree_species().to_str()
                        ));
                        replay.command(format!("ride {id} mount {boat_id}"));
                    }
                } else {
                    speed = WALK_PER_TICK;
                    if let Some(boat_id) = in_boat {
                        commands.entity(entity).remove::<InBoat>();
                        replay.command(format!("kill {}", boat_id.0));
                    }
                }
                // Not climbing, but possibly going up stairs
                if diff.length() < speed {
                    path.steps.pop_front();
                    if let Some(&next) = path.steps.front() {
                        path.vertical = (next.pos - next_node.pos).truncate() == IVec2::ZERO;
                        next_node = next;
                    }
                }
                if !path.vertical {
                    let diff = (next_node.pos.as_vec3() - pos.0).truncate();
                    pos.0 += (diff.normalize_or_zero() * speed).extend(0.);
                    if !next_node.boat {
                        set_walk_height(&level, &mut pos);
                    }
                }
            }
        } else {
            let path = pathfind(&level, pos.block(), goal.goal, goal.distance);
            commands.entity(entity).insert(MovePath {
                steps: path.path,
                vertical: false,
            });
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
