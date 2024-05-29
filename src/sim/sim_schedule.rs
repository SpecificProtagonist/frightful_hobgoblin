use bevy_ecs::{schedule::ExecutorKind, system::RunSystemOnce};

use crate::{pathfind::reachability_2d_from, sim::desire_lines::*};

use detect_existing_buildings::detect_existing_buildings;
use lumberjack::{plan_lumberjack, test_build_lumberjack};
use make_name::make_town_name;
use market::{init_stalls, plan_stalls};
use quarry::{plan_quarry, test_build_quarry};
use roads::init_roads;
use storage_pile::{update_lumber_pile_visuals, update_stone_pile_visuals};
use trees::{init_trees, spawn_trees};

use self::market::upgrade_plaza;

use super::*;

pub fn sim(mut level: Level, debug_save: bool) {
    let mut replay = Replay::new(&level);
    replay.say(
        &format!("{}: Founding of {}", rand(1400..1550), make_town_name()),
        Yellow,
    );

    let mut world = World::new();
    world.init_resource::<Tick>();

    let city_center = choose_starting_area(&level);
    let city_center_pos = level.ground(city_center.center());
    CENTER_BIOME.get_or_init(|| level.biome[city_center.center()]);
    (level.blocked)(city_center, Street);
    world.spawn((Pos(city_center_pos.as_vec3()), CityCenter(city_center)));
    level.reachability = reachability_2d_from(&level, city_center.center());
    replay.command(format!(
        "tp @p {} {} {}",
        city_center_pos.x,
        city_center_pos.z + 30,
        city_center_pos.y
    ));

    world.insert_resource(replay);
    world.insert_resource(level);

    world.init_resource::<DesireLines>();

    world.run_system_once(detect_existing_buildings);
    world.run_system_once(init_trees);
    world.run_system_once(starting_resources);
    world.run_system_once(init_stalls);
    world.run_system_once(init_roads);

    let mut sched = Schedule::default();
    // Because the systems are extremely lightweight, running them on a single thread
    // is much faster
    sched.set_executor_kind(ExecutorKind::SingleThreaded);
    sched.add_systems(
        (
            spawn_villagers,
            (grow_trees, spawn_trees),
            (
                assign_work,
                place,
                walk,
                build,
                pickup,
                deliver,
                check_construction_site_readiness,
            ),
            (
                lumberjack::assign_worker,
                lumberjack::make_lumber_piles,
                update_lumber_pile_visuals,
                lumberjack::work,
                lumberjack::chop,
            ),
            (
                quarry::assign_worker,
                quarry::make_stone_piles,
                update_stone_pile_visuals,
                quarry::work,
                quarry::quarry_rotation,
                quarry::update_quarry_rotation,
            ),
            (plan_house, plan_lumberjack, plan_quarry, plan_stalls),
            assign_builds,
            (
                test_build_house,
                test_build_lumberjack,
                test_build_quarry,
                upgrade_plaza,
                hitching_post,
            ),
            new_construction_site,
            desire_lines,
            personal_name::name,
            tick_replay,
            |mut tick: ResMut<Tick>| tick.0 += 1,
            World::clear_trackers,
        )
            .chain(),
    );

    for _ in 0..30000 {
        sched.run(&mut world);
    }
    world.resource_mut::<Replay>().say("Replay complete", Gray);
    world
        .resource_mut::<Replay>()
        .command("scoreboard players set sim speed 1".into());
    world.run_system_once(flush_unfinished_changes);
    steady_state::generate(&mut world);

    let level = world.remove_resource::<Level>().unwrap();

    // show_blocked(&mut level);

    if debug_save {
        level.debug_save();
        return;
    }

    let replay = world.remove_resource::<Replay>().unwrap();
    rayon::spawn(move || level.save_metadata());
    replay.finish();
}
