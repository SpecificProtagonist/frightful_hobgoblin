use crate::*;

#[derive(Default, Debug, Copy, Clone)]
#[repr(u8)]
pub enum Biome {
    #[default]
    Basic,
    River,
    Ocean,
    Beach,
    // Mountains,
    Snowy,
    Desert,
    Taiga,
    BirchForest,
    Swamp,
    Jungles,
    Mesa,
    Savanna,
    DarkForest,
    MangroveSwamp,
    CherryGrove,
}

use Biome::*;

impl Biome {
    pub fn from_id(id: &str) -> Self {
        let id = id.strip_prefix("minecraft:").unwrap();
        match id {
            "snowy_plains" | "ice_spikes" | "snowy_taiga" | "grove" | "snowy_slopes"
            | "frozen_peaks" | "jagged_peaks" | "stony_peaks" => Snowy,
            "desert" => Desert,
            "swamp" => Swamp,
            "mangrove_swamp" => MangroveSwamp,
            "birch_forest" | "old_growth_birch_forest" => BirchForest,
            "dark_forest" => DarkForest,
            "taiga" | "old_growth_pine_taiga" => Taiga,
            "savanna" | "savanna_plateau" | "windswept_savanna" => Savanna,
            "junge" | "sparse_junge" | "bamboo_jungle" => Jungles,
            "badlands" | "eroded_badlands" | "wooded_badland" => Mesa,
            "river" | "frozen_river" => River,
            "beach" | "snowy_beach" => Beach,
            "warm_ocean"
            | "lukewarm_ocean"
            | "deep_lukewarm_ocean"
            | "ocean"
            | "deep_ocean"
            | "cold_ocean"
            | "deep_cold_ocean"
            | "frozen_ocean"
            | "deep_frozen_ocean" => Ocean,
            _ => Basic,
        }
    }

    pub fn default_tree_species(self) -> TreeSpecies {
        match self {
            // Mountains => Spruce,
            Taiga | Snowy => Spruce,
            DarkForest => DarkOak,
            BirchForest => Birch,
            Jungles => Jungle,
            MangroveSwamp => Mangrove,
            Desert | Savanna => Acacia,
            CherryGrove => Cherry,
            _ => Oak,
        }
    }

    pub fn random_tree_species(self) -> TreeSpecies {
        match self {
            // Mountains => { if rand(0.6) { Oak } else { Spruce } }
            DarkForest => {
                if 0.25 > rand() {
                    TreeSpecies::Oak
                } else {
                    TreeSpecies::DarkOak
                }
            }
            Basic => {
                if 0.15 > rand() {
                    TreeSpecies::Birch
                } else {
                    TreeSpecies::Oak
                }
            }
            BirchForest => {
                if 0.15 > rand() {
                    TreeSpecies::Oak
                } else {
                    TreeSpecies::Birch
                }
            }
            other => other.default_tree_species(),
        }
    }

    pub fn default_topsoil(self) -> Block {
        match self {
            Desert | Ocean | Beach => Sand,
            River => Dirt,
            Taiga | Mesa => CoarseDirt,
            _ => Dirt,
        }
    }

    pub fn villager_type(self) -> &'static str {
        match self {
            Swamp => "minecraft:swamp",
            Savanna => "minecraft:savanna",
            Jungles => "minecraft:jungle",
            Desert => "minecraft:desert",
            Taiga => "minecraft:taige",
            Snowy => "minecraft:snow",
            _ => "minecraft:plains",
        }
    }
}
