use crate::*;
use bevy_ecs::prelude::*;
use sim::*;

use super::lumberjack::TreeIsNearLumberCamp;

// TODO: instead store
#[derive(Component, Deref, DerefMut)]
pub struct Blocked(pub Rect);

#[derive(Component, Deref, DerefMut)]
pub struct Planned(Rect);

#[derive(Component)]
pub struct House;

#[derive(Component)]
pub struct Quarry;

#[derive(Component)]
pub struct ToBeBuild;

pub fn not_blocked<'a>(blocked: impl IntoIterator<Item = &'a Blocked>, area: Rect) -> bool {
    blocked.into_iter().all(|blocker| !blocker.overlapps(area))
}

pub fn unevenness(level: &Level, area: Rect) -> f32 {
    let avg_height = level.average_height(area);
    area.into_iter()
        .map(|pos| (level.height(pos) as f32 - avg_height).abs().powf(2.))
        .sum::<f32>()
        / area.total() as f32
}

pub fn wateryness(level: &Level, area: Rect) -> f32 {
    area.into_iter()
        .filter(|pos| level.water_level(*pos).is_some())
        .count() as f32
        / area.total() as f32
}

pub fn choose_starting_area(level: &Level) -> Rect {
    optimize(
        Rect::new_centered(level.area().center(), IVec2::splat(44)),
        |area, temperature| {
            let max_move = (100. * temperature) as i32;
            let new = area.offset(ivec2(
                rand_range(-max_move..=max_move),
                rand_range(-max_move..=max_move),
            ));
            level.area().subrect(new).then_some(new)
        },
        |area| {
            let distance = area
                .center()
                .as_vec2()
                .distance(level.area().center().as_vec2())
                / (level.area().size().as_vec2().min_element() - 40.);
            wateryness(level, *area) * 20. + unevenness(level, *area) + distance.powf(2.) / 2.
        },
        300,
    )
    .shrink(10)
}

// Note this would require apply_defered after each placement
pub fn remove_outdated(
    mut commands: Commands,
    planned: Query<(Entity, &Planned)>,
    blocked: Query<&Blocked>,
    mut new: RemovedComponents<Planned>,
) {
    for entity in new.iter() {
        let Ok(blocked) = blocked.get(entity) else {
            continue;
        };
        for (planned, area) in &planned {
            if area.overlapps(blocked.0) {
                commands.entity(planned).despawn();
            }
        }
    }
}

pub fn plan_house(
    mut commands: Commands,
    level: Res<Level>,
    blocked: Query<&Blocked>,
    planned: Query<(With<House>, With<Planned>)>,
    center: Query<&Pos, With<CityCenter>>,
) {
    if planned.iter().len() > 0 {
        return;
    }

    let center = center.single().truncate();
    let start = Rect::new_centered(
        center.block(),
        ivec2(rand_range(7..=11), rand_range(7..=15)),
    );
    let area = optimize(
        start,
        |area, temperature| {
            let max_move = (60. * temperature) as i32;
            let mut new = area.offset(ivec2(
                rand_range(-max_move..=max_move),
                rand_range(-max_move..=max_move),
            ));
            if 0.2 > rand() {
                new = Rect::new_centered(new.center(), new.size().yx())
            }
            (level.area().subrect(new) & not_blocked(&blocked, new)).then_some(new)
        },
        |area| {
            let distance = center.distance(area.center().as_vec2()) / 50.;
            wateryness(&level, *area) * 20. + unevenness(&level, *area) + distance.powf(2.)
        },
        200,
    );
    if area == start {
        return;
    }

    commands.spawn((
        Pos(level.ground(area.center()).as_vec3()),
        Planned(area),
        House,
    ));
}

pub fn plan_lumberjack(
    mut commands: Commands,
    level: Res<Level>,
    blocked: Query<&Blocked>,
    planned: Query<(With<Lumberjack>, With<Planned>)>,
    center: Query<&Pos, With<CityCenter>>,
    trees: Query<(Entity, &Pos), (With<Tree>, Without<TreeIsNearLumberCamp>)>,
) {
    if !planned.is_empty() {
        return;
    }
    let center = center.single().truncate();

    let start = Rect::new_centered(center.block(), ivec2(rand_range(4..=6), rand_range(5..=8)));
    let area = optimize(
        start,
        |area, temperature| {
            let max_move = (60. * temperature) as i32;
            let mut new = area.offset(ivec2(
                rand_range(-max_move..=max_move),
                rand_range(-max_move..=max_move),
            ));
            if 0.2 > rand() {
                new = Rect::new_centered(new.center(), new.size().yx())
            }
            (level.area().subrect(new) & not_blocked(&blocked, new)).then_some(new)
        },
        |area| {
            let center_distance = center.distance(area.center().as_vec2()) / 50.;
            let tree_access = trees
                .iter()
                .map(|(_, p)| {
                    -1. / ((area.center().as_vec2().distance(p.truncate()) - 10.).max(7.))
                })
                .sum::<f32>();
            wateryness(&level, *area) * 20.
                + unevenness(&level, *area) * 1.
                + center_distance * 1.
                + tree_access * 5.
        },
        200,
    );
    if area == start {
        return;
    }

    for (tree, pos) in &trees {
        if pos.truncate().distance(area.center_vec2()) < 20. {
            commands.entity(tree).insert(TreeIsNearLumberCamp);
        }
    }

    commands.spawn((
        Pos(level.ground(area.center()).as_vec3()),
        Planned(area),
        Lumberjack,
    ));
}

pub fn _plan_quarry(
    mut commands: Commands,
    level: Res<Level>,
    blocked: Query<&Blocked>,
    planned: Query<(With<Quarry>, With<Planned>)>,
    center: Query<&Pos, With<CityCenter>>,
) {
    if !planned.is_empty() {
        return;
    }
    let center = center.single().truncate();

    let start = Rect::new_centered(level.area().center(), IVec2::splat(9));
    let area = optimize(
        start,
        |area, temperature| {
            let max_move = (60. * temperature) as i32;
            let new = area.offset(ivec2(
                rand_range(-max_move..=max_move),
                rand_range(-max_move..=max_move),
            ));
            (level.area().subrect(new) & not_blocked(&blocked, new)).then_some(new)
        },
        |area| {
            let center_distance = center.distance(area.center().as_vec2()) / 50.;
            wateryness(&level, *area) * 20. + unevenness(&level, *area) * -3. + center_distance * 1.
        },
        200,
    );
    if area == start {
        return;
    }

    commands.spawn((
        Pos(level.ground(area.center()).as_vec3()),
        Planned(area),
        Quarry,
    ));
}

// Very temporary, just for testing!
pub fn assign_builds(
    mut commands: Commands,
    construction_sites: Query<With<ConstructionSite>>,
    houses: Query<(With<House>, Without<Planned>)>,
    planned_houses: Query<(Entity, &Planned), With<House>>,
    lumberjacks: Query<(With<Lumberjack>, Without<Planned>)>,
    planned_lumberjacks: Query<(Entity, &Planned), With<Lumberjack>>,
    quarries: Query<(With<Quarry>, Without<Planned>)>,
    planned_quarries: Query<(Entity, &Planned), With<Quarry>>,
) {
    if construction_sites.iter().len() > 10 {
        return;
    }
    let mut plans = Vec::new();
    if houses.iter().len() < 30 {
        plans.extend(&planned_houses)
    }
    if lumberjacks.iter().len() < 20 {
        plans.extend(&planned_lumberjacks)
    }
    if quarries.iter().len() < 8 {
        plans.extend(&planned_quarries)
    }
    if let Some(&(selected, area)) = plans.try_choose() {
        commands
            .entity(selected)
            .remove::<Planned>()
            .insert((Blocked(area.0), ToBeBuild));
    }
}

// TMP
pub fn test_build_house(
    mut replay: ResMut<Replay>,
    mut commands: Commands,
    mut level: ResMut<Level>,
    new: Query<(Entity, &Blocked), (With<ToBeBuild>, With<House>)>,
) {
    for (entity, area) in &new {
        replay.dbg(&format!("building house at {:?}", area.center()));
        commands
            .entity(entity)
            .remove::<ToBeBuild>()
            .insert(ConstructionSite::new(house::house(&mut level, area.0)));
    }
}

// TMP
pub fn test_build_lumberjack(
    mut commands: Commands,
    mut level: ResMut<Level>,
    new: Query<(Entity, &Blocked), (With<ToBeBuild>, With<Lumberjack>)>,
) {
    for (entity, area) in &new {
        commands
            .entity(entity)
            .remove::<ToBeBuild>()
            .insert(ConstructionSite::new(house::shack(&mut level, area.0)));
    }
}

// TMP
pub fn test_build_quarry(
    mut commands: Commands,
    mut level: ResMut<Level>,
    new: Query<(Entity, &Blocked), (Added<ToBeBuild>, With<Quarry>)>,
) {
    for (entity, area) in &new {
        for pos in area.0 {
            let pos = level.ground(pos);
            level[pos] = Wool(Black)
        }
        commands.entity(entity).remove::<ToBeBuild>().insert(Quarry);
    }
}
