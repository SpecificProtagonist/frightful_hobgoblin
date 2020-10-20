mod geometry;
mod world;
mod remove_foliage;
mod make_trees;
mod make_divider;
mod make_misc;
mod make_name;
mod make_house;

use std::time::Instant;

use world::*;
use geometry::*;

// How far outside of the borders of the work area is loaded
const LOAD_MARGIN: i32 = 20;

fn main() {

    let tmp_world_load_path: &str = concat!(include_str!("../save_path"), "mc-gen base");
    let tmp_world_save_path: &str = concat!(include_str!("../save_path"), "mc-gen generated");
    let tmp_area = Rect {
        min: Column(-100, -100),
        max: Column(100, 100)
    };


    drop(std::fs::remove_dir_all(tmp_world_save_path));
    copy_dir::copy_dir(tmp_world_load_path, tmp_world_save_path).expect("Failed to create save");

    let time_start = Instant::now();
    let mut world = World::new(tmp_world_save_path, tmp_area);
    let time_loaded = Instant::now();
    generate(&mut world, tmp_area);
    let time_generated = Instant::now();
    world.save().unwrap();
    let time_saved = Instant::now();

    println!("Timings | load: {:?}, generation: {:?}, saving: {:?}, total: {:?}",
        time_loaded-time_start, time_generated-time_loaded, time_saved-time_generated, time_saved-time_start);
}

fn generate(world: &mut World, area: Rect) {
    world.entities.push(Entity {
        pos: Pos(0,200,0),
        data: Villager {
            name: String::from("Rollo"),
            biome: world.biome(Column(0,0)),
            profession: Profession::Leatherworker,
            carrying: Some(Log(TreeSpecies::DarkOak, LogType::Normal(Axis::X), LogOrigin::Manmade))
        },
        id: 1
    });
}