use rand::random;
use crate::world::TreeSpecies;
use BiomeType::*;
use Temperature::*;

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum BiomeType {
    Basic,
    Ocean,
    Desert,
    ExtremeHills,
    Taiga,
    Swamp,
    River,
    Jungle,
    Beach,
    BirchForest,
    RoofedForest,
    Savanna,
    Mesa,
    Other(u8)
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Temperature {
    Cold,
    Moderate,
    Hot
}


#[derive(Debug, Copy, Clone)]
pub struct Biome {
    pub base: BiomeType,
    pub temp: Temperature,
    pub hilly: bool,
    pub wooded: bool
}

impl Biome {
    pub fn from_bytes(id: u8) -> Self {
        // Ignore mutated biomes
        match id % 128 {
            0 => Biome { base: Ocean, temp: Moderate, hilly: false, wooded: false},
            1 => Biome { base: Basic, temp: Moderate, hilly: false, wooded: false},
            2 => Biome { base: Desert, temp: Hot, hilly: false, wooded: false},
            3 => Biome { base: ExtremeHills, temp: Moderate, hilly: true, wooded: false},
            4 => Biome { base: Basic, temp: Moderate, hilly: false, wooded: true},
            5 => Biome { base: Taiga, temp: Moderate, hilly: false, wooded: true},
            6 => Biome { base: Swamp, temp: Moderate, hilly: false, wooded: true},
            7 => Biome { base: River, temp: Moderate, hilly: false, wooded: false},
            10 => Biome { base: Ocean, temp: Cold, hilly: false, wooded: false},
            11 => Biome { base: River, temp: Cold, hilly: false, wooded: false},
            12 => Biome { base: Basic, temp: Cold, hilly: false, wooded: false},
            13 => Biome { base: Basic, temp: Cold, hilly: true, wooded: false},
            // Todo: mushroom
            16 => Biome { base: Beach, temp: Moderate, hilly: false, wooded: false},
            17 => Biome { base: Desert, temp: Hot, hilly: true, wooded: false},
            18 => Biome { base: Basic, temp: Moderate, hilly: true, wooded: true},
            19 => Biome { base: Taiga, temp: Moderate, hilly: true, wooded: true},
            20 => Biome { base: ExtremeHills, temp: Moderate, hilly: true, wooded: false},
            21 => Biome { base: Jungle, temp: Hot, hilly: false, wooded: true},
            22 => Biome { base: Jungle, temp: Hot, hilly: true, wooded: true},
            23 => Biome { base: Jungle, temp: Hot, hilly: false, wooded: true},
            24 => Biome { base: Ocean, temp: Moderate, hilly: false, wooded: false},
            25 => Biome { base: Beach, temp: Moderate, hilly: false, wooded: false},
            26 => Biome { base: Beach, temp: Cold, hilly: false, wooded: false},
            27 => Biome { base: BirchForest, temp: Moderate, hilly: false, wooded: true},
            28 => Biome { base: BirchForest, temp: Moderate, hilly: true, wooded: true},
            29 => Biome { base: RoofedForest, temp: Moderate, hilly: false, wooded: true},
            30 => Biome { base: Taiga, temp: Cold, hilly: false, wooded: true},
            31 => Biome { base: Taiga, temp: Cold, hilly: true, wooded: true},
            32 => Biome { base: Taiga, temp: Moderate, hilly: false, wooded: true},
            33 => Biome { base: Taiga, temp: Moderate, hilly: true, wooded: true},
            34 => Biome { base: ExtremeHills, temp: Moderate, hilly: true, wooded: true},
            35 => Biome { base: Savanna, temp: Hot, hilly: false, wooded: false},
            36 => Biome { base: Savanna, temp: Hot, hilly: false, wooded: false},
            37 => Biome { base: Mesa, temp: Hot, hilly: false, wooded: false},
            38 => Biome { base: Mesa, temp: Hot, hilly: false, wooded: true},
            39 => Biome { base: Mesa, temp: Hot, hilly: false, wooded: false},
            _ => Biome { base: Other(id), temp: Moderate, hilly: false, wooded: false}
        }
    }

    // Some information has been discarded, but (hopefully) none that has an impact on gameplay or appearance
    pub fn to_bytes(self) -> u8 {
        match self {
            Biome { base: Ocean, temp: Moderate, ..} => 0,
            Biome { base: Ocean, temp: Cold, ..} => 10,
            Biome { base: Basic, temp: Moderate, hilly: false, wooded: false} => 1,
            Biome { base: Basic, temp: Moderate, hilly: false, wooded: true} => 4,
            Biome { base: Basic, temp: Moderate, hilly: true, wooded: true} => 18,
            Biome { base: Basic, temp: Cold, hilly: false, wooded: false} => 12,
            Biome { base: Basic, temp: Cold, hilly: true, wooded: false} => 13,
            Biome { base: Desert, hilly: false, ..} => 2,
            Biome { base: Desert, hilly: true, ..} => 17,
            Biome { base: ExtremeHills, wooded: false, ..} => 3,
            Biome { base: ExtremeHills, wooded: true, ..} => 34,
            Biome { base: Taiga, temp: Moderate, hilly: false, ..} => 5,
            Biome { base: Taiga, temp: Moderate, hilly: true, ..} => 19,
            Biome { base: Taiga, temp: Cold, hilly: false, ..} => 30,
            Biome { base: Taiga, temp: Cold, hilly: true, ..} => 31,
            Biome { base: Swamp, ..} => 6,
            Biome { base: River, temp: Moderate, ..} => 7,
            Biome { base: River, temp: Cold, ..} => 11,
            Biome { base: Beach, temp: Moderate, ..} => 16,
            Biome { base: Beach, temp: Cold, ..} => 26,
            Biome { base: Jungle, hilly: false, ..} => 21,
            Biome { base: Jungle, hilly: true, ..} => 22,
            Biome { base: BirchForest, hilly: false, ..} => 27,
            Biome { base: BirchForest, hilly: true, ..} => 28,
            Biome { base: RoofedForest, ..} => 29,
            Biome { base: Savanna, ..} => 35,
            Biome { base: Mesa, wooded: false, ..} => 37,
            Biome { base: Mesa, wooded: true, ..} => 38,
            Biome { base: Other(id), ..} => id,
            _ => 0
        }
    }

    pub fn default_tree_species(self) -> TreeSpecies {
        match self {
            Biome {base: ExtremeHills, ..} => TreeSpecies::Spruce,
            Biome {base: Taiga, ..} => TreeSpecies::Spruce,
            Biome {base: RoofedForest, ..} => TreeSpecies::DarkOak,
            Biome {base: BirchForest, ..} => TreeSpecies::Birch,
            Biome {base: Jungle, ..} => TreeSpecies::Jungle,
            Biome {base: Mesa, ..} => TreeSpecies::Oak,
            Biome {temp: Hot, ..} => TreeSpecies::Acacia,
            Biome {temp: Moderate, ..} => TreeSpecies::Oak,
            Biome {temp: Cold, ..} => TreeSpecies::Spruce
        }
    }

    pub fn random_tree_species(self) -> TreeSpecies {
        match self {
            Biome {base: Basic, temp: Moderate, ..} => if random::<f32>() < 0.15 {TreeSpecies::Birch} else {TreeSpecies::Oak},
            Biome {base: ExtremeHills, ..} => if random::<f32>() < 0.6 {TreeSpecies::Oak} else {TreeSpecies::Spruce},
            Biome {base: RoofedForest, ..} => if random::<f32>() < 0.25 {TreeSpecies::Oak} else {TreeSpecies::DarkOak},
            _ => self.default_tree_species()
        }
    }

    pub fn villager_type(self) -> &'static str {
        match self {
            Biome {base: Swamp, ..} => "minecraft:swamp",
            Biome {base: Savanna, ..} => "minecraft:savanna",
            Biome {base: Jungle, ..} => "minecraft:jungle",
            Biome {base: Desert, ..} => "minecraft:desert",
            Biome {base: Taiga, ..} => "minecraft:taige",
            Biome {temp: Cold, ..} => "minecraft:snow",
            Biome {..} => "minecraft:plains"
        }
    }
}

impl Default for Biome {
    fn default() -> Self {
        Biome {base: Ocean, temp: Moderate, hilly: false, wooded: false}
    }
}