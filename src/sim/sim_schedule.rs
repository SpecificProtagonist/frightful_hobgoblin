use bevy_ecs::schedule::ExecutorKind;

use crate::{pathfind::reachability_2d_from, remove_foliage::find_trees, sim::desire_lines::*};

use super::*;

pub fn sim(mut level: Level) {
    let mut replay = Replay::new(&level);

    let mut world = World::new();
    world.init_resource::<Tick>();

    let city_center = choose_starting_area(&level);
    let city_center_pos = level.ground(city_center.center());
    println!("center: {city_center_pos:?}");

    let starting_resources = {
        let mut stock = Goods::default();
        stock.add(Stack::new(Good::Stone, 99999999.));
        stock.add(Stack::new(Good::Wood, 99999999.));
        stock.add(Stack::new(Good::Soil, 99999999.));
        stock
    };
    level.set_blocked(city_center);
    world.spawn((
        Pos(city_center_pos.as_vec3()),
        CityCenter(city_center),
        OutPile {
            available: starting_resources.clone(),
        },
        Pile::new(starting_resources),
    ));

    level.reachability = reachability_2d_from(&level, city_center.center());

    world.insert_resource(DesireLines::new(&level));

    // Find trees
    for (pos, species) in find_trees(&level, level.area()) {
        world.spawn((Pos(pos.as_vec3()), Tree::new(species)));
    }

    let mut sched = Schedule::default();
    sched.set_executor_kind(ExecutorKind::SingleThreaded);
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
            (
                lumberjack::assign_worker,
                lumberjack::make_lumber_piles,
                lumberjack::update_lumber_pile_visuals,
            ),
            (
                quarry::assign_worker,
                quarry::make_stone_piles,
                quarry::update_stone_pile_visuals,
            ),
            (plan_house, plan_lumberjack, plan_quarry),
            assign_builds,
            new_construction_site,
            (
                test_build_house,
                test_build_lumberjack,
                test_build_quarry,
                upgrade_plaza,
            ),
            desire_lines,
            personal_name::name,
            tick_replay,
            // remove_outdated,
            |mut tick: ResMut<Tick>| tick.0 += 1,
            |world: &mut World| world.clear_trackers(),
        )
            .chain(),
    );

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

        if tick < 40 {
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
    // level.debug_save();
    let replay = world.remove_resource::<Replay>().unwrap();
    rayon::spawn(move || level.save_metadata().unwrap());
    replay.finish();
}
