use argh::FromArgs;
use config::*;
use frightful_hobgoblin::sim::sim;
use frightful_hobgoblin::*;
use nanorand::*;

/// The config isn't commited to git because it just contains the paths to the world folders
#[path = "../../config_local.rs"]
mod config;

#[derive(FromArgs)]
/// Dev mode
struct Config {
    /// path to the world
    #[argh(option)]
    seed: Option<u64>,
    /// modify world instead of generating a replay
    /// (for debug; blockstates will be incorrect)
    #[argh(switch)]
    debug_save: bool,
}

fn main() {
    let config: Config = argh::from_env();
    let seed = config
        .seed
        .unwrap_or(tls_rng().generate::<u16>() as u64 % 999);
    println!("Seed: {seed}");
    RNG.set(WyRand::new_seed(seed));

    let area = Rect::new_centered(ivec2(AREA[0], AREA[1]), ivec2(AREA[2], AREA[3]));

    let level = Level::new(SAVE_READ_PATH, SAVE_WRITE_PATH, area);

    sim(level, config.debug_save);
}
