use super::*;

#[derive(Component, Eq, PartialEq, Copy, Clone)]
pub struct LumberPile {
    pub axis: HAxis,
    pub width: i32,
    pub length: i32,
}

impl LumberPile {
    pub fn max(&self) -> f32 {
        (self.length
            * 4
            * match self.width {
                3 => 7,
                4 => 10,
                5 => 13,
                _ => unreachable!(),
            }) as f32
    }

    pub fn make(level: &mut Level, target: Vec2, target_2: Vec2) -> (IVec3, Self) {
        let params = LumberPile {
            axis: if 0.5 > rand() { HAxis::X } else { HAxis::Y },
            width: 3,
            length: 5,
        };
        let area = |pos, params: LumberPile| {
            Rect::new_centered(
                pos,
                match params.axis {
                    HAxis::X => ivec2(params.length, params.width),
                    HAxis::Y => ivec2(params.width, params.length),
                },
            )
        };
        let (pos, params) = optimize(
            (target.block(), params),
            |(mut pos, mut params), temperature| {
                if 0.2 > rand() {
                    params.axis = params.axis.rotated()
                } else if 0.3 > rand() {
                    params.width = rand_range(3..=5);
                    params.length = rand_range(5..=6);
                } else {
                    let max_move = (20. * temperature) as i32;
                    pos += ivec2(
                        rand_range(-max_move..=max_move),
                        rand_range(-max_move..=max_move),
                    );
                }
                let area = area(pos, params);

                if !level.unblocked(area) | (wateryness(&level, area) > 0.) {
                    return None;
                }
                let center_distance = target_2.distance(pos.as_vec2()) / 70.;
                // TODO: use actual pathfinding distance (when there are proper pathable workplaces)
                let worker_distance = target.distance(pos.as_vec2()) / 20.;
                let size_bonus = (params.width + params.length) as f32 * 4.;
                let score =
                    center_distance + worker_distance + unevenness(&level, area) * 1. - size_bonus;
                Some(((pos, params), score))
            },
            100,
        )
        .unwrap();

        let z = level.average_height(area(pos, params).border()) as i32;
        (level.height)(area(pos, params), z);
        level.set_blocked(area(pos, params));
        (pos.extend(z + 1), params)
    }
}

pub fn update_lumber_pile_visuals(
    mut level: ResMut<Level>,
    query: Query<(&Pos, &LumberPile, &Pile), Changed<Pile>>,
) {
    for (pos, lumberpile, pile) in &query {
        let amount = pile.get(&Good::Wood).copied().unwrap_or_default();
        let logs = (amount / (4. * lumberpile.length as f32)).round() as usize;
        let log_positions: &[(i32, i32)] = match lumberpile.width {
            3 => &[(0, 0), (-1, 0), (1, 0), (0, 1), (1, 1), (-1, 1), (0, 2)],
            4 => &[
                (0, 0),
                (-1, 0),
                (1, 0),
                (2, 0),
                (1, 1),
                (0, 1),
                (-1, 1),
                (0, 2),
                (2, 1),
                (1, 2),
                // (-1, 2),
                // (0, 3),
                // (2, 2),
                // (1, 3),
            ],
            5 => &[
                (0, 0),
                (-1, 0),
                (-2, 0),
                (1, 0),
                (2, 0),
                (1, 1),
                (0, 1),
                (-1, 1),
                (0, 2),
                (2, 1),
                (1, 2),
                (-2, 1),
                (-1, 2),
            ],
            _ => unreachable!(),
        };
        for (i, (side, z)) in log_positions.iter().copied().enumerate() {
            for along in -lumberpile.length / 2..=(lumberpile.length + 1) / 2 {
                level(
                    pos.block()
                        + (lumberpile.axis.pos() * along + lumberpile.axis.rotated().pos() * side)
                            .extend(z),
                    if i < logs {
                        Log(Spruce, LogType::Normal(lumberpile.axis.into()))
                    } else {
                        Air
                    },
                )
            }
        }
        for along in [1 - lumberpile.length / 2, (lumberpile.length + 1) / 2 - 1] {
            for side in -(lumberpile.width - 1) / 2..=lumberpile.width / 2 {
                let mut pos = pos.block()
                    + (lumberpile.axis.pos() * along + lumberpile.axis.rotated().pos() * side)
                        .extend(1);
                if !level(pos - IVec3::Z).solid() {
                    continue;
                }
                while level(pos).solid() {
                    pos.z += 1
                }
                level(
                    pos,
                    if logs == 0 {
                        Air
                    } else {
                        Rail(lumberpile.axis)
                    },
                )
            }
        }
    }
}

#[derive(Component)]
pub struct StonePile {
    pub volume: Cuboid,
}

impl StonePile {
    pub fn make(level: &mut Level, target: Vec2) -> (Vec3, Self) {
        let area = optimize(
            Rect {
                min: target.block(),
                max: target.block() + ivec2(rand_range(3..=4), rand_range(3..=4)),
            },
            |area, temperature| {
                let area = if 0.3 > rand() {
                    Rect {
                        min: area.center(),
                        max: area.center() + ivec2(rand_range(3..=4), rand_range(3..=4)),
                    }
                } else {
                    let max_move = (20. * temperature) as i32;
                    area.offset(ivec2(
                        rand_range(-max_move..=max_move),
                        rand_range(-max_move..=max_move),
                    ))
                };
                if !level.unblocked(area) | (wateryness(&level, area) > 0.) {
                    return None;
                }
                // TODO: use actual pathfinding distance (when there are proper pathable workplaces)
                let worker_distance = target.distance(area.center_vec2()) / 20.;
                let size_bonus = area.total() as f32 * 4.;
                let score = worker_distance + unevenness(&level, area) * 1. - size_bonus;
                Some((area, score))
            },
            100,
        )
        .unwrap();

        let z = level.average_height(area.border()) as i32 + 1;
        (level.height)(area, z - 1);
        level.fill_at(area, z - 1, PackedMud);
        level.set_blocked(area);
        (
            area.center_vec2().extend(z as f32),
            StonePile {
                volume: Cuboid::new(area.min.extend(z), area.max.extend(z + 2)),
            },
        )
    }
}

pub fn update_stone_pile_visuals(
    mut level: ResMut<Level>,
    query: Query<(&StonePile, &Pile), Changed<Pile>>,
) {
    for (stonepile, pile) in &query {
        let mut leftover = pile.get(&Good::Stone).copied().unwrap_or(0.);
        for pos in stonepile.volume {
            level(
                pos,
                if leftover > 0. {
                    Full(SmoothStone)
                } else {
                    Air
                },
            );
            leftover -= 1.;
        }
    }
}
