use std::f32::consts::PI;

use crate::*;
use itertools::Itertools;
use sim::*;

use self::{
    desire_lines::{add_desire_line, DesireLines},
    pathfind::{pathfind, PathingNode},
    trees::{Tree, TreeGen, TreeState, Trees},
};

/// Direction: From center outwards
#[derive(Resource)]
pub struct Roads(pub Vec<VecDeque<PathingNode>>);

pub fn init_roads(
    mut commands: Commands,
    mut tree_map: ResMut<Trees>,
    mut level: ResMut<Level>,
    mut dl: ResMut<DesireLines>,
    city_center: Query<&Pos, With<CityCenter>>,
) {
    let center = city_center.single().block().truncate();
    let ray_start = center.as_vec2() * 0.5 + level.area().center_vec2() * 0.5;
    let count = 5;
    let paths = (0..count)
        .map(|i| {
            // Find path
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

            // Build road
            for node in &path.path {
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
            path.path
        })
        .collect_vec();

    // Line with trees
    // TODO: Hedges, other trees
    for path in &paths {
        let mut points_left = Vec::new();
        let mut points_right = Vec::new();
        for i in 50.. {
            let Some(upcomming) = path.get(i + 5).map(|n| n.pos.truncate().as_vec2()) else {
                break;
            };
            let prev = path[i - 5].pos.truncate().as_vec2();
            if upcomming == prev {
                continue;
            }
            let point = path[i].pos;
            let dir = (upcomming - prev).normalize();
            let side = vec2(dir.y, -dir.x) * 3.5;
            let pos = point.truncate().as_vec2() + vec2(0.5, 0.5);
            points_left.push((point.z, pos + side));
            points_right.push((point.z, pos - side));
        }
        let mut make_trees = |points: Vec<(i32, Vec2)>| {
            let mut prev = Vec2::default();
            'outer: for (path_z, point) in points {
                let pos = level.ground(point.block());
                if (point.distance(prev) < 6.)
                    | ((pos.z + 1 - path_z).abs() > 1)
                    | !level(pos).dirtsoil()
                    | level.water[pos].is_some()
                    || !level.free(
                        (-1..=1)
                            .flat_map(move |x| (-1..1).map(move |y| pos.truncate() + ivec2(x, y))),
                    )
                {
                    continue;
                }
                for column in Rect::new_centered(pos.truncate(), IVec2::splat(7)) {
                    if tree_map[column].is_some() {
                        continue 'outer;
                    }
                }
                prev = point;
                tree_map[point.block()] = Some(
                    commands
                        .spawn((
                            Pos((pos + IVec3::Z).as_vec3()),
                            Tree {
                                blocks: default(),
                                state: TreeState::Decorative,
                            },
                            TreeGen::Cypress.make(),
                        ))
                        .id(),
                );
            }
        };
        if rand(0.6) {
            make_trees(points_left);
        }
        if rand(0.6) {
            make_trees(points_right);
        }
    }

    commands.insert_resource(Roads(paths));
}
