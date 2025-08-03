#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
pub struct RegionPosition {
    pub x: i32,
    pub z: i32,
}

impl RegionPosition {
    pub fn new(x: i32, z: i32) -> RegionPosition {
        RegionPosition { x, z }
    }

    pub fn from_chunk_position(chunk_x: i32, chunk_z: i32) -> RegionPosition {
        let x = chunk_x >> 5;
        let z = chunk_z >> 5;

        RegionPosition::new(x, z)
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
pub struct RegionChunkPosition {
    pub x: u8,
    pub z: u8,
}

impl RegionChunkPosition {
    pub fn new(x: u8, z: u8) -> RegionChunkPosition {
        debug_assert!(32 > x, "Region chunk x coordinate out of bounds");
        debug_assert!(32 > z, "Region chunk z coordinate out of bounds");

        RegionChunkPosition { x, z }
    }

    pub fn from_chunk_position(chunk_x: i32, chunk_z: i32) -> RegionChunkPosition {
        let x = (chunk_x & 31) as u8;
        let z = (chunk_z & 31) as u8;

        RegionChunkPosition::new(x, z)
    }

    pub(crate) fn metadata_index(&self) -> usize {
        self.x as usize + self.z as usize * 32
    }
}
