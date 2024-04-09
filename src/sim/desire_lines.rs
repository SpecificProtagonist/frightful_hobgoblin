use bevy_math::Vec3Swizzles;

use super::*;

#[derive(Resource, Deref, DerefMut)]
pub struct DesireLines(ColumnMap<i32>);

impl FromWorld for DesireLines {
    fn from_world(world: &mut World) -> Self {
        Self(world.resource::<Level>().column_map(1, 0))
    }
}

pub fn desire_lines(
    mut level: ResMut<Level>,
    mut dl: ResMut<DesireLines>,
    walkers: Query<(&Pos, &PrevPos), With<Villager>>,
) {
    if 0.3 < rand() {
        return;
    }
    for (pos, prev_pos) in &walkers {
        // Check if walking
        if pos.xy() == prev_pos.xy() {
            continue;
        }
        // Broaden the path smoothly
        let pos = pos.block() - IVec3::Z;
        let alt_pos = pos
            + if 0.5 < rand() { IVec3::X } else { IVec3::Y } * if 0.5 < rand() { -1 } else { 1 };
        fn affected(block: Block) -> bool {
            matches!(
                block,
                Grass | Podzol | Dirt | CoarseDirt | Path | Sand | SnowBlock
            )
        }
        if !affected(level(pos)) {
            continue;
        }
        let (pos, wear) =
            if (dl[alt_pos.truncate()] + 1 < dl[pos.truncate()]) & affected(level(alt_pos)) {
                (alt_pos, &mut dl[alt_pos.truncate()])
            } else {
                (pos, &mut dl[pos.truncate()])
            };
        // Apply wear
        *wear += 1;
        let wear = *wear;
        if matches!(
            level(pos + IVec3::Z),
            SmallPlant(..) | TallPlant(..) | SnowLayer
        ) {
            level(pos + IVec3::Z, Air)
        } else {
            match level(pos) {
                Grass | Podzol if wear > 7 => {
                    level(pos, if 0.5 < rand() { Dirt } else { CoarseDirt })
                }
                Dirt | CoarseDirt
                    if (wear > 12) & {
                        // Try to add some steps (only works for shallow slopes)
                        let lower_neighbors = NEIGHBORS_2D
                            .iter()
                            .any(|off| level.height[pos.truncate() + *off] < level.height[pos]);
                        let heigher_neighbor_wear = NEIGHBORS_2D.iter().any(|off| {
                            (level.height[pos.truncate() + *off] > level.height[pos])
                                && (dl[pos.truncate() + *off] >= dl[pos] - 1)
                        });
                        heigher_neighbor_wear & !lower_neighbors
                    } =>
                {
                    level(pos + IVec3::Z, Slab(Granite, Bottom))
                }
                Dirt | CoarseDirt if (wear > 17) & (level(pos + IVec3::Z) == Air) => {
                    level(pos, Path)
                }
                Path if wear > 24 => level(pos, Gravel),
                Sand if wear > 17 => level(pos, PackedMud),
                SnowBlock if (wear > 9) & (0.3 < rand()) => level(pos, Gravel),
                _ => (),
            }
        }
    }
}
