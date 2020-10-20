use std::{
    path::*,
    fs::*,
    io::{self, *}
};
use itertools::Itertools;
use crate::world::*;
use crate::geometry::*;

/*
 *  Plan:
 *  - tag each villager with current path
 *  - one function for each path
 *  - scoreboard "timer" for time since start of path
 *  - every 5 or so ticks, for every villager:
 *    -  execute function for current path -> update Pos and Motion
 *    - if finished, set idle tag
 *    - increment "timer" for each villager
 *  - every 40 ticks or so, give @r[type=villager,tag=idle] a new task
 *  Potential performance improvement:
 *    the path functions could instead be implemented as command block chains
 *    with repeaters so only one function needs to be parsed/executed each update
 */
/*  Resulting file structure:
 *  data
 *    functions
 *      mc-gen
 *        loop.mcfunction
 *        update.mcfunction
 *        rollo
 *          on_idle.mcfunction
 *          path_0.mcfunction
 *          path_1.mcfunction
 *          ...
 *        torig
 *          on_idle.mcfunction
 *          path_0.mcfunction
 *          ...
 *        ...
 */


const TICKS_PER_UPDATE: i32 = 7;
// Villager movement speed in blocks/tick
const SPEED: f32 = 0.05f32;

pub fn save_behavior(world: &World) -> io::Result<()> {
    let mut fun_dir = PathBuf::from(&world.path);
    fun_dir.push("data/functions/mc-gen");
    create_dir_all(&fun_dir).expect("Failed to create function directory");

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

    // Function called every tick by gameLoopFunction
    {
        let mut loop_path = fun_dir.clone();
        loop_path.push("loop.mcfunction");
        let mut file = OpenOptions::new().write(true).create(true).open(loop_path)?;
        writeln!(file, "scoreboard players add 0-0-0-0-0 update_timer 1")?;
        writeln!(file, "function mc-gen:update if @e[type=armor_stand,score_update_timer_min={}]", TICKS_PER_UPDATE)?;
    }

    // Test data
    let id = 1;
    let path = &[
        Column(0,0),
        Column(20,0),
    ];

    
    {
        let mut update_path = fun_dir.clone();
        update_path.push("update.mcfunction");
        let mut file = OpenOptions::new().write(true).create(true).open(update_path)?;
        writeln!(file, "scoreboard players set 0-0-0-0-0 update_timer 0")?;
        writeln!(file, "scoreboard players add @e[type=villager] timer 1\n")?;
        writeln!(file, "execute 0-0-0-0-{0} 0 0 0 function mc-gen:villager_{0}/path_0", id)?;
    }



    {
        let mut fun_path = fun_dir.clone();
        fun_path.push(&format!("villager_{}", id));
        create_dir(&fun_path)?;
        fun_path.push("path_0.mcfunction");
        let mut file = OpenOptions::new().write(true).create(true).open(fun_path)?;

        let positions = positions(world, path);
        let motion_mod = 0.5; // Figure out right value!
        for (time, (curr, next)) in positions.iter().tuple_windows().enumerate() {
            writeln!(file, "entitydata @s[score_timer_min={0},score_timer={0}] {{Pos:[{1:.2},{2:.2},{3:.2}],Motion:[{4:.3},0.0,{5:.3}]}}",
                time,
                curr.0,
                curr.1,
                curr.2,
                (next.0-curr.0) * motion_mod,
                (next.2-curr.2) * motion_mod,
                // Villagers like looking around, which overrides Rotation after one tick (can only be disabled via NoAI)
                // ",Rotation:[{6:.1}f,{7:.1}f]"
                // (-next.0+curr.0).atan2(next.2-curr.2)/(2.0*std::f32::consts::PI) * 360.0,
                // (next.1 - curr.1) * -35.0
            )?;
        }
    }

    
    Ok(())
}

fn positions(world: &World, path: &[Column]) -> Vec<(f32, f32, f32)> {
    let step_length = SPEED * TICKS_PER_UPDATE as f32;
    let mut points = Vec::new();

    for (start, end) in path.iter().tuple_windows() {
        let offset = ((end.0 - start.0) as f32, (end.1 - start.1) as f32);
        let distance = (offset.0 * offset.0 + offset.1 * offset.1).sqrt();
        let direction = (offset.0/distance, offset.1/distance);
        for j in 0..(distance/step_length) as i32 {
            let offset_since_start = (direction.0*step_length * j as f32, direction.1*step_length as f32);
            let point_xz = (start.0 as f32 + offset_since_start.0, start.1 as f32 + offset_since_start.1);
            // Check the height at the villagers whole base, not just its center. Villager width is 0.6
            let mut height = 0.0f32;
            for corner in &[(-0.3,-0.3),(0.3,-0.3),(-0.3,0.3),(0.3,0.3)] {
                let column = Column((point_xz.0 + corner.0) as i32, (point_xz.1 + corner.1) as i32);
                height = height.max(
                    world.heightmap(column) as f32
                    + match world[column.at_height(world.heightmap(column) + 1)] {
                        Block::Slab {upper: false, ..} => 1.5,
                        _ => 1.0
                    }
                );
            }
            points.push((
                point_xz.0,
                height,
                point_xz.1,
            ));
        }
    }

    points
}
