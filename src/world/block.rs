use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use Block::*;


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
    GroundPlant(Plant),
    Crop(Crop),
    Debug(u8),
    Other { id: u8, data: u8 }
}

impl Default for Block {
    fn default() -> Self {
        Block::Air
    }
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
            9 => Water,
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
            6 => GroundPlant(Plant::Sapling(TreeSpecies::from_u8(data % 8).unwrap())),
            31 => GroundPlant(Plant::Small(match data {
                0 => SmallPlant::Grass,
                _ => SmallPlant::Fern
            })),
            32 => GroundPlant(Plant::Small(SmallPlant::DeadBush)),
            37 => GroundPlant(Plant::Small(SmallPlant::Dandelion)),
            38 => GroundPlant(Plant::Small(match data {
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
            39 => GroundPlant(Plant::Small(SmallPlant::BrownMushroom)),
            40 => GroundPlant(Plant::Small(SmallPlant::RedMushroom)),
            81 => GroundPlant(Plant::Cactus),
            83 => GroundPlant(Plant::Reeds),
            175 => GroundPlant(Plant::Tall{
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
                Soil::Farmland => (60, 0)
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
                Plant::Sapling(species) => (6, species as u8 ),
                Plant::Small(plant) => match plant {
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
                Plant::Cactus => (81, 0),
                Plant::Reeds => (83, 0),
                Plant::Tall{plant, upper} => (175, 
                    match plant {
                        TallPlant::Sunflower => 0,
                        TallPlant::Lilac => 1,
                        TallPlant::Grass => 2,
                        TallPlant::Fern => 3,
                        TallPlant::Rose => 4,
                        TallPlant::Peony => 5,
                    } + if upper {8} else {0}
                )
            },
            Crop(crop) => match crop {
                Crop::Wheat => (59, 0),
                Crop::Carrot => (141, 0),
                Crop::Potato => (142, 0),
                Crop::Beetroot => (207, 0)
            },
            Debug(data) => (35, data),
            Other {id, data} => (id, data),
        }
    }

    pub fn is_solid(self) -> bool {
        // Todo: expand this
        match self {
            Air => false,
            Water => false,
            Lava => false,
            GroundPlant(..) => false,
            Leaves(..) => false,
            Crop(..) => false,
            _ => true
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, FromPrimitive)]
#[repr(u8)]
pub enum Axis {
    Y,
    X,
    Z
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


#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum HorDir {
    XPos,
    XNeg,
    ZPos,
    ZNeg
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum FullDir {
    XPos,
    XNeg,
    YPos,
    YNeg,
    ZPos,
    ZNeg
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, FromPrimitive)]
#[repr(u8)]
pub enum TreeSpecies {
    Oak,
    Spruce,
    Birch,
    Junge,
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
    Farmland
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum Plant {
    Sapling(TreeSpecies),
    Cactus,
    Reeds,
    Small(SmallPlant),
    Tall {plant: TallPlant, upper: bool},
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