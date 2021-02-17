use std::sync::Arc;

pub use self::GroundPlant::*;
use crate::geometry::*;
use nbt::CompoundTag;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
pub use Block::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum Block {
    Air,
    Stone(Stone),
    Planks(TreeSpecies),
    Water,
    Lava,
    Soil(Soil),
    Log(TreeSpecies, LogType, LogOrigin),
    Leaves(TreeSpecies),
    GroundPlant(GroundPlant),
    Fence(Fence),
    Wool(Color),
    SnowLayer,
    Glowstone,
    GlassPane(Option<Color>),
    Hay,
    Slab(BuildBlock, Flipped),
    Stair(BuildBlock, HDir, Flipped),
    Cauldron { water: u8 },
    Repeater(HDir, u8),
    Barrier,
    Bedrock,
    CommandBlock(Arc<String>),
    Debug(u8),
    Other { id: u8, data: u8 },
}

impl Default for Block {
    fn default() -> Self {
        Air
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum LogType {
    Normal(Axis),
    FullBark,
}

// So far this is only used to check whether this log can sustain leaves
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum LogOrigin {
    Natural,
    Stump,
    Manmade,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, FromPrimitive)]
#[repr(u8)]
pub enum TreeSpecies {
    Oak,
    Spruce,
    Birch,
    Jungle,
    Acacia,
    DarkOak,
}

// Represents man-placed stone, even when the same blocks could occure naturally
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum Stone {
    Stone,
    Granite,
    Diorite,
    Andesite,
    Cobble,
    Stonebrick,
    Brick,
    // Todo: Sandstone, (Polished) Stones, Cracked/Mossy
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum Soil {
    Dirt,
    Grass,
    Sand,
    Gravel,
    Farmland,
    Path,
    Podzol,
    CoarseDirt,
    SoulSand,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum GroundPlant {
    Sapling(TreeSpecies),
    Cactus,
    Reeds,
    Pumpkin(HDir),
    Small(SmallPlant),
    Tall { plant: TallPlant, upper: bool },
    Crop(Crop),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum SmallPlant {
    Grass,
    DeadBush,
    Fern,
    BrownMushroom,
    RedMushroom,
    Dandelion,
    Poppy,
    BlueOrchid,
    Allium,
    AzureBluet,
    RedTulip,
    OrangeTulip,
    WhiteTulip,
    PinkTulip,
    OxeyeDaisy,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum TallPlant {
    Grass,
    Fern,
    Sunflower,
    Lilac,
    Rose,
    Peony,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum Crop {
    Wheat,
    Carrot,
    Potato,
    Beetroot,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum Fence {
    Wood(TreeSpecies),
    Stone { mossy: bool },
}

// Note: for dyes, id order is reversed
#[derive(Debug, Copy, Clone, Eq, PartialEq, FromPrimitive, Hash)]
#[repr(u8)]
pub enum Color {
    White,
    Orange,
    Magenta,
    LightBlue,
    Yellow,
    Lime,
    Pink,
    Gray,
    LightGray,
    Cyan,
    Purple,
    Blue,
    Brown,
    Green,
    Red,
    Black,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Flipped(pub bool);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BuildBlock {
    Wooden(TreeSpecies),
    Cobble,
    Stonebrick,
    Brick,
}

impl BuildBlock {
    pub fn full(self) -> Block {
        match self {
            BuildBlock::Wooden(species) => Planks(species),
            BuildBlock::Cobble => Stone(Stone::Cobble),
            BuildBlock::Brick => Stone(Stone::Brick),
            BuildBlock::Stonebrick => Stone(Stone::Stonebrick),
        }
    }
}

impl Block {
    // Deserialisation only neccessary for naturally occuring blocks
    pub fn from_bytes(id: u8, data: u8) -> Self {
        match id {
            0 => Air,
            1 => match data {
                0 => Stone(Stone::Stone),
                1 => Stone(Stone::Granite),
                3 => Stone(Stone::Diorite),
                5 => Stone(Stone::Andesite),
                _ => Other { id, data },
            },
            2 => Soil(Soil::Grass),
            3 => Soil(Soil::Dirt),
            12 => Soil(Soil::Sand),
            13 => Soil(Soil::Gravel),
            60 => Soil(Soil::Farmland),
            7 => Bedrock,
            8 | 9 => {
                if data == 0 {
                    Block::Water
                } else {
                    Block::Air
                    // TODO: Fix up the water source if the flow overlapps buildings/paths
                }
            }
            10 | 11 => {
                if data == 0 {
                    Block::Lava
                } else {
                    Block::Air
                }
            }
            17 => Log(
                TreeSpecies::from_u8(data % 4).unwrap(),
                match data >> 4 {
                    3 => LogType::FullBark,
                    dir => LogType::Normal(Axis::from_u8(dir).unwrap()),
                },
                LogOrigin::Natural,
            ),
            162 => Log(
                TreeSpecies::from_u8(data % 4 + 4).unwrap(),
                match data >> 4 {
                    3 => LogType::FullBark,
                    dir => LogType::Normal(Axis::from_u8(dir).unwrap()),
                },
                LogOrigin::Natural,
            ),
            18 => Leaves(TreeSpecies::from_u8(data % 4).unwrap()),
            161 => Leaves(TreeSpecies::from_u8(data % 4 + 4).unwrap()),
            6 => GroundPlant(GroundPlant::Sapling(
                TreeSpecies::from_u8(data % 8).unwrap(),
            )),
            31 => GroundPlant(GroundPlant::Small(match data {
                0 => SmallPlant::Grass,
                _ => SmallPlant::Fern,
            })),
            32 => GroundPlant(GroundPlant::Small(SmallPlant::DeadBush)),
            37 => GroundPlant(GroundPlant::Small(SmallPlant::Dandelion)),
            38 => GroundPlant(GroundPlant::Small(match data {
                0 => SmallPlant::Poppy,
                1 => SmallPlant::BlueOrchid,
                2 => SmallPlant::Allium,
                3 => SmallPlant::AzureBluet,
                4 => SmallPlant::RedTulip,
                5 => SmallPlant::OrangeTulip,
                6 => SmallPlant::WhiteTulip,
                7 => SmallPlant::PinkTulip,
                _ => SmallPlant::OxeyeDaisy,
            })),
            39 => GroundPlant(GroundPlant::Small(SmallPlant::BrownMushroom)),
            40 => GroundPlant(GroundPlant::Small(SmallPlant::RedMushroom)),
            81 => GroundPlant(GroundPlant::Cactus),
            83 => GroundPlant(GroundPlant::Reeds),
            89 => Glowstone,
            175 => GroundPlant(GroundPlant::Tall {
                plant: match id % 8 {
                    0 => TallPlant::Sunflower,
                    1 => TallPlant::Lilac,
                    2 => TallPlant::Grass,
                    3 => TallPlant::Fern,
                    4 => TallPlant::Rose,
                    _ => TallPlant::Peony,
                },
                upper: id >= 8,
            }),
            78 => SnowLayer,
            _ => Other { id, data },
        }
    }

    pub fn to_bytes(&self) -> (u8, u8) {
        match self {
            Air => (0, 0),
            Stone(stone) => match stone {
                Stone::Stone => (1, 0),
                Stone::Granite => (1, 1),
                Stone::Diorite => (1, 3),
                Stone::Andesite => (1, 5),
                Stone::Cobble => (4, 0),
                Stone::Brick => (45, 0),
                Stone::Stonebrick => (98, 0),
            },
            Planks(species) => (5, *species as u8),
            Soil(soil_type) => match soil_type {
                Soil::Grass => (2, 0),
                Soil::Dirt => (3, 0),
                Soil::Sand => (12, 0),
                Soil::Gravel => (13, 0),
                Soil::Farmland => (60, 0),
                Soil::Path => (208, 0),
                Soil::CoarseDirt => (3, 1),
                Soil::Podzol => (3, 2),
                Soil::SoulSand => (88, 0),
            },
            Bedrock => (7, 0),
            Water => (9, 0),
            Lava => (11, 0),
            Log(species, log_type, _) => (
                if (*species as u8) < 4 { 17 } else { 162 },
                (match log_type {
                    LogType::Normal(dir) => *dir as u8,
                    LogType::FullBark => 3,
                } << 2)
                    + (*species as u8) % 4,
            ),
            Leaves(species) => (
                if (*species as u8) < 4 { 18 } else { 161 },
                (*species as u8) % 4 + 4, // no decay
            ),
            GroundPlant(plant) => match plant {
                GroundPlant::Sapling(species) => (6, *species as u8),
                GroundPlant::Small(plant) => match plant {
                    SmallPlant::Grass => (31, 0),
                    SmallPlant::Fern => (31, 1),
                    SmallPlant::DeadBush => (32, 0),
                    SmallPlant::Dandelion => (37, 0),
                    SmallPlant::Poppy => (38, 0),
                    SmallPlant::BlueOrchid => (38, 1),
                    SmallPlant::Allium => (38, 2),
                    SmallPlant::AzureBluet => (38, 3),
                    SmallPlant::RedTulip => (38, 4),
                    SmallPlant::OrangeTulip => (38, 5),
                    SmallPlant::WhiteTulip => (38, 6),
                    SmallPlant::PinkTulip => (38, 7),
                    SmallPlant::OxeyeDaisy => (38, 8),
                    SmallPlant::BrownMushroom => (39, 0),
                    SmallPlant::RedMushroom => (40, 0),
                },
                GroundPlant::Cactus => (81, 0),
                GroundPlant::Reeds => (83, 0),
                GroundPlant::Pumpkin(dir) => (86, *dir as u8),
                GroundPlant::Tall { plant, upper } => (
                    175,
                    match plant {
                        TallPlant::Sunflower => 0,
                        TallPlant::Lilac => 1,
                        TallPlant::Grass => 2,
                        TallPlant::Fern => 3,
                        TallPlant::Rose => 4,
                        TallPlant::Peony => 5,
                    } + if *upper { 8 } else { 0 },
                ),
                GroundPlant::Crop(crop) => match crop {
                    Crop::Wheat => (59, 7),
                    Crop::Carrot => (141, 7),
                    Crop::Potato => (142, 7),
                    Crop::Beetroot => (207, 3),
                },
            },
            Fence(fence) => match fence {
                Fence::Wood(TreeSpecies::Oak) => (85, 0),
                Fence::Wood(TreeSpecies::Spruce) => (188, 0),
                Fence::Wood(TreeSpecies::Birch) => (189, 0),
                Fence::Wood(TreeSpecies::Jungle) => (190, 0),
                Fence::Wood(TreeSpecies::DarkOak) => (191, 0),
                Fence::Wood(TreeSpecies::Acacia) => (192, 0),
                Fence::Stone { mossy: false } => (139, 0),
                Fence::Stone { mossy: true } => (139, 1),
            },
            Wool(color) => (35, *color as u8),
            SnowLayer => (78, 0),
            Glowstone => (89, 0),
            GlassPane(color) => {
                if let Some(color) = color {
                    (160, *color as u8)
                } else {
                    (102, 0)
                }
            }
            Hay => (170, 0),
            Slab(material, flipped) => {
                let flipped = if matches!(flipped, Flipped(true)) {
                    8
                } else {
                    0
                };
                if let BuildBlock::Wooden(species) = material {
                    (126, *species as u8 + flipped)
                } else {
                    (
                        44,
                        match material {
                            BuildBlock::Cobble => 3,
                            BuildBlock::Brick => 4,
                            BuildBlock::Stonebrick => 4,
                            BuildBlock::Wooden(..) => panic!(),
                        } + flipped,
                    )
                }
            }
            Stair(material, dir, flipped) => (
                match material {
                    BuildBlock::Wooden(TreeSpecies::Oak) => 53,
                    BuildBlock::Wooden(TreeSpecies::Spruce) => 134,
                    BuildBlock::Wooden(TreeSpecies::Birch) => 135,
                    BuildBlock::Wooden(TreeSpecies::Jungle) => 136,
                    BuildBlock::Wooden(TreeSpecies::Acacia) => 163,
                    BuildBlock::Wooden(TreeSpecies::DarkOak) => 164,
                    BuildBlock::Cobble => 67,
                    BuildBlock::Brick => 108,
                    BuildBlock::Stonebrick => 109,
                },
                if matches!(flipped, Flipped(true)) {
                    4
                } else {
                    0
                } + match dir {
                    HDir::XPos => 0,
                    HDir::XNeg => 1,
                    HDir::ZPos => 2,
                    HDir::ZNeg => 3,
                },
            ),
            Cauldron { water } => (118, water % 4),
            Repeater(dir, delay) => (93, (*dir as u8 + 2) % 4 + delay * 4),
            Barrier => (166, 0),
            CommandBlock(_) => (137, 0),
            Debug(data) => (251, *data),
            Other { id, data } => (*id, *data),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Air => "air",
            Soil(soil_type) => match soil_type {
                Soil::Grass => "grass",
                Soil::Dirt | Soil::CoarseDirt | Soil::Podzol => "dirt",
                Soil::Sand => "sand",
                Soil::Gravel => "gravel",
                Soil::Farmland => "farmland",
                Soil::Path => "grass_path",
                Soil::SoulSand => "soul_sand",
            },
            Stone(stone) => match stone {
                Stone::Stone | Stone::Andesite | Stone::Diorite | Stone::Granite => "stone",
                Stone::Cobble => "cobblestone",
                Stone::Brick => "brick_block",
                Stone::Stonebrick => "stonebrick",
            },
            Planks(..) => "planks",
            Water => "water",
            Lava => "lava",
            Log(species, ..) => {
                if (*species as u8) < 4 {
                    "log"
                } else {
                    "log2"
                }
            }
            Leaves(species) => {
                if (*species as u8) < 4 {
                    "leaves"
                } else {
                    "leaves2"
                }
            }
            GroundPlant(plant) => match plant {
                GroundPlant::Sapling(_) => "sapling",
                GroundPlant::Small(plant) => match plant {
                    SmallPlant::Grass | SmallPlant::Fern => "tallgrass",
                    SmallPlant::DeadBush => "deadbush",
                    SmallPlant::Dandelion => "yellow_flower",
                    SmallPlant::Poppy
                    | SmallPlant::BlueOrchid
                    | SmallPlant::Allium
                    | SmallPlant::AzureBluet
                    | SmallPlant::RedTulip
                    | SmallPlant::OrangeTulip
                    | SmallPlant::WhiteTulip
                    | SmallPlant::PinkTulip
                    | SmallPlant::OxeyeDaisy => "red_flower",
                    SmallPlant::BrownMushroom => "brown_mushroom",
                    SmallPlant::RedMushroom => "red_mushroom",
                },
                GroundPlant::Cactus => "cactus",
                GroundPlant::Reeds => "reeds",
                GroundPlant::Pumpkin(_) => "pumpkin",
                GroundPlant::Tall { .. } => "double_plant",
                GroundPlant::Crop(crop) => match crop {
                    Crop::Wheat => "wheat",
                    Crop::Carrot => "carrots",
                    Crop::Potato => "potatoes",
                    Crop::Beetroot => "beetroots",
                },
            },
            Fence(fence) => match fence {
                Fence::Wood(TreeSpecies::Oak) => "fence",
                Fence::Wood(TreeSpecies::Spruce) => "spruce_fence",
                Fence::Wood(TreeSpecies::Birch) => "birch_fence",
                Fence::Wood(TreeSpecies::Jungle) => "jungle_fence",
                Fence::Wood(TreeSpecies::DarkOak) => "dark_oak_fence",
                Fence::Wood(TreeSpecies::Acacia) => "acacia_fence",
                Fence::Stone { .. } => "cobblestone_wall",
            },
            Wool(_) => "wool",
            SnowLayer => "snow_layer",
            GlassPane(color) => {
                if color.is_some() {
                    "stained_glass_pane"
                } else {
                    "glass_pane"
                }
            }
            Glowstone => "glowstone",
            Hay => "hay_block",
            Slab(material, ..) => match material {
                BuildBlock::Wooden(_) => "wooden_slab",
                _ => "stone_slab",
            },
            Stair(material, ..) => match material {
                BuildBlock::Wooden(species) => match species {
                    TreeSpecies::Oak => "oak_stairs",
                    TreeSpecies::Spruce => "spruce_stairs",
                    TreeSpecies::Birch => "birch_stairs",
                    TreeSpecies::Jungle => "jungle_stairs",
                    TreeSpecies::Acacia => "acacia_stairs",
                    TreeSpecies::DarkOak => "dark_oak_stairs",
                },
                BuildBlock::Cobble => "stone_stairs",
                BuildBlock::Brick => "brick_stairs",
                BuildBlock::Stonebrick => "stone_brick_stairs",
            },
            Cauldron { .. } => "cauldron",
            Repeater(..) => "unpowered_repeater",
            Bedrock => "bedrock",
            CommandBlock(_) => "command_block",
            Barrier => "barrier",
            Debug(_) => "concrete",
            Other { .. } => panic!(),
        }
    }

    pub fn tile_entity_nbt(&self, pos: Pos) -> Option<CompoundTag> {
        match self {
            CommandBlock(command) => {
                let mut nbt = CompoundTag::new();
                nbt.insert_i32("x", pos.0);
                nbt.insert_i32("y", pos.1 as i32);
                nbt.insert_i32("z", pos.2);
                nbt.insert_str("id", "command_block");
                nbt.insert_str("Command", &command);
                nbt.insert_bool("TrackOutput", false);
                Some(nbt)
            }
            _ => None,
        }
    }

    pub fn solid(&self) -> bool {
        // Todo: expand this
        !matches!(
            self,
            Air | Water | Lava | GroundPlant(..) | Leaves(..) | SnowLayer
        )
    }

    pub fn light_properties(&self) -> LightProperties {
        match self {
            Lava | Glowstone => LightProperties {
                emission: 15,
                transparent: true,
                filter_skylight: true,
            },
            Water | Leaves(_) => LightProperties {
                emission: 0,
                transparent: true,
                filter_skylight: false,
            },
            Air | Repeater { .. } | GroundPlant(..) | SnowLayer | GlassPane(..) | Fence(..) => {
                LightProperties {
                    emission: 0,
                    transparent: true,
                    filter_skylight: false,
                }
            }
            _ => LightProperties {
                emission: 0,
                transparent: false,
                filter_skylight: true,
            },
        }
    }
}

pub struct LightProperties {
    pub emission: u8,
    pub transparent: bool,
    pub filter_skylight: bool,
}
