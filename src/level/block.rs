use std::{
    borrow::Cow,
    cell::RefCell,
    fmt::{Display, Write},
    mem::size_of,
    str::FromStr,
    sync::{LazyLock, RwLock},
};

pub use self::GroundPlant::*;
use crate::{default, geometry::*, HashMap};
use nbt::CompoundTag;
use num_derive::FromPrimitive;

pub use Block::*;
pub use Color::*;
pub use Material::*;
pub use TreeSpecies::*;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Block {
    #[default]
    Air,
    Full(Material),
    Slab(Material, Flipped),
    Stair(Material, HDir, Flipped),
    Planks(TreeSpecies),
    Fence(Material),
    Water,
    Lava,
    Soil(Soil),
    // TODO: stripped logs
    Log(TreeSpecies, LogType),
    // Store distance from log if not persistent
    Leaves(TreeSpecies, Option<i8>),
    GroundPlant(GroundPlant),
    Wool(Color),
    Terracotta(Option<Color>),
    SmoothQuartz,
    SnowLayer,
    Glowstone,
    GlassPane(Option<Color>),
    WallBanner(HDir, Color),
    Hay,
    Cauldron {
        water: u8,
    },
    Trapdoor(TreeSpecies, HDir, DoorMeta),
    Door(TreeSpecies, HDir, DoorMeta),
    Bell(HDir, BellAttachment),
    Repeater(HDir, u8),
    Barrier,
    Bedrock,
    Other(u16),
}

const _: () = assert!(size_of::<Block>() == 4);

/// Used to deduplicate unknown blocks.
/// Does not affect performance but greatly reduced memory usage
/// (block only 4 bytes, fewer boxes → 1000×1000 fits into 1 gb ).
#[derive(Default, Clone)]
pub struct UnknownBlocks {
    map: HashMap<Blockstate, u16>,
    states: Vec<Blockstate>,
}

pub static UNKNOWN_BLOCKS: LazyLock<RwLock<UnknownBlocks>> = LazyLock::new(default);

pub fn debug_read_unknown(index: u16) -> Blockstate {
    UNKNOWN_BLOCKS.read().unwrap().states[index as usize].clone()
}

bitflags::bitflags! {
    #[derive(Copy,Clone, Debug, Eq, PartialEq, Hash)]
    pub struct DoorMeta: u8 {
        const TOP = 0b01;
        const OPEN = 0b10;
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum LogType {
    Normal(Axis),
    FullBark,
    Stripped(Axis),
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
    Warped,
    Crimson,
    Mangrove,
    Cherry,
    Azalea,
    FloweringAzalea,
}

impl TreeSpecies {
    pub fn to_str(self) -> &'static str {
        match self {
            Oak => "oak",
            Spruce => "spruce",
            Birch => "birch",
            Jungle => "jungle",
            Acacia => "acacia",
            DarkOak => "dark_oak",
            Warped => "warped",
            Crimson => "crimson",
            Mangrove => "mangrove",
            Cherry => "cherry",
            Azalea => "azalea",
            FloweringAzalea => "flowering_azalea",
        }
    }
}

impl Display for TreeSpecies {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
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
    Pumpkin,
    Small(SmallPlant),
    Tall(TallPlant, Flipped),
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

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                White => "white",
                Orange => "orange",
                Magenta => "magenta",
                LightBlue => "light_blue",
                Yellow => "yellow",
                Lime => "lime",
                Pink => "pink",
                Gray => "gray",
                LightGray => "light_gray",
                Cyan => "cyan",
                Purple => "purple",
                Blue => "blue",
                Brown => "brown",
                Green => "green",
                Red => "red",
                Black => "black",
            }
        )
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Flipped(pub bool);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Material {
    Stone,
    Granite,
    PolishedGranite,
    Diorite,
    PolishedDiorite,
    Andesite,
    PolishedAndesite,
    Wood(TreeSpecies),
    Cobble,
    MossyCobble,
    StoneBrick,
    MossyStonebrick,
    Brick,
    Sandstone,
    SmoothSandstone,
    RedSandstone,
    SmoothRedSandstone,
    Blackstone,
    PolishedBlackstone,
    PolishedBlackstoneBrick,
    PackedMud,
    MudBrick,
}

impl Display for Material {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl Material {
    pub fn to_str(self) -> &'static str {
        match self {
            Stone => "stone",
            Diorite => "diorite",
            PolishedDiorite => "polished_diorite",
            Granite => "granite",
            PolishedGranite => "polished_granite",
            Andesite => "andesite",
            PolishedAndesite => "polished_andesite",
            Wood(species) => species.to_str(),
            Cobble => "cobblestone",
            MossyCobble => "mossy_cobblestone",
            Brick => "brick",
            StoneBrick => "stone_brick",
            MossyStonebrick => "mossy_stone_brick",
            Sandstone => "sandstone",
            SmoothSandstone => "smooth_sandstone",
            RedSandstone => "red_sandstone",
            SmoothRedSandstone => "smooth_red_sandstone",
            Blackstone => "blackstone",
            PolishedBlackstone => "polished_blackstone",
            PolishedBlackstoneBrick => "polished_blackstone_brick",
            PackedMud => "packed_mud",
            MudBrick => "mud_bricks",
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BellAttachment {
    Floor,
    Ceiling,
    SingleWall,
    DoubleWall,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Blockstate(
    pub Cow<'static, str>,
    pub Vec<(Cow<'static, str>, Cow<'static, str>)>,
);

impl Display for Blockstate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[", self.0)?;
        for (name, state) in self.1.iter() {
            write!(f, "{}={},", name, state)?;
        }
        write!(f, "]")
    }
}

impl Blockstate {
    pub fn item_snbt(&self) -> String {
        let mut str = format!("{{id:\"{}\",Count:1,tag:{{BlockStateTag:{{", self.0);
        for (prop, value) in &self.1 {
            write!(str, "{prop}:\"{value}\",").unwrap();
        }
        str.push_str("}}}");
        str
    }
}

impl Block {
    // TODO: for fences & similar, emit block_ticks to make MC updateblockstates
    pub fn blockstate(&self, unknown: &UnknownBlocks) -> Blockstate {
        impl<Name: Into<Cow<'static, str>>> From<Name> for Blockstate {
            fn from(name: Name) -> Self {
                Self(name.into(), vec![])
            }
        }

        match self {
            Air => "air".into(),
            Full(material) => match material {
                Brick => "bricks".into(),
                StoneBrick => "stone_bricks".into(),
                MudBrick => "mud_bricks".into(),
                PolishedBlackstoneBrick => "polished_blackstone_bricks".into(),
                material => material.to_str().into(),
            },
            Planks(species) => format!("{}_planks", species).into(),
            Soil(soil_type) => match soil_type {
                Soil::Grass => "grass_block".into(),
                Soil::Dirt => "dirt".into(),
                Soil::Sand => "sand".into(),
                Soil::Gravel => "gravel".into(),
                Soil::Farmland => "farmland".into(),
                Soil::Path => "grass_path".into(),
                Soil::CoarseDirt => "coarse_dirt".into(),
                Soil::Podzol => "podzol".into(),
                Soil::SoulSand => "soul_sand".into(),
            },
            Bedrock => "bedrock".into(),
            Water => "water".into(),
            Lava => "lava".into(),
            Log(species, log_type) => match log_type {
                LogType::Normal(axis) => Blockstate(
                    match species {
                        Warped | Crimson => format!("{}_stem", species),
                        _ => format!("{}_log", species),
                    }
                    .into(),
                    vec![("axis".into(), axis.to_str().into())],
                ),
                LogType::FullBark => Blockstate(
                    match species {
                        Warped | Crimson => format!("{}_hyphae", species),
                        _ => format!("{}_wood", species),
                    }
                    .into(),
                    vec![],
                ),
                LogType::Stripped(axis) => Blockstate(
                    match species {
                        Warped | Crimson => format!("{}_stem", species),
                        _ => format!("stripped_{}_log", species),
                    }
                    .into(),
                    vec![("axis".into(), axis.to_str().into())],
                ),
            },
            Leaves(species, distance) => Blockstate(
                format!("{}_leaves", species).into(),
                if let Some(distance) = distance {
                    vec![
                        ("persistent".into(), "false".into()),
                        ("distance".into(), distance.to_string().into()),
                    ]
                } else {
                    vec![("persistent".into(), "true".into())]
                },
            ),
            GroundPlant(plant) => match plant {
                GroundPlant::Sapling(species) => format!("{}_sapling", species).into(),
                GroundPlant::Small(plant) => match plant {
                    SmallPlant::Grass => "grass".into(),
                    SmallPlant::Fern => "fern".into(),
                    SmallPlant::DeadBush => "dead_bush".into(),
                    SmallPlant::Dandelion => "dandelion".into(),
                    SmallPlant::Poppy => "poppy".into(),
                    SmallPlant::BlueOrchid => "blue_orchid".into(),
                    SmallPlant::Allium => "allium".into(),
                    SmallPlant::AzureBluet => "azure_bluet".into(),
                    SmallPlant::RedTulip => "red_tulip".into(),
                    SmallPlant::OrangeTulip => "orange_tulip".into(),
                    SmallPlant::WhiteTulip => "white_tulip".into(),
                    SmallPlant::PinkTulip => "pink_tulip".into(),
                    SmallPlant::OxeyeDaisy => "oxeye_daisy".into(),
                    SmallPlant::BrownMushroom => "brown_mushroom".into(),
                    SmallPlant::RedMushroom => "red_mushroom".into(),
                },
                GroundPlant::Cactus => "cactus".into(),
                GroundPlant::Reeds => "sugar_cane".into(),
                GroundPlant::Pumpkin => "pumpkin".into(),
                GroundPlant::Tall(plant, Flipped(upper)) => Blockstate(
                    match plant {
                        TallPlant::Sunflower => "sunflower".into(),
                        TallPlant::Lilac => "lilac".into(),
                        TallPlant::Grass => "tall_grass".into(),
                        TallPlant::Fern => "large_fern".into(),
                        TallPlant::Rose => "rose_bush".into(),
                        TallPlant::Peony => "peony".into(),
                    },
                    vec![("half".into(), if *upper { "upper" } else { "lower" }.into())],
                ),
                GroundPlant::Crop(crop) => match crop {
                    Crop::Wheat => Blockstate("wheat".into(), vec![("age".into(), "7".into())]),
                    Crop::Carrot => Blockstate("carrot".into(), vec![("age".into(), "7".into())]),
                    Crop::Potato => Blockstate("potato".into(), vec![("age".into(), "7".into())]),
                    Crop::Beetroot => {
                        Blockstate("beetroot".into(), vec![("age".into(), "3".into())])
                    }
                },
            },
            Fence(material) => match material {
                Wood(species) => format!("{}_fence", species).into(),
                material => format!("{}_wall", material).into(),
            },
            Wool(color) => format!("{}_wool", color).into(),
            Terracotta(Some(color)) => format!("{}_terracotta", color).into(),
            Terracotta(None) => "terracotta".into(),
            SmoothQuartz => "smooth_quartz".into(),
            SnowLayer => Blockstate("snow".into(), vec![("layers".into(), "1".into())]),
            Glowstone => "glowstone".into(),
            GlassPane(color) => {
                if let Some(color) = color {
                    format!("{}_stained_glass_pane", color).into()
                } else {
                    "glass_pane".into()
                }
            }
            WallBanner(facing, color) => Blockstate(
                format!("{}_wall_banner", color).into(),
                vec![("facing".into(), facing.to_str().into())],
            ),
            Hay => "hay_block".into(),
            Slab(material, Flipped(flipped)) => Blockstate(
                format!("{}_slab", material).into(),
                vec![(
                    "type".into(),
                    if *flipped { "top" } else { "bottom" }.into(),
                )],
            ),
            Stair(material, dir, Flipped(flipped)) => Blockstate(
                format!("{}_stairs", material).into(),
                vec![
                    (
                        "half".into(),
                        if *flipped { "top" } else { "bottom" }.into(),
                    ),
                    ("facing".into(), dir.to_str().into()),
                ],
            ),
            Cauldron { water } => Blockstate(
                "cauldron".into(),
                vec![(
                    "level".into(),
                    match water {
                        0 => "0".into(),
                        1 => "1".into(),
                        2 => "2".into(),
                        3 => "3".into(),
                        _ => panic!("Cauldron water level {}", water),
                    },
                )],
            ),
            Trapdoor(species, dir, meta) => Blockstate(
                format!("{}_trapdoor", species).into(),
                vec![
                    ("facing".into(), dir.to_str().into()),
                    (
                        "half".into(),
                        if meta.contains(DoorMeta::TOP) {
                            "top"
                        } else {
                            "bottom"
                        }
                        .into(),
                    ),
                    (
                        "open".into(),
                        format!("{}", meta.contains(DoorMeta::OPEN)).into(),
                    ),
                ],
            ),
            Door(species, dir, meta) => Blockstate(
                format!("{}_door", species).into(),
                vec![
                    ("facing".into(), dir.to_str().into()),
                    (
                        "half".into(),
                        if meta.contains(DoorMeta::TOP) {
                            "upper"
                        } else {
                            "lower"
                        }
                        .into(),
                    ),
                    (
                        "open".into(),
                        format!("{}", meta.contains(DoorMeta::OPEN)).into(),
                    ),
                ],
            ),
            Bell(facing, attachment) => Blockstate(
                "bell".into(),
                vec![
                    ("facing".into(), facing.to_str().into()),
                    (
                        "attachment".into(),
                        match attachment {
                            BellAttachment::Floor => "floor",
                            BellAttachment::Ceiling => "ceiling",
                            BellAttachment::DoubleWall => "double_wall",
                            BellAttachment::SingleWall => "single_wall",
                        }
                        .into(),
                    ),
                ],
            ),
            Repeater(dir, delay) => Blockstate(
                "repeater".into(),
                vec![
                    (
                        "delay".into(),
                        match delay {
                            1 => "1".into(),
                            2 => "2".into(),
                            3 => "3".into(),
                            4 => "4".into(),
                            _ => panic!("Repeater delay {}", delay),
                        },
                    ),
                    ("facing".into(), dir.to_str().into()),
                ],
            ),
            Barrier => "barrier".into(),
            Other(index) => unknown.states[*index as usize].clone(), // Unneccesary clone?
        }
    }

    pub fn tile_entity_nbt(&self, pos: IVec3) -> Option<CompoundTag> {
        match self {
            Bell(..) => {
                let mut nbt = CompoundTag::new();
                nbt.insert_str("id", "bell");
                Some(nbt)
            }
            WallBanner(..) => {
                let mut nbt = CompoundTag::new();
                nbt.insert_str("id", "banner");
                Some(nbt)
            }
            _ => None,
        }
        .map(|mut nbt| {
            nbt.insert_i32("x", pos.x);
            nbt.insert_i32("y", pos.z);
            nbt.insert_i32("z", pos.y);
            nbt
        })
    }

    /// This is for loading of the structure block format and very much incomplete
    /// (and panics on invalid blocks)
    pub fn from_nbt(nbt: &CompoundTag) -> Block {
        let name = nbt.get_str("Name").expect("Invalid block: no name");
        let name = name.strip_prefix("minecraft:").unwrap_or(name);
        let default_props = CompoundTag::new();
        let props = nbt.get_compound_tag("Properties").unwrap_or(&default_props);

        fn slab(material: Material, props: &CompoundTag) -> Block {
            match props.get_str("type").unwrap() {
                "top" => Slab(material, Flipped(true)),
                "double" => Full(material),
                _ => Slab(material, Flipped(false)),
            }
        }

        fn stair(material: Material, props: &CompoundTag) -> Block {
            Stair(
                material,
                HDir::from_str(props.get_str("facing").unwrap()).unwrap(),
                Flipped(props.get_str("half").unwrap() == "top"),
            )
        }

        fn leaves(species: TreeSpecies, props: &CompoundTag) -> Block {
            Leaves(
                species,
                if props.get_str("persistent").unwrap() == "false" {
                    Some(props.get_str("distance").unwrap().parse().unwrap())
                } else {
                    None
                },
            )
        }

        fn wall_banner(color: Color, props: &CompoundTag) -> Block {
            WallBanner(
                HDir::from_str(props.get_str("facing").unwrap()).unwrap(),
                color,
            )
        }

        fn trapdoor(species: TreeSpecies, props: &CompoundTag) -> Block {
            Trapdoor(
                species,
                HDir::from_str(props.get_str("facing").unwrap()).unwrap(),
                {
                    let mut meta = DoorMeta::empty();
                    if props.get_str("half").unwrap() == "top" {
                        meta |= DoorMeta::TOP;
                    }
                    if props.get_str("open").unwrap() == "true" {
                        meta |= DoorMeta::OPEN;
                    }
                    meta
                },
            )
        }

        fn door(species: TreeSpecies, props: &CompoundTag) -> Block {
            Door(
                species,
                HDir::from_str(props.get_str("facing").unwrap()).unwrap(),
                {
                    let mut meta = DoorMeta::empty();
                    if props.get_str("half").unwrap() == "upper" {
                        meta |= DoorMeta::TOP;
                    }
                    if props.get_str("open").unwrap() == "true" {
                        meta |= DoorMeta::OPEN;
                    }
                    meta
                },
            )
        }

        fn half(props: &CompoundTag) -> Flipped {
            Flipped(props.get_str("half").unwrap() == "upper")
        }

        fn log_axis(props: &CompoundTag) -> Axis {
            props.get_str("axis").unwrap().parse().unwrap()
        }

        fn known_block(name: &str, props: &CompoundTag) -> Option<Block> {
            // TODO: expand this
            Some(match name {
                "air" | "cave_air" => Air,
                // Let's ignore flowing water for now, maybe revise later
                "water" => match props.get_str("level") {
                    Ok("0") => Water,
                    _ => Air,
                },
                "stone" => Full(Stone),
                "granite" => Full(Granite),
                "diorite" => Full(Diorite),
                "andesite" => Full(Andesite),
                "cobblestone" => Full(Cobble),
                "bricks" => Full(Brick),
                "stone_bricks" => Full(StoneBrick),
                "mud_bricks" => Full(MudBrick),
                "packed_pud" => Full(PackedMud),
                "bedrock" => Bedrock,
                "gravel" => Soil(Soil::Gravel),
                "grass_block" => Soil(Soil::Grass),
                "sand" => Soil(Soil::Sand),
                "dirt" if props.get_str("variant").is_err() => Soil(Soil::Dirt),
                "dirt" if matches!(props.get_str("variant"), Ok("coarse_dirt")) => {
                    Soil(Soil::CoarseDirt)
                }
                "oak_planks" => Planks(Oak),
                "oak_log" => Log(Oak, LogType::Normal(log_axis(props))),
                "spruce_log" => Log(Spruce, LogType::Normal(log_axis(props))),
                "birch_log" => Log(Birch, LogType::Normal(log_axis(props))),
                "jungle_log" => Log(Jungle, LogType::Normal(log_axis(props))),
                "acacia_log" => Log(Acacia, LogType::Normal(log_axis(props))),
                "dark_oak_log" => Log(DarkOak, LogType::Normal(log_axis(props))),
                "mangrove_log" => Log(Mangrove, LogType::Normal(log_axis(props))),
                "cherry_log" => Log(Cherry, LogType::Normal(log_axis(props))),
                "stripped_oak_log" => Log(Oak, LogType::Stripped(log_axis(props))),
                "oak_leaves" => leaves(Oak, props),
                "spruce_leaves" => leaves(Spruce, props),
                "birch_leaves" => leaves(Birch, props),
                "jungle_leaves" => leaves(Jungle, props),
                "acacie_leaves" => leaves(Acacia, props),
                "dark_oak_leaves" => leaves(DarkOak, props),
                "azalea_leaves" => leaves(Azalea, props),
                "flowering_azalea_leaves" => leaves(FloweringAzalea, props),
                "grass" => GroundPlant(GroundPlant::Small(SmallPlant::Grass)),
                "fern" => GroundPlant(GroundPlant::Small(SmallPlant::Fern)),
                "tall_grass" => GroundPlant(GroundPlant::Tall(TallPlant::Grass, half(props))),
                "large_fern" => GroundPlant(GroundPlant::Tall(TallPlant::Fern, half(props))),
                "sunflower" => GroundPlant(GroundPlant::Tall(TallPlant::Sunflower, half(props))),
                "lilac" => GroundPlant(GroundPlant::Tall(TallPlant::Lilac, half(props))),
                "rose_bush" => GroundPlant(GroundPlant::Tall(TallPlant::Rose, half(props))),
                "peony" => GroundPlant(GroundPlant::Tall(TallPlant::Peony, half(props))),
                "snow" => SnowLayer, // Todo: store layer
                "fence" => Fence(Wood(Oak)),
                "cobblestone_wall" => Fence(MossyCobble),
                "mossy_cobblestone_wall" => Fence(MossyCobble),
                "oak_slab" => slab(Wood(Oak), props),
                "spruce_slab" => slab(Wood(Spruce), props),
                "birch_slab" => slab(Wood(Birch), props),
                "jungle_slab" => slab(Wood(Jungle), props),
                "acacia_slab" => slab(Wood(Acacia), props),
                "dark_oak_slab" => slab(Wood(DarkOak), props),
                "cobblestone_slab" => slab(Cobble, props),
                "mossy_cobblestone_slab" => slab(MossyCobble, props),
                "stone_brick_slab" => slab(StoneBrick, props),
                "mossy_stone_brick_slab" => slab(MossyStonebrick, props),
                "blackstone_slab" => slab(Blackstone, props),
                "polished_blackstone_slab" => slab(PolishedBlackstone, props),
                "mud_brick_slab" => slab(MudBrick, props),
                "oak_stairs" => stair(Wood(Oak), props),
                "spruce_stairs" => stair(Wood(Spruce), props),
                "birch_stairs" => stair(Wood(Birch), props),
                "jungle_stairs" => stair(Wood(Jungle), props),
                "acacia_stairs" => stair(Wood(Acacia), props),
                "dark_oak_stairs" => stair(Wood(DarkOak), props),
                "cobblestone_stairs" => stair(Cobble, props),
                "stone_brick_stairs" => stair(StoneBrick, props),
                "blackstone_stairs" => stair(Blackstone, props),
                "mud_brick_stairs" => stair(MudBrick, props),
                "terracotta" => Terracotta(None),
                "white_terracotta" => Terracotta(Some(White)),
                "orange_terracotta" => Terracotta(Some(Orange)),
                "magenta_terracotta" => Terracotta(Some(Magenta)),
                "light_blue_terracotta" => Terracotta(Some(LightBlue)),
                "yellow_terracotta" => Terracotta(Some(Yellow)),
                "lime_terracotta" => Terracotta(Some(Lime)),
                "pink_terracotta" => Terracotta(Some(Pink)),
                "gray_terracotta" => Terracotta(Some(Gray)),
                "light_gray_terracotta" => Terracotta(Some(LightGray)),
                "cyan_terracotta" => Terracotta(Some(Cyan)),
                "purple_terracotta" => Terracotta(Some(Purple)),
                "blue_terracotta" => Terracotta(Some(Blue)),
                "brown_terracotta" => Terracotta(Some(Brown)),
                "green_terracotta" => Terracotta(Some(Green)),
                "red_terracotta" => Terracotta(Some(Red)),
                "black_terracotta" => Terracotta(Some(Black)),
                "cauldron" => Cauldron {
                    water: props.get_str("level").unwrap().parse().unwrap(),
                },
                "oak_trapdoor" => trapdoor(Oak, props),
                "spruce_trapdoor" => trapdoor(Spruce, props),
                "oak_door" => door(Oak, props),
                "spruce_door" => door(Spruce, props),
                "bell" => Bell(
                    HDir::from_str(props.get_str("facing").unwrap()).unwrap(),
                    match props.get_str("attachment").unwrap() {
                        "floor" => BellAttachment::Floor,
                        "ceiling" => BellAttachment::Ceiling,
                        "single_wall" => BellAttachment::SingleWall,
                        _ => BellAttachment::DoubleWall,
                    },
                ),
                "red_wall_banner" => wall_banner(Red, props),
                "white_wall_banner" => wall_banner(Red, props),
                "blue_wall_banner" => wall_banner(Red, props),
                "green_wall_banner" => wall_banner(Red, props),
                "yellow_wall_banner" => wall_banner(Red, props),
                _ => return None,
            })
        }

        if let Some(known) = known_block(name, props) {
            return known;
        }

        let blockstate = Blockstate(
            name.to_owned().into(),
            if let Ok(props) = nbt.get_compound_tag("Properties") {
                props
                    .iter()
                    .map(|(name, value)| {
                        (
                            name.clone().into(),
                            if let nbt::Tag::String(value) = value {
                                value.clone().into()
                            } else {
                                panic!("Non-string blockstate value")
                            },
                        )
                    })
                    .collect()
            } else {
                Vec::new()
            },
        );

        thread_local! {
            static THREAD_PALETTE: RefCell<HashMap<Blockstate, u16>> = default();
        }

        THREAD_PALETTE.with_borrow_mut(|thread_palette| {
            Other(if let Some(&index) = thread_palette.get(&blockstate) {
                index
            } else {
                let mut unknown_blocks = UNKNOWN_BLOCKS.write().unwrap();
                if let Some(&index) = unknown_blocks.map.get(&blockstate) {
                    index
                } else {
                    let index = unknown_blocks.states.len() as u16;
                    unknown_blocks.map.insert(blockstate.clone(), index);
                    unknown_blocks.states.push(blockstate);
                    index
                }
            })
        })
    }

    pub fn to_nbt(&self, unknown: &UnknownBlocks) -> CompoundTag {
        let blockstate = self.blockstate(unknown);
        let mut nbt = CompoundTag::new();
        nbt.insert("Name", blockstate.0.into_owned());
        if !blockstate.1.is_empty() {
            nbt.insert("Properties", {
                let mut props = CompoundTag::new();
                for (prop, value) in blockstate.1 {
                    props.insert_str(prop, value);
                }
                props
            });
        }
        nbt
    }

    pub fn solid(&self) -> bool {
        // Todo: expand this
        !matches!(
            self,
            Air | Water | Lava | GroundPlant(..) | Leaves(..) | SnowLayer
        )
    }

    pub fn rotated(self, turns: u8) -> Self {
        match self {
            Log(species, LogType::Normal(Axis::X)) => Log(species, LogType::Normal(Axis::Y)),
            Log(species, LogType::Normal(Axis::Y)) => Log(species, LogType::Normal(Axis::X)),
            Stair(material, facing, flipped) => Stair(material, facing.rotated(turns), flipped),
            WallBanner(facing, color) => WallBanner(facing.rotated(turns), color),
            Repeater(dir, delay) => Repeater(dir.rotated(turns), delay),
            Trapdoor(species, dir, meta) => Trapdoor(species, dir.rotated(turns), meta),
            Door(species, dir, meta) => Door(species, dir.rotated(turns), meta),
            _ => self,
        }
    }

    pub fn swap_wood_type(self, species: TreeSpecies) -> Self {
        match self {
            Full(Wood(Oak)) => Full(Wood(species)),
            Slab(Wood(Oak), flipped) => Slab(Wood(species), flipped),
            Stair(Wood(Oak), dir, flipped) => Stair(Wood(species), dir, flipped),
            Planks(Oak) => Planks(species),
            Fence(Wood(Oak)) => Fence(Wood(species)),
            Log(Oak, typ) => Log(species, typ),
            Leaves(Oak, dist) => Leaves(species, dist),
            Trapdoor(Oak, dir, meta) => Trapdoor(species, dir, meta),
            Door(Oak, dir, meta) => Door(species, dir, meta),
            _ => self,
        }
    }
}

impl std::ops::BitOr<Block> for Block {
    type Output = Self;

    fn bitor(self, rhs: Block) -> Self::Output {
        if self.solid() {
            self
        } else {
            rhs
        }
    }
}

impl std::ops::BitOrAssign for Block {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs
    }
}
