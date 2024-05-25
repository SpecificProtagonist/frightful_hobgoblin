use nanorand::WyRand;

use crate::*;

pub fn rand<Generated>() -> Generated
where
    Generated: nanorand::RandomGen<WyRand, 8>,
{
    let mut rng = RNG.replace(WyRand::new_seed(0));
    let value = Generated::random(&mut rng);
    RNG.set(rng);
    value
}

pub fn rand_range<Number, Bounds>(range: Bounds) -> Number
where
    Number: nanorand::RandomRange<WyRand, 8>,
    Bounds: std::ops::RangeBounds<Number>,
{
    let mut rng = RNG.replace(WyRand::new_seed(0));
    let value = Number::random_range(&mut rng, range);
    RNG.set(rng);
    value
}

pub fn rand_f32(min: f32, max: f32) -> f32 {
    min + (max - min) * rand::<f32>()
}

pub fn rand_1(prob: f32) -> i32 {
    if prob > rand() {
        if 0.5 > rand() {
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
    let rng = total_weight * rand::<f32>();
    (total_weight > 0.).then(|| select(items, rng))
}

pub trait ChooseExt {
    type Item;
    fn try_choose(&self) -> Option<&Self::Item>;
    fn try_choose_mut(&mut self) -> Option<&mut Self::Item>;
    fn choose(&self) -> &Self::Item;
}

impl<T> ChooseExt for [T] {
    type Item = T;

    fn try_choose(&self) -> Option<&T> {
        self.get(rand_range(0..self.len()))
    }

    fn try_choose_mut(&mut self) -> Option<&mut T> {
        self.get_mut(rand_range(0..self.len()))
    }

    fn choose(&self) -> &T {
        self.try_choose().unwrap()
    }
}

thread_local! {
    pub static RNG: Cell<WyRand> = default();
}
