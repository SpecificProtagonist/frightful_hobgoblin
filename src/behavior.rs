use std::{num::NonZeroU32, fs::*, io::{self, *}, path::*};
use itertools::Itertools;
use nbt::CompoundTag;
use crate::*;
use Action::*;

pub struct Villager {
    pub name: String,
    pub actions: Vec<Action>,
    // TODO: repeated actions after village is build
}

pub enum Action {
    Pickup(Block),
    DropBlock,
    Walk(Vec<Column>), // TODO: store exact path (for walking on other stories, walking between heightmap updates, ...)
    Build(BuildRecord),
}

// Maybe add replay speed?
pub type Commands = Vec<String>;


const TICKS_PER_UPDATE: i32 = 7;
// Villager movement speed in blocks/tick
const SPEED: f32 = 0.08f32;

pub fn save_behavior(world: &mut World, villagers: &[Villager]) -> io::Result<()> {

    // Global scoreboard keeper
    world.entities.push(Entity {
        id: None,
        pos: world.area().center().at_height(0),
        data: EntityType::Marker,
        tags: vec!["global_scoreboard".into()]
    });

    // Create scoreboards
    {
        let mut scoreboard_path = PathBuf::from(&world.path);
        scoreboard_path.push("data/scoreboard.dat");
        // Todo: if there already is a scoreboard file, combine instead of overwriting it
        let mut file = OpenOptions::new().write(true).create(true).open(scoreboard_path)?;
        let mut nbt = CompoundTag::new();
        nbt.insert_compound_tag("data", {
            let mut data = CompoundTag::new();
            data.insert_compound_tag_vec("Objectives", vec![
                {
                    // Ticks since last update (global)
                    let mut objective = CompoundTag::new();
                    objective.insert_str("Name", "update_timer");
                    objective.insert_str("DisplayName", "update_timer");
                    objective.insert_str("CriteriaName", "dummy");
                    objective.insert_str("RenderType", "integer");
                    objective
                },{
                    // Time (in updates) since the villager has started its current task 
                    let mut objective = CompoundTag::new();
                    objective.insert_str("Name", "timer");
                    objective.insert_str("DisplayName", "timer");
                    objective.insert_str("CriteriaName", "dummy");
                    objective.insert_str("RenderType", "integer");
                    objective
                }
            ]);
            data.insert_compound_tag_vec("PlayerScores", vec![]);
            data.insert_compound_tag_vec("Teams", vec![]);
            data
        });
        nbt::encode::write_gzip_compound_tag(&mut file, nbt)?;
    }

    let mut functions = Vec::new();
    let mut command_blocks = Vec::new();

    // Function called every tick by gameLoopFunction
    let mut loop_fn = vec![
        "scoreboard players add @e[tag=global_scoreboard] update_timer 1".into(),
        format!("function mc-gen:update if @e[tag=global_scoreboard,score_update_timer_min={}]", TICKS_PER_UPDATE),
    ];

    let mut update_fn = vec![
        "scoreboard players set @e[tag=global_scoreboard] update_timer 0".into(),
        "scoreboard players add @e[type=villager] timer 1\n".into(),
    ];

    for (id, Villager { name, actions }) in villagers.iter().enumerate() {
        let id = id as u16;
        update_fn.push(format!(
            "execute 0-0-0-{}-0 0 0 0 \
             execute @s[tag=idle] 0 0 0 \
             function mc-gen:{}/on_idle", id, name));
        // For carried blocks
        loop_fn.push(format!(
            "execute 0-0-0-{0}-0 ~ ~ ~ \
             teleport 0-0-2-{0}-0 ~ ~0.6 ~", id));

        world.entities.push(Entity {
            id: Some(EntityID(0, 0, 0, id, 0)),
            pos: Pos(0,200,0),
            data: EntityType::Villager {
                name: name.to_owned(),
                biome: world.biome(Column(0,0)),
                profession: Profession::Leatherworker,
            },
            tags: vec!["idle".into()]
        });

        world.entities.push(Entity {
            id: Some(EntityID(0, 0, 2, id, 0)),
            pos: Pos(0,200,0),
            data: EntityType::Marker,
            tags: vec![]
        });

        // Right now all actions just get concatenated, this will change later
        let action_trigger_id = EntityID(0, 0, 1, id, 0);
        functions.push((
            format!("{}/on_idle", name),
            vec![
                "scoreboard players tag @s remove idle".into(),
                trigger_sequential(action_trigger_id)
            ]
        ));
        let mut commands = vec![];
        for (action_id, action) in actions.iter().enumerate() {
            commands.extend(match action {
                Pickup(block) => pickup(id, *block),
                DropBlock => drop_block(id),
                Walk(path) => walk(world, id, path),
                Build(recording) => recording.commands(),
            }.into_iter());
        }
        command_blocks.push((action_trigger_id, commands));
    }

    functions.push(("loop".into(), loop_fn));
    functions.push(("update".into(), update_fn));


    let mut fun_dir = PathBuf::from(&world.path);
    fun_dir.push("data/functions/mc-gen");
    export_parallel_executions(&fun_dir, functions)?;
    export_sequential_executions(world, command_blocks);
    
    Ok(())
}



fn pickup(id: u16, block: Block) -> Commands {
    // falling_block data changes aren't show till the player relogs
    // Therefore: use armor stands, teleported to villager in update.mcfunction
    vec![format!("entitydata {} {{ArmorItems:[{{}},{{}},{{}},{{Count:1,id:{},Damage:{}}}]}}", 
        EntityID(0,0,2,id,0), block.name(), block.to_bytes().1)]
}

fn drop_block(id: u16) -> Commands {
    vec![format!("entitydate {} {{ArmorItems:[{{}},{{}},{{}},{{}}]}}", EntityID(0,0,2,id,0))]
}

fn walk(world: &World, id: u16, path: &[Column]) -> Commands {
    let positions = positions(world, path);
    positions.iter().tuple_windows().map(|(curr, next)| {
        format!("tp 0-0-0-{}-0 {:.2} {:.2} {:.2} {:.1} {:.1}",
            id,
            curr.0,
            curr.1,
            curr.2,
            (-next.0+curr.0).atan2(next.2-curr.2)/(2.0*std::f32::consts::PI) * 360.0,
            (next.1 - curr.1) * -35.0
        )
    }).collect()
}

// Note: Colums refer to the center of the block, the returned values don't
// TODO: create MCPos type or some other name) to replace (f32, f32, f32)
fn positions(world: &World, path: &[Column]) -> Vec<(f32, f32, f32)> {
    let step_length = SPEED * 2f32;
    let mut points = Vec::new();

    for (start, end) in path.iter().tuple_windows() {
        let offset = ((end.0 - start.0) as f32, (end.1 - start.1) as f32);
        let distance = (offset.0 * offset.0 + offset.1 * offset.1).sqrt();
        let direction = (offset.0/distance, offset.1/distance);
        for j in 0..(distance/step_length) as i32 {
            let offset_since_start = (direction.0*step_length * j as f32, direction.1*step_length*j as f32);
            let point_xz = (start.0 as f32 + offset_since_start.0, start.1 as f32 + offset_since_start.1);
            // Check the height at the villagers whole base, not just its center. Villager width is 0.6
            let mut height = 0.0f32;
            for corner in &[(-0.3,-0.3),(0.3,-0.3),(-0.3,0.3),(0.3,0.3)] {
                let column = Column((point_xz.0 + corner.0) as i32, (point_xz.1 + corner.1) as i32);
                // TODO: fix this
                height = height.max(
                    world.heightmap(column) as f32
                    + match world.get(column.at_height(world.heightmap(column) + 1)) {
                        Block::Slab {upper: false, ..} => 1.5,
                        _ => 1.0
                    }
                );
            }
            points.push((
                point_xz.0 + 0.5,
                height,
                point_xz.1 + 0.5,
            ));
        }
    }

    points
}

fn trigger_parallel(name: &str) -> String {
    format!("function mc-gen:{}", name)
}

fn trigger_sequential(id: EntityID) -> String {
    format!("execute {} ~ ~ ~ setblock ~ ~ ~ redstone_block", id)
}

/// Creates functions for simultaneously executing commands
fn export_parallel_executions(fun_dir: &Path, functions: Vec<(String, Commands)>) -> io::Result<()> {
    for (name, commands) in functions {
        let mut fun_path = fun_dir.to_owned();
        fun_path.push(format!("{}.mcfunction", name));
        create_dir_all(&fun_path.parent().unwrap())?;
        let mut file = OpenOptions::new().write(true).create(true).open(fun_path)?;
        for command in commands {
            writeln!(file, "{}", command)?;
        }
    }

    Ok(())
}

/// Creates command blocks for sequentially executing commands
fn export_sequential_executions(world: &mut World, command_chains: Vec<(EntityID, Commands)>) {
    fn make_reset(world: &mut World, pos: Pos) {
        *world.get_mut(pos + Vec3(0,-1,0)) = CommandBlock;
        world.tile_entities.insert(pos + Vec3(0,-1,0), TileEntity::CommandBlock(
            "setblock ~ ~1 ~ stone".into()
        ));
    }
    let area = world.redstone_processing_area();
    let mut pos = area.min.at_height(1);
    for (marker_id, commands) in command_chains {
        let start = pos;
        make_reset(world, pos);
        pos.0 += 1;
        for command in commands {
            if pos.0 >= area.max.0 {
                let old_pos = pos;
                pos.2 += 2;
                pos.0 = area.min.0;
                if pos.2 > area.max.1 {
                    pos.2 = area.min.1;
                    pos.1 += 2;
                }
                // TODO: fix: triggering via redstone block takes a tick, so following timings can be of slightly
                *world.get_mut(old_pos) = CommandBlock;
                world.tile_entities.insert(old_pos, TileEntity::CommandBlock(
                    format!("setblock {} {} {} redstone_block", pos.0, pos.1, pos.2)
                ));
                make_reset(world, pos);
                pos.0 += 1;
            }
            *world.get_mut(pos) = Repeater(HDir::XPos, 0);
            // Encase in bedrock to prevent lava destroying redstone
            *world.get_mut(pos + Vec3(0, 1, 0)) = Bedrock;
            *world.get_mut(pos + Vec3(0, 0,-1)) = Bedrock;
            *world.get_mut(pos + Vec3(0, 0, 1)) = Bedrock;

            pos.0 += 1;
            *world.get_mut(pos) = CommandBlock;
            world.tile_entities.insert(pos, TileEntity::CommandBlock(command));
            pos.0 += 1;
        }

        world.entities.push(Entity {
            id: Some(marker_id),
            pos: start,
            data: EntityType::Marker,
            tags: vec![]
        });
    }
}