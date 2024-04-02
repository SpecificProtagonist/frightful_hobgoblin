use crate::*;
use bevy_ecs::prelude::*;
use sim::*;

use super::{lumberjack::TreeIsNearLumberCamp, quarry::Quarry};

#[derive(Component, Deref, DerefMut)]
pub struct Planned(Rect);

#[derive(Component)]
pub struct House {
    area: Rect,
}

#[derive(Component)]
pub struct ToBeBuild;

pub fn unevenness(level: &Level, area: Rect) -> f32 {
    let avg_height = level.average_height(area);
    area.into_iter()
        .map(|pos| (level.height[pos] as f32 - avg_height).abs().powf(2.))
        .sum::<f32>()
        / area.total() as f32
}

pub fn wateryness(level: &Level, area: Rect) -> f32 {
    area.into_iter()
        .filter(|pos| level.water[*pos].is_some())
        .count() as f32
        / area.total() as f32
}

pub fn choose_starting_area(level: &Level) -> Rect {
    optimize(
        Rect::new_centered(level.area().center(), IVec2::splat(44)),
        |mut area, temperature| {
            let max_move = (100. * temperature) as i32;
            area = area.offset(ivec2(
                rand_range(-max_move..=max_move),
                rand_range(-max_move..=max_move),
            ));

            if !level.area().has_subrect(area) {
                return None;
            }
            // TODO: Take biomes into account
            let distance = area
                .center()
                .as_vec2()
                .distance(level.area().center().as_vec2())
                / (level.area().size().as_vec2().min_element() - 40.);
            let score =
                wateryness(level, area) * 20. + unevenness(level, area) + distance.powf(2.) / 2.;
            Some((area, score))
        },
        300,
    )
    .unwrap()
    .shrink(10)
}

// Note this would require apply_defered after each placement
// pub fn remove_outdated(
//     mut commands: Commands,
//     level: Res<Level>,
//     planned: Query<(Entity, &Planned)>,
//     mut new: RemovedComponents<Planned>,
// ) {
//     for entity in new.read() {
//         let Ok(blocked) = blocked.get(entity) else {
//             continue;
//         };
//         for (planned, area) in &planned {
//             if area.overlapps(blocked.0) {
//                 commands.entity(planned).despawn();
//             }
//         }
//     }
// }

pub fn plan_house(
    mut commands: Commands,
    level: Res<Level>,
    planned: Query<(), (With<House>, With<Planned>)>,
    center: Query<&Pos, With<CityCenter>>,
) {
    if planned.iter().len() > 0 {
        return;
    }

    // TODO: On a large map, allow for multiple town centers
    let center = center.single().truncate();
    let Some(area) = optimize(
        Rect::new_centered(
            center.block(),
            ivec2(rand_range(7..=11), rand_range(7..=15)),
        ),
        |mut area, temperature| {
            let max_move = (60. * temperature) as i32;
            area = area.offset(ivec2(
                rand_range(-max_move..=max_move),
                rand_range(-max_move..=max_move),
            ));
            if 0.2 > rand() {
                area = Rect::new_centered(area.center(), area.size().yx())
            }

            if !level.unblocked(area) {
                return None;
            }
            let distance = level.reachability[area.center()] as f32;
            // TODO: try to minimize the amount of trees in the footprint
            let score = wateryness(&level, area) * 20.
                + unevenness(&level, area)
                + (distance / 100.).powf(2.);
            Some((area, score))
        },
        200,
    ) else {
        return;
    };

    commands.spawn((
        Pos(level.ground(area.center()).as_vec3()),
        Planned(area),
        House { area },
    ));
}

pub fn plan_lumberjack(
    mut commands: Commands,
    level: Res<Level>,
    planned: Query<(), (With<Lumberjack>, With<Planned>)>,
    trees: Query<(Entity, &Pos), (With<Tree>, Without<TreeIsNearLumberCamp>)>,
) {
    if !planned.is_empty() {
        return;
    }

    let Some(area) = optimize(
        Rect::new_centered(
            level.area().center(),
            ivec2(rand_range(4..=6), rand_range(5..=8)),
        ),
        |mut area, temperature| {
            let max_move = (60. * temperature) as i32;
            area = area.offset(ivec2(
                rand_range(-max_move..=max_move),
                rand_range(-max_move..=max_move),
            ));
            if 0.2 > rand() {
                area = Rect::new_centered(area.center(), area.size().yx())
            }

            if !level.unblocked(area) {
                return None;
            }
            let center_distance = level.reachability[area.center()].max(150) as f32;
            let tree_access = trees
                .iter()
                .map(|(_, p)| {
                    -1. / ((area.center().as_vec2().distance(p.truncate()) - 10.).max(7.))
                })
                .sum::<f32>();
            let score = wateryness(&level, area) * 20.
                + unevenness(&level, area) * 1.
                + center_distance / 200.
                + tree_access * 5.;
            Some((area, score))
        },
        200,
    ) else {
        return;
    };

    for (tree, pos) in &trees {
        if pos.truncate().distance(area.center_vec2()) < 20. {
            commands.entity(tree).insert(TreeIsNearLumberCamp);
        }
    }

    commands.spawn((
        Pos(level.ground(area.center()).as_vec3()),
        Planned(area),
        Lumberjack { area },
    ));
}

pub fn plan_quarry(
    mut commands: Commands,
    level: Res<Level>,
    planned: Query<(), (With<Quarry>, With<Planned>)>,
) {
    if !planned.is_empty() {
        return;
    }

    let Some(quarry) = optimize(
        Quarry {
            area: Rect::new_centered(level.area().center(), IVec2::splat(7)),
            dir: *HDir::ALL.choose(),
        },
        |mut quarry, temperature| {
            let max_move = (60. * temperature) as i32;
            quarry.area = quarry.area.offset(ivec2(
                rand_range(-max_move..=max_move),
                rand_range(-max_move..=max_move),
            ));
            if 0.3 < rand() {
                quarry.dir = *HDir::ALL.choose();
            }

            if !level.unblocked(quarry.area) | !level.unblocked(quarry.probing_area()) {
                return None;
            }
            let mut distance = level.reachability[quarry.area.center()] as f32 - 650.;
            // Penalize quarries near city center
            if distance < 0. {
                distance *= -5.
            }
            let avg_start_height = level.average_height(quarry.area);
            let quarried_height = level.average_height(quarry.probing_area()) - avg_start_height;
            // TODO: Pit quarries
            // TODO: check how much stone is available instead of checking height differences
            if quarried_height < 5. {
                return None;
            }
            let score = wateryness(&level, quarry.area) * 20.
                + unevenness(&level, quarry.area) * 1.5
                - quarried_height * 1.
                + distance / 100.;
            Some((quarry, score))
        },
        200,
    ) else {
        return;
    };

    commands.spawn((
        Pos(level.ground(quarry.area.center()).as_vec3()),
        Planned(quarry.area),
        quarry,
    ));
}

// Very temporary, just for testing!
pub fn assign_builds(
    mut commands: Commands,
    mut level: ResMut<Level>,
    construction_sites: Query<(), With<ConstructionSite>>,
    houses: Query<(), (With<House>, Without<Planned>)>,
    planned_houses: Query<(Entity, &Planned), With<House>>,
    lumberjacks: Query<(), (With<Lumberjack>, Without<Planned>)>,
    planned_lumberjacks: Query<(Entity, &Planned), With<Lumberjack>>,
    quarries: Query<(), (With<Quarry>, Without<Planned>)>,
    planned_quarries: Query<(Entity, &Planned), With<Quarry>>,
) {
    if construction_sites.iter().len() > 10 {
        return;
    }
    let mut plans = Vec::new();
    if houses.iter().len() < 30 {
        plans.extend(&planned_houses)
    }
    if lumberjacks.iter().len() < 10 {
        plans.extend(&planned_lumberjacks)
    }
    if quarries.iter().len() < 10 {
        plans.extend(&planned_quarries)
    }
    if let Some(&(selected, area)) = plans.try_choose() {
        level.set_blocked(area.0);
        commands
            .entity(selected)
            .remove::<Planned>()
            .insert(ToBeBuild);
    }
}

// TMP
pub fn test_build_house(
    mut replay: ResMut<Replay>,
    mut commands: Commands,
    mut level: ResMut<Level>,
    new: Query<(Entity, &House), With<ToBeBuild>>,
) {
    for (entity, house) in &new {
        replay.dbg(&format!("building house at {:?}", house.area.center()));
        commands
            .entity(entity)
            .remove::<ToBeBuild>()
            .insert(ConstructionSite::new(house::house(&mut level, house.area)));
    }
}

// TMP
pub fn test_build_lumberjack(
    mut commands: Commands,
    mut level: ResMut<Level>,
    new: Query<(Entity, &Lumberjack), With<ToBeBuild>>,
) {
    for (entity, lumberjack) in &new {
        commands
            .entity(entity)
            .remove::<ToBeBuild>()
            .insert(ConstructionSite::new(house::shack(
                &mut level,
                lumberjack.area,
            )));
    }
}

// TMP
pub fn test_build_quarry(
    mut commands: Commands,
    mut level: ResMut<Level>,
    new: Query<(Entity, &Quarry), Added<ToBeBuild>>,
) {
    for (entity, quarry) in &new {
        for pos in quarry.area {
            let pos = level.ground(pos);
            level(pos, Wool(Black))
        }
        for pos in quarry.probing_area() {
            level.blocked[pos] = true;
            let pos = level.ground(pos);
            level(pos, Wool(Red))
        }
        commands
            .entity(entity)
            .remove::<ToBeBuild>()
            .insert(ConstructionSite::new(quarry::make_quarry(
                &mut level, *quarry,
            )));
    }
}
