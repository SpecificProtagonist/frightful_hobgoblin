#![allow(dead_code)]
use config::*;
use mc_gen::*;
use mc_gen::{house::house, sim::sim};
use rand::{thread_rng, Rng};

fn main() {
    drop(std::fs::remove_dir_all(SAVE_WRITE_PATH));
    copy_dir::copy_dir(SAVE_READ_PATH, SAVE_WRITE_PATH).expect("Failed to create save");

    let area = Rect::new_centered(ivec2(AREA[0], AREA[1]), ivec2(AREA[2], AREA[3]));

    let mut level = Level::new(SAVE_WRITE_PATH, area);

    let mut rng = thread_rng();
    for x in (0..140).step_by(14) {
        for y in (0..140).step_by(14) {
            let min = ivec3(x, y, 120);
            let max = min
                + ivec3(
                    rng.gen_range(5, 13),
                    rng.gen_range(5, 13),
                    rng.gen_range(5, 10),
                );
            house(&mut level, Cuboid { min, max });
        }
    }

    // remove_foliage::trees(&mut level, area);
    level.save();

    // sim(level, true);
}
