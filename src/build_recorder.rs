use crate::*;
use hashlink::linked_hash_map::LinkedHashMap;
use std::collections::HashMap;

pub struct BuildRecorder<'a, T: WorldView>(&'a T, BuildRecord);

impl<'a, T: WorldView> BuildRecorder<'a, T> {
    pub fn new(world: &'a T) -> Self {
        Self(
            world,
            BuildRecord {
                blocks: LinkedHashMap::new(),
                heightmap: HashMap::new(),
                water_level: HashMap::new(),
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

    pub fn iter(&self) -> impl Iterator<Item = (&Pos, &Block)> {
        self.1.blocks.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Pos, &mut Block)> {
        self.1.blocks.iter_mut()
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

    fn get_mut_no_update_order(&mut self, pos: Pos) -> &mut Block {
        let BuildRecorder(world, record) = self;
        if !record.blocks.contains_key(&pos) {
            record.blocks.insert(pos, world.get(pos).clone());
        }
        record.blocks.get_mut(&pos).unwrap()
    }

    fn biome(&self, column: Column) -> Biome {
        self.0.biome(column)
    }

    fn height(&self, column: Column) -> u8 {
        *self
            .1
            .heightmap
            .get(&column)
            .unwrap_or(&self.0.height(column))
    }

    fn height_mut(&mut self, column: Column) -> &mut u8 {
        let BuildRecorder(world, record) = self;
        record
            .heightmap
            .entry(column)
            .or_insert_with(|| world.height(column))
    }

    fn water_level(&self, column: Column) -> Option<u8> {
        *self
            .1
            .water_level
            .get(&column)
            .unwrap_or(&self.0.water_level(column))
    }

    fn water_level_mut(&mut self, column: Column) -> &mut Option<u8> {
        let BuildRecorder(world, record) = self;
        record
            .water_level
            .entry(column)
            .or_insert_with(|| world.water_level(column))
    }

    fn area(&self) -> Rect {
        self.0.area()
    }
}

pub struct BuildRecord {
    blocks: LinkedHashMap<Pos, Block>,
    heightmap: HashMap<Column, u8>,
    water_level: HashMap<Column, Option<u8>>,
}

impl BuildRecord {
    pub fn apply_to(&self, world: &mut impl WorldView) {
        for (pos, block) in &self.blocks {
            world.set(*pos, block);
        }
        for (column, height) in &self.heightmap {
            *world.height_mut(*column) = *height;
        }
        for (column, height) in &self.water_level {
            *world.water_level_mut(*column) = *height;
        }
    }

    pub fn commands(&self) -> Commands {
        let mut commands = vec![];
        for (pos, block) in self.blocks.iter() {
            if let Some(tile_entity) = block.tile_entity_nbt(*pos) {
                commands.push(format!(
                    "setblock {} {} {} {}{}",
                    pos.0,
                    pos.1,
                    pos.2,
                    block.blockstate(),
                    tile_entity
                ));
            } else {
                commands.push(format!(
                    "setblock {} {} {} {}",
                    pos.0,
                    pos.1,
                    pos.2,
                    block.blockstate(),
                ));
            }
        }
        commands
    }
}
