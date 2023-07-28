use crate::geometry::*;
use crate::level::*;

pub fn make_tiny(level: &mut Level, base_pos: IVec3, species: TreeSpecies) {
    let log_block = Log(species, LogType::FullBark);
    let leaf_block = Leaves(species);

    let base_pos = base_pos + ivec3(0, 0, 1);
    level[base_pos] = log_block;

    let mut pos = base_pos + ivec3(0, 0, 1) + rand_2(0.2).extend(0);
    level[pos] = log_block;

    pos.x += 1;
    if rand(0.8) {
        if pos.truncate() == base_pos.truncate() {
            pos += rand_2(0.3).extend(0);
        }
        level[pos] = log_block;

        if rand(0.2) {
            pos.z += 1;
            level[pos] = log_block;
        }
    }

    level[pos + ivec3(1, 0, 0)] |= leaf_block;
    level[pos + ivec3(-1, 0, 0)] |= leaf_block;
    level[pos + ivec3(0, 1, 0)] |= leaf_block;
    level[pos + ivec3(0, -1, 0)] |= leaf_block;
    level[pos + ivec3(0, 0, 1)] |= leaf_block;
    level[pos + ivec3(0, 0, -1)] |= leaf_block;

    for leaf_pos in Cuboid::new(pos - IVec3::splat(2), pos + IVec3::splat(2)).iter() {
        let distance_squared = ((leaf_pos - pos).x * (leaf_pos - pos).x
            + (leaf_pos - pos).z * (leaf_pos - pos).z
            + (leaf_pos - pos).y * (leaf_pos - pos).y) as f32;
        if rand(1.0 - (distance_squared / 3.0)) {
            level[leaf_pos] |= leaf_block;
        }
    }
}

pub fn make_straight(level: &mut Level, pos: IVec3, species: TreeSpecies) {
    let log_block = Log(species, LogType::FullBark);
    let leaf_block = Leaves(species);

    let log_height = 5 + rand_1(0.5) + rand_1(0.5);

    for z in 1..=log_height {
        level[pos + ivec3(0, 0, z)] = log_block;
    }

    for off in &[ivec2(1, 0), ivec2(-1, 0), ivec2(0, 1), ivec2(0, -1)] {
        for y in 3..=log_height + 1 {
            level[pos + ivec3(off.x, y, off.y)] = leaf_block;
        }
    }

    level[pos + ivec3(0, 0, log_height + 1)] |= leaf_block;
    level[pos + ivec3(0, 0, log_height + 2)] |= leaf_block;
    if (log_height == 5) & rand(0.75) | (log_height > 5) {
        level[pos + ivec3(0, 0, log_height + 3)] |= leaf_block;
    }

    for off in &[ivec2(1, 1), ivec2(-1, 1), ivec2(1, -1), ivec2(-1, -1)] {
        for z in 4 + rand_1(0.5)..=log_height - 1 + rand_1(0.5) {
            level[pos + ivec3(off.x, off.y, z)] |= leaf_block;
        }
    }
}
