use bevy_ecs::prelude::*;

use crate::SliceExt;

#[derive(Component)]
pub struct Arrival {
    pub tick: i32,
    pub kind: ArrivalKind,
}

pub enum ArrivalKind {
    Migration,
}

pub fn make_name() -> Name {
    Name::new(*include!("../../names.rs").choose()).to_owned()
}
