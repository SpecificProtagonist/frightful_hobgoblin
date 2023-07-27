use std::fmt::Display;
use std::fs::{read, write};
use std::sync::atomic::{AtomicU32, Ordering};
use std::{fmt::Write, fs::create_dir_all, path::Path};

use crate::*;
use bevy_ecs::prelude::*;

struct Tree;

pub fn sim(level: Level, save_sim: bool) {
    Uuid::load(&level.path);
    let mut world = World::new();
    world.init_resource::<Replay>();
    world.insert_resource(level);
    let mut sched = Schedule::new();

    sched.add_systems(tick_replay);

    for t in 0..100 {
        // Test
        world.resource_mut::<Level>()[Vec3(0, 100 + t, 0)] = Block::Glowstone;

        sched.run(&mut world);
    }

    let level = world.remove_resource::<Level>().unwrap();

    Uuid::save(&level.path);
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

fn tick_replay(mut level: ResMut<Level>, mut replay: ResMut<Replay>) {
    for (pos, block) in level.pop_recording(default()) {
        replay.block(pos, block);
    }
    replay.tick += 1;
}

struct Uuid(u32);

impl Display for Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0-0-0-0-{:x}", self.0)
    }
}

impl Uuid {
    fn snbt(&self) -> String {
        format!("UUID:[I;0,0,0,{}]", self.0)
    }

    fn new() -> Self {
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
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

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

// In the future, this could have playback speed / reverse playback, controlled via a book
#[derive(Resource)]
struct Replay {
    tick: i32,
    marker: Uuid,
    // TODO: split into two levels if this gets too big
    commands: String,
    block_cache: HashMap<Block, String>,
    unknown_blocks: UnknownBlocks,
}

impl Default for Replay {
    fn default() -> Self {
        Self {
            tick: 0,
            marker: Uuid::new(),
            commands: default(),
            block_cache: default(),
            unknown_blocks: UNKNOWN_BLOCKS.read().unwrap().clone(),
        }
    }
}

impl Replay {
    fn start_commend(&mut self) {
        write!(
            self.commands,
            "execute if score {} sim_tick matches {} run ",
            self.marker, self.tick
        )
        .unwrap();
    }

    pub fn block(&mut self, pos: Vec3, block: Block) {
        self.start_commend();
        let cache = &mut self.block_cache;
        let unknown = &self.unknown_blocks;
        let block_string = cache
            .entry(block)
            .or_insert_with(|| block.blockstate(unknown).to_string());
        writeln!(self.commands, "setblock {pos} {block_string}").unwrap();
    }

    pub fn write(mut self, level: &Path) {
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

        writeln!(
            self.commands,
            "scoreboard players add {} sim_tick 1",
            self.marker
        )
        .unwrap();
        write(sim_path.join("tick.mcfunction"), self.commands).unwrap();
    }
}
