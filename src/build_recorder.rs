use crate::*;
use hashlink::linked_hash_map::LinkedHashMap;
use std::collections::HashMap;
use std::num::NonZeroU8;

pub struct BuildRecorder<'a, T: WorldView>(&'a T, BuildRecord);

impl<'a, T: WorldView> BuildRecorder<'a, T> {
    pub fn new(world: &'a T) -> Self {
        Self(
            world,
            BuildRecord {
                blocks: LinkedHashMap::new(),
                heightmap: HashMap::new(),
                watermap: HashMap::new(),
            },
        )
    }

    pub fn finish(self) -> BuildRecord {
        let BuildRecorder(world, mut record) = self;
        record
            .blocks
            .retain(|pos, block| (world.get(*pos) != block));
        record
    }
}

impl<T: WorldView> WorldView for BuildRecorder<'_, T> {
    fn get(&self, pos: Pos) -> &Block {
        self.1.blocks.get(&pos).unwrap_or_else(|| self.0.get(pos))
    }

    fn get_mut(&mut self, pos: Pos) -> &mut Block {
        let BuildRecorder(world, record) = self;
        record
            .blocks
            .entry(pos)
            .or_insert_with(|| world.get(pos).clone())
    }

    fn biome(&self, column: Column) -> Biome {
        self.0.biome(column)
    }

    fn heightmap(&self, column: Column) -> u8 {
        *self
            .1
            .heightmap
            .get(&column)
            .unwrap_or(&self.0.heightmap(column))
    }

    fn heightmap_mut(&mut self, column: Column) -> &mut u8 {
        let BuildRecorder(world, record) = self;
        record
            .heightmap
            .entry(column)
            .or_insert_with(|| world.heightmap(column))
    }

    fn watermap(&self, column: Column) -> Option<std::num::NonZeroU8> {
        *self
            .1
            .watermap
            .get(&column)
            .unwrap_or(&self.0.watermap(column))
    }

    fn watermap_mut(&mut self, column: Column) -> &mut Option<std::num::NonZeroU8> {
        let BuildRecorder(world, record) = self;
        record
            .watermap
            .entry(column)
            .or_insert_with(|| world.watermap(column))
    }

    fn area(&self) -> Rect {
        self.0.area()
    }
}

pub struct BuildRecord {
    blocks: LinkedHashMap<Pos, Block>,
    heightmap: HashMap<Column, u8>,
    watermap: HashMap<Column, Option<NonZeroU8>>,
}

impl BuildRecord {
    pub fn apply_to(&self, world: &mut impl WorldView) {
        for (pos, block) in &self.blocks {
            world.set(*pos, block);
        }
        for (column, height) in &self.heightmap {
            *world.heightmap_mut(*column) = *height;
        }
        for (column, height) in &self.watermap {
            *world.watermap_mut(*column) = *height;
        }
    }

    pub fn commands(&self) -> Commands {
        let mut commands = vec![];
        for (pos, block) in self.blocks.iter() {
            if let Some(tile_entity) = block.tile_entity_nbt(*pos) {
                commands.push(format!(
                    "setblock {} {} {} {} {} replace {}",
                    pos.0,
                    pos.1,
                    pos.2,
                    block.name(),
                    block.to_bytes().1,
                    tile_entity
                ));
            } else {
                commands.push(format!(
                    "setblock {} {} {} {} {}",
                    pos.0,
                    pos.1,
                    pos.2,
                    block.name(),
                    block.to_bytes().1,
                ));
            }
        }
        commands
    }
}
