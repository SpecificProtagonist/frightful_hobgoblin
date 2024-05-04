use crate::*;
use itertools::Itertools;
use roof::roof;

use self::{
    desire_lines::{add_desire_line, DesireLines},
    pathfind::pathfind_street,
    roof::roof_shape,
    sim::{logistics::MoveTask, ConsItem, ConsList},
};

pub fn house(level: &mut Level, dl: &mut DesireLines, untree: &mut Untree, area: Rect) -> ConsList {
    (level.blocked)(area, Free);
    let path = pathfind_street(level, area);
    for node in path.path {
        for (x_off, y_off) in (-1..=1).cartesian_product(-1..=1) {
            level.blocked[node.pos.truncate() + ivec2(x_off, y_off)] = Street;
        }
        for _ in 0..30 {
            add_desire_line(level, dl, node.pos - IVec3::Z);
        }
    }
    (level.blocked)(area, Blocked);

    let inner = area.shrink(1);
    let biome = level.biome[area.center()];

    let (floor, mut rec) = foundation(level, untree, area);

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
    let roof_z = second_floor + 3;
    let roof_area = area.grow(1);
    let roof_shape = roof_shape(biome, roof_z, roof_area);

    // Second story

    let species = biome.random_tree_species();
    for y in [area.min.y, area.max.y] {
        for x in inner.min.x..=inner.max.x {
            level(
                ivec3(x, y, second_floor),
                Log(species, LogType::Normal(Axis::X)),
            )
        }
    }
    for x in [area.min.x, area.max.x] {
        for y in inner.min.y..=inner.max.y {
            level(
                ivec3(x, y, second_floor),
                Log(species, LogType::Normal(Axis::Y)),
            )
        }
    }

    level.fill_at(inner, second_floor, Slab(Wood(Oak), Top));

    for column in area.corners() {
        for z in second_floor..=(roof_shape(column.as_vec2()) - 0.5) as i32 {
            level(column, z, Log(species, LogType::Normal(Axis::Z)))
        }
    }

    // Wattle
    let mut wattle = Vec::new();
    for column in area.border() {
        for z in second_floor + 1..=(roof_shape(column.as_vec2()) - 0.5) as i32 {
            let pos = column.extend(z);
            if !level(pos).solid() {
                wattle.push(pos);
            }
        }
    }
    wattle.sort_by_key(|p| p.z);
    level.fill(&wattle, MangroveRoots);

    rec.extend(level.pop_recording(cursor).map(ConsItem::Set));
    let roof_rec = roof(level, roof_area, roof_z, &roof_shape, roof::palette(biome));
    wattle.retain(|p| level(p) == MangroveRoots);
    rec.extend(roof_rec);

    // Some movement
    for i in 0..rec.len() {
        if 0.03 > rand() {
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

    // Daub
    level.fill(&wattle, MuddyMangroveRoots);

    rec.extend(level.pop_recording(cursor).map(ConsItem::Set));
    let cursor = level.recording_cursor();

    // Paint/Whitewash
    let paint = rand_weighted(&[
        (8., MushroomStem),
        (1., Terracotta(Some(White))),
        (1., Terracotta(Some(Red))),
        (1., Terracotta(Some(Orange))),
        (1., Terracotta(Some(Lime))),
        (1., Terracotta(Some(Green))),
        (1., Terracotta(Some(LightBlue))),
        (1., Terracotta(Some(Magenta))),
        (1., Terracotta(Some(Pink))),
    ]);
    level.fill(&wattle, paint);

    rec.extend(level.pop_recording(cursor).map(ConsItem::Set));
    rec
}

pub fn shack(level: &mut Level, untree: &mut Untree, area: Rect) -> ConsList {
    let (floor, mut rec) = foundation(level, untree, area);

    let biome = level.biome[area.center()];
    let species = biome.random_tree_species();

    // Roof build now so we know how high the walls have to be
    let roof_z = floor + 3;
    let roof_area = area.grow(1);
    let roof_shape = roof_shape(biome, roof_z, roof_area);
    let roof_rec = roof(level, roof_area, roof_z, &roof_shape, roof::palette(biome));

    let cursor = level.recording_cursor();

    let wall_mat = if rand() { Cobble } else { Wood(Oak) };

    if let Wood(_) = wall_mat {
        for column in area.corners() {
            for z in floor..=(roof_shape(column.as_vec2()) - 0.5) as i32 {
                level(column, z, Log(species, LogType::Normal(Axis::Z)))
            }
        }
    }

    for column in area.border() {
        for z in floor + 1..=(roof_shape(column.as_vec2()) - 0.5) as i32 {
            level(column.extend(z), |b| b | Full(wall_mat))
        }
    }

    rec.extend(level.pop_recording(cursor).map(ConsItem::Set));
    rec.extend(roof_rec);

    let cursor = level.recording_cursor();

    rec.extend(level.pop_recording(cursor).map(ConsItem::Set));
    rec
}

fn foundation(level: &mut Level, untree: &mut Untree, area: Rect) -> (i32, ConsList) {
    let floor = level.height.average(area.border()).round() as i32;

    let cursor = level.recording_cursor();
    untree.remove_trees(level, area.grow(1));

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
        while level(pos).soil() | !level(pos).soil() {
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
