use std::env::args;
use std::fs::read_to_string;

use debug_image::MapImage;
use frightful_hobgoblin::sim::sim;
use frightful_hobgoblin::*;
use itertools::Itertools;
use nanorand::*;

fn main() {
    let args = args().collect_vec();
    if args.len() != 2 {
        eprintln!("Expected exactly one argument: path to config file");
        std::process::exit(1)
    }
    let config_file = &args[1];
    let config: Config =
        toml::from_str(&read_to_string(config_file).expect("Failed to read config"))
            .expect("Failed to parse config");
    let seed = config
        .seed
        .unwrap_or(tls_rng().generate::<u16>() as u64 % 999);
    println!("Seed: {seed}");
    RNG.set(WyRand::new_seed(seed));

    let level = config.load_level();

    heightmap(&level);

    sim(level, config);
}

fn heightmap(level: &Level) {
    let mut map = MapImage::new(level.area());
    map.heightmap(level);
    map.water(level);
    map.save("heightmap.png");
}
