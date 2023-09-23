#![feature(option_get_or_insert_default)]
#![feature(lazy_cell)]
#![feature(let_chains)]

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
pub mod replay;
pub mod roof;
pub mod test_house;

use std::cell::RefCell;

pub use geometry::*;
pub use hashbrown::HashMap;
pub use level::*;
pub use prefab::PREFABS;
use rand::{rngs::StdRng, Rng, SeedableRng};

pub fn default<T: Default>() -> T {
    Default::default()
}

const DATA_VERSION: i32 = 3577;

/// How far outside of the borders of the work area is loaded
const LOAD_MARGIN: i32 = 20;

/// The config isn't commited to git because it just contains the paths to the world folders
pub mod config {
    include!("../config_local.rs");
}

pub fn rand(prob: f32) -> bool {
    RNG.with_borrow_mut(|rng| rng.gen_bool(prob.clamp(0., 1.) as f64))
}

pub fn rand_1(prob: f32) -> i32 {
    if rand(prob) {
        if rand(0.5) {
            1
        } else {
            -1
        }
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

/// Inclusive range
pub fn rand_range(min: i32, max: i32) -> i32 {
    RNG.with_borrow_mut(|rng| rng.gen_range(min, max + 1))
}

pub fn rand_f32(min: f32, max: f32) -> f32 {
    RNG.with_borrow_mut(|rng| rng.gen_range(min, max))
}

trait ChooseExt {
    type Item;
    fn try_choose(&self) -> Option<&Self::Item>;
    fn choose(&self) -> &Self::Item;
}

impl<T> ChooseExt for [T] {
    type Item = T;

    fn try_choose(&self) -> Option<&T> {
        RNG.with_borrow_mut(|rng| rand::seq::SliceRandom::choose(self, rng))
    }

    fn choose(&self) -> &T {
        self.try_choose().unwrap()
    }
}

trait ChooseExt2: ExactSizeIterator {
    // TODO
}

thread_local! {
    pub static RNG: RefCell<StdRng> = StdRng::from_entropy().into();
}
