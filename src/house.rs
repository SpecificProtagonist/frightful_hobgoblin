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
                for pos in [
                    pos + IVec2::from(dir),
                    pos + IVec2::from(dir) + IVec2::from(dir.rotated(1)),
                ] {
                    z = z.min(level.height[pos]);
                    if level.blocked[pos] != Free {
                        return None;
                    }
                }
                if (z < base - 3)
                    | !area
                        .shrink(1)
                        .contains(pos + IVec2::from(dir.rotated(1)) - IVec2::from(dir))
                    | (entrance.truncate() == pos + IVec2::from(dir.rotated(-1)))
                    | (entrance.truncate() == pos + 2 * IVec2::from(dir.rotated(1)))
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
        .flat_map(|&c| [c, c + IVec2::from(area.outside_face(c).rotated(1))])
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
                | !roof.covers(pos - IVec3::from(dir))
                | !roof.covers(pos + IVec3::from(dir.rotated(1)))
                | !roof.covers(pos + IVec3::from(dir.rotated(-1)))
            {
                continue;
            }
            for off in [
                IVec3::from(dir.rotated(1)),
                IVec3::ZERO,
                IVec3::from(dir.rotated(-1)),
            ] {
                if no_walls.contains(&(pos + off))
                    | chimney_columns.contains(&(pos + off).truncate())
                {
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

    level.pop_recording_into(&mut rec, cursor);
    let roof_rec = build_roof(level, roof.area, roof.z, &roof.shape, roof::palette());
    rec.extend(roof_rec);

    if let Some(chimney) = chimney {
        let dir = area.outside_face(chimney);
        level.blocked[chimney] = Blocked;
        level.blocked[chimney + IVec2::from(dir.rotated(1))] = Blocked;
        // Chimney
        for z in floors[0].z - 4.. {
            level((chimney + IVec2::from(dir)).extend(z), Full(Cobble));
            level(
                (chimney + IVec2::from(dir) + IVec2::from(dir.rotated(1))).extend(z),
                Full(Cobble),
            );
            if !roof.covers(chimney.extend(z - 1))
                & !roof.covers((chimney + IVec2::from(dir.rotated(1))).extend(z - 1))
            {
                level((chimney + IVec2::from(dir)).extend(z + 1), Fence(Andesite));
                level(
                    (chimney + IVec2::from(dir) + IVec2::from(dir.rotated(1))).extend(z + 1),
                    Fence(Andesite),
                );
                break;
            }
        }
        // Hearth
        'floor: for floor in floors {
            for i in -1..=2 {
                if !roof.covers((chimney + i * IVec2::from(dir.rotated(1))).extend(floor.z)) {
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
    level(entrance + IVec2::from(door_dir).extend(0), Air);
    level(entrance + IVec2::from(door_dir).extend(1), Air);
    if level(entrance + 2 * IVec2::from(door_dir).extend(0)).solid() {
        level(entrance + IVec2::from(door_dir).extend(2), Air);
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
