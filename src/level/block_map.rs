use crate::*;

pub type Section<T> = [T; 16 * 16 * 16];

pub struct BlockMap<T> {
    chunk_min: ChunkIndex,
    chunk_max: ChunkIndex,
    default: T,
    /// Sections in minecraft Z->X->Y order
    pub(super) sections: Vec<Option<Box<Section<T>>>>,
}

impl<T: Copy> BlockMap<T> {
    pub fn new(chunk_min: ChunkIndex, chunk_max: ChunkIndex, default: T) -> Self {
        let chunk_count =
            ((chunk_max.0 - chunk_min.0 + 1) * (chunk_max.1 - chunk_min.1 + 1)) as usize;
        Self {
            chunk_min,
            chunk_max,
            default,
            sections: vec![None; chunk_count * 24],
        }
    }

    fn block_in_section_index(pos: IVec3) -> usize {
        (pos.x.rem_euclid(16) + pos.y.rem_euclid(16) * 16 + pos.z.rem_euclid(16) * 16 * 16) as usize
    }

    fn section_index(&self, pos: IVec3) -> usize {
        self.chunk_index(pos.into()) * 24 + (pos.z / 16 + 4) as usize
    }

    // TODO: make fallible?
    fn chunk_index(&self, chunk: ChunkIndex) -> usize {
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

impl<T: Copy> std::ops::Index<IVec3> for BlockMap<T> {
    type Output = T;

    fn index(&self, pos: IVec3) -> &Self::Output {
        let index = self.section_index(pos);
        if let Some(section) = &self.sections[index] {
            &section[Self::block_in_section_index(pos)]
        } else {
            &self.default
        }
    }
}

impl<T: Copy> std::ops::IndexMut<IVec3> for BlockMap<T> {
    fn index_mut(&mut self, pos: IVec3) -> &mut Self::Output {
        let index = self.section_index(pos);
        let section =
            self.sections[index].get_or_insert_with(|| Box::new([self.default; 16 * 16 * 16]));
        &mut section[Self::block_in_section_index(pos)]
    }
}
