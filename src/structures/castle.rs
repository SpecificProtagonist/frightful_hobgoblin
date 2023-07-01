use std::cmp::Ordering::*;
use std::collections::HashMap;

use crate::*;
use structures::Template;
use terraform::*;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Usage {
    Wall,
    Window(HDir),
    FreeInterior,
}
pub struct Blueprint {
    pub area: Rect,
    pub y: i32,
    pub relative_height: i32,
}

impl Blueprint {
    pub fn build(&self, world: &mut impl WorldView) {
        make_foundation_straight(world, self.area, self.y, Cobble);

        let mut stories = Vec::new();

        let basic_plan = {
            let mut layout = HashMap::new();

            // Walls
            for column in self.area.border() {
                layout.insert(column, Usage::Wall);
            }

            // Floor
            for column in self.area.shrink(1) {
                layout.insert(column, Usage::FreeInterior);
            }
            layout
        };

        // Define stories
        // Todo: e.g. differing heights, cellar, ...
        {
            let max_surrounding_height = self
                .area
                .grow(2)
                .border()
                .map(|column| world.height(column))
                .max()
                .unwrap();
            let num_stories =
                (max_surrounding_height.saturating_sub(self.y) as f32 / 4.1 + 2.9) as u8;
            let mut floor_height = self.y;
            for _ in 0..num_stories {
                let story_height = 4;
                stories.push((floor_height, story_height, basic_plan.clone()));
                floor_height += story_height;
            }
        }

        // Windows
        let mut possible_privy_locations = Vec::new();
        for (floor_height, _, layout) in stories.iter_mut() {
            for column in self.area.border() {
                if let Some(facing) = HDir::iter().find(|facing| {
                    matches!(
                        layout.get(&(column - Vec2::from(*facing))),
                        Some(Usage::FreeInterior)
                    )
                }) {
                    // Don't build next to the corners or other windows
                    if matches!(
                        layout.get(&(column + Vec2::from(facing).clockwise())),
                        Some(Usage::Wall)
                    ) && matches!(
                        layout.get(&(column + Vec2::from(facing).counterclockwise())),
                        Some(Usage::Wall)
                    ) & layout
                        .get(&(column + Vec2::from(facing).clockwise() * 2))
                        .is_some()
                        & layout
                            .get(&(column + Vec2::from(facing).counterclockwise() * 2))
                            .is_some()
                    {
                        // Make more windows highter up
                        let height_outside = world
                            .height(column + Vec2::from(facing))
                            .max(world.height(column + Vec2::from(facing) * 3))
                            .max(world.height(
                                column + Vec2::from(facing) + Vec2::from(facing.rotated(1)) * 2,
                            ))
                            .max(world.height(
                                column + Vec2::from(facing) + Vec2::from(facing.rotated(3)) * 2,
                            ));
                        let height = floor_height.saturating_sub(height_outside);
                        if height > 5 {
                            possible_privy_locations.push((column.at(*floor_height), facing));
                        }
                        if rand(((height as f32 - 3.2) / 8.0).min(0.8)) {
                            layout.insert(column, Usage::Window(facing));
                        }
                    }
                }
            }
        }

        // Actually build this
        for (floor_height, story_height, layout) in stories {
            build_floor(world, &layout, floor_height);
            build_walls(world, &layout, floor_height + 1, story_height, true);
            build_windows(world, &layout, floor_height);
        }

        // Privy
        possible_privy_locations.sort_unstable_by_key(|(pos, dir)| {
            world.height(Column::from(*pos) - Vec2::from(*dir) * 2)
        });

        if let Some((pos, facing)) = possible_privy_locations.first() {
            build_privy(world, *pos, *facing);
        }
    }
}

fn build_floor(world: &mut impl WorldView, layout: &HashMap<Column, Usage>, floor_y: i32) {
    let floor_block = &Log(TreeSpecies::Spruce, LogType::Normal(Axis::X));
    for (column, usage) in layout {
        if matches!(usage, Usage::FreeInterior) {
            world.set(column.at(floor_y), floor_block);
        }
    }
}

fn build_walls(
    world: &mut impl WorldView,
    layout: &HashMap<Column, Usage>,
    base_y: i32,
    height: i32,
    fancy_corners: bool,
) {
    // Basic walls
    for (column, usage) in layout {
        if matches!(usage, Usage::Wall | Usage::Window(..)) {
            for y in base_y..(base_y + height) {
                world.set(column.at(y), FullBlock(Cobble));
            }
        }
    }
    // Fancy corners
    if fancy_corners {
        for (column, usage) in layout {
            let column = *column;
            // Detect corner
            if matches!(usage, Usage::Wall)
                & (matches!(
                    layout.get(&(column + Vec2(-1, 0))),
                    Some(Usage::Wall) | Some(Usage::Window(..))
                ) ^ matches!(
                    layout.get(&(column + Vec2(1, 0))),
                    Some(Usage::Wall) | Some(Usage::Window(..))
                ))
                & (matches!(
                    layout.get(&(column + Vec2(0, -1))),
                    Some(Usage::Wall) | Some(Usage::Window(..))
                ) ^ matches!(
                    layout.get(&(column + Vec2(0, 1))),
                    Some(Usage::Wall) | Some(Usage::Window(..))
                ))
            {
                for y in base_y..(base_y + height) {
                    world.set_override(column.at(y), FullBlock(Stonebrick));
                    if (base_y + height - y) % 2 == 0 {
                        for column in &[
                            column + Vec2(-1, 0),
                            column + Vec2(1, 0),
                            column + Vec2(0, -1),
                            column + Vec2(0, 1),
                        ] {
                            if let Some(Usage::Wall) = layout.get(column) {
                                world.set_override(column.at(y), FullBlock(Stonebrick));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Assumes walls are already build
fn build_windows(world: &mut impl WorldView, layout: &HashMap<Column, Usage>, base_y: i32) {
    for (column, usage) in layout {
        if let Usage::Window(facing) = usage {
            world.set_override(
                column.at(base_y + 1),
                Stair(Material::Cobble, facing.rotated(2), Flipped(false)),
            );
            world.set(column.at(base_y + 2), GlassPane(Some(Color::Brown)));
            world.set_override(
                column.at(base_y + 3),
                Stair(Material::Cobble, facing.rotated(2), Flipped(true)),
            );
        }
    }
}

fn build_privy(world: &mut impl WorldView, base_pos: Pos, facing: HDir) {
    Template::get("castle/privy").build(world, base_pos, facing);

    let drop_column = Column::from(base_pos) + Vec2::from(facing);
    let drop_column = drop_column
        + if let FullBlock(Cobble) = world.get(drop_column.at(world.height(drop_column))) {
            Vec2::from(facing)
        } else {
            Vec2(0, 0)
        };
    world.set(
        drop_column.at(world.height(drop_column)),
        Soil(Soil::SoulSand),
    );
}

// TODO: don't build on existing villages!
// TODO: Don't build in water!
// TODO: don't build on overhangs if the overhand it very thin/unsupported
pub fn generate_blueprints(world: &World) -> Vec<Blueprint> {
    let mut choices = Vec::new();

    // This doesn't really work that well, use a different algorithm
    let probe_distance = 6;
    for x in ((world.area().min.0 + probe_distance / 2)..world.area().max.0)
        .step_by(probe_distance as usize)
    {
        for z in ((world.area().min.1 + probe_distance / 2)..world.area().max.1)
            .step_by(probe_distance as usize)
        {
            fn ascend(world: &World, column: Column) -> Column {
                let max_dist = 8;
                let slope = slope(world, column);
                column
                    + Vec2(
                        slope.0.clamp(-max_dist, max_dist),
                        slope.1.clamp(-max_dist, max_dist),
                    )
            }
            let column = ascend(world, Column(x, z));
            if world.area().contains(column) {
                let column = ascend(world, column);
                if world.area().contains(column) {
                    choices.push(column);
                }
            }
        }
    }

    let mut choices: Vec<Blueprint> = choices
        .iter()
        .flat_map(|pos| find_good_footprint_at(world, *pos))
        .filter(|blueprint| blueprint.relative_height >= 0)
        .collect();

    choices.sort_unstable_by_key(|blueprint| -blueprint.relative_height);

    choices
}

// TODO: prefer expanding to border of steep terrain
// TODO: limit max height difference
fn find_good_footprint_at(world: &impl WorldView, pos: Column) -> Option<Blueprint> {
    const MIN_LENGTH: i32 = 5;
    const MAX_LENGTH: i32 = 14;
    const MAX_SECONDARY_LENGTH: i32 = 8;
    const MAX_SLOPE: i32 = 3;
    const MAX_HEIGHT_DIFF: i32 = 7;

    let mut area = Rect { min: pos, max: pos };
    let mut min_y = world.height(pos);
    let mut max_y = min_y;
    loop {
        // Store how much the height-range would have to be extended
        let check_height_diff = |max_diff: &mut i32, y: i32| {
            if min_y - y > max_diff.abs() {
                *max_diff = y - min_y;
            } else if y - max_y > max_diff.abs() {
                *max_diff = y - max_y;
            }
        };

        // Check in which direction the terrain is the flattest
        let mut height_diff_x_plus = 0;
        let slope_x_plus = (area.min.1..=area.max.1)
            .map(|z| {
                let height = world.height(Column(area.max.0, z));
                check_height_diff(&mut height_diff_x_plus, height);
                (height - world.height(Column(area.max.0 + 1, z))).abs()
            })
            .max()
            .unwrap();
        let mut height_diff_x_neg = 0;
        let slope_x_neg = (area.min.1..=area.max.1)
            .map(|z| {
                let height = world.height(Column(area.min.0, z));
                check_height_diff(&mut height_diff_x_neg, height);
                (height - world.height(Column(area.min.0 - 1, z))).abs()
            })
            .max()
            .unwrap();
        let mut height_diff_z_plus = 0;
        let slope_z_plus = (area.min.0..=area.max.0)
            .map(|x| {
                let height = world.height(Column(x, area.max.1));
                check_height_diff(&mut height_diff_z_plus, height);
                (height - world.height(Column(x, area.max.1 + 1))).abs()
            })
            .max()
            .unwrap();
        let mut height_diff_z_neg = 0;
        let slope_z_neg = (area.min.0..=area.max.0)
            .map(|x| {
                let height = world.height(Column(x, area.min.1));
                check_height_diff(&mut height_diff_z_neg, height);
                (height - world.height(Column(x, area.min.1 - 1))).abs()
            })
            .max()
            .unwrap();

        // Chose whether to prefer expansion into positive or negative direction, encode in sign
        let (slope_x, height_diff_x) = {
            let positive = match slope_x_neg.cmp(&slope_x_plus) {
                Less => false,
                Greater => true,
                Equal => rand(1.5),
            };
            if positive {
                (slope_x_plus as f32 + 1.0, height_diff_x_plus)
            } else {
                (-slope_x_neg as f32 - 1.0, height_diff_x_neg)
            }
        };
        let (slope_z, height_diff_z) = {
            let positive = match slope_z_neg.cmp(&slope_z_plus) {
                Less => false,
                Greater => true,
                Equal => rand(1.5),
            };
            if positive {
                (slope_z_plus as f32 + 1.0, height_diff_z_plus)
            } else {
                (-slope_z_neg as f32 - 1.0, height_diff_z_neg)
            }
        };

        let allowed = |slope: f32, y_diff: i32| {
            (slope.abs() <= MAX_SLOPE as f32)
                & (y_diff.abs() + max_y.saturating_sub(min_y) < MAX_HEIGHT_DIFF)
        };

        // If minimum size can't be reached, abort
        if ((area.size().0 < MIN_LENGTH) & !allowed(slope_x, height_diff_x))
            | ((area.size().1 < MIN_LENGTH) & !allowed(slope_z, height_diff_z))
        {
            return None;
        }

        enum Dir {
            X,
            Z,
        }

        let dir  =
        // First ensure minimum size is met
        if area.size().0 < MIN_LENGTH && area.size().0 <= area.size().1 {
            Dir::X
        } else if area.size().1 < MIN_LENGTH {
            Dir::Z
        } else if (slope_x.abs() <= slope_z.abs())
            & allowed(slope_x, height_diff_x)
            & (area.size().0 < MAX_LENGTH)
            & ((area.size().0 < MAX_SECONDARY_LENGTH) | (area.size().1 < MAX_LENGTH))
        {
            // Minimum size reached, just expand until maximum size or slope is met
            // TODO: check if this introduces a bias for the x direction
            Dir::X
        } else if (area.size().1 < MAX_LENGTH)
            & allowed(slope_z, height_diff_z)
            & ((area.size().1 < MAX_SECONDARY_LENGTH) | (area.size().0 < MAX_LENGTH))
        {
            Dir::Z
        } else {
            // Maximum size reached
            break;
        };

        let height_diff = if let Dir::X = dir {
            if slope_x.is_sign_negative() {
                area.min.0 -= 1;
            } else {
                area.max.0 += 1;
            }
            height_diff_x
        } else {
            if slope_z.is_sign_negative() {
                area.min.1 -= 1;
            } else {
                area.max.1 += 1;
            }
            height_diff_z
        };

        if height_diff < 0 {
            min_y += height_diff;
        } else {
            max_y += height_diff;
        }
    }

    let y = area
        .into_iter()
        .map(|column| world.height(column))
        .sum::<i32>()
        / (area.size().0 * area.size().1);

    Some(Blueprint {
        area,
        y,
        relative_height: {
            let mut count = 0;
            let mut sum = 0;
            for column in area.grow(3).border().chain(area.grow(6).border()) {
                count += 1;
                sum += y - world.height(column);
            }
            sum / count
        },
    })
}
