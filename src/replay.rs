use crate::sim::lumberjack::Lumberworker;
use crate::sim::quarry::Mason;
use crate::sim::*;
use crate::*;
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemChangeTick;
use flate2::write::GzEncoder;
use flate2::Compression;
use nbt::encode::write_compound_tag;
use nbt::{CompoundTag, Tag};
use serde::Serialize;

use std::fmt::{Display, Write};
use std::fs::{create_dir_all, read_to_string, write, File};
use std::io::Write as _;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;

use self::steady_state::Trader;

// TODO: When warping ahead, skip tps except for the last ones
// to do that, store tps in a seperate list
// TODO: allow (option when running the generator) to skip the replay of the first n ticks
// to do that, directly write the commands (skipping tps and setblocks that later get overwritten) to a init function

#[derive(Component, Copy, Clone)]
pub struct Id(u32);

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0-0-0-0-{:x}", self.0)
    }
}

// Maybe just use normal random uuids? Small numbers make it easier to debug though.
impl Id {
    /// This is only correct after Replay has been constructed
    pub fn new() -> Self {
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn snbt(&self) -> String {
        format!("UUID:[I;0,0,0,{}]", self.0)
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

static NEXT_ID: AtomicU32 = AtomicU32::new(1);
static INVOCATION: AtomicU8 = AtomicU8::new(0);
pub fn invocation() -> u8 {
    INVOCATION.load(Ordering::Relaxed)
}

// Used to offload encoding to gzipped nbt to worker threads
enum Command {
    Literal(String),
    Block(IVec3, Block, Option<String>),
    Dust(IVec3),
    Tp(Id, Vec3, Vec3),
}

impl Command {
    fn format(self, block_cache: &mut HashMap<Block, String>) -> String {
        match self {
            Command::Literal(s) => s,
            Command::Block(pos, block, nbt) => {
                let block_string = block_cache.entry(block).or_insert_with(|| {
                    block
                        .blockstate(&UNKNOWN_BLOCKS.read().unwrap())
                        .to_string()
                });
                format!(
                    "setblock {} {} {} {block_string}{}",
                    pos.x,
                    pos.z,
                    pos.y,
                    nbt.as_deref()
                        .map(|s| format!("{{{s}}}"))
                        .unwrap_or("".to_owned())
                )
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
    /// Hack needed to ignore changes from older tracks (can't set system change tick)
    pub skip_changes_once: bool,
    level_path: PathBuf,
    area: Rect,
    tracks: Vec<Track>,
    pub track: usize,
    skip_tick: bool,
    total_commands: u64,
    writes_in_flight: Arc<AtomicU32>,
    carry_ids: Vec<(Id, Id)>,
}

#[derive(Default)]
struct Track {
    // Stored in reverse order
    commands_this_tick: Vec<Command>,
    // Stored in reverse order
    commands: Vec<Vec<Command>>,
    command_chunk: i32,
    commands_this_chunk: i32,
}

const META_FILE: &str = "frightful-hobgoblin.toml";
#[derive(Serialize, Deserialize)]
struct StoredMeta {
    invocation: u8,
    next_id: u32,
}

impl Replay {
    pub fn new(level: &Level) -> Self {
        // Some information is needed if the generator is invoked multiple times
        // so that replays don't interfere with each other
        if let Ok(content) = read_to_string(level.path.join(META_FILE)) {
            let meta: StoredMeta = toml::from_str(&content).unwrap();
            INVOCATION.store(meta.invocation + 1, Ordering::Relaxed);
            NEXT_ID.store(meta.next_id, Ordering::Relaxed);
        };

        let mut replay = Self {
            skip_changes_once: false,
            level_path: level.path.clone(),
            area: level.area(),
            tracks: vec![default()],
            track: 0,
            skip_tick: false,
            total_commands: 0,
            writes_in_flight: default(),
            carry_ids: default(),
        };

        // Wait for the player to load in
        for _ in 0..20 {
            replay.tick();
        }
        replay.say("Defaulting replay speed to 5×", Gray);
        replay
    }

    fn track(&mut self) -> &mut Track {
        &mut self.tracks[self.track]
    }

    #[allow(unused_variables)]
    pub fn dbg(&mut self, msg: &str) {
        #[cfg(debug_assertions)]
        println!("{msg}");
        #[cfg(debug_assertions)]
        self.command(format!(
            "tellraw @a {{\"text\":\"{msg}\",\"color\":\"gray\"}}",
        ));
    }

    pub fn say(&mut self, msg: &str, color: Color) {
        self.command(format!(
            "tellraw @a[tag=sim_{}_in_area] {{\"text\":\"{msg}\",\"color\":\"{color}\"}}",
            invocation()
        ));
    }

    pub fn dust(&mut self, pos: IVec3) {
        self.track().commands_this_tick.push(Command::Dust(pos));
        self.track().commands_this_chunk += 1;
        self.total_commands += 1;
    }

    pub fn block(&mut self, pos: IVec3, block: Block, nbt: Option<String>) {
        self.track()
            .commands_this_tick
            .push(Command::Block(pos, block, nbt));
        self.track().commands_this_chunk += 1;
        self.total_commands += 1;
    }

    pub fn tp(&mut self, id: Id, pos: Vec3, facing: Vec3) {
        self.track()
            .commands_this_tick
            .push(Command::Tp(id, pos, facing));
        self.track().commands_this_chunk += 1;
        self.total_commands += 1;
    }

    pub fn command(&mut self, msg: String) {
        self.track().commands_this_tick.push(Command::Literal(msg));
        self.track().commands_this_chunk += 1;
        self.total_commands += 1;
    }

    fn tick(&mut self) {
        const MAX_COMMANDS_PER_CHUNK: i32 = 40000;
        if self.track().commands_this_chunk < MAX_COMMANDS_PER_CHUNK {
            let commands = std::mem::take(&mut self.track().commands_this_tick);
            self.track().commands.push(commands);
        } else {
            self.flush_chunk();
        }
    }

    fn flush_chunk(&mut self) {
        // Switch over to the next chunk on the same track
        // This needs to be the last commands to get executed this tick
        let chunk = self.track().command_chunk;
        self.command(format!(
            "data modify storage sim_{0}_track{1}:data commands set from storage sim_{0}_track{1}_chunk{2}:data commands",
            invocation(),
            self.track,
            chunk + 1
        ));
        let tick_commands = std::mem::take(&mut self.track().commands_this_tick);
        let mut commands = std::mem::replace(&mut self.track().commands, Vec::with_capacity(1000));
        commands.push(tick_commands);

        let data_path = self.level_path.join("data/");
        let track = self.track;
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
            let mut file = File::create(data_path.join(format!(
                "command_storage_sim_{}_track{track}_chunk{chunk}.dat",
                invocation()
            )))
            .unwrap();
            // Write to a buffer first.
            // If writing directly to a GzEncoder and the chunk size is too big, it
            // gets silently trunctated?!?
            let mut uncompressed = Vec::new();
            write_compound_tag(&mut uncompressed, &nbt).unwrap();
            GzEncoder::new(&mut file, Compression::new(1))
                .write_all(&uncompressed)
                .unwrap();

            arc.fetch_sub(1, Ordering::Relaxed);
        });

        self.track().command_chunk += 1;
        self.track().commands_this_chunk = 0;
    }

    pub fn begin_next_track(&mut self) -> usize {
        self.track = self.tracks.len();
        self.tracks.push(default());
        self.skip_tick = true;
        self.track
    }

    pub fn mcfunction(&self, name: &str, content: &str) {
        let path = self.level_path.join(format!(
            "datapacks/sim_{0}/data/sim_{0}/function/{name}.mcfunction",
            invocation()
        ));
        create_dir_all(path.parent().unwrap()).unwrap();
        write(path, content).unwrap();
    }

    pub fn finish(mut self) {
        for track in 0..self.tracks.len() {
            self.track = track;
            self.flush_chunk();
        }

        let pack_path = self
            .level_path
            .join(format!("datapacks/sim_{}/", invocation()));
        create_dir_all(&pack_path).unwrap();
        write(
            pack_path.join("pack.mcmeta"),
            r#"{"pack": {"pack_format": 48, "description": ""}}"#,
        )
        .unwrap();

        let tag_path = pack_path.join("data/minecraft/tags/function/");
        create_dir_all(&tag_path).unwrap();
        write(
            tag_path.join("load.json"),
            format!("{{values:[\"sim_{}:check_setup\"]}}", invocation()),
        )
        .unwrap();

        write(
            tag_path.join("tick.json"),
            format!("{{values:[\"sim_{}:check_tick\"]}}", invocation()),
        )
        .unwrap();

        // Could also just modify the world directly, but this is easier
        self.mcfunction(
            "setup",
            &format!(
                "
            data modify storage sim_{0}:data active_tracks set value []
            data modify storage sim_{0}:data newly_queued_tracks set value []
            data modify storage sim_{0}:data track set value {{}}
            scoreboard objectives add rand dummy
            scoreboard objectives add daytime dummy
            scoreboard objectives add sim_{0}_sleep dummy
            scoreboard objectives add sim_{0}_particle dummy
            function sim_{0}:play_track_global {{track:0}}
            scoreboard players set SIM_{0} sim_tick 0
            
            # How many sim ticks to replay per game tick (0 to stop)
            scoreboard objectives add speed dummy
            scoreboard players set SIM_{0} speed 1
            # Set to X to warp X sim ticks ahead
            scoreboard objectives add warp dummy
            scoreboard players set SIM_{0} warp 0

            scoreboard objectives add sim_blurb_cooldown dummy

            gamerule randomTickSpeed 0
            gamerule doMobSpawning false
            gamerule mobGriefing false
            gamerule doFireTick false
            gamerule doTileDrops false
            ",
                invocation()
            ),
        );

        self.mcfunction(
            "check_setup",
            &format!(
                "
            scoreboard objectives add sim_tick dummy
            execute unless score SIM_{0} sim_tick matches 0.. run function sim_{0}:setup
            ",
                invocation()
            ),
        );

        // Args: cmd
        self.mcfunction("eval", "$$(cmd)");
        // Args: track
        self.mcfunction(
            "run_current_commands",
            &format!("
            $function sim_{0}:eval with storage sim_{0}_track$(track):data commands[-1][-1]
            $data remove storage sim_{0}_track$(track):data commands[-1][-1]
            $execute if data storage sim_{0}_track$(track):data commands[-1][0] run function sim_{0}:run_current_commands {{track:$(track)}}
            ", invocation()),
        );
        // Args: track
        self.mcfunction("tick_track", &format!("
            $function sim_{0}:run_current_commands {{track:$(track)}}
            $execute if data storage sim_{0}_track$(track):data commands[0] run data modify storage sim_{0}:data progressable_tracks append value $(track)
            $execute unless data storage sim_{0}_track$(track):data commands[-1][0] run data remove storage sim_{0}_track$(track):data commands[-1]
        ", invocation()));

        self.mcfunction(
            "tick_tracks",
            &format!(
                "
            data modify storage sim_{0}:data track.track set from storage sim_{0}:data tracks_to_tick[-1]
            function sim_{0}:tick_track with storage sim_{0}:data track
            data remove storage sim_{0}:data tracks_to_tick[-1]
            execute if data storage sim_{0}:data tracks_to_tick[0] run function sim_{0}:tick_tracks
            ",
                invocation()
            ),
        );

        // Args: track
        // TODO: Can this be removed now that there is play_track_on_self?
        self.mcfunction(
            "play_track_global",
            &format!(
                "
            $data modify storage sim_{0}:data newly_queued_tracks append value $(track)
            ",
                invocation()
            ),
        );

        // Args: track
        self.mcfunction(
            "add_newly_queued_track",
            &format!(
                "
            $data modify storage sim_{0}_track$(track):data commands set from storage sim_{0}_track$(track)_chunk0:data commands
            ",
                invocation()
            ),
        );
        self.mcfunction(
            "add_newly_queued_tracks",
            &format!(
                "
            data modify storage sim_{0}:data active_tracks append from storage sim_{0}:data newly_queued_tracks[-1] 
            data modify storage sim_{0}:data track.track set from storage sim_{0}:data newly_queued_tracks[-1]
            execute if data storage sim_{0}:data newly_queued_tracks[0] run function sim_{0}:add_newly_queued_track with storage sim_{0}:data track
            data remove storage sim_{0}:data newly_queued_tracks[-1]
            execute if data storage sim_{0}:data newly_queued_tracks[0] run function sim_{0}:add_newly_queued_tracks
            ",
                invocation()
            ),
        );

        // Args: track
        self.mcfunction("tick_track_on_self", &format!("
            $execute if data entity @s data.play run data modify storage sim_{0}_track$(track):data commands set from storage sim_{0}_track$(track)_chunk0:data commands
            $function sim_{0}:run_current_commands {{track:$(track)}}
            $execute unless data storage sim_{0}_track$(track):data commands[-1][0] run data remove storage sim_{0}_track$(track):data commands[-1]
            $execute unless data storage sim_{0}_track$(track):data commands[0] run data remove entity @s data.track
        ", invocation()));

        // Args: on_idle
        self.mcfunction(
            "on_tick",
            &format!("
            data modify entity @s data.track set from entity @s data.play
            execute if data entity @s data.track run function sim_{0}:tick_track_on_self with entity @s data
            data remove entity @s data.play
            $execute unless data entity @s data.track run function sim_{0}:on_idle/$(on_idle)",
                invocation()
            ),
        );

        self.mcfunction(
            "say_blurb",
            &format!(
                "execute store result score @s sim_blurb_cooldown run random value 0..500
                execute store result storage sim_{0}:data say.index int 1. run random value 0..200
                function sim_{0}:say_blurb_macro with storage sim_{0}:data say",
                invocation()
            ),
        );

        // Args: index
        self.mcfunction(
            "say_blurb_macro",
            &format!(
                "$data modify storage sim_{0}:data say.blurb set from storage sim_{0}_language:data blurbs[$(index)]
                function sim_{0}:say_blurb_macro_2 with storage sim_{0}:data say",
                invocation()
            ),
        );

        // Args: blurb
        self.mcfunction(
            "say_blurb_macro_2",
            "$tellraw @a[distance=..10] [\"<\",{\"selector\":\"@s\"},\"> $(blurb)\"]",
        );

        self.mcfunction(
            "sim_tick",
            &format!("
            data modify storage sim_{0}:data tracks_to_tick set from storage sim_{0}:data active_tracks
            data modify storage sim_{0}:data progressable_tracks set value []
            function sim_{0}:tick_tracks
            data modify storage sim_{0}:data active_tracks set from storage sim_{0}:data progressable_tracks
            function sim_{0}:add_newly_queued_tracks
            execute as @e[tag=sim_{0}_tick] unless score @s sim_{0}_sleep matches 1.. run function sim_{0}:on_tick with entity @s data
            scoreboard players remove @e[scores={{sim_{0}_sleep=1..}}] sim_{0}_sleep 1
            scoreboard players add SIM_{0} sim_tick 1
            scoreboard players remove SIM_{0} warp 1

            execute as @a at @s run scoreboard players remove @e[tag=sim_{0}_villager,distance=..8] sim_blurb_cooldown 1
            execute as @e[scores={{sim_blurb_cooldown=..-160}}] at @s run function sim_{0}:say_blurb

            execute if score SIM_{0} warp matches 1.. run function sim_{0}:sim_tick
            ", invocation()),
        );

        self.mcfunction("game_tick", &{
            let mut tick = format!(
                "
                execute if score sim speed matches 0.. run scoreboard players operation SIM_{0} speed = sim speed
                scoreboard players set sim speed -1
                execute if score sim warp matches 0.. run scoreboard players operation SIM_{0} warp = sim warp
                scoreboard players set sim warp -1
                scoreboard players operation SIM_{0} warp += SIM_{0} speed
                execute if score SIM_{0} warp matches 1.. run function sim_{0}:sim_tick

                execute as @e[tag=sim_{0}_smoke] run scoreboard players remove @s sim_{0}_particle 1
                execute as @e[tag=sim_{0}_smoke,scores={{sim_{0}_particle=..0}}] store result score @s sim_{0}_particle run random value 0..10
                execute as @e[tag=sim_{0}_smoke,scores={{sim_{0}_particle=..0}}] at @s run particle minecraft:campfire_signal_smoke ~ ~1 ~ 0 0.2 0 0.003 1
                execute as @e[tag=sim_{0}_smoke,scores={{sim_{0}_particle=..0}}] at @s run particle minecraft:campfire_signal_smoke ~ ~3 ~ 0 1 0 0.003 1
            ",
                invocation()
            );
            for (vill, carry) in &self.carry_ids {
                writeln!(tick, "tp {carry} {vill}").unwrap();
            }
            writeln!(tick, "execute as @e[tag=carry] at @s run tp ~ ~0.8 ~").unwrap();
            tick
        });

        self.mcfunction(
            "check_tick",
            &format!(
                "
                tag @p[tag=sim_{4}_in_area] add sim_{4}_previous_in_area
                tag @a remove sim_{4}_in_area
                tag @e[type=player,x={},z={},dx={},dz={},y=-100,dy=400] add sim_{4}_in_area
                tellraw @a[tag=!sim_{4}_in_area,tag=sim_{4}_previous_in_area] {{\"text\":\"Exited build area, replay paused\",\"color\":\"gray\"}}
                tellraw @a[tag=sim_{4}_in_area,tag=!sim_{4}_previous_in_area] {{\"text\":\"Entered build area, replay resumed\",\"color\":\"gray\"}}
                tellraw @a[tag=sim_{4}_in_area,tag=!sim_{4}_previous_in_area] [{{\"text\":\"Click to set replay speed: \",\"color\":\"gray\"}},{{\"text\":\"[\",\"color\":\"dark_green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim speed 0\"}}}},{{\"text\":\"pause\",\"color\":\"green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim speed 0\"}}}},{{\"text\":\"]\",\"color\":\"dark_green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim speed 0\"}}}},{{\"text\":\" \"}},{{\"text\":\"[\",\"color\":\"dark_green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim speed 1\"}}}},{{\"text\":\"1×\",\"color\":\"green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim speed 1\"}}}},{{\"text\":\"]\",\"color\":\"dark_green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim speed 1\"}}}},{{\"text\":\" \"}},{{\"text\":\"[\",\"color\":\"dark_green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim speed 3\"}}}},{{\"text\":\"3×\",\"color\":\"green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim speed 3\"}}}},{{\"text\":\"]\",\"color\":\"dark_green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim speed 3\"}}}},{{\"text\":\" \"}},{{\"text\":\"[\",\"color\":\"dark_green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim speed 5\"}}}},{{\"text\":\"5×\",\"color\":\"green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim speed 5\"}}}},{{\"text\":\"]\",\"color\":\"dark_green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim speed 5\"}}}},{{\"text\":\" \"}},{{\"text\":\"[\",\"color\":\"dark_green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim speed 10\"}}}},{{\"text\":\"10×\",\"color\":\"green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim speed 10\"}}}},{{\"text\":\"]\",\"color\":\"dark_green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim speed 10\"}}}},{{\"text\":\" \"}},{{\"text\":\"[\",\"color\":\"dark_green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim speed 20\"}}}},{{\"text\":\"20×\",\"color\":\"green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim speed 20\"}}}},{{\"text\":\"]\",\"color\":\"dark_green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim speed 20\"}}}}]
                tellraw @a[tag=sim_{4}_in_area,tag=!sim_{4}_previous_in_area] [{{\"text\":\"Click to warp ahead: \",\"color\":\"gray\"}},{{\"text\":\"[\",\"color\":\"dark_green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim warp 1200\"}}}},{{\"text\":\"1 minute\",\"color\":\"green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim warp 1200\"}}}},{{\"text\":\"]\",\"color\":\"dark_green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim warp 1200\"}}}},{{\"text\":\" \"}},{{\"text\":\"[\",\"color\":\"dark_green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim warp 6000\"}}}},{{\"text\":\"5 minutes\",\"color\":\"green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim warp 6000\"}}}},{{\"text\":\"]\",\"color\":\"dark_green\",\"clickEvent\":{{\"action\":\"run_command\",\"value\":\"/scoreboard players set sim warp 6000\"}}}}]
                tag @a remove sim_{4}_previous_in_area
                execute if entity @p[tag=sim_{4}_in_area] run function sim_{}:game_tick",
                self.area.min.x,
                self.area.min.y,
                self.area.size().x,
                self.area.size().y,
                invocation()
            ),
        );

        // Could have used a condvar instead
        while self.writes_in_flight.load(Ordering::Relaxed) > 0 {
            std::thread::yield_now()
        }
        println!("Total commands: {}", self.total_commands);

        // Store information needed when the generator is invokes on
        // the same map multiple times
        write(
            self.level_path.join(META_FILE),
            toml::to_string(&StoredMeta {
                invocation: invocation(),
                next_id: NEXT_ID.load(Ordering::Relaxed),
            })
            .unwrap(),
        )
        .unwrap();
    }
}

pub fn tick_replay(
    change_tick: SystemChangeTick,
    mut level: ResMut<Level>,
    mut replay: ResMut<Replay>,
    new_vills: Query<(&Id, &Pos, &Villager, Has<Trader>), Added<Villager>>,
    named: Query<(&Id, &Name), Changed<Name>>,
    changed_vills: Query<&Villager, Changed<Villager>>,
    mut moved: Query<(&Id, &Pos, &mut PrevPos, Option<&InBoat>), Changed<Pos>>,
    lumberjacks: Query<&Id, Added<Lumberworker>>,
    masons: Query<&Id, Added<Mason>>,
) {
    if replay.skip_changes_once {
        replay.skip_changes_once = false;
        return;
    }
    if change_tick.last_run().get() == 0 {
        return;
    }

    let replay = replay.deref_mut();

    // Blocks
    for set in level.pop_recording(default()) {
        replay.block(set.pos, set.block, set.nbt);
    }
    // New villagers
    for (id, pos, vill, is_trader) in &new_vills {
        let biome = level.biome[pos.block().truncate()];
        // Display random profession since most aren't used yet
        let profession = rand_weighted(&[
            (5., "none"),
            (1., "armorer"),
            (1., "butcher"),
            (1., "cartographer"),
            (1., "cleric"),
            (1., "farmer"),
            (1., "fisherman"),
            (1., "fletcher"),
            (1., "leatherworker"),
            (1., "librarian"),
            (1., "shepherd"),
            (1., "toolsmith"),
            (1., "weaponsmith"),
        ]);
        replay.command(format!(
            "summon {} {} {} {} {{{}, NoAI:1, Invulnerable:1, VillagerData:{{type:\"{}\",profession:\"{}\"}}, Tags: [sim_{}_villager]}}",
            if is_trader {
                "wandering_trader"
            } else {
                "villager"
            },
            pos.x, pos.z, pos.y,
            id.snbt(),
            biome.villager_type(),
            profession,
            invocation()
        ));

        replay.command(format!(
            // TODO: Use block display?
            "summon armor_stand {} {} {} {{{}, Invulnerable:1, Invisible:1, NoGravity:1, Tags:[\"carry\"]}}",
            pos.x, pos.z + 0.8, pos.y,
            vill.carry_id.snbt(),
        ));
        replay.carry_ids.push((*id, vill.carry_id));
    }
    // Names
    for (id, name) in &named {
        replay.command(format!(
            "data modify entity {id} CustomName set value \"{{\\\"text\\\":\\\"{}\\\"}}\"",
            name.0
        ));
    }
    // Movement
    for (id, pos, mut prev, in_boat) in &mut moved {
        let delta = pos.0 - prev.0;
        let facing = pos.0 + delta;
        if let Some(in_boat) = in_boat {
            let off = vec3(0., 0., -0.48);
            // Unfortunately the boat lags behind (visually only)
            // TODO: use /ride instead
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
                    .good
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
    for id in &lumberjacks {
        replay.command(format!(
            "data modify entity {id} VillagerDate.profession set value \"nitwit\"",
        ));
    }
    for id in &masons {
        replay.command(format!(
            "data modify entity {id} VillagerDate.profession set value \"mason\"",
        ));
    }

    replay.tick();
}

// TODO: Play at a higher speed/pitch when replay faster?
pub fn playsound(sound: &str, pos: IVec3) -> String {
    format!("playsound {sound} ambient @a {} {} {}", pos.x, pos.z, pos.y)
}
