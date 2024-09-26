use std::{convert::identity, f32::consts::PI};

use crate::*;
use bevy_math::FloatOrd;
use itertools::Itertools;
use sim::*;

use self::storage_pile::StonePile;

pub fn quarryable(block: Block) -> bool {
    matches!(
        block,
        Full(Stone | Granite | Andesite | Diorite | Sandstone | RedSandstone) | Terracotta(_)
    )
}

#[derive(PartialEq, Copy, Clone)]
struct Params {
    pos: IVec2,
    dir: f32,
}

impl Params {
    fn base_area(self) -> impl Iterator<Item = IVec2> {
        Rect::new_centered(self.pos, IVec2::splat(7))
            .into_iter()
            .filter(move |c| (*c - self.pos).as_vec2().length() < 4.)
    }

    /// Area used to determine suitability for quarrying
    fn probed_mining_area(self) -> impl Iterator<Item = IVec2> {
        let off = (self.dir_vec2() * 10.).as_ivec2();
        Rect::new_centered(self.pos, IVec2::splat(23))
            .into_iter()
            .filter(move |c| (*c - self.pos).as_vec2().length() < 10.)
            .map(move |c| c + off)
    }

    fn dir_vec2(self) -> Vec2 {
        vec2(self.dir.cos(), self.dir.sin())
    }
}

#[derive(Component, PartialEq)]
pub struct Quarry {
    dir: f32,
    // Reverse order
    to_mine: Vec<IVec2>,
    crane_species: TreeSpecies,
    crane_pos: IVec3,
    pub crane_rot: i32,
    crane_rot_cooldown: i32,
    pub crane_rot_target: i32,
}

impl Quarry {
    pub fn rotating(&self) -> bool {
        (self.crane_rot != self.crane_rot_target) | (self.crane_rot_cooldown != 0)
    }
}

#[derive(Component)]
pub struct Mason {
    workplace: Entity,
    ready_to_work: bool,
}

pub fn plan_quarry(
    mut commands: Commands,
    level: Res<Level>,
    planned: Query<(), (With<Quarry>, With<Planned>)>,
    center: Query<&Pos, With<CityCenter>>,
) {
    if !planned.is_empty() {
        return;
    }

    let Some(params) = optimize(
        Params {
            pos: center.single().0.block().truncate(),
            dir: rand(0. ..2. * PI),
        },
        |params, temperature| {
            let max_move = (60. * temperature) as i32;
            params.pos += ivec2(rand(-max_move..=max_move), rand(-max_move..=max_move));
            params.dir += (rand(-1. ..1.)) * 2. * PI * temperature.min(0.5);

            if !level.free(params.base_area()) || !level.free(params.probed_mining_area()) {
                return f32::INFINITY;
            }
            let mut distance = level.reachability[params.pos] as f32 - 650.;
            // Penalize quarries near city center
            if distance < 0. {
                distance *= -5.
            }
            // TODO: determine floor height here, weighed by towards lower points along border
            let avg_start_height = level.height.average(params.base_area()) as i32;

            let mut stone = 0;
            let mut columns = 0;
            for column in params.probed_mining_area() {
                columns += 1;
                for z in avg_start_height..avg_start_height + 15 {
                    match level(column.extend(z)) {
                        block if quarryable(block) => stone += 1,
                        other if !other.solid() => break,
                        _ => (),
                    }
                }
            }
            let avg_stone = stone as f32 / columns as f32;

            // if avg_stone < 5. {
            //     return f32::INFINITY;
            // }
            let area = Rect::new_centered(params.pos, IVec2::splat(7));
            if area.into_iter().any(|c| level.water[c].is_some()) {
                return f32::INFINITY;
            }
            /*wateryness(&level, area) * 2000. +*/
            unevenness(&level, area) * 1.5 - avg_stone * 5. + distance / 100.
        },
        300,
        5,
    ) else {
        println!("failed to place quarry");
        return;
    };

    let mut to_mine = params
        .probed_mining_area()
        .filter(|c| (*c - params.pos).length() >= 4.)
        .collect_vec();
    to_mine.sort_by_key(|c| {
        FloatOrd(-(c.as_vec2() + params.dir_vec2() * 5. - params.pos.as_vec2()).length())
    });
    to_mine.drain(..(to_mine.len() as f32 * 0.35) as usize);

    let rect = Rect::new_centered(params.pos, ivec2(7, 7));
    let floor = level.height.average(rect.border()).round() as i32;

    let pos = params.pos.extend(floor + 1);
    commands.spawn((
        Pos(pos.as_vec3()),
        Planned(
            params
                .base_area()
                .chain(params.probed_mining_area())
                .collect(),
        ),
        Quarry {
            dir: params.dir,
            to_mine,
            crane_species: level.biome[params.pos].random_tree_species(),
            crane_pos: pos + (params.dir_vec2() * rand(-1.6..1.)).as_ivec2().extend(0),
            crane_rot: 0,
            crane_rot_cooldown: 0,
            crane_rot_target: 0,
        },
    ));
}

pub fn test_build_quarry(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut untree: Untree,
    new: Query<(Entity, &Pos, &Quarry), Added<ToBeBuild>>,
) {
    for (entity, pos, quarry) in &new {
        commands
            .entity(entity)
            .remove::<ToBeBuild>()
            .insert(ConstructionSite::new(quarry::make_quarry(
                &mut level,
                &mut untree,
                pos.block(),
                quarry,
            )));
    }
}

fn make_quarry(level: &mut Level, untree: &mut Untree, pos: IVec3, quarry: &Quarry) -> ConsList {
    let cursor = level.recording_cursor();
    untree.remove_trees(level, Rect::new_centered(pos.truncate(), ivec2(9, 9)));

    for column in (Params {
        pos: pos.truncate(),
        dir: quarry.dir,
    })
    .base_area()
    {
        let base = level.height[column].min(pos.z - 1);
        level.fill_at(Some(column), base..pos.z, PackedMud);
        level.height[column] = pos.z - 1;
        level.fill_at(Some(column), pos.z..pos.z + 8, Air);
    }

    prefab("crane_rot_a").build(
        level,
        quarry.crane_pos,
        YNeg,
        false,
        false,
        quarry.crane_species,
        identity,
    );

    level.pop_recording(cursor).map(ConsItem::Set).collect()
}

pub fn make_stone_piles(
    mut commands: Commands,
    mut level: ResMut<Level>,
    new_quarries: Query<&Pos, (With<Quarry>, Added<Built>)>,
    mut untree: Untree,
) {
    for quarry in &new_quarries {
        // TODO: disincentivise stone piles located higher than the quarry?
        let (pos, _, params) = StonePile::make(&mut level, &mut untree, quarry.truncate());
        commands.spawn((
            Pos(pos),
            params,
            OutPile::default(),
            Pile::new(default(), 3),
            StoragePile,
        ));
    }
}

pub fn quarry_rotation(
    mut quarries: Query<&mut Quarry, (Without<Planned>, Without<ConstructionSite>)>,
) {
    for mut quarry in &mut quarries {
        if rand(0.995) {
            continue;
        }
        quarry.crane_rot_target = rand(0..16);
    }
}

pub fn update_quarry_rotation(
    mut level: ResMut<Level>,
    mut quarries: Query<&mut Quarry, (Without<Planned>, Without<ConstructionSite>)>,
) {
    for mut quarry in &mut quarries {
        if quarry.crane_rot_cooldown > 0 {
            quarry.crane_rot_cooldown -= 1
        } else {
            quarry.crane_rot_cooldown = 20;
            let diff = quarry.crane_rot_target - quarry.crane_rot;
            if diff != 0 {
                let dir = if diff.abs() < 8 {
                    diff.signum()
                } else {
                    -diff.signum()
                };
                quarry.crane_rot = (quarry.crane_rot + dir).rem_euclid(16);
                let (name, rot, flip_x) = [
                    ("crane_rot_a", YNeg, false),
                    ("crane_rot_c", YNeg, false),
                    ("crane_rot_a", YNeg, true),
                    ("crane_rot_b", YNeg, true),
                    ("crane_rot_a", XPos, false),
                    ("crane_rot_c", XPos, false),
                    ("crane_rot_a", XPos, true),
                    ("crane_rot_b", XPos, true),
                    ("crane_rot_a", YPos, false),
                    ("crane_rot_c", YPos, false),
                    ("crane_rot_a", YPos, true),
                    ("crane_rot_b", YPos, true),
                    ("crane_rot_a", XNeg, false),
                    ("crane_rot_c", XNeg, false),
                    ("crane_rot_a", XNeg, true),
                    ("crane_rot_b", XNeg, true),
                ][quarry.crane_rot as usize];
                prefab(name).build(
                    &mut level,
                    quarry.crane_pos,
                    rot,
                    flip_x,
                    false,
                    quarry.crane_species,
                    identity,
                );
            }
        }
    }
}

pub fn assign_worker(
    mut commands: Commands,
    available: Query<(Entity, &Pos), With<Jobless>>,
    new: Query<(Entity, &Pos), (With<Quarry>, Added<Built>)>,
) {
    let assigned = Vec::new();
    for (workplace, pos) in &new {
        let mut possible_workers = available
            .iter()
            .filter(|(e, _)| !assigned.contains(e))
            .collect_vec();
        possible_workers.sort_by_key(|(_, p)| p.distance_squared(pos.0) as i32);
        for &(worker, _) in possible_workers.iter().take(2) {
            commands.entity(worker).remove::<Jobless>().insert(Mason {
                workplace,
                ready_to_work: true,
            });
        }
    }
}

pub fn work(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut untree: Untree,
    pos: Query<&Pos>,
    mut workers: Query<
        (Entity, &Villager, &mut Mason),
        (Without<PlaceTask>, Without<DeliverTask>, Without<MoveTask>),
    >,
    mut quarries: Query<&mut Quarry>,
    piles: Query<(Entity, &Pos, &Pile, &StonePile), With<StoragePile>>,
) {
    for (worker, villager, mut mason) in &mut workers {
        let worker_pos = pos.get(worker).unwrap();
        if mason.ready_to_work {
            // Go mining
            // First move there
            let mut quarry = quarries.get_mut(mason.workplace).unwrap();
            let Some(target) = quarry.to_mine.pop() else {
                commands.entity(worker).remove::<Mason>().insert(Jobless);
                continue;
            };
            let floor = pos.get(mason.workplace).unwrap().block().z;
            mason.ready_to_work = false;

            let mut place = PlaceTask(default());
            place.push_back(ConsItem::Goto(MoveTask {
                goal: target.extend(floor + 1),
                distance: 2,
            }));

            // Then mine
            let cursor = level.recording_cursor();
            untree.remove_trees(&mut level, Some(target));

            const CEILING: i32 = 8;
            for z in floor..floor + CEILING {
                level(target.extend(z), Air)
            }
            for z in floor + CEILING.. {
                if level(target.extend(z)).soil() {
                    level(target.extend(z), Air)
                } else {
                    break;
                }
            }

            let rec = level.pop_recording(cursor).collect_vec();
            let amount = rec.iter().filter(|set| quarryable(set.previous)).count() as f32 * 1.5;
            place.extend(rec.into_iter().map(ConsItem::Set));
            place.push_back(ConsItem::Carry(Some(Stack::new(Good::Stone, amount))));

            commands.entity(worker).insert(place);
        } else if let Some(stack) = villager.carry {
            // Drop off stone
            if let Some((to, _, _, _)) = piles
                .iter()
                .filter(|(_, pile_pos, current, stone_pile)| {
                    current.space_available(
                        Good::Stone,
                        stone_pile.max(),
                        min_walk_ticks(worker_pos.0, pile_pos.0),
                    ) >= stack.amount
                })
                .min_by_key(|(_, pos, _, _)| pos.distance(worker_pos.0) as i32)
            {
                commands.entity(worker).insert(DeliverTask { to });
            }
        } else {
            // Return home
            commands
                .entity(worker)
                .insert(MoveTask::new(pos.get(mason.workplace).unwrap().block()));
            mason.ready_to_work = true;
        }
    }
}
