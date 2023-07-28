use crate::*;

const STUMP_HEIGHT_0_CHANCE: f32 = 0.3;
const STUMP_HEIGHT_2_CHANCE: f32 = 0.2;

pub fn ground(level: &mut Level, area: Rect) {
    for column in area {
        let base_height = if let Some(water_height) = level.water_level(column) {
            water_height
        } else {
            level.height(column)
        };
        for z in base_height + 1..=base_height + 2 {
            let block = &mut level[column.extend(z)];
            if matches!(block, GroundPlant(..)) {
                *block = Block::Air
            }
        }
    }
}

pub fn trees(level: &mut Level, area: impl IntoIterator<Item = IVec2>, leave_stumps: bool) {
    for column in area {
        let z = level.height(column) + 1;
        if let Block::Log(..) = level[column.extend(z)] {
            tree(level, column.extend(z), leave_stumps);
        }
    }
}

/// Removes a tree, optinally leaves a stump behind. This isn't quite perfect, as leaves of the tree may be
/// identified as also belonging to another one, but this is kind of hard and any better solution would involve guesswork.
/// Currently it just removes leaves which would decay without this tree.
/// This function isn't very performant (e.g. >10k blocks checked for a dark oak),
/// but luckily this will be called a few hundred times at most and this isn't Python
pub fn tree(level: &mut Level, pos: IVec3, leave_stump: bool) {
    todo!("Fix this");

    if let Log(species, ..) = level[pos] {
        // Track area of stem for leaf removal
        let mut stem_area = Cuboid { min: pos, max: pos };

        // Remove logs
        {
            // Todo: fix heightmap in case of dark oak roots

            // Make sure the point of the cut is satisfactory
            let pos = if leave_stump {
                // Create visible roots
                // also think of 2x2 trees
                for x in -1..=1 {
                    for y in -1..=1 {
                        let pos = ivec3(pos.x + x, pos.y + y, pos.z);
                        if let Log(s, log_type) = level[pos] {
                            if s == species {
                                level[pos - ivec3(0, 0, 1)] = Log(species, log_type);
                            }
                        }
                    }
                }
                if rand::random::<f32>() < STUMP_HEIGHT_0_CHANCE {
                    pos
                } else {
                    ivec3(pos.x, pos.z, pos.y + 1)
                }
            } else {
                pos
            };

            level[pos] = Block::Air;
            for x in -1..=1 {
                for y in -1..=1 {
                    let block_below = level[ivec3(pos.x + x, pos.y + y, pos.z - 1)];
                    let block = &mut level[ivec3(pos.x + x, pos.y + y, pos.z)];
                    if let Log(s, log_type) = *block {
                        if s == species {
                            // Check block below in case of diagonal branch close to the ground
                            // when leave_stumps and stump height is 1 (mostly happens with dark oak)
                            if matches!(block_below, Block::Log(..))
                                & (rand::random::<f32>() < STUMP_HEIGHT_2_CHANCE)
                            {
                                *block = Log(species, log_type);
                            } else {
                                *block = Air;
                            }
                        }
                    }
                }
            }

            // Finally we can actually remove the log
            let mut log_removed = vec![pos];
            let stump_height = pos.z - 1; // Useful for dark oak roots
            while let Some(pos) = log_removed.pop() {
                stem_area = stem_area.extend_to(pos);
                for x in -1..=1 {
                    for y in -1..=1 {
                        // Check for neighbors in case of horizontal branches (large oaks)
                        for z in (if leave_stump { -1 } else { 0 })..=1 {
                            let pos = pos + ivec3(x, y, z);
                            let block = &mut level[pos];
                            if let Log(s, log_type) = block {
                                if *s == species {
                                    *block = if pos.z <= stump_height {
                                        Log(species, *log_type)
                                    } else {
                                        Air
                                    };
                                    log_removed.push(pos);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Remove leaves that don't belong to another tree
        {
            let decay_distance = 6;

            // Area containing all leaves of the tree
            let removal_area = Cuboid {
                min: stem_area.min - ivec3(decay_distance, decay_distance, 1),
                max: stem_area.max + ivec3(decay_distance, decay_distance, 4),
            };

            // Area in which to check for leaf-log distance (leaves may be shared with another tree)
            let check_area = Cuboid {
                min: removal_area.min - IVec3::splat(decay_distance),
                max: removal_area.max + IVec3::splat(decay_distance),
            };

            let mut blocks = Vec::with_capacity(
                (check_area.size().x * check_area.size().y * check_area.size().z) as usize,
            );

            let index = |pos: IVec3| {
                let offset = pos - check_area.min;
                (offset.x + check_area.size().z * (offset.y + check_area.size().y * offset.z))
                    as usize
            };

            for pos in check_area.iter() {
                blocks.push((level[pos], decay_distance as u8));
            }

            let inner_check_area = Cuboid {
                min: check_area.min + ivec3(1, 1, 1),
                max: check_area.max - ivec3(1, 1, 1),
            };

            for _ in 0..decay_distance {
                for pos in inner_check_area.iter() {
                    if let (Leaves(s), _) = blocks[index(pos)] {
                        if s == species {
                            let surrounds_distance = [
                                pos + ivec3(1, 0, 0),
                                pos + ivec3(-1, 0, 0),
                                pos + ivec3(0, 1, 0),
                                pos + ivec3(0, -1, 0),
                                pos + ivec3(0, 0, 1),
                                pos + ivec3(0, 0, -1),
                            ]
                            .iter()
                            .map(|neightbor: &IVec3| match blocks[index(*neightbor)] {
                                (Log(s, _), _) if s == species => 0,
                                (Leaves(..), distane) => distane,
                                _ => decay_distance as u8,
                            })
                            .min()
                            .unwrap();
                            let distance = &mut blocks[index(pos)].1;
                            *distance = (*distance).min(surrounds_distance + 1);
                        }
                    }
                }
            }

            // Remove identified leaves
            for pos in removal_area.iter() {
                if let (Leaves(s), distance) = blocks[index(pos)] {
                    if (s == species) & (distance == decay_distance as u8) {
                        level[pos] = Air;
                        // Vines (jungle/swamp) helpfully remove themselves
                    }
                }
            }
        }
    }
}

// Todo: remove_giant_mushroom()
