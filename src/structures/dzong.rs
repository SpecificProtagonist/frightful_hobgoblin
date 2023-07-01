use crate::*;
use structures::Template;
use terraform::*;

pub fn test(world: &mut impl WorldView) {
    build_test(
        world,
        Rect::new_centered(world.area().center(), Vec2(11, 19)),
        true,
        true,
    )
    .apply_to(world);

    build_test(
        world,
        Rect::new_centered(world.area().center() + Vec2(20, 14), Vec2(11, 7)),
        false,
        true,
    )
    .apply_to(world);

    build_test(
        world,
        Rect::new_centered(world.area().center() + Vec2(-10, -35), Vec2(27, 14)),
        true,
        true,
    )
    .apply_to(world);
}

fn build_test(
    world: &impl WorldView,
    area: Rect,
    roof_layered: bool,
    roof_fancy_top: bool,
) -> BuildRecord {
    let mut world = BuildRecorder::new(world);

    remove_foliage::trees(&mut world, area.into_iter(), false);

    let base_y = world.height(area.center());
    let roof_y = base_y + 12;

    make_foundation_sloped(&mut world, area, base_y, Diorite);

    for y in base_y..roof_y {
        for col in area.border().chain(area.shrink(1).border()) {
            world.set(
                col.at(y),
                if y < roof_y - 4 {
                    SmoothQuartz
                } else {
                    Terracotta(Some(Red))
                },
            )
        }
    }

    roof(&mut world, roof_y, area, roof_layered, roof_fancy_top);

    dye_some_red_banners_orange(&mut world);

    world.finish()
}

fn dye_some_red_banners_orange(world: &mut BuildRecorder<impl WorldView>) {
    const ORANGE_BANNER_CHANCE: f32 = 0.22;
    let mut to_dye = Vec::new();
    for (pos, block) in world.iter() {
        if let WallBanner(facing, Red) = block {
            // Make sure that banners in pairs stay the same color
            if rand(ORANGE_BANNER_CHANCE)
                && !((*facing == HDir::XNeg)
                    & (matches!(world.get(*pos + Vec2(2, 0)), WallBanner(HDir::XPos, Red))))
                && !((*facing == HDir::ZNeg)
                    & (matches!(world.get(*pos + Vec2(0, 2)), WallBanner(HDir::ZPos, Red))))
            {
                to_dye.push((*pos, WallBanner(*facing, Orange)));
                if (*facing == HDir::XPos)
                    && matches!(world.get(*pos - Vec2(2, 0)), WallBanner(HDir::XNeg, Red))
                {
                    to_dye.push((*pos - Vec2(2, 0), WallBanner(HDir::XNeg, Orange)));
                }
                if (*facing == HDir::ZPos)
                    && matches!(world.get(*pos - Vec2(0, 2)), WallBanner(HDir::ZNeg, Red))
                {
                    to_dye.push((*pos - Vec2(0, 2), WallBanner(HDir::ZNeg, Orange)));
                }
            }
        }
    }

    for (pos, block) in to_dye {
        world.set(pos, block);
    }
}

fn roof(world: &mut impl WorldView, y: i32, area: Rect, layered: bool, fancy_top: bool) {
    fn roof_ring(world: &mut impl WorldView, y: i32, area: Rect, edge: bool, second_layer: bool) {
        let (start, check_end, corner, middle) = if edge {
            if second_layer {
                (
                    2,
                    0,
                    Template::get("dzong/roof_layered_corner_edge"),
                    Template::get("dzong/roof_layered_edge"),
                )
            } else {
                (
                    3,
                    0,
                    Template::get("dzong/roof_corner_edge"),
                    Template::get("dzong/roof_edge"),
                )
            }
        } else {
            (
                4,
                2,
                Template::get("dzong/roof_corner"),
                Template::get("dzong/roof"),
            )
        };

        // Prevent overlapp from opposite sides.
        // Should only be necessary for centermost layered
        let full_area = area.grow(3);
        let clip_area_xneg = Rect {
            min: full_area.min,
            max: Column(full_area.center().0, full_area.max.1),
        };
        let clip_area_zpos = Rect {
            min: Column(full_area.min.0, full_area.center().1),
            max: full_area.max,
        };
        let clip_area_xpos = Rect {
            min: Column(full_area.center().0, full_area.min.1),
            max: full_area.max,
        };
        let clip_area_zneg = Rect {
            min: full_area.min,
            max: Column(full_area.max.0, full_area.center().1),
        };

        // Main sections
        let mut column = area.min + Vec2(0, start);
        while column.1 + check_end < area.max.1 {
            middle.build_clipped(world, column.at(y), HDir::XPos, clip_area_xneg);
            if edge & !second_layer {
                for x in area.min.0 + 2..=area.max.0 - 2 {
                    world.set(Pos(x, y, column.1 - 2), Log(Crimson, LogType::FullBark));
                    world.set(Pos(x, y, column.1 + 2), Log(Crimson, LogType::FullBark));
                }
            }
            column += Vec2(0, 4);
        }
        let mut column = Column(area.min.0, area.max.1) + Vec2(start, 0);
        while column.0 + check_end < area.max.0 {
            middle.build_clipped(world, column.at(y), HDir::ZNeg, clip_area_zpos);
            if edge & !second_layer {
                for z in area.min.1 + 2..=area.max.1 - 2 {
                    world.set(Pos(column.0 - 2, y, z), Log(Crimson, LogType::FullBark));
                    world.set(Pos(column.0 + 2, y, z), Log(Crimson, LogType::FullBark));
                }
            }
            column += Vec2(4, 0);
        }
        let mut column = area.max - Vec2(0, start);
        while column.1 - check_end > area.min.1 {
            middle.build_clipped(world, column.at(y), HDir::XNeg, clip_area_xpos);
            column -= Vec2(0, 4);
        }
        let mut column = Column(area.max.0, area.min.1) - Vec2(start, 0);
        while column.0 - check_end > area.min.0 {
            middle.build_clipped(world, column.at(y), HDir::ZPos, clip_area_zneg);
            column -= Vec2(4, 0);
        }

        // Corners
        corner.build_clipped(
            world,
            Pos(area.min.0, y, area.min.1),
            HDir::XPos,
            clip_area_xneg.overlap(clip_area_zneg),
        );
        corner.build_clipped(
            world,
            Pos(area.min.0, y, area.max.1),
            HDir::ZNeg,
            clip_area_zpos.overlap(clip_area_xneg),
        );
        corner.build_clipped(
            world,
            Pos(area.max.0, y, area.max.1),
            HDir::XNeg,
            clip_area_xpos.overlap(clip_area_zpos),
        );
        corner.build_clipped(
            world,
            Pos(area.max.0, y, area.min.1),
            HDir::ZPos,
            clip_area_zneg.overlap(clip_area_xpos),
        );
    }

    fn roof_layer(world: &mut impl WorldView, y: i32, area: Rect, second_layer: bool) {
        roof_ring(world, y, area, true, second_layer);
        let mut y = if second_layer { y + 3 } else { y + 2 };
        let mut area = area.shrink(2);
        while area.max.0 > area.min.0 {
            roof_ring(world, y, area, false, second_layer);
            y += 2;
            area = area.shrink(4);
        }
    }

    fn roof_top(world: &mut impl WorldView, center: Pos, length: i32, dir: HDir) {
        let end = Template::get("dzong/roof_top_end");
        let middle = Template::get("dzong/roof_top_middle");
        let pillar = Template::get("dzong/roof_top_pillar");
        if matches!(dir, HDir::XNeg | HDir::XPos) {
            end.build(world, center + Vec2(length / 2, 0), HDir::XPos);
            end.build(world, center - Vec2(length / 2, 0), HDir::XNeg);
            for x_off in 0..=(length / 2 - 3) {
                let template = if (length / 2 - x_off) % 3 == 0 {
                    pillar
                } else {
                    middle
                };
                template.build(world, center + Vec2(x_off, 0), HDir::XPos);
                template.build(world, center - Vec2(x_off, 0), HDir::XPos);
            }
        } else {
            end.build(world, center + Vec2(0, length / 2), HDir::ZPos);
            end.build(world, center - Vec2(0, length / 2), HDir::ZNeg);
            for z_off in 0..=(length / 2 - 3) {
                let template = if (length / 2 - z_off) % 3 == 0 {
                    pillar
                } else {
                    middle
                };
                template.build(world, center + Vec2(0, z_off), HDir::ZPos);
                template.build(world, center - Vec2(0, z_off), HDir::ZPos);
            }
        }
    }

    roof_layer(world, y, area, false);

    if layered {
        roof_layer(world, y + 2, area.shrink(1), true);
    }

    if fancy_top {
        let size_difference = area.size().0 - area.size().1;
        let y = y + area.size().0.min(area.size().1) / 4 + if layered { 4 } else { 2 };

        if size_difference > 4 {
            roof_top(
                world,
                area.center().at(y),
                size_difference.abs(),
                if size_difference > 0 {
                    HDir::XPos
                } else {
                    HDir::ZPos
                },
            );
        } else {
            Template::get("dzong/roof_turret").build(world, area.center().at(y), HDir::XPos);
        }
    }
}
