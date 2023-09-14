use crate::sim::*;
use crate::*;
use bevy_ecs::prelude::*;

use std::f32::consts::PI;
use std::fmt::{Display, Write};
use std::fs::{create_dir, create_dir_all, read, write};
use std::ops::{DerefMut, RangeInclusive};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Component)]
pub struct Id(u32);

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0-0-0-0-{:x}", self.0)
    }
}

// Maybe just use normal random uuids? Small numbers make it easier to debug though.
impl Id {
    fn snbt(&self) -> String {
        format!("UUID:[I;0,0,0,{}]", self.0)
    }

    // Make sure things work if run multiple times on the same world
    pub fn load(level: &Path) {
        if let Ok(vec) = read(level.join("mcgen-last_uuid")) {
            NEXT_ID.store(
                u32::from_be_bytes([vec[0], vec[1], vec[2], vec[3]]),
                Ordering::Relaxed,
            );
        }
    }

    pub fn save(level: &Path) {
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
pub struct Replay {
    tick: i32,
    commands: String,
    command_chunks: Vec<(RangeInclusive<i32>, String)>,
    chunk_start_tick: i32,
    commands_this_chunk: i32,
    block_cache: HashMap<Block, String>,
}

impl Default for Replay {
    fn default() -> Self {
        Self {
            tick: 20,
            command_chunks: default(),
            commands: default(),
            block_cache: default(),
            commands_this_chunk: 0,
            chunk_start_tick: 0,
        }
    }
}

const COMMANDS_PER_CHUNK: i32 = 1000;

impl Replay {
    pub fn dbg(&mut self, msg: &str) {
        println!("{}", msg);
        self.start_command();
        writeln!(self.commands, "say {msg}").unwrap();
    }

    pub fn command(&mut self, msg: &str) {
        self.start_command();
        writeln!(self.commands, "{msg}").unwrap();
    }

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
            "execute if score SIM sim_tick matches {} run ",
            self.tick
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

    // TODO: do this asynchronically in the background as commands come in
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
            "
            scoreboard players set SIM sim_tick 0
            gamerule randomTickSpeed 0
            gamerule doMobSpawning false
            gamerule mobGriefing false
            gamerule doFireTick false
            ",
        )
        .unwrap();

        write(
            sim_path.join("check_setup.mcfunction"),
            "
            scoreboard objectives add sim_tick dummy
            execute unless score SIM sim_tick matches 0.. run function sim:setup
            ",
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
        let chunk_path = sim_path.join("chunked");
        create_dir(&chunk_path).unwrap();
        for (i, (ticks, commands)) in self.command_chunks.iter().enumerate() {
            write(chunk_path.join(format!("{i}.mcfunction")), commands).unwrap();
            writeln!(
                tick,
                "execute if score SIM sim_tick matches {}..{} run function sim:chunked/{i}",
                ticks.start(),
                ticks.end(),
            )
            .unwrap();
        }
        writeln!(tick, "scoreboard players add SIM sim_tick 1").unwrap();
        write(sim_path.join("tick.mcfunction"), tick).unwrap();
    }
}

pub fn tick_replay(
    mut level: ResMut<Level>,
    mut replay: ResMut<Replay>,
    new_vills: Query<(&Id, &Pos, &Villager), Added<Villager>>,
    changed_vills: Query<&Villager, Changed<Villager>>,
    mut moved: Query<(&Id, &Pos, &mut PrevPos, Option<&Villager>), Changed<Pos>>,
) {
    let replay = replay.deref_mut();
    for set in level.pop_recording(default()) {
        replay.block(set.pos, set.block);
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
            // TODO: Use block display
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
            "tp {} {:.2} {:.2} {:.2} facing {:.2} {:.2} {:.2}",
            id,
            pos.x + 0.5,
            pos.z,
            pos.y + 0.5,
            facing.x + 0.5,
            facing.z,
            facing.y + 0.5
        )
        .unwrap();
        if let Some(vill) = vill {
            replay.start_command();
            writeln!(
                replay.commands,
                "tp {} {:.2} {:.2} {:.2} {:.0} 0",
                vill.carry_id,
                pos.x + 0.5,
                pos.z + 0.8,
                pos.y + 0.5,
                delta.y.atan2(delta.x) / PI * 180.,
            )
            .unwrap();
        }
        prev.0 = pos.0;
    }
    for vill in &changed_vills {
        replay.start_command();
        if let Some(stack) = vill.carry {
            writeln!(
                replay.commands,
                "data modify entity {} ArmorItems[3] set value {}",
                vill.carry_id,
                stack
                    .kind
                    .display_as_block()
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
