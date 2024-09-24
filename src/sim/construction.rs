use crate::*;
use bevy_ecs::prelude::*;
use sim::*;

#[derive(Component, Debug)]
pub struct BuildTask {
    pub building: Entity,
}

#[derive(Component)]
pub struct Built;

#[derive(Component, Debug)]
pub struct ConstructionSite {
    pub todo: ConsList,
    pub has_builder: bool,
}

impl ConstructionSite {
    pub fn new(blocks: ConsList) -> Self {
        Self {
            todo: blocks,
            has_builder: false,
        }
    }

    pub fn has_materials(&self, self_pile: &Pile, ticks_until: i32) -> bool {
        if let Some(ConsItem::Set(set)) = self.todo.front() {
            if let Some(needed) = goods_for_block(set.block) {
                self_pile.available(needed.good, ticks_until) > needed.amount
            } else {
                true
            }
        } else {
            true
        }
    }
}

#[derive(Component)]
pub struct RemoveWhenBlocked {
    pub check_area: Vec<IVec3>,
    pub restore: Vec<SetBlock>,
}

pub fn new_construction_site(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut new: Query<(Entity, &mut ConstructionSite, Option<&Pile>), Added<ConstructionSite>>,
    check_for_removal: Query<(Entity, &RemoveWhenBlocked)>,
) {
    for (entity, mut site, existing_pile) in &mut new {
        // Cleanup doodahs
        let cursor = level.recording_cursor();
        for (entity, data) in &check_for_removal {
            if data.check_area.iter().any(|c| level(c).solid()) {
                for set in &data.restore {
                    if set.previous == level(set.pos) {
                        level(set.pos, set.block)
                    }
                }
                commands.entity(entity).despawn()
            }
        }
        for item in level.pop_recording(cursor).map(ConsItem::Set) {
            site.todo.push_back(item);
        }

        let mut stock = existing_pile.cloned().unwrap_or_default();
        let mut requested = Goods::default();
        let mut priority = None;
        // Calculate required materials
        for cons in &site.todo {
            let ConsItem::Set(set) = cons else { continue };
            if let Some(stack) = goods_for_block(set.block) {
                requested.add(stack);
                if priority.is_none() {
                    priority = Some(stack.good)
                }
            }
            if let Some(mined) = goods_for_block(set.previous) {
                stock.add(mined)
            }
        }
        for (good, amount) in stock.goods.iter() {
            requested.remove(Stack::new(*good, *amount))
        }
        commands.entity(entity).insert((
            stock,
            InPile {
                requested,
                priority,
            },
        ));

        // Add construction noises
        let mut i = 0;
        let mut sound_cooldown = 0;
        while let Some(item) = site.todo.get(i) {
            if let ConsItem::Set(set) = item {
                sound_cooldown -= 1;
                if sound_cooldown < 0 {
                    sound_cooldown = 120;
                    let pos = set.pos;
                    site.todo
                        .insert(i, ConsItem::Command(playsound("construction", pos)));
                    i += 1;
                }
            }
            i += 1;
        }
    }
}

pub fn build(
    mut commands: Commands,
    mut replay: ResMut<Replay>,
    mut builders: Query<(Entity, &mut Villager, &BuildTask), (With<Villager>, Without<MoveTask>)>,
    mut sites: Query<(Entity, &mut ConstructionSite, &mut Pile, &mut InPile)>,
) {
    for (builder, mut villager, build_task) in &mut builders {
        let Ok((e_building, mut building, mut pile, mut in_pile)) =
            sites.get_mut(build_task.building)
        else {
            continue;
        };
        match building.todo.front() {
            Some(ConsItem::Goto(goto)) => {
                commands.entity(builder).insert(*goto);
                building.todo.pop_front();
            }
            Some(ConsItem::Carry(stack)) => villager.carry = *stack,
            Some(ConsItem::Set(set)) => {
                if let Some(missing) = pile.try_consume(set.block) {
                    building.has_builder = false;
                    in_pile.priority = Some(missing);
                    commands.entity(builder).remove::<BuildTask>();
                } else {
                    // TODO: check if current block is still the same as when the ConsList was created
                    replay.block(set.pos, set.block, set.nbt.clone());
                    replay.dust(set.pos);
                    building.todo.pop_front();
                }
            }
            Some(ConsItem::Command(cmd)) => {
                replay.command(cmd.clone());
                building.todo.pop_front();
            }
            None => {
                commands.entity(builder).remove::<BuildTask>();
                commands
                    .entity(e_building)
                    .remove::<(InPile, ConstructionSite)>()
                    .insert((Built, OutPile::default()));
            }
        }
    }
}

// pub fn check_construction_site_readiness(
//     mut query: Query<(&mut ConstructionSite, &Pile), Changed<Pile>>,
// ) {
//     for (mut site, pile) in &mut query {
//         if !site.has_materials {
//             match &site.todo[0] {
//                 ConsItem::Goto(_) | ConsItem::Carry(_) => {
//                     site.has_materials = true;
//                 }
//                 ConsItem::Set(set) => {
//                     if let Some(needed) = goods_for_block(set.block) {
//                         if pile.available(needed) {
//                             site.has_materials = true;
//                         }
//                     } else {
//                         site.has_materials = true
//                     }
//                 }
//             }
//         }
//     }
// }
