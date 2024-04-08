use crate::*;
use roof::roof;

use self::{
    remove_foliage::remove_trees,
    sim::{logistics::MoveTask, ConsItem, ConsList},
};

pub fn house(level: &mut Level, area: Rect) -> ConsList {
    let inner = area.shrink(1);

    let (floor, mut rec) = foundation(level, area);

    let cursor = level.recording_cursor();

    // Ground story
    for z in floor + 1..floor + 3 {
        level.fill_at(area.border(), z, Full(Cobble))
    }

    let door_pos = ivec3(rand_range(inner.min.x..=inner.max.x), area.min.y, floor + 1);
    level(door_pos, Air);
    level(door_pos + IVec3::Z, Air);
    level(door_pos.add(HDir::YNeg), Air);
    level(door_pos.add(HDir::YNeg) + IVec3::Z, Air);

    let second_floor = floor + 3;

    // Roof build now so we know how high the walls have to be
    let roof_mat = if 0.3 > rand() {
        Blackstone
    } else if 0.1 > rand() {
        Wood(DarkOak)
    } else if 0.1 > rand() {
        Wood(Mangrove)
    } else if 0.1 > rand() {
        Wood(Birch)
    } else {
        Wood(Spruce)
    };
    let roof_rec = roof(level, area.grow(1), second_floor + 3, roof_mat);

    // Second story

    for y in [area.min.y, area.max.y] {
        for x in inner.min.x..=inner.max.x {
            level(
                ivec3(x, y, second_floor),
                Log(Oak, LogType::Normal(Axis::X)),
            )
        }
    }
    for x in [area.min.x, area.max.x] {
        for y in inner.min.y..=inner.max.y {
            level(
                ivec3(x, y, second_floor),
                Log(Oak, LogType::Normal(Axis::Y)),
            )
        }
    }

    level.fill_at(inner, second_floor, Slab(Wood(Oak), Top));

    let mut roof_fixup = Vec::new();
    // TODO: Instead return roof height from roof function
    // to avoid issues if another roof is poking in
    let mut column_till_roof = |level: &mut Level, col: IVec2, block: Block| {
        for z in second_floor.. {
            let pos = col.extend(z);
            match level(pos) {
                Log(..) => (),
                Full(..) | Slab(_, Bottom) | Stair(_, _, Bottom) => return,
                Slab(..) | Stair(..) => {
                    roof_fixup.push(pos);
                    return;
                }
                _ => level(pos, block),
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

    rec.extend(level.pop_recording(cursor).map(ConsItem::Set));
    rec.extend(roof_rec);

    // Some movement
    for i in 0..rec.len() {
        if 0.06 > rand() {
            rec.insert(
                i,
                ConsItem::Goto(MoveTask::new(ivec3(
                    rand_range(inner.min.x..=inner.max.x),
                    rand_range(inner.min.y..=inner.max.y),
                    floor + 1,
                ))),
            );
        }
    }

    let cursor = level.recording_cursor();
    level.fill(roof_fixup, Full(roof_mat));

    // Daub
    'outer: for pos in area.border() {
        for z in second_floor + 1.. {
            let pos = pos.extend(z);
            match level(pos) {
                MangroveRoots => level(pos, MuddyMangroveRoots),
                _ => continue 'outer,
            }
        }
    }

    rec.extend(level.pop_recording(cursor).map(ConsItem::Set));
    let cursor = level.recording_cursor();

    // Paint/Whitewash
    let paint = if 0.5 > rand() {
        MushroomStem
    } else {
        *[
            Terracotta(Some(White)),
            Terracotta(Some(Red)),
            Terracotta(Some(Orange)),
            Terracotta(Some(Lime)),
            Terracotta(Some(Green)),
            Terracotta(Some(LightBlue)),
            Terracotta(Some(Magenta)),
            Terracotta(Some(Pink)),
        ]
        .choose()
    };
    'outer: for pos in area.border() {
        for z in second_floor + 1.. {
            let pos = pos.extend(z);
            match level(pos) {
                MuddyMangroveRoots => level(pos, paint),
                _ => continue 'outer,
            }
        }
    }

    rec.extend(level.pop_recording(cursor).map(ConsItem::Set));
    rec
}

pub fn shack(level: &mut Level, area: Rect) -> ConsList {
    let (floor, mut rec) = foundation(level, area);

    // Roof build now so we know how high the walls have to be
    let roof_rec = roof(level, area.grow(1), floor + 3, Wood(Oak));

    let cursor = level.recording_cursor();
    let mut roof_fixup = Vec::new();
    // TODO: Instead return roof height from roof function
    // to avoid issues if another roof is poking in
    let mut column_till_roof = |level: &mut Level, col: IVec2, block: Block| {
        for z in floor + 1.. {
            let pos = col.extend(z);
            match level(pos) {
                Log(..) => (),
                Full(..) | Slab(_, Bottom) | Stair(_, _, Bottom) => return,
                Slab(..) | Stair(..) => {
                    roof_fixup.push(pos);
                    return;
                }
                _ => level(pos, block),
            }
        }
    };

    let wall_mat = if rand() { Cobble } else { Wood(Oak) };

    if let Wood(_) = wall_mat {
        for pos in area.corners() {
            column_till_roof(level, pos, Log(Oak, LogType::Normal(Axis::Z)))
        }
    }

    for pos in area.border() {
        column_till_roof(level, pos, Full(wall_mat));
    }

    rec.extend(level.pop_recording(cursor).map(ConsItem::Set));
    rec.extend(roof_rec);

    let cursor = level.recording_cursor();
    level.fill(roof_fixup, Full(Wood(Oak)));

    rec.extend(level.pop_recording(cursor).map(ConsItem::Set));
    rec
}

fn foundation(level: &mut Level, area: Rect) -> (i32, ConsList) {
    let floor = level.average_height(area.border()).round() as i32;

    let cursor = level.recording_cursor();
    remove_trees(level, area.grow(1));

    for z in (floor + 1..floor + 10).rev() {
        level.fill_at(area, z, Air)
    }
    for col in area {
        for z in (level.height[col] + 1..=floor).rev() {
            level(col, z, Air)
        }
    }
    let mut rec: ConsList = level.pop_recording(cursor).map(ConsItem::Set).collect();
    let cursor = level.recording_cursor();
    for col in area.border() {
        // TODO: if ground is too far down, try to make supports against the nearest wall instead
        let mut pos = col.extend(floor);
        loop {
            if level(pos).solid() & !level(pos).soil() {
                break;
            }
            level(pos, Full(Cobble));
            if NEIGHBORS_2D.iter().all(|dir| level(pos.add(*dir)).solid()) {
                break;
            }
            pos -= IVec3::Z;
        }
    }
    for col in area.shrink(1) {
        for z in (level.height[col] - 1).min(floor)..=floor {
            let pos = col.extend(z);
            if (!level(pos).solid()) | (level(pos).soil()) {
                level(pos, PackedMud)
            }
        }
    }
    rec.extend(level.pop_recording(cursor).map(ConsItem::Set));

    (floor, rec)
}
