#![allow(dead_code)]
use config::*;
use mc_gen::sim::sim;
use mc_gen::*;
use nanorand::*;

fn main() {
    let seed = std::env::args()
        .nth(1)
        .map(|seed| seed.parse().expect("Invalid seed"));
    let seed = seed.unwrap_or(tls_rng().generate::<u16>() as u64);
    println!("Seed: {seed}");
    RNG.set(WyRand::new_seed(seed));

    let area = Rect::new_centered(ivec2(AREA[0], AREA[1]), ivec2(AREA[2], AREA[3]));

    let level = Level::new(SAVE_READ_PATH, SAVE_WRITE_PATH, area);

    sim(level);
}
