use crate::*;
use bevy_ecs::prelude::*;
use sim::*;

#[derive(Component)]
pub struct Lumberworker {
    workplace: Entity,
    ready_to_work: bool,
}

#[derive(Component, Eq, PartialEq, Copy, Clone)]
pub struct LumberPile {
    axis: HAxis,
    width: i32,
    length: i32,
}

impl LumberPile {
    fn max(&self) -> i32 {
        self.length
            * 4
            * match self.width {
                3 => 7,
                4 => 10,
                5 => 13,
                _ => unreachable!(),
            }
    }
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
    lumber_piles: Query<(Entity, &Pos), With<LumberPile>>,
) {
    for (entity, villager, mut lumberworker) in &mut workers {
        let worker_pos = pos.get(entity).unwrap();
        if lumberworker.ready_to_work {
            // Go chopping
            let Some((tree, _, mut tree_meta)) = trees
                .iter_mut()
                .filter(|(_, _, tree)| !tree.to_be_chopped)
                .min_by_key(|(_, p, _)| p.distance_squared(worker_pos.0) as i32)
            else {
                return;
            };
            commands.entity(entity).insert(ChopTask::new(tree));
            tree_meta.to_be_chopped = true;
            lumberworker.ready_to_work = false;
        } else if villager.carry.is_none() {
            // Return home
            commands.entity(entity).insert(MoveTask::new(
                pos.get(lumberworker.workplace).unwrap().block(),
            ));
            lumberworker.ready_to_work = true;
        } else if let Some((to, _)) = lumber_piles
            .iter()
            .min_by_key(|(_, pile)| pile.distance(worker_pos.0) as i32)
        {
            // Drop off lumber
            commands.entity(entity).insert(DeliverTask { to });
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
                let mut amount = 0.;
                for set in &place.0 {
                    amount += match set.previous {
                        Log(..) => 4.,
                        Fence(..) => 1.,
                        Leaves(..) => 0.25,
                        _ => 0.,
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

pub fn make_lumber_piles(
    mut commands: Commands,
    level: Res<Level>,
    blocked: Query<&Blocked>,
    center: Query<&Pos, With<CityCenter>>,
    new_lumberjacks: Query<&Pos, (With<Lumberjack>, Added<Built>)>,
) {
    for lumberjack in &new_lumberjacks {
        let center = center.single().truncate();

        let params = LumberPile {
            axis: if 0.5 > rand() { HAxis::X } else { HAxis::Y },
            width: 3,
            length: 5,
        };
        let area = |pos, params: LumberPile| {
            Rect::new_centered(
                pos,
                match params.axis {
                    HAxis::X => ivec2(params.length, params.width),
                    HAxis::Y => ivec2(params.width, params.length),
                },
            )
        };
        let (pos, axis) = optimize(
            (lumberjack.truncate().block(), params),
            |(mut pos, mut params), temperature| {
                if 0.2 > rand() {
                    params.axis = params.axis.rotated()
                } else if 0.3 > rand() {
                    params.width = rand_range(3..=5);
                    params.length = rand_range(5..=6);
                } else {
                    let max_move = (20. * temperature) as i32;
                    pos += ivec2(
                        rand_range(-max_move..=max_move),
                        rand_range(-max_move..=max_move),
                    );
                }
                let area = area(pos, params);
                (level.area().contains(pos)
                    & not_blocked(&blocked, area)
                    & (wateryness(&level, area) == 0.))
                    .then_some((pos, params))
            },
            |(pos, params)| {
                let center_distance = center.distance(pos.as_vec2()) / 70.;
                // TODO: use actual pathfinding distance (when there are proper pathable workplaces)
                let worker_distance = lumberjack.truncate().distance(pos.as_vec2()) / 20.;
                let size_bonus = (params.width + params.length) as f32 * 4.;
                center_distance + worker_distance + unevenness(&level, area(*pos, *params)) * 1.
                    - size_bonus
            },
            100,
        );

        commands.spawn((
            Pos(level.ground(pos).as_vec3() + Vec3::Z),
            params,
            Pile::default(),
            Blocked(area(pos, axis)),
        ));

        // TODO: Clear trees here
    }
}

pub fn update_lumber_pile_visuals(
    mut level: ResMut<Level>,
    query: Query<(&Pos, &LumberPile, &Pile), Changed<Pile>>,
) {
    for (pos, lumberpile, pile) in &query {
        let amount = pile.get(&Good::Wood).copied().unwrap_or_default();
        let logs = (amount / (4. * lumberpile.length as f32)).round() as usize;
        let log_positions: &[(i32, i32)] = match lumberpile.width {
            3 => &[(0, 0), (-1, 0), (1, 0), (0, 1), (1, 1), (-1, 1), (0, 2)],
            4 => &[
                (0, 0),
                (-1, 0),
                (1, 0),
                (2, 0),
                (1, 1),
                (0, 1),
                (-1, 1),
                (0, 2),
                (2, 1),
                (1, 2),
            ],
            5 => &[
                (0, 0),
                (-1, 0),
                (-2, 0),
                (1, 0),
                (2, 0),
                (1, 1),
                (0, 1),
                (-1, 1),
                (0, 2),
                (2, 1),
                (1, 2),
                (-2, 1),
                (-1, 2),
            ],
            _ => unreachable!(),
        };
        for (i, (side, z)) in log_positions.iter().copied().enumerate() {
            for along in -lumberpile.length / 2..=(lumberpile.length + 1) / 2 {
                level[pos.block()
                    + (lumberpile.axis.pos() * along + lumberpile.axis.rotated().pos() * side)
                        .extend(z)] = if i < logs {
                    Log(Spruce, LogType::Normal(lumberpile.axis.into()))
                } else {
                    Air
                }
            }
        }
        for along in [1 - lumberpile.length / 2, (lumberpile.length + 1) / 2 - 1] {
            for side in [-1, 1, 0] {
                let mut pos = pos.block()
                    + (lumberpile.axis.pos() * along + lumberpile.axis.rotated().pos() * side)
                        .extend(0);
                if !level[pos - IVec3::Z].solid() {
                    continue;
                }
                while level[pos].solid() {
                    pos.z += 1
                }
                level[pos] = if logs == 0 {
                    Air
                } else {
                    Rail(lumberpile.axis)
                }
            }
        }
    }
}
