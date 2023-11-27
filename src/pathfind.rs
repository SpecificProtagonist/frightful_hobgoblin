use std::{
    cmp::Ordering,
    collections::{BinaryHeap, VecDeque},
};

use crate::*;

#[derive(Eq, PartialEq)]
struct Node {
    pos: IVec3,
    cost: u32,
    cost_with_heuristic: u32,
    /// Allow but disincentivice steep paths
    stair_cooldown: i8,
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({} {} {})", self.pos.x, self.pos.y, self.pos.z)
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost_with_heuristic.cmp(&self.cost_with_heuristic)
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

const BASE_COST_PER_BLOCK: u32 = 3;
const STAIR_COOLDOWN: i8 = 7;

pub fn pathfind(
    level: &Level,
    mut start: IVec3,
    mut end: IVec3,
    range_to_end: i32,
) -> VecDeque<IVec3> {
    let area = level.area().shrink(2);
    if range_to_end == 0 {
        for pos in [&mut end, &mut start] {
            while level[*pos].solid() {
                *pos += IVec3::Z
            }
            while !level[*pos - IVec3::Z].solid() {
                *pos -= IVec3::Z
            }
        }
    }
    let mut closest_pos = start;
    let mut closest_heuristic = i32::MAX;
    // Can we use a vec instead? That means not checking if a position was already checked though
    let mut path = HashMap::<IVec3, IVec3>::default();
    let mut queue = BinaryHeap::new();
    queue.push(Node {
        pos: start,
        cost: 0,
        cost_with_heuristic: 0,
        stair_cooldown: 0,
    });
    while let Some(node) = queue.pop() {
        // TODO: Ladders
        for dir in HDir::ALL {
            let mut new_pos = node.pos.add(dir);
            // Only consider valid, novel paths
            if !area.contains(new_pos.truncate()) {
                continue;
            }
            if level[new_pos - IVec3::Z].cannot_walk_over() {
                continue;
            }
            let stairs_taken = if level[new_pos].solid() {
                if level[node.pos + IVec3::Z].solid() {
                    continue;
                }
                new_pos += IVec3::Z;
                true
            } else if !level[new_pos - IVec3::Z].solid() {
                if level[node.pos + IVec3::Z].solid() {
                    continue;
                }
                new_pos -= IVec3::Z;
                true
            } else {
                false
            };
            if !level[new_pos - IVec3::Z].solid()
                | level[new_pos].solid()
                | level[new_pos + IVec3::Z].solid()
            {
                continue;
            }
            if path.contains_key(&new_pos) {
                continue;
            }

            // Ok, new path to explore
            path.insert(new_pos, node.pos);

            let horizontal_diff = (new_pos.x - end.x).abs() + (new_pos.y - end.y).abs();
            let heuristic = horizontal_diff.max((new_pos.z - end.z).abs());
            let new_cost = node.cost
                + BASE_COST_PER_BLOCK
                + if stairs_taken {
                    node.stair_cooldown as u32
                } else {
                    0
                };
            queue.push(Node {
                pos: new_pos,
                cost: new_cost,
                cost_with_heuristic: new_cost + heuristic as u32 * BASE_COST_PER_BLOCK,
                stair_cooldown: if stairs_taken {
                    STAIR_COOLDOWN
                } else {
                    (node.stair_cooldown - 1).max(0)
                },
            });

            if heuristic < closest_heuristic {
                closest_heuristic = heuristic;
                closest_pos = new_pos;
            }

            // Can be reduced for performance
            //
            let exploration_limit_reached = path.len() > 8000;

            // Arrived at target
            if (heuristic <= range_to_end) | exploration_limit_reached {
                break;
            }
        }
    }

    let mut steps = VecDeque::with_capacity(100);
    steps.push_front(closest_pos);
    let mut prev = closest_pos;
    while let Some(&next) = path.get(&prev) {
        steps.push_front(next);
        if next == start {
            break;
        }
        prev = next;
    }
    steps
}
