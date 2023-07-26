use std::fmt::Display;
use std::fs::write;
use std::{fmt::Write, fs::create_dir_all, path::Path};

use crate::*;
use bevy_ecs::prelude::*;

struct Tree;

pub fn sim(level: Level, save_sim: bool) {
    let mut world = World::new();
    world.init_resource::<Replay>();
    world.insert_resource(level);
    // Timer marker
    assert_eq!(world.spawn_empty().id().to_bits(), 0);
    let mut sched = Schedule::new();

    sched.add_systems(tick_replay);

    for t in 0..100 {
        // Test
        world.resource_mut::<Level>()[Vec3(0, 100 + t, 0)] = Block::Glowstone;

        sched.run(&mut world);
    }

    let level = world.remove_resource::<Level>().unwrap();

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
    for (pos, block) in level.take_recording(default()) {
        replay.block(pos, block);
    }
    replay.tick += 1;
}

struct Uuid(Entity);

impl Display for Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let id = self.0.to_bits();
        write!(f, "0-0-0-{:x}-{:x}", id >> 16, id & 0xFFFFFFFFFFFF)
    }
}

impl Uuid {
    fn snbt(&self) -> String {
        format!(
            "UUID:[I;0,0,{},{}]",
            self.0.to_bits() >> 32,
            self.0.to_bits() & 0xFFFFFFFF
        )
    }
}

// In the future, this could have playback speed / reverse playback, controlled via a book
#[derive(Default, Resource)]
struct Replay {
    tick: i32,
    // TODO: split into two levels if this gets too big
    commands: String,
    block_cache: HashMap<Block, String>,
    unknown_blocks: UnknownBlocks,
}

impl Replay {
    fn start_commend(&mut self) {
        write!(
            self.commands,
            "execute if score 0-0-0-0-0 sim_tick matches {} run ",
            self.tick
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
            "
            summon minecraft:marker ~ ~ ~ {UUID:[I;0,0,0,0]}
            scoreboard objectives add sim_tick dummy
            scoreboard players set 0-0-0-0-0 sim_tick 0
            ",
        )
        .unwrap();

        write(
            sim_path.join("check_setup.mcfunction"),
            "execute unless entity 0-0-0-0-0 run function sim:setup",
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

        writeln!(self.commands, "scoreboard players add 0-0-0-0-0 sim_tick 1").unwrap();
        write(sim_path.join("tick.mcfunction"), self.commands).unwrap();
    }
}
