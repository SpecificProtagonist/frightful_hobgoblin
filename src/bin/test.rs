#![allow(dead_code)]
use config::*;
use mc_gen::make_trees::GrowTree;
use mc_gen::sim::sim;
use mc_gen::*;
use rand::rngs::StdRng;
use rand::{thread_rng, RngCore, SeedableRng};

fn main() {
    let seed = std::env::args()
        .nth(1)
        .map(|seed| seed.parse().expect("Invalid seed"));
    let seed = seed.unwrap_or(thread_rng().next_u32() as u16 as u64);
    println!("Seed: {seed}");
    RNG.set(StdRng::seed_from_u64(seed));

    let _ = std::fs::remove_dir_all(SAVE_WRITE_PATH);
    copy_dir::copy_dir(SAVE_READ_PATH, SAVE_WRITE_PATH).expect("Failed to create save");

    let area = Rect::new_centered(ivec2(AREA[0], AREA[1]), ivec2(AREA[2], AREA[3]));

    let mut level = Level::new(SAVE_WRITE_PATH, area);

    // for i in 0..10 {
    //     make_trees::make_straight(&mut level, ivec3(i * 10, -20, 120), Oak);
    // }

    // for i in 0..10 {
    //     make_trees::make_tiny(&mut level, ivec3(i * 10, -10, 120), Oak);
    // }

    // for i in 0..10 {
    //     let tree = GrowTree::make();
    //     for step in 1..=10 {
    //         tree.place(
    //             &mut level,
    //             vec3(i as f32 * 10., step as f32 * 10., 120.),
    //             step as f32 * 0.2,
    //         )
    //     }
    // }

    sim(level);
}
