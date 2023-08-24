#![feature(option_get_or_insert_default)]
#![feature(local_key_cell_methods)]
#![feature(lazy_cell)]
#![allow(dead_code)]

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
pub mod house;
pub mod house2;
pub mod roof;
pub mod sim_anneal;

pub use geometry::*;
pub use hashbrown::HashMap;
pub use level::*;
pub use prefab::PREFABS;

pub fn default<T: Default>() -> T {
    Default::default()
}

/// How far outside of the borders of the work area is loaded
const LOAD_MARGIN: i32 = 20;

/// The config isn't commited to git because it just contains the paths to the world folders
pub mod config {
    include!("../config_local.rs");
}
