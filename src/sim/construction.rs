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
    /// Whether it has the materials necessary for the next block
    pub has_materials: bool,
}

impl ConstructionSite {
    pub fn new(blocks: ConsList) -> Self {
        Self {
            todo: blocks,
            has_builder: false,
            has_materials: false,
        }
    }
}

pub fn new_construction_site(
    mut commands: Commands,
    new: Query<(Entity, &ConstructionSite, Option<&Pile>), Added<ConstructionSite>>,
) {
    for (entity, site, existing_pile) in &new {
        let mut stock = existing_pile.cloned().unwrap_or_default();
        let mut requested = Goods::default();
        let mut priority = None;
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
    }
}

pub fn build(
    mut commands: Commands,
    mut replay: ResMut<Replay>,
    mut builders: Query<(Entity, &mut Villager, &BuildTask), (With<Villager>, Without<MoveTask>)>,
    mut buildings: Query<(Entity, &mut ConstructionSite, &mut Pile, &mut InPile)>,
) {
    for (builder, mut villager, build_task) in &mut builders {
        let Ok((e_building, mut building, mut pile, mut in_pile)) =
            buildings.get_mut(build_task.building)
        else {
            continue;
        };
        match building.todo.front().copied() {
            Some(ConsItem::Goto(goto)) => {
                commands.entity(builder).insert(goto);
                building.todo.pop_front();
            }
            Some(ConsItem::Carry(stack)) => villager.carry = stack,
            Some(ConsItem::Set(set)) => {
                if let Some(missing) = pile.try_consume(set.block) {
                    building.has_builder = false;
                    building.has_materials = false;
                    in_pile.priority = Some(missing);
                    commands.entity(builder).remove::<BuildTask>();
                } else {
                    // TODO: check if current block is still the same as when the ConsList was created
                    replay.block(set.pos, set.block);
                    replay.dust(set.pos);
                    building.todo.pop_front();
                }
            }
            None => {
                commands.entity(builder).remove::<BuildTask>();
                commands
                    .entity(e_building)
                    .remove::<(InPile, ConstructionSite)>()
                    .insert((
                        Built,
                        OutPile {
                            available: pile.goods.clone(),
                        },
                    ));
            }
        }
    }
}

pub fn check_construction_site_readiness(
    mut query: Query<(&mut ConstructionSite, &Pile), Changed<Pile>>,
) {
    for (mut site, pile) in &mut query {
        if !site.has_materials {
            match site.todo[0] {
                ConsItem::Goto(_) | ConsItem::Carry(_) => {
                    site.has_materials = true;
                }
                ConsItem::Set(set) => {
                    if let Some(needed) = goods_for_block(set.block) {
                        if pile.has(needed) {
                            site.has_materials = true;
                        }
                    } else {
                        site.has_materials = true
                    }
                }
            }
        }
    }
}
