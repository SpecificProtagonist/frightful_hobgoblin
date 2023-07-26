use crate::*;

#[derive(Debug, Copy, Clone)]
pub enum DividerType {
    Hedge { small: bool },
    Fence(LineStyle),
    Wall(LineStyle),
}

pub fn make_divider_single(level: &mut Level, start: Vec2, end: Vec2, kind: DividerType) {
    match kind {
        DividerType::Hedge { small } => make_hedge(level, start, end, start, small),
        DividerType::Fence(style) => make_fence(
            level,
            start,
            end,
            start,
            Wood(level.biome(start).default_tree_species()),
            style,
        ),
        DividerType::Wall(style) => make_fence(level, start, end, start, Cobble, style),
    }
}

pub fn make_divider(
    level: &mut Level,
    mut segments: impl Iterator<Item = (Vec2, Vec2)>,
    kind: DividerType,
) {
    let mut make = |(start, end), prev| match kind {
        DividerType::Hedge { small } => make_hedge(level, start, end, prev, small),
        DividerType::Fence(style) => make_fence(
            level,
            start,
            end,
            prev,
            Wood(level.biome(start).default_tree_species()),
            style,
        ),
        DividerType::Wall(style) => make_fence(level, start, end, prev, Cobble, style),
    };
    if let Some(mut segment) = segments.next() {
        make(segment, segment.0);
        for next in segments {
            make(next, segment.1);
            segment = next;
        }
    }
}

fn make_hedge(level: &mut Level, start: Vec2, end: Vec2, prev: Vec2, small: bool) {
    // Maybe have the tree species be a parameter instead for consistency at biome borders?
    let leaf_block = Block::Leaves(level.biome(start).default_tree_species());
    let mut prev = prev.at(0);
    for column in ColumnLineIter::new(start, end, LineStyle::ThickWobbly) {
        let pos = column.at(level.height(column) + 1);
        level[pos] |= leaf_block;
        if prev.1 > pos.1 {
            level[pos + Vec3(0, 1, 0)] |= leaf_block;
        }
        if prev.1 < pos.1 {
            level[prev + Vec3(0, 1, 0)] |= leaf_block;
        }
        if !small {
            if rand(0.8) {
                level[pos + Vec3(0, 1, 0)] |= leaf_block;
            }
            if (prev.1 > pos.1) & rand(0.7) {
                level[pos + Vec3(0, 2, 0)] |= leaf_block;
            }
            if (prev.1 < pos.1) & rand(0.7) {
                level[prev + Vec3(0, 2, 0)] |= leaf_block;
            }
            let mut try_place = |col: Vec2| {
                let new_pos = col.at(level.height(col) + 1);
                if (new_pos.1 == pos.1) | (new_pos.1 == pos.1 + 1) {
                    level[new_pos] |= leaf_block;
                    true
                } else {
                    false
                }
            };
            if (prev.0 != column.0) & (prev.2 != column.1) {
                let placed = if rand(2.0 / 3.0) {
                    try_place(Vec2(prev.0, column.1))
                } else {
                    false
                };
                if !placed | rand(0.5) {
                    try_place(Vec2(column.0, prev.2));
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
// TODO: random mossyness?
fn make_fence(
    level: &mut Level,
    start: Vec2,
    end: Vec2,
    prev: Vec2,
    material: Material,
    style: LineStyle,
) {
    let mut prev = prev.at(level.height(prev) + 1);
    for column in ColumnLineIter::new(start, end, style) {
        let pos = column.at(level.height(column) + 1);
        level[pos] |= Fence(material);
        // Bride height variation (don't bother if too steep)
        if prev.1 == pos.1 + 1 {
            level[pos + Vec3(0, 1, 0)] |= Fence(material);
        }
        if prev.1 + 1 == pos.1 {
            level[prev + Vec3(0, 1, 0)] |= Fence(material);
        }
        prev = pos;
    }
}
