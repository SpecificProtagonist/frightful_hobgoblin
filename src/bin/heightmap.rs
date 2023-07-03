use mc_gen::{config::*, *};

fn main() {
    let area = Rect::new_centered(Vec2(AREA[0], AREA[1]), Vec2(AREA[2], AREA[3]));

    let world = World::new(SAVE_WRITE_PATH, area);

    debug_image::heightmap(&world).save("heightmap.png");
}
