use mc_gen::{config::*, *};

fn main() {
    let area = Rect::new_centered(ivec2(AREA[0], AREA[1]), ivec2(AREA[2], AREA[3]));

    let level = Level::new(SAVE_WRITE_PATH, area);

    debug_image::heightmap(&level).save("heightmap.png");
}
