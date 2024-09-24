use std::ops::DerefMut;

use super::*;
use crate::{goods::Good, *};

use bevy_ecs::prelude::*;
use storage_pile::UpdatePileVisuals;

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

#[derive(Copy, Clone, Debug)]
pub struct Delta {
    ticks_until: i32,
    added: Stack,
}

#[derive(Component, Debug, Clone)]
pub struct Pile {
    pub goods: Goods,
    pub interact_distance: i32,
    /// If this is some, clear unblock the area and despawn this pile if it empties
    pub despawn_when_empty: Option<Rect>,
    pub future_deltas: Vec<Delta>,
}

impl Pile {
    pub fn new(goods: Goods, interact_distance: i32) -> Self {
        Self {
            goods,
            interact_distance,
            despawn_when_empty: None,
            future_deltas: default(),
        }
    }

    pub fn add(&mut self, stack: Stack) {
        self.goods.add(stack)
    }

    pub fn add_at(&mut self, added: Stack, ticks_until: i32) {
        self.future_deltas.push(Delta { ticks_until, added })
    }

    pub fn try_consume(&mut self, block: Block) -> Option<Good> {
        self.goods.try_consume(block)
    }

    pub fn space_available(&self, good: Good, limit: f32, ticks_until: i32) -> f32 {
        let mut max = self.goods.get(&good).copied().unwrap_or(0.);
        let mut at_tick = max;
        for delta in &self.future_deltas {
            if delta.added.good == good {
                at_tick += delta.added.amount;
                if delta.ticks_until <= ticks_until {
                    max += delta.added.amount
                } else if delta.added.amount > 0. {
                    max = max.max(at_tick)
                }
            }
        }
        limit - max
    }

    pub fn available(&self, good: Good, ticks_until: i32) -> f32 {
        let mut available = self.goods.get(&good).copied().unwrap_or(0.);
        let mut at_tick = available;
        for delta in &self.future_deltas {
            if delta.added.good == good {
                at_tick += delta.added.amount;
                if delta.ticks_until <= ticks_until {
                    available += delta.added.amount
                } else if delta.added.amount < 0. {
                    available = available.min(at_tick)
                }
            }
        }
        available
    }
}

impl Default for Pile {
    fn default() -> Self {
        Self {
            goods: default(),
            interact_distance: 1,
            despawn_when_empty: None,
            future_deltas: default(),
        }
    }
}

pub fn update_piles(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut piles: Query<(Entity, &mut Pile)>,
) {
    for (entity, mut pile) in &mut piles {
        let Pile {
            goods,
            despawn_when_empty,
            future_deltas,
            ..
        } = pile.deref_mut();
        future_deltas.retain_mut(|delta| {
            delta.ticks_until -= 1;
            if delta.ticks_until == 0 {
                goods.add(delta.added);
                false
            } else {
                true
            }
        });
        if let Some(area) = despawn_when_empty {
            if future_deltas.is_empty() & goods.iter().all(|(_, &amount)| amount <= 0.) {
                (level.blocked)(*area, Free);
                commands.entity(entity).despawn();
            }
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
    pub reserved: HashMap<Good, f32>,
}

// /// Goods available, not including goods present but promised for another delivery
// pub available: Goods,

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct PickupReady;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct DeliverReady;

pub fn pickup(
    mut commands: Commands,
    level: Res<Level>,
    pos: Query<&Pos>,
    mut out_piles: Query<(&mut Pile, &mut OutPile)>,
    mut pickup: Query<(Entity, &mut Villager, &PickupTask, Has<PickupReady>), Without<MoveTask>>,
) {
    for (entity, mut villager, task, pickup_ready) in &mut pickup {
        if !pickup_ready {
            let (mut pile, mut out_pile) = out_piles.get_mut(task.from).unwrap();
            let goal = pos.get(task.from).unwrap().block();
            let distance = pile.interact_distance;
            let path = MovePath::new(&level, pos.get(entity).unwrap().block(), goal, distance);
            pile.add_at(-task.stack, path.ticks() + 2);
            *out_pile.reserved.get_mut(&task.stack.good).unwrap() -= task.stack.amount;
            commands
                .entity(entity)
                .insert((path, MoveTask { goal, distance }, PickupReady));
        } else if villager.carry.is_none() {
            commands.trigger_targets(UpdatePileVisuals, task.from);
            let (mut pile, out_pile) = out_piles.get_mut(task.from).unwrap();
            // If more goods have been deposited since the task was set, take them too
            let missing = task.max_stack - task.stack.amount;
            let available = pile.available(task.stack.good, 0)
                - out_pile
                    .reserved
                    .get(&task.stack.good)
                    .copied()
                    .unwrap_or(0.);
            let extra = missing.min(available);
            pile.add(Stack::new(task.stack.good, -extra));
            villager.carry = Some(Stack::new(task.stack.good, task.stack.amount + extra));
            commands
                .entity(entity)
                .remove::<(PickupTask, PickupReady)>();
        }
    }
}

pub fn deliver(
    mut commands: Commands,
    mut replay: ResMut<Replay>,
    level: Res<Level>,
    pos: Query<&Pos>,
    mut piles: Query<(&mut Pile, Option<&mut InPile>)>,
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
            let goal = pos.get(task.to).unwrap().block();
            let distance = piles.get(task.to).unwrap().0.interact_distance;
            let path = MovePath::new(&level, pos.get(entity).unwrap().block(), goal, distance);
            let (mut pile, in_pile) = piles.get_mut(task.to).unwrap();
            pile.add_at(stack, path.ticks());
            if let Some(mut in_pile) = in_pile {
                if in_pile.priority == Some(stack.good) {
                    in_pile.priority = None
                }
            }
            commands
                .entity(entity)
                .insert((MoveTask { goal, distance }, path, DeliverReady));
        } else {
            commands.trigger_targets(UpdatePileVisuals, task.to);
            replay.command(playsound("drop", pos.get(entity).unwrap().block()));

            villager.carry = None;
            commands
                .entity(entity)
                .remove::<(DeliverTask, DeliverReady)>();
        }
    }
}

const WALK_PER_TICK: f32 = 0.16;
const BOATING_PER_TICK: f32 = 0.2;
const CLIMB_PER_TICK: f32 = 0.09;

pub fn min_walk_ticks(start: Vec3, end: Vec3) -> i32 {
    ((start - end).abs().element_sum() / WALK_PER_TICK) as i32
}

/// Path to move along
#[derive(Component, Debug)]
pub struct MovePath(VecDeque<MovePathNode>);

impl MovePath {
    fn new(level: &Level, start: IVec3, goal: IVec3, target_distance: i32) -> Self {
        let mut path = pathfind(level, start, goal, target_distance).path;
        let mut steps = VecDeque::<MovePathNode>::new();
        let mut pos = start.as_vec3();
        let mut vertical = false;
        while let Some(mut next_node) = path.front().copied() {
            let diff = (next_node.pos.as_vec3() - pos).truncate();
            if vertical {
                // Climbing
                if if next_node.pos.z as f32 > pos.z {
                    pos.z += CLIMB_PER_TICK;
                    pos.z > next_node.pos.z as f32
                } else {
                    pos.z -= CLIMB_PER_TICK;
                    pos.z < next_node.pos.z as f32
                } {
                    path.pop_front();
                    if let Some(&next) = path.front() {
                        vertical = (next.pos - next_node.pos).truncate() == IVec2::ZERO;
                    }
                }
            } else {
                let boat = next_node.boat;
                let speed = if boat {
                    BOATING_PER_TICK
                } else {
                    WALK_PER_TICK
                };
                // Not climbing, but possibly going up stairs
                if diff.length() < speed {
                    path.pop_front();
                    if let Some(&next) = path.front() {
                        vertical = (next.pos - next_node.pos).truncate() == IVec2::ZERO;
                        next_node = next;
                    }
                }
                if !vertical {
                    let diff = (next_node.pos.as_vec3() - pos).truncate();
                    pos += (diff.normalize_or_zero() * speed).extend(0.);
                    if !next_node.boat {
                        set_walk_height(level, &mut pos);
                    }
                }
                steps.push_back(MovePathNode { pos, boat });
            }
        }
        Self(steps)
    }

    fn ticks(&self) -> i32 {
        self.0.len() as i32
    }
}

#[derive(Debug)]
struct MovePathNode {
    pos: Vec3,
    boat: bool,
}

// TODO: Calculate the exact path at the beginning, then store it & use it for walk_ticks
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
    config: Res<Config>,
) {
    for (entity, id, mut pos, goal, in_boat, path) in &mut query {
        if config.skip_walk {
            pos.0 = goal.goal.as_vec3();
            commands.entity(entity).remove::<(MoveTask, MovePath)>();
            continue;
        }

        let Some(mut path) = path else {
            commands.entity(entity).insert(MovePath::new(
                &level,
                pos.block(),
                goal.goal,
                goal.distance,
            ));
            continue;
        };

        let Some(node) = path.0.pop_front() else {
            commands.entity(entity).remove::<(MoveTask, MovePath)>();
            continue;
        };

        pos.0 = node.pos;

        if node.boat {
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
        } else if let Some(boat_id) = in_boat {
            commands.entity(entity).remove::<InBoat>();
            replay.command(format!("kill {}", boat_id.0));
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
