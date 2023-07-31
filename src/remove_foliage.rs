use crate::*;

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

pub fn trees(level: &mut Level, area: impl IntoIterator<Item = IVec2>) {
    for column in area {
        let z = level.height(column) + 1;
        if let Block::Log(..) = level[column.extend(z)] {
            tree(level, column.extend(z));
        }
    }
}

pub fn tree(level: &mut Level, pos: IVec3) {
    let Log(species, ..) = level[pos] else {
        println!("Tried to remove tree at {pos:?} but not found");
        return;
    };
    // Store distance from log, 0 mean log
    let mut blocks = vec![(pos, 0)];
    while let Some((pos, distance)) = blocks.pop() {
        level[pos] = Air;
        for off_x in -1..=1 {
            for off_y in -1..=1 {
                for off_z in -1..=1 {
                    let off = ivec3(off_x, off_y, off_z);
                    let pos = pos + off;
                    match level[pos] {
                        Log(s, ..) if s == species => blocks.push((pos, 0)),
                        // Checking species can leave leaves behind when trees intersect
                        // Also, azalea
                        Leaves(_, Some(d)) if d > distance => blocks.push((pos, d)),
                        // TODO: Beehives
                        // TODO: Snoe
                        _ => (),
                    }
                }
            }
        }
    }
}

// Todo: remove_giant_mushroom()
