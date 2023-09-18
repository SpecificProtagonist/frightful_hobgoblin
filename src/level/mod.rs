mod biome;
mod block;
mod view;

use anvil_region::{
    position::{RegionChunkPosition, RegionPosition},
    provider::{FolderRegionProvider, RegionProvider},
};
use anyhow::{anyhow, Result};
use bevy_ecs::system::Resource;
use itertools::Itertools;
use nbt::CompoundTag;
use rayon::prelude::*;
use std::{
    ops::{Index, IndexMut, Range, RangeInclusive, Shr},
    path::PathBuf,
};

use crate::{default, geometry::*, HashMap, DATA_VERSION};
pub use biome::*;
pub use block::*;

#[derive(Resource)]
pub struct Level {
    pub path: PathBuf,
    /// Loaded area; aligned with chunk borders (-> usually larger than area specified in new())
    /// Both minimum and maximum inclusive
    chunk_min: ChunkIndex,
    chunk_max: ChunkIndex,
    /// Sections in Z->X->Y order
    sections: Vec<Option<Box<Section>>>,
    /// Minecraft stores biomes in 3d, but we only store 2d (at height 64)
    biome: Vec<Biome>,
    heightmap: Vec<i32>,
    watermap: Vec<Option<i32>>,
    dirty_chunks: Vec<bool>,
    setblock_recording: Vec<RecordSetBlock>,
}

impl Level {
    // No nice error handling, but we don't really need that for just the three invocations
    pub fn new(path: &str, area: Rect) -> Self {
        let region_path = {
            let mut region_path = PathBuf::from(path);
            region_path.push("region");
            region_path.into_os_string().into_string().unwrap()
        };
        let chunk_provider = FolderRegionProvider::new(&region_path);
        let chunk_min: ChunkIndex =
            (area.min - ivec2(crate::LOAD_MARGIN, crate::LOAD_MARGIN)).into();
        let chunk_max: ChunkIndex =
            (area.max + ivec2(crate::LOAD_MARGIN, crate::LOAD_MARGIN)).into();

        let chunk_count =
            ((chunk_max.0 - chunk_min.0 + 1) * (chunk_max.1 - chunk_min.1 + 1)) as usize;

        let mut sections = vec![None; chunk_count * 24];
        let mut biome = vec![Biome::Basic; chunk_count * 4 * 4];
        let mut heightmap = vec![0; chunk_count * 16 * 16];
        let mut watermap = vec![None; chunk_count * 16 * 16];

        // Load chunks. Collecting indexes to vec neccessary for zip
        (chunk_min.1..=chunk_max.1)
            .flat_map(|z| (chunk_min.0..=chunk_max.0).map(move |x| (x, z)))
            .collect_vec()
            .par_iter()
            .zip(sections.par_chunks_exact_mut(24))
            .zip(biome.par_chunks_exact_mut(4 * 4))
            .zip(heightmap.par_chunks_exact_mut(16 * 16))
            .zip(watermap.par_chunks_exact_mut(16 * 16))
            .for_each(|((((index, sections), biome), heightmap), watermap)| {
                load_chunk(
                    &chunk_provider,
                    (*index).into(),
                    sections,
                    biome,
                    heightmap,
                    watermap,
                )
                .expect(&format!("Failed to load chunk ({},{}): ", index.0, index.1))
            });

        Self {
            path: PathBuf::from(path),
            chunk_min,
            chunk_max,
            sections,
            biome,
            heightmap,
            watermap,
            dirty_chunks: vec![false; chunk_count],
            setblock_recording: default(),
        }
    }

    pub fn save(&self) {
        // Write chunks
        let mut region_path = self.path.clone();
        region_path.push("region");
        // Internally, AnvilChunkProvider stores a path. So why require a str??
        let region_path = region_path.into_os_string().into_string().unwrap();
        let chunk_provider = FolderRegionProvider::new(&region_path);

        // Saving isn't thread safe
        for ((index, sections), dirty) in (self.chunk_min.1..=self.chunk_max.1)
            .flat_map(|z| (self.chunk_min.0..=self.chunk_max.0).map(move |x| (x, z)))
            .zip(self.sections.chunks_exact(24))
            .zip(&self.dirty_chunks)
        {
            // Don't save outermost chunks, since we don't modify them & leaving out the border simplifies things
            if dirty
                & (index.0 > self.chunk_min.0)
                & (index.0 < self.chunk_max.0)
                & (index.1 > self.chunk_min.1)
                & (index.1 < self.chunk_max.1)
            {
                save_chunk(&chunk_provider, index.into(), sections)
                    .unwrap_or_else(|_| panic!("Failed to save chunk ({},{}): ", index.0, index.1))
            }
        }

        self.save_metadata().unwrap();
    }

    pub fn save_metadata(&self) -> Result<()> {
        // Edit metadata
        let level_nbt_path =
            self.path.clone().into_os_string().into_string().unwrap() + "/level.dat";
        let mut file = std::fs::File::open(&level_nbt_path).expect("Failed to open level.dat");
        let mut nbt =
            nbt::decode::read_gzip_compound_tag(&mut file).expect("Failed to open level.dat");
        let data: &mut CompoundTag = nbt.get_mut("Data").expect("Corrupt level.dat");

        let name: &mut String = data.get_mut("LevelName").expect("Corrupt level.dat");
        name.push_str(" [generated]");

        data.insert(
            "LastPlayed",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
        );

        data.insert_i8("Difficulty", 0);

        let gamerules: &mut CompoundTag = data.get_mut("GameRules").unwrap();
        gamerules.insert_str("commandBlockOutput", "false");
        gamerules.insert_str("gameLoopFunction", "mc-gen:loop");

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(&level_nbt_path)
            .expect("Failed to open level.dat");
        nbt::encode::write_gzip_compound_tag(&mut file, &nbt).expect("Failed to write level.dat");
        Ok(())
    }

    fn biome_index(&self, column: IVec2) -> usize {
        self.chunk_index(column.into()) * 4 * 4
            + column.y.rem_euclid(16) as usize / 4 * 4
            + column.x.rem_euclid(16) as usize / 4
    }

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

    fn section_index(&self, pos: IVec3) -> usize {
        self.chunk_index(pos.into()) * 24 + (pos.z / 16 + 4) as usize
    }

    fn column_index(&self, column: IVec2) -> usize {
        self.chunk_index(column.into()) * 16 * 16
            + (column.x.rem_euclid(16) + column.y.rem_euclid(16) * 16) as usize
    }

    fn block_in_section_index(pos: IVec3) -> usize {
        (pos.x.rem_euclid(16) + pos.y.rem_euclid(16) * 16 + pos.z.rem_euclid(16) * 16 * 16) as usize
    }

    pub fn chunk_min(&self) -> ChunkIndex {
        self.chunk_min
    }

    pub fn chunk_max(&self) -> ChunkIndex {
        self.chunk_max
    }

    pub fn chunks(&self) -> impl Iterator<Item = ChunkIndex> {
        (self.chunk_min.0..=self.chunk_max.0)
            .cartesian_product(self.chunk_min.1..=self.chunk_max.1)
            .map(|(x, y)| ChunkIndex(x, y))
    }

    pub fn area(&self) -> Rect {
        Rect {
            min: ivec2(self.chunk_min.0 * 16, self.chunk_min.1 * 16),
            max: ivec2(self.chunk_max.0 * 16 + 15, self.chunk_max.1 * 16 + 15),
        }
        .shrink(crate::LOAD_MARGIN)
    }

    pub fn biome(&self, column: IVec2) -> Biome {
        self.biome[self.biome_index(column)]
    }

    pub fn height(&self, column: IVec2) -> i32 {
        self.heightmap[self.column_index(column)]
    }

    pub fn height_mut(&mut self, column: IVec2) -> &mut i32 {
        let index = self.column_index(column);
        &mut self.heightmap[index]
    }

    pub fn water_level(&self, column: IVec2) -> Option<i32> {
        self.watermap[self.column_index(column)]
    }

    pub fn water_level_mut(&mut self, column: IVec2) -> &mut Option<i32> {
        let index = self.column_index(column);
        &mut self.watermap[index]
    }

    pub fn recording_cursor(&self) -> RecordingCursor {
        RecordingCursor(self.setblock_recording.len())
    }

    pub fn pop_recording(
        &mut self,
        cursor: RecordingCursor,
    ) -> impl Iterator<Item = SetBlock> + '_ {
        // Intermediate storage because of borrow conflict
        let rec = self.setblock_recording.drain(cursor.0..).collect_vec();
        rec.into_iter().filter_map(move |setblock| {
            let current = self[setblock.pos];
            (current != setblock.previous).then_some(SetBlock {
                pos: setblock.pos,
                block: current,
                previous: setblock.previous,
            })
        })
    }

    pub fn fill(&mut self, iter: impl IntoIterator<Item = IVec3>, block: Block) {
        for pos in iter {
            self[pos] = block;
        }
    }

    pub fn fill_at(
        &mut self,
        iter: impl IntoIterator<Item = IVec2>,
        z: impl RangeOrSingle,
        block: Block,
    ) {
        for pos in iter {
            for z in z.start()..=z.end() {
                self[pos.extend(z)] = block;
            }
        }
    }

    pub fn ground(&self, col: IVec2) -> IVec3 {
        col.extend(self.height(col))
    }

    pub fn average_height(&self, area: impl IntoIterator<Item = IVec2>) -> f32 {
        let mut count = 0;
        let total: f32 = area
            .into_iter()
            .map(|p| {
                count += 1;
                self.height(p) as f32
            })
            .sum();
        total / count as f32
    }
}

pub trait RangeOrSingle {
    fn start(&self) -> i32;
    fn end(&self) -> i32;
}

impl RangeOrSingle for Range<i32> {
    fn start(&self) -> i32 {
        self.start
    }

    fn end(&self) -> i32 {
        self.end - 1
    }
}

impl RangeOrSingle for RangeInclusive<i32> {
    fn start(&self) -> i32 {
        *self.start()
    }

    fn end(&self) -> i32 {
        *self.end()
    }
}

impl RangeOrSingle for i32 {
    fn start(&self) -> i32 {
        *self
    }

    fn end(&self) -> i32 {
        *self
    }
}

impl Index<IVec3> for Level {
    type Output = Block;

    fn index(&self, pos: IVec3) -> &Self::Output {
        if let Some(section) = &self.sections[self.section_index(pos)] {
            &section.blocks[Self::block_in_section_index(pos)]
        } else {
            &Block::Air
        }
    }
}

impl IndexMut<IVec3> for Level {
    fn index_mut(&mut self, pos: IVec3) -> &mut Self::Output {
        let chunk_index = self.chunk_index(pos.into());
        self.dirty_chunks[chunk_index] = true;
        let index = self.section_index(pos);
        let section = self.sections[index].get_or_insert_default();
        let block = &mut section.blocks[Self::block_in_section_index(pos)];
        self.setblock_recording.push(RecordSetBlock {
            pos,
            previous: *block,
        });
        block
    }
}

impl Index<Vec3> for Level {
    type Output = Block;

    fn index(&self, pos: Vec3) -> &Self::Output {
        &self[pos.block()]
    }
}

impl IndexMut<Vec3> for Level {
    fn index_mut(&mut self, pos: Vec3) -> &mut Self::Output {
        &mut self[pos.block()]
    }
}

// TODO: load stored heightmaps, compare to found heightmaps to detect
// man-made structures
fn load_chunk(
    chunk_provider: &FolderRegionProvider,
    chunk_index: ChunkIndex,
    sections: &mut [Option<Box<Section>>],
    biomes: &mut [Biome],
    heightmap: &mut [i32],
    watermap: &mut [Option<i32>],
) -> Result<()> {
    let nbt = chunk_provider
        .get_region(RegionPosition::from_chunk_position(
            chunk_index.0,
            chunk_index.1,
        ))?
        .read_chunk(RegionChunkPosition::from_chunk_position(
            chunk_index.0,
            chunk_index.1,
        ))
        .map_err(|_| anyhow!("Chunk read error"))?;
    let version = nbt.get_i32("DataVersion").unwrap();
    if (version > DATA_VERSION) | (version < 3465) {
        eprintln!(
            "Using version {}; only 1.20.1 is currently tested.",
            version
        );
    }

    // TODO: store CarvingMasks::AIR, seems useful
    // Also, check out Heightmaps. Maybe we can reuse them or gleam additional information from them

    let sections_nbt = nbt.get_compound_tag_vec("sections").unwrap();

    for section_nbt in sections_nbt {
        let y_index = section_nbt.get_i8("Y").unwrap();

        // Use a 2d representation of biomes
        if y_index == 5 {
            let biome = section_nbt.get_compound_tag("biomes").unwrap();
            let palette = biome.get_str_vec("palette").unwrap();
            let palette: Vec<Biome> = palette.iter().map(|n| Biome::from_id(n)).collect();
            if palette.len() == 1 {
                for biome in &mut *biomes {
                    *biome = palette[0];
                }
            } else {
                let bits_per_index = palette.len().next_power_of_two().ilog2();
                let Ok(indices) = biome.get_i64_vec("data") else {
                    continue;
                };

                let mut current_long = 0;
                let mut current_bit_shift = 0;
                for biome in &mut *biomes {
                    let packed = indices[current_long] as u64;
                    let index = packed.shr(current_bit_shift) as usize % (1 << bits_per_index);
                    *biome = palette[index];

                    current_bit_shift += bits_per_index;
                    if current_bit_shift > (64 - bits_per_index) {
                        current_bit_shift = 0;
                        current_long += 1;
                    }
                }
            }
        }

        let block_states = section_nbt.get_compound_tag("block_states").unwrap();
        let palette = block_states.get_compound_tag_vec("palette").unwrap();
        let palette: Vec<Block> = palette.iter().map(|nbt| Block::from_nbt(nbt)).collect();

        sections[(y_index + 4) as usize] = Some(Default::default());
        let section = sections[(y_index + 4) as usize].as_mut().unwrap();
        let Ok(indices) = block_states.get_i64_vec("data") else {
            continue;
        };
        let bits_per_index = bits_per_index(palette.len());

        let mut current_long = 0;
        let mut current_bit_shift = 0;
        for i in 0..(16 * 16 * 16) {
            let packed = indices[current_long] as u64;
            let index = packed.shr(current_bit_shift) as usize % (1 << bits_per_index);
            section.blocks[i] = palette[index];

            current_bit_shift += bits_per_index;
            if current_bit_shift > (64 - bits_per_index) {
                current_bit_shift = 0;
                current_long += 1;
            }
        }
    }

    // Build water- & heightmap
    // There are build in heightmaps, but they don't ignore logs nor do they work on custom-made maps
    for x in 0..16 {
        for z in 0..16 {
            'column: for section_index in (-4..20).rev() {
                if let Some(section) = &sections[(section_index + 4i32) as usize] {
                    for y in (0..16).rev() {
                        let block = &section.blocks[x + z * 16 + y as usize * 16 * 16];
                        let height = section_index * 16 + y;
                        if match block {
                            Block::Log(..) => false,
                            _ => block.solid(),
                        } {
                            heightmap[x + z * 16] = height;
                            break 'column;
                        } else if matches!(block, Block::Water /*TODO: | Block::Ice*/) {
                            watermap[x + z * 16].get_or_insert(section_index * 16 + y);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn bits_per_index(palette_len: usize) -> usize {
    palette_len.next_power_of_two().ilog2().max(4) as usize
}

fn save_chunk(
    chunk_provider: &FolderRegionProvider,
    index: ChunkIndex,
    sections: &[Option<Box<Section>>],
) -> Result<()> {
    chunk_provider
        .get_region(RegionPosition::from_chunk_position(index.0, index.1))?
        .write_chunk(
            RegionChunkPosition::from_chunk_position(index.0, index.1),
            {
                let mut nbt = CompoundTag::new();
                nbt.insert_i32("DataVersion", DATA_VERSION);
                nbt.insert_i32("xVec3", index.0);
                nbt.insert_i32("zVec3", index.1);

                nbt.insert_i64("LastUpdate", 0);
                nbt.insert_i8("TerrainPopulated", 1);
                nbt.insert_i64("InhabitetTime", 0);
                nbt.insert_str("Status", "full");

                // Collect tile entities
                let mut tile_entities = Vec::new();

                nbt.insert_compound_tag_vec("sections", {
                    sections
                        .iter()
                        .enumerate()
                        .filter_map(|(y_index, section)| {
                            let y_index = y_index as i32 - 4;
                            //https://github.com/rust-lang/rust-clippy/issues/8281
                            #[allow(clippy::question_mark)]
                            let Some(section) = section
                            else {
                                return None;
                            };
                            let mut nbt = CompoundTag::new();
                            nbt.insert_i8("Y", y_index as i8);

                            let mut block_states = CompoundTag::new();
                            // Build the palette first (for length)
                            // Minecraft seems to always have Air as id 0 even if there is none
                            let unknown_blocks = UNKNOWN_BLOCKS.read().unwrap();
                            let mut palette = HashMap::new();
                            block_states.insert_compound_tag_vec(
                                "palette",
                                Some(Air)
                                    .iter()
                                    .chain(section.blocks.iter())
                                    .flat_map(|block| {
                                        if !palette.contains_key(block) {
                                            palette.insert(block, palette.len());
                                            Some(block.to_nbt(&unknown_blocks))
                                        } else {
                                            None
                                        }
                                    }),
                            );

                            let bits_per_index = bits_per_index(palette.len());

                            // Reserve minimum required
                            let mut blocks = Vec::with_capacity(4096 / 64 * 4);
                            blocks.push(0);
                            let mut current_long = 0;
                            let mut current_bit_shift = 0;

                            for (i, block) in section.blocks.iter().enumerate() {
                                blocks[current_long] |=
                                    (palette[block] << current_bit_shift) as i64;
                                current_bit_shift += bits_per_index;
                                if current_bit_shift > 64 - bits_per_index {
                                    current_bit_shift = 0;
                                    current_long += 1;
                                    // If there's an unnecessary empty long at the end,
                                    // the chunk can't be loaded
                                    if (i < 4095) | (64 % bits_per_index != 0) {
                                        blocks.push(0);
                                    }
                                }

                                // Collect TileEntity data
                                {
                                    let section_base =
                                        ivec3(index.0 * 16, index.1 * 16, y_index * 16);
                                    let pos = section_base
                                        + ivec3(
                                            i as i32 % 16,
                                            i as i32 % (16 * 16) / 16,
                                            i as i32 / (16 * 16),
                                        );
                                    tile_entities.extend(block.tile_entity_nbt(pos));
                                }
                            }
                            block_states.insert_i64_vec("data", blocks);
                            nbt.insert("block_states", block_states);

                            Some(nbt)
                        })
                });

                nbt.insert_compound_tag_vec("block_entities", tile_entities);

                nbt
            },
        )
        .map_err(|_| anyhow!("Chunk write error"))?;
    Ok(())
}

#[derive(Clone)]
pub struct Section {
    blocks: [Block; 16 * 16 * 16],
}

impl Default for Section {
    fn default() -> Self {
        const AIR: Block = Block::Air;
        Section {
            blocks: [AIR; 16 * 16 * 16],
        }
    }
}

struct RecordSetBlock {
    pos: IVec3,
    previous: Block,
}

#[derive(Copy, Clone, Debug)]
pub struct SetBlock {
    pub pos: IVec3,
    pub block: Block,
    // TODO: This isn't accurate when the same block if overwritten multiple times (probably doesn't matter though)
    pub previous: Block,
}

#[derive(Default)]
pub struct RecordingCursor(usize);
