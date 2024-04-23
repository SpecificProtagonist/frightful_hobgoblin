#![allow(dead_code)]
use std::time::Instant;

#[path = "../../config_local.rs"]
mod config;
use config::*;
use e24u::pathfind::{pathfind, reachability_2d_from, reachability_from};
use e24u::*;

fn main() {
    let area = Rect::new_centered(ivec2(AREA[0], AREA[1]), ivec2(AREA[2], AREA[3]));

    let level = Level::new(SAVE_READ_PATH, SAVE_WRITE_PATH, area);

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
