use bevy_math::Vec3Swizzles;

use super::*;

#[derive(Resource, Deref, DerefMut)]
pub struct DesireLines(ColumnMap<i32>);

impl FromWorld for DesireLines {
    fn from_world(world: &mut World) -> Self {
        Self(world.resource::<Level>().column_map(0))
    }
}

pub fn desire_lines(
    mut level: ResMut<Level>,
    mut dl: ResMut<DesireLines>,
    walkers: Query<(&Pos, &PrevPos), With<Villager>>,
) {
    if rand(0.7) {
        return;
    }
    for (pos, prev_pos) in &walkers {
        // Check if walking
        if pos.xy() == prev_pos.xy() {
            continue;
        }
        let pos = pos.block() - IVec3::Z;
        add_desire_line(&mut level, &mut dl, pos);
    }
}

pub fn add_desire_line(level: &mut Level, dl: &mut DesireLines, pos: IVec3) {
    // Broaden the path smoothly
    let alt_pos =
        pos + if rand(0.5) { IVec3::X } else { IVec3::Y } * if rand(0.5) { -1 } else { 1 };
    fn affected(block: Block) -> bool {
        matches!(
            block,
            Grass
                | Podzol
                | Dirt
                | CoarseDirt
                | PackedMud
                | Sand
                | SnowBlock
                | PowderedSnow
                | Slab(Granite, Bottom)
        )
    }
    if !affected(level(pos)) {
        return;
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
            Grass | Podzol if wear > 7 => level(pos, if rand(0.5) { Dirt } else { CoarseDirt }),
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
                level(pos, PackedMud)
            }
            Slab(Granite, Bottom) if wear > 26 => level(pos, Slab(Andesite, Bottom)),
            PackedMud if wear > 24 => {
                if level(pos - IVec3::Z).solid() {
                    level(pos, Gravel)
                } else {
                    level(pos, Full(Cobble))
                }
            }
            Sand if wear > 17 => level(pos, PackedMud),
            SnowBlock | PowderedSnow if (wear > 9) & rand(0.7) => {
                if level(pos - IVec3::Z).solid() {
                    level(pos, Gravel)
                } else {
                    level(pos, Full(Cobble))
                }
            }
            _ => (),
        }
    }
}
