use crate::{sim::ConsList, *};
use bevy_math::Vec2Swizzles;

use self::sim::ConsItem;

pub fn roof(level: &mut Level, area: Rect, base_z: i32, mat: BlockMaterial) -> ConsList {
    let cursor = level.recording_cursor();

    let shape = roof_shape(level.biome[area.center()], base_z, area.size().as_vec2());

    let center = area.center_vec2() - Vec2::splat(0.5);
    let shape: Shape = if area.size().y > area.size().x {
        Box::new(move |pos: Vec2| shape((pos - center).yx()))
    } else {
        Box::new(move |pos: Vec2| shape(pos - center))
    };

    // Basic structure
    for pos in area {
        let z = shape(pos.as_vec2());
        let z_block = z.round();
        let mut grad = [
            (shape(pos.as_vec2() + vec2(0.5, 0.)), XPos),
            (shape(pos.as_vec2() + vec2(-0.5, 0.)), XNeg),
            (shape(pos.as_vec2() + vec2(0., 0.5)), YPos),
            (shape(pos.as_vec2() + vec2(0., -0.5)), YNeg),
        ];
        grad.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let block = if grad[3].0 >= z + 0.5 {
            Stair(mat, grad[3].1, Bottom)
        } else if z >= z_block {
            Slab(mat, Top)
        } else {
            Slab(mat, Bottom)
        };
        level(pos, z_block as i32, |b| b | block);
    }

    // Fix-ups
    for pos in area {
        let z_block = shape(pos.as_vec2()).round() as i32;
        for dir in HDir::ALL {
            // Fix-up outer corners
            if (level((pos + dir).extend(z_block)) == Stair(mat, dir.rotated(1), Bottom))
                & (level((pos + dir.rotated(1)).extend(z_block)) == Stair(mat, dir, Bottom))
            {
                level(pos, z_block, Stair(mat, dir, Bottom))
            }

            // Fill holes in steep roofs
            let mut lower = shape(pos.as_vec2() + IVec2::from(dir).as_vec2()).round() as i32;
            let adjacent = level((pos + dir).extend(lower));
            if matches!(adjacent, Slab(_, Top) | Full(..) | Stair(_, _, Top))
                | matches!(adjacent, Stair(_, d, Bottom) if d==dir.rotated(2))
                | !area.contains(pos + dir)
            {
                lower += 1;
            }
            let mut upper = z_block;
            if matches!(level(pos.extend(upper)), Slab(_, Top) | Stair(_, _, Top)) {
                upper += 1;
            }
            for z in lower..upper {
                let block = if level(pos.extend(z - 1)).solid()
                    || (matches!(level(pos.extend(z)), Full(..) | Stair(..)))
                {
                    Full(mat)
                } else if matches!(level((pos + dir).extend(z)), Slab(..)) {
                    Slab(mat, Top)
                } else {
                    Stair(mat, dir, Top)
                };
                level(pos, z, block);
            }
        }
    }

    let mut list = level.pop_recording(cursor).collect::<Vec<_>>();
    list.sort_by_key(|setblock| setblock.pos.z);
    list.into_iter().map(ConsItem::Set).collect()
}

pub fn roof_material(biome: Biome) -> BlockMaterial {
    use Biome::*;
    rand_weighted(match biome {
        Plain | Forest | River | Ocean | Beach => &[
            (1., Wood(Spruce)),
            (0.3, Blackstone),
            (0.1, Wood(Mangrove)),
            (0.1, Wood(DarkOak)),
        ],
        Snowy => &[(1.0, Blackstone), (0.5, Wood(Spruce)), (0.5, Wood(DarkOak))],
        Desert => &[
            (1.0, Andesite),
            (0.4, Granite),
            (0.4, Wood(Spruce)),
            (0.4, Wood(Birch)),
        ],
        Taiga => &[
            (1., Wood(Spruce)),
            (0.1, Blackstone),
            (0.1, Wood(Mangrove)),
            (0.1, Wood(DarkOak)),
        ],
        BirchForest => &[(1., Wood(Birch)), (0.1, Blackstone), (0.1, Wood(Mangrove))],
        Swamp | MangroveSwamp => &[
            (1., Wood(Spruce)),
            (1., Wood(Mangrove)),
            (0.3, Wood(Crimson)),
            (0.3, Wood(Warped)),
        ],
        Jungles => &[(1., Wood(Jungle)), (0.2, Wood(Acacia))],
        Mesa => &[(1., Wood(Spruce)), (0.5, Brick)],
        Savanna => &[(1., Wood(Acacia)), (0.2, Granite), (0.2, MudBrick)],
        DarkForest => &[(1., Wood(DarkOak)), (0.7, Blackstone)],
        CherryGrove => &[(1., Wood(Cherry)), (0.2, Wood(Birch)), (0.2, Diorite)],
    })
}

fn roof_shape(biome: Biome, mut base_z: i32, size: Vec2) -> Shape {
    use Biome::*;
    // Hip roofs only work on relatively square footprints
    // TODO: check that they still generate often enough
    // let hip_base = size.min_element() as f32 / size.max_element() as f32;
    let hip_base = (1.0 - (size.max_element() - size.min_element()) / 4.).min(0.);
    let (curve, base_shape): (&[(f32, Curve)], &[(f32, BaseShape)]) = match biome {
        Plain | Forest | Beach | River | BirchForest | DarkForest | CherryGrove => (
            &[
                (1., straight),
                (1., straight_high),
                (1., straight_low),
                (1., kerb),
                (1., reverse_kerb),
            ],
            &[(1., gable), (1., raised_gable), (hip_base, hip)],
        ),
        Ocean => (
            &[
                (1., straight),
                (2., straight_high),
                (1., kerb),
                (1., reverse_kerb),
            ],
            &[(1., gable), (2., raised_gable), (2. * hip_base, hip)],
        ),
        Snowy | Taiga => (
            &[
                (1., straight),
                (2., straight_high),
                (1., kerb),
                (1., reverse_kerb),
            ],
            &[(1., gable), (1., raised_gable), (hip_base, hip)],
        ),
        Desert => (
            // TODO: just use flat roofs
            &[(1., straight_low)],
            &[(1., gable), (2. * hip_base, hip)],
        ),
        Swamp | MangroveSwamp => (
            &[
                (1., straight),
                (1., straight_low),
                (1., kerb),
                (1., reverse_kerb),
            ],
            &[(hip_base, hip)],
        ),
        Jungles => (
            &[
                (1., straight),
                (1., straight_low),
                (1., kerb),
                (1., reverse_kerb),
            ],
            &[(1., raised_gable), (2. * hip_base, hip)],
        ),
        Mesa | Savanna => (
            &[(1., straight_low)],
            &[(1., gable), (1., raised_gable), (1.5 * hip_base, hip)],
        ),
    };
    let curve = rand_weighted(curve);
    let base_shape = rand_weighted(base_shape);

    // TODO: Incorporate this into the curve directly
    #[allow(clippy::fn_address_comparisons)]
    if (curve == kerb) | (curve == straight_high) {
        base_z -= 1
    }
    base_shape(base_z as f32, size, curve)
}

type Curve = fn(f32) -> f32;
type BaseShape = fn(f32, Vec2, Curve) -> Shape;
type Shape = Box<dyn Fn(Vec2) -> f32>;

fn straight(frac: f32) -> f32 {
    frac
}

fn straight_low(frac: f32) -> f32 {
    frac * 0.5
}

fn straight_high(frac: f32) -> f32 {
    frac * 2.
}

fn kerb(frac: f32) -> f32 {
    (frac * 2.).min(frac * 0.5 + 0.13)
}

fn reverse_kerb(frac: f32) -> f32 {
    (frac * 0.5).max(frac * 2. - 0.25)
}

fn gable(base: f32, size: Vec2, curve: Curve) -> Shape {
    // let base = base - curve(-1. / size.y) * size.y - 1.;
    Box::new(move |pos: Vec2| base + size.y * curve(0.5 - pos.y.abs() / size.y))
}

fn hip(base: f32, size: Vec2, curve: Curve) -> Shape {
    // let base = base - curve(-1. / size.y) * size.y - 1.;
    Box::new(move |pos: Vec2| {
        let scale = size.y.min(size.x);
        base + scale * curve((0.5 * size.y - pos.y.abs()).min(0.5 * size.x - pos.x.abs()) / scale)
    })
}

fn _tented(base: f32, size: Vec2, curve: Curve) -> Shape {
    // let base = base - curve(-1. / size.y) * size.y - 1.;
    Box::new(move |pos: Vec2| {
        base + size.y.min(size.x)
            * curve((0.5 - pos.y.abs() / size.y).min(0.5 - pos.x.abs() / size.x))
    })
}

fn raised_gable(base: f32, size: Vec2, curve: Curve) -> Shape {
    // let base = base - curve(-1. / size.y) * size.y - 1.;
    let scale = (4. + size.y) * size.x.powf(0.1) * 0.03;
    Box::new(move |pos: Vec2| {
        base + size.y * curve(0.5 - pos.y.abs() / size.y)
            + (pos.x.abs() * 2. / size.y).powf(1.9) * scale
    })
}

fn _half_hip(base: f32, size: Vec2, curve: Curve) -> Shape {
    let size = vec2(size.x + size.y * 0.5, size.y);
    hip(base, size, curve)
}

fn _circular(base: f32, radius: f32, curve: Curve) -> Shape {
    Box::new(move |pos: Vec2| base + radius * curve(1. - pos.length() / radius).max(0.))
}
