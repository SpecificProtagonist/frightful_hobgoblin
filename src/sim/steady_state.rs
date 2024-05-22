use bevy_ecs::schedule::ExecutorKind;
use itertools::Itertools;
use std::fmt::Write;

use self::quarry::Quarry;
use crate::*;
use sim::*;

// TODO: walk around; turn chimney smoke on/off

pub fn generate(world: &mut World) {
    let mut handlers = 0;
    let tick = world.register_system(tick_replay);
    // Ugh, ugly hack because you can't set the change ticks of a system
    world.resource_mut::<Replay>().skip_changes_once = true;
    world.run_system(tick).unwrap();
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
            let start_rot = quarries.get(world, quarry).unwrap().crane_rot;
            quarries.get_mut(world, quarry).unwrap().crane_rot_target = rand_range(0..16);
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
                    "execute if score @s rand matches {i} run data modify entity @s data.track set value {track}",
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
}

// /// Animations to be perpetually run after the replay is done
// pub fn _animate(
//     level: Res<Level>,
//     houses: Query<&Pos, (With<House>, Without<Planned>)>,
//     stalls: Query<&Pos, With<MarketStall>>,
//     quarries: Query<&Quarry, Without<Planned>>,
// ) {
//     for quarry in &quarries {
//         world.run_system_once(|mut replay: ResMut<Replay>| replay.begin_next_track());
//         world.insert_resource(Tick(0));
//         let sys = world.register_system(tick_replay);
//         let mut sched = Schedule::default();
//         sched.set_executor_kind(ExecutorKind::SingleThreaded);
//         sched.add_systems((
//             move |mut tick: ResMut<Tick>, mut replay: ResMut<Replay>| {
//                 replay.dbg(&format!("track {track} tick {}", tick.0));
//                 tick.0 += 1;
//             },
//             tick_replay,
//         ));
//         world.run_system(sys).unwrap();
//         for _ in 0..70 {
//             sched.run(&mut world);
//             world.run_system(sys).unwrap();
//         }
//     }
//     // for start in &houses {
//     //     let mut prev_total = 0;
//     //     let mut paths = Vec::new();
//     //     for (weight, end) in houses
//     //         .iter()
//     //         .map(|p| (1, p))
//     //         .chain(stalls.iter().map(|p| (4, p)))
//     //     {
//     //         if start == end {
//     //             continue;
//     //         }
//     //         let path = pathfind(&level, start.block(), end.block(), 2);
//     //         paths.push((prev_total..prev_total + weight, path));
//     //         prev_total += weight;
//     //     }
//     // }
// }
