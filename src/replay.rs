use crate::sim::*;
use crate::*;
use bevy_ecs::prelude::*;
use nbt::{CompoundTag, Tag};

use std::collections::VecDeque;
use std::f32::consts::PI;
use std::fmt::{Display, Write};
use std::fs::{create_dir_all, read, write};
use std::ops::DerefMut;
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
    block_cache: HashMap<Block, String>,
    commands_this_tick: Vec<Tag>,
    commands: VecDeque<Tag>,
    // Only out of curiosity
    total_commands: u64,
}

impl Default for Replay {
    fn default() -> Self {
        let mut replay = Self {
            block_cache: default(),
            commands_this_tick: default(),
            commands: default(),
            total_commands: 0,
        };
        // Wait for the player to load in
        for _ in 0..10 {
            replay.tick();
        }
        replay
    }
}

impl Replay {
    pub fn dbg(&mut self, msg: &str) {
        self.command(format!("say {msg}"));
    }

    pub fn block(&mut self, pos: IVec3, block: Block) {
        let block_string = self.block_cache.entry(block).or_insert_with(|| {
            block
                .blockstate(&UNKNOWN_BLOCKS.read().unwrap())
                .to_string()
        });
        let command = format!("setblock {} {} {} {block_string}", pos.x, pos.z, pos.y);
        self.command(command);
    }

    pub fn command(&mut self, msg: String) {
        self.commands_this_tick.push({
            let mut nbt = CompoundTag::new();
            nbt.insert("cmd", msg);
            nbt.into()
        });
        self.total_commands += 1;
    }

    fn tick(&mut self) {
        let commands = std::mem::take(&mut self.commands_this_tick);
        self.commands.push_front(commands.into());
    }

    // TODO: do this asynchronically in the background as commands come in
    pub fn write(mut self, level: &Path) {
        // println!("Total commands: {}", self.total_commands);
        if self.total_commands > i32::MAX as u64 {
            eprintln!(
                "Too many commands: {} (reemploy chunking)",
                self.total_commands
            );
        }
        // Flush final commands
        let commands = std::mem::take(&mut self.commands_this_tick);
        self.commands.push_front(commands.into());
        // Write commands to nbt
        let data_path = level.join("data/");
        create_dir_all(&data_path).unwrap();
        let mut nbt = CompoundTag::new();
        nbt.insert("DataVersion", DATA_VERSION);
        nbt.insert("data", {
            let mut nbt = CompoundTag::new();
            nbt.insert("contents", {
                let mut nbt = CompoundTag::new();
                nbt.insert("data", {
                    let mut nbt = CompoundTag::new();
                    nbt.insert(
                        "commands",
                        nbt::Tag::List(self.commands.drain(..).collect()),
                    );
                    nbt
                });
                nbt
            });
            nbt
        });
        nbt::encode::write_gzip_compound_tag(
            &mut std::fs::File::create(data_path.join("command_storage_sim.dat")).unwrap(),
            &nbt,
        )
        .unwrap();

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

        write(sim_path.join("eval.mcfunction"), "$$(cmd)").unwrap();
        write(
            sim_path.join("run_current_commands.mcfunction"),
            "
            function sim:eval with storage sim:data commands[-1][-1]
            data remove storage sim:data commands[-1][-1]
            execute if data storage sim:data commands[-1][0] run function sim:run_current_commands
            ",
        )
        .unwrap();
        let mut tick = String::new();
        // TODO
        writeln!(
            tick,
            "
            function sim:run_current_commands
            data remove storage sim:data commands[-1]
            scoreboard players add SIM sim_tick 1
            "
        )
        .unwrap();
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
        replay.command(format!(
            "summon villager {} {} {} {{{}, NoAI:1, Invulnerable:1}}",
            pos.x,
            pos.z,
            pos.y,
            id.snbt()
        ));
        replay.command(format!(
            // TODO: Use block display
            "summon armor_stand {} {} {} {{{}, Invulnerable:1, Invisible:1, NoGravity:1}}",
            pos.x,
            pos.z + 0.8,
            pos.y,
            vill.carry_id.snbt(),
        ));
    }
    for (id, pos, mut prev, vill) in &mut moved {
        let delta = pos.0 - prev.0;
        let facing = pos.0 + delta;
        replay.command(format!(
            "tp {} {:.2} {:.2} {:.2} facing {:.2} {:.2} {:.2}",
            id,
            pos.x + 0.5,
            pos.z,
            pos.y + 0.5,
            facing.x + 0.5,
            facing.z,
            facing.y + 0.5
        ));
        if let Some(vill) = vill {
            replay.command(format!(
                "tp {} {:.2} {:.2} {:.2} {:.0} 0",
                vill.carry_id,
                pos.x + 0.5,
                pos.z + 0.8,
                pos.y + 0.5,
                delta.y.atan2(delta.x) / PI * 180.,
            ));
        }
        prev.0 = pos.0;
    }
    for vill in &changed_vills {
        if let Some(stack) = vill.carry {
            replay.command(format!(
                "data modify entity {} ArmorItems[3] set value {}",
                vill.carry_id,
                stack
                    .kind
                    .display_as_block()
                    .blockstate(&UNKNOWN_BLOCKS.write().unwrap())
                    .item_snbt()
            ));
        } else {
            replay.command(format!(
                "data modify entity {} ArmorItems[3] set value {{}}",
                vill.carry_id,
            ));
        }
    }
    replay.tick();
}
