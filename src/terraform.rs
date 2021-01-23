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
    height: u8,
    crest: WallCrest,
) {
    // Placement order matters for replay -> build wall first
    let wall_block = &Stone(Stone::Cobble);
    let crest = &match crest {
        WallCrest::None => Air,
        WallCrest::Full => wall_block.clone(),
        WallCrest::Fence => {
            Block::Fence(Fence::Wood(world.biome(area.0[0]).default_tree_species()))
        }
        WallCrest::Wall => Block::Fence(Fence::Stone { mossy: false }),
    };

    for column in area.border(LineStyle::ThickWobbly) {
        let mut y = world.heightmap(column);
        // Check if wall is neccessary
        if y > height || (y == height && !side_exposted(world, column.at_height(y))) {
            // Todo: also skip this column if the only exposed side is within the polygon
            continue;
        }

        // Build wall
        while matches!(world.get(column.at_height(y)), Soil(_)) {
            y -= 1;
        }
        for y in y..=height {
            world.set(column.at_height(y), wall_block)
        }
        let above = world.get_mut(column.at_height(height + 1));
        if matches!((crest, &above), (Air, GroundPlant(_))) {
            *above = Air
        } else {
            *above = crest.clone()
        }

        *world.heightmap_mut(column) = height;
    }

    // Then fill
    // TODO: bottom to top
    for column in area.iter() {
        if world.heightmap(column) < height {
            let soil = &Soil(get_filling_soil(world, column));
            for y in world.heightmap(column)..=height {
                world.set(column.at_height(y), soil)
            }
            *world.heightmap_mut(column) = height;
        }
    }
}

fn get_filling_soil(world: &impl WorldView, column: Column) -> Soil {
    if let Soil(soil) = *world.get(column.at_height(world.heightmap(column))) {
        soil
    } else {
        world.biome(column).default_topsoil()
    }
}

pub fn side_exposted(world: &impl WorldView, pos: Pos) -> bool {
    return !(world.get(pos + Vec2(0, 1)).solid()
        && world.get(pos + Vec2(0, -1)).solid()
        && world.get(pos + Vec2(1, 0)).solid()
        && world.get(pos + Vec2(-1, 0)).solid());
}
