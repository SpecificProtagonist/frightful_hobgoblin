use crate::*;
use itertools::Itertools;
use sim::*;

use crate::{
    sim::building_plan::{House, Planned},
    sim::CityCenter,
};

use self::construction::ConstructionSite;

// TODO: Generate villagers visiting stalls

#[derive(Component)]
pub struct MarketStall {
    pos: IVec2,
    facing: HDir,
}

pub fn init_stalls(mut commands: Commands, center: Query<&Pos, With<CityCenter>>) {
    let center = center.single().block().truncate();
    for x in -1..=1 {
        let offset = || ivec2(x * 7 + rand_range(0..=1), -rand_range(5..=6));
        commands.spawn(MarketStall {
            pos: center + offset(),
            facing: HDir::YPos,
        });
        commands.spawn(MarketStall {
            pos: center - offset(),
            facing: HDir::YNeg,
        });
    }
}

pub fn plan_stalls(
    mut commands: Commands,
    mut level: ResMut<Level>,
    houses: Query<(), (With<House>, Without<Planned>, Without<ConstructionSite>)>,
    possible_stalls: Query<(Entity, &MarketStall)>,
) {
    let houses = houses.iter().count();
    let stalls = 6 - possible_stalls.iter().count();
    let desired_stalls = houses / 3;
    if stalls < desired_stalls {
        let possible = possible_stalls.iter().collect_vec();
        let Some((entity, params)) = possible.try_choose() else {
            return;
        };
        commands.entity(*entity).remove::<MarketStall>().insert((
            Pos(level.ground(params.pos).as_vec3() + Vec3::Z),
            ConstructionSite::new(stall(&mut level, params.pos, params.facing)),
        ));
    }
}

fn stall(level: &mut Level, pos: IVec2, facing: HDir) -> ConsList {
    let cursor = level.recording_cursor();
    let z = level.height[pos];
    level.fill_at(
        Rect::new(
            pos - ivec2(2, 1).rotated(YPos.difference(facing)),
            pos + ivec2(2, 2).rotated(YPos.difference(facing)),
        ),
        z - 1..=z,
        |b: Block| if b.soil() | !b.solid() { PackedMud } else { b },
    );

    use Biome::*;
    let cloth_chance = match level.biome[pos] {
        Beach | Ocean | Desert => 1.,
        Snowy | Taiga | Forest | BirchForest | Jungles | DarkForest | CherryGrove => 0.2,
        _ => 0.6,
    };
    let stall = prefab(if cloth_chance > rand() {
        "stall_0"
    } else {
        "stall_1"
    });
    let wares = prefab(&format!("stall_wares_{}", rand_range(0..=6)));

    let pos = level.ground(pos) + IVec3::Z;
    let biome = level.biome[pos];
    stall.build(
        level,
        pos,
        facing,
        0.5 > rand(),
        false,
        biome.random_tree_species(),
        replace_wool_colors(),
    );
    wares.build(
        level,
        pos,
        facing,
        0.5 > rand(),
        false,
        biome.random_tree_species(),
        replace_wool_colors(),
    );
    level.pop_recording(cursor).map(ConsItem::Set).collect()
}

fn replace_wool_colors() -> impl Fn(Color) -> Color {
    let available = [
        LightGray, Gray, Orange, Red, Yellow, Purple, Green, Brown, LightBlue, Cyan,
    ];
    let mut replace = [White; 16];
    for color in &mut replace {
        *color = *available.choose()
    }
    move |c| replace[c as usize]
}
