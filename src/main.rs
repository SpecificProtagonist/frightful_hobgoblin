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
    remove_ground_foilage(world, area);
}

fn remove_ground_foilage(world: &mut World, area: Area) {
    for x in area.min.0 ..= area.max.0 {
        for z in area.min.1 ..= area.max.1 {
            let base_height = if let Some(water_height) = world.watermap(Column(x,z)) {
                water_height.into()
            } else {
                world.heightmap(Column(x,z))
            };
            for y in base_height + 1 ..= base_height + 2 {
                let block = &mut world[Pos(x,y,z)];
                if match block { Block::Log(..) => false, Block::Leaves(..) => false, _ => true } {
                    *block = Block::Air
                } 
            }
        }
    }
}