use crate::*;
use bevy_ecs::prelude::*;
use sim::*;

#[derive(Component)]
pub struct BuildTask {
    pub building: Entity,
}

#[derive(Component)]
pub struct Built;

#[derive(Component, Debug)]
pub struct ConstructionSite {
    pub todo: PlaceList,
    pub has_builder: bool,
    /// Whether it has the materials necessary for the next block
    pub has_materials: bool,
}

impl ConstructionSite {
    pub fn new(blocks: PlaceList) -> Self {
        Self {
            todo: blocks,
            has_builder: false,
            has_materials: false,
        }
    }
}

pub fn new_construction_site(
    mut commands: Commands,
    new: Query<(Entity, &ConstructionSite), Added<ConstructionSite>>,
) {
    for (entity, site) in &new {
        let mut stock = Pile::default();
        let mut requested = Pile::default();
        let mut priority = None;
        for set_block in &site.todo {
            if let Some(stack) = goods_for_block(set_block.block) {
                requested.add(stack);
                if priority.is_none() {
                    priority = Some(stack.kind)
                }
            }
            if let Some(mined) = goods_for_block(set_block.previous) {
                stock.add(mined)
            }
        }
        for (good, amount) in &stock.goods {
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
    mut builders: Query<(Entity, &BuildTask), (With<Villager>, Without<MoveTask>)>,
    mut buildings: Query<(Entity, &mut ConstructionSite, &mut Pile)>,
) {
    for (builder, build_task) in &mut builders {
        let Ok((entity, mut building, mut pile)) = buildings.get_mut(build_task.building) else {
            continue;
        };
        if let Some(set) = building.todo.get(0).copied() {
            if let Some(block) = pile.build(set.block) {
                replay.block(set.pos, block);
                replay.dust(set.pos);
                building.todo.pop_front();
            } else {
                building.has_builder = false;
                building.has_materials = false;
                commands.entity(builder).remove::<BuildTask>();
            }
        } else {
            replay.dbg("Building finished");
            commands.entity(builder).remove::<BuildTask>();
            commands
                .entity(entity)
                .remove::<(InPile, ConstructionSite)>()
                .insert((
                    Built,
                    OutPile {
                        available: pile.clone(),
                    },
                ));
        }
    }
}

pub fn check_construction_site_readiness(
    mut query: Query<(&mut ConstructionSite, &Pile, &mut InPile), Changed<Pile>>,
) {
    for (mut site, pile, mut in_pile) in &mut query {
        if !site.has_materials {
            if let Some(needed) = goods_for_block(site.todo[0].block) {
                if pile.has(needed) {
                    site.has_materials = true;
                } else if in_pile.priority.is_none() {
                    in_pile.priority = Some(needed.kind);
                }
            } else {
                site.has_materials = true
            }
        }
    }
}
