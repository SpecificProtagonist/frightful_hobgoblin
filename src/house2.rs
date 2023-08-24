use crate::*;
use rand::prelude::*;

fn build(level: &mut Level, area: Rect) {
    let mut rng = thread_rng();
    let inner = area.shrink(1);
    let floor = level.average_height(area).round() as i32;

    // Foundations
    for z in (1..4).rev() {
        level.fill_at(area, z, Air)
    }
    for col in area {
        for z in (level.height(col)..=floor).rev() {
            level[col.extend(z)] = Air
        }
    }
    for col in area.border() {
        let mut pos = col.extend(level.height(col).min(floor));
        while level[pos].soil() {
            level[pos] = Full(Cobble);
            pos -= IVec3::Z;
        }
    }
    for col in inner {
        for z in level.height(col)..=floor {
            level[col.extend(z)] |= PackedMud
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
    level[door_pos] = Air;
    level[door_pos] = Air;
}
