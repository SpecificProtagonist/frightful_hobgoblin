use bevy_ecs::prelude::*;
use std::f32::consts::PI;

use crate::{
    replay::Replay,
    sim::{Pos, Tick},
    *,
};

pub fn make_tiny(level: &mut Level, base_pos: IVec3, species: TreeSpecies) {
    let log_block = Log(species, LogType::FullBark);
    let leaf_block = Leaves(species, None);

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

    for leaf_pos in Cuboid::new(pos - IVec3::splat(2), pos + IVec3::splat(2)) {
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
    let leaf_block = Leaves(species, None);

    let log_height = 5 + rand_1(0.5) + rand_1(0.5);

    for z in 1..=log_height {
        level[pos + ivec3(0, 0, z)] = log_block;
    }

    for off in &[ivec2(1, 0), ivec2(-1, 0), ivec2(0, 1), ivec2(0, -1)] {
        for z in 3..=log_height + 1 {
            level[pos + ivec3(off.x, off.y, z)] = leaf_block;
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

// Very basic, but good enough for testing
#[derive(Component)]
pub struct GrowTree {
    species: TreeSpecies,
    stem: Branch,
    previous_stage: Vec<(IVec3, Block)>,
    size: f32,
    last_grown: i32,
}

struct Branch {
    thickness: f32,
    extent: Vec3,
    children: Vec<Branch>,
}

pub fn grow_trees(
    mut level: ResMut<Level>,
    tick: Res<Tick>,
    mut trees: Query<(&Pos, &mut GrowTree)>,
) {
    for (pos, mut tree) in &mut trees {
        if rand_range(0, tick.0 - tree.last_grown) > 100 {
            if tree.size < 4. {
                tree.build(&mut level, pos.0);
                tree.size += 0.2;
            }
            tree.last_grown = tick.0;
        }
    }
}

impl GrowTree {
    pub fn make(species: TreeSpecies, tick: i32) -> Self {
        fn branch(thickness: f32, len: f32, dir: Vec3) -> Branch {
            let extent = dir * len;
            let children = if thickness > 0.2 {
                let ratio = rand_f32(0.25, 0.5);
                let angle = rand_f32(0., PI * 2.);
                let split_dir = vec2(angle.sin(), angle.cos());
                let dir_primary = (dir + split_dir.extend(0.) * (1. - ratio)).normalize();
                let dir_secondary = (dir - split_dir.extend(0.) * ratio).normalize();
                let primary = branch(
                    thickness * (1. - ratio),
                    thickness * (1.5 - ratio) * 1.5,
                    dir_primary,
                );
                let secondary = branch(
                    thickness * ratio,
                    thickness * (1. + ratio) * 2.,
                    dir_secondary,
                );
                vec![primary, secondary]
            } else {
                Vec::new()
            };
            Branch {
                thickness,
                extent,
                children,
            }
        }
        let stem = branch(1., 2., Vec3::Z);
        Self {
            species,
            stem,
            previous_stage: default(),
            size: 0.,
            last_grown: tick,
        }
    }

    pub fn build(&mut self, level: &mut Level, pos: Vec3) {
        if self.size < 0.2 {
            level[pos.block()] = GroundPlant(Sapling(self.species));
            return;
        }
        for (pos, block) in self.previous_stage.drain(..) {
            if level[pos] == block {
                level[pos] = Air
            }
        }
        let cursor = level.recording_cursor();
        fn place(
            level: &mut Level,
            pos: Vec3,
            scale: f32,
            species: TreeSpecies,
            branch: &Branch,
            branch_base: Vec3,
            i: i32,
        ) {
            let start = pos + branch_base * scale;
            let extent = branch.extent * scale;
            if (branch.thickness * scale < 0.2) | branch.children.is_empty() {
                for block_pos in Cuboid::around((start + extent).block(), i - 1) {
                    if block_pos.as_vec3().distance(start + extent) < i as f32 - rand_f32(0.8, 2.) {
                        level[block_pos] |= Leaves(species, None);
                    }
                }
                return;
            }
            let fence = (scale < 0.8) | (i as f32 > scale + 0.25);
            let steps =
                (extent.length() * branch.thickness * if fence { 4. } else { 2. }).round() as i32;

            for step in 0..=steps {
                let block_pos = (start + extent / steps as f32 * step as f32).block();
                if fence {
                    level[block_pos] |= Fence(Wood(species));
                } else {
                    level[block_pos] = Log(species, LogType::FullBark);
                }
            }
            for child in &branch.children {
                place(
                    level,
                    pos,
                    scale,
                    species,
                    child,
                    branch_base + branch.extent,
                    i + 1,
                )
            }
        }
        place(
            level,
            pos,
            self.size,
            self.species,
            &self.stem,
            Vec3::ZERO,
            0,
        );
        self.previous_stage
            .extend(level.get_recording(cursor).map(|r| (r.pos, r.block)));
    }
}
