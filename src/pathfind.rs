use std::{
    cmp::Ordering,
    collections::{BinaryHeap, VecDeque},
};

use crate::*;

#[derive(Eq, PartialEq)]
struct Node {
    pos: IVec3,
    dist: u32,
    with_heuristic: u32,
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({} {} {})", self.pos.x, self.pos.y, self.pos.z)
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.with_heuristic.cmp(&self.with_heuristic)
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn pathfind(
    level: &Level,
    mut start: IVec3,
    mut end: IVec3,
    range_to_end: i32,
) -> VecDeque<IVec3> {
    let area = level.area().shrink(2);
    for pos in [&mut end, &mut start] {
        while level[*pos].solid() {
            *pos += IVec3::Z
        }
        while !level[*pos - IVec3::Z].solid() {
            *pos -= IVec3::Z
        }
    }
    let mut path = HashMap::<IVec3, IVec3>::default();
    let mut queue = BinaryHeap::new();
    queue.push(Node {
        pos: start,
        dist: 0,
        with_heuristic: 0,
    });
    while let Some(node) = queue.pop() {
        for dir in HDir::ALL {
            let mut new_pos = node.pos.add(dir);
            if !area.contains(new_pos.truncate()) {
                continue;
            }
            if level[node.pos.add(dir)].solid() {
                new_pos += IVec3::Z;
                if level[node.pos + IVec3::Z].solid() {
                    continue;
                }
            } else if !level[node.pos.add(dir) - IVec3::Z].solid() {
                if level[node.pos + IVec3::Z].solid() {
                    continue;
                }
                new_pos -= IVec3::Z;
            }
            if !level[new_pos - IVec3::Z].solid()
                | level[new_pos].solid()
                | level[new_pos + IVec3::Z].solid()
            {
                continue;
            }

            if path.contains_key(&new_pos) {
                continue;
            }
            path.insert(new_pos, node.pos);

            let heuristic = (new_pos.x - end.x).abs()
                + (new_pos.y - end.y).abs()
                + ((new_pos.z - end.z) * (new_pos.z - node.pos.z)).clamp(0, 1);
            queue.push(Node {
                pos: new_pos,
                dist: node.dist + 1,
                with_heuristic: node.dist + 1 + heuristic as u32,
            });

            // TMP
            let tmp_early_break = path.len() > 20000;

            if (heuristic <= range_to_end) | tmp_early_break {
                let mut steps = VecDeque::with_capacity((node.dist + 1) as usize);
                steps.push_front(end);
                let mut prev = end;
                while let Some(&next) = path.get(&prev) {
                    steps.push_front(next);
                    if next == start {
                        break;
                    }
                    prev = next;
                }
                return steps;
            }
        }
    }

    // Maybe: If a path hasn't been found (search time depends on distance + constant factor),
    // just go to the closest location and pretend that's enough?
    // todo!();
    vec![end].into()
}
