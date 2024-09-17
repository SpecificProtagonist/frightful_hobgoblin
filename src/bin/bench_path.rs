#![allow(dead_code)]
use std::fs::read_to_string;
use std::time::Instant;

use frightful_hobgoblin::pathfind::{pathfind, reachability_2d_from, reachability_from};
use frightful_hobgoblin::*;
use itertools::Itertools;

fn main() {
    let args = std::env::args().collect_vec();
    if args.len() != 2 {
        eprintln!("Expected exactly one argument: path to config file");
        std::process::exit(1)
    }
    let config_file = &args[1];
    let config: Config =
        toml::from_str(&read_to_string(config_file).expect("Failed to read config"))
            .expect("Failed to parse config");

    let level = config.load_level();

    let i = 2000;
    let start = Instant::now();
    for _ in 0..i {
        std::hint::black_box(pathfind(
            &level,
            level.ground(ivec2(-50, 50)) + IVec3::Z,
            level.ground(ivec2(100, -100)),
            1,
        ));
    }
    let time = Instant::now() - start;
    println!(
        "Pathing {i} iterations, took {}ms/iter",
        time.as_micros() as f32 / 1000. / i as f32
    );

    let i = 200;
    let start = Instant::now();
    for _ in 0..i {
        std::hint::black_box(reachability_from(
            &level,
            level.ground(ivec2(0, 0)) + IVec3::Z,
        ));
    }
    let time = Instant::now() - start;
    println!(
        "Reachability {i} iterations, took {}ms/iter",
        time.as_micros() as f32 / 1000. / i as f32
    );

    let i = 200;
    let start = Instant::now();
    for _ in 0..i {
        std::hint::black_box(reachability_2d_from(&level, ivec2(0, 0)));
    }
    let time = Instant::now() - start;
    println!(
        "Reachability (2d) {i} iterations, took {}ms/iter",
        time.as_micros() as f32 / 1000. / i as f32
    );
}
