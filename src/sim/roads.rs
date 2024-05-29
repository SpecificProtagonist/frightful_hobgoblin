use std::f32::consts::PI;

use crate::*;
use itertools::Itertools;
use sim::*;

use self::{
    desire_lines::{add_desire_line, DesireLines},
    pathfind::pathfind,
};

pub fn init_roads(
    mut level: ResMut<Level>,
    mut dl: ResMut<DesireLines>,
    city_center: Query<&Pos, With<CityCenter>>,
) {
    let center = city_center.single().block().truncate();
    let ray_start = center.as_vec2() * 0.5 + level.area().center_vec2() * 0.5;
    let count = 5;
    for i in 0..count {
        let angle = (i as f32 + rand(0. ..0.4)) * 2. * PI / count as f32;
        let direction = vec2(angle.cos(), angle.sin());
        let mut pos = ray_start;
        while level.area().contains(pos.block()) {
            pos += direction;
        }
        pos += direction * 5.;
        let start = level.ground(center) + IVec3::Z;
        let end = level.ground(pos.block()) + IVec3::Z;
        let path = pathfind(&level, start, end, 10);
        // if !path.success {
        //     continue;
        // }
        for node in path.path {
            for (x_off, y_off) in (-1..=1).cartesian_product(-1..=1) {
                let offset = ivec2(x_off, y_off);
                if !node.boat {
                    level.blocked[node.pos.truncate() + offset] = Street;
                }
                for _ in 0..10 {
                    add_desire_line(&mut level, &mut dl, node.pos + offset.extend(-1));
                }
            }
        }
    }
}
