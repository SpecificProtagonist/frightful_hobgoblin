#![allow(clippy::type_complexity)]

use crate::optimize::optimize;
use crate::*;
use crate::{remove_foliage::find_trees, replay::*};

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_math::Vec2Swizzles;
use rand::prelude::*;

pub fn sim(mut level: Level) {
    Id::load(&level.path);

    let mut world = World::new();

    let city_center = choose_starting_area(&level);
    let city_center_pos = level.ground(city_center.center());

    world.spawn((
        Pos(city_center_pos.as_vec3()),
        Blocked(city_center),
        CityCenter,
    ));

    for pos in city_center {
        let pos = level.ground(pos);
        level[pos] = Wool(Magenta)
    }

    // Find trees
    for tree in find_trees(&level, level.area()) {
        world.spawn((Pos(tree.as_vec3()), Tree::default()));
    }

    let pos = vec3(-50., 90., 200.);
    world.spawn((Id::default(), Villager::default(), Pos(pos), PrevPos(pos)));

    let mut sched = Schedule::new();
    sched.add_systems(
        (
            (
                place,
                chop,
                walk,
                build,
                plan_house,
                plan_lumberjack,
                plan_quarry,
            ),
            apply_deferred,
            assign_builds,
            apply_deferred,
            (test_build_house, test_build_lumberjack, test_build_quarry),
            apply_deferred,
            (tick_replay, remove_outdated),
            |world: &mut World| world.clear_trackers(),
        )
            .chain(),
    );

    world.init_resource::<Replay>();
    world.insert_resource(level);
    for _ in 0..10000 {
        sched.run(&mut world);
    }

    let level = world.remove_resource::<Level>().unwrap();
    let replay = world.remove_resource::<Replay>().unwrap();
    Id::save(&level.path);
    replay.write(&level.path);
    level.save_metadata().unwrap();
    // level.save();
}

#[derive(Component)]
struct CityCenter;

#[derive(Clone, Copy)]
enum Resource {
    Stone,
    Wood,
}

type PlaceList = Vec<(IVec3, Block)>;

#[derive(Clone, Copy)]
struct BuildStage {
    resource: Resource,
    prefab: &'static str,
}

#[derive(Component)]
struct Building {
    stages: Vec<BuildStage>,
}

#[derive(Component)]
struct MoveTask(Vec2);

#[derive(Component)]
struct PlaceTask(PlaceList);

#[derive(Component)]
struct BuildTask {
    building: Entity,
}

#[derive(Component)]
struct ChopTask {
    tree: Entity,
}

#[derive(Component, Default)]
struct Tree {
    to_be_chopped: bool,
}

fn build(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut buildings: Query<(&Pos, &mut Building)>,
    mut builders: Query<
        (Entity, &Pos, &mut Villager, &BuildTask),
        (Without<ChopTask>, Without<PlaceTask>),
    >,
    mut trees: Query<(Entity, &Pos, &mut Tree)>,
) {
    for (builder, pos, mut villager, build_task) in &mut builders {
        let (building_pos, mut building) = buildings.get_mut(build_task.building).unwrap();
        if let Some(stage) = building.stages.first().cloned() {
            if let Some(carry) = villager.carry {
                if pos.truncate() != building_pos.truncate() {
                    commands
                        .entity(builder)
                        .insert(MoveTask(building_pos.truncate()));
                } else {
                    let wood_type = match carry {
                        Log(species, _) => species,
                        _ => Oak,
                    };
                    let cursor = level.recording_cursor();
                    PREFABS[stage.prefab].build(&mut level, building_pos.block(), YPos, wood_type);
                    villager.carry = None;
                    commands
                        .entity(builder)
                        .insert(PlaceTask(level.pop_recording(cursor).collect()));
                    building.stages.remove(0);
                }
            } else {
                let (tree, _, mut tree_meta) = trees
                    .iter_mut()
                    .filter(|(_, _, meta)| !meta.to_be_chopped)
                    .min_by_key(|(_, pos, _)| pos.distance_squared(building_pos.0) as i32)
                    .expect("no trees");
                tree_meta.to_be_chopped = true;
                commands.entity(builder).insert(ChopTask { tree });
            }
        } else {
            commands.entity(builder).remove::<BuildTask>();
            commands.entity(build_task.building).remove::<Building>();
        }
    }
}

fn chop(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut lumberjacks: Query<(Entity, &mut Villager, &Pos, &ChopTask), Without<MoveTask>>,
    trees: Query<&Pos>,
) {
    for (jack, mut vill, pos, task) in &mut lumberjacks {
        let target = trees.get(task.tree).unwrap();
        const CHOP_DIST: f32 = 1.5;
        let target_pos =
            target.0 - (target.0 - pos.0).truncate().normalize().extend(0.) * CHOP_DIST * 0.99;
        if pos.0.distance(target_pos) <= CHOP_DIST {
            let cursor = level.recording_cursor();
            vill.carry = Some(level[target.block()]);
            remove_foliage::tree(&mut level, target.block());
            commands.entity(task.tree).despawn();
            commands
                .entity(jack)
                .remove::<ChopTask>()
                .insert(PlaceTask(level.pop_recording(cursor).collect()));
        } else {
            commands
                .entity(jack)
                .insert(MoveTask(target_pos.truncate()));
        }
    }
}

fn place(
    mut commands: Commands,
    mut replay: ResMut<Replay>,
    mut builders: Query<(Entity, &mut PlaceTask), Without<MoveTask>>,
) {
    for (entity, mut build) in &mut builders {
        if let Some((pos, block)) = build.0.pop() {
            replay.block(pos, block);
        } else {
            commands.entity(entity).remove::<PlaceTask>();
        }
    }
}

fn walk(
    mut commands: Commands,
    level: Res<Level>,
    mut query: Query<(Entity, &mut Pos, &MoveTask), With<Villager>>,
) {
    for (entity, mut pos, goal) in &mut query {
        const BLOCKS_PER_TICK: f32 = 0.15;
        let diff = goal.0 - pos.0.truncate();
        if diff.length() < BLOCKS_PER_TICK {
            pos.0 = goal.0.extend(pos.z);
            commands.entity(entity).remove::<MoveTask>();
        } else {
            pos.0 += diff.normalize().extend(0.) * BLOCKS_PER_TICK;
            set_walk_height(&level, &mut pos);
        }
    }
}

fn set_walk_height(level: &Level, pos: &mut Vec3) {
    let size = 0.35;
    let mut height = 0f32;
    for off in [vec2(1., 1.), vec2(-1., 1.), vec2(1., -1.), vec2(-1., -1.)] {
        let mut block_pos = (*pos + off.extend(0.) * size).block();
        while !level[block_pos].solid() {
            block_pos.z -= 1
        }
        while level[block_pos].solid() {
            block_pos.z += 1
        }
        height = height.max(
            block_pos.z as f32
                - match level[block_pos - ivec3(0, 0, 1)] {
                    Slab(_, Bottom) => 0.5,
                    // In theory also do stairs here
                    _ => 0.,
                },
        );
    }
    pos.z = height;
}

#[derive(Component, Deref, DerefMut, PartialEq)]
pub struct Pos(pub Vec3);

#[derive(Component, Deref, DerefMut)]
pub struct PrevPos(pub Vec3);

#[derive(Component, Default)]
pub struct Villager {
    pub carry: Option<Block>,
    pub carry_id: Id,
}

#[derive(Component, Deref, DerefMut)]
pub struct Blocked(Rect);

#[derive(Component, Deref, DerefMut)]
pub struct Planned(Rect);

#[derive(Component)]
pub struct House;

#[derive(Component)]
pub struct Lumberjack;

#[derive(Component)]
pub struct Quarry;

#[derive(Component)]
pub struct ToBeBuild;

#[derive(Component)]
pub struct Build;

fn not_blocked<'a>(blocked: impl IntoIterator<Item = &'a Blocked>, area: Rect) -> bool {
    blocked.into_iter().all(|blocker| !blocker.overlapps(area))
}

fn unevenness(level: &Level, area: Rect) -> f32 {
    let avg_height = level.average_height(area);
    area.into_iter()
        .map(|pos| (level.height(pos) as f32 - avg_height).abs().powf(2.))
        .sum::<f32>()
        / area.total() as f32
}

fn wateryness(level: &Level, area: Rect) -> f32 {
    area.into_iter()
        .filter(|pos| level.water_level(*pos).is_some())
        .count() as f32
        / area.total() as f32
}

fn choose_starting_area(level: &Level) -> Rect {
    optimize(
        Rect::new_centered(level.area().center(), IVec2::splat(44)),
        |area, temperature| {
            let mut rng = thread_rng();
            let max_move = (100. * temperature) as i32;
            let new = area.offset(ivec2(
                rng.gen_range(-max_move, max_move + 1),
                rng.gen_range(-max_move, max_move + 1),
            ));
            level.area().subrect(new).then_some(new)
        },
        |area| {
            let distance = area
                .center()
                .as_vec2()
                .distance(level.area().center().as_vec2())
                / (level.area().size().as_vec2().min_element() - 40.);
            wateryness(level, *area) * 10. + unevenness(level, *area) + distance.powf(2.) / 2.
        },
        200,
    )
    .shrink(10)
}

// Note this would require apply_defered after each placement
fn remove_outdated(
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

fn plan_house(
    mut commands: Commands,
    level: Res<Level>,
    blocked: Query<&Blocked>,
    planned: Query<(With<House>, With<Planned>)>,
    center: Query<&Pos, With<CityCenter>>,
) {
    if planned.iter().len() > 0 {
        return;
    }

    let mut rng = thread_rng();
    let center = center.single().truncate();
    let start = Rect::new_centered(
        center.block(),
        ivec2(rng.gen_range(7, 11), rng.gen_range(7, 15)),
    );
    let area = optimize(
        start,
        |area, temperature| {
            let mut rng = thread_rng();
            let max_move = (60. * temperature) as i32;
            let mut new = area.offset(ivec2(
                rng.gen_range(-max_move, max_move + 1),
                rng.gen_range(-max_move, max_move + 1),
            ));
            if rand(0.2) {
                new = Rect::new_centered(new.center(), new.size().yx())
            }
            (level.area().subrect(new) & not_blocked(&blocked, new)).then_some(new)
        },
        |area| {
            let distance = center.distance(area.center().as_vec2()) / 50.;
            wateryness(&level, *area) * 5. + unevenness(&level, *area) + distance.powf(2.)
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

fn plan_lumberjack(
    mut commands: Commands,
    level: Res<Level>,
    blocked: Query<&Blocked>,
    planned: Query<(With<Lumberjack>, With<Planned>)>,
    center: Query<&Pos, With<CityCenter>>,
    trees: Query<&Pos, With<Tree>>,
) {
    if !planned.is_empty() {
        return;
    }
    let center = center.single().truncate();

    let start = Rect::new_centered(level.area().center(), IVec2::splat(9));
    let area = optimize(
        start,
        |area, temperature| {
            let mut rng = thread_rng();
            let max_move = (60. * temperature) as i32;
            let new = area.offset(ivec2(
                rng.gen_range(-max_move, max_move + 1),
                rng.gen_range(-max_move, max_move + 1),
            ));
            (level.area().subrect(new) & not_blocked(&blocked, new)).then_some(new)
        },
        |area| {
            let center_distance = center.distance(area.center().as_vec2()) / 50.;
            let tree_access = trees
                .iter()
                .map(|p| -1. / ((area.center().as_vec2().distance(p.truncate()) - 10.).max(7.)))
                .sum::<f32>();
            wateryness(&level, *area) * 5.
                + unevenness(&level, *area) * 1.
                + center_distance * 1.
                + tree_access * 5.
        },
        200,
    );
    if area == start {
        return;
    }

    commands.spawn((
        Pos(level.ground(area.center()).as_vec3()),
        Planned(area),
        Lumberjack,
    ));
}

fn plan_quarry(
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
            let mut rng = thread_rng();
            let max_move = (60. * temperature) as i32;
            let new = area.offset(ivec2(
                rng.gen_range(-max_move, max_move + 1),
                rng.gen_range(-max_move, max_move + 1),
            ));
            (level.area().subrect(new) & not_blocked(&blocked, new)).then_some(new)
        },
        |area| {
            let center_distance = center.distance(area.center().as_vec2()) / 50.;
            wateryness(&level, *area) * 5. + unevenness(&level, *area) * -3. + center_distance * 1.
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

fn assign_builds(
    mut commands: Commands,
    extant_houses: Query<(With<House>, Without<Planned>)>,
    planned_houses: Query<(Entity, &Planned), With<House>>,
    extant_lumberjacks: Query<(With<Lumberjack>, Without<Planned>)>,
    planned_lumberjacks: Query<(Entity, &Planned), With<Lumberjack>>,
    extant_quarries: Query<(With<Quarry>, Without<Planned>)>,
    planned_quarries: Query<(Entity, &Planned), With<Quarry>>,
) {
    let mut plans = Vec::new();
    if extant_houses.iter().len() < 30 {
        plans.extend(&planned_houses)
    }
    if extant_lumberjacks.iter().len() < 10 {
        plans.extend(&planned_lumberjacks)
    }
    if extant_quarries.iter().len() < 10 {
        plans.extend(&planned_quarries)
    }
    if let Some(&(selected, area)) = plans.choose(&mut thread_rng()) {
        commands
            .entity(selected)
            .remove::<Planned>()
            .insert((Blocked(area.0), ToBeBuild));
    }
}

fn test_build_house(
    mut commands: Commands,
    mut level: ResMut<Level>,
    new: Query<(Entity, &Blocked), (Added<ToBeBuild>, With<House>)>,
) {
    for (entity, area) in &new {
        for pos in area.0 {
            let pos = level.ground(pos);
            level[pos] = Wool(Red)
        }
        for pos in area.border() {
            let pos = level.ground(pos) + IVec3::Z;
            level[pos] = Wool(Red)
        }
        commands.entity(entity).remove::<ToBeBuild>().insert(Build);
    }
}

fn test_build_lumberjack(
    mut commands: Commands,
    mut level: ResMut<Level>,
    new: Query<(Entity, &Blocked), (Added<ToBeBuild>, With<Lumberjack>)>,
) {
    for (entity, area) in &new {
        for pos in area.0 {
            let pos = level.ground(pos);
            level[pos] = Wool(Orange)
        }
        commands.entity(entity).remove::<ToBeBuild>().insert(Build);
    }
}

fn test_build_quarry(
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
