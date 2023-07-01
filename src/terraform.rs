use std::{collections::HashMap, usize};

use crate::*;

pub enum WallCrest {
    None,
    Full,
    Fence,
    Wall,
}

pub fn make_retaining_wall(
    world: &mut impl WorldView,
    area: &Polygon,
    height: i32,
    crest: WallCrest,
) {
    let material = Cobble;
    // Placement order matters for replay -> build wall first
    let crest = &match crest {
        WallCrest::None => Air,
        WallCrest::Full => FullBlock(material),
        WallCrest::Fence => Block::Fence(Wood(world.biome(area.0[0]).default_tree_species())),
        WallCrest::Wall => Fence(material),
    };

    for column in area.border(LineStyle::ThickWobbly) {
        let mut y = world.height(column);
        // Check if wall is neccessary
        if y > height || (y == height && !side_exposted(world, column.at(y))) {
            // Todo: also skip this column if the only exposed side is within the polygon
            continue;
        }

        // Build wall
        while matches!(world.get(column.at(y)), Soil(_)) {
            y -= 1;
        }
        for y in y..=height {
            world.set(column.at(y), FullBlock(material))
        }
        let above = world.get_mut(column.at(height + 1));
        if matches!((crest, &above), (Air, GroundPlant(_))) {
            *above = Air
        } else {
            *above = crest.clone()
        }

        *world.height_mut(column) = height;
    }

    // Then fill
    // TODO: bottom to top
    for column in area.iter() {
        if world.height(column) < height {
            let soil = &Soil(get_filling_soil(world, column));
            for y in world.height(column)..=height {
                world.set(column.at(y), soil)
            }
            *world.height_mut(column) = height;
        }
    }
}

fn get_filling_soil(world: &impl WorldView, column: Column) -> Soil {
    if let Soil(soil) = *world.get(column.at(world.height(column))) {
        soil
    } else {
        world.biome(column).default_topsoil()
    }
}

pub fn make_foundation_sloped(
    world: &mut impl WorldView,
    mut area: Rect,
    height: i32,
    material: Material,
) {
    // TODO: proper placement order

    remove_foliage::trees(world, area.into_iter(), false);
    for column in area {
        world.set(column.at(height), FullBlock(material));
    }

    let mut y = height - 1;
    let mut width_increased_last_layer = false;
    let mut outmost_is_wall = false;
    let mut block_placed_this_layer = true;

    while block_placed_this_layer {
        block_placed_this_layer = false;
        for column in area.shrink(1) {
            world.set_if_not_solid(column.at(y), FullBlock(material));
        }
        for column in area.border() {
            if !world.get(column.at(y)).solid() || side_exposted(world, column.at(y)) {
                block_placed_this_layer = true;
                world.set(column.at(y), FullBlock(material));
            }
        }
        if outmost_is_wall {
            for column in area.grow(1).border() {
                if !world.get(column.at(y)).solid() {
                    block_placed_this_layer = true;
                    world.set(column.at(y), Fence(material));
                }
            }
        }

        y -= 1;

        if !width_increased_last_layer {
            if outmost_is_wall {
                area = area.grow(1);
            }
            outmost_is_wall ^= true;
        }
        width_increased_last_layer ^= true;
    }
}

pub fn make_foundation_straight(
    world: &mut impl WorldView,
    area: Rect,
    height: i32,
    material: Material,
) {
    for column in area {
        world.set(column.at(height), FullBlock(material));
        let mut y = height - 1;
        let ground_height = world.height(column);
        while (y > ground_height) | soil_exposted(world, column.at(y)) {
            world.set(column.at(y), FullBlock(material));
            y -= 1;
        }
        for y in (height + 1)..=ground_height {
            world.set(column.at(y), Air);
        }
    }

    make_support(
        world,
        ((area.min.0 + 1)..area.max.0).map(|x| Column(x, area.min.1)),
        height,
        HDir::ZPos,
        material,
    );
    make_support(
        world,
        ((area.min.0 + 1)..area.max.0).map(|x| Column(x, area.max.1)),
        height,
        HDir::ZNeg,
        material,
    );
    make_support(
        world,
        ((area.min.1 + 1)..area.max.1).map(|z| Column(area.min.0, z)),
        height,
        HDir::XPos,
        material,
    );
    make_support(
        world,
        ((area.min.1 + 1)..area.max.1).map(|z| Column(area.max.0, z)),
        height,
        HDir::XNeg,
        material,
    );

    fn make_support(
        world: &mut impl WorldView,
        columns: impl Iterator<Item = Column>,
        y: i32,
        facing: HDir,
        material: Material,
    ) {
        let support_chance = 0.7;
        let min_height = 3;
        let max_height = 6;
        let mut just_placed = false;
        for column in columns {
            let column = column - Vec2::from(facing);
            let mut ground_distance = y.saturating_sub(world.height(column));
            while soil_exposted(world, column.at(y - ground_distance - 1)) {
                ground_distance += 1;
            }
            just_placed = if (ground_distance >= min_height)
                & (ground_distance <= max_height)
                & !just_placed
                & rand(support_chance)
            {
                world.set(column.at(y), Stair(material, facing, Flipped(false)));
                for y in y - ground_distance..y {
                    world.set(column.at(y), FullBlock(material));
                }
                true
            } else {
                false
            };
        }
    }
}

pub fn soil_exposted(world: &impl WorldView, pos: Pos) -> bool {
    matches!(world.get(pos), Soil(..)) & side_exposted(world, pos)
}

pub fn side_exposted(world: &impl WorldView, pos: Pos) -> bool {
    !(world.get(pos + Vec2(0, 1)).solid()
        && world.get(pos + Vec2(0, -1)).solid()
        && world.get(pos + Vec2(1, 0)).solid()
        && world.get(pos + Vec2(-1, 0)).solid())
}

pub fn average_height(world: &impl WorldView, area: impl Iterator<Item = Column>) -> u8 {
    let mut sum = 0.0;
    let mut count = 0;
    for column in area {
        sum += world.height(column) as f32;
        count += 1;
    }
    (sum / count as f32) as u8
}

pub fn slope(world: &impl WorldView, column: Column) -> Vec2 {
    let mut neighbors = [0; 9];
    for dx in -1..=1 {
        for dz in -1..=1 {
            neighbors[(4 + dx + 3 * dz) as usize] = world.height(column + Vec2(dx, dz)) as i32;
        }
    }
    // Sobel kernel
    let slope_x = (neighbors[2] + 2 * neighbors[5] + neighbors[8])
        - (neighbors[0] + 2 * neighbors[3] + neighbors[6]);
    let slope_z = (neighbors[6] + 2 * neighbors[7] + neighbors[8])
        - (neighbors[0] + 2 * neighbors[1] + neighbors[2]);
    Vec2(slope_x, slope_z)
}

/*
/// Neighborborhood_size specifies a square. Results aren't fully acurate, but that's ok
pub fn find_local_maxima(world: &impl WorldView, area: Rect, neighborhood_size: u8) -> Vec<Pos> {
    // Divide area into cells
    let cell_size = neighborhood_size as i32 / 3;
    let cell_count = area.size() / cell_size;
    // Actually searched area is rounded down to integer number of cells
    let area = {
        let min = area.min + (area.size() % cell_size) / 2;
        Rect {
            min,
            max: min + cell_count * cell_size,
        }
    };
    for z in (area.min.1..area.max.1).step_by(cell_size as usize) {
        for x in (area.min.0..area.max.0).step_by(cell_size as usize) {
            Rect {
                min: Vec2(x, z),
                max: Vec2(x + cell_size, z + cell_size),
            }
            .iter()
        }
    }
    // find highest in each cell
    // return highest in cell when n higher in surrounding cells

    todo!()
}
*/

// TODO: add average
// TODO: move into World, cache
pub fn max_chunk_heights(world: &World) -> HashMap<ChunkIndex, i32> {
    world
        .chunks()
        .map(|chunk| {
            (
                chunk,
                chunk
                    .area()
                    .into_iter()
                    .map(|column| world.height(column))
                    .max()
                    .unwrap(),
            )
        })
        .collect()
}
