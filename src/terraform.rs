use std::usize;

use crate::*;

pub enum WallCrest {
    None,
    Full,
    Fence,
    Wall,
}

pub fn make_retaining_wall(level: &mut Level, area: &Polygon, height: i32, crest: WallCrest) {
    let material = Cobble;
    // Placement order matters for replay -> build wall first
    let crest = match crest {
        WallCrest::None => Air,
        WallCrest::Full => FullBlock(material),
        WallCrest::Fence => Block::Fence(Wood(level.biome(area.0[0]).default_tree_species())),
        WallCrest::Wall => Fence(material),
    };

    for column in area.border(LineStyle::ThickWobbly) {
        let mut y = level.height(column);
        // Check if wall is neccessary
        if y > height || (y == height && !side_exposted(level, column.at(y))) {
            // Todo: also skip this column if the only exposed side is within the polygon
            continue;
        }

        // Build wall
        while matches!(level[column.at(y)], Soil(_)) {
            y -= 1;
        }
        for y in y..=height {
            level[column.at(y)] = FullBlock(material)
        }
        let above = &mut level[column.at(height + 1)];
        if matches!((crest, &above), (Air, GroundPlant(_))) {
            *above = Air
        } else {
            *above = crest
        }

        *level.height_mut(column) = height;
    }

    // Then fill
    // TODO: bottom to top
    for column in area.iter() {
        if level.height(column) < height {
            for y in level.height(column)..=height {
                level[column.at(y)] = Soil(get_filling_soil(level, column))
            }
            *level.height_mut(column) = height;
        }
    }
}

fn get_filling_soil(level: &Level, column: Vec2) -> Soil {
    if let Soil(soil) = level[column.at(level.height(column))] {
        soil
    } else {
        level.biome(column).default_topsoil()
    }
}

pub fn make_foundation_sloped(level: &mut Level, mut area: Rect, height: i32, material: Material) {
    // TODO: proper placement order

    remove_foliage::trees(level, area.into_iter(), false);
    for column in area {
        level[column.at(height)] = FullBlock(material);
    }

    let mut y = height - 1;
    let mut width_increased_last_layer = false;
    let mut outmost_is_wall = false;
    let mut block_placed_this_layer = true;

    while block_placed_this_layer {
        block_placed_this_layer = false;
        for column in area.shrink(1) {
            level[column.at(y)] |= FullBlock(material);
        }
        for column in area.border() {
            if !level[column.at(y)].solid() || side_exposted(level, column.at(y)) {
                block_placed_this_layer = true;
                level[column.at(y)] = FullBlock(material);
            }
        }
        if outmost_is_wall {
            for column in area.grow(1).border() {
                if !level[column.at(y)].solid() {
                    block_placed_this_layer = true;
                    level[column.at(y)] = Fence(material);
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

pub fn make_foundation_straight(level: &mut Level, area: Rect, height: i32, material: Material) {
    for column in area {
        level[column.at(height)] = FullBlock(material);
        let mut y = height - 1;
        let ground_height = level.height(column);
        while (y > ground_height) | soil_exposted(level, column.at(y)) {
            level[column.at(y)] = FullBlock(material);
            y -= 1;
        }
        for y in (height + 1)..=ground_height {
            level[column.at(y)] = Air;
        }
    }

    make_support(
        level,
        ((area.min.0 + 1)..area.max.0).map(|x| Vec2(x, area.min.1)),
        height,
        ZPos,
        material,
    );
    make_support(
        level,
        ((area.min.0 + 1)..area.max.0).map(|x| Vec2(x, area.max.1)),
        height,
        ZNeg,
        material,
    );
    make_support(
        level,
        ((area.min.1 + 1)..area.max.1).map(|z| Vec2(area.min.0, z)),
        height,
        XVec3,
        material,
    );
    make_support(
        level,
        ((area.min.1 + 1)..area.max.1).map(|z| Vec2(area.max.0, z)),
        height,
        XNeg,
        material,
    );

    fn make_support(
        level: &mut Level,
        columns: impl Iterator<Item = Vec2>,
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
            let mut ground_distance = y.saturating_sub(level.height(column));
            while soil_exposted(level, column.at(y - ground_distance - 1)) {
                ground_distance += 1;
            }
            just_placed = if (ground_distance >= min_height)
                & (ground_distance <= max_height)
                & !just_placed
                & rand(support_chance)
            {
                level[column.at(y)] = Stair(material, facing, Flipped(false));
                for y in y - ground_distance..y {
                    level[column.at(y)] = FullBlock(material);
                }
                true
            } else {
                false
            };
        }
    }
}

pub fn soil_exposted(level: &Level, pos: Vec3) -> bool {
    matches!(level[pos], Soil(..)) & side_exposted(level, pos)
}

pub fn side_exposted(level: &Level, pos: Vec3) -> bool {
    !(level[pos + Vec2(0, 1)].solid()
        && level[pos + Vec2(0, -1)].solid()
        && level[pos + Vec2(1, 0)].solid()
        && level[pos + Vec2(-1, 0)].solid())
}

pub fn slope(level: &Level, column: Vec2) -> Vec2 {
    let mut neighbors = [0; 9];
    for dx in -1..=1 {
        for dz in -1..=1 {
            neighbors[(4 + dx + 3 * dz) as usize] = level.height(column + Vec2(dx, dz));
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
pub fn find_local_maxima(level: &Level, area: Rect, neighborhood_size: u8) -> Vec<Vec3> {
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
