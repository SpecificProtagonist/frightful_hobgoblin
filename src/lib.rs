#![feature(option_get_or_insert_default)]
#![feature(lazy_cell)]
#![feature(let_chains)]
// Feeling cute, might delete later
#![feature(unboxed_closures)]
#![feature(fn_traits)]

// Flat module hierarchy is ok for now
pub mod debug_image;
mod geometry;
mod level;
// pub mod make_divider;
pub mod make_name;
pub mod make_trees;
pub mod prefab;
pub mod remove_foliage;
pub mod sim;
// pub mod terraform;
pub mod goods;
pub mod house;
pub mod optimize;
pub mod pathfind;
pub mod rand;
pub mod replay;
pub mod roof;
pub mod test_house;

use std::cell::Cell;

// Replaces SipHash with ahash & disables randomness
pub use bevy_utils::{StableHashMap as HashMap, StableHashSet as HashSet};
pub use geometry::*;
pub use level::*;
pub use prefab::PREFABS;
pub use rand::*;

pub fn default<T: Default>() -> T {
    Default::default()
}

const DATA_VERSION: i32 = 3578;

/// How far outside of the borders of the work area is loaded
const LOAD_MARGIN: i32 = 20;

/// The config isn't commited to git because it just contains the paths to the world folders
#[path = "../config_local.rs"]
pub mod config;
