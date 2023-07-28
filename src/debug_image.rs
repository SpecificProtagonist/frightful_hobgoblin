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
            buffer: RgbImage::new(area.size().x as u32 + 1, area.size().y as u32 + 1),
        }
    }

    pub fn set(&mut self, column: IVec2, color: Color) {
        let pixel = column - self.area.min;
        self.buffer.put_pixel(
            pixel.x as u32,
            pixel.y as u32,
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

pub fn heightmap(level: &Level) -> MapImage {
    heightmap_with(level, 60, 140)
}

pub fn heightmap_with(level: &Level, min: i32, max: i32) -> MapImage {
    let mut image = MapImage::new(level.area());
    for column in level.area() {
        image.set(
            column,
            if level.water_level(column).is_some() {
                Color::Water
            } else {
                let height = level.height(column);
                Color::Grey(
                    (((height as f32 - min as f32) / (max as f32 - min as f32)).clamp(0., 255.)
                        * 255.) as u8,
                )
            },
        )
    }
    image
}
