use bevy_ecs::schedule::ScheduleLabel;

use super::{quarry::StonePile, *};

pub fn sim(mut level: Level) {
    Id::load(&level.path);

    let mut world = World::new();
    world.init_resource::<Tick>();

    let city_center = choose_starting_area(&level);
    let city_center_pos = level.ground(city_center.center());

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
        CityCenter,
        OutPile {
            available: starting_resources.clone(),
        },
        Pile::new(starting_resources),
    ));

    for pos in city_center {
        let pos = level.ground(pos);
        level(pos, Wool(Magenta))
    }

    // Find trees
    for (pos, species) in find_trees(&level, level.area()) {
        world.spawn((Pos(pos.as_vec3()), Tree::new(species)));
    }

    #[derive(ScheduleLabel, Clone, Hash, Eq, PartialEq, Debug)]
    struct MainLoop;
    let mut sched = Schedule::new(MainLoop);
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
            (quarry::make_stone_piles, quarry::update_stone_pile_visuals),
            // plan_house,
            // plan_lumberjack,
            // plan_quarry,
            |mut commands: Commands, query: Query<Entity, (With<StonePile>, Without<InPile>)>| {
                for e in &query {
                    commands.entity(e).insert(InPile {
                        requested: {
                            let mut stock = Goods::default();
                            stock.add(Stack::new(Good::Stone, 30.));
                            stock
                        },
                        priority: None,
                    });
                }
            },
            apply_deferred,
            assign_builds,
            apply_deferred,
            new_construction_site,
            (test_build_house, test_build_lumberjack, test_build_quarry),
            apply_deferred,
            tick_replay,
            // remove_outdated,
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

        if tick < 5 {
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
