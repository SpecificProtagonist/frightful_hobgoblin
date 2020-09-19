mod geometry_types;
mod world;
mod simplify;
mod remove_foliage;

use world::*;
use geometry_types::*;

const TMP_WORLD_LOAD_PATH: &str = concat!(include_str!("../save_path"), "mc-gen base");
const TMP_WORLD_SAVE_PATH: &str = concat!(include_str!("../save_path"), "mc-gen generated");

fn main() {
    drop(std::fs::remove_dir_all(TMP_WORLD_SAVE_PATH));
    copy_dir::copy_dir(TMP_WORLD_LOAD_PATH, TMP_WORLD_SAVE_PATH).unwrap();

    let area = Rect {
        min: Column(-50, -50),
        max: Column(50, 50)
    };

    let margin = 20;
    let loaded_area = Rect {
        min: area.min - Vec2(margin, margin),
        max: area.max + Vec2(margin, margin)
    };

    let mut world = World::new(TMP_WORLD_SAVE_PATH);
    world.load_area(loaded_area).unwrap();
    generate(&mut world, area);
    world.save().unwrap();
}

fn generate(world: &mut World, area: Rect) {
    remove_foliage::remove_trees(world, area, true);
}

