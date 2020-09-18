mod world;
use world::*;

const TMP_WORLD_LOAD_PATH: &str = concat!(include_str!("../save_path"), "mc-gen base");
const TMP_WORLD_SAVE_PATH: &str = concat!(include_str!("../save_path"), "mc-gen generated");

fn main() {
    drop(std::fs::remove_dir_all(TMP_WORLD_SAVE_PATH));
    copy_dir::copy_dir(TMP_WORLD_LOAD_PATH, TMP_WORLD_SAVE_PATH).unwrap();

    let area = Area {
        min: Column(-50, -50),
        max: Column(50, 50)
    };

    let mut world = World::new(TMP_WORLD_SAVE_PATH);
    world.load_area(area).unwrap();
    generate(&mut world, area);
    world.save().unwrap();
}

fn generate(world: &mut World, area: Area) {
    for x in area.min.0 ..= area.max.0 {
        for z in area.min.1 ..= area.max.1 {
            for y in 40 .. 100 {
                let block = &mut world[Pos(x, y, z)];
                if let Block::Log(..) = block {
                    *block = Block::Stone;
                }
            }
            world[Pos(x, 80, z)] = Block::Stone;
        }
    }

    for y in 80 .. 200 {
        world[Pos(0, y, 0)] = Block::Log(TreeSpecies::Acacia, LogType::FullBark);
    }
}