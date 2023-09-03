use crate::{
    remove_foliage::{find_trees, remove_tree},
    roof::roof,
    *,
};
use itertools::Itertools;
use rand::prelude::*;

pub fn build(level: &mut Level, area: Rect) -> Vec<(IVec3, Block)> {
    let mut rng = thread_rng();
    let inner = area.shrink(1);
    let floor = level.average_height(area.border()).round() as i32;

    let cursor = level.recording_cursor();

    for tree in find_trees(level, area.grow(1)) {
        remove_tree(level, tree)
    }

    // Foundations
    for z in (floor + 1..floor + 10).rev() {
        level.fill_at(area, z, Air)
    }
    for col in area {
        for z in (level.height(col)..=floor).rev() {
            level[col.extend(z)] = Air
        }
    }
    let mut rec = level.pop_recording(cursor).collect_vec();
    let cursor = level.recording_cursor();
    for col in area.border() {
        let mut pos = col.extend(floor);
        while level[pos].soil() | !level[pos].solid() {
            level[pos] = Full(Cobble);
            pos -= IVec3::Z;
        }
    }
    for col in inner {
        for z in level.height(col) - 1..=floor {
            let pos = col.extend(z);
            if (!level[pos].solid()) | (level[pos].soil()) {
                level[pos] = PackedMud
            }
        }
    }

    // Ground story

    for z in floor + 1..floor + 3 {
        level.fill_at(area.border(), z, Full(Cobble))
    }

    let door_pos = ivec3(
        rng.gen_range(inner.min.x, inner.max.x),
        area.min.y,
        floor + 1,
    );
    level[door_pos] = Air;
    level[door_pos + IVec3::Z] = Air;
    level[door_pos.add(HDir::YNeg)] = Air;
    level[door_pos.add(HDir::YNeg) + IVec3::Z] = Air;

    let second_floor = floor + 3;

    rec.extend(level.pop_recording(cursor));

    // Roof build now so we know how high the walls have to be
    let cursor = level.recording_cursor();
    roof(level, area.grow(1), second_floor + 3, Wood(Oak));
    let roof_rec = level.pop_recording(cursor).collect_vec();
    let cursor = level.recording_cursor();

    // Second story

    for y in [area.min.y, area.max.y] {
        for x in inner.min.x..=inner.max.x {
            level[ivec3(x, y, second_floor)] = Log(Oak, LogType::Normal(Axis::X))
        }
    }
    for x in [area.min.x, area.max.x] {
        for y in inner.min.y..=inner.max.y {
            level[ivec3(x, y, second_floor)] = Log(Oak, LogType::Normal(Axis::Y))
        }
    }

    level.fill_at(inner, second_floor, Slab(Wood(Oak), Top));

    let mut roof_fixup = Vec::new();
    // TODO: Instead return roof height from roof function
    // to avoid issues if another roof is poking in
    let mut column_till_roof = |level: &mut Level, col: IVec2, block: Block| {
        for z in second_floor.. {
            let pos = col.extend(z);
            match level[pos] {
                Log(..) => (),
                Full(..) | Slab(_, Bottom) | Stair(_, _, Bottom) => return,
                Slab(..) | Stair(..) => {
                    roof_fixup.push(pos);
                    return;
                }
                _ => level[pos] = block,
            }
        }
    };

    for pos in area.corners() {
        column_till_roof(level, pos, Log(Oak, LogType::Normal(Axis::Z)))
    }

    // Wattle
    for pos in area.border() {
        column_till_roof(level, pos, MangroveRoots);
    }

    rec.extend(level.pop_recording(cursor));
    rec.extend(roof_rec);

    let cursor = level.recording_cursor();
    level.fill(roof_fixup, Full(Wood(Oak)));

    // Daub
    'outer: for pos in area.border() {
        for z in second_floor + 1.. {
            let pos = pos.extend(z);
            match level[pos] {
                MangroveRoots => level[pos] = MuddyMangroveRoots,
                _ => continue 'outer,
            }
        }
    }

    rec.extend(level.pop_recording(cursor));
    let cursor = level.recording_cursor();

    // Whitewash
    'outer: for pos in area.border() {
        for z in second_floor + 1.. {
            let pos = pos.extend(z);
            match level[pos] {
                MuddyMangroveRoots => level[pos] = MushroomStem,
                _ => continue 'outer,
            }
        }
    }

    rec.extend(level.pop_recording(cursor));
    rec
}
