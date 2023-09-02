#![feature(option_get_or_insert_default)]
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
pub mod material;
pub mod optimize;
pub mod pathfind;
pub mod replay;
pub mod roof;

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

pub fn rand(prob: f32) -> bool {
    rand::random::<f32>() < prob
}

pub fn rand_1(prob: f32) -> i32 {
    if rand::random::<f32>() < prob {
        rand::Rng::gen_range(&mut rand::thread_rng(), -1, 2)
    } else {
        0
    }
}

pub fn rand_2(prob: f32) -> IVec2 {
    ivec2(rand_1(prob), rand_1(prob))
}

pub fn rand_3(prob: f32) -> IVec3 {
    ivec3(rand_1(prob), rand_1(prob), rand_1(prob))
}

// Inclusive range
pub fn rand_range(min: i32, max: i32) -> i32 {
    rand::Rng::gen_range(&mut rand::thread_rng(), min, max)
}
