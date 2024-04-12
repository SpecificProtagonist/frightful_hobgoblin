use argh::FromArgs;
use mc_gen::sim::sim;
use mc_gen::*;
use nanorand::*;

#[derive(FromArgs)]
/// GDMC generator
struct Config {
    /// path to the world
    #[argh(positional)]
    path: String,
    /// seed to use. If not set, a random seed is chosen.
    #[argh(option)]
    seed: Option<u64>,
    /// lower x bound of building area
    #[argh(positional)]
    min_x: i32,
    /// upper x bound of building area
    #[argh(positional)]
    max_x: i32,
    /// lower y bound of building area
    #[argh(positional)]
    min_y: i32,
    /// upper y bound of building area
    #[argh(positional)]
    max_y: i32,
}

fn main() {
    let config: Config = argh::from_env();
    let seed = config
        .seed
        .unwrap_or(tls_rng().generate::<u16>() as u64 % 999);
    println!("Seed: {seed}");
    RNG.set(WyRand::new_seed(seed));

    let area = Rect::new_centered(
        ivec2(config.min_x, config.min_y),
        ivec2(config.max_x, config.max_y),
    );

    let level = Level::new(&config.path, &config.path, area);

    sim(level);
}
