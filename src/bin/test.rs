#![allow(dead_code)]
use mc_gen::*;
use structures::*;

fn main() {
    let tmp_world_load_path: &str = concat!(include_str!("../../save_path"), "mc-gen base");
    let tmp_world_save_path: &str = concat!(include_str!("../../save_path"), "mc-gen generated");
    let tmp_area = Rect {
        min: Column(-100, -100),
        max: Column(100, 100),
    };

    drop(std::fs::remove_dir_all(tmp_world_save_path));
    copy_dir::copy_dir(tmp_world_load_path, tmp_world_save_path).expect("Failed to create save");

    let mut world = World::new(tmp_world_save_path, tmp_area);

    //let villagers = test_fortified_house_animated(&mut world);
    test_fortified_house(&mut world);

    //save_behavior(&mut world, &villagers).unwrap();
    world.save().unwrap();
}

fn test_retaining_wall(world: &mut World) {
    let height = world.heightmap(Column(0, 0));
    let corners = vec![
        Column(22, -6),
        Column(18, 0),
        Column(15, 10),
        Column(18, 16),
        Column(16, 25),
        Column(-10, 30),
        Column(-10, -15),
    ];
    terraform::make_retaining_wall(world, &Polygon(corners), height, terraform::WallCrest::Wall)
}

fn test_fortified_house(world: &mut World) {
    let blueprints = castle::generate_blueprints(world);
    let mut blocked = Vec::new();
    for blueprint in &blueprints {
        if blocked.len() > 20 {
            break;
        }
        if blocked.iter().all(|rect| !blueprint.area.overlapps(*rect)) {
            blocked.push(blueprint.area);
            blueprint.build(world);
        }
    }
}

fn test_fortified_house_animated(world: &mut World) -> Vec<Villager> {
    let mut villagers = vec![];
    let blueprints = castle::generate_blueprints(world);
    for blueprint in blueprints {
        let mut view = BuildRecorder::new(world);
        blueprint.build(&mut view);
        villagers.push(Villager {
            name: format!("Test {}", villagers.len()),
            actions: vec![
                Action::Walk(vec![Column(0, 0), Column(0, 0)]),
                Action::Build(view.finish()),
            ],
        });
    }
    villagers
}
