use crate::*;
use rand::prelude::*;

pub fn choose_starting_area(level: &Level) -> Rect {
    let mut rng = thread_rng();
    let mut area = Rect::new_centered(level.area().center(), IVec2::splat(24));
    let mut old_score = score(level, area);
    let steps = 200;
    for step in 0..steps {
        let temperature = (1. - step as f32 / steps as f32).powf(0.3);
        let max_move = (60. * temperature) as i32;
        let new = area.offset(ivec2(
            rng.gen_range(-max_move, max_move + 1),
            rng.gen_range(-max_move, max_move + 1),
        ));
        if !level.area().subrect(new) {
            continue;
        }
        let new_score = score(level, new);
        if new_score < old_score {
            old_score = new_score;
            area = new;
        }
        println!("{area:?}   {new_score}   {temperature}");
    }
    area
}

fn score(level: &Level, area: Rect) -> f32 {
    let unevennes_curve = 2.;
    let distance_curve = 2.;
    let avg_height = level.average_height(area);
    area.into_iter()
        .map(|pos| {
            (level.height(pos) as f32 - avg_height)
                .abs()
                .powf(unevennes_curve)
                + if level.water_level(pos).is_some() {
                    5.
                } else {
                    0.
                }
        })
        .sum::<f32>()
        / area.total() as f32
        + (area
            .center()
            .as_vec2()
            .distance(level.area().center().as_vec2())
            / level.area().size().as_vec2().min_element()
            * 2.)
            .powf(distance_curve)
}
