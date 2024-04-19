mod biome;
mod block;
mod block_map;
mod column_map;
mod index_call;

use anvil_region::{
    position::{RegionChunkPosition, RegionPosition},
    provider::{FolderRegionProvider, RegionProvider},
};
use bevy_ecs::system::Resource;
use itertools::Itertools;
use nbt::CompoundTag;
use rayon::prelude::*;
use std::{
    ops::{Range, RangeInclusive, Shr},
    path::PathBuf,
};

use crate::{default, geometry::*, HashMap, DATA_VERSION};
pub use biome::*;
pub use block::*;
pub use column_map::ColumnMap;

use self::block_map::{BlockMap, Section};

#[derive(Resource)]
pub struct Level {
    pub path: PathBuf,
    /// Loaded area; aligned with chunk borders (-> usually larger than area specified in new())
    /// Both minimum and maximum inclusive
    chunk_min: ChunkIndex,
    chunk_max: ChunkIndex,
    blocks: BlockMap<Block>,
    /// Minecraft stores biomes in 3d, but we only store 2d (at height 64)
    pub biome: ColumnMap<Biome, 4>,
    pub height: ColumnMap<i32>,
    pub water: ColumnMap<Option<i32>>,
    // This might store a Option<Entity> later
    pub blocked: ColumnMap<bool>,
    // Pathfinding cost from center (may not be up to date)
    pub reachability: ColumnMap<u32>,
    dirty_chunks: ColumnMap<bool, 16>,
    setblock_recording: Vec<SetBlock>,
}

impl Level {
    // No nice error handling, but we don't really need that for just the three invocations
    pub fn new(read_path: &str, write_path: &str, area: Rect) -> Self {
        if read_path != write_path {
            let read_path = read_path.to_owned();
            let write_path = write_path.to_owned();
            rayon::spawn(move || {
                let _ = std::fs::remove_dir_all(&write_path);
                copy_dir::copy_dir(read_path, write_path).expect("Failed to create save");
            });
        }
        let region_path = {
            let mut region_path = PathBuf::from(read_path);
            region_path.push("region");
            region_path.into_os_string().into_string().unwrap()
        };
        let chunk_provider = FolderRegionProvider::new(&region_path);
        // TODO: use area just as a settlement area but try to load a wider margin around it (need to detect if chunks are present)
        let chunk_min = ChunkIndex::from(area.min - ivec2(crate::LOAD_MARGIN, crate::LOAD_MARGIN));
        let chunk_max = ChunkIndex::from(area.max + ivec2(crate::LOAD_MARGIN, crate::LOAD_MARGIN));

        let mut blocks = BlockMap::new(chunk_min, chunk_max);
        let mut biome = ColumnMap::new(chunk_min, chunk_max);
        let mut height = ColumnMap::new(chunk_min, chunk_max);
        let mut water = ColumnMap::new(chunk_min, chunk_max);

        // Load chunks. Collecting indexes to vec neccessary for zip
        (chunk_min.1..=chunk_max.1)
            .flat_map(|z| (chunk_min.0..=chunk_max.0).map(move |x| (x, z)))
            .collect_vec()
            .par_iter()
            .zip(blocks.sections.par_chunks_exact_mut(24))
            .zip(biome.data.par_chunks_exact_mut(4 * 4))
            .zip(height.data.par_chunks_exact_mut(16 * 16))
            .zip(water.data.par_chunks_exact_mut(16 * 16))
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
            path: PathBuf::from(write_path),
            chunk_min,
            chunk_max,
            blocks,
            biome,
            height,
            water,
            blocked: ColumnMap::new(chunk_min, chunk_max),
            reachability: ColumnMap::new(chunk_min, chunk_max),
            dirty_chunks: ColumnMap::new(chunk_min, chunk_max),
            setblock_recording: default(),
        }
    }

    /// Saves the world to disk. This is suitable only for debug visualizations:
    /// Some blocks may be changes/information is discarded even though it's not touched,
    /// blockstates ignore neighboring blocks.
    pub fn debug_save(&self) {
        // Write chunks
        let mut region_path = self.path.clone();
        region_path.push("region");
        // Internally, AnvilChunkProvider stores a path. So why require a str??
        let region_path = region_path.into_os_string().into_string().unwrap();
        let chunk_provider = FolderRegionProvider::new(&region_path);

        // Saving isn't thread safe
        for (index, sections) in (self.chunk_min.1..=self.chunk_max.1)
            .flat_map(|z| (self.chunk_min.0..=self.chunk_max.0).map(move |x| (x, z)))
            .zip(self.blocks.sections.chunks_exact(24))
        {
            if self.dirty_chunks[ChunkIndex::from(index).area().min] {
                save_chunk(&chunk_provider, index.into(), sections)
            }
        }

        self.save_metadata();
    }

    pub fn save_metadata(&self) {
        // Edit metadata
        let level_nbt_path =
            self.path.clone().into_os_string().into_string().unwrap() + "/level.dat";
        let mut file = std::fs::File::open(&level_nbt_path).expect("Failed to open level.dat");
        let mut nbt =
            nbt::decode::read_gzip_compound_tag(&mut file).expect("Failed to open level.dat");
        let data: &mut CompoundTag = nbt.get_mut("Data").expect("Corrupt level.dat");

        let name: &mut String = data.get_mut("LevelName").expect("Corrupt level.dat");
        // TODO: adjust if multiple invocations ("[2 settlements generated]")
        name.push_str(" [generated]");

        data.insert(
            "LastPlayed",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
        );

        data.insert_bool("allowCommands", true);

        data.insert_i8("Difficulty", 0);

        let gamerules: &mut CompoundTag = data.get_mut("GameRules").unwrap();
        gamerules.insert_str("commandBlockOutput", "false");

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(&level_nbt_path)
            .expect("Failed to open level.dat");
        nbt::encode::write_gzip_compound_tag(&mut file, &nbt).expect("Failed to write level.dat");
    }

    pub fn column_map<T: Clone, const RES: i32>(&self, default: T) -> ColumnMap<T, RES> {
        ColumnMap::new_with(self.chunk_min, self.chunk_max, default)
    }

    fn block_mut(&mut self, pos: IVec3) -> &mut Block {
        self.dirty_chunks[pos] = true;
        self.blocks.get_mut(pos)
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

    pub fn unblocked(&self, area: impl IntoIterator<Item = IVec2>) -> bool {
        area.into_iter()
            .all(|column| self.area().contains(column) && !self.blocked[column])
    }

    pub fn recording_cursor(&self) -> RecordingCursor {
        RecordingCursor(self.setblock_recording.len())
    }

    pub fn undo_recording(&mut self, cursor: RecordingCursor) -> Vec<SetBlock> {
        let rec = self.setblock_recording.drain(cursor.0..).collect_vec();
        for set in rec.iter().rev() {
            *self.block_mut(set.pos) = set.block
        }
        rec
    }

    pub fn pop_recording(
        &mut self,
        cursor: RecordingCursor,
    ) -> impl Iterator<Item = SetBlock> + '_ {
        self.setblock_recording.drain(cursor.0..)
    }

    pub fn get_recording(
        &mut self,
        cursor: RecordingCursor,
    ) -> impl Iterator<Item = SetBlock> + '_ {
        self.setblock_recording[cursor.0..].iter().copied()
    }

    pub fn fill(&mut self, iter: impl IntoIterator<Item = IVec3>, mut block: impl BlockOrFn) {
        for pos in iter {
            self(pos, |b| block.get(b));
        }
    }

    pub fn fill_at(
        &mut self,
        iter: impl IntoIterator<Item = IVec2>,
        z: impl RangeOrSingle,
        mut block: impl BlockOrFn,
    ) {
        for pos in iter {
            for z in z.start()..=z.end() {
                self(pos.extend(z), |b| block.get(b));
            }
        }
    }

    pub fn ground(&self, column: IVec2) -> IVec3 {
        column.extend(self.height[column])
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

pub trait BlockOrFn {
    fn get(&mut self, present: Block) -> Block;
}

impl BlockOrFn for Block {
    fn get(&mut self, _: Block) -> Block {
        *self
    }
}

impl<F: FnMut(Block) -> Block> BlockOrFn for F {
    fn get(&mut self, present: Block) -> Block {
        self(present)
    }
}

// TODO: load stored heightmaps, compare to found heightmaps to detect
// man-made structures
fn load_chunk(
    chunk_provider: &FolderRegionProvider,
    chunk_index: ChunkIndex,
    sections: &mut [Option<Box<Section<Block>>>],
    biomes: &mut [Biome],
    heightmap: &mut [i32],
    watermap: &mut [Option<i32>],
) -> Option<()> {
    let nbt = chunk_provider
        .get_region(RegionPosition::from_chunk_position(
            chunk_index.0,
            chunk_index.1,
        ))
        .ok()?
        .read_chunk(RegionChunkPosition::from_chunk_position(
            chunk_index.0,
            chunk_index.1,
        ))
        .ok()?;
    let version = nbt.get_i32("DataVersion").unwrap();
    if !(3465..=DATA_VERSION).contains(&version) {
        eprintln!(
            "Using version {}; only 1.20.2 is currently tested.",
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

        sections[(y_index + 4) as usize] = Some(Box::new([Air; 16 * 16 * 16]));
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
            section[i] = palette[index];

            current_bit_shift += bits_per_index;
            if current_bit_shift > (64 - bits_per_index) {
                current_bit_shift = 0;
                current_long += 1;
            }
        }
    }

    // Build water- & heightmap
    // There are build in heightmaps, but they don't ignore logs nor do they work on custom-made maps
    // TODO: Ignore (packed)ice
    for x in 0..16 {
        for z in 0..16 {
            'column: for section_index in (-4..20).rev() {
                if let Some(section) = &sections[(section_index + 4i32) as usize] {
                    for y in (0..16).rev() {
                        let block = &section[x + z * 16 + y as usize * 16 * 16];
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

    Some(())
}

fn bits_per_index(palette_len: usize) -> usize {
    palette_len.next_power_of_two().ilog2().max(4) as usize
}

fn save_chunk(
    chunk_provider: &FolderRegionProvider,
    index: ChunkIndex,
    sections: &[Option<Box<Section<Block>>>],
) {
    chunk_provider
        .get_region(RegionPosition::from_chunk_position(index.0, index.1))
        .unwrap()
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
                            let mut palette = HashMap::default();
                            block_states.insert_compound_tag_vec(
                                "palette",
                                Some(Air).iter().chain(section.iter()).flat_map(|block| {
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

                            for (i, block) in section.iter().enumerate() {
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
        .unwrap();
}

#[derive(Copy, Clone, Debug)]
pub struct SetBlock {
    pub pos: IVec3,
    pub block: Block,
    pub previous: Block,
}

#[derive(Default, Clone)]
pub struct RecordingCursor(usize);
