use crate::*;
use sim::*;

use self::storage_pile::StonePile;

#[derive(Component, Eq, PartialEq, Copy, Clone)]
pub struct Quarry {
    pub area: Rect,
    // TODO: better shapes / not just in a compas direction
    pub dir: HDir,
}

impl Quarry {
    /// Area used to determine suitability for quarrying
    pub fn probing_area(&self) -> Rect {
        Rect::new_centered(
            self.area.center() + IVec2::from(self.dir) * 9,
            IVec2::splat(11),
        )
    }
}

#[derive(Component)]
pub struct Mason {
    workplace: Entity,
    ready_to_work: bool,
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
        commands.entity(worker).remove::<Jobless>().insert(Mason {
            workplace,
            ready_to_work: true,
        });
    }
}

pub fn make_quarry(level: &mut Level, quarry: Quarry) -> ConsList {
    let floor = level.average_height(quarry.area.border()).round() as i32;

    let cursor = level.recording_cursor();
    remove_trees(level, quarry.area.grow(1));
    for column in quarry.area {
        let mut pos = level.ground(column);
        pos.z = pos.z.min(floor);
        level.height[column] = pos.z;
        level(pos, PackedMud)
    }
    level.fill_at(quarry.area, floor + 1..floor + 5, Air);

    let pos = level.ground(quarry.area.center() + ivec2(rand_range(-2..=2), rand_range(-2..=2)))
        + IVec3::Z;
    level(pos, CraftingTable);
    let pos = level.ground(quarry.area.center() + ivec2(rand_range(-2..=2), rand_range(-2..=2)))
        + IVec3::Z;
    level(pos, Stonecutter(HAxis::X));

    level.pop_recording(cursor).map(ConsItem::Set).collect()
}

pub fn make_stone_piles(
    mut commands: Commands,
    mut level: ResMut<Level>,
    new_quarries: Query<&Pos, (With<Quarry>, Added<Built>)>,
) {
    for quarry in &new_quarries {
        let (pos, params) = StonePile::make(&mut level, quarry.truncate());
        commands.spawn((
            Pos(pos),
            params,
            OutPile {
                available: default(),
            },
            Pile {
                goods: default(),
                interact_distance: 2,
            },
        ));

        // TODO: Clear trees here
    }
}
