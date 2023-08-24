use rand::{seq::SliceRandom, thread_rng, Rng};

use crate::*;

pub fn house(level: &mut Level, outer: Cuboid) {
    let mut rng = thread_rng();
    let inner = outer.shrink(1);
    // Clear space
    level.fill(inner, Air);
    // Floor
    level.fill_at(inner.d2().grow(2), outer.min.z, PackedMud);
    // Walls
    for pos in outer.d2().border() {
        wall_column(level, pos, outer.min.z, outer.max.z - 1);
    }
    // Corners
    level.fill_at(
        outer.d2().corners(),
        outer.min.z..=outer.max.z,
        Full(MudBrick),
    );

    // Roof
    level.fill_at(outer.d2(), outer.max.z, Full(MudBrick));

    let door_pos = ivec2(rng.gen_range(inner.min.x, inner.max.x), outer.min.y);
    level[door_pos.extend(inner.min.z)] = Door(Oak, YPos, DoorMeta::empty());
    level[door_pos.extend(inner.min.z + 1)] = Door(Oak, YPos, DoorMeta::TOP);

    let mut roof_access = false;
    if rng.gen_bool(0.7) {
        roof_access = true;
        let ladder_pos = *inner
            .d2()
            .border()
            .filter(|p| !p.touch_face(door_pos))
            .collect::<Vec<_>>()
            .choose(&mut rng)
            .unwrap();
        let dir = wall_dir(level, ladder_pos.extend(inner.min.z)).rotated(2);
        level.fill_at(Some(ladder_pos), inner.min.z..=outer.max.z, Ladder(dir));
    }
    if roof_access {
        // Crenellation
        for (i, x) in (inner.min.x..=inner.max.x).enumerate() {
            for y in [outer.min.y, outer.max.y] {
                level[ivec3(x, y, outer.max.z + 1)] = crenel(inner.size().x, i as i32);
            }
        }
        for (i, y) in (inner.min.y..=inner.max.y).enumerate() {
            for x in [outer.min.x, outer.max.x] {
                level[ivec3(x, y, outer.max.z + 1)] = crenel(inner.size().y, i as i32).rotated(1);
            }
        }
        level.fill_at(outer.d2().corners(), outer.max.z + 1, Full(MudBrick));
    } else {
        // Flat wooden roof
        if rng.gen_bool(0.5) {
            level.fill_at(
                inner.d2().grow2(if outer.size().x > outer.size().y {
                    IVec2::X
                } else {
                    IVec2::Y
                }),
                outer.max.z + 1,
                Slab(Wood(Oak), Bottom),
            );
            if rng.gen_bool(0.5) {
                level.fill_at(outer.d2().corners(), outer.max.z + 1, Full(MudBrick));
            }
        } else {
            level.fill_at(inner.d2(), outer.max.z, Slab(Wood(Oak), Bottom));
            if rng.gen_bool(0.5) {
                level.fill_at(
                    outer.d2().corners(),
                    outer.max.z + 1,
                    Slab(MudBrick, Bottom),
                );
            }
        }
    }
}

/// Default direction: XPos
fn crenel(width: i32, i: i32) -> Block {
    if width % 2 == 0 {
        if i % 2 == 0 {
            Stair(MudBrick, XPos, Bottom)
        } else {
            Stair(MudBrick, XNeg, Bottom)
        }
    } else {
        if (width / 2) % 3 == 2 {
            if i == 0 {
                return Stair(MudBrick, XPos, Bottom);
            } else if i == width - 1 {
                return Stair(MudBrick, XNeg, Bottom);
            }
        }
        [
            Stair(MudBrick, XPos, Bottom),
            Slab(MudBrick, Bottom),
            Stair(MudBrick, XNeg, Bottom),
        ][(width + i) as usize % 3]
    }
}

/// z_max inclusive
fn wall_column(level: &mut Level, column: IVec2, z_min: i32, z_max: i32) {
    let mut rng = thread_rng();
    let offset = rng.gen_range(-0.2, 0.2);
    for z in z_min..=z_max {
        let rel_height = (z - z_min) as f32 / (z_max + 1 - z_min) as f32;
        let block = if rel_height + offset + rng.gen_range(-0.3, 0.3) > 0.6 {
            Full(Wood(Birch))
        } else {
            Log(Birch, LogType::Stripped(Axis::Z))
        };
        level[column.extend(z)] = block;
    }
}

fn wall_dir(level: &Level, pos: IVec3) -> HDir {
    let mut rng = thread_rng();
    let mut count = 0;
    for dir in HDir::ALL {
        if level[pos.add(dir)].solid() {
            count += 1
        }
    }
    if count == 0 {
        return *HDir::ALL.choose(&mut rng).unwrap();
    }
    for dir in HDir::ALL {
        if level[pos.add(dir)].solid() {
            if rng.gen_range(0, count) == 0 {
                return dir;
            }
            count -= 1
        }
    }
    unreachable!()
}
