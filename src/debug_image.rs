use crate::*;
use image::{Rgb, RgbImage};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Color {
    Ground,
    Water,
    Path,
    Building,
    Grey(u8),
}

pub struct MapImage {
    area: Rect,
    buffer: RgbImage,
}

impl MapImage {
    pub fn new(area: Rect) -> Self {
        Self {
            area,
            buffer: RgbImage::new(area.size().0 as u32 + 1, area.size().1 as u32 + 1),
        }
    }

    pub fn set(&mut self, column: Column, color: Color) {
        let pixel = column - self.area.min;
        self.buffer.put_pixel(
            pixel.0 as u32,
            pixel.1 as u32,
            match color {
                Color::Ground => Rgb([255, 255, 255]),
                Color::Water => Rgb([0, 0, 200]),
                Color::Path => Rgb([120, 120, 0]),
                Color::Building => Rgb([30, 20, 0]),
                Color::Grey(value) => Rgb([value, value, value]),
            },
        )
    }

    pub fn save(&self, filename: &str) {
        self.buffer.save(filename).unwrap();
    }
}

pub fn heightmap(world: &World) -> MapImage {
    heightmap_with(world, 60, 120)
}

pub fn heightmap_with(world: &World, min: i32, max: i32) -> MapImage {
    let mut image = MapImage::new(world.area());
    for column in world.area() {
        image.set(
            column,
            if world.water_level(column).is_some() {
                Color::Water
            } else {
                let height = world.height(column);
                Color::Grey((height.saturating_sub(min) as u32 * 256 / (max - min) as u32) as u8)
            },
        )
    }
    image
}
