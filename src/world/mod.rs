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
use std::{mem::swap, path::PathBuf};

use crate::geometry::*;
pub use biome::*;
pub use block::*;
pub use entity::*;

const MAX_VERSION: i32 = 1343;

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
            biome: vec![Biome::default(); chunk_count * 16 * 16],
            heightmap: vec![0; chunk_count * 16 * 16],
            watermap: vec![None; chunk_count * 16 * 16],
            entities: vec![],
        };

        // Load chunks. Collecting indexes to vec neccessary for zip
        (chunk_min.1..=chunk_max.1)
            .flat_map(|z| (chunk_min.1..=chunk_max.1).map(move |x| (x, z)))
            .collect_vec()
            .par_iter()
            .zip(world.sections.par_chunks_exact_mut(16))
            .zip(world.biome.par_chunks_exact_mut(16 * 16))
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

        let light = calculate_block_light(&self);

        // Sadly chunk_provider saveing isn't thread safe
        (self.chunk_min.1..=self.chunk_max.1)
            .flat_map(|z| (self.chunk_min.1..=self.chunk_max.1).map(move |x| (x, z)))
            .zip(self.sections.chunks_exact(16))
            .zip(self.biome.chunks_exact(16 * 16))
            .zip(entities_chunked)
            .zip(light)
            .for_each(|((((index, sections), biome), entities), light)| {
                save_chunk(
                    &chunk_provider,
                    index.into(),
                    sections,
                    biome,
                    &entities,
                    light,
                )
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
        self.biome[self.column_index(column)]
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
    index: ChunkIndex,
    sections: &mut [Option<Box<Section>>],
    biomes: &mut [Biome],
    heightmap: &mut [u8],
    watermap: &mut [Option<u8>],
) -> Result<()> {
    let nbt = chunk_provider
        .get_region(RegionPosition::from_chunk_position(index.0, index.1))?
        .read_chunk(RegionChunkPosition::from_chunk_position(index.0, index.1))?;
    let version = nbt.get_i32("DataVersion").unwrap();
    if version > MAX_VERSION {
        // Todo: 1.13+ support (palette)
        println!(
            "Unsupported version: {}. Only 1.12 is supported currently.",
            version
        );
    }

    let level_nbt = nbt.get_compound_tag("Level").unwrap();

    let biome_ids = level_nbt.get_i8_vec("Biomes").unwrap();
    for i in 0..(16 * 16) {
        biomes[i] = Biome::from_bytes(biome_ids[i] as u8);
    }

    let sections_nbt = level_nbt.get_compound_tag_vec("Sections").unwrap();

    for section_nbt in sections_nbt {
        let index = section_nbt.get_i8("Y").unwrap();
        sections[index as usize] = Some(Box::new(Default::default()));
        let section = sections[index as usize].as_mut().unwrap();
        // Ignore Add tag (not neccessary for vanilla)
        let block_ids = section_nbt.get_i8_vec("Blocks").unwrap();
        let block_data = section_nbt.get_i8_vec("Data").unwrap();
        for i in 0..(16 * 16 * 16) {
            section.blocks[i] = Block::from_bytes(block_ids[i] as u8, {
                let byte = block_data[i / 2] as u8;
                if i % 2 == 0 {
                    byte % 16
                } else {
                    byte >> 4
                }
            })
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
                        } else if matches!(block, Block::Water) {
                            watermap[x + z * 16].get_or_insert((section_index * 16 + y) as u8);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn save_chunk(
    chunk_provider: &FolderRegionProvider,
    index: ChunkIndex,
    sections: &[Option<Box<Section>>],
    biomes: &[Biome],
    entities: &[&Entity],
    light: ChunkLight,
) -> Result<()> {
    chunk_provider
        .get_region(RegionPosition::from_chunk_position(index.0, index.1))?
        .write_chunk(
            RegionChunkPosition::from_chunk_position(index.0, index.1),
            {
                let mut nbt = CompoundTag::new();
                nbt.insert_i32("DataVersion", 1343);
                nbt.insert_compound_tag("Level", {
                    let mut nbt = CompoundTag::new();
                    nbt.insert_i32("xPos", index.0);
                    nbt.insert_i32("zPos", index.1);

                    nbt.insert_i64("LastUpdate", 0);
                    nbt.insert_i8("LightPopulated", 1);
                    nbt.insert_i8("TerrainPopulated", 1);
                    nbt.insert_i64("InhabitetTime", 0);

                    nbt.insert_compound_tag_vec("Entities", Vec::new());
                    nbt.insert_compound_tag_vec("TileEntities", Vec::new());
                    // Todo: correct heightmap
                    nbt.insert_i8_vec("HeightMap", vec![0; 16 * 16]);

                    // Minecraft actually loads the chunk if the biomes tag is missing,
                    // but regenerates the biomes incorrectly
                    nbt.insert_i8_vec(
                        "Biomes",
                        biomes.iter().map(|biome| biome.to_bytes() as i8).collect(),
                    );

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
                                    let mut block_ids = Vec::new();
                                    let mut block_data = Vec::new();
                                    for (i, block) in section.blocks.iter().enumerate() {
                                        // Store id (byte) and data (nibble)
                                        let (id, data) = block.to_bytes();
                                        block_ids.push(id as i8);
                                        if i % 2 == 0 {
                                            block_data.push(data as i8);
                                        } else {
                                            let prev_data = block_data.last_mut().unwrap();
                                            *prev_data = ((*prev_data as u8) + (data << 4)) as i8;
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
                                    nbt.insert_i8_vec("Blocks", block_ids);
                                    nbt.insert_i8_vec("Data", block_data);

                                    let light = &light
                                        [y_index * (16 * 16 * 16)..(y_index + 1) * 16 * 16 * 16];
                                    let mut block_light = Vec::new();
                                    let mut sky_light = Vec::new();
                                    for i in (0..(16 * 16 * 16)).step_by(2) {
                                        block_light.push(
                                            (light[i].block + (light[i + 1].block << 4)) as i8,
                                        );
                                        sky_light
                                            .push((light[i].sky + (light[i + 1].sky << 4)) as i8);
                                    }
                                    nbt.insert_i8_vec("BlockLight", block_light);
                                    nbt.insert_i8_vec("SkyLight", sky_light);

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

#[derive(Clone, Copy)]
struct Light {
    block: u8,
    sky: u8,
}
type ChunkLight = [Light; 16 * 16 * 256];

fn calculate_block_light(world: &World) -> Vec<ChunkLight> {
    let width_chunks = (world.chunk_max.0 - world.chunk_min.0 + 1) as usize;
    let num_chunks = width_chunks * (world.chunk_max.0 - world.chunk_min.0 + 1) as usize;
    let mut light_buffer = vec![[Light { block: 0, sky: 0 }; 16 * 16 * 256]; num_chunks];
    let mut back_buffer = light_buffer.clone();

    let chunk_offset = |index: usize| -> Vec2 {
        Vec2(
            world.chunk_min.0 + (index % width_chunks) as i32,
            world.chunk_min.1 + (index / width_chunks) as i32,
        ) * 16
    };

    fn index_in_chunk(pos: Pos) -> usize {
        pos.0 as usize + pos.2 as usize * 16 + pos.1 as usize * 256
    }

    // Fill in areas of direct sunlight
    light_buffer
        .par_iter_mut()
        .enumerate()
        .for_each(|(index, buffer)| {
            let chunk_offset = chunk_offset(index);
            for x in 0..16 {
                for z in 0..16 {
                    for y in (0..=255).rev() {
                        if world
                            .get(Pos(x, y, z) + chunk_offset)
                            .light_properties()
                            .filter_skylight
                        {
                            break;
                        } else {
                            buffer[index_in_chunk(Pos(x, y, z))].sky = 15;
                        }
                    }
                }
            }
        });

    // Flood fill block-light & sky-light simultaneously
    for _ in 0..15 {
        back_buffer
            .iter_mut()
            .enumerate()
            .for_each(|(index, back_buffer)| {
                let mut chunk = Cuboid {
                    min: Pos(0, 1, 0), // Don't update layers 0 and 255; TODO: fix this
                    max: Pos(15, 254, 15),
                };

                // Skip border
                // TODO: fix this (low priority as this creates a helpfull border marker for now)
                // Maybe just don't process the outer chunks and keep and use their original light values
                if index < width_chunks {
                    chunk.min.2 = 1
                }
                if index >= num_chunks - width_chunks {
                    chunk.max.2 = 14
                }
                if index % width_chunks == 0 {
                    chunk.min.0 = 1
                }
                if index % width_chunks == 15 {
                    chunk.max.0 = 14
                }

                let chunk_off = chunk_offset(index);
                for pos in chunk.iter() {
                    let block = world.get(pos + chunk_off).light_properties();
                    if block.transparent {
                        let mut light = Light {
                            block: block.emission,
                            sky: light_buffer[index][index_in_chunk(pos)].sky,
                        };
                        for pos in &[
                            pos + Vec2(-1, 0),
                            pos + Vec2(1, 0),
                            pos + Vec2(0, -1),
                            pos + Vec2(0, 1),
                            pos + Vec3(0, -1, 0),
                            pos + Vec3(0, 1, 0),
                        ] {
                            let neighbor = if pos.0 < 0 {
                                light_buffer[index - 1][index_in_chunk(*pos + Vec2(16, 0))]
                            } else if pos.0 > 15 {
                                let buff = &light_buffer[index + 1];
                                buff[index_in_chunk(*pos - Vec2(16, 0))]
                            } else if pos.2 < 0 {
                                light_buffer[index - width_chunks]
                                    [index_in_chunk(*pos + Vec2(0, 16))]
                            } else if pos.2 > 15 {
                                light_buffer[index + width_chunks]
                                    [index_in_chunk(*pos - Vec2(0, 16))]
                            } else {
                                light_buffer[index][index_in_chunk(*pos)]
                            };
                            light.block = light.block.max(neighbor.block.saturating_sub(1));
                            light.sky = light.sky.max(neighbor.sky.saturating_sub(1));
                        }

                        back_buffer[index_in_chunk(pos)] = light;
                    }
                }
            });
        swap(&mut light_buffer, &mut back_buffer);
    }

    light_buffer
}
