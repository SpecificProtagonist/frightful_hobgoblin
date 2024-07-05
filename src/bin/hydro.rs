use bevy_math::ivec2;
use frightful_hobgoblin::{building_plan::choose_starting_area, debug_image::*, Level, Rect, RNG};
#[path = "../../config_local.rs"]
mod config;
use config::*;
use nanorand::{tls_rng, Rng, WyRand};

fn main() {
    let area = Rect::new_centered(ivec2(AREA[0], AREA[1]), ivec2(AREA[2], AREA[3]));

    let level = Level::new(SAVE_READ_PATH, SAVE_WRITE_PATH, area);

    let mut map = MapImage::new(area);
    map.heightmap(&level);
    map.water(&level);
    map.ocean_and_river(&level);
    let seed = tls_rng().generate::<u16>() as u64 % 999;
    RNG.set(WyRand::new_seed(seed));
    println!("Seed: {seed}");
    for column in choose_starting_area(&level) {
        map.set(column, Color::Building);
    }

    map.save("hydro.png");
}
