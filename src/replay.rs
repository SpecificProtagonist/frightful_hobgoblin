use crate::sim::lumberjack::Lumberworker;
use crate::sim::quarry::Mason;
use crate::sim::*;
use crate::*;
use bevy_ecs::prelude::*;
use nbt::{CompoundTag, Tag};

use std::fmt::{Display, Write};
use std::fs::{create_dir_all, read, write};
use std::ops::DerefMut;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[derive(Component, Copy, Clone)]
pub struct Id(u32);

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0-0-0-0-{:x}", self.0)
    }
}

// Maybe just use normal random uuids? Small numbers make it easier to debug though.
impl Id {
    pub fn snbt(&self) -> String {
        format!("UUID:[I;0,0,0,{}]", self.0)
    }
}

// This is only correct after Replay has been constructed
impl Default for Id {
    fn default() -> Self {
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

// Used to offload encoding to gzipped nbt to worker threads
enum Command {
    Literal(String),
    Block(IVec3, Block),
    Dust(IVec3),
    Tp(Id, Vec3, Vec3),
}

impl Command {
    fn format(self, block_cache: &mut HashMap<Block, String>) -> String {
        match self {
            Command::Literal(s) => s,
            Command::Block(pos, block) => {
                let block_string = block_cache.entry(block).or_insert_with(|| {
                    block
                        .blockstate(&UNKNOWN_BLOCKS.read().unwrap())
                        .to_string()
                });
                format!("setblock {} {} {} {block_string}", pos.x, pos.z, pos.y)
            }
            Command::Dust(pos) => format!(
                "particle campfire_cosy_smoke {} {} {} 1.3 1.3 1.3 0.006 10",
                pos.x, pos.z, pos.y
            ),
            // TODO: how to do facing when climbing a ladder?
            Command::Tp(id, pos, facing) => format!(
                "tp {} {:.2} {:.2} {:.2} facing {:.2} {:.2} {:.2}",
                id,
                pos.x + 0.5,
                pos.z,
                pos.y + 0.5,
                facing.x + 0.5,
                facing.z,
                facing.y + 0.5
            ),
        }
    }
}

#[derive(Resource)]
pub struct Replay {
    level_path: PathBuf,
    invocation: u8,
    area: Rect,
    // Stored in reverse order
    commands_this_tick: Vec<Command>,
    // Stored in reverse order
    commands: Vec<Vec<Command>>,
    command_chunk: i32,
    commands_this_chunk: i32,
    total_commands: u64,
    writes_in_flight: Arc<AtomicU32>,
    carry_ids: Vec<(Id, Id)>,
}

impl Replay {
    pub fn new(level: &Level) -> Self {
        let mut invocation = 0;
        // Some information is needed if the generator is invoked multiple times
        // so that replays don't interfere with each other
        if let Ok(vec) = read(level.path.join("mcgen-meta")) {
            invocation = vec[0] + 1;
            NEXT_ID.store(
                u32::from_be_bytes([vec[1], vec[2], vec[3], vec[4]]),
                Ordering::Relaxed,
            );
        };

        let mut replay = Self {
            level_path: level.path.clone(),
            invocation,
            area: level.area(),
            commands_this_tick: default(),
            commands: default(),
            command_chunk: 0,
            commands_this_chunk: 0,
            total_commands: 0,
            writes_in_flight: default(),
            carry_ids: default(),
        };

        // Wait for the player to load in
        for _ in 0..20 {
            replay.tick();
        }
        replay
    }

    pub fn dbg(&mut self, msg: &str) {
        self.command(format!("say {msg}"));
    }

    pub fn dust(&mut self, pos: IVec3) {
        self.commands_this_tick.push(Command::Dust(pos));
        self.commands_this_chunk += 1;
        self.total_commands += 1;
    }

    pub fn block(&mut self, pos: IVec3, block: Block) {
        self.commands_this_tick.push(Command::Block(pos, block));
        self.commands_this_chunk += 1;
        self.total_commands += 1;
    }

    pub fn tp(&mut self, id: Id, pos: Vec3, facing: Vec3) {
        self.commands_this_tick.push(Command::Tp(id, pos, facing));
        self.commands_this_chunk += 1;
        self.total_commands += 1;
    }

    pub fn command(&mut self, msg: String) {
        self.commands_this_tick.push(Command::Literal(msg));
        self.commands_this_chunk += 1;
        self.total_commands += 1;
    }

    fn tick(&mut self) {
        const MAX_COMMANDS_PER_CHUNK: i32 = 30000;
        if self.commands_this_chunk < MAX_COMMANDS_PER_CHUNK {
            let commands = std::mem::take(&mut self.commands_this_tick);
            self.commands.push(commands);
        } else {
            self.flush_chunk();
        }
    }

    fn flush_chunk(&mut self) {
        const INITIAL_CAPACITY: usize = 1000;
        // This needs to be the last commands to get executed this tick
        self.command(format!(
            "data modify storage sim_{0}_0:data commands set from storage sim_{0}_{1}:data commands",
            self.invocation,
            self.command_chunk + 1
        ));
        let tick_commands = std::mem::take(&mut self.commands_this_tick);
        let mut commands =
            std::mem::replace(&mut self.commands, Vec::with_capacity(INITIAL_CAPACITY));
        commands.push(tick_commands);

        let data_path = self.level_path.join("data/");
        let invocation = self.invocation;
        let chunk = self.command_chunk;
        let arc = self.writes_in_flight.clone();
        arc.fetch_add(1, Ordering::Relaxed);
        rayon::spawn(move || {
            create_dir_all(&data_path).unwrap();
            let mut block_cache = default();
            let commands_tag = Tag::List(
                commands
                    .into_iter()
                    .rev()
                    .map(|c| {
                        nbt::Tag::List(
                            c.into_iter()
                                .rev()
                                .map(|c| {
                                    let mut nbt = CompoundTag::new();
                                    nbt.insert("cmd", c.format(&mut block_cache));
                                    nbt.into()
                                })
                                .collect(),
                        )
                    })
                    .collect(),
            );
            let mut nbt = CompoundTag::new();
            nbt.insert("DataVersion", DATA_VERSION);
            nbt.insert("data", {
                let mut nbt = CompoundTag::new();
                nbt.insert("contents", {
                    let mut nbt = CompoundTag::new();
                    nbt.insert("data", {
                        let mut data = CompoundTag::new();
                        data.insert("commands", commands_tag);
                        data
                    });
                    nbt
                });
                nbt
            });
            nbt::encode::write_gzip_compound_tag(
                &mut std::fs::File::create(
                    data_path.join(format!("command_storage_sim_{invocation}_{chunk}.dat")),
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

    pub fn finish(mut self) {
        self.flush_chunk();

        let pack_path = self
            .level_path
            .join(format!("datapacks/sim_{}/", self.invocation));
        create_dir_all(&pack_path).unwrap();
        write(
            pack_path.join("pack.mcmeta"),
            r#"{"pack": {"pack_format": 10, "description": ""}}"#,
        )
        .unwrap();

        let sim_path = pack_path.join(format!("data/sim_{}/functions/", self.invocation));
        create_dir_all(&sim_path).unwrap();

        // Could also just modify the world directly, but this is easier
        write(
            sim_path.join("setup.mcfunction"),
            format!(
                "
            scoreboard players set SIM_{0} sim_tick 0
            # How many sim ticks to replay per game tick (0 to stop)
            scoreboard objectives add speed dummy
            scoreboard players set SIM_{0} speed 1
            # Set to X to warp X sim ticks ahead
            scoreboard objectives add warp dummy
            scoreboard players set SIM_{0} warp 0
            gamerule randomTickSpeed 0
            gamerule doMobSpawning false
            gamerule mobGriefing false
            gamerule doFireTick false
            gamerule doTileDrops false
            ",
                self.invocation
            ),
        )
        .unwrap();

        write(
            sim_path.join("check_setup.mcfunction"),
            format!(
                "
            scoreboard objectives add sim_tick dummy
            execute unless score SIM_{0} sim_tick matches 0.. run function sim_{0}:setup
            ",
                self.invocation
            ),
        )
        .unwrap();

        let tag_path = pack_path.join("data/minecraft/tags/functions/");
        create_dir_all(&tag_path).unwrap();
        write(
            tag_path.join("load.json"),
            format!("{{values:[\"sim_{}:check_setup\"]}}", self.invocation),
        )
        .unwrap();

        // For now just start automatically
        write(
            tag_path.join("tick.json"),
            format!("{{values:[\"sim_{}:check_tick\"]}}", self.invocation),
        )
        .unwrap();

        write(sim_path.join("eval.mcfunction"), "$$(cmd)").unwrap();
        write(
            sim_path.join("run_current_commands.mcfunction"),
            format!("
            function sim_{0}:eval with storage sim_{0}_0:data commands[-1][-1]
            data remove storage sim_{0}_0:data commands[-1][-1]
            execute if data storage sim_{0}_0:data commands[-1][0] run function sim_{0}:run_current_commands
            ", self.invocation),
        )
        .unwrap();

        write(
            sim_path.join("sim_tick.mcfunction"),
            format!("
            function sim_{0}:run_current_commands
            execute unless data storage sim_{0}_0:data commands[-1][0] run data remove storage sim_{0}_0:data commands[-1]
            scoreboard players add SIM_{0} sim_tick 1
            scoreboard players remove SIM_{0} warp 1
            execute if score SIM_{0} warp matches 1.. run function sim_{0}:sim_tick
            ", self.invocation),
        )
        .unwrap();
        write(sim_path.join("tick.mcfunction"), {
            let mut tick = format!(
                "
                scoreboard players operation SIM_{0} warp += SIM_{0} speed
                execute if score SIM_{0} warp matches 1.. run function sim_{0}:sim_tick
            ",
                self.invocation
            );
            for (vill, carry) in self.carry_ids {
                writeln!(tick, "tp {carry} {vill}").unwrap();
            }
            writeln!(tick, "execute as @e[tag=carry] at @s run tp ~ ~0.8 ~").unwrap();
            tick
        })
        .unwrap();
        write(
            sim_path.join("check_tick.mcfunction"),
            format!(
                "execute if entity @e[type=player,x={},z={},dx={},dz={},y=-100,dy=400] run function sim_{}:tick",
                self.area.min.x,
                self.area.min.y,
                self.area.size().x,
                self.area.size().y,
                self.invocation
            ),
        )
        .unwrap();

        // Could have used a condvar instead
        while self.writes_in_flight.load(Ordering::Relaxed) > 0 {
            std::thread::yield_now()
        }
        println!("Total commands: {}", self.total_commands);

        // Store information needed when the generator is invokes on
        // the same map multiple times
        let mut meta = Vec::new();
        meta.push(self.invocation);
        meta.extend_from_slice(&NEXT_ID.load(Ordering::Relaxed).to_be_bytes());
        write(self.level_path.join("mcgen-meta"), meta).unwrap();
    }
}

pub fn tick_replay(
    mut level: ResMut<Level>,
    mut replay: ResMut<Replay>,
    new_vills: Query<(&Id, &Pos, &Villager), Added<Villager>>,
    changed_vills: Query<&Villager, Changed<Villager>>,
    mut moved: Query<(&Id, &Pos, &mut PrevPos, Option<&InBoat>), Changed<Pos>>,
    jobless: Query<&Id, Added<Jobless>>,
    lumberjacks: Query<&Id, Added<Lumberworker>>,
    masons: Query<&Id, Added<Mason>>,
) {
    let replay = replay.deref_mut();
    // Blocks
    for set in level.pop_recording(default()) {
        replay.block(set.pos, set.block);
    }
    // New villagers
    for (id, pos, vill) in &new_vills {
        let biome = level.biome(pos.block().truncate());
        replay.command(format!(
            "summon villager {} {} {} {{{}, NoAI:1, Invulnerable:1, VillagerData:{{type:\"{}\"}}}}",
            pos.x,
            pos.z,
            pos.y,
            id.snbt(),
            biome.villager_type()
        ));
        replay.command(format!(
            // TODO: Use block display?
            "summon armor_stand {} {} {} {{{}, Invulnerable:1, Invisible:1, NoGravity:1, Tags:[\"carry\"]}}",
            pos.x,
            pos.z + 0.8,
            pos.y,
            vill.carry_id.snbt(),
        ));
        replay.carry_ids.push((*id, vill.carry_id));
    }
    // Movement
    for (id, pos, mut prev, in_boat) in &mut moved {
        let delta = pos.0 - prev.0;
        let facing = pos.0 + delta;
        if let Some(in_boat) = in_boat {
            let off = vec3(0., 0., -0.48);
            // Unfortunately the boat lags behind (visually only?)
            replay.tp(in_boat.0, pos.0 + off, facing + off);
        } else {
            replay.tp(*id, pos.0, facing);
        }
        prev.0 = pos.0;
    }
    // Carrying
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
    // Professions
    for id in &jobless {
        replay.command(format!(
            "data modify entity {id} VillagerDate.profession set value none",
        ));
    }
    for id in &lumberjacks {
        replay.command(format!(
            "data modify entity {id} VillagerDate.profession set value nitwit",
        ));
    }
    for id in &masons {
        replay.command(format!(
            "data modify entity {id} VillagerDate.profession set value mason",
        ));
    }

    replay.tick();
}
