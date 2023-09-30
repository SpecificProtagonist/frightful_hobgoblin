use crate::*;
use bevy_ecs::prelude::*;
use sim::*;

#[derive(Component)]
pub struct Lumberworker {
    workplace: Entity,
}

// This is a separate component to allow giving this task to other villagers too
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ChopTask {
    tree: Entity,
    stage: ChopStage,
}

impl ChopTask {
    pub fn new(tree: Entity) -> Self {
        Self {
            tree,
            stage: ChopStage::Goto,
        }
    }
}

enum ChopStage {
    Goto,
    Chop,
    Finish,
}

pub fn assign_worker(
    mut commands: Commands,
    mut replay: ResMut<Replay>,
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
        replay.dbg("assign lumberjack");
        commands
            .entity(worker)
            .remove::<Jobless>()
            .insert(Lumberworker { workplace });
    }
}

pub fn work(
    mut commands: Commands,
    pos: Query<&Pos>,
    workers: Query<(Entity, &Villager, &Lumberworker), (Without<ChopTask>, Without<DeliverTask>)>,
    mut trees: Query<(Entity, &Pos, &mut Tree)>,
) {
    for (entity, villager, lumberworker) in &workers {
        let pos = pos.get(entity).unwrap();
        if villager.carry.is_none() {
            let Some((tree, _, mut tree_meta)) = trees
                .iter_mut()
                .filter(|(_, _, tree)| !tree.to_be_chopped)
                .min_by_key(|(_, p, _)| p.distance_squared(pos.0) as i32)
            else {
                return;
            };
            commands.entity(entity).insert(ChopTask::new(tree));
            tree_meta.to_be_chopped = true;
        } else {
            commands.entity(entity).insert(DeliverTask {
                to: lumberworker.workplace,
            });
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
    trees: Query<(&Pos, &Tree)>,
) {
    for (jack, mut vill, mut task) in &mut lumberjacks {
        match task.stage {
            ChopStage::Goto => {
                let (target, _tree) = trees.get(task.tree).unwrap();
                commands.entity(jack).insert((MoveTask {
                    goal: target.block(),
                    distance: 2,
                },));
                task.stage = ChopStage::Chop;
            }
            ChopStage::Chop => {
                let (target, _tree) = trees.get(task.tree).unwrap();
                let cursor = level.recording_cursor();
                remove_tree(&mut level, target.block());
                let place = PlaceTask(level.pop_recording(cursor).collect());
                // Could also calculate from age?
                let mut amount = 0.;
                for set in &place.0 {
                    if let Log(..) = set.block {
                        amount += 4.;
                    }
                    if let Fence(..) = set.block {
                        amount += 1.;
                    }
                }
                vill.carry = Some(Stack::new(Good::Wood, amount));
                commands.entity(task.tree).despawn();
                commands.entity(jack).insert(place);
                task.stage = ChopStage::Finish;
            }
            ChopStage::Finish => {
                commands.entity(jack).remove::<ChopTask>();
            }
        }
    }
}
