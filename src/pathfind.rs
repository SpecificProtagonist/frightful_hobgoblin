use std::{cmp::Ordering, collections::BinaryHeap};

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

pub fn pathfind(level: &Level, mut start: IVec3, mut end: IVec3) -> Vec<IVec3> {
    for pos in [&mut start, &mut end] {
        while level[*pos].solid() {
            *pos += IVec3::Z
        }
        while !level[*pos - IVec3::Z].solid() {
            *pos -= IVec3::Z
        }
    }
    let mut path = HashMap::<IVec3, IVec3>::new();
    let mut queue = BinaryHeap::new();
    queue.push(Node {
        pos: end,
        dist: 0,
        with_heuristic: 0,
    });
    while let Some(node) = queue.pop() {
        for dir in HDir::ALL {
            let mut new_pos = node.pos.add(dir);
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

            let heuristic = (new_pos.x - start.x).abs()
                + (new_pos.y - start.y).abs()
                + ((new_pos.z - start.z) * (new_pos.z - node.pos.z)).clamp(0, 1);
            queue.push(Node {
                pos: new_pos,
                dist: node.dist + 1,
                with_heuristic: node.dist + 1 + heuristic as u32,
            });

            if new_pos == start {
                let mut steps = Vec::with_capacity((node.dist + 1) as usize);
                steps.push(start);
                let mut prev = start;
                loop {
                    let next = path[&prev];
                    steps.push(next);
                    if next == end {
                        break;
                    }
                    prev = next;
                }
                return steps;
            }
        }
        if path.len() > 20000 {
            break;
        }
    }
    todo!()
}
