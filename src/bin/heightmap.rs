use mc_gen::{config::*, debug_image::*, *};

fn main() {
    let area = Rect::new_centered(ivec2(AREA[0], AREA[1]), ivec2(AREA[2], AREA[3]));

    let level = Level::new(SAVE_READ_PATH, SAVE_WRITE_PATH, area);

    let mut map = MapImage::new(area);
    map.heightmap(&level);
    map.water(&level);
    map.save("heightmap.png");
}
