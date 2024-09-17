use std::ops::{Index, IndexMut};

use crate::*;

// TODO: test with non-po2 resolution
pub struct ColumnMap<T, const RES: i32 = 1> {
    area: Rect,
    chunk_min: ChunkIndex,
    chunk_max: ChunkIndex,
    pub data: Vec<T>,
}

impl<T: Default + Clone, const RES: i32> ColumnMap<T, RES> {
    pub fn new(area: Rect) -> Self {
        Self::new_with(area, default())
    }
}

impl<T: Clone, const RES: i32> ColumnMap<T, RES> {
    pub fn new_with(area: Rect, default: T) -> Self {
        let chunk_min = ChunkIndex::from(area.min);
        let chunk_max = ChunkIndex::from(area.max);
        let chunk_count = (chunk_max.0 - chunk_min.0 + 1) * (chunk_max.1 - chunk_min.1 + 1);
        Self {
            area,
            chunk_min,
            chunk_max,
            data: vec![default; (chunk_count * (16 / RES) * (16 / RES)) as usize],
        }
    }
}

impl<T, const RES: i32> ColumnMap<T, RES> {
    pub fn area(&self) -> Rect {
        self.area
    }

    fn column_index(&self, column: IVec2) -> usize {
        self.chunk_index(column.into()) * (16 / RES as usize) * (16 / RES as usize)
            + (column.x.rem_euclid(16) / RES + column.y.rem_euclid(16) / RES * (16 / RES)) as usize
    }

    fn chunk_index(&self, chunk: ChunkIndex) -> usize {
        // Should this return the default value instead?
        if (chunk.0 < self.chunk_min.0)
            | (chunk.0 > self.chunk_max.0)
            | (chunk.1 < self.chunk_min.1)
            | (chunk.1 > self.chunk_max.1)
        {
            panic!("Out of bounds access to chunk {}, {}", chunk.0, chunk.1);
        } else {
            ((chunk.0 - self.chunk_min.0)
                + (chunk.1 - self.chunk_min.1) * (self.chunk_max.0 - self.chunk_min.0 + 1))
                as usize
        }
    }

    pub fn cells(&self) -> impl Iterator<Item = IVec2> {
        let (min, max) = (self.chunk_min, self.chunk_max);
        ((min.0 * 16)..=(max.0 * 16 + 15))
            .step_by(RES as usize)
            .flat_map(move |x| {
                ((min.1 * 16)..=(max.1 * 16 + 15))
                    .step_by(RES as usize)
                    .map(move |y| ivec2(x, y))
            })
    }
}

impl<const RES: i32> ColumnMap<i32, RES> {
    pub fn average(&self, area: impl IntoIterator<Item = IVec2>) -> f32 {
        let mut count = 0;
        let total: f32 = area
            .into_iter()
            .map(|p| {
                count += 1;
                self[p] as f32
            })
            .sum();
        total / count as f32
    }
}

impl<T: Copy, const RES: i32> Index<IVec2> for ColumnMap<T, RES> {
    type Output = T;

    fn index(&self, index: IVec2) -> &Self::Output {
        &self.data[self.column_index(index)]
    }
}

impl<T: Copy, const RES: i32> IndexMut<IVec2> for ColumnMap<T, RES> {
    fn index_mut(&mut self, index: IVec2) -> &mut Self::Output {
        let index = self.column_index(index);
        &mut self.data[index]
    }
}

impl<T: Copy, const RES: i32> Index<IVec3> for ColumnMap<T, RES> {
    type Output = T;

    fn index(&self, index: IVec3) -> &Self::Output {
        &self.data[self.column_index(index.truncate())]
    }
}

impl<T: Copy, const RES: i32> IndexMut<IVec3> for ColumnMap<T, RES> {
    fn index_mut(&mut self, index: IVec3) -> &mut Self::Output {
        let index = self.column_index(index.truncate());
        &mut self.data[index]
    }
}

impl<I, T, const RES: i32> FnOnce<(I, T)> for ColumnMap<T, RES>
where
    I: IntoIterator<Item = IVec2>,
    T: Copy,
{
    type Output = ();

    extern "rust-call" fn call_once(mut self, args: (I, T)) -> Self::Output {
        self.call_mut(args)
    }
}

impl<I, T, const RES: i32> FnMut<(I, T)> for ColumnMap<T, RES>
where
    I: IntoIterator<Item = IVec2>,
    T: Copy,
{
    extern "rust-call" fn call_mut(&mut self, (iter, value): (I, T)) -> Self::Output {
        for pos in iter {
            self[pos] = value
        }
    }
}
