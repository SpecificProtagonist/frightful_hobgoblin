use std::{
    collections::VecDeque,
    ffi::OsString,
    fs::{read_dir, File},
    path::PathBuf,
    sync::LazyLock,
};

use nbt::{decode::read_gzip_compound_tag, CompoundTag, Tag};

use crate::{config::TEMPLATE_PATH, *};

#[derive(Clone, Debug)]
pub struct TemplateMark(IVec3, Option<HDir>, Vec<String>);

// Hand-build structure, stored via structure blocks
#[derive(Clone)]
pub struct Prefab {
    _size: IVec3,
    blocks: VecDeque<(IVec3, Block)>,
    markers: HashMap<String, TemplateMark>,
}

impl Prefab {
    pub fn build(&self, level: &mut Level, pos: IVec3, facing: HDir, wood: TreeSpecies) {
        let rotation = facing as i32 + 4 - self.markers["origin"].1.unwrap() as i32;
        for (offset, block) in self.blocks.iter() {
            level(
                pos + offset.rotated(rotation),
                block.rotated(rotation).swap_wood_type(wood),
            );
        }
    }

    pub fn build_clipped(&self, level: &mut Level, pos: IVec3, facing: HDir, area: Rect) {
        let rotation = facing as i32 + 4 - self.markers["origin"].1.unwrap() as i32;
        for (offset, block) in self.blocks.iter() {
            let pos = pos + offset.rotated(rotation);
            if area.contains(pos.truncate()) {
                level(pos, block.rotated(rotation));
            }
        }
    }

    // TODO: palette swap
}

pub static PREFABS: LazyLock<HashMap<String, Prefab>> = LazyLock::new(|| {
    let mut map = HashMap::default();
    load_folder(&mut map, "prefabs".into(), "");
    load_folder(&mut map, TEMPLATE_PATH.into(), "");
    map
});

/// Panics when file is not found, isn't a valid structure or contains unknown blocks
/// (since file is not specified by user)
fn load_folder(map: &mut HashMap<String, Prefab>, folder: PathBuf, path: &str) {
    for entry in read_dir(folder).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            load_folder(
                map,
                entry.path(),
                &format!("{path}{}/", entry.file_name().into_string().unwrap()),
            );
        } else if entry.path().extension() == Some(&OsString::from("nbt")) {
            let name = format!(
                "{path}{}",
                entry.path().file_stem().unwrap().to_string_lossy()
            );
            let mut file = File::open(&entry.path()).unwrap();
            let nbt = read_gzip_compound_tag(&mut file)
                .unwrap_or_else(|_| panic!("Invalid nbt: {:?}", path));
            let prefab = load_from_nbt(&nbt, &name);
            map.insert(name, prefab);
        }
    }
}

/// Can also panic, but eh, won't happen when the user is executing the program
/// Oh, and of course CompountTagError holds a reference to the original tag
/// so I can't just use anyhow (TODO: PR)
fn load_from_nbt(nbt: &CompoundTag, name: &str) -> Prefab {
    #[allow(clippy::ptr_arg)]
    fn read_pos(nbt: &Vec<Tag>) -> IVec3 {
        match [&nbt[0], &nbt[1], &nbt[2]] {
            [Tag::Int(x), Tag::Int(z), Tag::Int(y)] => ivec3(*x, *y, *z),
            _ => panic!(),
        }
    }

    let size = read_pos(nbt.get("size").unwrap());

    // Look for markers such as the origin
    let markers: HashMap<_, _> = nbt
        .get_compound_tag_vec("entities")
        .unwrap()
        .iter()
        .filter_map(|nbt| {
            let pos = read_pos(nbt.get("blockPos").unwrap());
            let nbt = nbt.get_compound_tag("nbt").unwrap();
            if let Ok("minecraft:armor_stand") = nbt.get_str("id") {
                let tags: Vec<String> = nbt
                    .get_str_vec("Tags")
                    .unwrap_or_default()
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
                    Some(XPos)
                } else if tags.contains(&String::from("xneg")) {
                    Some(XNeg)
                } else if tags.contains(&String::from("zpos")) {
                    Some(YPos)
                } else if tags.contains(&String::from("zneg")) {
                    Some(YNeg)
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
        .unwrap_or_else(|| panic!("Failed to load prefab {}: No origin set", name))
        .0;

    let palette: Vec<Block> = nbt
        .get_compound_tag_vec("palette")
        .unwrap()
        .iter()
        .map(|nbt| Block::from_nbt(nbt))
        .collect();

    // for block in &palette {
    //     println!("{}", block.blockstate(&UNKNOWN_BLOCKS.read().unwrap()));
    // }

    let mut blocks = VecDeque::new();
    let mut air = VecDeque::new();

    for nbt in nbt
        .get_compound_tag_vec("blocks")
        .unwrap()
        .into_iter()
        .rev()
    {
        let pos = read_pos(nbt.get("pos").unwrap());
        let block = palette[nbt.get_i32("state").unwrap() as usize];
        // TODO: nbt data
        if block == Air {
            // Clear out the area first (from top to bottom)
            air.push_front((pos - origin, Air));
        } else {
            // Then do the building (from bottom to top)
            blocks.push_back((pos - origin, block));
        }
    }
    blocks.extend(air);

    Prefab {
        _size: size,
        blocks,
        markers,
    }
}
