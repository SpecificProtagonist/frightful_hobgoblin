#![allow(dead_code)]
use std::fs::File;

use config::*;
use mc_gen::sim::sim;
use mc_gen::*;
use nanorand::*;
use nbt::decode::read_gzip_compound_tag;

fn main() {
    let seed = match std::env::args().nth(1) {
        Some(seed) if seed == "random" => tls_rng().generate::<u16>() % 999 as u64,
        Some(seed) => seed.parse().expect("Invalid seed"),
        None => get_seed(SAVE_READ_PATH),
    };
    println!("Seed: {seed}");
    RNG.set(WyRand::new_seed(seed));

    let area = Rect::new_centered(ivec2(AREA[0], AREA[1]), ivec2(AREA[2], AREA[3]));

    let level = Level::new(SAVE_READ_PATH, SAVE_WRITE_PATH, area);

    sim(level);
}

fn get_seed(path: &str) -> u64 {
    let nbt =
        read_gzip_compound_tag(&mut File::open(format!("{path}/level.dat")).unwrap()).unwrap();
    nbt.get_compound_tag("Data")
        .unwrap()
        .get_compound_tag("WorldGenSettings")
        .unwrap()
        .get_i64("seed")
        .unwrap() as u64
}
