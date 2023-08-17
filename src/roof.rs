use crate::*;
use rand::prelude::*;

pub fn roof(level: &mut Level, base: IVec3) {
    let mut rng = thread_rng();
    let mat = Wood(Cherry);
    let middle = base.truncate();
    let area = Rect::new_centered(
        middle,
        ivec2(1 + rng.gen_range(6, 20), rng.gen_range(5, 14)),
    );
    let shape = gable(base.z as f32, area.size().as_vec2(), kerb);
    for pos in area {
        let rel = (pos - middle).as_vec2();
        let z = shape(rel);
        let z_block = z.round();
        let mut grad = [
            (shape(rel + vec2(0.5, 0.)), XPos),
            (shape(rel + vec2(-0.5, 0.)), XNeg),
            (shape(rel + vec2(0., 0.5)), YPos),
            (shape(rel + vec2(0., -0.5)), YNeg),
        ];
        grad.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let block = if grad[3].0 >= z + 0.5 {
            Stair(mat, grad[3].1, Bottom)
        } else if z >= z_block {
            Slab(mat, Top)
        } else {
            Slab(mat, Bottom)
        };
        level[pos.extend(z_block as i32)] = block;
    }
    for pos in area {
        let rel = (pos - middle).as_vec2();
        let z_block = shape(rel).round() as i32;
        for dir in HDir::ALL {
            // Fix-up outer corners
            if (level[(pos + dir).extend(z_block)] == Stair(mat, dir.rotated(1), Bottom))
                & (level[(pos + dir.rotated(1)).extend(z_block)] == Stair(mat, dir, Bottom))
            {
                level[pos.extend(z_block)] = Stair(mat, dir, Bottom)
            }

            // Fill holes in steep roofs
            let mut lower = shape(rel + IVec2::from(dir).as_vec2()).round() as i32;
            let adjacent = level[(pos + dir).extend(lower)];
            if matches!(adjacent, Slab(_, Top) | Stair(..) | Full(..)) | !area.contains(pos + dir) {
                lower += 1;
            }
            let mut upper = z_block;
            if level[pos.extend(upper)] == Slab(mat, Top) {
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
type Z = Box<dyn Fn(Vec2) -> f32>;

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

fn gable(base: f32, size: Vec2, curve: Curve) -> Z {
    let base = base - curve(-1. / size.x) * size.x - 1.;
    Box::new(move |pos: Vec2| base + size.x * curve(0.5 - pos.y.abs() / size.x))
}

fn hip(base: f32, size: Vec2, curve: Curve) -> Z {
    let base = base - curve(-1. / size.x) * size.x - 1.;
    Box::new(move |pos: Vec2| {
        let scale = size.x.min(size.y);
        base + scale * curve((0.5 * size.x - pos.y.abs()).min(0.5 * size.y - pos.x.abs()) / scale)
    })
}

fn tented(base: f32, size: Vec2, curve: Curve) -> Z {
    let base = base - curve(-1. / size.x) * size.x - 1.;
    Box::new(move |pos: Vec2| {
        base + size.x.min(size.y)
            * curve((0.5 - pos.y.abs() / size.x).min(0.5 - pos.x.abs() / size.y))
    })
}

fn raised_gable(base: f32, size: Vec2, curve: Curve) -> Z {
    let base = base - curve(-1. / size.x) * size.x - 1.;
    let scale = size.x.powf(0.3) * size.y.powf(0.) * 0.2;
    Box::new(move |pos: Vec2| {
        base + size.x * curve(0.5 - pos.y.abs() / size.x)
            + (pos.x.abs() * 2. / size.x).powf(1.9) * scale
    })
}
