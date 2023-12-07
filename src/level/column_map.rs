use crate::*;

pub struct ColumnMap<T> {
    chunk_min: ChunkIndex,
    chunk_max: ChunkIndex,
    resolution: i32,
    pub data: Vec<T>,
}

impl<T: Copy> ColumnMap<T> {
    pub fn new(chunk_min: ChunkIndex, chunk_max: ChunkIndex, resolution: i32, default: T) -> Self {
        let chunk_count = (chunk_max.0 - chunk_min.0 + 1) * (chunk_max.1 - chunk_min.1 + 1);
        Self {
            chunk_min,
            chunk_max,
            resolution,
            data: vec![default; (chunk_count * (16 / resolution) * (16 / resolution)) as usize],
        }
    }

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

impl<T: Copy> FnOnce<(IVec2,)> for ColumnMap<T> {
    type Output = T;

    extern "rust-call" fn call_once(self, column: (IVec2,)) -> Self::Output {
        self.call(column)
    }
}

impl<T: Copy> FnMut<(IVec2,)> for ColumnMap<T> {
    extern "rust-call" fn call_mut(&mut self, column: (IVec2,)) -> Self::Output {
        self.call(column)
    }
}

impl<T: Copy> Fn<(IVec2,)> for ColumnMap<T> {
    extern "rust-call" fn call(&self, (column,): (IVec2,)) -> Self::Output {
        self.data[self.column_index(column)]
    }
}

impl<T: Copy> FnOnce<(IVec2, T)> for ColumnMap<T> {
    type Output = ();

    extern "rust-call" fn call_once(mut self, args: (IVec2, T)) -> Self::Output {
        self.call_mut(args)
    }
}

impl<T: Copy> FnMut<(IVec2, T)> for ColumnMap<T> {
    extern "rust-call" fn call_mut(&mut self, (column, value): (IVec2, T)) -> Self::Output {
        let index = self.column_index(column);
        self.data[index] = value
    }
}
