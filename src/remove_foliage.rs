use crate::*;

pub fn ground(level: &mut Level, area: Rect) {
    for column in area {
        let base_height = if let Some(water_height) = level.water[column] {
            water_height
        } else {
            level.height[column]
        };
        for z in base_height + 1..=base_height + 2 {
            level(column, z, |block| {
                if matches!(block, GroundPlant(..)) {
                    Block::Air
                } else {
                    block
                }
            })
        }
    }
}
