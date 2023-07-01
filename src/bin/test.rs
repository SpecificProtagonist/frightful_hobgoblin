#![allow(dead_code)]
use config::*;
use mc_gen::*;
use structures::*;

fn main() {
    drop(std::fs::remove_dir_all(SAVE_WRITE_PATH));
    copy_dir::copy_dir(SAVE_READ_PATH, SAVE_WRITE_PATH).expect("Failed to create save");

    let area = Rect::new_centered(Column(AREA[0], AREA[1]), Vec2(AREA[2], AREA[3]));

    let mut world = World::new(SAVE_WRITE_PATH, area);

    for x in -150..150 {
        for z in -150..150 {
            world[Pos(x, 100, z)] = Block::SmoothQuartz;
        }
    }

    world.save().unwrap();
}

fn test_farms(world: &mut World) {
    let area = world.area().shrink(20);
    let mut fields = Vec::new();
    for x in (area.min.0..area.max.0).step_by(20) {
        for z in (area.min.1..area.max.1).step_by(20) {
            if let Some(blueprint) = farm::Blueprint::new(world, Column(x, z)) {
                fields.push(blueprint);
            }
        }
    }
    for field in &fields {
        field.render(world);
    }
    farm::make_hedge_edge(world, &fields);
}

fn test_retaining_wall(world: &mut World) {
    let height = world.height(Column(0, 0));
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
