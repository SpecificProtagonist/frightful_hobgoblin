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
    in_boat: bool,
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

const WALK_COST_PER_BLOCK: u32 = 3;
const BOATING_COST_PER_BLOCK: u32 = 2;
const STAIR_COOLDOWN: i8 = 7;
const BOAT_TOGGLE_COST: u32 = 40 * WALK_COST_PER_BLOCK;

#[derive(Debug)]
pub struct PathSearch {
    pub path: VecDeque<PathingNode>,
    pub success: bool,
    pub cost: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct PathingNode {
    pub pos: IVec3,
    pub boat: bool,
}

// TODO: Make walking on paths faster; make stairs reduce stair cost
// TODO: Acknowledge that boats are wider than one block
pub fn pathfind(level: &Level, mut start: IVec3, mut end: IVec3, range_to_end: i32) -> PathSearch {
    let area = level.area().shrink(2);
    if range_to_end == 0 {
        for pos in [&mut end, &mut start] {
            while level(*pos).solid() {
                *pos += IVec3::Z
            }
            while !level(*pos - IVec3::Z).solid() {
                *pos -= IVec3::Z
            }
        }
    }
    let mut closest_pos = start;
    let mut closest_heuristic = i32::MAX;
    let mut closest_cost = 0;
    let mut success = false;
    let mut path = HashMap::<IVec3, (IVec3, bool)>::default();
    let mut queue = BinaryHeap::new();
    queue.push(Node {
        pos: start,
        cost: 0,
        cost_with_heuristic: 0,
        stair_cooldown: 0,
        in_boat: false,
    });
    'outer: while let Some(node) = queue.pop() {
        for off in NEIGHBORS_3D {
            let Some(CheckedPos {
                new_pos,
                new_cost,
                boat,
                stairs_taken,
            }) = try_pos(level, area, &mut path, &node, off)
            else {
                continue;
            };

            let horizontal_diff = (new_pos.x - end.x).abs() + (new_pos.y - end.y).abs();
            let heuristic = horizontal_diff.max((new_pos.z - end.z).abs());
            queue.push(Node {
                pos: new_pos,
                cost: new_cost,
                cost_with_heuristic: new_cost
                    + heuristic as u32
                        * if boat {
                            BOATING_COST_PER_BLOCK
                        } else {
                            WALK_COST_PER_BLOCK
                        },
                stair_cooldown: if boat {
                    0
                } else if stairs_taken {
                    STAIR_COOLDOWN
                } else {
                    (node.stair_cooldown - 1).max(0)
                },
                in_boat: boat,
            });

            if heuristic < closest_heuristic {
                closest_heuristic = heuristic;
                closest_pos = new_pos;
                closest_cost = new_cost;
            }

            // Can be reduced for performance
            let exploration_limit_reached = path.len() > 8000;

            // Arrived at target
            if heuristic <= range_to_end {
                success = true;
                break 'outer;
            } else if exploration_limit_reached {
                break 'outer;
            }
        }
    }

    let mut steps = VecDeque::with_capacity(100);
    steps.push_front(PathingNode {
        pos: closest_pos,
        boat: false,
    });
    let mut prev = closest_pos;
    while let Some((next, boat)) = path.get(&prev) {
        steps.front_mut().unwrap().boat = *boat;
        steps.push_front(PathingNode {
            pos: *next,
            boat: false,
        });
        if *next == start {
            break;
        }
        prev = *next;
    }
    PathSearch {
        path: steps,
        success,
        cost: closest_cost,
    }
}

pub fn reachability_2d_from(level: &Level, start: IVec2) -> ColumnMap<u32> {
    let area = level.area().shrink(2);
    let mut path = HashMap::<IVec3, (IVec3, bool)>::default();
    // Merging this into path would be more perfomant
    let mut reachability = level.column_map(1, u32::MAX);
    let mut reach_z = level.column_map(1, i32::MIN);
    let mut queue = BinaryHeap::new();
    queue.push(Node {
        pos: level.ground(start) + IVec3::Z,
        cost: 0,
        cost_with_heuristic: 0,
        stair_cooldown: 0,
        in_boat: false,
    });
    while let Some(node) = queue.pop() {
        for off in NEIGHBORS_3D {
            let Some(CheckedPos {
                new_pos,
                new_cost,
                boat,
                stairs_taken,
            }) = try_pos(level, area, &mut path, &node, off)
            else {
                continue;
            };

            if reach_z[new_pos.truncate()] < new_pos.z {
                reach_z[new_pos.truncate()] = new_pos.z;
                reachability[new_pos.truncate()] = new_cost;
            }

            queue.push(Node {
                pos: new_pos,
                cost: new_cost,
                cost_with_heuristic: new_cost,
                stair_cooldown: if boat {
                    0
                } else if stairs_taken {
                    STAIR_COOLDOWN
                } else {
                    (node.stair_cooldown - 1).max(0)
                },
                in_boat: boat,
            });
        }
    }
    reachability
}

pub fn reachability_from(level: &Level, start: IVec3) -> HashMap<IVec3, u32> {
    let area = level.area().shrink(2);
    let mut path = HashMap::<IVec3, (IVec3, bool)>::default();
    // Merging this into path would be more perfomant
    let mut reachability = HashMap::<IVec3, u32>::default();
    let mut queue = BinaryHeap::new();
    queue.push(Node {
        pos: start,
        cost: 0,
        cost_with_heuristic: 0,
        stair_cooldown: 0,
        in_boat: false,
    });
    while let Some(node) = queue.pop() {
        for off in NEIGHBORS_3D {
            let Some(CheckedPos {
                new_pos,
                new_cost,
                boat,
                stairs_taken,
            }) = try_pos(level, area, &mut path, &node, off)
            else {
                continue;
            };

            reachability.insert(new_pos, new_cost);

            queue.push(Node {
                pos: new_pos,
                cost: new_cost,
                cost_with_heuristic: new_cost,
                stair_cooldown: if boat {
                    0
                } else if stairs_taken {
                    STAIR_COOLDOWN
                } else {
                    (node.stair_cooldown - 1).max(0)
                },
                in_boat: boat,
            });
        }
    }
    reachability
}

struct CheckedPos {
    new_pos: IVec3,
    new_cost: u32,
    boat: bool,
    stairs_taken: bool,
}

fn try_pos(
    level: &Level,
    area: Rect,
    path: &mut HashMap<IVec3, (IVec3, bool)>,
    node: &Node,
    off: IVec3,
) -> Option<CheckedPos> {
    let mut new_pos = node.pos + off;
    // Only consider valid, novel paths
    if !area.contains(new_pos.truncate()) {
        return None;
    }
    // Will we be in a boat in the new node?
    let boat = matches!(level(new_pos - IVec3::Z), Water);
    let mut stairs_taken = false;
    if boat {
        if off.z != 0 {
            return None;
        }
    } else {
        if off.z < 0 {
            // Ladder downwards taken
            if !level(new_pos).climbable() {
                return None;
            }
            stairs_taken = true;
        } else if off.z > 0 {
            // Ladder upwards taken
            if !level(node.pos).climbable() | level(node.pos + IVec3::Z * 2).solid() {
                return None;
            }
            stairs_taken = true;
        } else {
            if level(node.pos).climbable() {
            } else if level(new_pos).solid() {
                if level(node.pos + IVec3::Z).solid() {
                    return None;
                }
                new_pos += IVec3::Z;
                stairs_taken = true;
            } else if !level(new_pos - IVec3::Z).walkable() {
                if level(node.pos + IVec3::Z).solid() {
                    return None;
                }
                new_pos -= IVec3::Z;
                stairs_taken = true;
            }
        }
        // TODO: clean this up
        if level(new_pos - IVec3::Z).no_pathing() | level(new_pos).no_pathing() {
            return None;
        }
        if !level(new_pos - IVec3::Z).walkable() {
            return None;
        }
    };
    if level(new_pos).solid() | level(new_pos + IVec3::Z).solid() {
        return None;
    }
    if path.contains_key(&new_pos) {
        return None;
    }

    // Ok, new path to explore
    path.insert(new_pos, (node.pos, boat));

    let new_cost = node.cost
        + WALK_COST_PER_BLOCK
        + if stairs_taken {
            node.stair_cooldown as u32
        } else {
            0
        }
        + if boat != node.in_boat {
            BOAT_TOGGLE_COST
        } else {
            0
        };

    Some(CheckedPos {
        new_pos,
        new_cost,
        boat,
        stairs_taken,
    })
}
