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

pub fn sim(mut level: Level, save_sim: bool) {
    Id::load(&level.path);

    let house_pos = ivec3(-77, 117, 94);
    let house = Building {
        stages: vec![
            BuildStage {
                resource: Resource::Stone,
                place: {
                    let cursor = level.recording_cursor();
                    Prefab::get("test-house/0").build(&mut level, house_pos, HDir::YPos);
                    level.pop_recording(cursor).collect()
                },
            },
            BuildStage {
                resource: Resource::Wood,
                place: {
                    let cursor = level.recording_cursor();
                    Prefab::get("test-house/1").build(&mut level, house_pos, HDir::YPos);
                    level.pop_recording(cursor).collect()
                },
            },
            BuildStage {
                resource: Resource::Wood,
                place: {
                    let cursor = level.recording_cursor();
                    Prefab::get("test-house/2").build(&mut level, house_pos, HDir::YPos);
                    level.pop_recording(cursor).collect()
                },
            },
        ],
    };

    // for i in 0..4 {
    //     let dir = [HDir::XPos, HDir::YPos, HDir::XNeg, HDir::YNeg][i as usize];
    //     Prefab::get("test-house/2").build(&mut level, ivec3(20 * i, 0, 100), dir);
    // }

    let mut world = World::new();
    world.init_resource::<Replay>();
    world.insert_resource(level);

    world.spawn((Pos(house_pos.as_vec3()), house));

    let pos = vec3(-50., 90., 200.);
    world.spawn((
        Id::default(),
        Villager {
            carry: Some(Bedrock),
            carry_id: default(),
        },
        Pos(pos),
        PrevPos(pos),
    ));

    let mut sched = Schedule::new();
    sched.add_systems(test_walk);

    let mut last = Schedule::new();
    last.add_systems(apply_deferred);
    last.add_systems(tick_replay);

    for _ in 0..1000 {
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

enum Resource {
    Stone,
    Wood,
}

struct BuildStage {
    resource: Resource,
    place: Vec<(IVec3, Block)>,
}

#[derive(Component)]
struct Building {
    stages: Vec<BuildStage>,
}

fn test_walk(mut rot: Local<f32>, level: Res<Level>, mut query: Query<&mut Pos, With<Villager>>) {
    *rot += 0.01;
    for mut pos in &mut query {
        pos.0 += vec3(0.15 * rot.sin(), 0.15 * rot.cos(), 0.0);
        set_walk_height(&level, &mut pos);
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
                carry.blockstate(&replay.unknown_blocks).item_snbt()
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
        let mut block_pos = (*pos + off.extend(0.) * size).floor().as_ivec3();
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

#[derive(Component, Deref, DerefMut)]
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
    unknown_blocks: UnknownBlocks,
}

impl Default for Replay {
    fn default() -> Self {
        Self {
            tick: 0,
            marker: default(),
            command_chunks: default(),
            commands: default(),
            block_cache: default(),
            unknown_blocks: UNKNOWN_BLOCKS.read().unwrap().clone(),
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
        let cache = &mut self.block_cache;
        let unknown = &self.unknown_blocks;
        let block_string = cache
            .entry(block)
            .or_insert_with(|| block.blockstate(unknown).to_string());
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
