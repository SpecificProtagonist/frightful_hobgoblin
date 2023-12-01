use crate::*;
use sim::*;

#[derive(Component)]
pub struct Quarry;

#[derive(Component)]
pub struct StonePile {
    volume: Cuboid,
}

pub fn make_stone_piles(
    mut commands: Commands,
    level: ResMut<Level>,
    blocked: Query<&Blocked>,
    new_quarries: Query<&Pos, (With<Quarry>, Added<Built>)>,
) {
    for quarry in &new_quarries {
        let area = optimize(
            Rect {
                min: quarry.block().truncate(),
                max: quarry.block().truncate() + ivec2(rand_range(3..=4), rand_range(3..=4)),
            },
            |area, temperature| {
                let area = if 0.3 > rand() {
                    Rect {
                        min: quarry.block().truncate(),
                        max: quarry.block().truncate()
                            + ivec2(rand_range(3..=4), rand_range(3..=4)),
                    }
                } else {
                    let max_move = (20. * temperature) as i32;
                    area.offset(ivec2(
                        rand_range(-max_move..=max_move),
                        rand_range(-max_move..=max_move),
                    ))
                };
                (level.area().has_subrect(area)
                    & not_blocked(&blocked, area)
                    & (wateryness(&level, area) == 0.))
                    .then_some(area)
            },
            |area| {
                // TODO: use actual pathfinding distance (when there are proper pathable workplaces)
                let worker_distance = quarry.truncate().distance(area.center_vec2()) / 20.;
                let size_bonus = area.total() as f32 * 4.;
                worker_distance + unevenness(&level, *area) * 1. - size_bonus
            },
            100,
        );

        let z = level.average_height(area.border()) as i32 + 1;
        commands.spawn((
            Pos(area.center_vec2().extend(z as f32)),
            StonePile {
                volume: Cuboid::new(area.min.extend(z), area.max.extend(z + 2)),
            },
            Pile {
                goods: default(),
                interact_distance: area.size().x.max(area.size().y),
            },
            Blocked(area),
        ));

        // TODO: Clear trees here
    }
}

pub fn update_stone_pile_visuals(
    mut level: ResMut<Level>,
    query: Query<(&StonePile, &Pile), Changed<Pile>>,
) {
    for (stonepile, pile) in &query {
        level.fill_at(stonepile.volume.d2(), stonepile.volume.min.z - 1, PackedMud);
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
