#![feature(option_get_or_insert_default)]
#![feature(lazy_cell)]
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
