use super::*;
use crate::{
    goods::{Good, Stockpile},
    *,
};

use bevy_ecs::prelude::*;

#[derive(Component, Debug)]
pub struct MoveTask {
    pub goal: IVec3,
    pub distance: i32,
}

/// Path to move along, in reverse order
#[derive(Component)]
pub struct MovePath(VecDeque<IVec3>);

impl MoveTask {
    pub fn new(goal: IVec3) -> MoveTask {
        Self { goal, distance: 0 }
    }
}

// Assumes reservations have already been made
#[derive(Component)]
pub struct CarryTask {
    pub from: Entity,
    pub to: Entity,
    pub stack: Stack,
    pub max_stack: f32,
}

#[derive(Component, Default, Debug)]
pub struct InPile {
    pub stock: Stockpile,
    pub requested: Stockpile,
    // Gets reset after delivery of priority good
    pub priority: Option<Good>,
}

#[derive(Component, Default, Debug)]
pub struct OutPile {
    // TODO: When adding piles that visualize what is available,
    // this also needs a `current` field
    pub available: Stockpile,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct PickupReady;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct DeliverReady;

pub fn carry(
    mut commands: Commands,
    pos: Query<&Pos>,
    mut out_piles: Query<&mut OutPile>,
    mut in_piles: Query<&mut InPile>,
    mut workers: Query<
        (
            Entity,
            &mut Villager,
            &mut CarryTask,
            Has<PickupReady>,
            Has<DeliverReady>,
        ),
        Without<MoveTask>,
    >,
) {
    for (entity, mut villager, mut task, pickup_ready, deliver_ready) in &mut workers {
        if !pickup_ready {
            commands.entity(entity).insert((
                MoveTask::new(pos.get(task.from).unwrap().block()),
                PickupReady,
            ));
        } else if villager.carry.is_none() {
            let mut out = out_piles.get_mut(task.from).unwrap();
            // If more goods have been deposited since the task was set, take them too
            let missing = task.max_stack - task.stack.amount;
            let extra = out.available.remove_up_to(Stack {
                kind: task.stack.kind,
                amount: missing,
            });
            task.stack.amount += extra.amount;
            villager.carry = Some(task.stack);
        } else if !deliver_ready {
            commands.entity(entity).insert((
                MoveTask::new(pos.get(task.to).unwrap().block()),
                DeliverReady,
            ));
        } else {
            let mut pile = in_piles.get_mut(task.to).unwrap();
            pile.stock.add(task.stack);
            if pile.priority == Some(task.stack.kind) {
                pile.priority = None
            }
            villager.carry = None;
            commands
                .entity(entity)
                .remove::<(CarryTask, PickupReady, DeliverReady)>();
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
            let diff = next_node.as_vec3() - pos.0; //.truncate();
            if diff.length() < BLOCKS_PER_TICK {
                path.0.pop_front();
                if let Some(&next) = path.0.front() {
                    next_node = next;
                } else {
                    commands.entity(entity).remove::<(MoveTask, MovePath)>();
                }
            }
            let diff = next_node.as_vec3() - pos.0; //.truncate();
            pos.0 += diff.normalize_or_zero() * BLOCKS_PER_TICK;
            //.extend(0.);
            // set_walk_height(&level, &mut pos);
        } else {
            let path = pathfind(&level, pos.block(), goal.goal, goal.distance);
            commands.entity(entity).insert(MovePath(path));
        }
    }
}

// fn set_walk_height(level: &Level, pos: &mut Vec3) {
//     let size = 0.35;
//     let mut height = 0f32;
//     for off in [vec2(1., 1.), vec2(-1., 1.), vec2(1., -1.), vec2(-1., -1.)] {
//         let mut block_pos = (*pos + off.extend(0.) * size).block();
//         if !level[block_pos].solid() {
//             block_pos.z -= 1
//         }
//         if level[block_pos].solid() {
//             block_pos.z += 1
//         }
//         height = height.max(
//             block_pos.z as f32
//                 - match level[block_pos - ivec3(0, 0, 1)] {
//                     Slab(_, Bottom) => 0.5,
//                     // In theory also do stairs here
//                     _ => 0.,
//                 },
//         );
//     }
//     pos.z = height;
// }
