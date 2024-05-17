use std::convert::identity;

use crate::*;
use itertools::Itertools;
use roof::build_roof;

use self::{
    construction::RemoveWhenBlocked,
    desire_lines::{add_desire_line, DesireLines},
    pathfind::pathfind_street,
    roof::{roof_shape, Shape},
    sim::{logistics::MoveTask, ConsItem, ConsList},
};
use Biome::*;

struct Floor {
    z: i32,
    material: WallMaterial,
    /// Applies to Wattle & Planks (move into WallMaterial?)
    wood_framing: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WallMaterial {
    Cobble,
    Wattle,
    Logs,
    Planks,
}

struct Roof {
    z: i32,
    area: Rect,
    shape: Shape,
}

impl Roof {
    fn covers(&self, pos: IVec3) -> bool {
        (self.shape)(pos.truncate().as_vec2()) - 0.5 >= pos.z as f32
    }
}

pub fn house(
    commands: &mut Commands,
    level: &mut Level,
    dl: &mut DesireLines,
    untree: &mut Untree,
    area: Rect,
) -> ConsList {
    let inner = area.shrink(1);

    (level.blocked)(area, Free);
    // TODO: On sides that are wider than ~9 blocks, don't allow starting next to corner
    let path = pathfind_street(level, area);
    for node in &path.path {
        for (x_off, y_off) in (-1..=1).cartesian_product(-1..=1) {
            level.blocked[node.pos.truncate() + ivec2(x_off, y_off)] = Street;
        }
        for _ in 0..30 {
            add_desire_line(level, dl, node.pos - IVec3::Z);
        }
    }
    (level.blocked)(area, Blocked);
    let side_columns = (inner.min.x..inner.max.x)
        .flat_map(|x| [ivec2(x, area.min.y - 1), ivec2(x, area.max.y + 1)])
        .chain(
            (inner.min.y..inner.max.y)
                .flat_map(|y| [ivec2(area.min.x - 1, y), ivec2(area.max.x + 1, y)]),
        )
        .map(|c| level.height[c]);
    let mut sides_min = side_columns.clone().min().unwrap();
    while side_columns.clone().filter(|&h| h <= sides_min).count() < 3 {
        sides_min += 1
    }

    let entrance = path.path[0].pos.truncate().extend(path.path[1].pos.z);

    let tall_first_floor = entrance.z > sides_min + 5;
    let base = if entrance.z > sides_min + 3 {
        entrance.z - 4
    } else {
        entrance.z - 1
    };

    let log_weight_lower = match center_biome() {
        Desert => 0.,
        Forest | BirchForest | Jungles | Taiga => 1.3,
        DarkForest => 2.,
        _ => 0.4,
    };
    let log_weight_upper = match center_biome() {
        Savanna => 0.,
        _ => log_weight_lower,
    };
    let wood_framing = !matches!(center_biome(), Desert | Savanna);
    let mut floors = vec![
        Floor {
            z: base + if tall_first_floor { 0 } else { 1 },
            material: rand_weighted(&[
                (1.0, WallMaterial::Cobble),
                (log_weight_lower, WallMaterial::Logs),
            ]),
            wood_framing: false,
        },
        Floor {
            z: base + 4,
            material: rand_weighted(&[
                (1.0, WallMaterial::Wattle),
                (log_weight_upper, WallMaterial::Logs),
            ]),
            wood_framing,
        },
    ];

    // Roof determined now so we know how high the walls have to be
    let roof_z = floors.last().unwrap().z + 2;
    let roof_area = area.grow(1);
    let roof_shape = roof_shape(roof_z, roof_area);
    let roof = Roof {
        z: roof_z,
        area: roof_area,
        shape: roof_shape,
    };

    let upper_floors_keep_material = !roof.covers(area.center().extend(base + 11));

    // This sometimes breaks with low roofs?!?
    while Rect::new_centered(area.center(), IVec2::splat(4))
        .border()
        .all(|c| (roof.shape)(c.as_vec2()) > floors.last().unwrap().z as f32 + 4.0)
    {
        floors.push(Floor {
            z: floors.last().unwrap().z + rand_range(3..=4),
            material: if upper_floors_keep_material {
                floors.last().unwrap().material
            } else {
                WallMaterial::Wattle
            },
            wood_framing,
        })
    }

    // Chimney
    let chimney = if match center_biome() {
        Desert | Savanna => 0.1,
        Snowy => 1.,
        _ => 0.8,
    } > rand()
    {
        let possible = area
            .border_no_corners()
            .filter_map(|pos| {
                let dir = area.outside_face(pos);
                let mut z = i32::MAX;
                for pos in [pos + dir.offset(1, 0), pos + dir.offset(1, 1)] {
                    z = z.min(level.height[pos]);
                    if level.blocked[pos] != Free {
                        return None;
                    }
                }
                if (z < base - 3)
                    | !area.shrink(1).contains(pos + dir.offset(-1, 1))
                    | (entrance.truncate() == pos + dir.offset(0, -1))
                    | (entrance.truncate() == pos + dir.offset(0, 2))
                {
                    return None;
                }
                Some((
                    1. / ((base - z) as f32 / 2. + area.center_vec2().distance(pos.as_vec2())),
                    pos,
                ))
            })
            .collect_vec();
        try_rand_weighted(&possible)
    } else {
        None
    };

    building(
        commands, level, untree, area, entrance, &floors, roof, chimney,
    )
}

pub fn shack(
    commands: &mut Commands,
    level: &mut Level,
    untree: &mut Untree,
    area: Rect,
) -> ConsList {
    let mut entrance = ivec3(0, 0, i32::MAX);
    for column in area.border_no_corners() {
        let pos = level.ground(column + IVec2::from(area.outside_face(column))) + IVec3::Z;
        if pos.z < entrance.z {
            entrance = column.extend(pos.z)
        }
    }
    let floors = [Floor {
        z: entrance.z,
        material: rand_weighted(&[
            (1., WallMaterial::Cobble),
            (0.5, WallMaterial::Planks),
            (0.3, WallMaterial::Logs),
        ]),
        wood_framing: true,
    }];

    let roof_z = floors.last().unwrap().z + 2;
    let roof_area = area.grow(1);
    let roof_shape = roof_shape(roof_z, roof_area);
    let roof = Roof {
        z: roof_z,
        area: roof_area,
        shape: roof_shape,
    };

    building(commands, level, untree, area, entrance, &floors, roof, None)
}

fn building(
    commands: &mut Commands,
    level: &mut Level,
    untree: &mut Untree,
    area: Rect,
    entrance: IVec3,
    floors: &[Floor],
    roof: Roof,
    chimney: Option<IVec2>,
) -> ConsList {
    let inner = area.shrink(1);
    let mut no_walls = vec![entrance, entrance + IVec3::Z];

    let chimney_columns = chimney
        .iter()
        .flat_map(|&c| [c, c + area.outside_face(c).offset(0, 1)])
        .collect_vec();

    let biome = level.biome[area.center()];
    let species = biome.random_tree_species();
    let floorbords = biome.random_tree_species();
    let log_stripped = if match species {
        Birch => 1.,
        DarkOak => 0.6,
        Spruce => 0.2,
        _ => 0.,
    } > rand()
    {
        LogType::Stripped
    } else {
        LogType::Normal
    };

    let mut rec = foundation(level, untree, area, floors[0].z - 1);

    let cursor = level.recording_cursor();

    let paint = rand_weighted(&[
        (8., MushroomStem),
        (1., Terracotta(Some(White))),
        (1., Terracotta(Some(Red))),
        (1., Terracotta(Some(Orange))),
        (1., Terracotta(Some(Lime))),
        (1., Terracotta(Some(Green))),
        (1., Terracotta(Some(LightBlue))),
        (1., Terracotta(Some(Magenta))),
        (1., Terracotta(Some(Pink))),
    ]);

    let wall_log_axis = |pos: IVec3| {
        if area.corners().contains(&pos.truncate()) {
            [Axis::X, Axis::Y][(pos.z % 2) as usize]
        } else if [HDir::XNeg, HDir::XPos].contains(&area.outside_face(pos.truncate())) {
            Axis::Y
        } else {
            Axis::X
        }
    };

    let mut windows = Vec::new();
    for (i, floor) in floors.iter().enumerate() {
        // Determine windows
        let mut prev_window = rand_range(0..3);
        'windows: for column in area.border_no_corners() {
            let pos = column.extend(floor.z + 1);
            let dir = area.outside_face(column);
            if level(pos + IVec3::from(dir)).solid()
                | level(pos + IVec2::from(dir).extend(-1)).solid() & (0.7 > rand())
                | !roof.covers(pos)
                | !roof.covers(pos + dir.offset(-1, 0).extend(0))
                | !roof.covers(pos + dir.offset(0, 1).extend(0))
                | !roof.covers(pos + dir.offset(0, -1).extend(0))
            {
                continue;
            }
            for i in -1..=1 {
                let check = pos + dir.offset(0, i).extend(0);
                if no_walls.contains(&check) | chimney_columns.contains(&check.truncate()) {
                    continue 'windows;
                }
            }
            if prev_window < 2 {
                prev_window += 1;
            } else {
                prev_window = 0;
                windows.push((pos, dir));
                no_walls.push(pos);
            }
        }

        let ceiling = floors.get(i + 1).map(|f| f.z - 1);

        // Determine wall blocks
        let mut wall = Vec::new();
        for column in area.border() {
            for z in floor.z.. {
                if z > ceiling
                    .unwrap_or(i32::MAX)
                    .min(((roof.shape)(column.as_vec2()) - 0.5) as i32)
                {
                    break;
                }
                wall.push(column.extend(z));
            }
        }
        wall.retain(|p| !windows.iter().any(|(w, _)| w == p));
        wall.sort_by_key(|p| p.z);

        // Fill wall
        match floor.material {
            WallMaterial::Cobble => level.fill(wall, Full(Cobble)),
            WallMaterial::Wattle | WallMaterial::Planks => {
                let mut wall_fill = Vec::new();
                for pos in &wall {
                    if floor.wood_framing & area.corners().contains(&pos.truncate()) {
                        // Wood frame
                        level(*pos, Log(species, log_stripped, Axis::Z));
                    } else {
                        wall_fill.push(*pos);
                    }
                }
                if let WallMaterial::Planks = floor.material {
                    let block = Full(Wood(biome.random_tree_species()));
                    level.fill(&wall_fill, block);
                } else {
                    // Wattle
                    level.fill(&wall_fill, MangroveRoots);
                    level.pop_recording_into(&mut rec, cursor);
                    // Daub
                    level.fill(&wall_fill, MuddyMangroveRoots);
                    level.pop_recording_into(&mut rec, cursor);
                    // Paint/Whitewash
                    level.fill(&wall_fill, paint);
                    level.pop_recording_into(&mut rec, cursor);
                }
            }
            WallMaterial::Logs => {
                for pos in &wall {
                    level(*pos, Log(species, LogType::Stripped, wall_log_axis(*pos)));
                }
            }
        }

        if let Some(below) = floors.get(i.wrapping_sub(1)) {
            if floor.wood_framing & (below.material != WallMaterial::Logs) {
                for column in area.border() {
                    let pos = column.extend(floor.z - 1);
                    let axis = wall_log_axis(column.extend(floor.z - 1));
                    if roof.covers(pos) {
                        level(pos, Log(species, log_stripped, axis))
                    }
                }
            }
        }

        // Ceiling
        if let Some(ceiling) = ceiling {
            for column in inner {
                let pos = column.extend(ceiling);
                if roof.covers(pos) {
                    level(pos, Slab(Wood(floorbords), Top));
                }
            }
        }
    }

    // Roof
    level.pop_recording_into(&mut rec, cursor);
    let roof_rec = build_roof(level, roof.area, roof.z, &roof.shape, roof::palette());
    let mut roof_underside = HashMap::default();
    for item in &roof_rec {
        if let ConsItem::Set(SetBlock { pos, .. }) = item {
            // Roof already sorted by z
            roof_underside.entry(pos.truncate()).or_insert(pos.z);
        }
    }
    rec.extend(roof_rec);

    // Chimney
    if let Some(chimney) = chimney {
        let dir = area.outside_face(chimney);
        level.blocked[chimney + dir.offset(1, 0)] = Blocked;
        level.blocked[chimney + dir.offset(1, 1)] = Blocked;
        for z in floors[0].z - 4.. {
            level((chimney + dir.offset(1, 0)).extend(z), Full(Cobble));
            level((chimney + dir.offset(1, 1)).extend(z), Full(Cobble));
            if !roof.covers(chimney.extend(z - 1))
                & !roof.covers((chimney + dir.offset(0, 1)).extend(z - 1))
            {
                level((chimney + dir.offset(1, 0)).extend(z + 1), Fence(Andesite));
                level((chimney + dir.offset(1, 1)).extend(z + 1), Fence(Andesite));
                break;
            }
        }
        // Hearth
        'floor: for floor in floors {
            for i in -1..=2 {
                if roof_underside[&(chimney + dir.offset(0, i))] <= floor.z + 1 {
                    break 'floor;
                }
            }
            prefab(&format!("hearth_{}", rand_range(0..=1))).build(
                level,
                chimney.extend(floor.z),
                dir,
                false,
                false,
                Oak,
                identity,
            );
        }
    }

    // Some movement
    for i in 0..rec.len() {
        if 0.03 > rand() {
            rec.insert(
                i,
                ConsItem::Goto(MoveTask::new(ivec3(
                    rand_range(inner.min.x..=inner.max.x),
                    rand_range(inner.min.y..=inner.max.y),
                    entrance.z,
                ))),
            );
        }
    }

    // Stairs
    enum StairSupportStyle {
        Stair,
        Fence,
    }
    let stair_support_style = [StairSupportStyle::Stair, StairSupportStyle::Fence].choose();
    let stair_material = Wood(species);
    let stair_rot_dir = if 0.5 > rand() { 1 } else { -1 };
    for upper_floor_index in 1..floors.len() {
        let lower_z = floors[upper_floor_index - 1].z - 1;
        let upper_z = floors[upper_floor_index].z - 1;
        let mut choices = Vec::new();
        'outer: for column in inner.border_no_corners() {
            if matches!(level(column.extend(lower_z)), Air | Stair(..)) {
                continue;
            }
            let stair_cursor = level.recording_cursor();
            let mut column = column;
            let mut dir = inner.outside_face(column).rotated(stair_rot_dir);
            let mut z = lower_z;
            let mut prev = (column + dir.offset(-1, 0)).extend(z);
            if !inner.contains(prev.truncate()) {
                prev += dir.offset(1, -stair_rot_dir).extend(0)
            }
            while z < upper_z {
                if !inner.contains(column + IVec2::from(dir)) {
                    dir = dir.rotated(stair_rot_dir);
                    level(column.extend(z), |b| b | Full(stair_material));
                } else {
                    z += 1;
                    level(column.extend(z), Stair(stair_material, dir, Bottom));
                    if z - 1 > lower_z {
                        level(column.extend(z - 1), |b| {
                            b | match stair_support_style {
                                StairSupportStyle::Stair => {
                                    Stair(stair_material, dir.rotated(2), Top)
                                }
                                StairSupportStyle::Fence => Fence(stair_material),
                            }
                        });
                    }
                }
                level.fill_at(Some(column), z + 1..z + 3, Air);
                if prev.z < z {
                    level(prev + 3 * IVec3::Z, Air)
                }
                if (roof_underside[&column] <= z + 2)
                    | ((prev.z != z) & (roof_underside[&prev.truncate()] <= prev.z + 3))
                    | (0..2)
                        .map(|z_off| (column + dir.offset(0, -stair_rot_dir)).extend(z + z_off))
                        .contains(&entrance)
                    | chimney_columns.contains(&(column + dir.offset(0, -stair_rot_dir)))
                {
                    level.undo_recording(stair_cursor);
                    continue 'outer;
                }
                prev = column.extend(z);
                column += IVec2::from(dir);
            }
            choices.push(level.undo_recording(stair_cursor));
        }
        if let Some(stair_rec) = choices.try_choose_mut() {
            level.apply_recording(&*stair_rec);
            rec.extend(stair_rec.drain(..).map(ConsItem::Set));
        } else {
            // TODO ladder
        }
        // TODO: Block windows
    }

    // Keep windows/doors free
    rec.retain(|s| {
        if let ConsItem::Set(SetBlock { pos, block, .. }) = s {
            (*block == Air) | !no_walls.contains(pos)
        } else {
            true
        }
    });

    // Door
    let door_dir = area.outside_face(entrance.truncate());
    let door_type = biome.random_tree_species();
    level(entrance, Door(door_type, door_dir, DoorMeta::empty()));
    level(
        entrance + IVec3::Z,
        Door(door_type, door_dir, DoorMeta::TOP),
    );
    level(entrance + door_dir.offset(1, 0).extend(0), Air);
    level(entrance + door_dir.offset(1, 0).extend(1), Air);
    if level(entrance + door_dir.offset(2, 0).extend(0)).solid() {
        level(entrance + door_dir.offset(1, 0).extend(2), Air);
    }

    // Windows
    for (pos, dir) in windows {
        let glass = GlassPane(rand_weighted(&[
            (1., None),
            (0.1, Some(LightGray)),
            (0.1, Some(Brown)),
        ]));
        level(pos, glass);
        let mut shutter_pos = pos + IVec3::from(dir) + IVec3::from(dir.rotated(1));
        if level(shutter_pos).solid()
            | area
                .corners()
                .contains(&(pos.truncate() + IVec2::from(dir.rotated(1))))
        {
            shutter_pos += IVec3::from(dir.rotated(-1)) * 2
        }
        level(shutter_pos, |b| b | Trapdoor(species, dir, DoorMeta::OPEN));

        commands.spawn(RemoveWhenBlocked {
            check_area: vec![pos.truncate() + IVec2::from(dir)],
            restore: vec![SetBlock {
                pos,
                block: level(pos - IVec3::Z),
                previous: glass,
                nbt: None,
            }],
        });
    }

    level.pop_recording_into(&mut rec, cursor);

    // Fill smokers
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

fn foundation(level: &mut Level, untree: &mut Untree, area: Rect, floor: i32) -> ConsList {
    let cursor = level.recording_cursor();
    untree.remove_trees(level, area.grow(1));

    for z in (floor + 1..floor + 10).rev() {
        level.fill_at(area, z, Air)
    }
    for col in area {
        for z in (level.height[col] + 1..=floor).rev() {
            level(col, z, Air)
        }
    }
    let mut rec: ConsList = level.pop_recording(cursor).map(ConsItem::Set).collect();
    let cursor = level.recording_cursor();
    for col in area.border() {
        // TODO: if ground is too far down, try to make supports against the nearest wall instead
        let mut pos = col.extend(floor);
        while level(pos).soil() | !level(pos).soil() {
            level(pos, Full(Cobble));
            if NEIGHBORS_2D.iter().all(|dir| level(pos.add(*dir)).solid()) {
                break;
            }
            pos -= IVec3::Z;
        }
    }
    for col in area.shrink(1) {
        for z in (level.height[col] - 1).min(floor)..=floor {
            let pos = col.extend(z);
            if (!level(pos).solid()) | (level(pos).soil()) {
                level(pos, PackedMud)
            }
        }
    }
    rec.extend(level.pop_recording(cursor).map(ConsItem::Set));

    rec
}
