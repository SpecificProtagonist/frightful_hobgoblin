#![allow(clippy::type_complexity)]
use std::f32::consts::PI;
use std::fmt::Display;
use std::fs::{create_dir, read, write};
use std::ops::{DerefMut, RangeInclusive};
use std::sync::atomic::{AtomicU32, Ordering};
use std::{fmt::Write, fs::create_dir_all, path::Path};

use crate::structures::Prefab;
use crate::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;

pub fn sim(level: Level, save_sim: bool) {
    Id::load(&level.path);

    let mut world = World::new();

    let house_pos = ivec3(-77, 117, 94);
    let house = Building {
        stages: vec![
            BuildStage {
                resource: Resource::Stone,
                prefab: "test-house/0",
            },
            BuildStage {
                resource: Resource::Wood,
                prefab: "test-house/1",
            },
            BuildStage {
                resource: Resource::Wood,
                prefab: "test-house/2",
            },
        ],
    };
    let house = world.spawn((Pos(house_pos.as_vec3()), house)).id();

    for column in level.area() {
        let pos = column.extend(level.height(column) + 1);
        if let Block::Log(..) = level[pos] {
            world.spawn((Pos(pos.as_vec3()), Tree::default()));
        }
    }

    let pos = vec3(-50., 90., 200.);
    world.spawn((
        Id::default(),
        Villager {
            carry: Some(Full(Cobble)),
            carry_id: default(),
        },
        Pos(pos),
        PrevPos(pos),
        BuildTask { building: house },
    ));

    let mut sched = Schedule::new();
    sched.add_systems(place);
    sched.add_systems(chop);
    sched.add_systems(walk);
    sched.add_systems(build);

    let mut last = Schedule::new();
    last.add_systems(apply_deferred);
    last.add_systems(tick_replay);

    world.init_resource::<Replay>();
    world.insert_resource(level);
    for _ in 0..10000 {
        sched.run(&mut world);
        last.run(&mut world);
    }

    let level = world.remove_resource::<Level>().unwrap();

    Id::save(&level.path);
    if save_sim {
        world
            .remove_resource::<Replay>()
            .unwrap()
            .write(&level.path);
        level.save_metadata().unwrap();
    } else {
        level.save();
    }
}

#[derive(Clone, Copy)]
enum Resource {
    Stone,
    Wood,
}

type PlaceList = Vec<(IVec3, Block)>;

#[derive(Clone, Copy)]
struct BuildStage {
    resource: Resource,
    prefab: &'static str,
}

#[derive(Component)]
struct Building {
    stages: Vec<BuildStage>,
}

#[derive(Component)]
struct MoveTask(Vec2);

#[derive(Component)]
struct PlaceTask(PlaceList);

#[derive(Component)]
struct BuildTask {
    building: Entity,
}

#[derive(Component)]
struct ChopTask {
    tree: Entity,
}

#[derive(Component, Default)]
struct Tree {
    to_be_chopped: bool,
}

fn build(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut buildings: Query<(&Pos, &mut Building)>,
    mut builders: Query<
        (Entity, &Pos, &mut Villager, &BuildTask),
        (Without<ChopTask>, Without<PlaceTask>),
    >,
    mut trees: Query<(Entity, &Pos, &mut Tree)>,
) {
    for (builder, pos, mut villager, build_task) in &mut builders {
        let (building_pos, mut building) = buildings.get_mut(build_task.building).unwrap();
        if let Some(stage) = building.stages.first().cloned() {
            if let Some(carry) = villager.carry {
                if pos.truncate() != building_pos.truncate() {
                    commands
                        .entity(builder)
                        .insert(MoveTask(building_pos.truncate()));
                } else {
                    let wood_type = match carry {
                        Log(species, _) => species,
                        _ => Oak,
                    };
                    let cursor = level.recording_cursor();
                    Prefab::get(stage.prefab).build(
                        &mut level,
                        building_pos.block(),
                        HDir::YPos,
                        wood_type,
                    );
                    villager.carry = None;
                    commands
                        .entity(builder)
                        .insert(PlaceTask(level.pop_recording(cursor).collect()));
                    building.stages.remove(0);
                }
            } else {
                let (tree, _, mut tree_meta) = trees
                    .iter_mut()
                    .filter(|(_, _, meta)| !meta.to_be_chopped)
                    .min_by_key(|(_, pos, _)| pos.distance_squared(building_pos.0) as i32)
                    .expect("no trees");
                tree_meta.to_be_chopped = true;
                commands.entity(builder).insert(ChopTask { tree });
            }
        } else {
            commands.entity(builder).remove::<BuildTask>();
            commands.entity(build_task.building).remove::<Building>();
        }
    }
}

fn chop(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut lumberjacks: Query<(Entity, &mut Villager, &Pos, &ChopTask), Without<MoveTask>>,
    trees: Query<&Pos>,
) {
    for (jack, mut vill, pos, task) in &mut lumberjacks {
        let target = trees.get(task.tree).unwrap();
        const CHOP_DIST: f32 = 1.5;
        let target_pos =
            target.0 - (target.0 - pos.0).truncate().normalize().extend(0.) * CHOP_DIST * 0.99;
        if pos.0.distance(target_pos) <= CHOP_DIST {
            let cursor = level.recording_cursor();
            vill.carry = Some(level[target.block()]);
            remove_foliage::tree(&mut level, target.block());
            commands.entity(task.tree).despawn();
            commands
                .entity(jack)
                .remove::<ChopTask>()
                .insert(PlaceTask(level.pop_recording(cursor).collect()));
        } else {
            commands
                .entity(jack)
                .insert(MoveTask(target_pos.truncate()));
        }
    }
}

fn place(
    mut commands: Commands,
    mut replay: ResMut<Replay>,
    mut builders: Query<(Entity, &mut PlaceTask), Without<MoveTask>>,
) {
    for (entity, mut build) in &mut builders {
        if let Some((pos, block)) = build.0.pop() {
            replay.block(pos, block);
        } else {
            commands.entity(entity).remove::<PlaceTask>();
        }
    }
}

fn walk(
    mut commands: Commands,
    level: Res<Level>,
    mut query: Query<(Entity, &mut Pos, &MoveTask), With<Villager>>,
) {
    for (entity, mut pos, goal) in &mut query {
        const BLOCKS_PER_TICK: f32 = 0.15;
        let diff = goal.0 - pos.0.truncate();
        if diff.length() < BLOCKS_PER_TICK {
            pos.0 = goal.0.extend(pos.z);
            commands.entity(entity).remove::<MoveTask>();
        } else {
            pos.0 += diff.normalize().extend(0.) * BLOCKS_PER_TICK;
            set_walk_height(&level, &mut pos);
        }
    }
}

fn tick_replay(
    mut level: ResMut<Level>,
    mut replay: ResMut<Replay>,
    new_vills: Query<(&Id, &Pos, &Villager), Added<Villager>>,
    changed_vills: Query<&Villager, Changed<Villager>>,
    mut moved: Query<(&Id, &Pos, &mut PrevPos, Option<&Villager>), Changed<Pos>>,
) {
    let replay = replay.deref_mut();
    for (pos, block) in level.pop_recording(default()) {
        replay.block(pos, block);
    }
    for (id, pos, vill) in &new_vills {
        replay.start_command();
        writeln!(
            replay.commands,
            "summon villager {} {} {} {{{}, NoAI:1, Invulnerable:1}}",
            pos.x,
            pos.z,
            pos.y,
            id.snbt()
        )
        .unwrap();
        replay.start_command();
        writeln!(
            replay.commands,
            "summon armor_stand {} {} {} {{{}, Invulnerable:1, Invisible:1, NoGravity:1}}",
            pos.x,
            pos.z + 0.8,
            pos.y,
            vill.carry_id.snbt(),
        )
        .unwrap();
    }
    for (id, pos, mut prev, vill) in &mut moved {
        let delta = pos.0 - prev.0;
        let facing = pos.0 + delta;
        replay.start_command();
        writeln!(
            replay.commands,
            "tp {} {} {} {} facing {} {} {}",
            id, pos.x, pos.z, pos.y, facing.x, facing.z, facing.y
        )
        .unwrap();
        if let Some(vill) = vill {
            replay.start_command();
            writeln!(
                replay.commands,
                "tp {} {} {} {} {} 0",
                vill.carry_id,
                pos.x,
                pos.z + 0.8,
                pos.y,
                delta.y.atan2(delta.x) / PI * 180.,
            )
            .unwrap();
        }
        prev.0 = pos.0;
    }
    for vill in &changed_vills {
        replay.start_command();
        if let Some(carry) = vill.carry {
            writeln!(
                replay.commands,
                "data modify entity {} ArmorItems[3] set value {}",
                vill.carry_id,
                carry
                    .blockstate(&UNKNOWN_BLOCKS.write().unwrap())
                    .item_snbt()
            )
            .unwrap();
        } else {
            writeln!(
                replay.commands,
                "data modify entity {} ArmorItems[3] set value {{}}",
                vill.carry_id,
            )
            .unwrap();
        }
    }
    replay.tick += 1;
}

fn set_walk_height(level: &Level, pos: &mut Vec3) {
    let size = 0.35;
    let mut height = 0f32;
    for off in [vec2(1., 1.), vec2(-1., 1.), vec2(1., -1.), vec2(-1., -1.)] {
        let mut block_pos = (*pos + off.extend(0.) * size).block();
        while !level[block_pos].solid() {
            block_pos.z -= 1
        }
        while level[block_pos].solid() {
            block_pos.z += 1
        }
        height = height.max(
            block_pos.z as f32
                - match level[block_pos - ivec3(0, 0, 1)] {
                    Slab(_, Flipped(false)) => 0.5,
                    // In theory also do stairs here
                    _ => 0.,
                },
        );
    }
    pos.z = height;
}

#[derive(Component, Deref, DerefMut, PartialEq)]
struct Pos(Vec3);

#[derive(Component, Deref, DerefMut)]
struct PrevPos(Vec3);

#[derive(Component)]
struct Villager {
    carry: Option<Block>,
    carry_id: Id,
}

#[derive(Component)]
struct Id(u32);

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0-0-0-0-{:x}", self.0)
    }
}

impl Id {
    fn snbt(&self) -> String {
        format!("UUID:[I;0,0,0,{}]", self.0)
    }

    // Make sure things work if run multiple times on the same world
    fn load(level: &Path) {
        if let Ok(vec) = read(level.join("mcgen-last_uuid")) {
            NEXT_ID.store(
                u32::from_be_bytes([vec[0], vec[1], vec[2], vec[3]]),
                Ordering::Relaxed,
            );
        }
    }

    fn save(level: &Path) {
        write(
            level.join("mcgen-last-uuid"),
            NEXT_ID.load(Ordering::Relaxed).to_be_bytes(),
        )
        .unwrap();
    }
}

impl Default for Id {
    fn default() -> Self {
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

// In the future, this could have playback speed / reverse playback, controlled via a book
#[derive(Resource)]
struct Replay {
    tick: i32,
    marker: Id,
    commands: String,
    command_chunks: Vec<(RangeInclusive<i32>, String)>,
    chunk_start_tick: i32,
    commands_this_chunk: i32,
    block_cache: HashMap<Block, String>,
    // unknown_blocks: UnknownBlocks,
}

impl Default for Replay {
    fn default() -> Self {
        Self {
            tick: 0,
            marker: default(),
            command_chunks: default(),
            commands: default(),
            block_cache: default(),
            // unknown_blocks: UNKNOWN_BLOCKS.read().unwrap().clone(),
            commands_this_chunk: 0,
            chunk_start_tick: 0,
        }
    }
}

const COMMANDS_PER_CHUNK: i32 = 1000;

impl Replay {
    fn start_command(&mut self) {
        if self.commands_this_chunk == COMMANDS_PER_CHUNK {
            self.command_chunks.push((
                self.chunk_start_tick..=self.tick,
                std::mem::take(&mut self.commands),
            ));
            self.commands_this_chunk = 0;
            self.chunk_start_tick = self.tick;
        }
        self.commands_this_chunk += 1;
        write!(
            self.commands,
            "execute if score {} sim_tick matches {} run ",
            self.marker, self.tick
        )
        .unwrap();
    }

    pub fn block(&mut self, pos: IVec3, block: Block) {
        self.start_command();
        let block_string = self.block_cache.entry(block).or_insert_with(|| {
            block
                .blockstate(&UNKNOWN_BLOCKS.read().unwrap())
                .to_string()
        });
        writeln!(
            self.commands,
            "setblock {} {} {} {block_string}",
            pos.x, pos.z, pos.y
        )
        .unwrap();
    }

    pub fn write(mut self, level: &Path) {
        self.command_chunks
            .push((self.chunk_start_tick..=self.tick, self.commands));

        let pack_path = level.join("datapacks/sim/");
        create_dir_all(&pack_path).unwrap();
        write(
            pack_path.join("pack.mcmeta"),
            r#"{"pack": {"pack_format": 10, "description": ""}}"#,
        )
        .unwrap();

        let sim_path = pack_path.join("data/sim/functions/");
        create_dir_all(&sim_path).unwrap();

        // Could also just modify the world directly, but this is easier
        write(
            sim_path.join("setup.mcfunction"),
            format!(
                "
                summon minecraft:marker ~ ~ ~ {{{}}}
                scoreboard objectives add sim_tick dummy
                scoreboard players set {} sim_tick 0
                ",
                self.marker.snbt(),
                self.marker
            ),
        )
        .unwrap();

        write(
            sim_path.join("check_setup.mcfunction"),
            // TODO: This doesn't actually work. Perhaps the entity isn't yet loaded when this is run?
            format!(
                "execute unless entity {} run function sim:setup",
                self.marker
            ),
        )
        .unwrap();

        let tag_path = pack_path.join("data/minecraft/tags/functions/");
        create_dir_all(&tag_path).unwrap();
        write(
            tag_path.join("load.json"),
            r#"{values:["sim:check_setup"]}"#,
        )
        .unwrap();

        // For now just start automatically
        write(tag_path.join("tick.json"), r#"{values:["sim:tick"]}"#).unwrap();

        let mut tick = String::new();
        let tick_path = sim_path.join("ticks");
        create_dir(&tick_path).unwrap();
        for (ticks, commands) in self.command_chunks {
            write(
                tick_path.join(format!("{}.mcfunction", ticks.start())),
                commands,
            )
            .unwrap();
            writeln!(
                tick,
                "execute if score {} sim_tick matches {}..{} run function sim:ticks/{}",
                self.marker,
                ticks.start(),
                ticks.end(),
                ticks.start()
            )
            .unwrap();
        }
        writeln!(tick, "scoreboard players add {} sim_tick 1", self.marker).unwrap();
        write(sim_path.join("tick.mcfunction"), tick).unwrap();
    }
}
