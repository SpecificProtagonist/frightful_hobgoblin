use crate::geometry::*;
use crate::world::*;
use num_traits::FromPrimitive;
use rand::prelude::*;

// Todo: maybe make a prefab system in mc (with replacement of block)?
// Would allow for very easy creation & good quality, but less parametricism

pub fn make_scarecrow(world: &mut World, column: Column) {
    let mut pos = column.at_height(world.heightmap(column) + 1);
    let fence_block = &Fence(Fence::Wood(world.biome(column).random_tree_species()));
    let direction = HDir::from_u8(thread_rng().gen_range(0, 4)).unwrap();

    let colors = &[
        Color::Black,
        Color::Gray,
        Color::LightGray,
        Color::White,
        Color::Red,
        Color::Brown,
        Color::Blue,
        Color::Green,
    ];

    let center_block = &if rand(0.5) {
        Wool(colors[thread_rng().gen_range(0, colors.len())])
    } else {
        Hay
    };

    world.set(pos, fence_block);
    pos += Vec3(0, 1, 0);
    if rand(0.34) {
        if rand(0.65) {
            world.set(pos, fence_block);
        } else {
            world.set(pos, center_block);
        }
        pos += Vec3(0, 1, 0);
    }
    world.set(pos, center_block);
    world.set(pos + Vec2::from(direction).clockwise(), fence_block);
    world.set(pos + Vec2::from(direction).counterclockwise(), fence_block);
    pos += Vec3(0, 1, 0);
    world.set(pos, GroundPlant(GroundPlant::Pumpkin(direction)));
}

pub fn make_signpost() {}
