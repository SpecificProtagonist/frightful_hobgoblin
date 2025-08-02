pub mod building_plan;
pub mod construction;
pub mod desire_lines;
pub mod infinite_sim;
pub mod logistics;
pub mod lumberjack;
pub mod quarry;
pub mod roads;
mod social;
mod storage_pile;
mod villager;

use std::collections::VecDeque;
use std::sync::OnceLock;

use crate::desire_lines::{desire_lines_sys, DesireLines};
use crate::goods::*;
use crate::lang::Lang;
use crate::lumberjack::{plan_lumberjack_sys, test_build_lumberjack_sys};
use crate::market::{init_stalls_sys, plan_stalls_sys, upgrade_plaza_sys};
use crate::names::make_town_name;
use crate::optimize::optimize;
use crate::pathfind::reachability_2d_from;
use crate::quarry::{plan_quarry_sys, test_build_quarry_sys};
use crate::roads::init_roads_sys;
use crate::sim::social::{Arrival, ArrivalKind};
use crate::sim::storage_pile::update_pile_visuals;
use crate::trees::{grow_trees_sys, init_trees_sys, spawn_trees_sys};
use crate::*;
use crate::{pathfind::pathfind, replay::*};
use bevy_ecs::schedule::ExecutorKind;
use bevy_ecs::system::RunSystemOnce;
use building_plan::*;
use construction::*;
use detect_existing_buildings::detect_existing_buildings_sys;
use logistics::*;
use lumberjack::LumberjackShack;
use num_traits::FromPrimitive;
use storage_pile::{LumberPile, StonePile};
pub use villager::*;

use bevy_derive::{Deref, DerefMut};
pub use bevy_ecs::prelude::*;
use bevy_math::Vec2Swizzles;

pub fn sim(mut level: Level, config: Config) {
    if config.show_level_borders {
        for column in level.area().border() {
            let z = level.height[column];
            level(column.extend(z), Wool(White));
        }
    }

    let city_center = choose_starting_area(&level);
    let mut replay = Replay::new(&level);
    replay.say(
        &format!("{}: Founding of {}", rand(1400..1550), make_town_name()),
        Yellow,
    );

    let mut world = World::new();
    world.insert_resource(config);
    world.init_resource::<CurrentTick>();

    let city_center_pos = level.ground(city_center.center());
    CENTER_BIOME.get_or_init(|| level.biome[city_center.center()]);
    (level.blocked)(city_center, Street);
    world.spawn((Pos(city_center_pos.as_vec3()), CityCenter(city_center)));
    level.reachability = reachability_2d_from(&level, city_center.center());

    world.init_resource::<Lang>();

    if world.resource::<Config>().show_reachability {
        for column in level.area() {
            let block: Block =
                Wool(Color::from_u32((level.reachability[column] / 100).min(15)).unwrap());
            let pos = level.ground(column);
            level(pos, block);
        }
    }

    replay.command(format!(
        "tp @p {} {} {}",
        city_center_pos.x,
        city_center_pos.z + 30,
        city_center_pos.y
    ));

    world.insert_resource(replay);
    world.insert_resource(level);

    world.init_resource::<DesireLines>();

    world
        .run_system_once(detect_existing_buildings_sys)
        .unwrap();
    world.run_system_once(init_trees_sys).unwrap();
    world
        .run_system_once::<_, (), _>(starting_resources_sys)
        .unwrap();
    world.run_system_once::<_, (), _>(init_stalls_sys).unwrap();
    world.run_system_once::<_, (), _>(init_roads_sys).unwrap();

    let mut sched = Schedule::default();
    // Because the systems are extremely lightweight, running them on a single thread
    // is much faster
    sched.set_executor_kind(ExecutorKind::SingleThreaded);
    sched.add_systems(
        (
            spawn_villagers_sys,
            (grow_trees_sys, spawn_trees_sys),
            (
                assign_work_sys,
                place_sys,
                walk_sys,
                build_sys,
                pickup_sys,
                deliver_sys,
                // check_construction_site_readiness_sys,
                update_piles_sys,
            ),
            (
                lumberjack::assign_worker_sys,
                lumberjack::make_lumber_piles_sys,
                lumberjack::work_sys,
                lumberjack::chop_sys,
            ),
            (
                quarry::assign_worker_sys,
                quarry::make_stone_pile_sys,
                quarry::work_sys,
                quarry::quarry_rotation_sys,
                quarry::update_quarry_rotation_sys,
            ),
            (
                plan_house_sys,
                plan_lumberjack_sys,
                plan_quarry_sys,
                plan_stalls_sys,
            ),
            assign_builds_sys,
            (
                test_build_house_sys,
                test_build_lumberjack_sys,
                test_build_quarry_sys,
                upgrade_plaza_sys,
                hitching_post_sys,
            ),
            new_construction_site_sys,
            desire_lines_sys,
            tick_replay_sys,
            |mut tick: ResMut<CurrentTick>| tick.0 += 1,
            World::clear_trackers,
        )
            .chain(),
    );
    world.add_observer(update_pile_visuals);

    for _ in 0..world.resource::<Config>().ticks {
        sched.run(&mut world);
        world.increment_change_tick();
    }
    world.resource_mut::<Replay>().say("Replay complete", Gray);
    world
        .resource_mut::<Replay>()
        .command("scoreboard players set sim speed 1".into());
    world.run_system_once(flush_unfinished_changes).unwrap();
    infinite_sim::generate(&mut world);

    let level = world.remove_resource::<Level>().unwrap();

    world.resource::<Lang>().write_blurbs(&level.path);

    if world.resource::<Config>().no_replay {
        level.debug_save();
    } else {
        let replay = world.remove_resource::<Replay>().unwrap();
        rayon::spawn(move || level.save_metadata());
        replay.finish();
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct CurrentTick(pub i32);

#[derive(Component, Deref)]
pub struct CityCenter(Rect);

/// For convenience
static CENTER_BIOME: OnceLock<Biome> = OnceLock::new();
pub fn center_biome() -> Biome {
    *CENTER_BIOME.get().unwrap()
}

#[derive(Component, Deref, DerefMut, PartialEq, Copy, Clone)]
#[require(PrevPos)]
pub struct Pos(pub Vec3);

impl std::fmt::Display for Pos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.0.x, self.0.y, self.0.z)
    }
}

#[derive(Component, Default, Deref, DerefMut)]
pub struct PrevPos(pub Vec3);

fn starting_resources_sys(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut untree: Untree,
    city_center: Query<(Entity, &Pos), With<CityCenter>>,
) -> Result<()> {
    let (center, pos) = city_center.single()?;
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
    Ok(())
}

fn spawn_villagers_sys(
    mut commands: Commands,
    level: Res<Level>,
    tick: Res<CurrentTick>,
    city_center: Query<&Pos, With<CityCenter>>,
    config: Res<Config>,
) -> Result<()> {
    if (tick.0 < config.villagers * 4) & (tick.0 % 4 == 0) {
        let column = city_center.single()?.truncate() + vec2(rand(-5. ..5.), rand(-5. ..5.));
        commands.spawn((
            Id::default(),
            Villager::default(),
            Jobless,
            Pos(level.ground(column.block()).as_vec3() + Vec3::Z),
            Arrival {
                tick: tick.0,
                kind: ArrivalKind::Migration,
            },
        ));
    }
    Ok(())
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
