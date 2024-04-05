use crate::*;
use itertools::Itertools;
use sim::*;

#[derive(Component)]
pub struct Lumberjack {
    pub area: Rect,
}

#[derive(Component)]
pub struct TreeIsNearLumberCamp;

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
    fn max(&self) -> f32 {
        (self.length
            * 4
            * match self.width {
                3 => 7,
                4 => 10,
                5 => 13,
                _ => unreachable!(),
            }) as f32
    }
}

// This is a separate component to allow giving this task to other villagers too
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ChopTask {
    tree: Entity,
    chopped: bool,
}

impl ChopTask {
    pub fn new(tree: Entity) -> Self {
        Self {
            tree,
            chopped: false,
        }
    }
}

pub fn assign_worker(
    mut commands: Commands,
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
    trees: Query<(&Pos, &Tree)>,
) {
    for (jack, mut vill, mut task) in &mut lumberjacks {
        if !task.chopped {
            let (target, _tree) = trees.get(task.tree).unwrap();

            let mut place = PlaceTask(default());
            place.push_back(ConsItem::Goto(MoveTask {
                goal: target.block(),
                distance: 2,
            }));

            let cursor = level.recording_cursor();
            remove_tree(&mut level, target.block());
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

            commands.entity(task.tree).despawn();
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
        let (pos, params) = optimize(
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

                if !level.unblocked(area) | (wateryness(&level, area) > 0.) {
                    return None;
                }
                let center_distance = center.distance(pos.as_vec2()) / 70.;
                // TODO: use actual pathfinding distance (when there are proper pathable workplaces)
                let worker_distance = lumberjack.truncate().distance(pos.as_vec2()) / 20.;
                let size_bonus = (params.width + params.length) as f32 * 4.;
                let score =
                    center_distance + worker_distance + unevenness(&level, area) * 1. - size_bonus;
                Some(((pos, params), score))
            },
            100,
        )
        .unwrap();

        let z = level.average_height(area(pos, params).border()) + 1.;
        level.set_blocked(area(pos, params));
        commands.spawn((
            Pos(pos.as_vec2().extend(z)),
            params,
            Pile {
                goods: default(),
                interact_distance: params.width,
            },
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
                level(
                    pos.block()
                        + (lumberpile.axis.pos() * along + lumberpile.axis.rotated().pos() * side)
                            .extend(z),
                    if i < logs {
                        Log(Spruce, LogType::Normal(lumberpile.axis.into()))
                    } else {
                        Air
                    },
                )
            }
        }
        for along in [1 - lumberpile.length / 2, (lumberpile.length + 1) / 2 - 1] {
            for side in -(lumberpile.width - 1) / 2..=lumberpile.width / 2 {
                let mut pos = pos.block()
                    + (lumberpile.axis.pos() * along + lumberpile.axis.rotated().pos() * side)
                        .extend(1);
                if !level(pos - IVec3::Z).solid() {
                    continue;
                }
                while level(pos).solid() {
                    pos.z += 1
                }
                level(
                    pos,
                    if logs == 0 {
                        Air
                    } else {
                        Rail(lumberpile.axis)
                    },
                )
            }
        }
    }
}
