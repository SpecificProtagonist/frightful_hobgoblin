use std::fmt::Display;
use std::fs::{read, write};
use std::sync::atomic::{AtomicU32, Ordering};
use std::{fmt::Write, fs::create_dir_all, path::Path};

use crate::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;

pub fn sim(level: Level, save_sim: bool) {
    Id::load(&level.path);
    let mut world = World::new();
    world.init_resource::<Replay>();
    world.insert_resource(level);

    let pos = vec3(0., 0., 200.);
    world.spawn((
        Id::default(),
        Villager {
            carry: None,
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

fn test_walk(level: Res<Level>, mut query: Query<&mut Pos, With<Villager>>) {
    for mut pos in &mut query {
        pos.0 += vec3(0.05, 0.1, 0.0);
        set_walk_height(&level, &mut pos);
    }
}

fn tick_replay(
    mut level: ResMut<Level>,
    mut replay: ResMut<Replay>,
    new_vills: Query<(&Id, &Pos, &Villager), Added<Villager>>,
    _changed_vills: Query<(&Id, &Villager), Changed<Villager>>,
    mut moved: Query<(&Id, &Pos, &mut PrevPos), Changed<Pos>>,
) {
    for (pos, block) in level.pop_recording(default()) {
        replay.block(pos, block);
    }
    for (id, pos, _vill) in &new_vills {
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
        // TODO: carrying
    }
    for (id, pos, mut prev) in &mut moved {
        let facing = pos.0 * 2.0 - prev.0;
        replay.start_command();
        writeln!(
            replay.commands,
            "tp {} {} {} {} facing {} {} {}",
            id, pos.x, pos.z, pos.y, facing.x, facing.z, facing.y
        )
        .unwrap();
        // TODO: carrying
        prev.0 = pos.0;
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
    // TODO: split into two levels if this gets too big
    commands: String,
    block_cache: HashMap<Block, String>,
    unknown_blocks: UnknownBlocks,
}

impl Default for Replay {
    fn default() -> Self {
        Self {
            tick: 0,
            marker: default(),
            commands: default(),
            block_cache: default(),
            unknown_blocks: UNKNOWN_BLOCKS.read().unwrap().clone(),
        }
    }
}

impl Replay {
    fn start_command(&mut self) {
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
