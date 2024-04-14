use std::f32::{consts::PI, INFINITY};

use crate::*;
use itertools::Itertools;
use num_traits::FromPrimitive;
use sim::*;

use self::{storage_pile::StonePile, trees::Untree};

#[derive(PartialEq, Copy, Clone)]
struct Params {
    pos: IVec2,
    dir: f32,
}

impl Params {
    fn base_area(self) -> impl Iterator<Item = IVec2> {
        Rect::new_centered(self.pos, IVec2::splat(7))
            .into_iter()
            .filter(move |c| (*c - self.pos).as_vec2().length() < 4.)
    }

    /// Area used to determine suitability for quarrying
    fn probed_mining_area(self) -> impl Iterator<Item = IVec2> {
        let off = (self.dir_vec2() * 8.).as_ivec2();
        Rect::new_centered(self.pos, IVec2::splat(17))
            .into_iter()
            .filter(move |c| (*c - self.pos).as_vec2().length() < 8.)
            .map(move |c| c + off)
    }

    fn dir_vec2(self) -> Vec2 {
        vec2(self.dir.cos(), self.dir.sin())
    }
}

#[derive(Component, PartialEq, Copy, Clone)]
pub struct Quarry {
    params: Params,
}

#[derive(Component)]
pub struct Mason {
    _workplace: Entity,
    _ready_to_work: bool,
}

pub fn plan_quarry(
    mut commands: Commands,
    level: Res<Level>,
    planned: Query<(), (With<Quarry>, With<Planned>)>,
) {
    if !planned.is_empty() {
        return;
    }

    let Some(params) = optimize(
        Params {
            pos: level.area().center(),
            dir: rand_f32(0., 2. * PI),
        },
        |params, temperature| {
            let max_move = (60. * temperature) as i32;
            params.pos += ivec2(
                rand_range(-max_move..=max_move),
                rand_range(-max_move..=max_move),
            );
            params.dir += (rand_f32(-1., 1.)) * 2. * PI * temperature.min(0.5);

            if !level.unblocked(params.base_area()) || !level.unblocked(params.probed_mining_area())
            {
                return INFINITY;
            }
            let mut distance = level.reachability[params.pos] as f32 - 650.;
            // Penalize quarries near city center
            if distance < 0. {
                distance *= -5.
            }
            let avg_start_height = level.height.average(params.base_area());
            let quarried_height =
                level.height.average(params.probed_mining_area()) - avg_start_height;
            // TODO: Pit quarries
            // TODO: check how much stone is available instead of checking height differences
            if quarried_height < 5. {
                return INFINITY;
            }
            let area = Rect::new_centered(params.pos, IVec2::splat(7));
            wateryness(&level, area) * 20. + unevenness(&level, area) * 1.5
                - quarried_height * 1.
                - avg_start_height * 0.15
                + distance / 100.
        },
        200,
    ) else {
        return;
    };

    // TODO: create quarry now
    commands.spawn((
        Pos(level.ground(params.pos).as_vec3()),
        Planned(
            params
                .base_area()
                .chain(params.probed_mining_area())
                .collect(),
        ),
        Quarry { params },
    ));
}

pub fn test_build_quarry(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut untree: Untree,
    new: Query<(Entity, &Quarry), Added<ToBeBuild>>,
) {
    for (entity, quarry) in &new {
        for pos in quarry.params.probed_mining_area() {
            level.blocked[pos] = true;
            let pos = level.ground(pos);
            level(pos, Wool(Red))
        }
        for pos in quarry.params.base_area() {
            let pos = level.ground(pos);
            level(pos, Wool(Black))
        }
        commands
            .entity(entity)
            .remove::<ToBeBuild>()
            .insert(ConstructionSite::new(quarry::make_quarry(
                &mut level,
                &mut untree,
                *quarry,
            )));
    }
}

pub fn assign_worker(
    mut commands: Commands,
    available: Query<(Entity, &Pos), With<Jobless>>,
    new: Query<(Entity, &Pos), (With<Quarry>, Added<Built>)>,
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
            _workplace: workplace,
            _ready_to_work: true,
        });
    }
}

pub fn make_quarry(level: &mut Level, untree: &mut Untree, quarry: Quarry) -> ConsList {
    let rect = Rect::new_centered(quarry.params.pos, ivec2(7, 7));
    let floor = level.height.average(rect.border()).round() as i32;

    let cursor = level.recording_cursor();
    untree.remove_trees(level, rect.grow(1));
    for column in quarry.params.base_area() {
        let mut pos = level.ground(column);
        if pos.z < floor {
            level.fill_at(Some(column), pos.z..floor, PackedMud)
        }
        pos.z = pos.z.min(floor);
        level.height[column] = pos.z;
        level(pos, PackedMud)
    }
    level.fill_at(quarry.params.base_area(), floor + 1..floor + 5, Air);

    level(
        quarry.params.pos.extend(floor) + ivec3(rand_range(-2..=2), rand_range(-2..=2), 1),
        CraftingTable,
    );
    level(
        quarry.params.pos.extend(floor) + ivec3(rand_range(-2..=2), rand_range(-2..=2), 1),
        Stonecutter(HAxis::X),
    );

    let mut to_mine = quarry
        .params
        .probed_mining_area()
        .filter(|c| (*c - quarry.params.pos).length() >= 4.)
        .collect_vec();
    to_mine.sort_by_key(|c| {
        (c.as_vec2() + quarry.params.dir_vec2() * 4. - quarry.params.pos.as_vec2()).length() as i32
    });
    for (i, &column) in to_mine.iter().enumerate() {
        level.fill_at(Some(column), floor + 1..floor + 5, Air);
        let color = Color::from_u8((i as f32 / to_mine.len() as f32 * 15.) as u8).unwrap();
        level(column.extend(floor), Wool(color));
    }

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
