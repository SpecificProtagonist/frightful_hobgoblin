use crate::geometry::*;
use crate::world::*;

#[derive(Debug, Copy, Clone)]
pub enum DividerType {
    Hedge { small: bool },
    Fence(LineStyle),
    Wall(LineStyle),
}

pub fn make_divider_single(
    world: &mut impl WorldView,
    start: Column,
    end: Column,
    kind: DividerType,
) {
    match kind {
        DividerType::Hedge { small } => make_hedge(world, start, end, start, small),
        DividerType::Fence(style) => make_fence(world, start, end, start, false, style),
        DividerType::Wall(style) => make_fence(world, start, end, start, true, style),
    }
}

pub fn make_divider(
    world: &mut impl WorldView,
    mut segments: impl Iterator<Item = (Column, Column)>,
    kind: DividerType,
) {
    let mut make = |(start, end), prev| match kind {
        DividerType::Hedge { small } => make_hedge(world, start, end, prev, small),
        DividerType::Fence(style) => make_fence(world, start, end, prev, false, style),
        DividerType::Wall(style) => make_fence(world, start, end, prev, true, style),
    };
    if let Some(mut segment) = segments.next() {
        make(segment, segment.0);
        while let Some(next) = segments.next() {
            make(next, segment.1);
            segment = next;
        }
    }
}

fn make_hedge(world: &mut impl WorldView, start: Column, end: Column, prev: Column, small: bool) {
    // Maybe have the tree species be a parameter instead for consistency at biome borders?
    let leaf_block = &Block::Leaves(world.biome(start).default_tree_species());
    let mut prev = prev.at_height(0);
    for column in ColumnLineIter::new(start, end, LineStyle::ThickWobbly) {
        let pos = column.at_height(world.heightmap(column) + 1);
        world.set_if_not_solid(pos, leaf_block);
        if prev.1 > pos.1 {
            world.set_if_not_solid(pos + Vec3(0, 1, 0), leaf_block);
        }
        if prev.1 < pos.1 {
            world.set_if_not_solid(prev + Vec3(0, 1, 0), leaf_block);
        }
        if !small {
            if rand(0.8) {
                world.set_if_not_solid(pos + Vec3(0, 1, 0), leaf_block);
            }
            if (prev.1 > pos.1) & rand(0.7) {
                world.set_if_not_solid(pos + Vec3(0, 2, 0), leaf_block);
            }
            if (prev.1 < pos.1) & rand(0.7) {
                world.set_if_not_solid(prev + Vec3(0, 2, 0), leaf_block);
            }
            let mut try_place = |col: Column| {
                let new_pos = col.at_height(world.heightmap(col) + 1);
                if (new_pos.1 == pos.1) | (new_pos.1 == pos.1 + 1) {
                    world.set_if_not_solid(new_pos, leaf_block);
                    true
                } else {
                    false
                }
            };
            if (prev.0 != column.0) & (prev.2 != column.1) {
                let placed = if rand(2.0 / 3.0) {
                    try_place(Column(prev.0, column.1))
                } else {
                    false
                };
                if !placed | rand(0.5) {
                    try_place(Column(column.0, prev.2));
                }
            } else if (column != start) & (column != end) {
                try_place(
                    column
                        + if rand(0.5) {
                            Vec2(rand_1(1.0), 0)
                        } else {
                            Vec2(0, rand_1(1.0))
                        },
                );
            }
        }
        prev = pos;
    }
}

// Even with gapless, there can be gaps if the terrain is too steep, but it looks a bit awkward otherwise
// Todo: Mobs can escape if inside is higher than ground under fence
// (best fix: raise ground in that case, but only in that case. If that's too hard, maybe just set chattep movement rate to 0)
fn make_fence<T: WorldView>(
    world: &mut T,
    start: Column,
    end: Column,
    prev: Column,
    stone: bool,
    style: LineStyle,
) {
    let wooden_fence_type = Fence::Wood(world.biome(start).default_tree_species());
    let place_fence_block = |world: &mut T, pos: Pos| {
        world.set_if_not_solid(
            pos,
            Block::Fence(if stone {
                // Todo: Maybe have mossyness depend on biome wetness (-> none in desert) (have this generally available)
                Fence::Stone { mossy: rand(0.3) }
            } else {
                wooden_fence_type
            }),
        )
    };

    let mut prev = prev.at_height(world.heightmap(prev) + 1);
    for column in ColumnLineIter::new(start, end, style) {
        let pos = column.at_height(world.heightmap(column) + 1);
        place_fence_block(world, pos);
        // Bride height variation (don't bother if too steep)
        if prev.1 == pos.1 + 1 {
            place_fence_block(world, pos + Vec3(0, 1, 0));
        }
        if prev.1 + 1 == pos.1 {
            place_fence_block(world, prev + Vec3(0, 1, 0));
        }
        prev = pos;
    }
}
