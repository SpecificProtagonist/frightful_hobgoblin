use num_traits::FromPrimitive;
use num_derive::FromPrimitive;
use crate::geometry::*;
pub use Block::*;
pub use self::GroundPlant::*;


#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Block {
    Air,
    Stone(Stone),
    Water,
    Lava,
    Soil(Soil),
    Log(TreeSpecies, LogType, LogOrigin),
    Leaves(TreeSpecies),
    GroundPlant(GroundPlant),
    Fence(Fence),
    Wool(Color),
    Glowstone,
    Hay,
    Slab { kind: Slab, upper: bool },
    Barrier,
    Debug(u8),
    Other { id: u8, data: u8 }
}

impl Default for Block {
    fn default() -> Self {
        Air
    }
}


#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum LogType {
    Normal(Axis),
    FullBark
}

// So far this is only used to check whether this log can sustain leaves
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum LogOrigin {
    Natural,
    Stump,
    Manmade
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, FromPrimitive)]
#[repr(u8)]
pub enum TreeSpecies {
    Oak,
    Spruce,
    Birch,
    Jungle,
    Acacia,
    DarkOak
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum Stone {
    Stone,
    Granite,
    Diorite,
    Andesite,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum Soil {
    Dirt,
    Grass,
    Sand,
    Gravel,
    Farmland,
    Path,
    Podzol,
    CoarseDirt
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum GroundPlant {
    Sapling(TreeSpecies),
    Cactus,
    Reeds,
    Pumpkin(HDir),
    Small(SmallPlant),
    Tall {plant: TallPlant, upper: bool},
    Crop(Crop)
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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
    OxeyeDaisy
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum TallPlant {
    Grass,
    Fern,
    Sunflower,
    Lilac,
    Rose,
    Peony
}


#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum Crop {
    Wheat,
    Carrot,
    Potato,
    Beetroot
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum Fence {
    Wood(TreeSpecies),
    Stone {mossy: bool}
}

// Note: for dyes, id order is reversed 
#[derive(Debug, Copy, Clone, Eq, PartialEq, FromPrimitive)]
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
	Black
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum Slab {
    Wooden(TreeSpecies)
}




// Can the serialize & deserialize-funtions somehow be unified?A
// maybe a macro could help
impl Block {
    pub fn from_bytes(id: u8, data: u8) -> Self {
        match id {
            0 => Air,
            1 => match data {
                0 => Stone(Stone::Stone),
                1 => Stone(Stone::Granite),
                3 => Stone(Stone::Diorite),
                5 => Stone(Stone::Andesite),
                _ => Other { id, data }
            },
            2 => Soil(Soil::Grass),
            3 => Soil(Soil::Dirt),
            12 => Soil(Soil::Sand),
            13 => Soil(Soil::Gravel),
            60 => Soil(Soil::Farmland),
            8 => Block::Air, // Flowing water
            9 => Water,
            10 => Block::Air, // Flowing lava
            11 => Lava,
            17 => Log(
                TreeSpecies::from_u8(data % 4).unwrap(), 
                match data >> 4 {
                    3 => LogType::FullBark,
                    dir => LogType::Normal(Axis::from_u8(dir).unwrap())
                },
                LogOrigin::Natural
            ),
            162 => Log(
                TreeSpecies::from_u8(data % 4 + 4).unwrap(), 
                match data >> 4 {
                    3 => LogType::FullBark,
                    dir => LogType::Normal(Axis::from_u8(dir).unwrap())
                },
                LogOrigin::Natural
            ),
            18 => Leaves(TreeSpecies::from_u8(data % 4).unwrap()),
            161 => Leaves(TreeSpecies::from_u8(data % 4 + 4).unwrap()),
            6 => GroundPlant(GroundPlant::Sapling(TreeSpecies::from_u8(data % 8).unwrap())),
            31 => GroundPlant(GroundPlant::Small(match data {
                0 => SmallPlant::Grass,
                _ => SmallPlant::Fern
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
                _ => SmallPlant::OxeyeDaisy
            })),
            39 => GroundPlant(GroundPlant::Small(SmallPlant::BrownMushroom)),
            40 => GroundPlant(GroundPlant::Small(SmallPlant::RedMushroom)),
            81 => GroundPlant(GroundPlant::Cactus),
            83 => GroundPlant(GroundPlant::Reeds),
            175 => GroundPlant(GroundPlant::Tall{
                plant: match id % 8 {
                    0 => TallPlant::Sunflower,
                    1 => TallPlant::Lilac,
                    2 => TallPlant::Grass,
                    3 => TallPlant::Fern,
                    4 => TallPlant::Rose,
                    _ => TallPlant::Peony
                },
                upper: id >= 8
            }),
            _ => Other { id, data }
        }
    }

    pub fn to_bytes(self) -> (u8, u8) {
        match self {
            Air => (0, 0),
            Stone(mineral) => (1, match mineral {
                Stone::Stone => 0,
                Stone::Granite => 1,
                Stone::Diorite => 3,
                Stone::Andesite => 5
            }),
            Soil(soil_type) => match soil_type {
                Soil::Grass => (2, 0),
                Soil::Dirt => (3, 0),
                Soil::Sand => (12, 0),
                Soil::Gravel => (13, 0),
                Soil::Farmland => (60, 0),
                Soil::Path => (208, 0),
                Soil::CoarseDirt => (3, 1),
                Soil::Podzol => (3, 2)
            },
            Water => (9, 0),
            Lava => (11, 0),
            Log(species, log_type, _) => (
                if (species as u8) < 4 {17} else {162},
                (match log_type {
                    LogType::Normal(dir) => dir as u8,
                    LogType::FullBark => 3
                } << 2) + (species as u8) % 4
            ),
            Leaves(species) => (
                if (species as u8) < 4 {18} else {161},
                (species as u8)%4 + 4 // no decay
            ),
            GroundPlant(plant) => match plant {
                GroundPlant::Sapling(species) => (6, species as u8 ),
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
                GroundPlant::Pumpkin(dir) => (86, dir as u8),
                GroundPlant::Tall{plant, upper} => (175, 
                    match plant {
                        TallPlant::Sunflower => 0,
                        TallPlant::Lilac => 1,
                        TallPlant::Grass => 2,
                        TallPlant::Fern => 3,
                        TallPlant::Rose => 4,
                        TallPlant::Peony => 5,
                    } + if upper {8} else {0}
                ),
                GroundPlant::Crop(crop) => match crop {
                    Crop::Wheat => (59, 7),
                    Crop::Carrot => (141, 7),
                    Crop::Potato => (142, 7),
                    Crop::Beetroot => (207, 3)
                },
            },
            Fence(fence) => match fence {
                Fence::Wood(TreeSpecies::Oak) => (85, 0),
                Fence::Wood(TreeSpecies::Spruce) => (188, 0),
                Fence::Wood(TreeSpecies::Birch) => (189, 0),
                Fence::Wood(TreeSpecies::Jungle) => (190, 0),
                Fence::Wood(TreeSpecies::DarkOak) => (191, 0),
                Fence::Wood(TreeSpecies::Acacia) => (192, 0),
                Fence::Stone {mossy: false} => (139, 0),
                Fence::Stone {mossy: true} => (139, 1),
            },
            Wool(color) => (35, color as u8),
            Glowstone => (89, 0),
            Hay => (170, 0),
            Block::Slab { kind, upper } => match kind {
                Slab::Wooden(species) => (126, species as u8 + if upper {8} else {0})
            },
            Barrier => (166, 0),
            Debug(data) => (35, data),
            Other {id, data} => (id, data),
        }
    }

    pub fn solid(self) -> bool {
        // Todo: expand this
        match self {
            Air => false,
            Water => false,
            Lava => false,
            GroundPlant(..) => false,
            Leaves(..) => false,
            _ => true
        }
    }
}