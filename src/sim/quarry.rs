use crate::*;
use sim::*;

#[derive(Component, Eq, PartialEq, Copy, Clone)]
pub struct Quarry {
    pub area: Rect,
    // TODO: better shapes / not just in a compas direction
    pub dir: HDir,
}

impl Quarry {
    /// Area used to determine suitability for quarrying
    pub fn probing_area(&self) -> Rect {
        Rect::new_centered(
            self.area.center() + IVec2::from(self.dir) * 9,
            IVec2::splat(11),
        )
    }
}

#[derive(Component)]
pub struct Mason {
    workplace: Entity,
    ready_to_work: bool,
}

#[derive(Component)]
pub struct StonePile {
    volume: Cuboid,
}

pub fn assign_worker(
    mut commands: Commands,
    available: Query<(Entity, &Pos), With<Jobless>>,
    new: Query<(Entity, &Pos), (With<Lumberjack>, Added<Built>)>,
) {
    let assigned = Vec::new();
    for (workplace, pos) in &new {
        let Some((worker, _)) = available
            .iter()
            .filter(|(e, _)| !assigned.contains(e))
            .min_by_key(|(_, p)| p.distance_squared(pos.0) as i32)
        else {
            return;
        };
        commands.entity(worker).remove::<Jobless>().insert(Mason {
            workplace,
            ready_to_work: true,
        });
    }
}

pub fn make_quarry(level: &mut Level, quarry: Quarry) -> PlaceList {
    let floor = level.average_height(quarry.area.border()).round() as i32;

    let cursor = level.recording_cursor();
    remove_trees(level, quarry.area.grow(1));
    for column in quarry.area {
        let mut pos = level.ground(column);
        pos.z = pos.z.min(floor);
        level.height[column] = pos.z;
        level(pos, PackedMud)
    }
    level.fill_at(quarry.area, floor + 1..floor + 5, Air);

    let pos = level.ground(quarry.area.center() + ivec2(rand_range(-2..=2), rand_range(-2..=2)))
        + IVec3::Z;
    level(pos, CraftingTable);
    let pos = level.ground(quarry.area.center() + ivec2(rand_range(-2..=2), rand_range(-2..=2)))
        + IVec3::Z;
    level(pos, Stonecutter(HAxis::X));

    level.pop_recording(cursor).collect()
}

pub fn make_stone_piles(
    mut commands: Commands,
    mut level: ResMut<Level>,
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
                if !level.unblocked(area) | (wateryness(&level, area) > 0.) {
                    return None;
                }
                // TODO: use actual pathfinding distance (when there are proper pathable workplaces)
                let worker_distance = quarry.truncate().distance(area.center_vec2()) / 20.;
                let size_bonus = area.total() as f32 * 4.;
                let score = worker_distance + unevenness(&level, area) * 1. - size_bonus;
                Some((area, score))
            },
            100,
        )
        .unwrap();

        let z = level.average_height(area.border()) as i32 + 1;
        level.set_blocked(area);
        commands.spawn((
            Pos(area.center_vec2().extend(z as f32)),
            StonePile {
                volume: Cuboid::new(area.min.extend(z), area.max.extend(z + 2)),
            },
            Pile {
                goods: default(),
                interact_distance: area.size().x.max(area.size().y),
            },
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
