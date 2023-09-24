#![allow(dead_code)]
use config::*;
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

    let area = Rect::new_centered(ivec2(AREA[0], AREA[1]), ivec2(AREA[2], AREA[3]));

    let level = Level::new(SAVE_READ_PATH, SAVE_WRITE_PATH, area);

    sim(level);
}
