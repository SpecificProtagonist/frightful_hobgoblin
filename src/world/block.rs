use std::{borrow::Cow, fmt::Display, sync::Arc};

pub use self::GroundPlant::*;
use crate::geometry::*;
use nbt::{CompoundTag, CompoundTagError};
use num_derive::FromPrimitive;

pub use Block::*;
pub use Color::*;
pub use TreeSpecies::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum Block {
    Air,
    Stone(Stone),
    Planks(TreeSpecies),
    Water,
    Lava,
    Soil(Soil),
    Log(TreeSpecies, LogType),
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
    Other(Arc<Blockstate>),
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

impl TreeSpecies {
    pub fn from_str(name: &str) -> Option<TreeSpecies> {
        match name {
            "oak" => Some(TreeSpecies::Oak),
            "spruce" => Some(TreeSpecies::Spruce),
            "birch" => Some(TreeSpecies::Birch),
            "jungle" => Some(TreeSpecies::Jungle),
            "acacia" => Some(TreeSpecies::Acacia),
            "dark_oak" => Some(TreeSpecies::DarkOak),
            _ => None,
        }
    }

    pub fn to_str(self) -> &'static str {
        match self {
            Oak => "oak",
            Spruce => "spruce",
            Birch => "birch",
            Jungle => "jungle",
            Acacia => "acacia",
            DarkOak => "dark_oak",
        }
    }
}

impl Display for TreeSpecies {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
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

// TODO: expand and replace with BuildBlock
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

impl Display for BuildBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildBlock::Wooden(species) => species.fmt(f),
            BuildBlock::Cobble => write!(f, "cobblestone"),
            BuildBlock::Brick => write!(f, "brick"),
            BuildBlock::Stonebrick => write!(f, "stone_brick"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Blockstate(
    pub Cow<'static, str>,
    pub Vec<(Cow<'static, str>, Cow<'static, str>)>,
);

impl Display for Blockstate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)?;
        if self.1.len() > 0 {
            write!(f, "[")?;
            for (i, (name, state)) in self.1.iter().enumerate() {
                write!(f, "{}={}", name, state)?;
                if i + 1 < self.1.len() {
                    write!(f, ",")?;
                }
            }
            write!(f, "]")?;
        }
        Ok(())
    }
}

impl Block {
    pub fn blockstate(&self) -> Blockstate {
        impl<Name: Into<Cow<'static, str>>> From<Name> for Blockstate {
            fn from(name: Name) -> Self {
                Self(name.into(), vec![])
            }
        }

        match self {
            Air => "air".into(),
            Stone(stone) => match stone {
                Stone::Stone => "stone".into(),
                Stone::Granite => "granite".into(),
                Stone::Diorite => "diorite".into(),
                Stone::Andesite => "andesite".into(),
                Stone::Cobble => "cobblestone".into(),
                Stone::Brick => "bricks".into(),
                Stone::Stonebrick => "stone_bricks".into(),
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
            // TODO: water level
            Water => "water".into(),
            Lava => "lava".into(),
            Log(species, log_type) => match log_type {
                LogType::Normal(axis) => Blockstate(
                    format!("{}_log", species).into(),
                    vec![("axis".into(), axis.to_str().into())],
                ),
                LogType::FullBark => Blockstate(format!("{}_wood", species).into(), vec![]),
            },
            Leaves(species) => Blockstate(
                format!("{}_leaves", species).into(),
                vec![("persistent".into(), "true".into())],
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
            Fence(fence) => match fence {
                Fence::Wood(species) => format!("{}_fence", species).into(),
                Fence::Stone { mossy: false } => "cobblestone_wall".into(),
                Fence::Stone { mossy: true } => "mossy_cobblestone_wall".into(),
            },
            Wool(color) => format!("{}_wool", color).into(),
            SnowLayer => Blockstate("snow".into(), vec![("layers".into(), "1".into())]),
            Glowstone => "glowstone".into(),
            GlassPane(color) => {
                if let Some(color) = color {
                    format!("{}_stained_glass_pane", color).into()
                } else {
                    "glass_pane".into()
                }
            }
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
            CommandBlock(_) => "command_block".into(),
            Other(blockstate) => (**blockstate).clone(), // Unneccesary clone?
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

    /// This is for loading of the structure block format and very much incomplete
    /// (and panics on invalid blocks)
    pub fn from_nbt(nbt: &CompoundTag) -> Block {
        let name = nbt.get_str("Name").expect("Invalid block: no name");
        let name = name.strip_prefix("minecraft:").unwrap_or(name);
        let default_props = CompoundTag::new();
        let props = nbt.get_compound_tag("Properties").unwrap_or(&default_props);

        fn stair(material: BuildBlock, props: &CompoundTag) -> Block {
            Stair(
                material,
                HDir::from_str(props.get_str("facing").unwrap()).unwrap(),
                Flipped(props.get_str("half").unwrap() == "top"),
            )
        }

        fn log(species: TreeSpecies, props: &CompoundTag) -> Block {
            Log(
                species,
                match props.get_str("axis").unwrap() {
                    "x" => LogType::Normal(Axis::X),
                    "y" => LogType::Normal(Axis::Y),
                    "z" => LogType::Normal(Axis::Z),
                    "none" => LogType::FullBark,
                    unknown => panic!("Invalid log axis {}", unknown),
                },
            )
        }

        fn known_block<'a>(
            name: &str,
            props: &'a CompoundTag,
        ) -> Result<Block, CompoundTagError<'a>> {
            // TMP
            //return Err(CompoundTagError::TagNotFound { name: "" });
            // TODO: expand this
            Ok(match name {
                "air" | "cave_air" => Air,
                "stone" => Stone(Stone::Stone),
                "granite" => Stone(Stone::Granite),
                "diorite" => Stone(Stone::Diorite),
                "andesite" => Stone(Stone::Andesite),
                "cobblestone" => Stone(Stone::Cobble),
                "bricks" => Stone(Stone::Brick),
                "stone_bricks" => Stone(Stone::Stonebrick),
                "bedrock" => Bedrock,
                "gravel" => Soil(Soil::Gravel),
                "grass_block" => Soil(Soil::Grass),
                "sand" => Soil(Soil::Sand),
                "dirt" if matches!(props.get_str("variant"), Err(_)) => Soil(Soil::Dirt),
                "dirt" if matches!(props.get_str("variant")?, "coarse_dirt") => {
                    Soil(Soil::CoarseDirt)
                }
                "oak_log" => log(TreeSpecies::Oak, props),
                "spruce_log" => log(TreeSpecies::Spruce, props),
                "birch_log" => log(TreeSpecies::Birch, props),
                "jungle_log" => log(TreeSpecies::Jungle, props),
                "acacia_log" => log(TreeSpecies::Acacia, props),
                "dark_oak_log" => log(TreeSpecies::DarkOak, props),
                "oak_leaves" => Leaves(TreeSpecies::Oak),
                "spruce_leaves" => Leaves(TreeSpecies::Spruce),
                "birch_leaves" => Leaves(TreeSpecies::Birch),
                "jungle_leaves" => Leaves(TreeSpecies::Jungle),
                "acacie_leaves" => Leaves(TreeSpecies::Acacia),
                "dark_oak_leaves" => Leaves(TreeSpecies::DarkOak),
                "grass" => GroundPlant(GroundPlant::Small(SmallPlant::Grass)),
                "fence" => Fence(Fence::Wood(TreeSpecies::Oak)),
                "cobblestone_wall" if matches!(props.get_str("variant")?, "cobblestone") => {
                    Fence(Fence::Stone { mossy: false })
                }
                "wooden_slab" => Slab(
                    BuildBlock::Wooden(TreeSpecies::from_str(props.get_str("variant")?).unwrap()),
                    Flipped(props.get_str("half")? == "top"),
                ),
                "oak_stairs" => stair(BuildBlock::Wooden(TreeSpecies::Oak), props),
                "stone_brick_stairs" => stair(BuildBlock::Stonebrick, props),
                "cauldron" => Cauldron {
                    water: props.get_str("level")?.parse().unwrap(),
                },
                "wooden_door" => {
                    // TODO
                    Air
                }
                // This is quite hacky, maybe just use anyhow?
                _ => Err(CompoundTagError::TagNotFound {
                    name: "this is an unknown block",
                })?,
            })
        }

        known_block(name, props).unwrap_or_else(|_| {
            Other(Arc::new(Blockstate(
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
            )))
        })
    }

    pub fn to_nbt(&self) -> CompoundTag {
        let blockstate = self.blockstate();
        let mut nbt = CompoundTag::new();
        nbt.insert("Name", blockstate.0.into_owned());
        if blockstate.1.len() > 0 {
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

    pub fn rotated(&self, turns: u8) -> Self {
        match self {
            Log(species, LogType::Normal(Axis::X)) => Log(*species, LogType::Normal(Axis::Z)),
            Log(species, LogType::Normal(Axis::Z)) => Log(*species, LogType::Normal(Axis::X)),
            Stair(material, facing, flipped) => Stair(*material, facing.rotated(turns), *flipped),
            Repeater(dir, delay) => Repeater(dir.rotated(turns), *delay),
            _ => self.clone(),
        }
    }
}
