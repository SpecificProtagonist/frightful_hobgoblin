use std::collections::HashMap;

use crate::*;
use terraform::*;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Usage {
    Wall,
    Window(HDir),
    FreeInterior,
}
pub struct Blueprint {
    pub area: Rect,
}

impl Blueprint {
    pub fn build(&self, world: &mut impl WorldView) {
        let wall_block = &Stone(Stone::Cobble);
        let wall_accent_block = &Stone(Stone::Stonebrick);
        let floor_block = &Log(
            TreeSpecies::Spruce,
            LogType::Normal(Axis::X),
            LogOrigin::Manmade,
        );
        let stories = if rand(0.7) { 3 } else { 4 };

        let mut floor_height = world.heightmap(self.area.center());
        make_foundation(world, self.area, floor_height, Stone::Cobble);

        for story in 0..stories {
            let story_height = 4;

            let mut layout = HashMap::new();

            // Walls
            for column in self.area.border() {
                layout.insert(column, Usage::Wall);
            }

            // Floor
            for column in self.area.shrink(1).iter() {
                layout.insert(column, Usage::FreeInterior);
                world.set(column.at(floor_height), floor_block);
            }

            // Windows
            for column in self.area.border() {
                if let Some(facing) = HDir::iter().find(|facing| {
                    matches!(
                        layout.get(&(column + Vec2::from(*facing))),
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
                        .get(&(column + Vec2::from(facing).clockwise()))
                        .is_some()
                        & layout
                            .get(&(column + Vec2::from(facing).counterclockwise()))
                            .is_some()
                    {
                        // Make more windows highter up
                        let height = floor_height
                            .saturating_sub(world.heightmap(column - Vec2::from(facing)));
                        if rand(((height as f32 - 3.2) / 8.0).min(0.8)) {
                            layout.insert(column, Usage::Window(facing));
                        }
                    }
                }
            }

            build_walls(world, &layout, floor_height + 1, story_height, true);
            build_windows(world, &layout, floor_height);

            floor_height += story_height;
        }
    }
}

/// Assumes walls are already build
fn build_windows(world: &mut impl WorldView, layout: &HashMap<Column, Usage>, base_y: u8) {
    for (column, usage) in layout {
        if let Usage::Window(facing) = usage {
            world.set_override(
                column.at(base_y + 1),
                StoneStair(Stone::Cobble, *facing, false),
            );
            world.set(column.at(base_y + 2), GlassPane(Some(Color::White)));
            world.set_override(
                column.at(base_y + 3),
                StoneStair(Stone::Cobble, *facing, true),
            );
        }
    }
}

fn build_walls(
    world: &mut impl WorldView,
    layout: &HashMap<Column, Usage>,
    base_y: u8,
    height: u8,
    fancy_corners: bool,
) {
    // Basic walls
    for (column, usage) in layout {
        if matches!(usage, Usage::Wall | Usage::Window(..)) {
            for y in base_y..(base_y + height) {
                world.set(column.at(y), Stone(Stone::Cobble));
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
                    world.set_override(column.at(y), Stone(Stone::Stonebrick));
                    if (base_y + height - y) % 2 == 0 {
                        for column in &[
                            column + Vec2(-1, 0),
                            column + Vec2(1, 0),
                            column + Vec2(0, -1),
                            column + Vec2(0, 1),
                        ] {
                            if let Some(Usage::Wall) = layout.get(column) {
                                world.set_override(column.at(y), Stone(Stone::Stonebrick));
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn generate_blueprints(world: &World) -> Vec<Blueprint> {
    // Find a suitable location
    let chunk_heights = terraform::max_chunk_heights(world);

    // TODO:
    let positions = vec![
        Column(60, 32),
        Column(72, 48),
        Column(2, -12),
        Column(26, -14),
        Column(44, -34),
        Column(-32, -7),
    ];

    positions
        .iter()
        .map(|pos| Blueprint {
            area: find_good_footprint_at(world, *pos),
        })
        .collect()
}

// TODO: return Option
// TODO: prefer expanding to cliff
fn find_good_footprint_at(world: &impl WorldView, pos: Column) -> Rect {
    const MIN_LENGTH: i32 = 5;
    const MAX_LENGTH: i32 = 12;
    const MAX_SECONDARY_LENGTH: i32 = 8;
    const MAX_SLOPE: i32 = 8;

    let mut area = Rect { min: pos, max: pos };
    loop {
        // Check in which direction the terrain is the flattest
        // TODO: simply compare directly neighboring instead of gaussian (should fit better on some sharp corners)
        let max_slope_x_plus = (area.min.1..=area.max.1)
            .map(|z| {
                (world.heightmap(Column(area.max.0, z)) as i32
                    - world.heightmap(Column(area.max.0 + 1, z)) as i32)
                    .abs()
            })
            .max()
            .unwrap();
        let max_slope_x_neg = (area.min.1..=area.max.1)
            .map(|z| {
                (world.heightmap(Column(area.min.0, z)) as i32
                    - world.heightmap(Column(area.min.0 - 1, z)) as i32)
                    .abs()
            })
            .max()
            .unwrap();
        let max_slope_z_plus = (area.min.0..=area.max.0)
            .map(|x| {
                (world.heightmap(Column(x, area.max.1)) as i32
                    - world.heightmap(Column(x, area.max.1 + 1)) as i32)
                    .abs()
            })
            .max()
            .unwrap();
        let max_slope_z_neg = (area.min.0..=area.max.0)
            .map(|x| {
                (world.heightmap(Column(x, area.min.1)) as i32
                    - world.heightmap(Column(x, area.min.1 - 1)) as i32)
                    .abs()
            })
            .max()
            .unwrap();

        // Chose whether to prefer expansion into positive or negative direction, encode in sign
        let slope_x = if max_slope_x_neg < max_slope_x_plus {
            -(max_slope_x_neg as f32)
        } else if max_slope_x_neg > max_slope_x_plus {
            max_slope_x_plus as f32
        } else {
            if rand(1.5) {
                -(max_slope_x_neg as f32)
            } else {
                max_slope_x_plus as f32
            }
        };
        let slope_z = if max_slope_z_neg < max_slope_z_plus {
            -(max_slope_z_neg as f32)
        } else if max_slope_z_neg > max_slope_z_plus {
            max_slope_z_plus as f32
        } else {
            if rand(1.5) {
                -(max_slope_z_neg as f32)
            } else {
                max_slope_z_plus as f32
            }
        };

        // First ensure minimum size is met
        if area.size().0 < MIN_LENGTH && area.size().0 <= area.size().1 {
            if slope_x.is_sign_negative() {
                area.min.0 -= 1;
            } else {
                area.max.0 += 1;
            }
            continue;
        } else if area.size().1 < MIN_LENGTH {
            if slope_z.is_sign_negative() {
                area.min.1 -= 1;
            } else {
                area.max.1 += 1;
            }
            continue;
        }

        // If it is, just expand until maximum size or slope is met
        if (slope_x.abs() <= slope_z.abs())
            & ((max_slope_x_neg <= MAX_SLOPE) | (max_slope_x_plus <= MAX_SLOPE))
            & (area.size().0 < MAX_LENGTH)
            & ((area.size().0 < MAX_SECONDARY_LENGTH) | !(area.size().1 < MAX_LENGTH))
        {
            if slope_x.is_sign_negative() {
                area.min.0 -= 1;
            } else {
                area.max.0 += 1;
            }
        } else if (area.size().1 < MAX_LENGTH)
            & ((max_slope_z_neg <= MAX_SLOPE) | (max_slope_z_plus <= MAX_SLOPE))
            & ((area.size().1 < MAX_SECONDARY_LENGTH) | !(area.size().0 < MAX_LENGTH))
        {
            if slope_z.is_sign_negative() {
                area.min.1 -= 1;
            } else {
                area.max.1 += 1;
            }
        } else {
            // Maximum size reached
            break;
        }
    }

    area
}
