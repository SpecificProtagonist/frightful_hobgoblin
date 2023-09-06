#![allow(clippy::type_complexity)]

use std::collections::VecDeque;

use crate::material::Mat;
use crate::optimize::optimize;
use crate::remove_foliage::remove_tree;
use crate::*;
use crate::{pathfind::pathfind, remove_foliage::find_trees, replay::*};

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_math::Vec2Swizzles;
use rand::prelude::*;

pub fn sim(mut level: Level) {
    Id::load(&level.path);

    let mut world = World::new();

    let city_center = choose_starting_area(&level);
    let city_center_pos = level.ground(city_center.center());
    println!("Center: {:?}", city_center_pos.truncate());

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

    world.spawn((
        Id::default(),
        Villager::default(),
        Pos(city_center_pos.as_vec3() + Vec3::Z),
        PrevPos(default()),
    ));

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

pub type PlaceList = VecDeque<(IVec3, Block)>;

#[derive(Clone, Copy)]
struct BuildStage {
    resource: Mat,
    prefab: &'static str,
}

#[derive(Component)]
struct Building {
    stages: Vec<BuildStage>,
}

#[derive(Component)]
struct MoveTask {
    goal: IVec3,
    distance: i32,
}

impl MoveTask {
    fn new(goal: IVec3) -> MoveTask {
        Self { goal, distance: 0 }
    }
}

/// Path to move along, in reverse order
#[derive(Component, Deref, DerefMut)]
struct MovePath(Vec<IVec3>);

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

#[derive(Component)]
struct ReadyToChop;

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
                        .insert(MoveTask::new(building_pos.block() + IVec3::Z));
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
    mut lumberjacks: Query<
        (Entity, &mut Villager, &ChopTask, Option<&ReadyToChop>),
        Without<MoveTask>,
    >,
    trees: Query<&Pos>,
) {
    for (jack, mut vill, task, ready) in &mut lumberjacks {
        let target = trees.get(task.tree).unwrap();
        if ready.is_some() {
            let cursor = level.recording_cursor();
            vill.carry = Some(level[target.block()]);
            remove_tree(&mut level, target.block());
            commands.entity(task.tree).despawn();
            commands
                .entity(jack)
                .remove::<(ChopTask, ReadyToChop)>()
                .insert(PlaceTask(level.pop_recording(cursor).collect()));
        } else {
            commands.entity(jack).insert((
                ReadyToChop,
                MoveTask {
                    goal: target.block(),
                    distance: 2,
                },
            ));
        }
    }
}

fn place(
    mut commands: Commands,
    mut replay: ResMut<Replay>,
    mut builders: Query<(Entity, &mut PlaceTask), Without<MoveTask>>,
) {
    for (entity, mut build) in &mut builders {
        if let Some((pos, block)) = build.0.pop_front() {
            replay.block(pos, block);
        } else {
            commands.entity(entity).remove::<PlaceTask>();
        }
    }
}

// TODO: Smooth this out
fn walk(
    mut commands: Commands,
    level: Res<Level>,
    mut query: Query<(Entity, &mut Pos, &MoveTask, Option<&mut MovePath>), With<Villager>>,
) {
    for (entity, mut pos, goal, path) in &mut query {
        if let Some(mut path) = path {
            const BLOCKS_PER_TICK: f32 = 0.13;
            let mut next_node = *path.last().unwrap();
            let diff = (next_node.as_vec3() - pos.0).truncate();
            if diff.length() < BLOCKS_PER_TICK {
                path.pop();
                if let Some(&next) = path.last() {
                    next_node = next;
                } else {
                    commands.entity(entity).remove::<(MoveTask, MovePath)>();
                }
            }
            let diff = (next_node.as_vec3() - pos.0).truncate();
            pos.0 += (diff.normalize() * BLOCKS_PER_TICK).extend(0.);
            set_walk_height(&level, &mut pos);
        } else {
            let path = pathfind(&level, goal.goal, pos.block());
            commands.entity(entity).insert(MovePath(path));
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
            wateryness(level, *area) * 20. + unevenness(level, *area) + distance.powf(2.) / 2.
        },
        300,
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

    let mut rng = thread_rng();
    let start = Rect::new_centered(
        center.block(),
        ivec2(rng.gen_range(5, 7), rng.gen_range(5, 11)),
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
            let center_distance = center.distance(area.center().as_vec2()) / 50.;
            let tree_access = trees
                .iter()
                .map(|p| -1. / ((area.center().as_vec2().distance(p.truncate()) - 10.).max(7.)))
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
    mut replay: ResMut<Replay>,
    mut commands: Commands,
    mut level: ResMut<Level>,
    new: Query<(Entity, &Blocked), (With<ToBeBuild>, With<House>)>,
    builders: Query<Entity, (With<Villager>, Without<PlaceTask>)>,
) {
    let mut assigned = Vec::new();
    for builder in &builders {
        for (entity, area) in &new {
            if assigned.contains(&entity) {
                continue;
            }
            replay.say(&format!("building house at {:?}", area.center()));
            assigned.push(entity);
            commands
                .entity(builder)
                .insert(PlaceTask(house::house(&mut level, area.0)));
            commands.entity(entity).remove::<ToBeBuild>().insert(Build);
            break;
        }
    }
}

fn test_build_lumberjack(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut replay: ResMut<Replay>,
    new: Query<(Entity, &Blocked), (Added<ToBeBuild>, With<Lumberjack>)>,
) {
    for (entity, area) in &new {
        for (pos, block) in house::lumberjack(&mut level, area.0) {
            replay.block(pos, block)
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
