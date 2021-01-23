use mc_gen::*;

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
    test_retaining_wall(&mut world);
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
