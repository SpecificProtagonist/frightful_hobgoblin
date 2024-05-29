use crate::*;
use itertools::Itertools;
use sim::*;

use crate::{
    sim::building_plan::{HousePlan, Planned},
    sim::CityCenter,
};

use self::{construction::ConstructionSite, logistics::MoveTask};

// TODO: Generate villagers visiting stalls

#[derive(Component)]
pub struct MarketStall {
    pos: IVec2,
    facing: HDir,
}

#[derive(Component)]
pub struct StallNotYetPlanned;

pub fn init_stalls(mut commands: Commands, center: Query<&Pos, With<CityCenter>>) {
    let center = center.single().block().truncate();
    for x in -1..=1 {
        let offset = || ivec2(x * 8 + rand(0..=1), -rand(3..=4));
        commands.spawn((
            MarketStall {
                pos: center + offset(),
                facing: HDir::YPos,
            },
            StallNotYetPlanned,
        ));
        commands.spawn((
            MarketStall {
                pos: center - offset(),
                facing: HDir::YNeg,
            },
            StallNotYetPlanned,
        ));
    }
}

pub fn plan_stalls(
    mut commands: Commands,
    mut level: ResMut<Level>,
    houses: Query<(), (With<HousePlan>, Without<Planned>, Without<ConstructionSite>)>,
    possible_stalls: Query<(Entity, &MarketStall), With<StallNotYetPlanned>>,
) {
    let houses = houses.iter().count();
    let stalls = 6 - possible_stalls.iter().count();
    let desired_stalls = houses / 3;
    if stalls < desired_stalls {
        let possible = possible_stalls.iter().collect_vec();
        let Some((entity, params)) = possible.try_choose() else {
            return;
        };
        commands
            .entity(*entity)
            .remove::<StallNotYetPlanned>()
            .insert((
                Pos(level.ground(params.pos).as_vec3() + Vec3::Z),
                ConstructionSite::new(stall(&mut level, params.pos, params.facing)),
            ));
    }
}

fn stall(level: &mut Level, pos: IVec2, facing: HDir) -> ConsList {
    let pos = pos + facing.offset(-2, 0);
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
    let stall = prefab(if rand(cloth_chance) {
        "stall_0"
    } else {
        "stall_1"
    });
    let wares = prefab(&format!("stall_wares_{}", rand(0..=6)));

    let pos = level.ground(pos) + IVec3::Z;
    let biome = level.biome[pos];
    stall.build(
        level,
        pos,
        facing,
        rand(0.5),
        false,
        biome.random_tree_species(),
        replace_wool_colors(),
    );
    wares.build(
        level,
        pos,
        facing,
        rand(0.5),
        false,
        biome.random_tree_species(),
        replace_wool_colors(),
    );
    let mut rec: ConsList = level.pop_recording(cursor).map(ConsItem::Set).collect();
    for item in &mut rec {
        if let ConsItem::Set(SetBlock {
            block: Smoker(..),
            nbt,
            ..
        }) = item
        {
            *nbt = Some(loot::smoker())
        }
    }
    rec
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

pub fn upgrade_plaza(
    mut commands: Commands,
    mut level: ResMut<Level>,
    tick: Res<Tick>,
    center: Query<(Entity, &CityCenter)>,
    mut untree: Untree,
) {
    if tick.0 != 1000 {
        return;
    }
    let (entity, rect) = center.single();

    let cursor = level.recording_cursor();
    let mut rec = ConsList::new();

    // Visit blocks in a spiral from the center
    let mut offset = IVec2::ZERO;
    let mut dir = HDir::YNeg;
    for _ in 0..rect.size().max_element().pow(2) {
        // Rounded corners
        let metr = offset.as_vec2().powf(4.);
        if metr.x + metr.y < (rect.size().max_element() as f32 / 2.).powf(4.) + 0.6 {
            let pos = level.ground(rect.center() + offset);
            level(pos, PackedMud);
            if rand(0.2) {
                rec.push_back(ConsItem::Goto(MoveTask {
                    goal: pos + IVec3::Z,
                    distance: 2,
                }));
            }
            untree.remove_trees(&mut level, Some(pos.truncate()));
            level.pop_recording_into(&mut rec, cursor);
        }
        if (offset.x == offset.y)
            | (offset.x < 0) & (offset.x == -offset.y)
            | (offset.x > 0) & (offset.x == 1 - offset.y)
        {
            dir = dir.rotated(1);
        }
        offset += dir;
    }
    commands.entity(entity).insert(ConstructionSite::new(rec));
}
