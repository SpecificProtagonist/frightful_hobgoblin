use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, system::SystemParam};
use std::f32::consts::PI;

use crate::{
    sim::{Pos, Tick},
    *,
};

use self::desire_lines::DesireLines;

#[derive(Component)]
pub struct Tree {
    pub blocks: Vec<(IVec3, Block)>,
    pub state: TreeState,
}

#[derive(Eq, PartialEq)]
pub enum TreeState {
    Young,
    Ready,
    MarkedForChoppage,
    Decorative,
}

#[derive(Resource, Deref, DerefMut)]
pub struct Trees(ColumnMap<Option<Entity>>);

#[derive(Resource, Deref, DerefMut)]
pub struct TreeNoise(ColumnMap<f32, 2>);

pub fn init_trees(mut commands: Commands, level: Res<Level>) {
    let mut noise = level.column_map(0.);
    let mut trees = level.column_map(None);

    // Make some noise!
    for column in noise.cells() {
        noise[column] = rand(0. ..1.);
    }

    // Find vanilla trees
    let mut found = HashSet::<IVec3>::default();
    for column in level.area() {
        let pos = level.ground(column) + IVec3::Z;
        if let Block::Log(species, LogType::Normal, Axis::Z) = level(pos) {
            // Check whether this is a tree instead of part of a man-made structure
            let mut check = pos;
            while let Block::Log(..) = level(check) {
                check += IVec3::Z;
            }
            if !matches!(level(check), Leaves(..)) {
                continue;
            }

            // Find all blocks
            // Store distance from log, 0 means log
            let mut blocks: Vec<(IVec3, Block)> = Vec::new();
            let mut to_check = vec![(pos, 0)];
            while let Some((pos, distance)) = to_check.pop() {
                found.insert(pos);
                blocks.push((pos, level(pos)));
                for off_x in -1..=1 {
                    for off_y in -1..=1 {
                        for off_z in -1..=1 {
                            let off = ivec3(off_x, off_y, off_z);
                            let pos = pos + off;
                            if found.contains(&pos) {
                                continue;
                            }
                            match level(pos) {
                                Log(s, ..) if (s == species) & (distance <= 1) => {
                                    to_check.push((pos, 0))
                                }
                                // Checking species can leave leaves behind when trees intersect
                                // Also, azalea
                                Leaves(_, Some(d))
                                    if (d > distance) & (off.length_squared() == 1) =>
                                {
                                    to_check.push((pos, d))
                                }
                                // TODO: Beehives
                                // TODO: Snoe
                                _ => (),
                            }
                        }
                    }
                }
            }

            trees[pos] = Some(
                commands
                    .spawn((
                        Pos(pos.as_vec3()),
                        Tree {
                            blocks,
                            state: TreeState::Ready,
                        },
                    ))
                    .id(),
            );
            noise[pos] = 1.;
        }
    }

    commands.insert_resource(Trees(trees));
    commands.insert_resource(TreeNoise(noise));
}

pub fn spawn_trees(
    mut commands: Commands,
    noise: ResMut<TreeNoise>,
    level: Res<Level>,
    tick: Res<Tick>,
    dl: Res<DesireLines>,
    mut tree_map: ResMut<Trees>,
) {
    if (tick.0 % 250) != 1 {
        return;
    }

    'outer: for column in noise.cells() {
        if !level.area().contains(column) {
            continue;
        }

        // Check whether the tree has already been placed
        if tree_map[column].is_some() {
            continue;
        }

        // Stagger spawn
        if rand(0.98) {
            continue;
        }

        // Don't grow on paths
        if NEIGHBORS_2D
            .iter()
            .map(|&off| dl[column + off])
            .max()
            .unwrap()
            > 8
        {
            continue;
        }

        let ground = level.ground(column);
        if (level.blocked[column] != Free)
            | level.water[column].is_some()
            | !level(ground).dirtsoil()
        {
            continue;
        }

        // Check for local maximum
        for x_off in -2..=1 {
            for y_off in -2..=1 {
                if noise[column + ivec2(x_off, y_off)] > noise[column] {
                    continue 'outer;
                }
            }
        }

        use Biome::*;
        let chance = match level.biome[column] {
            Plain | Ocean | Beach | Mesa | Savanna => 0.3,
            River => 0.6,
            Snowy => 0.,
            Desert => 0.05,
            Forest | Taiga | BirchForest | DarkForest | CherryGrove => 0.75,
            Swamp | MangroveSwamp => 0.8,
            Jungles => 1.,
        };
        let kind: &[_] = match level.biome[column] {
            Plain | River | Ocean | Beach | Forest | Swamp | MangroveSwamp => &[
                (1., TreeGen::Oak),
                (0.7, TreeGen::Pine),
                (0.4, TreeGen::Birch),
                (0.2, TreeGen::Cherry),
            ],
            Savanna => &[
                (1., TreeGen::Oak),
                (0.7, TreeGen::Cypress),
                (0.4, TreeGen::Birch),
            ],
            Taiga | Snowy => &[
                (1., TreeGen::Pine),
                (0.4, TreeGen::Birch),
                (0.2, TreeGen::Oak),
            ],
            Desert => &[
                (1., TreeGen::Cypress),
                (0.3, TreeGen::Oak),
                (0.3, TreeGen::Cherry),
            ],
            BirchForest => &[(1., TreeGen::Birch)],
            Jungles => &[(1., TreeGen::Jungle), (0.3, TreeGen::Oak)],
            Mesa => &[
                (1., TreeGen::Pine),
                (0.3, TreeGen::Cypress),
                (0.5, TreeGen::Oak),
            ],
            DarkForest => &[(1., TreeGen::Oak)],
            CherryGrove => &[(1.0, TreeGen::Cherry), (0.2, TreeGen::Birch)],
        };

        if chance < noise[column] {
            continue;
        }

        tree_map[column] = Some(
            commands
                .spawn((
                    Pos((ground + IVec3::Z).as_vec3()),
                    Tree {
                        blocks: default(),
                        state: TreeState::Young,
                    },
                    rand_weighted(kind).make(),
                ))
                .id(),
        );
    }
}

pub fn grow_trees(
    mut level: ResMut<Level>,
    tick: Res<Tick>,
    mut trees: Query<(&Pos, &mut Tree, &mut GrowTree)>,
) {
    for (pos, mut tree, mut grow) in &mut trees {
        if rand(0..=(tick.0 - grow.last_grown).max(1)) > 500 {
            if rand(grow.size..grow.max_size + 2.) < grow.max_size {
                grow.build(&mut level, pos.0, &mut tree.blocks);
                if (grow.size > 1.3) & (tree.state == TreeState::Young) {
                    tree.state = TreeState::Ready;
                }
                grow.size += rand(0.13..0.25);
            }
            grow.last_grown = tick.0;
        }
    }
}

#[derive(Copy, Clone)]
pub enum TreeGen {
    Oak,
    Birch,
    Pine,
    Cherry,
    Jungle,
    Cypress,
}

impl TreeGen {
    pub fn make(self) -> GrowTree {
        match self {
            Self::Oak => basic_tree(BasicParams {
                species: Oak,
                max_size: rand(1.5..2.0) * rand(1.1..2.2),
                leaf_z_factor: 1.,
                stem_thickness: 1.,
                stem_len: 2.,
            }),
            Self::Birch => basic_tree(BasicParams {
                species: Birch,
                max_size: rand(1.5..2.0) * rand(1.0..2.0),
                leaf_z_factor: 1.3,
                stem_thickness: 1.,
                stem_len: 2.,
            }),
            Self::Pine => basic_tree(BasicParams {
                species: Spruce,
                max_size: rand(1.5..2.0) * rand(1.0..1.8),
                leaf_z_factor: 0.4,
                stem_thickness: 0.7,
                stem_len: 1.,
            }),
            Self::Cherry => basic_tree(BasicParams {
                species: Cherry,
                max_size: rand(1.5..2.0) * rand(1.1..2.2),
                leaf_z_factor: 1.,
                stem_thickness: 1.,
                stem_len: 2.,
            }),
            Self::Jungle => basic_tree(BasicParams {
                species: Jungle,
                max_size: rand(1.3..4.0),
                leaf_z_factor: 0.4,
                stem_thickness: 1.,
                stem_len: 2.,
            }),
            Self::Cypress => GrowTree {
                size: -0.1,
                max_size: rand(1.5..2.0),
                last_grown: 0,
                gen: Generator::Cypress,
            },
        }
    }
}

struct BasicParams {
    species: TreeSpecies,
    max_size: f32,
    leaf_z_factor: f32,
    stem_thickness: f32,
    stem_len: f32,
}

fn basic_tree(params: BasicParams) -> GrowTree {
    fn branch(thickness: f32, len: f32, dir: Vec3, sibling_angle: Option<f32>) -> (Branch, f32) {
        let extent = dir * len;
        if thickness > 0.4 {
            let ratio = rand(0.3..0.5);
            let mut angle = rand(0. ..PI * 2.);
            if let Some(sibling) = sibling_angle {
                while (angle - sibling + PI).abs() < PI / 4. {
                    angle = rand(0. ..PI * 2.);
                }
            }
            let split_dir = vec2(angle.sin(), angle.cos());
            let dir_primary = (dir + split_dir.extend(0.) * (1. - ratio)).normalize();
            let dir_secondary = (dir - split_dir.extend(0.) * ratio).normalize();
            let (primary, primary_angle) = branch(
                thickness * (1. - ratio),
                thickness * (1.8 - ratio) * 1.0 + 0.,
                dir_primary,
                None,
            );
            let (secondary, _) = branch(
                thickness * ratio,
                thickness * (1.0 + ratio) * 1.0 + 0.,
                dir_secondary,
                Some(primary_angle),
            );
            (
                Branch {
                    thickness,
                    extent,
                    children: vec![primary, secondary],
                },
                angle,
            )
        } else {
            (
                Branch {
                    thickness,
                    extent,
                    children: default(),
                },
                0.,
            )
        }
    }
    let (stem, _) = branch(params.stem_thickness, params.stem_len, Vec3::Z, None);
    GrowTree {
        size: -0.1,
        max_size: params.max_size,
        last_grown: 0,
        gen: Generator::Basic(BasicTree {
            stem,
            species: params.species,
            leaf_z_factor: params.leaf_z_factor,
        }),
    }
}

// Very basic, but good enough for testing
#[derive(Component)]
pub struct GrowTree {
    pub size: f32,
    max_size: f32,
    last_grown: i32,
    gen: Generator,
}

enum Generator {
    Basic(BasicTree),
    Cypress,
}

struct BasicTree {
    stem: Branch,
    species: TreeSpecies,
    leaf_z_factor: f32,
}

impl BasicTree {
    fn build(&self, level: &mut Level, pos: Vec3, size: f32) {
        if size < 0.25 {
            level(pos, GroundPlant(Sapling(self.species)));
            return;
        }
        level(pos, Fence(Wood(self.species)));

        self.place(level, pos, size, &self.stem, Vec3::ZERO, 0);
    }

    fn place(
        &self,
        level: &mut Level,
        pos: Vec3,
        scale: f32,
        branch: &Branch,
        branch_base: Vec3,
        i: i32,
    ) {
        let start = pos + branch_base * scale;
        let extent = branch.extent * scale;
        if (branch.thickness * scale < 0.8) | branch.children.is_empty() {
            for block_pos in Cuboid::around((start + extent).block(), 5) {
                let mut diff = block_pos.as_vec3() - (start + extent);
                diff.z /= self.leaf_z_factor;
                if diff.length() < scale * 1.0 + rand(-0.7..0.4) {
                    level(block_pos, |b| b | Leaves(self.species, None));
                }
            }
            return;
        }

        let fence = (scale < 0.8) | (i as f32 > scale + 0.25);
        let steps = (extent.length() * 10.) as i32;
        let mut prev_pos = start.block();
        for step in 0..=steps {
            let pos = (start + extent.normalize() * 0.1 * step as f32).block();
            let check_pos = (start + extent.normalize() * 0.1 * (step + 1) as f32).block();
            let diff = (check_pos - prev_pos).abs();
            if fence {
                if diff != IVec3::ZERO {
                    level(pos, |b| b | Fence(Wood(self.species)));
                    prev_pos = pos;
                }
            } else if ((branch.thickness > 0.6) & (diff != IVec3::ZERO)) | (diff.max_element() > 1)
            {
                level(pos, Log(self.species, LogType::FullBark, Axis::Z));
                prev_pos = pos;
            }
        }
        level(
            (start + extent).block(),
            if fence {
                Fence(Wood(self.species))
            } else {
                Log(self.species, LogType::FullBark, Axis::Z)
            },
        );

        for child in &branch.children {
            self.place(level, pos, scale, child, branch_base + branch.extent, i + 1)
        }
    }
}

struct Branch {
    thickness: f32,
    extent: Vec3,
    children: Vec<Branch>,
}

fn build_cypress(level: &mut Level, pos: Vec3, size: f32) {
    let pos = pos.block();
    if size < 0.25 {
        level(pos, GroundPlant(Sapling(Spruce)));
        return;
    }
    for i in 0..=(size * 1.5) as i32 {
        level(
            pos + i * IVec3::Z,
            if size < 1. {
                Fence(Wood(Jungle))
            } else {
                Fence(Andesite)
            },
        )
    }
    let top = (size * 4.) as i32;
    for i in 0..top {
        for x in -1..=1 {
            for y in -1..=1 {
                let distance = ivec2(x, y).as_vec2().length();
                let radius = (1. - ((i as f32 / top as f32) - 0.2).abs()) * (size + 1.) / 2.;
                if distance < radius * rand(1. ..1.3) {
                    level(pos + ivec3(x, y, i + 1), |b| b | Leaves(Spruce, None))
                }
            }
        }
    }
}

impl GrowTree {
    pub fn build(
        &mut self,
        level: &mut Level,
        pos: Vec3,
        current_blocks: &mut Vec<(IVec3, Block)>,
    ) {
        for (pos, block) in current_blocks.drain(..) {
            if level(pos) == block {
                level(pos, Air)
            }
        }
        let cursor = level.recording_cursor();
        match &self.gen {
            Generator::Basic(params) => params.build(level, pos, self.size),
            Generator::Cypress => build_cypress(level, pos, self.size),
        };
        current_blocks.extend(level.get_recording(cursor).map(|r| (r.pos, r.block)));
    }
}

// currently unused
pub fn make_tiny(level: &mut Level, base_pos: IVec3, species: TreeSpecies) {
    let log_block = Log(species, LogType::FullBark, Axis::Z);
    let leaf_block = Leaves(species, None);

    let base_pos = base_pos + ivec3(0, 0, 1);
    level(base_pos, log_block);

    let mut pos = base_pos + ivec3(0, 0, 1) + rand_2(0.2).extend(0);
    level(pos, log_block);

    pos.x += 1;
    if rand(0.8) {
        if pos.truncate() == base_pos.truncate() {
            pos += rand_2(0.3).extend(0);
        }
        level(pos, log_block);

        if rand(0.2) {
            pos.z += 1;
            level(pos, log_block);
        }
    }

    level(pos + ivec3(1, 0, 0), |b| b | leaf_block);
    level(pos + ivec3(-1, 0, 0), |b| b | leaf_block);
    level(pos + ivec3(0, 1, 0), |b| b | leaf_block);
    level(pos + ivec3(0, -1, 0), |b| b | leaf_block);
    level(pos + ivec3(0, 0, 1), |b| b | leaf_block);
    level(pos + ivec3(0, 0, -1), |b| b | leaf_block);

    for leaf_pos in Cuboid::new(pos - IVec3::splat(2), pos + IVec3::splat(2)) {
        let distance_squared = ((leaf_pos - pos).x * (leaf_pos - pos).x
            + (leaf_pos - pos).z * (leaf_pos - pos).z
            + (leaf_pos - pos).y * (leaf_pos - pos).y) as f32;
        if rand(1.0 - (distance_squared / 3.0)) {
            level(leaf_pos, |b| b | leaf_block);
        }
    }
}

// currently unused
pub fn make_straight(level: &mut Level, pos: IVec3, species: TreeSpecies) {
    let log_block = Log(species, LogType::FullBark, Axis::Z);
    let leaf_block = Leaves(species, None);

    let log_height = 5 + rand_1(0.5) + rand_1(0.5);

    for z in 1..=log_height {
        level(pos + IVec3::Z * z, log_block);
    }

    for off in &[ivec2(1, 0), ivec2(-1, 0), ivec2(0, 1), ivec2(0, -1)] {
        for z in 3..=log_height + 1 {
            level(pos + ivec3(off.x, off.y, z), leaf_block);
        }
    }

    level(pos + IVec3::Z * (log_height + 1), |b| b | leaf_block);
    level(pos + IVec3::Z * (log_height + 2), |b| b | leaf_block);
    if (log_height == 5) & rand(0.75) | (log_height > 5) {
        level(pos + IVec3::Z * (log_height + 3), |b| b | leaf_block);
    }

    for off in &[ivec2(1, 1), ivec2(-1, 1), ivec2(1, -1), ivec2(-1, -1)] {
        for z in 4 + rand_1(0.5)..=log_height - 1 + rand_1(0.5) {
            level(pos + ivec3(off.x, off.y, z), |b| b | leaf_block);
        }
    }
}

#[derive(SystemParam)]
pub struct Untree<'w, 's> {
    commands: Commands<'w, 's>,
    tree_map: ResMut<'w, Trees>,
    trees: Query<'w, 's, &'static Tree>,
}

impl<'w, 's> Untree<'w, 's> {
    pub fn remove_trees(&mut self, level: &mut Level, area: impl IntoIterator<Item = IVec2>) {
        for column in area.into_iter() {
            if let Some(entity) = self.tree_map[column] {
                let tree = self.trees.get(entity).unwrap();
                for (pos, block) in &tree.blocks {
                    if level(*pos) == *block {
                        level(*pos, Air)
                    }
                }
                self.tree_map[column] = None;
                self.commands.entity(entity).despawn();
            }
        }
    }
}
