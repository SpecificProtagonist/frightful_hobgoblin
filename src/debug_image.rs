use crate::*;
use image::{Rgb, RgbImage};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Color {
    Ground,
    Water,
    Ocean,
    River,
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
                Color::Ground => Rgb([40, 140, 40]),
                Color::Water => Rgb([0, 0, 200]),
                Color::Ocean => Rgb([100, 000, 200]),
                Color::River => Rgb([100, 100, 255]),
                Color::Path => Rgb([120, 120, 0]),
                Color::Building => Rgb([30, 20, 0]),
                Color::Grey(value) => Rgb([value, value, value]),
            },
        )
    }

    pub fn save(&self, filename: &str) {
        self.buffer.save(filename).unwrap();
    }

    pub fn ocean_and_river(&mut self, level: &Level) {
        for column in self.area {
            match level.biome[column] {
                Biome::River => self.set(column, Color::River),
                Biome::Ocean => self.set(column, Color::Ocean),
                _ => (),
            }
        }
    }

    pub fn heightmap(&mut self, level: &Level) {
        self.heightmap_with(level, 60, 140)
    }

    pub fn heightmap_with(&mut self, level: &Level, min: i32, max: i32) {
        for column in self.area {
            self.set(column, {
                let height = level.height[column];
                Color::Grey(
                    (((height as f32 - min as f32) / (max as f32 - min as f32)).clamp(0., 255.)
                        * 255.) as u8,
                )
            })
        }
    }

    pub fn water(&mut self, level: &Level) {
        for column in self.area {
            if level.water[column].is_some() {
                self.set(column, Color::Water)
            }
        }
    }
}
