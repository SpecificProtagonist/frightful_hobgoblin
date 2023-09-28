#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use std::collections::VecDeque;

use crate::goods::*;
use crate::make_trees::{grow_trees, GrowTree};
use crate::optimize::optimize;
use crate::remove_foliage::remove_tree;
use crate::*;
use crate::{pathfind::pathfind, remove_foliage::find_trees, replay::*};

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_ecs::query::Has;
use bevy_math::Vec2Swizzles;

pub fn sim(mut level: Level) {
    Id::load(&level.path);

    let mut world = World::new();
    world.init_resource::<Tick>();

    let city_center = choose_starting_area(&level);
    let city_center_pos = level.ground(city_center.center());
    println!("Center: {:?}", city_center_pos.truncate());

    world.spawn((
        Pos(city_center_pos.as_vec3()),
        Blocked(city_center),
        CityCenter,
        OutPile {
            available: {
                let mut stock = Stockpile::default();
                stock.add(Stack::new(Good::Stone, 99999999.));
                stock.add(Stack::new(Good::Wood, 99999999.));
                stock.add(Stack::new(Good::Soil, 99999999.));
                stock
            },
        },
    ));

    for pos in city_center {
        let pos = level.ground(pos);
        level[pos] = Wool(Magenta)
    }

    // Find trees
    for (pos, species) in find_trees(&level, level.area()) {
        world.spawn((Pos(pos.as_vec3()), Tree::new(species)));
    }

    // for x in 0..20 {
    //     for y in 0..20 {
    //         // let mut tree = [GrowTree::pine, GrowTree::oak, GrowTree::birch].choose()();
    //         // tree.size = rand_f32(0.3, 4.);
    //         // let pos = level.ground(ivec2(x * 7, y * 7)).as_vec3() + Vec3::Z;
    //         // tree.build(&mut level, pos);
    //         let col = ivec2(x * 7 + rand_range(0..5), y * 7 + rand_range(0..5));
    //         if level.water_level(col).is_none() {
    //             world.spawn((Pos(level.ground(col).as_vec3() + Vec3::Z), GrowTree::pine()));
    //         }
    //     }
    // }

    let mut sched = Schedule::new();
    sched.add_systems(
        (
            grow_trees,
            (
                place,
                chop,
                walk,
                build,
                carry,
                check_construction_site_readiness,
            ),
            plan_house,
            // plan_lumberjack,
            // plan_quarry,
            apply_deferred,
            assign_builds,
            assign_work,
            apply_deferred,
            new_construction_site,
            test_build_house,
            test_build_lumberjack,
            test_build_quarry,
            apply_deferred,
            tick_replay,
            remove_outdated,
            |mut tick: ResMut<Tick>| tick.0 += 1,
            |world: &mut World| world.clear_trackers(),
        )
            .chain(),
    );

    let mut replay = Replay::new(level.path.clone());
    replay.command(format!(
        "tp @p {} {} {}",
        city_center_pos.x,
        city_center_pos.z + 30,
        city_center_pos.y
    ));
    world.insert_resource(replay);
    world.insert_resource(level);
    for tick in 0..20000 {
        sched.run(&mut world);

        if tick < 20 {
            world.spawn((
                Id::default(),
                Villager::default(),
                Pos(city_center_pos.as_vec3() + Vec3::Z),
                PrevPos(default()),
            ));
        }
    }

    let level = world.remove_resource::<Level>().unwrap();
    let replay = world.remove_resource::<Replay>().unwrap();
    Id::save(&level.path);
    rayon::spawn(move || level.save_metadata().unwrap());
    replay.finish();
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct Tick(pub i32);

#[derive(Component)]
struct CityCenter;

pub type PlaceList = VecDeque<SetBlock>;

#[derive(Component)]
struct ConstructionSite {
    todo: PlaceList,
    has_builder: bool,
    /// Whether it has the materials necessary for the next block
    has_materials: bool,
}

impl ConstructionSite {
    fn new(blocks: PlaceList) -> Self {
        Self {
            todo: blocks,
            has_builder: false,
            has_materials: false,
        }
    }
}

#[derive(Component, Debug)]
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
#[derive(Component)]
struct MovePath(VecDeque<IVec3>);

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

// Assumes reservations have already been made
#[derive(Component)]
struct CarryTask {
    from: Entity,
    to: Entity,
    stack: Stack,
    max_stack: f32,
}

#[derive(Component)]
struct Tree {
    _species: TreeSpecies,
    _to_be_chopped: bool,
}

impl Tree {
    fn new(species: TreeSpecies) -> Self {
        Self {
            _species: species,
            _to_be_chopped: false,
        }
    }
}

#[derive(Component, Default, Debug)]
struct InPile {
    stock: Stockpile,
    requested: Stockpile,
    // Gets reset after delivery of priority good
    priority: Option<Good>,
}

#[derive(Component, Default, Debug)]
struct OutPile {
    // TODO: When adding piles that visualize what is available,
    // this also needs a `current` field
    available: Stockpile,
}

fn new_construction_site(
    mut commands: Commands,
    new: Query<(Entity, &ConstructionSite), Added<ConstructionSite>>,
) {
    for (entity, site) in &new {
        let mut stock = Stockpile::default();
        let mut requested = Stockpile::default();
        let mut priority = None;
        for set_block in &site.todo {
            if let Some(stack) = goods_for_block(set_block.block) {
                requested.add(stack);
                if priority.is_none() {
                    priority = Some(stack.kind);
                }
            }
            if let Some(mined) = goods_for_block(set_block.previous) {
                stock.add(mined);
                requested.remove(mined);
            }
        }
        commands.entity(entity).insert(InPile {
            stock,
            requested,
            priority,
        });
    }
}

fn assign_work(
    mut commands: Commands,
    idle: Query<(Entity, &Pos), (With<Villager>, Without<CarryTask>, Without<BuildTask>)>,
    mut out_piles: Query<(Entity, &Pos, &mut OutPile)>,
    mut in_piles: Query<(Entity, &Pos, &mut InPile)>,
    mut construction_sites: Query<(Entity, &Pos, &mut ConstructionSite)>,
) {
    for (vill, vil_pos) in &idle {
        if let Some((building, pos, mut site)) = construction_sites
            .iter_mut()
            .filter(|(_, _, site)| site.has_materials & !site.has_builder)
            .min_by_key(|(_, pos, _)| pos.distance_squared(vil_pos.0) as u32)
        {
            site.has_builder = true;
            commands
                .entity(vill)
                .insert((MoveTask::new(pos.0.block()), BuildTask { building }));
            continue;
        }

        if let Some((_, task)) = out_piles
            .iter_mut()
            .filter_map(|(out_entity, out_pos, out_pile)| {
                let mut best_score = f32::INFINITY;
                let mut task = None;
                for (good, &amount) in &out_pile.available.0 {
                    if amount == 0. {
                        continue;
                    }
                    for (in_entity, in_pos, in_pile) in &mut in_piles {
                        if let Some(&requested) = in_pile.requested.0.get(good) && requested > 0. {
                            if let Some(priority) = in_pile.priority {
                                if priority != *good {continue}
                            }
                            let mut score = out_pos.distance_squared(in_pos.0);
                            // Try to reduce the amount of trips
                            if amount < requested {
                                score *= 2.;
                            }
                            if score < best_score {
                                best_score = score;
                                task = Some(CarryTask {
                                    from: out_entity,
                                    to: in_entity,
                                    stack: Stack::new(
                                        *good,
                                        amount.min(requested).min(CARRY_CAPACITY),
                                    ),
                                    max_stack: requested.min(CARRY_CAPACITY),
                                });
                            }
                        }
                    }
                }
                task.map(|task| (vil_pos.distance_squared(out_pos.0), task))
            })
            // TODO: Also influence via best_score?
            .min_by_key(|(d, _)| *d as i32)
        {
            out_piles
                .get_component_mut::<OutPile>(task.from)
                .unwrap()
                .available
                .remove(task.stack);
            in_piles
                .get_component_mut::<InPile>(task.to)
                .unwrap()
                .requested
                .remove(task.stack);
            commands.entity(vill).insert(task);
        }
    }
}

#[derive(Component)]
#[component(storage = "SparseSet")]
struct PickupReady;

#[derive(Component)]
#[component(storage = "SparseSet")]
struct DeliverReady;

fn carry(
    mut commands: Commands,
    pos: Query<&Pos>,
    mut out_piles: Query<&mut OutPile>,
    mut in_piles: Query<&mut InPile>,
    mut workers: Query<
        (
            Entity,
            &mut Villager,
            &mut CarryTask,
            Has<PickupReady>,
            Has<DeliverReady>,
        ),
        Without<MoveTask>,
    >,
) {
    for (entity, mut villager, mut task, pickup_ready, deliver_ready) in &mut workers {
        if !pickup_ready {
            commands.entity(entity).insert((
                MoveTask::new(pos.get(task.from).unwrap().block()),
                PickupReady,
            ));
        } else if villager.carry.is_none() {
            let mut out = out_piles.get_mut(task.from).unwrap();
            // If more goods have been deposited since the task was set, take them too
            let missing = task.max_stack - task.stack.amount;
            let extra = out.available.remove_up_to(Stack {
                kind: task.stack.kind,
                amount: missing,
            });
            task.stack.amount += extra.amount;
            villager.carry = Some(task.stack);
        } else if !deliver_ready {
            commands.entity(entity).insert((
                MoveTask::new(pos.get(task.to).unwrap().block()),
                DeliverReady,
            ));
        } else {
            // TODO: Got a querrydoesnotmatch error?!?
            let mut pile = in_piles.get_mut(task.to).unwrap();
            pile.stock.add(task.stack);
            if pile.priority == Some(task.stack.kind) {
                pile.priority = None
            }
            villager.carry = None;
            commands
                .entity(entity)
                .remove::<(CarryTask, PickupReady, DeliverReady)>();
        }
    }
}

fn build(
    mut commands: Commands,
    mut replay: ResMut<Replay>,
    mut builders: Query<
        (Entity, &BuildTask),
        (With<Villager>, Without<ChopTask>, Without<MoveTask>),
    >,
    mut buildings: Query<(Entity, &mut ConstructionSite, &mut InPile)>,
) {
    for (builder, build_task) in &mut builders {
        let Ok((entity, mut building, mut pile)) = buildings.get_mut(build_task.building) else {
            continue;
        };
        if let Some(set) = building.todo.get(0).copied() {
            if let Some(block) = pile.stock.build(set.block) {
                replay.block(set.pos, block);
                replay.dust(set.pos);
                building.todo.pop_front();
            } else {
                building.has_builder = false;
                building.has_materials = false;
                commands.entity(builder).remove::<BuildTask>();
            }
        } else {
            replay.dbg("Building finished");
            commands.entity(builder).remove::<BuildTask>();
            commands
                .entity(entity)
                .remove::<(InPile, ConstructionSite)>()
                .insert(OutPile {
                    available: std::mem::take(&mut pile.stock),
                });
        }
    }
}

fn check_construction_site_readiness(
    mut query: Query<(&mut ConstructionSite, &mut InPile), Changed<InPile>>,
) {
    for (mut site, mut pile) in &mut query {
        if !site.has_materials {
            if let Some(needed) = goods_for_block(site.todo[0].block) {
                if pile.stock.has(needed) {
                    site.has_materials = true
                } else if pile.priority.is_none() {
                    pile.priority = Some(needed.kind);
                }
            } else {
                site.has_materials = true
            }
        }
    }
}

#[derive(Component)]
#[component(storage = "SparseSet")]
struct ChopReady;

fn chop(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut lumberjacks: Query<(Entity, &mut Villager, &ChopTask, Has<ChopReady>), Without<MoveTask>>,
    trees: Query<(&Pos, &Tree)>,
) {
    for (jack, mut vill, task, ready) in &mut lumberjacks {
        let (target, _tree) = trees.get(task.tree).unwrap();
        if ready {
            let cursor = level.recording_cursor();
            vill.carry = Some(Stack::new(Good::Wood, 1.));
            remove_tree(&mut level, target.block());
            commands.entity(task.tree).despawn();
            commands
                .entity(jack)
                .remove::<(ChopTask, ChopReady)>()
                .insert(PlaceTask(level.pop_recording(cursor).collect()));
        } else {
            commands.entity(jack).insert((
                ChopReady,
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
        if let Some(set) = build.0.pop_front() {
            replay.block(set.pos, set.block);
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
            const BLOCKS_PER_TICK: f32 = 0.16;
            let mut next_node = *path.0.front().unwrap();
            let diff = next_node.as_vec3() - pos.0; //.truncate();
            if diff.length() < BLOCKS_PER_TICK {
                path.0.pop_front();
                if let Some(&next) = path.0.front() {
                    next_node = next;
                } else {
                    commands.entity(entity).remove::<(MoveTask, MovePath)>();
                }
            }
            let diff = next_node.as_vec3() - pos.0; //.truncate();
            pos.0 += diff.normalize_or_zero() * BLOCKS_PER_TICK;
            //.extend(0.);
            // set_walk_height(&level, &mut pos);
        } else {
            let path = pathfind(&level, pos.block(), goal.goal, goal.distance);
            commands.entity(entity).insert(MovePath(path));
        }
    }
}

// fn set_walk_height(level: &Level, pos: &mut Vec3) {
//     let size = 0.35;
//     let mut height = 0f32;
//     for off in [vec2(1., 1.), vec2(-1., 1.), vec2(1., -1.), vec2(-1., -1.)] {
//         let mut block_pos = (*pos + off.extend(0.) * size).block();
//         if !level[block_pos].solid() {
//             block_pos.z -= 1
//         }
//         if level[block_pos].solid() {
//             block_pos.z += 1
//         }
//         height = height.max(
//             block_pos.z as f32
//                 - match level[block_pos - ivec3(0, 0, 1)] {
//                     Slab(_, Bottom) => 0.5,
//                     // In theory also do stairs here
//                     _ => 0.,
//                 },
//         );
//     }
//     pos.z = height;
// }

#[derive(Component, Deref, DerefMut, PartialEq)]
pub struct Pos(pub Vec3);

impl std::fmt::Display for Pos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.0.x, self.0.y, self.0.z)
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct PrevPos(pub Vec3);

#[derive(Component, Default)]
pub struct Villager {
    pub carry: Option<Stack>,
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

    let start = Rect::new_centered(center.block(), ivec2(rand_range(5..=7), rand_range(5..=11)));
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
fn assign_builds(
    mut commands: Commands,
    extant_houses: Query<(With<House>, Without<Planned>)>,
    wip_houses: Query<(With<House>, With<ConstructionSite>)>,
    planned_houses: Query<(Entity, &Planned), With<House>>,
    extant_lumberjacks: Query<(With<Lumberjack>, Without<Planned>)>,
    planned_lumberjacks: Query<(Entity, &Planned), With<Lumberjack>>,
    extant_quarries: Query<(With<Quarry>, Without<Planned>)>,
    planned_quarries: Query<(Entity, &Planned), With<Quarry>>,
) {
    let mut plans = Vec::new();
    if (extant_houses.iter().len() < 50) & (wip_houses.iter().len() <= 12) {
        // println!("{} {}", extant_houses.iter().len(), wip_houses.iter().len());
        plans.extend(&planned_houses)
    }
    if extant_lumberjacks.iter().len() < 10 {
        plans.extend(&planned_lumberjacks)
    }
    if extant_quarries.iter().len() < 10 {
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
fn test_build_house(
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
fn test_build_lumberjack(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut replay: ResMut<Replay>,
    new: Query<(Entity, &Blocked), (Added<ToBeBuild>, With<Lumberjack>)>,
) {
    for (entity, area) in &new {
        for set in house::lumberjack(&mut level, area.0) {
            replay.block(set.pos, set.block)
        }
        commands.entity(entity).remove::<ToBeBuild>().insert(Build);
    }
}

// TMP
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
