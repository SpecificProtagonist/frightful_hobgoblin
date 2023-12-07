#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

pub mod building_plan;
mod construction;
mod logistics;
pub mod lumberjack;
mod main_loop;
pub mod quarry;

pub use main_loop::sim;

use std::collections::VecDeque;

use crate::goods::*;
use crate::make_trees::grow_trees;
use crate::optimize::optimize;
use crate::remove_foliage::remove_tree;
use crate::*;
use crate::{pathfind::pathfind, remove_foliage::remove_trees, replay::*};
use building_plan::*;
use construction::*;
use logistics::*;
use lumberjack::Lumberjack;

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_math::Vec2Swizzles;

#[derive(Resource, Default, Deref, DerefMut)]
pub struct Tick(pub i32);

#[derive(Component)]
pub struct CityCenter;

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

#[derive(Component, Default)]
pub struct InBoat(pub Id);

/// Doesn't have a specialized job, but can carry and build.
#[derive(Component)]
pub struct Jobless;

pub type PlaceList = VecDeque<SetBlock>;

#[derive(Component)]
pub struct PlaceTask(PlaceList);

#[derive(Component)]
pub struct Tree {
    _species: TreeSpecies,
    to_be_chopped: bool,
}

impl Tree {
    fn new(species: TreeSpecies) -> Self {
        Self {
            _species: species,
            to_be_chopped: false,
        }
    }
}

fn assign_work(
    mut commands: Commands,
    mut replay: ResMut<Replay>,
    idle: Query<
        (Entity, &Pos),
        (
            With<Villager>,
            With<Jobless>,
            Without<DeliverTask>,
            Without<BuildTask>,
        ),
    >,
    mut out_piles: Query<(Entity, &Pos, &mut OutPile)>,
    mut in_piles: Query<(Entity, &Pos, &mut InPile)>,
    mut construction_sites: Query<(Entity, &Pos, &mut ConstructionSite)>,
) {
    for (vill, vil_pos) in &idle {
        // Construction
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

        // Transport
        if let Some((_, task)) = out_piles
            .iter_mut()
            .filter_map(|(out_entity, out_pos, out_pile)| {
                let mut best_score = f32::INFINITY;
                let mut task = None;
                for (good, &amount) in out_pile.available.iter() {
                    if amount == 0. {
                        continue;
                    }
                    for (in_entity, in_pos, in_pile) in &mut in_piles {
                        if let Some(&requested) = in_pile.requested.get(good)
                            && requested > 0.
                        {
                            if let Some(priority) = in_pile.priority {
                                if priority != *good {
                                    continue;
                                }
                            }
                            let mut score = out_pos.distance_squared(in_pos.0);
                            // Try to reduce the amount of trips
                            if amount < requested {
                                score *= 2.;
                            }
                            if score < best_score {
                                best_score = score;
                                task = Some((
                                    PickupTask {
                                        from: out_entity,
                                        stack: Stack::new(
                                            *good,
                                            amount.min(requested).min(CARRY_CAPACITY),
                                        ),
                                        max_stack: requested.min(CARRY_CAPACITY),
                                    },
                                    DeliverTask { to: in_entity },
                                ));
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
                .get_component_mut::<OutPile>(task.0.from)
                .unwrap()
                .available
                .remove(task.0.stack);
            in_piles
                .get_component_mut::<InPile>(task.1.to)
                .unwrap()
                .requested
                .remove(task.0.stack);
            replay.dbg("assign carry");
            commands.entity(vill).insert(task);
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
