pub mod building_plan;
pub mod construction;
pub mod desire_lines;
pub mod logistics;
pub mod lumberjack;
mod personal_name;
pub mod quarry;
pub mod roads;
mod sim_schedule;
pub mod steady_state;
mod storage_pile;

use std::collections::VecDeque;
use std::sync::OnceLock;

use crate::goods::*;
use crate::optimize::optimize;
use crate::trees::grow_trees;
use crate::*;
use crate::{pathfind::pathfind, replay::*};
use building_plan::*;
use construction::*;
use logistics::*;
use lumberjack::LumberjackShack;
pub use sim_schedule::sim;
use storage_pile::{LumberPile, StonePile};

use bevy_derive::{Deref, DerefMut};
pub use bevy_ecs::prelude::*;
use bevy_math::Vec2Swizzles;

#[derive(Resource, Default, Deref, DerefMut)]
pub struct Tick(pub i32);

#[derive(Component, Deref)]
pub struct CityCenter(Rect);

/// For convenience
static CENTER_BIOME: OnceLock<Biome> = OnceLock::new();
pub fn center_biome() -> Biome {
    *CENTER_BIOME.get().unwrap()
}

#[derive(Component, Deref, DerefMut, PartialEq)]
pub struct Pos(pub Vec3);

impl std::fmt::Display for Pos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.0.x, self.0.y, self.0.z)
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct PrevPos(pub Vec3);

#[derive(Component, Default)]
pub struct Villager {
    pub carry: Option<Stack>,
    pub carry_id: Id,
}

#[derive(Component)]
pub struct Name(pub String);

#[derive(Component, Default)]
pub struct InBoat(pub Id);

/// Doesn't have a specialized job, but can carry and build.
#[derive(Component)]
pub struct Jobless;

pub type ConsList = VecDeque<ConsItem>;

#[derive(Clone, Debug)]
pub enum ConsItem {
    Set(SetBlock),
    Goto(MoveTask),
    Carry(Option<Stack>),
    Command(String),
}

#[derive(Component, Deref, DerefMut)]
pub struct PlaceTask(ConsList);

fn assign_work(
    mut commands: Commands,
    idle: Query<
        (Entity, &Pos),
        (
            With<Villager>,
            With<Jobless>,
            Without<DeliverTask>,
            Without<BuildTask>,
        ),
    >,
    mut out_piles: Query<(Entity, &Pos, &mut OutPile, &mut Pile)>,
    mut in_piles: Query<(Entity, &Pos, &mut InPile)>,
    mut construction_sites: Query<(Entity, &Pos, &mut ConstructionSite, &Pile), Without<OutPile>>,
) {
    for (vill, vil_pos) in &idle {
        // Construction
        if let Some((building, pos, mut site, _)) = construction_sites
            .iter_mut()
            .filter(|(_, site_pos, site, pile)| {
                site.has_materials(pile, min_walk_ticks(vil_pos.0, site_pos.0)) & !site.has_builder
            })
            .min_by_key(|(_, pos, _, _)| pos.distance_squared(vil_pos.0) as u32)
        {
            site.has_builder = true;
            commands
                .entity(vill)
                .insert((MoveTask::new(pos.0.block()), BuildTask { building }));
            continue;
        }

        // Transport
        if let Some((_, task)) = out_piles
            .iter_mut()
            .filter_map(|(out_entity, out_pos, out_pile, pile)| {
                let min_ticks = min_walk_ticks(vil_pos.0, out_pos.0);
                let mut best_score = f32::INFINITY;
                let mut task = None;
                for good in pile.goods.keys() {
                    let amount = pile.available(*good, min_ticks)
                        - out_pile.reserved.get(good).copied().unwrap_or(0.);
                    if amount <= 0. {
                        continue;
                    }
                    for (in_entity, in_pos, in_pile) in &mut in_piles {
                        if let Some(&requested) = in_pile.requested.get(good)
                            && requested > 0.
                        {
                            if let Some(priority) = in_pile.priority {
                                if priority != *good {
                                    continue;
                                }
                            }
                            let mut score = out_pos.distance_squared(in_pos.0);
                            // Try to reduce the amount of trips
                            if amount < requested {
                                score *= 2.;
                            }
                            if score < best_score {
                                best_score = score;
                                task = Some((
                                    PickupTask {
                                        from: out_entity,
                                        stack: Stack::new(
                                            *good,
                                            amount.min(requested).min(CARRY_CAPACITY),
                                        ),
                                        max_stack: requested.min(CARRY_CAPACITY),
                                    },
                                    DeliverTask { to: in_entity },
                                ));
                            }
                        }
                    }
                }
                task.map(|task| (vil_pos.distance_squared(out_pos.0), task))
            })
            // TODO: Also influence via best_score?
            .min_by_key(|(d, _)| *d as i32)
        {
            *out_piles
                .get_mut(task.0.from)
                .unwrap()
                .2
                .reserved
                .entry(task.0.stack.good)
                .or_insert(0.) += task.0.stack.amount;
            in_piles
                .get_mut(task.1.to)
                .unwrap()
                .2
                .requested
                .remove(task.0.stack);
            commands.entity(vill).insert(task);
        }
    }
}

fn place(
    mut commands: Commands,
    mut replay: ResMut<Replay>,
    mut builders: Query<(Entity, &mut Villager, &mut PlaceTask), Without<MoveTask>>,
) {
    for (entity, mut villager, mut build) in &mut builders {
        match build.0.pop_front() {
            Some(ConsItem::Set(set)) => {
                replay.block(set.pos, set.block, set.nbt);
            }
            Some(ConsItem::Goto(goto)) => {
                commands.entity(entity).insert(goto);
            }
            Some(ConsItem::Carry(stack)) => {
                villager.carry = stack;
            }
            Some(ConsItem::Command(cmd)) => replay.command(cmd),
            None => {
                commands.entity(entity).remove::<PlaceTask>();
            }
        }
    }
}

fn starting_resources(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut untree: Untree,
    city_center: Query<(Entity, &Pos), With<CityCenter>>,
) {
    let (center, pos) = city_center.single();
    for _ in 0..6 {
        let (pos, area, params) =
            LumberPile::make(&mut level, &mut untree, pos.truncate(), pos.truncate());

        let goods = {
            let mut stock = Goods::default();
            stock.add(Stack::new(Good::Wood, 200.));
            stock
        };
        commands.spawn((
            Pos(pos.as_vec3()),
            params,
            OutPile::default(),
            Pile {
                goods,
                interact_distance: params.width,
                despawn_when_empty: Some(area),
                future_deltas: default(),
            },
        ));
    }
    for _ in 0..6 {
        let (pos, area, params) = StonePile::make(&mut level, &mut untree, pos.truncate());

        let goods = {
            let mut stock = Goods::default();
            stock.add(Stack::new(Good::Stone, 140.));
            stock
        };
        commands.spawn((
            Pos(pos),
            params,
            OutPile::default(),
            Pile {
                goods,
                interact_distance: 2,
                despawn_when_empty: Some(area),
                future_deltas: default(),
            },
        ));
    }
    // Temporary, for testing
    let starting_resources = {
        let mut stock = Goods::default();
        stock.add(Stack::new(Good::Soil, 99999999.));
        // stock.add(Stack::new(Good::Wood, 99999999.));
        // stock.add(Stack::new(Good::Stone, 99999999.));
        stock
    };
    commands
        .entity(center)
        .insert((OutPile::default(), Pile::new(starting_resources, 1)));
}

fn spawn_villagers(
    mut commands: Commands,
    level: Res<Level>,
    tick: Res<Tick>,
    city_center: Query<&Pos, With<CityCenter>>,
    config: Res<Config>,
) {
    if (tick.0 < config.villagers * 4) & (tick.0 % 4 == 0) {
        let column = city_center.single().truncate() + vec2(rand(-5. ..5.), rand(-5. ..5.));
        commands.spawn((
            Id::default(),
            Villager::default(),
            Jobless,
            Pos(level.ground(column.block()).as_vec3() + Vec3::Z),
            PrevPos(default()),
        ));
    }
}

fn flush_unfinished_changes(
    mut replay: ResMut<Replay>,
    cs: Query<&ConstructionSite>,
    place_tasks: Query<&PlaceTask>,
) {
    for site in &cs {
        for item in &site.todo {
            if let ConsItem::Set(block) = item {
                replay.block(block.pos, block.block, block.nbt.clone());
            }
        }
    }
    for task in &place_tasks {
        for item in &task.0 {
            if let ConsItem::Set(block) = item {
                replay.block(block.pos, block.block, block.nbt.clone());
            }
        }
    }
}
