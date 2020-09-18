use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use Block::*;


#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Block {
    Air,
    Stone(StoneType),
    Water,
    Lava,
    Soil(SoilType),
    Log(TreeSpecies, LogType),
    Leaves(TreeSpecies),
    Plant(PlantType),
    Crop(CropType),
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
                0 => Stone(StoneType::Stone),
                1 => Stone(StoneType::Granite),
                3 => Stone(StoneType::Diorite),
                5 => Stone(StoneType::Andesite),
                _ => Other { id, data }
            },
            2 => Soil(SoilType::Grass),
            3 => Soil(SoilType::Dirt),
            12 => Soil(SoilType::Sand),
            13 => Soil(SoilType::Gravel),
            60 => Soil(SoilType::Farmland),
            9 => Water,
            11 => Lava,
            17 => Log(
                TreeSpecies::from_u8(data % 4).unwrap(), 
                match data >> 4 {
                    3 => LogType::FullBark,
                    dir => LogType::Normal(Axis::from_u8(dir).unwrap())
                }
            ),
            162 => Log(
                TreeSpecies::from_u8(data % 4 + 4).unwrap(), 
                match data >> 4 {
                    3 => LogType::FullBark,
                    dir => LogType::Normal(Axis::from_u8(dir).unwrap())
                }
            ),
            18 => Leaves(TreeSpecies::from_u8(data % 4).unwrap()),
            161 => Leaves(TreeSpecies::from_u8(data % 4 + 4).unwrap()),
            6 => Plant(PlantType::Sapling(TreeSpecies::from_u8(data % 8).unwrap())),
            31 => Plant(PlantType::Small(match data {
                0 => SmallPlant::Grass,
                _ => SmallPlant::Fern
            })),
            32 => Plant(PlantType::Small(SmallPlant::DeadBush)),
            37 => Plant(PlantType::Small(SmallPlant::Dandelion)),
            38 => Plant(PlantType::Small(match data {
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
            39 => Plant(PlantType::Small(SmallPlant::BrownMushroom)),
            40 => Plant(PlantType::Small(SmallPlant::RedMushroom)),
            81 => Plant(PlantType::Cactus),
            83 => Plant(PlantType::Reeds),
            175 => Plant(PlantType::Tall{
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
                StoneType::Stone => 0,
                StoneType::Granite => 1,
                StoneType::Diorite => 3,
                StoneType::Andesite => 5
            }),
            Soil(soil_type) => match soil_type {
                SoilType::Grass => (2, 0),
                SoilType::Dirt => (3, 0),
                SoilType::Sand => (12, 0),
                SoilType::Gravel => (13, 0),
                SoilType::Farmland => (60, 0)
            },
            Water => (9, 0),
            Lava => (11, 0),
            Log(species, log_type) => (
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
            Plant(plant) => match plant {
                PlantType::Sapling(species) => (6, species as u8 ),
                PlantType::Small(plant) => match plant {
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
                PlantType::Cactus => (81, 0),
                PlantType::Reeds => (83, 0),
                PlantType::Tall{plant, upper} => (175, 
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
                CropType::Wheat => (59, 0),
                CropType::Carrot => (141, 0),
                CropType::Potato => (142, 0),
                CropType::Beetroot => (207, 0)
            },
            Other {id, data} => (id, data),
        }
    }

    pub fn is_solid(self) -> bool {
        // Todo: expand this
        match self {
            Air => false,
            Water => false,
            Lava => false,
            Plant(..) => false,
            Leaves(..) => false,
            Crop(..) => false,
            _ => true
        }
    }
}

#[derive(Debug, Copy, Clone, FromPrimitive)]
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

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum HorDir {
    XPos,
    XNeg,
    ZPos,
    ZNeg
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum FullDir {
    XPos,
    XNeg,
    YPos,
    YNeg,
    ZPos,
    ZNeg
}

#[derive(Debug, Copy, Clone, FromPrimitive)]
#[repr(u8)]
pub enum TreeSpecies {
    Oak,
    Spruce,
    Birch,
    Junge,
    Acacia,
    DarkOak
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum StoneType {
    Stone,
    Granite,
    Diorite,
    Andesite,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum SoilType {
    Dirt,
    Grass,
    Sand,
    Gravel,
    Farmland
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum PlantType {
    Sapling(TreeSpecies),
    Cactus,
    Reeds,
    Small(SmallPlant),
    Tall {plant: TallPlant, upper: bool},
}

#[derive(Debug, Copy, Clone)]
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

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum TallPlant {
    Grass,
    Fern,
    Sunflower,
    Lilac,
    Rose,
    Peony
}


#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum CropType {
    Wheat,
    Carrot,
    Potato,
    Beetroot
}