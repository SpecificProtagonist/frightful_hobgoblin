#![feature(option_get_or_insert_default)]
#![feature(let_chains)]
// Feeling cute, might delete later
#![feature(unboxed_closures)]
#![feature(fn_traits)]
// Mostly for bevy stuff
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

// Flat module hierarchy is ok for now
pub mod debug_image;
pub mod detect_existing_buildings;
mod geometry;
pub mod goods;
pub mod house;
mod level;
pub mod loot;
pub mod make_name;
pub mod market;
pub mod noise;
pub mod optimize;
pub mod pathfind;
pub mod prefab;
pub mod rand;
pub mod remove_foliage;
pub mod replay;
pub mod roof;
pub mod shipping;
pub mod sim;
pub mod test_house;
pub mod trees;
// pub mod terraform;
// pub mod make_divider;

use std::cell::Cell;

use bevy_utils::FixedState;
pub use geometry::*;
pub use level::*;
pub use prefab::prefab;
pub use rand::*;
use serde::Deserialize;
pub use sim::*;
pub use trees::Untree;

// Replaces SipHash with ahash & disables randomness
pub type HashMap<K, V> = std::collections::HashMap<K, V, FixedState>;
pub type HashSet<K> = std::collections::HashSet<K, FixedState>;

pub fn default<T: Default>() -> T {
    Default::default()
}

const DATA_VERSION: i32 = 3578;

/// How far outside of the borders of the work area is loaded
const LOAD_MARGIN: i32 = 20;

#[derive(Deserialize, Resource)]
pub struct Config {
    // World settings
    pub path: String,
    pub out_path: Option<String>,
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
    // Generator settings
    pub seed: Option<u64>,
    pub villagers: i32,
    pub ticks: i32,
    // Debug options
    #[serde(default)]
    pub no_building_cost: bool,
    #[serde(default)]
    pub no_replay: bool,
    #[serde(default)]
    pub skip_walk: bool,
    #[serde(default)]
    pub show_reachability: bool,
    #[serde(default)]
    pub show_blocked: bool,
}

impl Config {
    pub fn area(&self) -> Rect {
        Rect {
            min: ivec2(self.min_x, self.min_y),
            max: ivec2(self.max_x, self.max_y),
        }
    }

    pub fn load_level(&self) -> Level {
        Level::new(
            &self.path,
            self.out_path.as_ref().unwrap_or(&self.path),
            self.area(),
        )
    }
}
