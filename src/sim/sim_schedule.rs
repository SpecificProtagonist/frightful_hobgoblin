use bevy_ecs::schedule::ExecutorKind;

use crate::{pathfind::reachability_2d_from, remove_foliage::find_trees, sim::desire_lines::*};

use self::{
    make_name::make_town_name,
    storage_pile::{update_lumber_pile_visuals, update_stone_pile_visuals, LumberPile, StonePile},
};

use super::*;

pub fn sim(mut level: Level) {
    let mut replay = Replay::new(&level);
    replay.say(&format!(
        "{}: Founding of {}",
        rand_range(1400..1550),
        make_town_name()
    ));

    let mut world = World::new();
    world.init_resource::<Tick>();

    let city_center = choose_starting_area(&level);
    let city_center_pos = level.ground(city_center.center());

    // Starting resources
    for _ in 0..6 {
        let (pos, params) = LumberPile::make(
            &mut level,
            city_center.center_vec2(),
            city_center.center_vec2(),
        );

        let goods = {
            let mut stock = Goods::default();
            stock.add(Stack::new(Good::Wood, 200.));
            stock
        };
        world.spawn((
            Pos(pos.as_vec3()),
            params,
            OutPile {
                available: goods.clone(),
            },
            Pile {
                goods,
                interact_distance: params.width,
            },
        ));
    }
    for _ in 0..6 {
        let (pos, params) = StonePile::make(&mut level, city_center.center_vec2());

        let goods = {
            let mut stock = Goods::default();
            stock.add(Stack::new(Good::Stone, 140.));
            stock
        };
        world.spawn((
            Pos(pos),
            params,
            OutPile {
                available: goods.clone(),
            },
            Pile {
                goods,
                interact_distance: 2,
            },
        ));
    }
    let starting_resources = {
        let mut stock = Goods::default();
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
                update_lumber_pile_visuals,
            ),
            (
                quarry::assign_worker,
                quarry::make_stone_piles,
                update_stone_pile_visuals,
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

        if tick < 20 {
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
