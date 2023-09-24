use crate::sim::*;
use crate::*;
use bevy_ecs::prelude::*;
use nbt::{CompoundTag, Tag};

use std::collections::VecDeque;
use std::f32::consts::PI;
use std::fmt::Display;
use std::fs::{create_dir_all, read, write};
use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

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
    level_path: PathBuf,
    block_cache: HashMap<Block, String>,
    commands_this_tick: Vec<Tag>,
    commands: VecDeque<Tag>,
    command_chunk: i32,
    commands_this_chunk: i32,
    total_commands: u64,
    writes_in_flight: Arc<AtomicU32>,
}

impl Replay {
    pub fn new(level_path: PathBuf) -> Self {
        let mut replay = Self {
            level_path,
            block_cache: default(),
            commands_this_tick: default(),
            commands: default(),
            command_chunk: 0,
            commands_this_chunk: 0,
            total_commands: 0,
            writes_in_flight: default(),
        };

        // Wait for the player to load in
        for _ in 0..10 {
            replay.tick();
        }
        replay
    }

    pub fn dbg(&mut self, msg: &str) {
        self.command(format!("say {msg}"));
    }

    pub fn dust(&mut self, pos: IVec3) {
        self.command(format!(
            "particle campfire_cosy_smoke {} {} {} 1.3 1.3 1.3 0.006 10",
            pos.x, pos.z, pos.y
        ));
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
        self.commands_this_chunk += 1;
        self.total_commands += 1;
    }

    fn tick(&mut self) {
        const MAX_COMMANDS_PER_CHUNK: i32 = 50000;
        let commands = std::mem::take(&mut self.commands_this_tick);
        if self.commands_this_chunk < MAX_COMMANDS_PER_CHUNK {
            self.commands.push_front(commands.into());
        } else {
            self.flush_chunk();
        }
    }

    fn flush_chunk(&mut self) {
        const INITIAL_CAPACITY: usize = 5000;
        self.command(format!(
            "data modify storage sim_0:data commands set from storage sim_{}:data commands",
            self.command_chunk + 1
        ));
        let tick_commands = std::mem::take(&mut self.commands_this_tick);
        let mut commands = std::mem::replace(
            &mut self.commands,
            VecDeque::with_capacity(INITIAL_CAPACITY),
        );
        commands.push_front(tick_commands.into());

        let data_path = self.level_path.join("data/");
        let chunk = self.command_chunk;
        let arc = self.writes_in_flight.clone();
        arc.fetch_add(1, Ordering::Relaxed);
        rayon::spawn(move || {
            create_dir_all(&data_path).unwrap();
            let mut nbt = CompoundTag::new();
            nbt.insert("DataVersion", DATA_VERSION);
            nbt.insert("data", {
                let mut nbt = CompoundTag::new();
                nbt.insert("contents", {
                    let mut nbt = CompoundTag::new();
                    nbt.insert("data", {
                        let mut nbt = CompoundTag::new();
                        nbt.insert("commands", nbt::Tag::List(commands.drain(..).collect()));
                        nbt
                    });
                    nbt
                });
                nbt
            });
            nbt::encode::write_gzip_compound_tag(
                &mut std::fs::File::create(
                    data_path.join(format!("command_storage_sim_{}.dat", chunk)),
                )
                .unwrap(),
                &nbt,
            )
            .unwrap();
            arc.fetch_sub(1, Ordering::Relaxed);
        });

        self.command_chunk += 1;
        self.commands_this_chunk = 0;
    }

    // TODO: do this asynchronically in the background as commands come in
    pub fn finish(mut self) {
        self.flush_chunk();
        // Could have used a condvar instead
        while self.writes_in_flight.load(Ordering::Relaxed) > 0 {
            std::thread::yield_now()
        }
        println!("Total commands: {}", self.total_commands);

        let pack_path = self.level_path.join("datapacks/sim/");
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
            function sim:eval with storage sim_0:data commands[-1][-1]
            data remove storage sim_0:data commands[-1][-1]
            execute if data storage sim_0:data commands[-1][0] run function sim:run_current_commands
            ",
        )
        .unwrap();

        write(
            sim_path.join("tick.mcfunction"),
            "
            function sim:run_current_commands
            data remove storage sim_0:data commands[-1]
            scoreboard players add SIM sim_tick 1
            ",
        )
        .unwrap();
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
