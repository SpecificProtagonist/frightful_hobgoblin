#![feature(option_get_or_insert_default)]
#![feature(lazy_cell)]
#![feature(let_chains)]
// Feeling cute, might delete later
#![feature(unboxed_closures)]
#![feature(fn_traits)]

// Flat module hierarchy is ok for now
pub mod debug_image;
mod geometry;
pub mod goods;
pub mod house;
mod level;
pub mod make_name;
pub mod optimize;
pub mod pathfind;
pub mod prefab;
pub mod rand;
pub mod remove_foliage;
pub mod replay;
pub mod roof;
pub mod sim;
pub mod stall;
pub mod test_house;
pub mod trees;
// pub mod terraform;
// pub mod make_divider;

use std::cell::Cell;

use bevy_utils::FixedState;
pub use geometry::*;
pub use level::*;
pub use rand::*;
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
