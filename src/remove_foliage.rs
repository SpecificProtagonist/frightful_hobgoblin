use crate::*;

pub fn ground(level: &mut Level, area: Rect) {
    for column in area {
        let base_height = if let Some(water_height) = level.water[column] {
            water_height
        } else {
            level.height[column]
        };
        for z in base_height + 1..=base_height + 2 {
            level(column, z, |block| {
                if matches!(block, GroundPlant(..)) {
                    Block::Air
                } else {
                    block
                }
            })
        }
    }
}

pub fn find_minecraft_trees(
    level: &Level,
    area: impl IntoIterator<Item = IVec2>,
) -> Vec<(IVec3, TreeSpecies)> {
    let mut trees = HashSet::default();
    for column in area {
        let z = level.height[column] + 1;
        if let Block::Log(species, _) = level(column.extend(z)) {
            // Check whether this is a tree instead of part of a man-made structure
            let mut pos = column.extend(z);
            while let Block::Log(..) = level(pos) {
                pos += IVec3::Z;
            }
            if !matches!(level(pos), Leaves(..)) {
                continue;
            }
            // Find origin
            // TODO: find connected blocks to make this work for all kinds of trees
            let mut pos = column.extend(z);
            if let Block::Log(..) = level(pos - IVec3::X) {
                pos -= IVec3::X
            }
            if let Block::Log(..) = level(pos - IVec3::Y) {
                pos -= IVec3::Y
            }
            trees.insert((pos, species));
        }
    }
    trees.into_iter().collect()
}

// TODO: Also remove tree entities & custom trees
// Passing the necessary systemparams around everywhere is kind of annoying
// maybe a custom systemparam is enough?
// or maybe add a clear area task before building is generated?
pub fn remove_tree(
    // mut commands: Commands,
    // mut trees: ResMut<Trees>,
    level: &mut Level,
    pos: IVec3,
) {
    // let Some(entity) = trees[pos] else {
    //     println!("Tried to remove tree at {pos:?} but no entity");
    //     return;
    // };
    // trees[pos] = None;
    // commands.entity(entity).despawn();

    // TODO: Remove custom trees (GrowTree)

    // Handle normal Minecraft trees:

    // let pos = level.ground(pos) + IVec3::Z;
    let species = match level(pos) {
        Log(s, ..) => s,
        _ => return,
    };
    // Store distance from log, 0 means log
    let mut blocks = vec![(pos, 0)];
    while let Some((pos, distance)) = blocks.pop() {
        level(pos, Air);
        for off_x in -1..=1 {
            for off_y in -1..=1 {
                for off_z in -1..=1 {
                    let off = ivec3(off_x, off_y, off_z);
                    let pos = pos + off;
                    match level(pos) {
                        Log(s, ..) if (s == species) & (distance <= 1) => blocks.push((pos, 0)),
                        // Checking species can leave leaves behind when trees intersect
                        // Also, azalea
                        Leaves(_, Some(d)) if (d > distance) & (off.length_squared() == 1) => {
                            blocks.push((pos, d))
                        }
                        // TODO: Beehives
                        // TODO: Snoe
                        _ => (),
                    }
                }
            }
        }
    }
}

pub fn remove_trees(level: &mut Level, area: impl IntoIterator<Item = IVec2>) {
    for (pos, _) in find_minecraft_trees(level, area) {
        remove_tree(level, pos)
    }
}

// Todo: remove_giant_mushroom()
