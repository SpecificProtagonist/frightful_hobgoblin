use nanorand::{RandomGen, WyRand};

use crate::*;

// Very high frequency noise for testing
pub fn noise2d(column: IVec2) -> f32 {
    let mask = &[
        (ivec2(-2, -1), 0.1),
        (ivec2(-2, 0), 0.1),
        (ivec2(-2, 1), 0.1),
        (ivec2(2, -1), 0.1),
        (ivec2(2, 0), 0.1),
        (ivec2(2, 1), 0.1),
        (ivec2(-1, -2), 0.1),
        (ivec2(0, -2), 0.1),
        (ivec2(1, -2), 0.1),
        (ivec2(-1, 2), 0.1),
        (ivec2(0, 2), 0.1),
        (ivec2(1, 2), 0.1),
        (ivec2(-1, -1), 0.2),
        (ivec2(-1, 1), 0.2),
        (ivec2(1, -1), 0.2),
        (ivec2(1, 1), 0.2),
        (ivec2(-1, 0), 0.5),
        (ivec2(0, -1), 0.5),
        (ivec2(1, 0), 0.5),
        (ivec2(0, 1), 0.5),
        (ivec2(0, 0), 0.6),
    ];

    mask.iter()
        .map(|&(off, weight)| hash(column + off) * weight)
        .sum()
}

pub fn hash(ivec2: IVec2) -> f32 {
    // TODO: maybe do a better hash
    f32::random(&mut WyRand::new_seed(
        ((ivec2.x as u32 as u64) << 5) + ivec2.y as u32 as u64,
    )) * 2.
        - 1.
}
