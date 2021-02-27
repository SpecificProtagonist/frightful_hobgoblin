use std::fmt::Display;

use crate::*;
use nbt::{CompoundTag, Tag};

pub use Chattel::*;
use EntityType::*;

/*
 * UUIDs
 * 0-0-0-{villager id}-0           Villager
 * 0-0-1-{villager id}-{action id} Marker für Villager-Actions
 * 0-0-2-{villager id}-0           carried block (armor stand)
 * all others simply not saved
 */

pub struct Entity {
    pub id: Option<EntityID>,
    pub pos: Pos,
    pub data: EntityType,
    pub tags: Vec<String>,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct EntityID(pub u32, pub u16, pub u16, pub u16, pub u64);

pub enum EntityType {
    Chattel(Chattel),
    Villager {
        name: String,
        biome: Biome,
        profession: Profession,
    },
    Cat,
    Marker,
}

pub enum Chattel {
    Chicken,
    Cattle,
    Pig,
    Sheep(Color),
    Llama,
}

pub enum Profession {
    Armorer,
    Butcher,
    Cartographer,
    Cleric,
    Farmer,
    Fisherman,
    Fletcher,
    Leatherworker,
    Librarian,
    Nitwit,
    Mason,
    Toolsmith,
    Weaponsmith,
}

impl Entity {
    pub fn to_nbt(&self) -> CompoundTag {
        let mut nbt = CompoundTag::new();
        nbt.insert_str(
            "id",
            match &self.data {
                Chattel(Chicken) => "chicken",
                Chattel(Cattle) => "cow",
                Chattel(Pig) => "pig",
                Chattel(Sheep(..)) => "sheep",
                Chattel(Llama) => "llama",
                EntityType::Villager { .. } => "villager",
                Cat => "cat",
                Marker { .. } => "armor_stand",
            },
        );

        if let Some(id) = self.id {
            write_uuid(&mut nbt, id);
        }

        nbt.insert(
            "Pos",
            Tag::List(vec![
                Tag::Double(self.pos.0 as f64 + 0.5),
                Tag::Double(self.pos.1 as f64),
                Tag::Double(self.pos.2 as f64 + 0.5),
            ]),
        );

        nbt.insert_str_vec("Tags", self.tags.iter());

        match &self.data {
            Chattel(Sheep(color)) => {
                nbt.insert_i8("Color", *color as i8);
            }
            EntityType::Villager {
                name,
                biome,
                profession,
            } => {
                nbt.insert_str("CustomName", &name);
                nbt.insert_compound_tag("VillagerData", {
                    let mut data = CompoundTag::new();
                    data.insert_str(
                        "profession",
                        match profession {
                            Profession::Armorer => "minecraft:armorer",
                            Profession::Butcher => "minecraft:butcher",
                            Profession::Cartographer => "minecraft:carographer",
                            Profession::Cleric => "minecraft:cleric",
                            Profession::Farmer => "minecraft:farmer",
                            Profession::Fisherman => "minecraft:fisherman",
                            Profession::Fletcher => "minecraft:fletcher",
                            Profession::Leatherworker => "minecraft:leatherworker",
                            Profession::Librarian => "minecraft:librarian",
                            Profession::Mason => "minecraft:mason",
                            Profession::Nitwit => "minecraft:nitwit",
                            Profession::Toolsmith => "minecraft:toolsmith",
                            Profession::Weaponsmith => "minecraft:weaponsmith",
                        },
                    );
                    data.insert_str(
                        "type",
                        match biome {
                            Biome { base: Swamp, .. } => "minecraft:swamp",
                            Biome { base: Savanna, .. } => "minecraft:savanna",
                            Biome { base: Jungle, .. } => "minecraft:jungle",
                            Biome { base: Desert, .. } => "minecraft:desert",
                            Biome { base: Taiga, .. } => "minecraft:taige",
                            Biome { temp: Cold, .. } => "minecraft:snow",
                            Biome { .. } => "minecraft:plains",
                        },
                    );
                    data
                });
                nbt.insert_compound_tag_vec(
                    "Attributes",
                    vec![
                        {
                            // Disable movement
                            let mut attr = CompoundTag::new();
                            attr.insert_str("Name", "generic.movementSpeed");
                            attr.insert_f64("Base", 0.0);
                            attr
                        },
                        {
                            // Ignore knockback since we can't take it into account
                            let mut attr = CompoundTag::new();
                            attr.insert_str("Name", "generic.knockbackResistance");
                            attr.insert_f64("Base", 1.0);
                            attr
                        },
                        {
                            // Make them a bit beefier in case somthing goes wrong
                            // TODO: make this actually work
                            let mut attr = CompoundTag::new();
                            attr.insert_str("Name", "generic.maxHealth");
                            attr.insert_f64("Base", 10000.0);
                            attr
                        },
                    ],
                );

                // NoAI allows us to specify where villagers are looking,
                // but also disables motion :/
                nbt.insert_bool("NoAI", true);

                // Disable trades
                // Todo: add fitting trades
                nbt.insert_compound_tag("Offers", {
                    let mut offers = CompoundTag::new();
                    offers.insert_compound_tag_vec("Recipes", Vec::new());
                    offers
                });
            }
            Marker => {
                nbt.insert_bool("Marker", true);
                nbt.insert_bool("NoGravity", true);
                nbt.insert_bool("Invisible", true);
            }
            _ => (),
        }

        nbt
    }
}

fn write_uuid(nbt: &mut CompoundTag, id: EntityID) {
    nbt.insert_i64(
        "UUIDMost",
        ((id.0 as i64) << 32) + ((id.1 as i64) << 16) + id.2 as i64,
    );
    nbt.insert_i64("UUIDLeast", ((id.3 as i64) << 48) + id.4 as i64);
}

impl Display for EntityID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}-{}-{}-{}", self.0, self.1, self.2, self.3, self.4)
    }
}
