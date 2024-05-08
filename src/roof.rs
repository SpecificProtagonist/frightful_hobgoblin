use crate::{noise::noise2d, sim::ConsList, *};
use bevy_math::Vec2Swizzles;

use self::sim::ConsItem;

pub fn build_roof(
    level: &mut Level,
    area: Rect,
    base_z: i32,
    shape: &Shape,
    palette: impl Fn(f32, i32) -> BlockMaterial,
) -> ConsList {
    let cursor = level.recording_cursor();

    let mat = |column: IVec2| -> BlockMaterial {
        let mut val = noise2d(column) + 1.5;
        val -= (shape(column.as_vec2()) - base_z as f32) / 5.;
        let distance = (column - area.min)
            .abs()
            .min((column - area.max).abs())
            .min_element();
        palette(val, distance)
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
        let mat = mat(pos);
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
        let mat = mat(pos);
        for dir in HDir::ALL {
            // Fix-up outer corners
            if matches!(level((pos + dir).extend(z_block)), Stair(_, f_dir, Bottom) if f_dir== dir.rotated(1))
                & matches!(
                    level((pos + dir.rotated(1)).extend(z_block)),
                    Stair(_, f_dir, Bottom) if f_dir == dir
                )
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

pub fn palette() -> impl Fn(f32, i32) -> BlockMaterial {
    // This function is kinda ugly but it was a pain to get working
    type C = (&'static [(f32, BlockMaterial)], bool, bool);
    const SLATE: C = (
        &[
            (0., PolishedBlackstoneBrick),
            (0.5, DeepslateTile),
            (1.0, PolishedDeepslate),
            (0., CobbledDeepslate),
        ],
        true,
        true,
    );
    const BRICK: C = (&[(0., Brick), (0., Granite)], true, true);
    const OAK: C = (&[(-0.5, Wood(Spruce)), (0., Wood(Oak))], false, true);
    const SPRUCE: C = (&[(-0.5, Wood(DarkOak)), (0., Wood(Spruce))], false, true);
    const DARK_OAK: C = (&[(-0.5, DeepslateTile), (0., Wood(DarkOak))], false, true);
    const MANGROVE: C = (&[(1., Wood(Mangrove)), (0., Wood(Crimson))], true, true);
    const ANDESITE: C = (&[(-0.5, PolishedAndesite), (0., Andesite)], false, true);
    const CRIMSON: C = (&[(0., Wood(Crimson))], false, false);
    const WARPED: C = (&[(1., Wood(Warped)), (0., DarkPrismarine)], true, true);
    const BIRCH: C = (&[(-0.2, Sandstone), (0., Wood(Birch))], true, false);
    const JUNGLE: C = (&[(0., Wood(Jungle))], false, false);
    const ACACIA: C = (&[(0., Wood(Acacia))], false, false);
    const CHERRY: C = (&[(0., Wood(Cherry))], false, false);
    const MUDBRICK: C = (&[(0., MudBrick)], false, false);
    use Biome::*;
    let (items, use_val, use_distance): (&[(f32, BlockMaterial)], bool, bool) =
        rand_weighted(match center_biome() {
            Plain | Forest | River | Ocean | Beach => {
                &[(0.4, SPRUCE), (0.3, OAK), (0.4, SLATE), (0.1, MANGROVE)]
            }
            Snowy => &[(1.0, SLATE), (0.5, SPRUCE), (0.5, DARK_OAK)],
            Desert => &[
                (1., MUDBRICK),
                (1., BRICK),
                (0.4, ANDESITE),
                (0.4, SPRUCE),
                (0.4, BIRCH),
            ],
            Taiga => &[(1., SPRUCE), (0.1, SLATE), (0.1, MANGROVE), (0.1, DARK_OAK)],
            BirchForest => &[(1., BIRCH), (0.1, ANDESITE), (0.1, MANGROVE)],
            Swamp | MangroveSwamp => &[(1., SPRUCE), (1., MANGROVE), (0.3, CRIMSON), (0.3, WARPED)],
            Jungles => &[(1., JUNGLE), (0.2, ACACIA), (0.2, BRICK)],
            Mesa => &[(1., SPRUCE), (0.5, BRICK)],
            Savanna => &[(1., ACACIA), (0.2, BRICK), (0.2, MUDBRICK)],
            DarkForest => &[(1., DARK_OAK), (0.7, SLATE)],
            CherryGrove => &[(1., CHERRY), (0.2, BIRCH), (0.2, ANDESITE)],
        });
    let items = Vec::from(items);
    move |value, distance| {
        let mut val = if use_val { value } else { 0. };
        if use_distance {
            if distance == 0 {
                val = val * 0.5 - 1.0;
            } else if distance == 1 {
                val = val * 0.7 - 0.4;
            }
        }
        select(&items, val)
    }
}

#[allow(clippy::fn_address_comparisons)]
pub fn roof_shape(mut base_z: i32, area: Rect) -> Shape {
    let rot = area.size().y > area.size().x;
    let size = if rot { area.size().yx() } else { area.size() }.as_vec2();

    use Biome::*;
    let curve: &[(f32, Curve)] = match center_biome() {
        Plain | Forest | Beach | River | BirchForest | DarkForest | CherryGrove => &[
            (1., straight),
            (1., straight_high),
            (1., straight_low),
            (1., kerb),
            (1., reverse_kerb),
        ],
        Ocean => &[
            (1., straight),
            (2., straight_high),
            (1., kerb),
            (1., reverse_kerb),
        ],
        Snowy | Taiga => &[
            (1., straight),
            (2., straight_high),
            (1., kerb),
            (1., reverse_kerb),
        ],
        Desert => &[(1., straight_low)],
        Swamp | MangroveSwamp => &[
            (1., straight),
            (1., straight_low),
            (1., kerb),
            (1., reverse_kerb),
        ],
        Jungles => &[
            (1., straight),
            (1., straight_low),
            (1., kerb),
            (1., reverse_kerb),
        ],
        Mesa | Savanna => &[(1., straight_low)],
    };
    let curve = rand_weighted(curve);

    let hip_base = if (curve == straight_high) | (curve == reverse_kerb) {
        0.3
    } else if curve == kerb {
        0.
    } else {
        1.
    };
    let base_shape: &[(f32, BaseShape)] = match center_biome() {
        Plain | Forest | Beach | River | BirchForest | DarkForest | CherryGrove => {
            &[(1., gable), (1., raised_gable), (hip_base, hip)]
        }
        Ocean => &[(1., gable), (2., raised_gable), (2. * hip_base, hip)],
        Snowy | Taiga => &[(1., gable), (1., raised_gable), (hip_base, hip)],
        Desert => &[(1., gable), (2. * hip_base, hip)],
        Swamp | MangroveSwamp => &[(hip_base, hip)],
        Jungles => &[(1., raised_gable), (2. * hip_base, hip)],
        Mesa | Savanna => &[(1., gable), (1., raised_gable), (1.5 * hip_base, hip)],
    };
    let base_shape = rand_weighted(base_shape);

    // TODO: Incorporate this into the curve directly
    if (curve == kerb) | (curve == straight_high) {
        base_z -= 1
    }
    let shape = base_shape(base_z as f32, size, curve);

    let center = area.center_vec2() - Vec2::splat(0.5);
    if rot {
        Box::new(move |pos: Vec2| shape((pos - center).yx()))
    } else {
        Box::new(move |pos: Vec2| shape(pos - center))
    }
}

type Curve = fn(f32) -> f32;
type BaseShape = fn(f32, Vec2, Curve) -> Shape;
pub type Shape = Box<dyn Fn(Vec2) -> f32>;

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
