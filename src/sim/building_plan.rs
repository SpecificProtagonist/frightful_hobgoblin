use crate::*;
use bevy_ecs::prelude::*;
use itertools::Itertools;
use sim::*;

use self::{
    desire_lines::{add_desire_line, DesireLines},
    pathfind::pathfind_street,
    quarry::Quarry,
};

#[derive(Component, Deref, DerefMut)]
pub struct Planned(pub Vec<IVec2>);

#[derive(Component)]
pub struct HousePlan {
    area: Rect,
}

#[derive(Component)]
pub struct House {
    pub chimney: Option<Vec3>,
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
            *area += ivec2(rand(-max_move..=max_move), rand(-max_move..=max_move));

            if !level.area().has_subrect(*area) {
                return f32::INFINITY;
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
        1,
    )
    .unwrap()
    .shrink(10)
}

pub fn plan_house(
    mut commands: Commands,
    level: Res<Level>,
    planned: Query<(), (With<HousePlan>, With<Planned>)>,
    center: Query<&Pos, With<CityCenter>>,
) {
    if planned.iter().len() > 0 {
        return;
    }

    // TODO: On a large map, allow for multiple town centers
    let center = center.single().truncate();
    let Some(area) = optimize(
        Rect::new_centered(center.block(), ivec2(rand(7..=11), rand(7..=15))),
        |area, temperature| {
            let max_move = (60. * temperature) as i32;
            *area += ivec2(rand(-max_move..=max_move), rand(-max_move..=max_move));
            if rand(0.2) {
                *area = Rect::new_centered(area.center(), area.size().yx())
            }

            if !level.free(*area) {
                return f32::INFINITY;
            }
            let distance = level.reachability[area.center()] as f32;
            // TODO: try to minimize the amount of trees in the footprint
            wateryness(&level, *area) * 30.
                + unevenness(&level, *area)
                + (distance / 100.).powf(1.6)
        },
        200,
        20,
    ) else {
        return;
    };

    commands.spawn((
        Pos(level.ground(area.center()).as_vec3()),
        Planned(area.into_iter().collect()),
        HousePlan { area },
    ));
}

// Needed to delay until the post itself has been placed in MC
#[derive(Component)]
pub struct SpawnHitchedHorse(IVec3);

pub fn hitching_post(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut dl: ResMut<DesireLines>,
    mut replay: ResMut<Replay>,
    tick: Res<Tick>,
    center: Query<&Pos, With<CityCenter>>,
    new: Query<&SpawnHitchedHorse, Added<SpawnHitchedHorse>>,
) {
    for new in &new {
        let pos = new.0;
        replay.command(format!("summon horse {} {} {} {{Tame:1,SaddleItem:{{Count:1,id:\"saddle\"}},Leash:{{X:{0},Y:{3},Z:{2}}}}}", pos.x, pos.z-1, pos.y, pos.z));
    }
    if (tick.0 != 20000) & (tick.0 != 30000) {
        return;
    }
    let center = center.single().truncate().block();
    let Some(area) = optimize(
        Rect::new_centered(level.area().center(), ivec2(5, 5)),
        |area, temperature| {
            let max_move = (60. * temperature) as i32;
            *area += ivec2(rand(-max_move..=max_move), rand(-max_move..=max_move));
            if !level.free(*area) || area.grow(4).into_iter().all(|b| level.blocked[b] != Street) {
                return f32::INFINITY;
            }
            let path = pathfind_street(&level, *area);
            if !path.success {
                return f32::INFINITY;
            }
            let distance = center.as_vec2().distance(area.center_vec2());
            // TODO: try to minimize the amount of trees in the footprint
            wateryness(&level, *area) * 30. + unevenness(&level, *area) + path.cost as f32
                - distance.min(100.) / 100.
        },
        200,
        10,
    ) else {
        return;
    };

    for column in area {
        level.blocked[column] = Street;
    }
    for node in pathfind_street(&level, area.shrink(1)).path {
        for (x_off, y_off) in (-1..=1).cartesian_product(-1..=1) {
            level.blocked[node.pos.truncate() + ivec2(x_off, y_off)] = Street;
        }
        for _ in 0..30 {
            add_desire_line(&mut level, &mut dl, node.pos - IVec3::Z);
        }
    }

    let pos = level.ground(area.center());
    let species = level.biome[pos].random_tree_species();
    level(pos, Full(Cobble));
    level(pos + IVec3::Z, Fence(Wood(species)));
    level(pos + 2 * IVec3::Z, Fence(Wood(species)));

    let hay = level.ground(area.center() + ivec2(2, 1)) + IVec3::Z;
    level(hay, Hay);

    commands.spawn(SpawnHitchedHorse(pos + 2 * IVec3::Z));
}

// Very temporary, just for testing!
pub fn assign_builds(
    mut commands: Commands,
    mut level: ResMut<Level>,
    construction_sites: Query<(), With<ConstructionSite>>,
    houses: Query<(), (With<HousePlan>, Without<Planned>)>,
    planned_houses: Query<(Entity, &Planned), With<HousePlan>>,
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
    if lumberjacks.iter().len() < 8 {
        plans.extend(&planned_lumberjacks)
    }
    if quarries.iter().len() < 3 {
        plans.extend(&planned_quarries);
    }
    if let Some(&(selected, area)) = plans.try_choose() {
        if !level.free(area.iter().copied()) {
            commands.entity(selected).despawn();
        } else {
            (level.blocked)(area.iter().copied(), Blocked);
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
    mut dl: ResMut<DesireLines>,
    mut untree: Untree,
    new: Query<(Entity, &HousePlan), With<ToBeBuild>>,
) {
    for (entity, house) in &new {
        let (rec, house) =
            house::house(&mut commands, &mut level, &mut dl, &mut untree, house.area);
        let site = ConstructionSite::new(rec);
        commands
            .entity(entity)
            .remove::<ToBeBuild>()
            .insert((site, house));
    }
}
