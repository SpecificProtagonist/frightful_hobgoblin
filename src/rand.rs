use std::ops::{Range, RangeInclusive};

use nanorand::{RandomGen, RandomRange, Rng, WyRand};

use crate::*;

thread_local! {
    pub static RNG: Cell<WyRand> = default();
}

pub fn rand<T: RandArg>(arg: T) -> T::T {
    let mut rng = RNG.replace(WyRand::new_seed(0));
    let value = T::gen(arg, &mut rng);
    RNG.set(rng);
    value
}

pub trait RandArg {
    type T;
    fn gen(self, rand: &mut WyRand) -> Self::T;
}

impl RandArg for f32 {
    type T = bool;

    fn gen(self, rng: &mut WyRand) -> Self::T {
        self > f32::random(rng)
    }
}

impl RandArg for Range<f32> {
    type T = f32;

    fn gen(self, rng: &mut WyRand) -> Self::T {
        self.start + (self.end - self.start) * f32::random(rng)
    }
}

pub trait Num: RandomRange<WyRand, 8> {}
impl Num for i32 {}
impl Num for usize {}

impl<T: Num> RandArg for Range<T> {
    type T = T;

    fn gen(self, rand: &mut WyRand) -> T {
        T::random_range(rand, self)
    }
}

impl<T: Num> RandArg for RangeInclusive<T> {
    type T = T;

    fn gen(self, rand: &mut WyRand) -> T {
        T::random_range(rand, self)
    }
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

pub fn select<T: Clone>(items: &[(f32, T)], mut selector: f32) -> T {
    let mut chosen = items[0].1.clone();
    for (weight, item) in items {
        chosen = item.clone();
        selector -= weight;
        if selector < 0. {
            break;
        }
    }
    chosen
}

pub fn rand_weighted<T: Clone>(items: &[(f32, T)]) -> T {
    try_rand_weighted(items).unwrap()
}

pub fn try_rand_weighted<T: Clone>(items: &[(f32, T)]) -> Option<T> {
    let total_weight = items.iter().fold(0., |acc, &(weight, _)| acc + weight);
    let rng = rand(0. ..total_weight);
    (total_weight > 0.).then(|| select(items, rng))
}

pub trait SliceExt {
    type Item;
    fn try_choose(&self) -> Option<&Self::Item>;
    fn try_choose_mut(&mut self) -> Option<&mut Self::Item>;
    fn choose(&self) -> &Self::Item;
    fn shuffle(&mut self) -> &mut Self;
}

impl<T> SliceExt for [T] {
    type Item = T;

    fn try_choose(&self) -> Option<&T> {
        self.get(rand(0..self.len()))
    }

    fn try_choose_mut(&mut self) -> Option<&mut T> {
        self.get_mut(rand(0..self.len()))
    }

    fn choose(&self) -> &T {
        self.try_choose().unwrap()
    }

    fn shuffle(&mut self) -> &mut Self {
        let mut rng = RNG.replace(WyRand::new_seed(0));
        Rng::shuffle(&mut rng, &mut *self);
        RNG.set(rng);
        self
    }
}

pub trait IterExt {
    type Item;
    fn shuffled(self) -> Vec<Self::Item>;
}

impl<I: Iterator> IterExt for I {
    type Item = I::Item;

    fn shuffled(self) -> Vec<Self::Item> {
        let mut items: Vec<_> = self.collect();
        items.shuffle();
        items
    }
}
