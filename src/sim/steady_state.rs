use bevy_ecs::{schedule::ExecutorKind, system::RunSystemOnce};
use bevy_utils::default;
use itertools::Itertools;
use std::fmt::Write;

use self::{
    market::{MarketStall, StallNotYetPlanned},
    personal_name::name,
    quarry::Quarry,
    roads::Roads,
};
use crate::*;
use sim::*;

#[derive(Component)]
pub struct Trader;

/// Animations to be perpetually run after the replay is done
pub fn generate(world: &mut World) {
    let mut handlers = 0;
    let tick = world.register_system(tick_replay);
    // Ugh, ugly hack because you can't set the change ticks of a system
    world.resource_mut::<Replay>().skip_changes_once = true;
    world.run_system(tick).unwrap();

    // Quarry crane
    let mut quarries = world.query_filtered::<&mut Quarry, With<Built>>();
    for mut quarry in quarries.iter_mut(world) {
        quarry.crane_rot_target = quarry.crane_rot;
    }
    for quarry in world
        .query_filtered::<Entity, (With<Quarry>, With<Built>)>()
        .iter(world)
        .collect_vec()
    {
        let pos = world.get::<Pos>(quarry).unwrap().0;

        let mut tracks = Vec::new();
        for _ in 0..5 {
            tracks.push(world.resource_mut::<Replay>().begin_next_track());
            let mut sched = Schedule::default();
            sched.set_executor_kind(ExecutorKind::SingleThreaded);
            sched.add_systems((quarry::update_quarry_rotation,));
            let mut data = quarries.get_mut(world, quarry).unwrap();
            data.crane_rot_target = (data.crane_rot + rand(4..12)) % 16;
            let start_rot = data.crane_rot;
            while quarries.get(world, quarry).unwrap().rotating() {
                sched.run(world);
                world.run_system(tick).unwrap();
            }
            for _ in 0..rand(10..100) {
                world.run_system(tick).unwrap();
            }
            quarries.get_mut(world, quarry).unwrap().crane_rot_target = start_rot;
            while quarries.get(world, quarry).unwrap().rotating() {
                sched.run(world);
                world.run_system(tick).unwrap();
            }
        }

        let mut replay = world.resource_mut::<Replay>();
        let handler_name = format!("quarry_{handlers}");
        handlers += 1;
        replay.mcfunction(&format!("on_idle/{handler_name}"), &{
            let mut str = format!(
                "
                execute store result score @s sim_{0}_sleep run random value 0..200
                execute store result score @s rand run random value 0..{1}\n",
                invocation(),
                tracks.len()
            );
            for (i, track) in tracks.iter().enumerate() {
                writeln!(
                    str,
                    "execute if score @s rand matches {i} run data modify entity @s data.play set value {track}",
                )
                .unwrap();
            }
            str
        });
        replay.track = 0;
        replay.command(format!(
            "summon marker {} {} {} {{Tags:[\"sim_{3}_tick\"],data:{{on_idle:\"{4}\"}}}}",
            pos.x,
            pos.z,
            pos.y,
            invocation(),
            handler_name
        ));
    }

    // Villagers
    let walk = world.register_system(walk);
    let walk = |world: &mut World, villager: Entity, goal: IVec3, distance: i32| {
        world
            .entity_mut(villager)
            .insert(MoveTask { goal, distance });
        while world.get::<MoveTask>(villager).is_some() {
            world.run_system(walk).unwrap();
            world.run_system(tick).unwrap();
        }
    };
    let houses = world
        .query_filtered::<Entity, With<House>>()
        .iter(world)
        .collect_vec();
    let mut chimneys = HashMap::default();
    world.resource_mut::<Replay>().track = 0;
    for &house in &houses {
        if let Some(pos) = world.get::<House>(house).unwrap().chimney {
            let id = Id::new();
            chimneys.insert(house, id);
            world.resource_mut::<Replay>().command(format!(
                "summon marker {} {} {} {{{}}}",
                pos.x,
                pos.z,
                pos.y,
                id.snbt()
            ));
        }
    }
    let mut assignable = houses.clone();
    let villagers = world
        .query_filtered::<Entity, With<Villager>>()
        .iter(world)
        .collect_vec();
    for &villager in &villagers {
        world
            .entity_mut(villager)
            .remove::<(MoveTask, MovePath, InBoat)>();
    }
    for villager in villagers {
        world.get_mut::<Villager>(villager).unwrap().carry = None;
        if assignable.is_empty() {
            assignable.clone_from(&houses);
        }
        let home = assignable.swap_remove(rand(0..assignable.len()));
        let mut destinations = world
            .query_filtered::<Entity, (With<MarketStall>, Without<StallNotYetPlanned>)>()
            .iter(world)
            .collect_vec();
        for _ in 0..6 {
            let house = *houses.choose();
            if house != home {
                destinations.push(house);
            }
        }

        let returning = world.resource_mut::<Replay>().begin_next_track();
        let home_pos = world.get::<Pos>(home).unwrap().block();
        walk(world, villager, home_pos, 0);

        let mut tracks = Vec::new();
        for destination in destinations {
            tracks.push(world.resource_mut::<Replay>().begin_next_track());
            let goal = world.get::<Pos>(destination).unwrap().block()
                + ivec3(rand(-1..=1), rand(-1..=1), 0);
            walk(world, villager, goal, 1);
            for _ in 0..rand(20..200) {
                world.run_system(tick).unwrap();
            }
            walk(world, villager, home_pos, 0);
        }
        let biome = world.resource::<Level>().biome[home_pos];
        let mut replay = world.resource_mut::<Replay>();
        let handler_name = format!("villager_{handlers}");
        handlers += 1;
        replay.mcfunction(
            &format!("on_idle/{handler_name}"),
            &{
                let mut str = format!(
                    "
                    data modify entity @s[tag=!returned] data.play set value {returning}
                    execute store result score @s[tag=returned] sim_{0}_sleep run random value 100..600
                    execute store result score @s daytime run time query daytime
                    execute if score @s daytime matches 13000..23000 run return 0
                    execute store result score @s[tag=returned] rand run random value 0..{1}
                    tag @s add returned
                    ",
                    invocation(),
                    tracks.len(),
                );
                for (i, track) in tracks.iter().enumerate() {
                    writeln!(
                        str,
                        "execute if score @s[tag=returned] rand matches {i} run data modify entity @s data.play set value {track}",
                    )
                    .unwrap();
                }
                if let Some(&chimney) = chimneys.get(&home) {
                    use Biome::*;
                    let chance = match biome {
                        Snowy => 3,
                        Taiga => 2,
                        Desert |
                        Mesa |
                        Savanna => 0,
                        _ => 1
                    };
                    writeln!(str, "
                        execute store result score @s[tag=returned] rand run random value 0..1
                        execute if score @s[tag=returned] rand matches 0 run tag {chimney} remove sim_{0}_smoke
                        execute store result score @s[tag=returned] rand run random value 0..10
                        execute if score @s[tag=returned] rand matches 0..{chance} run tag {chimney} add sim_{0}_smoke
                    ", invocation()).unwrap();
                }
                str
            }
        );
        replay.track = 0;
        replay.command(format!(
            "summon marker {} {} {} {{Tags:[\"sim_{3}_tick\"],data:{{on_idle:\"{4}\"}}}}",
            home_pos.x,
            home_pos.z,
            home_pos.y,
            invocation(),
            handler_name,
        ));
    }

    // Wandering traders
    let stalls = world
        .query_filtered::<&Pos, With<MarketStall>>()
        .iter(world)
        .map(|p| p.block())
        .collect_vec();
    let mut road_starts = world
        .resource::<Roads>()
        .0
        .iter()
        .filter(|path| (path.len() > 60) & (path.iter().all(|n| !n.boat)))
        .map(|p| p.back().unwrap().pos)
        .collect_vec();

    let traders_count = stalls.len().min(road_starts.len());
    for _ in 0..traders_count {
        let i = rand(0..road_starts.len());
        let road_start = road_starts.remove(i);
        let id = Id::new();
        let Some(tavern) = world
            .query_filtered::<&Pos, With<Tavern>>()
            .iter(world)
            .next()
            .map(|p| p.block())
        else {
            break;
        };

        let track_enter = world.resource_mut::<Replay>().begin_next_track();
        let trader = world
            .spawn((
                id,
                PrevPos(default()),
                Pos(road_start.as_vec3()),
                Villager::default(),
                Trader,
            ))
            .id();
        world.run_system_once(name);
        walk(world, trader, tavern, 1);

        let track_trade = world.resource_mut::<Replay>().begin_next_track();
        for _ in 0..6 {
            walk(world, trader, *stalls.choose(), rand(0..=2));
            for _ in 0..rand(100..300) {
                world.run_system(tick).unwrap();
            }
        }
        walk(world, trader, tavern, 0);

        let track_leave = world.resource_mut::<Replay>().begin_next_track();
        walk(world, trader, road_start, 1);
        let mut replay = world.resource_mut::<Replay>();
        replay.command(format!("effect give {id} invisibility"));
        replay.command(format!("kill {id}"));

        let handler_name = format!("trader_{handlers}");
        handlers += 1;
        replay.mcfunction(
            &format!("on_idle/{handler_name}"),
            &format!(
                "
                data modify entity @s[tag=!entered] data.play set value {track_enter}
                execute store result score @s[tag=entered] sim_{0}_sleep run random value 100..600
                execute store result score @s daytime run time query daytime
                execute if score @s daytime matches 13000..23000 run return 0
                execute store result score @s[tag=entered] rand run random value 0..3
                execute if score @s[tag=entered] rand matches 0 run data modify entity @s data.play set value {track_leave}
                execute if score @s[tag=entered] rand matches 1.. run data modify entity @s data.play set value {track_trade}
                tag @s[tag=entered] add already_entered
                tag @s add entered
                tag @s[tag=already_entered] remove entered
                tag @s remove already_entered
                ",invocation()
            )
        );
        replay.track = 0;
        replay.command(format!(
            "summon marker {} {} {} {{Tags:[\"sim_{3}_tick\"],data:{{on_idle:\"{4}\"}}}}",
            tavern.x,
            tavern.z,
            tavern.y,
            invocation(),
            handler_name,
        ));
    }
}
