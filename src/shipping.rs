use std::collections::VecDeque;

use num_traits::FromPrimitive;

use crate::*;

pub fn identify_water_courses(mut level: ResMut<Level>) {
    let mut rivers = Vec::new();
    let mut claimed = level.column_map::<bool, 1>(false);
    let mut color = 0;
    for column in level.area() {
        if claimed[column] {
            continue;
        }
        if level.water[column].is_some() {
            color = (color + 1) % 16;
            let mut river = HashSet::default();
            let mut to_check = VecDeque::from(vec![column]);
            while let Some(column) = to_check.pop_front() {
                for off in NEIGHBORS_2D {
                    let next = column + off;
                    if level.area().contains(next) && !claimed[next] & level.water[next].is_some() {
                        river.insert(next);
                        claimed[next] = true;
                        to_check.push_back(next)
                    }
                }
                level(column.extend(80), Wool(Color::from_i32(color).unwrap()));
            }
            rivers.push(river);
        }
    }
}
