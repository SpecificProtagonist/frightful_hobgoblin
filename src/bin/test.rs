use itertools::Itertools;
use std::time::Instant;

use mc_gen::*;

fn main() {
    // Temporary configuration TODO: parse arguments
    let tmp_replay_generation = true;
    let tmp_world_load_path: &str = concat!(include_str!("../../save_path"), "mc-gen base");
    let tmp_world_save_path: &str = concat!(include_str!("../../save_path"), "mc-gen generated");
    let tmp_area = Rect {
        min: Column(-100, -100),
        max: Column(100, 100),
    };

    drop(std::fs::remove_dir_all(tmp_world_save_path));
    copy_dir::copy_dir(tmp_world_load_path, tmp_world_save_path).expect("Failed to create save");

    let time_start = Instant::now();
    let mut world = World::new(tmp_world_save_path, tmp_area);
    let time_loaded = Instant::now();
    let villagers = generate(&mut world, tmp_area);
    if !tmp_replay_generation {
        apply_builds(&mut world, &villagers);
    }
    let time_generated = Instant::now();
    if tmp_replay_generation {
        save_behavior(&mut world, &villagers).expect("Failed to write mcfunctions");
    }
    let time_behavior_save = Instant::now();
    world.save().unwrap();
    let time_saved = Instant::now();

    println!(
        "Timings | load: {}s, generation: {}s, saving behavior: {}s, saving world: {}s",
        (time_loaded - time_start).as_secs_f64(),
        (time_generated - time_loaded).as_secs_f64(),
        (time_behavior_save - time_generated).as_secs_f64(),
        (time_saved - time_behavior_save).as_secs_f64()
    );

    debug_image::heightmap(&world).save("heightmap.png");
}

fn generate(world: &mut World, area: Rect) -> Vec<Villager> {
    // Temporary test function

    let mut actions = vec![];
    let mut tree_list = Vec::new();
    for x in -15..=15 {
        for z in -15..=15 {
            let pos = Pos(x, world.heightmap(Column(x, z)) + 1, z);
            if let Log(..) = world.get(pos) {
                tree_list.push(pos);
            }
        }
    }
    tree_list.sort_unstable_by_key(|pos| pos.0.abs() + pos.2.abs());

    let mut world = BuildRecorder::new(world);
    let start_pos = Pos(0, world.heightmap(Column(0, 0)) + 1, 0);
    for (start, tree_pos) in Some(start_pos)
        .iter()
        .chain(tree_list.iter())
        .tuple_windows()
    {
        fn dontstandinthetree(pos: Pos) -> Column {
            if pos.0.abs() > pos.2.abs() {
                Column(pos.0 - pos.0.signum(), pos.2)
            } else {
                Column(pos.0, pos.2 - pos.2.signum())
            }
        }
        let log_block = world.get(*tree_pos).clone();
        actions.push(Action::Walk(vec![
            dontstandinthetree(*start),
            dontstandinthetree(*tree_pos),
        ]));
        let mut view = BuildRecorder::new(&world);
        remove_foliage::remove_tree(&mut view, *tree_pos, false);
        let view = view.finish();
        view.apply_to(&mut world);
        actions.push(Action::Build(view));
        actions.push(Action::Pickup(log_block));
    }

    vec![Villager {
        name: "Rollo".into(),
        actions,
    }]
}
