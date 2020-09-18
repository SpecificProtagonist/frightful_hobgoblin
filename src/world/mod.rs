mod block;
mod biome;

use std::ops::{Index, IndexMut};
use std::collections::HashMap;
use anvil_region::*;
use nbt::CompoundTag;

pub use block::*;
pub use biome::*;


const MAX_VERSION: i32 = 1343;

#[derive(Debug, Copy, Clone)]
pub struct Area {
    pub min: Column,
    pub max: Column
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Pos(pub i32, pub u8, pub i32);

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Column(pub i32, pub i32);
impl From<Pos> for Column {
    fn from(pos: Pos) -> Self {
        Column(pos.0, pos.2)
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct ChunkIndex(pub i32, pub i32);
impl From<Column> for ChunkIndex {
    fn from(column: Column) -> Self {
        ChunkIndex(
            column.0.div_euclid(16),
            column.1.div_euclid(16)
        )
    }
}
impl From<Pos> for ChunkIndex {
    fn from(pos: Pos) -> Self {
        Column(pos.0, pos.2).into()
    }
}


pub struct World {
    region_path: String,
    chunks: HashMap<ChunkIndex, Chunk>
} 

impl World {
    pub fn new(path: &str) -> Self {
        World {
            region_path: String::from(path) + "/region",
            chunks: HashMap::new(),
        }
    }

    pub fn load_area(&mut self, area: Area) -> Result<(), ChunkLoadError> {
        let chunk_provider = AnvilChunkProvider::new(&self.region_path);
        let chunk_min: ChunkIndex = area.min.into();
        let chunk_max: ChunkIndex = area.max.into();
        for chunk_x in chunk_min.0 ..= chunk_max.0 {
            for chunk_z in chunk_min.1 ..= chunk_max.1 {
                let index = ChunkIndex(chunk_x, chunk_z);
                if !self.chunks.contains_key(&index) {
                    self.chunks.insert(index, Chunk::load(&chunk_provider, index)?);
                }
            }
        }

        Ok(())
    }

    pub fn save(&self) -> Result<(), ChunkSaveError> {
        let chunk_provider = AnvilChunkProvider::new(&self.region_path);
        for chunk in self.chunks.values() {
            chunk.save(&chunk_provider)?;
        }
        Ok(())
    }
}

// load_area must have been called before
// todo: remove this requirement
impl Index<Pos> for World {
    type Output = Block;
    fn index(&self, pos: Pos) -> &Self::Output {
        let chunk = self.chunks.get(&pos.into()).unwrap();
        if let Some(section) = &chunk.sections[pos.1 as usize / 16] {
            &section.blocks[
                pos.0.rem_euclid(16) as usize
              + pos.1.rem_euclid(16) as usize * 16 * 16
              + pos.2.rem_euclid(16) as usize * 16
            ]
        } else {
            &Block::Air
        }
    }
}

impl IndexMut<Pos> for World {
    fn index_mut(&mut self, pos: Pos) -> &mut Self::Output {
        let chunk = self.chunks.get_mut(&pos.into()).unwrap();
        let section = chunk.sections[pos.1 as usize / 16].get_or_insert_with(||
            Box::new(Section::default())
        );
        &mut section.blocks[
            pos.0.rem_euclid(16) as usize
            + pos.1.rem_euclid(16) as usize * 16 * 16
            + pos.2.rem_euclid(16) as usize * 16
        ]
    }
}

pub struct Chunk {
    index: ChunkIndex,
    sections: [Option<Box<Section>>; 16],
    biomes: [Biome; 16 * 16],
    // Todo: Entities, TileEntities
}

impl Chunk {
    fn load(chunk_provider: &AnvilChunkProvider, index: ChunkIndex) -> Result<Self, ChunkLoadError> {
        let nbt = chunk_provider.load_chunk(index.0, index.1)?;
        let version = nbt.get_i32("DataVersion").unwrap();
        if version > MAX_VERSION {
            // Todo: 1.16 support
            println!("Unsupported version: {}. Only 1.12 is supported currently.", version);
        }

        let level_nbt = nbt.get_compound_tag("Level").unwrap();

        let mut biomes = [Biome::Other(0); 16 * 16];
        let biome_ids = level_nbt.get_i8_vec("Biomes").unwrap();
        for i in 0..(16*16) {
            biomes[i] = Biome::from_bytes(biome_ids[i] as u8);
        }


        let mut sections: [Option<Box<Section>>; 16] = Default::default();
        let sections_nbt = level_nbt.get_compound_tag_vec("Sections").unwrap();
        
        for section_nbt in sections_nbt {
            let index = section_nbt.get_i8("Y").unwrap();
            sections[index as usize] = Some(Box::new(Default::default()));
            let section = sections[index as usize].as_mut().unwrap();
            // Ignore Add tag (not neccessary for vanilla)
            let block_ids = section_nbt.get_i8_vec("Blocks").unwrap();
            let block_data = section_nbt.get_i8_vec("Data").unwrap();
            for i in 0..(16*16*16) {
                section.blocks[i] = Block::from_bytes(
                    block_ids[i] as u8, 
                    {
                        let byte = block_data[i/2] as u8;
                        // Todo: Check if this is the right way around!
                        if i%2 == 0 {
                            byte % 16
                        } else {
                            byte >> 4
                        }
                    }
                )
            }
        }

        Ok(Chunk {
            index,
            sections,
            biomes
        })
    }

    fn save(&self, chunk_provider: &AnvilChunkProvider) -> Result<(), ChunkSaveError> {
        chunk_provider.save_chunk(self.index.0, self.index.1, {
            let mut nbt = CompoundTag::new();
            nbt.insert_i32("DataVersion", 1343);
            nbt.insert_compound_tag("Level", {
                let mut nbt = CompoundTag::new();
                nbt.insert_i32("xPos", self.index.0);
                nbt.insert_i32("zPos", self.index.1);

                nbt.insert_i64("LastUpdate", 0);
                nbt.insert_i8("LightPopulated", 0);
                nbt.insert_i8("TerrainPopulated", 1);
                nbt.insert_i64("InhabitetTime", 0);

                nbt.insert_compound_tag_vec("Entities", Vec::new());
                nbt.insert_compound_tag_vec("TileEntities", Vec::new());
                // Todo: correct heightmap
                nbt.insert_i8_vec("HeightMap", vec![0; 16*16]);

                nbt.insert_i8_vec("Biomes", 
                    self.biomes.iter().map(|biome|biome.to_bytes() as i8).collect()
                );

                nbt.insert_compound_tag_vec("Sections", {
                    self.sections.iter().enumerate().filter_map(|(y_index, section)|
                        if let Some(section) = section {
                            let mut nbt = CompoundTag::new();
                            nbt.insert_i8("Y", y_index as i8);
                            let mut block_ids = Vec::new();
                            let mut block_data = Vec::new();
                            for (i, block) in section.blocks.iter().enumerate() {
                                let (id, data) = block.to_bytes();
                                block_ids.push(id as i8);
                                if i % 2 == 0 {
                                    block_data.push(data as i8);
                                } else {
                                    let prev_data = block_data.last_mut().unwrap();
                                    *prev_data = ((*prev_data as u8) + (data << 4)) as i8;
                                }
                            }
                            nbt.insert_i8_vec("Blocks", block_ids);
                            nbt.insert_i8_vec("Data", block_data);

                            // Todo: correct lighting (without these tags, minecraft rejects the chunk)
                            nbt.insert_i8_vec("BlockLight", vec![0; 16*16*16/2]);
                            nbt.insert_i8_vec("SkyLight", vec![0; 16*16*16/2]);

                            Some(nbt)
                        } else {
                            None
                        }
                    ).collect()
                });
                nbt
            });
            nbt
        })
    }
}


pub struct Section {
    blocks: [Block; 16 * 16 * 16]
}

impl Default for Section {
    fn default() -> Self {
        Section {
            blocks: [Block::Air; 16 * 16 * 16]
        }
    }
}