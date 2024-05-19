use crate::*;
use sim::*;

use self::stall::MarketStall;

// TODO: walk around; turn chimney smoke on/off

/// Animations to be perpetually run after the replay is done
pub fn _animate(
    level: Res<Level>,
    houses: Query<&Pos, (With<House>, Without<Planned>)>,
    stalls: Query<&Pos, With<MarketStall>>,
) {
    for start in &houses {
        let mut prev_total = 0;
        let mut paths = Vec::new();
        for (weight, end) in houses
            .iter()
            .map(|p| (1, p))
            .chain(stalls.iter().map(|p| (4, p)))
        {
            if start == end {
                continue;
            }
            let path = pathfind(&level, start.block(), end.block(), 2);
            paths.push((prev_total..prev_total + weight, path));
            prev_total += weight;
        }
    }
}
