#![allow(dead_code)]
#[path = "../../config_local.rs"]
mod config;
use config::*;
use frightful_hobgoblin::pathfind::reachability_from;
use frightful_hobgoblin::sim::building_plan::choose_starting_area;
use frightful_hobgoblin::*;
use nanorand::*;
use num_traits::FromPrimitive;

fn main() {
    let seed = std::env::args()
        .nth(1)
        .map(|seed| seed.parse().expect("Invalid seed"));
    let seed = seed.unwrap_or(tls_rng().generate::<u16>() as u64 % 999);
    println!("Seed: {seed}");
    RNG.set(WyRand::new_seed(seed));

    let area = Rect::new_centered(ivec2(AREA[0], AREA[1]), ivec2(AREA[2], AREA[3]));

    let mut level = Level::new(SAVE_READ_PATH, SAVE_WRITE_PATH, area);

    let city_center = choose_starting_area(&level);
    let center = level.ground(city_center.center()) + IVec3::Z;

    for (pos, cost) in reachability_from(&level, center) {
        level(pos, Wool(Color::from_u32((cost / 100).min(15)).unwrap()))
    }

    level.debug_save();
}
