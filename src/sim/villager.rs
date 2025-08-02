use bevy_ecs::prelude::*;

use crate::sim::social::make_name;
use crate::sim::*;
use crate::*;

#[derive(Component, Default)]
#[require(Name = make_name())]
pub struct Villager {
    pub carry: Option<Stack>,
    pub carry_id: Id,
}

#[derive(Component, Default)]
pub struct InBoat(pub Id);

/// Doesn't have a specialized job, but can carry and build.
#[derive(Component)]
pub struct Jobless;

pub type ConsList = VecDeque<ConsItem>;

#[derive(Clone, Debug)]
pub enum ConsItem {
    Set(SetBlock),
    Goto(MoveTask),
    Carry(Option<Stack>),
    Command(String),
}

#[derive(Component, Deref, DerefMut)]
pub struct PlaceTask(pub ConsList);

pub fn assign_work_sys(
    mut commands: Commands,
    idle: Query<
        (Entity, &Pos),
        (
            With<Villager>,
            With<Jobless>,
            Without<DeliverTask>,
            Without<BuildTask>,
        ),
    >,
    mut out_piles: Query<(Entity, &Pos, &mut OutPile, &mut Pile)>,
    mut in_piles: Query<(Entity, &Pos, &mut InPile)>,
    mut construction_sites: Query<(Entity, &Pos, &mut ConstructionSite, &Pile), Without<OutPile>>,
) {
    for (vill, vil_pos) in &idle {
        // Construction
        if let Some((building, pos, mut site, _)) = construction_sites
            .iter_mut()
            .filter(|(_, site_pos, site, pile)| {
                site.has_materials(pile, min_walk_ticks(vil_pos.0, site_pos.0)) & !site.has_builder
            })
            .min_by_key(|(_, pos, _, _)| pos.distance_squared(vil_pos.0) as u32)
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
            .filter_map(|(out_entity, out_pos, out_pile, pile)| {
                let min_ticks = min_walk_ticks(vil_pos.0, out_pos.0);
                let mut best_score = f32::INFINITY;
                let mut task = None;
                for good in pile.goods.keys() {
                    let amount = pile.available(*good, min_ticks)
                        - out_pile.reserved.get(good).copied().unwrap_or(0.);
                    if amount <= 0. {
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
            *out_piles
                .get_mut(task.0.from)
                .unwrap()
                .2
                .reserved
                .entry(task.0.stack.good)
                .or_insert(0.) += task.0.stack.amount;
            in_piles
                .get_mut(task.1.to)
                .unwrap()
                .2
                .requested
                .remove(task.0.stack);
            commands.entity(vill).insert(task);
        }
    }
}

pub fn place_sys(
    mut commands: Commands,
    mut replay: ResMut<Replay>,
    mut builders: Query<(Entity, &mut Villager, &mut PlaceTask), Without<MoveTask>>,
) {
    for (entity, mut villager, mut build) in &mut builders {
        match build.0.pop_front() {
            Some(ConsItem::Set(set)) => {
                replay.block(set.pos, set.block, set.nbt);
            }
            Some(ConsItem::Goto(goto)) => {
                commands.entity(entity).insert(goto);
            }
            Some(ConsItem::Carry(stack)) => {
                villager.carry = stack;
            }
            Some(ConsItem::Command(cmd)) => replay.command(cmd),
            None => {
                commands.entity(entity).remove::<PlaceTask>();
            }
        }
    }
}
