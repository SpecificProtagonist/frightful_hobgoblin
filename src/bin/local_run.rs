use config::*;
use mc_gen::sim::sim;
use mc_gen::*;
use nanorand::*;

/// The config isn't commited to git because it just contains the paths to the world folders
#[path = "../../config_local.rs"]
mod config;

fn main() {
    let seed = match std::env::args().nth(1) {
        Some(seed) => seed.parse().expect("Invalid seed"),
        None => tls_rng().generate::<u16>() as u64 % 999,
    };
    println!("Seed: {seed}");
    RNG.set(WyRand::new_seed(seed));

    let area = Rect::new_centered(ivec2(AREA[0], AREA[1]), ivec2(AREA[2], AREA[3]));

    let level = Level::new(SAVE_READ_PATH, SAVE_WRITE_PATH, area);

    sim(level);
}
