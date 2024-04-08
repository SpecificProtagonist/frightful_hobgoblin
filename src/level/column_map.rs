use std::ops::{Index, IndexMut};

use crate::*;

pub struct ColumnMap<T> {
    chunk_min: ChunkIndex,
    chunk_max: ChunkIndex,
    resolution: i32,
    pub data: Vec<T>,
}

impl<T: Default + Clone> ColumnMap<T> {
    pub fn new(chunk_min: ChunkIndex, chunk_max: ChunkIndex, resolution: i32) -> Self {
        Self::new_with(chunk_min, chunk_max, resolution, default())
    }
}

impl<T: Clone> ColumnMap<T> {
    pub fn new_with(
        chunk_min: ChunkIndex,
        chunk_max: ChunkIndex,
        resolution: i32,
        default: T,
    ) -> Self {
        let chunk_count = (chunk_max.0 - chunk_min.0 + 1) * (chunk_max.1 - chunk_min.1 + 1);
        Self {
            chunk_min,
            chunk_max,
            resolution,
            data: vec![default; (chunk_count * (16 / resolution) * (16 / resolution)) as usize],
        }
    }
}

impl<T> ColumnMap<T> {
    fn column_index(&self, column: IVec2) -> usize {
        self.chunk_index(column.into())
            * (16 / self.resolution as usize)
            * (16 / self.resolution as usize)
            + (column.x.rem_euclid(16) / self.resolution
                + column.y.rem_euclid(16) / self.resolution * (16 / self.resolution))
                as usize
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
}

impl<T: Copy> Index<IVec2> for ColumnMap<T> {
    type Output = T;

    fn index(&self, index: IVec2) -> &Self::Output {
        &self.data[self.column_index(index)]
    }
}

impl<T: Copy> IndexMut<IVec2> for ColumnMap<T> {
    fn index_mut(&mut self, index: IVec2) -> &mut Self::Output {
        let index = self.column_index(index);
        &mut self.data[index]
    }
}

impl<T: Copy> Index<IVec3> for ColumnMap<T> {
    type Output = T;

    fn index(&self, index: IVec3) -> &Self::Output {
        &self.data[self.column_index(index.truncate())]
    }
}

impl<T: Copy> IndexMut<IVec3> for ColumnMap<T> {
    fn index_mut(&mut self, index: IVec3) -> &mut Self::Output {
        let index = self.column_index(index.truncate());
        &mut self.data[index]
    }
}

impl<I, T> FnOnce<(I, T)> for ColumnMap<T>
where
    I: IntoIterator<Item = IVec2>,
    T: Copy,
{
    type Output = ();

    extern "rust-call" fn call_once(mut self, args: (I, T)) -> Self::Output {
        self.call_mut(args)
    }
}

impl<I, T> FnMut<(I, T)> for ColumnMap<T>
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
