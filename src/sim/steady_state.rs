use bevy_ecs::schedule::ExecutorKind;
use itertools::Itertools;
use std::fmt::Write;

use self::{
    quarry::Quarry,
    stall::{MarketStall, StallNotYetPlanned},
};
use crate::*;
use sim::*;

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
            data.crane_rot_target = (data.crane_rot + rand_range(4..12)) % 16;
            let start_rot = data.crane_rot;
            while quarries.get(world, quarry).unwrap().rotating() {
                sched.run(world);
                world.run_system(tick).unwrap();
            }
            for _ in 0..rand_range(10..100) {
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

    // Walking
    let houses = world
        .query_filtered::<Entity, With<House>>()
        .iter(world)
        .collect_vec();
    let mut chimneys = HashMap::default();
    world.resource_mut::<Replay>().track = 0;
    for &house in &houses {
        if let Some(pos) = world.get::<House>(house).unwrap().chimney {
            let id = Id::default();
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
        let home = assignable.swap_remove(rand_range(0..assignable.len()));
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

        let walk = world.register_system(walk);
        let returning = world.resource_mut::<Replay>().begin_next_track();
        let home_pos = world.get::<Pos>(home).unwrap().block();
        world.entity_mut(villager).insert(MoveTask::new(home_pos));
        while world.get::<MoveTask>(villager).is_some() {
            world.run_system(walk).unwrap();
            world.run_system(tick).unwrap();
        }

        let mut tracks = Vec::new();
        for destination in destinations {
            tracks.push(world.resource_mut::<Replay>().begin_next_track());
            let goal = world.get::<Pos>(destination).unwrap().block()
                + ivec3(rand_range(-1..=1), rand_range(-1..=1), 0);
            world
                .entity_mut(villager)
                .insert(MoveTask { goal, distance: 1 });
            while world.get::<MoveTask>(villager).is_some() {
                world.run_system(walk).unwrap();
                world.run_system(tick).unwrap();
            }
            for _ in 0..rand_range(20..200) {
                world.run_system(tick).unwrap();
            }
            world.entity_mut(villager).insert(MoveTask::new(home_pos));
            while world.get::<MoveTask>(villager).is_some() {
                world.run_system(walk).unwrap();
                world.run_system(tick).unwrap();
            }
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
}
