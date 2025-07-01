use crate::*;
use sim::*;

pub fn name(mut commands: Commands, new: Query<Entity, (With<Villager>, Without<Name>)>) {
    for entity in &new {
        commands
            .entity(entity)
            .insert(Name::new(*include!("../../names").choose()).to_owned());
    }
}
