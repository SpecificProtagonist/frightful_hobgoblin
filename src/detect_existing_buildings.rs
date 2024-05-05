use crate::*;
use sim::*;

pub fn detect_existing_buildings(mut level: ResMut<Level>) {
    for column in level.area() {
        match level(level.ground(column)) {
            Full(
                SmoothStone
                | PolishedGranite
                | PolishedDiorite
                | PolishedAndesite
                | Wood(..)
                | Cobble
                | MossyCobble
                | StoneBrick
                | MossyStonebrick
                | Brick
                | SmoothSandstone
                | RedSandstone
                | SmoothRedSandstone
                | Blackstone
                | PolishedBlackstone
                | PolishedBlackstoneBrick
                | MudBrick,
            )
            | Wool(..)
            | Carpet(..)
            | Stair(..)
            | Slab(..)
            | Fence(..)
            | FenceGate(..)
            | Glass(..)
            | Hay
            | Rail(..) => {
                for x_off in -1..=1 {
                    for y_off in -1..=1 {
                        level.blocked[column + ivec2(x_off, y_off)] = Blocked
                    }
                }
            }
            Path => level.blocked[column] = Street,
            _ => (),
        }
    }
}
