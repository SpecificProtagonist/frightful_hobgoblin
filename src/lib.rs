// Flat module hierarchy is ok for now
mod behavior;
mod build_recorder;
pub mod debug_image;
mod geometry;
pub mod make_divider;
pub mod make_misc;
pub mod make_name;
pub mod make_trees;
pub mod remove_foliage;
pub mod structures;
pub mod terraform;
mod world;

pub use behavior::*;
pub use build_recorder::*;
pub use geometry::*;
pub use world::*;

/// How far outside of the borders of the work area is loaded
const LOAD_MARGIN: i32 = 20;

/// The config isn't commited to git because it just contains the paths to the world folders
pub mod config {
    include!("../config_local.rs");
}
