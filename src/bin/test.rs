#![allow(dead_code)]
use config::*;
use mc_gen::sim::sim;
use mc_gen::*;
use roof::roof;

fn main() {
    drop(std::fs::remove_dir_all(SAVE_WRITE_PATH));
    copy_dir::copy_dir(SAVE_READ_PATH, SAVE_WRITE_PATH).expect("Failed to create save");

    let area = Rect::new_centered(ivec2(AREA[0], AREA[1]), ivec2(AREA[2], AREA[3]));

    let mut level = Level::new(SAVE_WRITE_PATH, area);

    let test_area = sim_anneal::choose_starting_area(&level);
    for pos in test_area {
        let pos = level.ground(pos);
        level[pos] = Wool(Magenta)
    }
    // for x in (0..180).step_by(18) {
    //     for y in (0..180).step_by(18) {
    //         roof(&mut level, ivec3(x, y, 130));
    //     }
    // }
    // roof(&mut level, ivec3(0, 0, 120));

    // remove_foliage::trees(&mut level, area);
    // level.save();

    sim(level, true);
}
