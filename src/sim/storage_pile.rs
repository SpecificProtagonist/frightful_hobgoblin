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

    pub fn make(
        level: &mut Level,
        untree: &mut Untree,
        target: Vec2,
        target_2: Vec2,
    ) -> (IVec3, Rect, Self) {
        let params = LumberPile {
            axis: if rand(0.5) { HAxis::X } else { HAxis::Y },
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
            |(pos, params), temperature| {
                if rand(0.2) {
                    params.axis = params.axis.rotated()
                } else if rand(0.3) {
                    params.width = rand(3..=5);
                    params.length = rand(5..=6);
                } else {
                    let max_move = (20. * temperature) as i32;
                    *pos += ivec2(rand(-max_move..=max_move), rand(-max_move..=max_move));
                }
                let area = area(*pos, *params);

                if !level.free(area) | (wateryness(level, area) > 0.) {
                    return f32::INFINITY;
                }
                let center_distance = target_2.distance(pos.as_vec2()) / 70.;
                // TODO: use actual pathfinding distance (when there are proper pathable workplaces)
                let worker_distance = target.distance(pos.as_vec2()) / 20.;
                let size_bonus = (params.width + params.length) as f32 * 4.;
                center_distance + worker_distance + unevenness(level, area) * 1. - size_bonus
            },
            100,
            5,
        )
        .unwrap();
        let area = area(pos, params);
        untree.remove_trees(level, area);

        let z = level.height.average(area.border()) as i32;
        (level.height)(area, z);
        (level.blocked)(area, Blocked);
        (pos.extend(z + 1), area, params)
    }
}

#[derive(Component)]
pub struct StonePile {
    pub volume: Cuboid,
}

impl StonePile {
    pub fn make(level: &mut Level, untree: &mut Untree, target: Vec2) -> (Vec3, Rect, Self) {
        let area = optimize(
            Rect {
                min: target.block(),
                max: target.block() + ivec2(rand(3..=4), rand(3..=4)),
            },
            |area, temperature| {
                if rand(0.3) {
                    *area = Rect {
                        min: area.center(),
                        max: area.center() + ivec2(rand(3..=4), rand(3..=4)),
                    }
                } else {
                    let max_move = (20. * temperature) as i32;
                    *area += ivec2(rand(-max_move..=max_move), rand(-max_move..=max_move));
                }
                if !level.free(*area) | (wateryness(level, *area) > 0.) {
                    return f32::INFINITY;
                }
                // TODO: use actual pathfinding distance (when there are proper pathable workplaces)
                let worker_distance = target.distance(area.center_vec2()) / 20.;
                let size_bonus = area.total() as f32 * 4.;
                worker_distance + unevenness(level, *area) * 1. - size_bonus
            },
            100,
            5,
        )
        // TODO
        .unwrap();

        untree.remove_trees(level, area);

        let z = level.height.average(area.border()) as i32 + 1;
        (level.height)(area, z - 1);
        level.fill_at(area, z - 1, PackedMud);
        (level.blocked)(area, Blocked);
        (
            area.center_vec2().extend(z as f32),
            area,
            StonePile {
                volume: Cuboid::new(area.min.extend(z), area.max.extend(z + 2)),
            },
        )
    }

    pub fn max(&self) -> f32 {
        self.volume.volume() as f32
    }
}

#[derive(Event)]
pub struct UpdatePileVisuals;

pub fn update_pile_visuals(
    trigger: Trigger<UpdatePileVisuals>,
    mut level: ResMut<Level>,
    query: Query<(&Pos, &Pile)>,
    lumber: Query<&LumberPile>,
    stone: Query<&StonePile>,
) {
    let Ok((pos, pile)) = query.get(trigger.entity()) else {
        return;
    };

    if let Ok(lumberpile) = lumber.get(trigger.entity()) {
        let amount = pile.goods.get(&Good::Wood).copied().unwrap_or(0.);
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
                        Log(Spruce, LogType::Normal, lumberpile.axis.into())
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

    if let Ok(stonepile) = stone.get(trigger.entity()) {
        let mut leftover = pile.available(Good::Stone, 0);
        for pos in stonepile.volume {
            level(pos, if leftover > 0. { Full(Andesite) } else { Air });
            leftover -= 1.;
        }
    }
}
