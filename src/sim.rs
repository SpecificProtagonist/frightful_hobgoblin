#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

mod building_plan;
mod construction;
mod logistics;
mod lumberjack;

use std::collections::VecDeque;

use crate::goods::*;
use crate::make_trees::grow_trees;
use crate::optimize::optimize;
use crate::remove_foliage::remove_tree;
use crate::*;
use crate::{pathfind::pathfind, remove_foliage::find_trees, replay::*};
use building_plan::*;
use construction::*;
use logistics::*;

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_ecs::query::Has;
use bevy_math::Vec2Swizzles;

pub fn sim(mut level: Level) {
    Id::load(&level.path);

    let mut world = World::new();
    world.init_resource::<Tick>();

    let city_center = choose_starting_area(&level);
    let city_center_pos = level.ground(city_center.center());

    world.spawn((
        Pos(city_center_pos.as_vec3()),
        Blocked(city_center),
        CityCenter,
        OutPile,
        {
            let mut stock = Pile::default();
            stock.add(Stack::new(Good::Stone, 99999999.));
            stock.add(Stack::new(Good::Wood, 99999999.));
            stock.add(Stack::new(Good::Soil, 99999999.));
            stock
        },
    ));

    for pos in city_center {
        let pos = level.ground(pos);
        level[pos] = Wool(Magenta)
    }

    // Find trees
    for (pos, species) in find_trees(&level, level.area()) {
        world.spawn((Pos(pos.as_vec3()), Tree::new(species)));
    }

    let mut sched = Schedule::new();
    sched.add_systems(
        (
            grow_trees,
            assign_work,
            (
                place,
                lumberjack::work,
                lumberjack::chop,
                walk,
                build,
                pickup,
                deliver,
                check_construction_site_readiness,
            ),
            lumberjack::assign_worker,
            // plan_house,
            plan_lumberjack,
            // plan_quarry,
            apply_deferred,
            assign_builds,
            apply_deferred,
            new_construction_site,
            test_build_house,
            test_build_lumberjack,
            test_build_quarry,
            apply_deferred,
            tick_replay,
            remove_outdated,
            |mut tick: ResMut<Tick>| tick.0 += 1,
            |world: &mut World| world.clear_trackers(),
        )
            .chain(),
    );

    let mut replay = Replay::new(level.path.clone());
    replay.command(format!(
        "tp @p {} {} {}",
        city_center_pos.x,
        city_center_pos.z + 30,
        city_center_pos.y
    ));
    world.insert_resource(replay);
    world.insert_resource(level);
    for tick in 0..30000 {
        sched.run(&mut world);

        if tick < 2 {
            world.spawn((
                Id::default(),
                Villager::default(),
                Jobless,
                Pos(city_center_pos.as_vec3() + Vec3::Z),
                PrevPos(default()),
            ));
        }
    }

    let level = world.remove_resource::<Level>().unwrap();
    let replay = world.remove_resource::<Replay>().unwrap();
    Id::save(&level.path);
    rayon::spawn(move || level.save_metadata().unwrap());
    replay.finish();
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct Tick(pub i32);

#[derive(Component)]
pub struct CityCenter;

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

/// Doesn't have a specialized job, but can carry and build.
#[derive(Component)]
pub struct Jobless;

pub type PlaceList = VecDeque<SetBlock>;

#[derive(Component)]
pub struct PlaceTask(PlaceList);

#[derive(Component)]
pub struct Tree {
    _species: TreeSpecies,
    to_be_chopped: bool,
}

impl Tree {
    fn new(species: TreeSpecies) -> Self {
        Self {
            _species: species,
            to_be_chopped: false,
        }
    }
}

fn assign_work(
    mut commands: Commands,
    mut replay: ResMut<Replay>,
    idle: Query<
        (Entity, &Pos),
        (
            With<Villager>,
            With<Jobless>,
            Without<DeliverTask>,
            Without<BuildTask>,
        ),
    >,
    mut out_piles: Query<(Entity, &Pos, &mut Pile), With<OutPile>>,
    mut in_piles: Query<(Entity, &Pos, &mut InPile)>,
    mut construction_sites: Query<(Entity, &Pos, &mut ConstructionSite)>,
) {
    for (vill, vil_pos) in &idle {
        for site in &construction_sites {
            replay.dbg(&format!(
                "mat: {}, builder: {}",
                site.2.has_materials, site.2.has_builder
            ));
        }
        if let Some((building, pos, mut site)) = construction_sites
            .iter_mut()
            .filter(|(_, _, site)| site.has_materials & !site.has_builder)
            .min_by_key(|(_, pos, _)| pos.distance_squared(vil_pos.0) as u32)
        {
            replay.dbg("assign build");
            site.has_builder = true;
            commands
                .entity(vill)
                .insert((MoveTask::new(pos.0.block()), BuildTask { building }));
            continue;
        }

        if let Some((_, task)) = out_piles
            .iter_mut()
            .filter_map(|(out_entity, out_pos, out_pile)| {
                let mut best_score = f32::INFINITY;
                let mut task = None;
                for (good, &amount) in &out_pile.0 {
                    if amount == 0. {
                        continue;
                    }
                    for (in_entity, in_pos, in_pile) in &mut in_piles {
                        if let Some(&requested) = in_pile.requested.0.get(good) && requested > 0. {
                            if let Some(priority) = in_pile.priority {
                                if priority != *good {continue}
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
                                    DeliverTask{
                                        to: in_entity
                                    }
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
            out_piles
                .get_component_mut::<Pile>(task.0.from)
                .unwrap()
                .remove(task.0.stack);
            in_piles
                .get_component_mut::<InPile>(task.1.to)
                .unwrap()
                .requested
                .remove(task.0.stack);
            replay.dbg("assign carry");
            commands.entity(vill).insert(task);
        }
    }
}

fn place(
    mut commands: Commands,
    mut replay: ResMut<Replay>,
    mut builders: Query<(Entity, &mut PlaceTask), Without<MoveTask>>,
) {
    for (entity, mut build) in &mut builders {
        if let Some(set) = build.0.pop_front() {
            replay.block(set.pos, set.block);
        } else {
            commands.entity(entity).remove::<PlaceTask>();
        }
    }
}
