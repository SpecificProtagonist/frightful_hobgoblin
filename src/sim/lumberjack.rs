use crate::*;
use itertools::Itertools;
use sim::*;

use self::{
    storage_pile::LumberPile,
    trees::{Tree, TreeState, Untree},
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

// TODO: Cap lumber piles accidentally get chopped down when next to tree?

pub fn work(
    mut commands: Commands,
    pos: Query<&Pos>,
    mut workers: Query<
        (Entity, &Villager, &mut Lumberworker),
        (Without<ChopTask>, Without<DeliverTask>, Without<MoveTask>),
    >,
    mut trees: Query<(Entity, &Pos, &mut Tree)>,
    piles: Query<(Entity, &Pos, &Pile, &LumberPile)>,
) {
    for (entity, villager, mut lumberworker) in &mut workers {
        let worker_pos = pos.get(entity).unwrap();
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
                .entity(entity)
                .insert(ChopTask::new(tree_pos.block()));
            tree_meta.state = TreeState::MarkedForChoppage;
            lumberworker.ready_to_work = false;
        } else if let Some(stack) = villager.carry {
            // Drop off lumber
            // TODO: This allows overly full piles due to multiple simultaneous deliveries. Fix this by introducing storage piles?
            if let Some((to, _, _, _)) = piles
                .iter()
                .filter(|(_, _, current, lumber_pile)| {
                    current.get(&Good::Wood).copied().unwrap_or_default() + stack.amount
                        <= lumber_pile.max()
                })
                .min_by_key(|(_, pos, _, _)| pos.distance(worker_pos.0) as i32)
            {
                commands.entity(entity).insert(DeliverTask { to });
            }
        } else {
            // Return home
            commands.entity(entity).insert(MoveTask::new(
                pos.get(lumberworker.workplace).unwrap().block(),
            ));
            lumberworker.ready_to_work = true;
        }
    }
}

pub fn chop(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut lumberjacks: Query<
        (Entity, &mut Villager, &mut ChopTask),
        (Without<MoveTask>, Without<PlaceTask>),
    >,
    mut untree: Untree,
) {
    for (jack, mut vill, mut task) in &mut lumberjacks {
        if !task.chopped {
            let mut place = PlaceTask(default());
            place.push_back(ConsItem::Goto(MoveTask {
                goal: task.tree,
                distance: 2,
            }));

            let cursor = level.recording_cursor();
            untree.remove_trees(&mut level, Some(task.tree.truncate()));
            let rec = level.pop_recording(cursor).collect_vec();
            let mut amount = 0.;
            for set in &rec {
                amount += match set.previous {
                    Log(..) => 4.,
                    Fence(..) => 1.,
                    Leaves(..) => 0.25,
                    _ => 0.,
                }
            }
            place.extend(rec.into_iter().map(ConsItem::Set));
            vill.carry = Some(Stack::new(Good::Wood, amount));

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
    center: Query<&Pos, With<CityCenter>>,
    new_lumberjacks: Query<&Pos, Added<LumberjackFocus>>,
) {
    for lumberjack in &new_lumberjacks {
        let (pos, params) = LumberPile::make(
            &mut level,
            lumberjack.0.truncate(),
            center.single().truncate(),
        );
        commands.spawn((
            Pos(pos.as_vec3()),
            params,
            OutPile {
                available: default(),
            },
            Pile {
                goods: default(),
                interact_distance: params.width,
            },
        ));

        // TODO: Clear trees here
    }
}
