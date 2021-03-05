// TODO: check if empty sections really are (and stay) None

mod biome;
mod block;
mod entity;

use anvil_region::{
    position::{RegionChunkPosition, RegionPosition},
    provider::{FolderRegionProvider, RegionProvider},
};
use anyhow::Result;
use itertools::Itertools;
use nbt::CompoundTag;
use rayon::prelude::*;
use std::{collections::HashMap, ops::Shr, path::PathBuf};

use crate::geometry::*;
pub use biome::*;
pub use block::*;
pub use entity::*;

// Ugh, can't impl Index and IndexMut because of orphan rules
pub trait WorldView {
    fn get(&self, pos: Pos) -> &Block;
    fn get_mut(&mut self, pos: Pos) -> &mut Block;
    fn get_mut_no_update_order(&mut self, pos: Pos) -> &mut Block;

    fn biome(&self, column: Column) -> Biome;

    /// Height of the ground, ignores vegetation
    fn height(&self, column: Column) -> u8;
    fn height_mut(&mut self, column: Column) -> &mut u8;

    fn water_level(&self, column: Column) -> Option<u8>;
    fn water_level_mut(&mut self, column: Column) -> &mut Option<u8>;

    fn area(&self) -> Rect;

    /// Convenience method
    fn set(&mut self, pos: Pos, block: impl BlockOrRef) {
        *self.get_mut(pos) = block.get();
    }
    fn set_override(&mut self, pos: Pos, block: impl BlockOrRef) {
        *self.get_mut_no_update_order(pos) = block.get();
    }
    /// Convenience method
    fn set_if_not_solid<'a, 's>(&'s mut self, pos: Pos, block: impl BlockOrRef) {
        let block_ref = self.get_mut(pos);
        if !block_ref.solid() {
            *block_ref = block.get();
        }
    }
}

pub trait BlockOrRef {
    fn get(self) -> Block;
}

impl BlockOrRef for Block {
    fn get(self) -> Block {
        self
    }
}

impl BlockOrRef for &Block {
    fn get(self) -> Block {
        self.clone()
    }
}

// Maybe have a subworld not split into chunks for efficiency?
pub struct World {
    pub path: PathBuf,
    /// Loaded area; aligned with chunk borders (-> usually larger than area specified in new())
    /// Both minimum and maximum inclusive
    chunk_min: ChunkIndex,
    chunk_max: ChunkIndex,
    /// Sections in Z->X->Y order
    sections: Vec<Option<Box<Section>>>,
    /// Minecraft stores biomes in 3d, but we only store 2d (at height 64)
    biome: Vec<Biome>,
    heightmap: Vec<u8>,
    watermap: Vec<Option<u8>>,
    pub entities: Vec<Entity>,
}

impl World {
    pub fn new(path: &str, area: Rect) -> Self {
        let region_path = {
            let mut region_path = PathBuf::from(path);
            region_path.push("region");
            region_path.into_os_string().into_string().unwrap()
        };
        let chunk_provider = FolderRegionProvider::new(&region_path);
        let chunk_min: ChunkIndex =
            (area.min - Vec2(crate::LOAD_MARGIN, crate::LOAD_MARGIN)).into();
        let chunk_max: ChunkIndex =
            (area.max + Vec2(crate::LOAD_MARGIN, crate::LOAD_MARGIN)).into();

        let chunk_count =
            ((chunk_max.0 - chunk_min.0 + 1) * (chunk_max.1 - chunk_min.1 + 1)) as usize;

        let mut world = Self {
            path: PathBuf::from(path),
            chunk_min,
            chunk_max,
            sections: vec![None; chunk_count * 16],
            biome: vec![Biome::default(); chunk_count * 2 * 2],
            heightmap: vec![0; chunk_count * 16 * 16],
            watermap: vec![None; chunk_count * 16 * 16],
            entities: Vec::new(),
        };

        // Load chunks. Collecting indexes to vec neccessary for zip
        (chunk_min.1..=chunk_max.1)
            .flat_map(|z| (chunk_min.1..=chunk_max.1).map(move |x| (x, z)))
            .collect_vec()
            .par_iter() //TMP no par
            .zip(world.sections.par_chunks_exact_mut(16))
            .zip(world.biome.par_chunks_exact_mut(2 * 2))
            .zip(world.heightmap.par_chunks_exact_mut(16 * 16))
            .zip(world.watermap.par_chunks_exact_mut(16 * 16))
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

        world
    }

    pub fn save(&self) -> Result<()> {
        // Write chunks
        let mut region_path = self.path.clone();
        region_path.push("region");
        // Internally, AnvilChunkProvider stores a path. So why require a str??
        let region_path = region_path.into_os_string().into_string().unwrap();
        let chunk_provider = FolderRegionProvider::new(&region_path);

        let chunk_count = ((self.chunk_max.0 - self.chunk_min.0 + 1)
            * (self.chunk_max.1 - self.chunk_min.1 + 1)) as usize;
        let mut entities_chunked = vec![vec![]; chunk_count];
        for entity in &self.entities {
            entities_chunked[self.chunk_index(entity.pos.into())].push(entity);
        }

        // Sadly saveing isn't thread safe
        (self.chunk_min.1..=self.chunk_max.1)
            .flat_map(|z| (self.chunk_min.1..=self.chunk_max.1).map(move |x| (x, z)))
            .zip(self.sections.chunks_exact(16))
            .zip(entities_chunked)
            .for_each(|((index, sections), entities)| {
                save_chunk(&chunk_provider, index.into(), sections, &entities)
                    .expect(&format!("Failed to save chunk ({},{}): ", index.0, index.1))
            });

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

        // Set spawn to the center of the area to ensure all command blocks stay loaded
        data.insert_i32("SpawnX", self.area().center().0);
        data.insert_i32("SpawnZ", self.area().center().1);

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(&level_nbt_path)
            .expect("Failed to open level.dat");
        nbt::encode::write_gzip_compound_tag(&mut file, &nbt).expect("Failed to write level.dat");
        Ok(())
    }

    pub fn redstone_processing_area(&self) -> Rect {
        let min = self.area().center() - Vec2(111, 111);
        let max = self.area().center() + Vec2(111, 111);
        Rect {
            min: Column((min.0 / 16) * 16, (min.1 / 16) * 16),
            max: Column((max.0 / 16) * 16 + 15, (max.1 / 16) * 16 + 15),
        }
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

    fn section_index(&self, pos: Pos) -> usize {
        self.chunk_index(pos.into()) * 16 + (pos.1 / 16) as usize
    }

    fn column_index(&self, column: Column) -> usize {
        self.chunk_index(column.into()) * 16 * 16
            + (column.0.rem_euclid(16) + column.1.rem_euclid(16) * 16) as usize
    }

    fn block_in_section_index(pos: Pos) -> usize {
        (pos.0.rem_euclid(16) + pos.1.rem_euclid(16) as i32 * 16 * 16 + pos.2.rem_euclid(16) * 16)
            as usize
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
            .map(|(x, z)| ChunkIndex(x, z))
    }

    pub fn area(&self) -> Rect {
        Rect {
            min: Column(self.chunk_min.0 * 16, self.chunk_min.1 * 16),
            max: Column(self.chunk_max.0 * 16 + 15, self.chunk_max.1 * 16 + 15),
        }
        .shrink(crate::LOAD_MARGIN)
    }
}

impl WorldView for World {
    fn get(&self, pos: Pos) -> &Block {
        if let Some(section) = &self.sections[self.section_index(pos)] {
            &section.blocks[Self::block_in_section_index(pos)]
        } else {
            &Block::Air
        }
    }

    fn get_mut(&mut self, pos: Pos) -> &mut Block {
        let index = self.section_index(pos);
        let section = self.sections[index].get_or_insert_with(|| Box::new(Section::default()));
        &mut section.blocks[Self::block_in_section_index(pos)]
    }

    fn get_mut_no_update_order(&mut self, pos: Pos) -> &mut Block {
        self.get_mut(pos)
    }

    fn biome(&self, column: Column) -> Biome {
        self.biome[self.column_index(Column(column.0 / 4, column.1 / 4))]
    }

    fn height(&self, column: Column) -> u8 {
        self.heightmap[self.column_index(column)]
    }

    fn height_mut(&mut self, column: Column) -> &mut u8 {
        let index = self.column_index(column);
        &mut self.heightmap[index]
    }

    fn water_level(&self, column: Column) -> Option<u8> {
        self.watermap[self.column_index(column)]
    }

    fn water_level_mut(&mut self, column: Column) -> &mut Option<u8> {
        let index = self.column_index(column);
        &mut self.watermap[index]
    }

    fn area(&self) -> Rect {
        Rect {
            min: Column(self.chunk_min.0 * 16, self.chunk_min.1 * 16),
            max: Column(self.chunk_max.0 * 16 + 15, self.chunk_max.1 * 16 + 15),
        }
    }
}

fn load_chunk(
    chunk_provider: &FolderRegionProvider,
    chunk_index: ChunkIndex,
    sections: &mut [Option<Box<Section>>],
    biomes: &mut [Biome],
    heightmap: &mut [u8],
    watermap: &mut [Option<u8>],
) -> Result<()> {
    let nbt = chunk_provider
        .get_region(RegionPosition::from_chunk_position(
            chunk_index.0,
            chunk_index.1,
        ))?
        .read_chunk(RegionChunkPosition::from_chunk_position(
            chunk_index.0,
            chunk_index.1,
        ))?;
    let version = nbt.get_i32("DataVersion").unwrap();
    if (version > 2586) | (version < 2566) {
        // Todo: 1.13+ support (palette)
        println!(
            "Unsupported version: {}. Only 1.16.* is currently supported.",
            version
        );
    }

    let level_nbt = nbt.get_compound_tag("Level").unwrap();

    let biome_ids = level_nbt.get_i32_vec("Biomes").unwrap();
    for i in 0..(2 * 2) {
        biomes[i] = Biome::from_bytes(biome_ids[i] as u8);
    }

    // TODO: store CarvingMasks::AIR, seems useful
    // Also, check out Heightmaps. Maybe we can reuse them or gleam additional information from them

    let sections_nbt = level_nbt.get_compound_tag_vec("Sections").unwrap();

    for section_nbt in sections_nbt {
        let y_index = section_nbt.get_i8("Y").unwrap();

        // Build the palette. Yes, this doesn't deduplicate unrecognised blockstates between sections
        let palette: Vec<Block> = if let Ok(palette) = section_nbt.get_compound_tag_vec("Palette") {
            palette.iter().map(|nbt| Block::from_nbt(nbt)).collect()
        } else {
            continue;
        };

        sections[y_index as usize] = Some(Box::new(Default::default()));
        let section = sections[y_index as usize].as_mut().unwrap();
        let indices = section_nbt.get_i64_vec("BlockStates").unwrap();
        let bits_per_index = bits_per_index(palette.len());

        let mut current_long = 0;
        let mut current_bit_shift = 0;
        for i in 0..(16 * 16 * 16) {
            let packed = indices[current_long] as u64;
            let index = packed.shr(current_bit_shift) as usize % (1 << bits_per_index);
            section.blocks[i] = palette[index].clone();

            current_bit_shift += bits_per_index;
            if current_bit_shift > (64 - bits_per_index) {
                current_bit_shift = 0;
                current_long += 1;
            }
        }
    }

    // Build water- & heightmap
    for x in 0..16 {
        for z in 0..16 {
            'column: for section_index in (0..16).rev() {
                if let Some(section) = &sections[section_index] {
                    for y in (0..16).rev() {
                        let block = &section.blocks[x + z * 16 + y * 16 * 16];
                        let height = (section_index * 16 + y) as u8;
                        if match block {
                            Block::Log(..) => false,
                            _ => block.solid(),
                        } {
                            heightmap[x + z * 16] = height;
                            break 'column;
                        } else if matches!(block, Block::Water /*TODO: | Block::Ice*/) {
                            watermap[x + z * 16].get_or_insert((section_index * 16 + y) as u8);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn bits_per_index(palette_len: usize) -> usize {
    for bits in 4.. {
        if palette_len <= 1 << bits {
            return bits;
        }
    }
    unreachable!()
}

fn save_chunk(
    chunk_provider: &FolderRegionProvider,
    index: ChunkIndex,
    sections: &[Option<Box<Section>>],
    entities: &[&Entity],
) -> Result<()> {
    chunk_provider
        .get_region(RegionPosition::from_chunk_position(index.0, index.1))?
        .write_chunk(
            RegionChunkPosition::from_chunk_position(index.0, index.1),
            {
                let mut nbt = CompoundTag::new();
                nbt.insert_i32("DataVersion", 2586);
                nbt.insert_compound_tag("Level", {
                    let mut nbt = CompoundTag::new();
                    nbt.insert_i32("xPos", index.0);
                    nbt.insert_i32("zPos", index.1);

                    nbt.insert_i64("LastUpdate", 0);
                    nbt.insert_i8("TerrainPopulated", 1);
                    nbt.insert_i64("InhabitetTime", 0);
                    nbt.insert_str("Status", "full");

                    nbt.insert_compound_tag_vec("Entities", Vec::new());
                    nbt.insert_compound_tag_vec("TileEntities", Vec::new());

                    // Collect tile entities
                    let mut tile_entities = Vec::new();

                    nbt.insert_compound_tag_vec("Sections", {
                        sections
                            .iter()
                            .enumerate()
                            .filter_map(|(y_index, section)| {
                                if let Some(section) = section {
                                    let mut nbt = CompoundTag::new();
                                    nbt.insert_i8("Y", y_index as i8);

                                    // Build the palette first (for length)
                                    // Minecraft seems to always have Air as id 0 even if there is none
                                    let mut palette = HashMap::new();
                                    nbt.insert_compound_tag_vec(
                                        "Palette",
                                        Some(Air).iter().chain(section.blocks.iter()).flat_map(
                                            |block| {
                                                if !palette.contains_key(block) {
                                                    palette.insert(block.clone(), palette.len());
                                                    Some(block.to_nbt())
                                                } else {
                                                    None
                                                }
                                            },
                                        ),
                                    );

                                    let bits_per_index = bits_per_index(palette.len());
                                    let mut blocks = vec![0];
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
                                                Pos(index.0 * 16, y_index as u8 * 16, index.1 * 16);
                                            let pos = section_base
                                                + Vec3(
                                                    i as i32 % 16,
                                                    i as i32 / (16 * 16),
                                                    i as i32 % (16 * 16) / 16,
                                                );
                                            tile_entities.extend(block.tile_entity_nbt(pos));
                                        }
                                    }
                                    nbt.insert_i64_vec("BlockStates", blocks);

                                    Some(nbt)
                                } else {
                                    None
                                }
                            })
                    });

                    nbt.insert_compound_tag_vec("Entities", entities.iter().map(|e| e.to_nbt()));

                    nbt.insert_compound_tag_vec("TileEntities", tile_entities);

                    nbt
                });
                nbt
            },
        )?;
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
