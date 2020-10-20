use nbt::{CompoundTag, Tag};
use crate::geometry::Pos;
use super::block::{Block, Color};
use super::{Biome, BiomeType::*, Temperature::*};

pub use EntityType::*;
pub use Chattel::*;

pub struct Entity {
    pub pos: Pos,
    pub data: EntityType,
    pub id: u32
}

pub enum EntityType {
    Chattel(Chattel),
    Villager { 
        name: String,
        biome: Biome,
        profession: Profession,
        carrying: Option<Block>
    },
    Cat,
    Marker
}

pub enum Chattel {
    Chicken,
    Cow,
    Pig,
    Sheep(Color),
    Llama
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
    Weaponsmith
}

impl Entity {
    pub fn to_nbt(&self) -> CompoundTag {
        let mut nbt = CompoundTag::new();
        nbt.insert_str("id", match self.data {
            Chattel(Chicken) => "chicken",
            Chattel(Cow) => "cow",
            Chattel(Pig) => "pig",
            Chattel(Sheep(..)) => "sheep",
            Chattel(Llama) => "llama",
            Villager {..} => "villager",
            Cat => "cat",
            Marker {..} => "armor_stand",
        });

        nbt.insert("Pos", Tag::List(vec![
            Tag::Double(self.pos.0 as f64),
            Tag::Double(self.pos.1 as f64),
            Tag::Double(self.pos.2 as f64),
        ]));

        nbt.insert_i64("UUIDMost", 0);
        nbt.insert_i64("UUIDLeast", self.id as i64);

        match &self.data {
            Chattel(Sheep(color)) => {
                nbt.insert_i8("Color", *color as i8);
            },
            Villager { name, biome, profession, carrying} => {
                nbt.insert_str("CustomName", &name);
                nbt.insert_compound_tag("VillagerData", {
                    let mut data = CompoundTag::new();
                    data.insert_str("profession", match profession {
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
                        Profession::Weaponsmith => "minecraft:weaponsmith"
                    });
                    data.insert_str("type", match biome {
                        Biome {base: Swamp, ..} => "minecraft:swamp",
                        Biome {base: Savanna, ..} => "minecraft:savanna",
                        Biome {base: Jungle, ..} => "minecraft:jungle",
                        Biome {base: Desert, ..} => "minecraft:desert",
                        Biome {base: Taiga, ..} => "minecraft:taige",
                        Biome {temp: Cold, ..} => "minecraft:snow",
                        Biome {..} => "minecraft:plains"
                    });
                    data
                });
                nbt.insert_compound_tag_vec("Attributes", vec![
                    {
                        // Disable movement
                        let mut attr = CompoundTag::new();
                        attr.insert_str("Name", "generic.movementSpeed");
                        attr.insert_f64("Base", 0.0);
                        attr
                    },{
                        // Ignore knockback since we can't take it into account
                        let mut attr = CompoundTag::new();
                        attr.insert_str("Name", "generic.knockbackResistance");
                        attr.insert_f64("Base", 1.0);
                        attr
                    },{
                        // Make them a bit beefier in case somthing goes wrong
                        let mut attr = CompoundTag::new();
                        attr.insert_str("Name", "generic.maxHealth");
                        attr.insert_f64("Base", 10000.0);
                        attr
                    }
                ]);

                // Adding NoAI would allow us to specify where villagers are looking,
                // but would also disable motion :/
                // nbt.insert_bool("NoAI", true);

                // Disable trades
                // Todo: add fitting trades
                nbt.insert_compound_tag("Offers", {
                    let mut offers = CompoundTag::new();
                    offers.insert_compound_tag_vec("Recipes", Vec::new());
                    offers
                });

                if let Some(carrying) = carrying {
                    nbt.insert_compound_tag_vec("Passengers", vec![{
                        // Create a spacing block, else the carried block would obscure the villager head
                        let mut spacing = CompoundTag::new();
                        spacing.insert_str("id", "falling_block");
                        spacing.insert_i32("TileID", Block::Barrier.to_bytes().0 as i32);
                        // Make sure the block doesn't despawn (as consequence of a failed mc block dupe bugfix)
                        spacing.insert_i32("Time", 1);
                        // Todo: refresh time every 30s in case it gets carried for longer
                        spacing.insert_bool("DropItem", false);
                        spacing.insert_compound_tag_vec("Passengers", vec![{
                            let (block_id, data) = carrying.to_bytes();
                            let mut block = CompoundTag::new();
                            block.insert_str("id", "falling_block");
                            block.insert_i32("TileID", block_id as i32);
                            block.insert_i8("Data", data as i8);
                            block.insert_i32("Time", 1);
                            block.insert_bool("DropItem", false);
                            block
                        }]);
                        spacing
                    }])
                }
            },
            Marker => {
                nbt.insert_bool("Marker", true);
                nbt.insert_bool("NoGravity", true);
            },
            _ => ()
        }

        nbt
    }
}