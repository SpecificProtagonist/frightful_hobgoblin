use crate::error::{ChunkReadError, ChunkWriteError};
use crate::position::{RegionChunkPosition, RegionPosition};
use bitvec::prelude::*;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use log::debug;
use nbt::decode::{read_gzip_compound_tag, read_zlib_compound_tag};
use nbt::encode::write_zlib_compound_tag;
use nbt::CompoundTag;
use std::io;
use std::io::{Cursor, Error, Read, Seek, SeekFrom, Write};
use std::time::{SystemTime, UNIX_EPOCH};

/// Amount of chunks in region.
const REGION_CHUNKS: usize = 1024;
/// Length of chunks metadata in region.
const REGION_CHUNKS_METADATA_LENGTH: usize = 2 * REGION_CHUNKS;
/// Region header length in bytes.
const REGION_HEADER_BYTES_LENGTH: u64 = 8 * REGION_CHUNKS as u64;
/// Region sector length in bytes.
const REGION_SECTOR_BYTES_LENGTH: u16 = 4096;
/// Maximum chunk length in bytes.
const CHUNK_MAXIMUM_BYTES_LENGTH: u32 = REGION_SECTOR_BYTES_LENGTH as u32 * 256;

/// Gzip compression type value.
const GZIP_COMPRESSION_TYPE: u8 = 1;
/// Zlib compression type value.
const ZLIB_COMPRESSION_TYPE: u8 = 2;

/// Region represents a 32x32 group of chunks.
pub struct Region<S> {
    /// Region position in the world.
    position: RegionPosition,
    /// Source in which region are stored.
    source: S,
    /// Array of chunks metadata.
    chunks_metadata: [ChunkMetadata; REGION_CHUNKS],
    /// Used sectors for chunks data.
    used_sectors: BitVec,
}

impl<S> Region<S> {
    /// Returns chunk metadata at specified coordinates.
    fn get_metadata(&self, position: &RegionChunkPosition) -> ChunkMetadata {
        self.chunks_metadata[position.metadata_index()]
    }
}

/// Calculates used sectors.
fn used_sectors(total_sectors: usize, chunks_metadata: &[ChunkMetadata]) -> BitVec {
    // First two sectors are used to store metadata.
    let mut used_sectors = bitvec![0; total_sectors];

    used_sectors.set(0, true);
    used_sectors.set(1, true);

    for metadata in chunks_metadata {
        if metadata.is_empty() {
            continue;
        }

        let start_index = metadata.start_sector_index as usize;
        let end_index = start_index + metadata.sectors as usize;

        for index in start_index..end_index {
            used_sectors.set(index, true);
        }
    }

    used_sectors
}

/// First 8KB of source are header of 1024 offsets and 1024 timestamps.
fn read_header<S: Read>(
    source: &mut S,
    source_len: u64,
) -> Result<[ChunkMetadata; REGION_CHUNKS], io::Error> {
    let mut chunks_metadata = [Default::default(); REGION_CHUNKS];

    if REGION_HEADER_BYTES_LENGTH > source_len {
        return Ok(chunks_metadata);
    }

    let mut values = [0u32; REGION_CHUNKS_METADATA_LENGTH];

    for index in 0..REGION_CHUNKS_METADATA_LENGTH {
        values[index] = source.read_u32::<BigEndian>()?;
    }

    for index in 0..REGION_CHUNKS {
        let last_modified_timestamp = values[REGION_CHUNKS + index];
        let offset = values[index];

        let start_sector_index = offset >> 8;
        let sectors = (offset & 0xFF) as u8;

        let metadata = ChunkMetadata::new(start_sector_index, sectors, last_modified_timestamp);
        chunks_metadata[index] = metadata;
    }

    return Ok(chunks_metadata);
}

impl<S: Read + Seek> Region<S> {
    pub fn load(position: RegionPosition, mut source: S) -> Result<Self, io::Error> {
        let source_len = source.len()?;
        let chunks_metadata = read_header(&mut source, source_len)?;

        let total_sectors = if source_len > REGION_HEADER_BYTES_LENGTH {
            (source_len as usize + (REGION_SECTOR_BYTES_LENGTH as usize - 1))
                / REGION_SECTOR_BYTES_LENGTH as usize
        } else {
            2
        };

        let used_sectors = used_sectors(total_sectors, &chunks_metadata);

        let region = Region {
            position,
            source,
            chunks_metadata,
            used_sectors,
        };

        Ok(region)
    }

    pub fn read_chunk(
        &mut self,
        position: RegionChunkPosition,
    ) -> Result<CompoundTag, ChunkReadError> {
        let metadata = self.get_metadata(&position);

        if metadata.is_empty() {
            return Err(ChunkReadError::ChunkNotFound { position });
        }

        let seek_offset = metadata.start_sector_index as u64 * REGION_SECTOR_BYTES_LENGTH as u64;
        let maximum_length = (metadata.sectors as u32 * REGION_SECTOR_BYTES_LENGTH as u32)
            .min(CHUNK_MAXIMUM_BYTES_LENGTH);

        self.source.seek(SeekFrom::Start(seek_offset))?;
        let length = self.source.read_u32::<BigEndian>()?;

        if length > maximum_length {
            return Err(ChunkReadError::LengthExceedsMaximum {
                length,
                maximum_length,
            });
        }

        let compression_scheme = self.source.read_u8()?;
        let mut compressed_buffer = vec![0u8; (length - 1) as usize];
        self.source.read_exact(&mut compressed_buffer)?;

        let mut cursor = Cursor::new(&compressed_buffer);

        match compression_scheme {
            GZIP_COMPRESSION_TYPE => Ok(read_gzip_compound_tag(&mut cursor)?),
            ZLIB_COMPRESSION_TYPE => Ok(read_zlib_compound_tag(&mut cursor)?),
            _ => Err(ChunkReadError::UnsupportedCompressionScheme { compression_scheme }),
        }
    }
}

impl<S: Write + Seek> Region<S> {
    pub fn write_chunk(
        &mut self,
        position: RegionChunkPosition,
        chunk_compound_tag: CompoundTag,
    ) -> Result<(), ChunkWriteError> {
        let mut buffer = Vec::new();

        // If necessary, extend the source length to the length of the header.
        if REGION_HEADER_BYTES_LENGTH > self.source.len()? {
            debug!(target: "anvil-region", "Extending source to header length");
            self.source.extend_len(REGION_HEADER_BYTES_LENGTH)?;
        }

        buffer.write_u8(ZLIB_COMPRESSION_TYPE)?;
        write_zlib_compound_tag(&mut buffer, &chunk_compound_tag)?;

        // 4 bytes for data length.
        let length = (buffer.len() + 4) as u32;

        if length > CHUNK_MAXIMUM_BYTES_LENGTH {
            return Err(ChunkWriteError::LengthExceedsMaximum { length });
        }

        let mut metadata = self.find_place(&position, length)?;
        let seek_offset = metadata.start_sector_index as u64 * REGION_SECTOR_BYTES_LENGTH as u64;

        self.source.seek(SeekFrom::Start(seek_offset))?;
        self.source.write_u32::<BigEndian>(buffer.len() as u32)?;
        self.source.write_all(&buffer)?;

        // Padding to align sector.
        let padding_len = REGION_SECTOR_BYTES_LENGTH - length as u16 % REGION_SECTOR_BYTES_LENGTH;

        if padding_len > 0 {
            self.source.write_all(&vec![0; padding_len as usize])?;
        }

        metadata.update_last_modified_timestamp();
        self.update_metadata(&position, metadata)?;

        Ok(())
    }

    /// Finds a place where chunk data of a given length can be put.
    ///
    /// If cannot find a place to put chunk data will extend source.
    fn find_place(
        &mut self,
        position: &RegionChunkPosition,
        chunk_length: u32,
    ) -> Result<ChunkMetadata, io::Error> {
        let sectors_required = (chunk_length / REGION_SECTOR_BYTES_LENGTH as u32) as u8 + 1;
        let metadata = self.get_metadata(position);

        // Chunk still fits in the old place.
        if metadata.sectors == sectors_required {
            debug!(
                target: "anvil-region",
                "Region x: {}, z: {} chunk x: {}, z: {} with length {} still fits in the old place",
                self.position.x, self.position.z, position.x, position.z, chunk_length
            );

            return Ok(metadata);
        }

        // Release previously used sectors.
        for i in 0..metadata.sectors {
            let sector_index = metadata.start_sector_index as usize + i as usize;
            self.used_sectors.set(sector_index, false);
        }

        let source_len = self.source.len()?;
        let total_sectors = source_len / REGION_SECTOR_BYTES_LENGTH as u64;

        // Trying to find enough big gap between sectors to put chunk.
        let mut sectors_free = 0;

        for sector_index in 0..total_sectors {
            // Sector occupied and we can't place chunk.
            if self.used_sectors[sector_index as usize] {
                sectors_free = 0;
                continue;
            }

            debug!(target: "anvil-region", "Sector {} is free", sector_index);
            sectors_free += 1;

            // Can put chunk in gap.
            if sectors_free == sectors_required {
                let put_sector_index = sector_index as u32 - sectors_free as u32 + 1;

                // Marking new sectors as used.
                for i in 0..sectors_free {
                    let sector_index = put_sector_index as usize + i as usize;
                    self.used_sectors.set(sector_index, true);
                }

                debug!(
                    target: "anvil-region",
                    "Region x: {}, z: {} chunk x: {}, z: {} with {} required sectors \
                    can be placed in free sectors gap between from {} to {}",
                    self.position.x,
                    self.position.z,
                    position.x,
                    position.z,
                    sectors_required,
                    put_sector_index,
                    sector_index
                );

                return Ok(ChunkMetadata::new(put_sector_index, sectors_required, 0));
            }
        }

        // Extending source because cannot find a place to put chunk data.
        let extend_sectors = sectors_required - sectors_free;
        let extend_len = (REGION_SECTOR_BYTES_LENGTH * extend_sectors as u16) as u64;

        debug!(
            target: "anvil-region",
            "Extending region x: {}, z: {} source for {} bytes to place chunk data",
            self.position.x,
            self.position.z,
            extend_len
        );

        self.source.extend_len(source_len + extend_len)?;

        // Mark new sectors as used.
        for _ in 0..extend_sectors {
            self.used_sectors.push(true);
        }

        return Ok(ChunkMetadata::new(
            total_sectors as u32 - sectors_free as u32,
            sectors_required,
            0,
        ));
    }

    /// Updates chunk metadata.
    fn update_metadata(
        &mut self,
        position: &RegionChunkPosition,
        metadata: ChunkMetadata,
    ) -> Result<(), io::Error> {
        let metadata_index = position.metadata_index();
        self.chunks_metadata[metadata_index] = metadata;

        let start_seek_offset = SeekFrom::Start((metadata_index * 4) as u64);
        let offset = (metadata.start_sector_index << 8) | metadata.sectors as u32;

        self.source.seek(start_seek_offset)?;
        self.source.write_u32::<BigEndian>(offset)?;

        let next_seek_offset = SeekFrom::Current(REGION_SECTOR_BYTES_LENGTH as i64 - 4);
        let last_modified_timestamp = metadata.last_modified_timestamp;

        self.source.seek(next_seek_offset)?;
        self.source
            .write_u32::<BigEndian>(last_modified_timestamp)?;

        Ok(())
    }
}

impl<S: Read + Seek> IntoIterator for Region<S> {
    type Item = <RegionIterator<S> as Iterator>::Item;
    type IntoIter = RegionIterator<S>;

    fn into_iter(self) -> Self::IntoIter {
        RegionIterator {
            inner: self,
            current: 0,
        }
    }
}

pub struct RegionIterator<S: Read + Seek> {
    inner: Region<S>,
    current: usize,
}

impl<S: Read + Seek> Iterator for RegionIterator<S> {
    type Item = CompoundTag;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == REGION_CHUNKS {
            return None;
        }

        let x = self.current % 32;
        let z = self.current / 32;

        self.current += 1;

        let pos = RegionChunkPosition::new(x as u8, z as u8);
        match self.inner.read_chunk(pos) {
            Ok(chunk) => Some(chunk),
            Err(_) => self.next(),
        }
    }
}

/// Chunk metadata are stored in header.
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
struct ChunkMetadata {
    /// Sector index from which starts chunk data.
    start_sector_index: u32,
    /// Amount of sectors used to store chunk.
    sectors: u8,
    /// Last time in seconds when chunk was modified.
    last_modified_timestamp: u32,
}

impl ChunkMetadata {
    fn new(start_sector_index: u32, sectors: u8, last_modified_timestamp: u32) -> Self {
        ChunkMetadata {
            start_sector_index,
            sectors,
            last_modified_timestamp,
        }
    }

    fn update_last_modified_timestamp(&mut self) {
        let system_time = SystemTime::now();
        let time = system_time.duration_since(UNIX_EPOCH).unwrap();

        self.last_modified_timestamp = time.as_secs() as u32
    }

    fn is_empty(&self) -> bool {
        self.sectors == 0
    }
}

/// Trait adds additional helper methods for `Seek`.
trait SeekExt {
    fn len(&mut self) -> Result<u64, io::Error>;
}

impl<S: Seek> SeekExt for S {
    fn len(&mut self) -> Result<u64, Error> {
        let old_pos = self.seek(SeekFrom::Current(0))?;
        self.seek(SeekFrom::Start(0))?;
        let len = self.seek(SeekFrom::End(0))?;

        if old_pos != len {
            self.seek(SeekFrom::Start(old_pos))?;
        }

        Ok(len)
    }
}

/// Trait adds additional helper methods for `Seek+Write`.
trait SeekWriteExt {
    fn extend_len(&mut self, new_len: u64) -> Result<(), io::Error>;
}

impl<S: Seek + Write> SeekWriteExt for S {
    fn extend_len(&mut self, new_len: u64) -> Result<(), Error> {
        let old_pos = self.seek(SeekFrom::Current(0))?;
        self.seek(SeekFrom::Start(0))?;
        let len = self.seek(SeekFrom::End(0))?;

        if new_len > len {
            let padding_len = new_len - len;
            self.write_all(&vec![0; padding_len as usize])?;
        }

        if old_pos != len {
            self.seek(SeekFrom::Start(old_pos))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::error::ChunkReadError;
    use crate::position::{RegionChunkPosition, RegionPosition};
    use crate::region;
    use crate::region::{
        read_header, ChunkMetadata, Region, SeekExt, SeekWriteExt, REGION_HEADER_BYTES_LENGTH,
        REGION_SECTOR_BYTES_LENGTH,
    };
    use nbt::CompoundTag;
    use std::fs::File;
    use std::io::Cursor;

    #[test]
    fn test_header_read() {
        let expected_data = vec![
            ChunkMetadata::new(61, 2, 1570215508),
            ChunkMetadata::new(102, 2, 1570215511),
            ChunkMetadata::new(177, 2, 1570215515),
            ChunkMetadata::new(265, 2, 1570215519),
            ChunkMetadata::new(56, 2, 1570215508),
        ];

        let file = File::open("test/region/r.0.0.mca").unwrap();
        let region = Region::load(RegionPosition::new(0, 0), file).unwrap();

        for (index, expected_chunk_metadata) in expected_data.iter().enumerate() {
            let chunk_metadata = region.chunks_metadata[256 + index];

            assert_eq!(&chunk_metadata, expected_chunk_metadata);
        }
    }

    #[test]
    fn test_read_chunk() {
        let file = File::open("test/region/r.0.0.mca").unwrap();
        let mut region = Region::load(RegionPosition::new(0, 0), file).unwrap();

        let compound_tag = region.read_chunk(RegionChunkPosition::new(15, 3)).unwrap();
        let level_tag = compound_tag.get_compound_tag("Level").unwrap();

        assert_eq!(level_tag.get_i32("xPos").unwrap(), 15);
        assert_eq!(level_tag.get_i32("zPos").unwrap(), 3);
    }

    #[test]
    fn test_read_chunk_not_found() {
        let file = File::open("test/empty_region.mca").unwrap();
        let mut region = Region::load(RegionPosition::new(0, 0), file).unwrap();

        let load_error = region
            .read_chunk(RegionChunkPosition::new(14, 12))
            .err()
            .unwrap();

        match load_error {
            ChunkReadError::ChunkNotFound { position } => {
                assert_eq!(position.x, 14);
                assert_eq!(position.z, 12);
            }
            _ => panic!("Expected `ChunkNotFound` but got `{:?}`", load_error),
        }
    }

    #[test]
    fn test_iterate_region() {
        let file = File::open("test/region/r.0.0.mca").unwrap();
        let region = Region::load(RegionPosition::new(0, 0), file).unwrap();

        let mut hit = false;

        for compound_tag in region.into_iter() {
            let level_tag = compound_tag.get_compound_tag("Level").unwrap();

            if level_tag.get_i32("xPos").unwrap() == 15 && level_tag.get_i32("zPos").unwrap() == 3 {
                hit = true;
            }
        }

        assert_eq!(hit, true);
    }

    #[test]
    fn test_iterate_region_not_found() {
        let file = File::open("test/region/r.0.0.mca").unwrap();
        let region = Region::load(RegionPosition::new(0, 0), file).unwrap();

        for compound_tag in region.into_iter() {
            let level_tag = compound_tag.get_compound_tag("Level").unwrap();

            if level_tag.get_i32("xPos").unwrap() == 28 && level_tag.get_i32("zPos").unwrap() == 1 {
                panic!("this chunk should not be hit")
            }
        }
    }

    #[test]
    fn test_iterate_region_empty() {
        let file = File::open("test/empty_region.mca").unwrap();
        let region = Region::load(RegionPosition::new(0, 0), file).unwrap();

        for _compound_tag in region {
            panic!("there should not be anything in there!")
        }
    }

    #[test]
    fn test_update_metadata() {
        let cursor = Cursor::new(vec![0; REGION_HEADER_BYTES_LENGTH as usize]);
        let mut region = Region::load(RegionPosition::new(1, 1), cursor).unwrap();

        let mut metadata = ChunkMetadata::new(500, 10, 0);
        metadata.update_last_modified_timestamp();

        let position = RegionChunkPosition::new(15, 15);

        region.update_metadata(&position, metadata).unwrap();

        // Reset current cursor position.
        region.source.set_position(0);

        let chunks_metadata = read_header(&mut region.source, REGION_HEADER_BYTES_LENGTH).unwrap();
        let metadata_index = position.metadata_index();

        // In memory metadata.
        assert_eq!(region.get_metadata(&position), metadata);
        // Written to file metadata.
        assert_eq!(chunks_metadata[metadata_index], metadata);
    }

    #[test]
    fn test_write_chunk_with_source_extend() {
        let cursor = Cursor::new(Vec::new());
        let mut region = Region::load(RegionPosition::new(1, 1), cursor).unwrap();

        let mut write_compound_tag = CompoundTag::new();
        write_compound_tag.insert_bool("test_bool", true);
        write_compound_tag.insert_str("test_str", "test");

        region
            .write_chunk(RegionChunkPosition::new(15, 15), write_compound_tag)
            .unwrap();

        assert_eq!(
            region.source.len().unwrap(),
            REGION_HEADER_BYTES_LENGTH + REGION_SECTOR_BYTES_LENGTH as u64
        );

        assert_eq!(region.used_sectors.len(), 3);

        let read_compound_tag = region.read_chunk(RegionChunkPosition::new(15, 15)).unwrap();

        assert!(read_compound_tag.get_bool("test_bool").unwrap());
        assert_eq!(read_compound_tag.get_str("test_str").unwrap(), "test");
    }

    #[test]
    fn test_write_chunk_same_sector() {
        let cursor = Cursor::new(Vec::new());
        let mut region = Region::load(RegionPosition::new(1, 1), cursor).unwrap();

        let mut write_compound_tag_1 = CompoundTag::new();
        write_compound_tag_1.insert_bool("test_bool", true);
        write_compound_tag_1.insert_str("test_str", "test");
        write_compound_tag_1.insert_f32("test_f32", 1.23);

        region
            .write_chunk(RegionChunkPosition::new(15, 15), write_compound_tag_1)
            .unwrap();

        let mut write_compound_tag_2 = CompoundTag::new();
        write_compound_tag_2.insert_bool("test_bool", true);
        write_compound_tag_2.insert_str("test_str", "test");

        region
            .write_chunk(RegionChunkPosition::new(15, 15), write_compound_tag_2)
            .unwrap();

        assert_eq!(
            region.source.len().unwrap(),
            REGION_HEADER_BYTES_LENGTH + REGION_SECTOR_BYTES_LENGTH as u64
        );

        assert_eq!(region.used_sectors.len(), 3);

        let read_compound_tag = region.read_chunk(RegionChunkPosition::new(15, 15)).unwrap();

        assert!(read_compound_tag.get_bool("test_bool").unwrap());
        assert_eq!(read_compound_tag.get_str("test_str").unwrap(), "test");
        assert!(!read_compound_tag.contains_key("test_f32"));
    }

    #[test]
    fn test_write_chunk_same_sector_with_source_expand() {
        let cursor = Cursor::new(Vec::new());
        let mut region = Region::load(RegionPosition::new(1, 1), cursor).unwrap();

        let mut write_compound_tag_1 = CompoundTag::new();
        write_compound_tag_1.insert_bool("test_bool", true);
        write_compound_tag_1.insert_str("test_str", "test");

        region
            .write_chunk(RegionChunkPosition::new(15, 15), write_compound_tag_1)
            .unwrap();

        let mut write_compound_tag_2 = CompoundTag::new();
        let mut i32_vec = Vec::new();

        // Extending chunk to second sector.
        // Due compression we need to write more than 1024 ints.
        for i in 0..3000 {
            i32_vec.push(i)
        }

        write_compound_tag_2.insert_i32_vec("test_i32_vec", i32_vec);

        region
            .write_chunk(RegionChunkPosition::new(15, 15), write_compound_tag_2)
            .unwrap();

        assert_eq!(
            region.source.len().unwrap(),
            REGION_HEADER_BYTES_LENGTH + REGION_SECTOR_BYTES_LENGTH as u64 * 2
        );

        assert_eq!(region.used_sectors.len(), 4);
    }

    #[test]
    fn test_write_chunk_with_insert_in_middle_gap() {
        let cursor = Cursor::new(Vec::new());
        let mut region = Region::load(RegionPosition::new(1, 1), cursor).unwrap();

        let mut write_compound_tag = CompoundTag::new();
        write_compound_tag.insert_bool("test_bool", true);
        write_compound_tag.insert_str("test_str", "test");

        // First two sectors are occupied by header.
        for _ in 0..3 {
            region.used_sectors.push(true);
        }

        region.used_sectors.set(2, false);

        let length = REGION_HEADER_BYTES_LENGTH + REGION_SECTOR_BYTES_LENGTH as u64 * 3;
        region.source.extend_len(length).unwrap();

        region
            .write_chunk(RegionChunkPosition::new(15, 15), write_compound_tag)
            .unwrap();

        for i in 0..5 {
            assert!(region.used_sectors.get(i).unwrap());
        }

        assert_eq!(region.source.len().unwrap(), length);
        assert_eq!(region.used_sectors.len(), 5);
    }

    #[test]
    fn test_write_chunk_not_enough_gap() {
        let cursor = Cursor::new(Vec::new());
        let mut region = Region::load(RegionPosition::new(1, 1), cursor).unwrap();

        let mut write_compound_tag_1 = CompoundTag::new();
        write_compound_tag_1.insert_bool("test_bool", true);
        write_compound_tag_1.insert_str("test_str", "test");

        region
            .write_chunk(
                RegionChunkPosition::new(15, 15),
                write_compound_tag_1.clone(),
            )
            .unwrap();

        region
            .write_chunk(RegionChunkPosition::new(0, 0), write_compound_tag_1)
            .unwrap();

        let mut write_compound_tag_2 = CompoundTag::new();
        let mut i32_vec = Vec::new();

        // Extending chunk to second sector.
        // Due compression we need to write more than 1024 ints.
        for i in 0..3000 {
            i32_vec.push(i)
        }

        write_compound_tag_2.insert_i32_vec("test_i32_vec", i32_vec);

        region
            .write_chunk(RegionChunkPosition::new(15, 15), write_compound_tag_2)
            .unwrap();

        assert_eq!(region.used_sectors.clone().into_vec()[0], 0b00111011);
        assert_eq!(region.used_sectors.len(), 6);
        assert_eq!(
            region.source.len().unwrap(),
            REGION_HEADER_BYTES_LENGTH + REGION_SECTOR_BYTES_LENGTH as u64 * 4
        );
    }

    #[test]
    fn test_used_sectors_only_header() {
        let empty_chunks_metadata = Vec::new();
        let used_sectors = region::used_sectors(8, &empty_chunks_metadata);

        // Two sectors are used for header data.
        assert_eq!(used_sectors.into_vec()[0], 0b00000011);
    }

    #[test]
    fn test_used_sectors_all() {
        let chunks_metadata = vec![ChunkMetadata::new(2, 6, 0)];
        let used_sectors = region::used_sectors(8, &chunks_metadata);

        assert_eq!(used_sectors.into_vec()[0], 0b11111111);
    }

    #[test]
    fn test_used_sectors_partially() {
        let chunks_metadata = vec![ChunkMetadata::new(3, 3, 0), ChunkMetadata::new(8, 1, 0)];

        let used_sectors = region::used_sectors(10, &chunks_metadata);
        let used_vec = used_sectors.into_vec();

        assert_eq!(used_vec[0], 0b100111011);
    }

    #[test]
    fn test_len() {
        let mut cursor = Cursor::new(vec![1, 2, 3, 4, 5]);
        let len = cursor.len().unwrap();

        assert_eq!(len, 5);
    }

    #[test]
    fn test_extend_len() {
        let mut cursor = Cursor::new(vec![1, 2, 3, 4, 5]);
        cursor.extend_len(10).unwrap();
        let len = cursor.len().unwrap();

        assert_eq!(len, 10);
    }
}
