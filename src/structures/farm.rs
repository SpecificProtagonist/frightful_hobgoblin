use std::collections::{BinaryHeap, HashSet};

use num_traits::FromPrimitive;
use rand::prelude::*;

use crate::geometry::*;
use crate::world::*;
use crate::{remove_foliage, terraform::*};

// Future note: If there are trees in the way, return two stages, so choped woods
// (with stumps) can be shown in preparation of future fields

pub struct Blueprint {
    start: Column, // only for debug purposes
    area: HashSet<Column>,
    border: HashSet<Column>,
}

impl Blueprint {
    pub fn new(world: &impl WorldView, start: Column) -> Option<Blueprint> {
        // Spread from starting point, avoiding slopes
        // TODO: make more circular (and maybe a bit random), currently Manhatten on flat ground
        const START_STRENGTH: f32 = 20.0;
        const SLOPE_COST: f32 = 0.3;
        const HEIGHT_DIFF_COST: f32 = 2.5;
        const MIN_STRENGTH_FOR_NEW_HEIGHT: f32 = 3.0;
        const MAX_SLOPE: f32 = 7.0;

        let mut area = HashSet::new();
        let mut considered = BinaryHeap::new();
        considered.push(Considered(start, START_STRENGTH));
        let mut border = HashSet::new();

        while let Some(Considered(column, strength)) = considered.pop() {
            area.insert(column);
            border.remove(&column);

            let height = world.height(column);
            for neightbor in [Vec2(1, 0), Vec2(0, 1), Vec2(-1, 0), Vec2(0, -1)].iter() {
                let neighbor = column + *neightbor;

                if area.contains(&neighbor)
                    || !matches!(
                        world.get(neighbor.at(world.height(neighbor))),
                        Soil(Soil::Grass) | Soil(Soil::Dirt)
                    )
                {
                    continue;
                }

                let height_diff = (height as f32 - world.height(neighbor) as f32).abs();
                let slope = slope(world, neighbor).len();

                let neighbor_strength =
                    strength - 1.0 - slope * SLOPE_COST - height_diff * HEIGHT_DIFF_COST;

                let required_strength = if height_diff > 0.0 {
                    MIN_STRENGTH_FOR_NEW_HEIGHT
                } else {
                    0.0
                };

                if (height_diff < 2.0)
                    & (slope < MAX_SLOPE)
                    & (neighbor_strength >= required_strength)
                {
                    considered.push(Considered(neighbor, neighbor_strength));
                } else {
                    border.insert(neighbor);
                }
            }
        }

        if area.len() > (START_STRENGTH * 0.4).powf(1.7) as usize {
            Some(Blueprint {
                start,
                area,
                border,
            })
        } else {
            None
        }
    }

    pub fn render(&self, world: &mut impl WorldView) {
        // TODO: border, esp on downwards edge
        remove_foliage::trees(world, self.area.iter().cloned(), false);

        const SCARECROW_CHANCE: f32 = 0.01;
        const MIN_SCARECROW_DISTANCE: f32 = 5.0;
        let mut scarecrows = Vec::new();

        for column in self.area.iter() {
            let y = world.height(*column);
            if rand(SCARECROW_CHANCE)
                && scarecrows
                    .iter()
                    .all(|existing: &Column| (*column - *existing).len() > MIN_SCARECROW_DISTANCE)
            {
                scarecrows.push(*column);
                make_scarecrow(world, *column);
            } else {
                world.set(column.at(y), Soil(Soil::Farmland));
                world.set(column.at(y + 1), GroundPlant(Crop(Crop::Wheat)));
            }
        }

        // TMP
        world.set(self.start.at(world.height(self.start) + 1), Wool(Red));
    }
}

pub fn make_hedge_edge(world: &mut World, fields: &[Blueprint]) {
    for i in 0..fields.len() {
        'outer: for column in &fields[i].border {
            for other_field in &fields[i + 1..] {
                if other_field.area.contains(column) {
                    continue 'outer;
                }
            }
            world.set(
                column.at(world.height(*column) + 1),
                Block::Leaves(TreeSpecies::DarkOak),
            );
        }
    }
}

// NaN is why we can't have nice things
#[derive(PartialEq)]
struct Considered(Column, f32);

impl Eq for Considered {}

impl PartialOrd for Considered {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.1.partial_cmp(&other.1)
    }
}

impl Ord for Considered {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap() // This is fine, we don't have NaN
    }
}

pub fn make_scarecrow(world: &mut impl WorldView, column: Column) {
    let mut pos = column.at(world.height(column) + 1);
    let fence_block = &Fence(Wood(world.biome(column).random_tree_species()));
    let direction = HDir::from_u8(thread_rng().gen_range(0, 4)).unwrap();

    let colors = &[
        Color::Black,
        Color::Gray,
        Color::LightGray,
        Color::White,
        Color::Red,
        Color::Brown,
        Color::Blue,
        Color::Green,
    ];

    let center_block = &if rand(0.5) {
        Wool(colors[thread_rng().gen_range(0, colors.len())])
    } else {
        Hay
    };

    world.set(pos, fence_block);
    pos += Vec3(0, 1, 0);
    if rand(0.34) {
        if rand(0.65) {
            world.set(pos, fence_block);
        } else {
            world.set(pos, center_block);
        }
        pos += Vec3(0, 1, 0);
    }
    world.set(pos, center_block);
    world.set(pos + Vec2::from(direction).clockwise(), fence_block);
    world.set(pos + Vec2::from(direction).counterclockwise(), fence_block);
    pos += Vec3(0, 1, 0);
    // TODO: carve
    world.set(pos, GroundPlant(GroundPlant::Pumpkin));
}

pub fn make_signpost() {}
