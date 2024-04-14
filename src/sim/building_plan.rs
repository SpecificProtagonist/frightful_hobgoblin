use std::f32::INFINITY;

use crate::*;
use bevy_ecs::prelude::*;
use sim::*;

use self::{
    quarry::Quarry,
    trees::{Tree, Untree},
};

use super::lumberjack::TreeIsNearLumberCamp;

#[derive(Component, Deref, DerefMut)]
pub struct Planned(pub Vec<IVec2>);

#[derive(Component)]
pub struct House {
    area: Rect,
}

#[derive(Component)]
pub struct ToBeBuild;

pub fn unevenness(level: &Level, area: Rect) -> f32 {
    let avg_height = level.height.average(area);
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
        |area, temperature| {
            let max_move = (100. * temperature) as i32;
            *area += ivec2(
                rand_range(-max_move..=max_move),
                rand_range(-max_move..=max_move),
            );

            if !level.area().has_subrect(*area) {
                return INFINITY;
            }
            // TODO: try place near river
            // TODO: Take biomes into account
            let distance = area
                .center()
                .as_vec2()
                .distance(level.area().center().as_vec2())
                / (level.area().size().as_vec2().min_element() - 40.);
            wateryness(level, *area) * 20. + unevenness(level, *area) + distance.powf(2.) / 2.
        },
        300,
    )
    .unwrap()
    .shrink(10)
}

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
        |area, temperature| {
            let max_move = (60. * temperature) as i32;
            *area += ivec2(
                rand_range(-max_move..=max_move),
                rand_range(-max_move..=max_move),
            );
            if 0.2 > rand() {
                *area = Rect::new_centered(area.center(), area.size().yx())
            }

            if !level.unblocked(*area) {
                return INFINITY;
            }
            let distance = level.reachability[area.center()] as f32;
            // TODO: try to minimize the amount of trees in the footprint
            wateryness(&level, *area) * 20. + unevenness(&level, *area) + (distance / 100.).powf(2.)
        },
        200,
    ) else {
        return;
    };

    commands.spawn((
        Pos(level.ground(area.center()).as_vec3()),
        Planned(area.into_iter().collect()),
        House { area },
    ));
}

pub fn plan_lumberjack(
    mut commands: Commands,
    level: Res<Level>,
    planned: Query<(), (With<LumberjackShack>, With<Planned>)>,
    trees: Query<(Entity, &Pos), (With<Tree>, Without<TreeIsNearLumberCamp>)>,
) {
    if !planned.is_empty() {
        return;
    }

    // TODO: Seperate focus and shack position selection
    let Some(area) = optimize(
        Rect::new_centered(
            level.area().center(),
            ivec2(rand_range(4..=6), rand_range(5..=8)),
        ),
        |area, temperature| {
            let max_move = (60. * temperature) as i32;
            *area += ivec2(
                rand_range(-max_move..=max_move),
                rand_range(-max_move..=max_move),
            );
            if 0.2 > rand() {
                *area = Rect::new_centered(area.center(), area.size().yx())
            }

            if !level.unblocked(*area) {
                return INFINITY;
            }
            let center_distance = level.reachability[area.center()].max(150) as f32;
            let tree_access = trees
                .iter()
                .map(|(_, p)| {
                    -1. / ((area.center().as_vec2().distance(p.truncate()) - 10.).max(7.))
                })
                .sum::<f32>();
            wateryness(&level, *area) * 20.
                + unevenness(&level, *area) * 1.
                + center_distance / 200.
                + tree_access * 5.
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

    commands.spawn((Pos(level.ground(area.center()).as_vec3()), LumberjackFocus));
    commands.spawn((
        Pos(level.ground(area.center()).as_vec3()),
        Planned(area.into_iter().collect()),
        LumberjackShack { area },
    ));
}

pub fn upgrade_plaza(
    mut commands: Commands,
    mut level: ResMut<Level>,
    tick: Res<Tick>,
    center: Query<(Entity, &CityCenter)>,
) {
    if tick.0 != 1000 {
        return;
    }
    let (entity, rect) = center.single();

    let cursor = level.recording_cursor();
    let mut rec = ConsList::new();

    // Visit blocks in a spiral from the center
    let mut offset = IVec2::ZERO;
    let mut dir = HDir::YNeg;
    for _ in 0..rect.size().max_element().pow(2) {
        // Rounded corners
        let metr = offset.as_vec2().powf(4.);
        if metr.x + metr.y < (rect.size().max_element() as f32 / 2.).powf(4.) + 0.6 {
            let pos = level.ground(rect.center() + offset);
            level(pos, Path);
            if 0.2 > rand() {
                rec.push_back(ConsItem::Goto(MoveTask {
                    goal: pos + IVec3::Z,
                    distance: 2,
                }));
            }
            rec.extend(level.pop_recording(cursor.clone()).map(ConsItem::Set));
        }
        if (offset.x == offset.y)
            | (offset.x < 0) & (offset.x == -offset.y)
            | (offset.x > 0) & (offset.x == 1 - offset.y)
        {
            dir = dir.rotated(1);
        }
        offset += dir;
    }
    commands.entity(entity).insert(ConstructionSite::new(rec));
}

// Very temporary, just for testing!
pub fn assign_builds(
    mut commands: Commands,
    mut level: ResMut<Level>,
    construction_sites: Query<(), With<ConstructionSite>>,
    houses: Query<(), (With<House>, Without<Planned>)>,
    planned_houses: Query<(Entity, &Planned), With<House>>,
    lumberjacks: Query<(), (With<LumberjackShack>, Without<Planned>)>,
    planned_lumberjacks: Query<(Entity, &Planned), With<LumberjackShack>>,
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
    if quarries.iter().len() < 5 {
        plans.extend(&planned_quarries);
    }
    if let Some(&(selected, area)) = plans.try_choose() {
        if area.iter().any(|c| level.blocked[*c]) {
            commands.entity(selected).despawn();
        } else {
            level.set_blocked(area.iter().copied());
            commands
                .entity(selected)
                .remove::<Planned>()
                .insert(ToBeBuild);
        }
    }
}

// TMP
pub fn test_build_house(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut untree: Untree,
    new: Query<(Entity, &House), With<ToBeBuild>>,
) {
    for (entity, house) in &new {
        commands
            .entity(entity)
            .remove::<ToBeBuild>()
            .insert(ConstructionSite::new(house::house(
                &mut level,
                &mut untree,
                house.area,
            )));
    }
}

// TMP
pub fn test_build_lumberjack(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut untree: Untree,
    new: Query<(Entity, &LumberjackShack), With<ToBeBuild>>,
) {
    for (entity, lumberjack) in &new {
        commands
            .entity(entity)
            .remove::<ToBeBuild>()
            .insert(ConstructionSite::new(house::shack(
                &mut level,
                &mut untree,
                lumberjack.area,
            )));
    }
}
