use crate::*;
use bevy_math::Vec2Swizzles;
use rand::prelude::*;

pub fn roof(level: &mut Level, area: Rect, mut base_z: i32, mat: BlockMaterial) {
    let mut rng = thread_rng();

    let curve = *[straight, straight_high, straight_low, kerb, reverse_kerb]
        .choose(&mut rng)
        .unwrap();

    // TODO: Incorporate this into the curve directly
    #[allow(clippy::fn_address_comparisons)]
    if (curve == kerb) | (curve == straight_high) {
        base_z -= 1
    }

    let size = if area.size().y > area.size().x {
        area.size().yx()
    } else {
        area.size()
    };

    let base_shape = [gable, raised_gable, hip].choose(&mut rng).unwrap();
    let shape = base_shape(base_z as f32, size.as_vec2(), curve);

    let center = area.center_vec2() - Vec2::splat(0.5);
    let shape: Shape = if area.size().y > area.size().x {
        Box::new(move |pos: Vec2| shape((pos - center).yx()))
    } else {
        Box::new(move |pos: Vec2| shape(pos - center))
    };

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
        level[pos.extend(z_block as i32)] |= block;
    }

    for pos in area {
        let z_block = shape(pos.as_vec2()).round() as i32;
        for dir in HDir::ALL {
            // Fix-up outer corners
            if (level[(pos + dir).extend(z_block)] == Stair(mat, dir.rotated(1), Bottom))
                & (level[(pos + dir.rotated(1)).extend(z_block)] == Stair(mat, dir, Bottom))
            {
                level[pos.extend(z_block)] = Stair(mat, dir, Bottom)
            }

            // Fill holes in steep roofs
            let mut lower = shape(pos.as_vec2() + IVec2::from(dir).as_vec2()).round() as i32;
            let adjacent = level[(pos + dir).extend(lower)];
            if matches!(adjacent, Slab(_, Top) | Full(..) | Stair(_, _, Top))
                | matches!(adjacent, Stair(_, d, Bottom) if d==dir.rotated(2))
                | !area.contains(pos + dir)
            {
                lower += 1;
            }
            let mut upper = z_block;
            if matches!(level[pos.extend(upper)], Slab(_, Top) | Stair(_, _, Top)) {
                upper += 1;
            }
            for z in lower..upper {
                level[pos.extend(z)] = if level[pos.extend(z - 1)].solid()
                    || (matches!(level[pos.extend(z)], Full(..) | Stair(..)))
                {
                    Full(mat)
                } else if matches!(level[(pos + dir).extend(z)], Slab(..)) {
                    Slab(mat, Top)
                } else {
                    Stair(mat, dir, Top)
                };
            }
        }
    }
}

type Curve = fn(f32) -> f32;
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

fn tented(base: f32, size: Vec2, curve: Curve) -> Shape {
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

fn half_hip(base: f32, size: Vec2, curve: Curve) -> Shape {
    let size = vec2(size.x + size.y * 0.5, size.y);
    hip(base, size, curve)
}

fn circular(base: f32, radius: f32, curve: Curve) -> Shape {
    Box::new(move |pos: Vec2| base + radius * curve(1. - pos.length() / radius).max(0.))
}
