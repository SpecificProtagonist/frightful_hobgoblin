use std::{collections::HashMap, fs::File, path::Path, sync::Mutex};

use lazy_static::lazy_static;
use nbt::{decode::read_gzip_compound_tag, CompoundTag, CompoundTagError, Tag};

use crate::*;

pub mod castle;
pub mod dzong;
pub mod farm;

#[derive(Clone)]
pub struct TemplateMark(Vec3, Option<HDir>, Vec<String>);

// Hand-build structure, stored via structure blocks
#[derive(Clone)]
pub struct Template {
    name: String,
    size: Vec3,
    blocks: HashMap<Vec3, Block>,
    markers: HashMap<String, TemplateMark>,
}

// Cache the templates
lazy_static! {
    static ref TEMPLATES: Mutex<HashMap<String, &'static Template>> = Default::default();
}

impl Template {
    pub fn get(name: &str) -> &'static Self {
        let mut templates = TEMPLATES.lock().unwrap();
        templates
            .entry(name.into())
            .or_insert_with(|| Box::leak(Box::new(Self::load(name))))
    }

    /// Panics when file is not found, isn't a valid structure or contains unknown blocks
    /// (since file is not specified by user)
    fn load(name: &str) -> Self {
        let mut path = Path::new("templates").join(name);
        path.set_extension("nbt");
        let mut file = File::open(&path).expect(&format!("Structure file {:?} not found", path));
        let nbt = read_gzip_compound_tag(&mut file).expect(&format!("Invalid nbt: {:?}", path));

        Self::load_from_nbt(&nbt, name)
            .unwrap_or_else(|err| panic!("Invalid structure {:?}: {:?}", path, err))
    }

    /// Can also panic, but eh, won't happen when the user is executing the program
    /// Oh, and of course CompountTagError holds a reference to the original tag
    /// so I can't just use anyhow (TODO: PR)
    fn load_from_nbt<'a>(
        nbt: &'a CompoundTag,
        name: &'a str,
    ) -> Result<Template, CompoundTagError<'a>> {
        fn read_pos(nbt: &Vec<Tag>) -> Vec3 {
            match [&nbt[0], &nbt[1], &nbt[2]] {
                [Tag::Int(x), Tag::Int(y), Tag::Int(z)] => Vec3(*x, *y, *z),
                _ => panic!(),
            }
        }

        let size = read_pos(nbt.get("size")?);

        // Look for markers such as the origin
        let markers: HashMap<_, _> = nbt
            .get_compound_tag_vec("entities")?
            .iter()
            .filter_map(|nbt| {
                let pos = read_pos(nbt.get("blockPos").unwrap());
                let nbt = nbt.get_compound_tag("nbt").unwrap();
                if let Ok("minecraft:armor_stand") = nbt.get_str("id") {
                    let tags: Vec<String> = nbt
                        .get_str_vec("Tags")
                        .unwrap_or(Vec::new())
                        .iter()
                        .map(|tag| (*tag).to_owned())
                        .collect();
                    // For some reason, CustomName doesn't work anymore?
                    let name = tags
                        .iter()
                        .find(|tag| tag.starts_with("name:"))
                        .expect("Unnamed marker")
                        .strip_prefix("name:")
                        .unwrap()
                        .to_owned();

                    let dir = if tags.contains(&String::from("xpos")) {
                        Some(HDir::XPos)
                    } else if tags.contains(&String::from("xneg")) {
                        Some(HDir::XNeg)
                    } else if tags.contains(&String::from("zpos")) {
                        Some(HDir::ZPos)
                    } else if tags.contains(&String::from("zneg")) {
                        Some(HDir::ZNeg)
                    } else {
                        None
                    };
                    Some((name, TemplateMark(pos, dir, tags)))
                } else {
                    None
                }
            })
            .collect();

        let origin = markers
            .get("origin")
            .expect(&format!("Failed to load template {}: No origin set", name))
            .0;

        let palette: Vec<Block> = nbt
            .get_compound_tag_vec("palette")?
            .iter()
            .map(|nbt| Block::from_nbt(nbt))
            .collect();

        let mut blocks = HashMap::new();

        for nbt in nbt.get_compound_tag_vec("blocks")? {
            let pos = read_pos(nbt.get("pos")?);
            let block = (&palette[nbt.get_i32("state")? as usize]).clone();
            // TODO: nbt data
            blocks.insert(pos - origin, block);
        }

        Ok(Self {
            name: name.to_owned(),
            size,
            blocks,
            markers,
        })
    }

    pub fn build(&self, world: &mut impl WorldView, pos: Pos, facing: HDir) {
        let rotation = facing as u8 + 4 - self.markers["origin"].1.unwrap() as u8;
        // TODO: better build order
        for (offset, block) in self.blocks.iter() {
            world.set(pos + offset.rotated(rotation), block.rotated(rotation));
        }
    }

    pub fn build_clipped(&self, world: &mut impl WorldView, pos: Pos, facing: HDir, area: Rect) {
        let rotation = facing as u8 + 4 - self.markers["origin"].1.unwrap() as u8;
        for (offset, block) in self.blocks.iter() {
            let pos = pos + offset.rotated(rotation);
            if area.contains(Column(pos.0, pos.2)) {
                world.set(pos, block.rotated(rotation));
            }
        }
    }

    // TODO: palette swap
}
