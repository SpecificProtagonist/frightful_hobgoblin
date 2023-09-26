use bevy_ecs::prelude::*;
use std::f32::consts::PI;

use crate::{
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
    if 0.8 > rand() {
        if pos.truncate() == base_pos.truncate() {
            pos += rand_2(0.3).extend(0);
        }
        level[pos] = log_block;

        if 0.2 > rand() {
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
        if 1.0 - (distance_squared / 3.0) > rand() {
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
    if (log_height == 5) & (0.75 > rand()) | (log_height > 5) {
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
    pub blocks: Vec<(IVec3, Block)>,
    pub size: f32,
    stem: Branch,
    last_grown: i32,
    params: TreeParams,
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
        if rand_range(0..(tick.0 - tree.last_grown).max(1)) > 100 {
            if tree.size < 4. {
                tree.build(&mut level, pos.0);
                tree.size += rand_f32(0.13, 0.25);
            }
            tree.last_grown = tick.0;
        }
    }
}

struct TreeParams {
    species: TreeSpecies,
    leaf_z_factor: f32,
    stem_thickness: f32,
    stem_len: f32,
}

impl GrowTree {
    pub fn oak() -> Self {
        Self::make(TreeParams {
            species: Oak,
            leaf_z_factor: 1.,
            stem_thickness: 1.,
            stem_len: 2.,
        })
    }

    pub fn birch() -> Self {
        Self::make(TreeParams {
            species: Birch,
            leaf_z_factor: 1.3,
            stem_thickness: 1.,
            stem_len: 2.,
        })
    }

    pub fn pine() -> Self {
        Self::make(TreeParams {
            species: Spruce,
            leaf_z_factor: 0.4,
            stem_thickness: 0.7,
            stem_len: 1.,
        })
    }

    fn make(params: TreeParams) -> Self {
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
        let stem = branch(params.stem_thickness, params.stem_len, Vec3::Z);
        Self {
            stem,
            blocks: default(),
            size: -0.1,
            last_grown: 0,
            params,
        }
    }

    pub fn build(&mut self, level: &mut Level, pos: Vec3) {
        if self.size < 0.25 {
            level[pos.block()] = GroundPlant(Sapling(self.params.species));
            return;
        }
        for (pos, block) in self.blocks.drain(..) {
            if level[pos] == block {
                level[pos] = Air
            }
        }
        let cursor = level.recording_cursor();
        fn place(
            params: &TreeParams,
            level: &mut Level,
            pos: Vec3,
            scale: f32,
            branch: &Branch,
            branch_base: Vec3,
            i: i32,
        ) {
            let start = pos + branch_base * scale;
            let extent = branch.extent * scale;
            if (branch.thickness * scale < 0.2) | branch.children.is_empty() {
                for block_pos in Cuboid::around((start + extent).block(), i) {
                    let mut diff = block_pos.as_vec3() - (start + extent);
                    diff.z /= params.leaf_z_factor;
                    if diff.length() < i as f32 - rand_f32(0.8, 2.) {
                        level[block_pos] |= Leaves(params.species, None);
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
                    level[block_pos] |= Fence(Wood(params.species));
                } else {
                    level[block_pos] = Log(params.species, LogType::FullBark);
                }
            }
            for child in &branch.children {
                place(
                    params,
                    level,
                    pos,
                    scale,
                    child,
                    branch_base + branch.extent,
                    i + 1,
                )
            }
        }
        place(
            &self.params,
            level,
            pos,
            self.size,
            &self.stem,
            Vec3::ZERO,
            0,
        );
        self.blocks
            .extend(level.get_recording(cursor).map(|r| (r.pos, r.block)));
    }
}
