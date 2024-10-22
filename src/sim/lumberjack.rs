use crate::*;
use itertools::Itertools;
use sim::*;

use self::{
    storage_pile::LumberPile,
    trees::{Tree, TreeState},
};

#[derive(Component)]
pub struct LumberjackFocus;

#[derive(Component)]
pub struct LumberjackShack {
    pub area: Rect,
}

#[derive(Component)]
pub struct TreeIsNearLumberCamp;

#[derive(Component)]
pub struct Lumberworker {
    workplace: Entity,
    ready_to_work: bool,
}

// This is a separate component to allow giving this task to other villagers too
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ChopTask {
    tree: IVec3,
    chopped: bool,
}

impl ChopTask {
    pub fn new(tree: IVec3) -> Self {
        Self {
            tree,
            chopped: false,
        }
    }
}

pub fn plan_lumberjack(
    mut commands: Commands,
    level: Res<Level>,
    planned: Query<(), (With<LumberjackShack>, With<Planned>)>,
    trees: Query<(Entity, &Pos, &Tree), Without<TreeIsNearLumberCamp>>,
    center: Query<&Pos, With<CityCenter>>,
) {
    if !planned.is_empty() {
        return;
    }

    // TODO: Seperate focus and shack position selection
    let Some(area) = optimize(
        Rect::new_centered(
            center.single().0.block().truncate(),
            ivec2(rand(4..=6), rand(5..=8)),
        ),
        |area, temperature| {
            let max_move = (60. * temperature) as i32;
            *area += ivec2(rand(-max_move..=max_move), rand(-max_move..=max_move));
            if rand(0.2) {
                *area = Rect::new_centered(area.center(), area.size().yx())
            }

            if !level.free(area.grow(1)) {
                return f32::INFINITY;
            }
            let center_distance = level.reachability[area.center()] as f32;
            let tree_access = trees
                .iter()
                .filter_map(|(_, p, tree)| {
                    (tree.state == TreeState::Ready).then_some(
                        -1. / ((area.center().as_vec2().distance(p.truncate()) - 10.).max(7.)),
                    )
                })
                .sum::<f32>();
            wateryness(&level, *area) * 20.
                + unevenness(&level, *area) * 1.
                + center_distance / 200.
                + tree_access * 5.
        },
        200,
        1,
    ) else {
        return;
    };

    for (tree, pos, _) in &trees {
        if pos.truncate().distance(area.center_vec2()) < 20. {
            commands.entity(tree).insert(TreeIsNearLumberCamp);
        }
    }

    commands.spawn((Pos(level.ground(area.center()).as_vec3()), LumberjackFocus));
    commands.spawn((
        Pos(level.ground(area.center()).as_vec3()),
        Planned(area.grow(1).into_iter().collect()),
        LumberjackShack { area },
    ));
}

// TMP
pub fn test_build_lumberjack(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut untree: Untree,
    new: Query<(Entity, &LumberjackShack), With<ToBeBuild>>,
) {
    for (entity, lumberjack) in &new {
        let site = ConstructionSite::new(house::shack(
            &mut commands,
            &mut level,
            &mut untree,
            lumberjack.area,
        ));
        commands.entity(entity).remove::<ToBeBuild>().insert(site);
    }
}

pub fn assign_worker(
    mut commands: Commands,
    available: Query<(Entity, &Pos), With<Jobless>>,
    new: Query<(Entity, &Pos), Added<LumberjackFocus>>,
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
        commands
            .entity(worker)
            .remove::<Jobless>()
            .insert(Lumberworker {
                workplace,
                ready_to_work: true,
            });
    }
}

pub fn work(
    mut commands: Commands,
    pos: Query<&Pos>,
    mut workers: Query<
        (Entity, &Villager, &mut Lumberworker),
        (Without<ChopTask>, Without<DeliverTask>, Without<MoveTask>),
    >,
    mut trees: Query<(Entity, &Pos, &mut Tree)>,
    piles: Query<(Entity, &Pos, &Pile, &LumberPile), With<StoragePile>>,
) {
    for (worker, villager, mut lumberworker) in &mut workers {
        let worker_pos = pos.get(worker).unwrap();
        if lumberworker.ready_to_work {
            // Go chopping
            let Some((_, tree_pos, mut tree_meta)) = trees
                .iter_mut()
                .filter(|(_, _, tree)| tree.state == TreeState::Ready)
                // TODO: prefer larger trees
                .min_by_key(|(_, p, _)| p.distance_squared(worker_pos.0) as i32)
            else {
                return;
            };
            commands
                .entity(worker)
                .insert(ChopTask::new(tree_pos.block()));
            tree_meta.state = TreeState::MarkedForChoppage;
            lumberworker.ready_to_work = false;
        } else if let Some(stack) = villager.carry {
            // Drop off lumber
            if let Some((to, _, _, _)) = piles
                .iter()
                .filter(|(_, pile_pos, current, lumber_pile)| {
                    current.space_available(
                        Good::Wood,
                        lumber_pile.max(),
                        min_walk_ticks(worker_pos.0, pile_pos.0),
                    ) >= stack.amount
                })
                .min_by_key(|(_, pos, _, _)| pos.distance(worker_pos.0) as i32)
            {
                commands.entity(worker).insert(DeliverTask { to });
            }
        } else {
            // Return home
            commands.entity(worker).insert(MoveTask::new(
                pos.get(lumberworker.workplace).unwrap().block(),
            ));
            lumberworker.ready_to_work = true;
        }
    }
}

// Seperate system to allow other villagers to chop. Should this be merge into `work`?
pub fn chop(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut lumberjacks: Query<(Entity, &mut ChopTask), (Without<MoveTask>, Without<PlaceTask>)>,
    mut untree: Untree,
) {
    for (jack, mut task) in &mut lumberjacks {
        if !task.chopped {
            let mut place = PlaceTask(default());
            place.push_back(ConsItem::Goto(MoveTask {
                goal: task.tree,
                distance: 2,
            }));
            place.push_back(ConsItem::Command(playsound("chop", task.tree)));

            let cursor = level.recording_cursor();
            untree.remove_trees(&mut level, Some(task.tree.truncate()));
            let rec = level.pop_recording(cursor).collect_vec();
            let mut amount = 0.;
            for set in &rec {
                amount += match set.previous {
                    Log(..) => 4.,
                    Fence(Andesite) => 2.,
                    Fence(..) => 1.,
                    Leaves(..) => 0.25,
                    _ => 0.,
                }
            }
            place.extend(rec.into_iter().map(ConsItem::Set));
            place.push_back(ConsItem::Carry(Some(Stack::new(Good::Wood, amount))));

            commands.entity(jack).insert(place);
            task.chopped = true;
        } else {
            commands.entity(jack).remove::<ChopTask>();
        }
    }
}

pub fn make_lumber_piles(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut untree: Untree,
    center: Query<&Pos, With<CityCenter>>,
    new_lumberjacks: Query<&Pos, Added<LumberjackFocus>>,
) {
    for lumberjack in &new_lumberjacks {
        let (pos, _, params) = LumberPile::make(
            &mut level,
            &mut untree,
            lumberjack.0.truncate(),
            center.single().truncate(),
        );
        commands.spawn((
            Pos(pos.as_vec3()),
            params,
            OutPile::default(),
            Pile::new(default(), params.width),
            StoragePile,
        ));
    }
}
