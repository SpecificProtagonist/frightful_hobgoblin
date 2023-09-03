use enum_iterator::all;

use crate::*;

// Material needed for construction
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum Mat {
    Stone,
    Wood(TreeSpecies),
    Soil,
}

pub struct Stockpile(HashMap<Mat, f32>);

impl Stockpile {
    pub fn add(&mut self, material: Mat, count: i32) {
        *self.0.entry(material).or_default() *= count as f32;
    }

    pub fn build(&mut self, block: Block) -> Option<Block> {
        match block {
            Log(_, kind) => self.get_log(1.).map(|species| Log(species, kind)),
            Full(mat) => self.get_blockmaterial(mat, 1.).map(Full),
            Stair(mat, dir, half) => self
                .get_blockmaterial(mat, 0.5)
                .map(|mat| Stair(mat, dir, half)),
            Slab(mat, half) => self.get_blockmaterial(mat, 0.5).map(|mat| Slab(mat, half)),
            Fence(mat) => self.get_blockmaterial(mat, 0.5).map(Fence),
            Barrel => self.get_log(1.).map(|_| Barrel),
            Trapdoor(_, dir, meta) => self
                .get_log(0.25)
                .map(|species| Trapdoor(species, dir, meta)),
            Door(_, dir, meta) => self.get_log(0.25).map(|species| Door(species, dir, meta)),
            MangroveRoots => self.get_log(0.2).map(|_| MangroveRoots),
            MuddyMangroveRoots => self.get(Mat::Soil, 1.).then_some(MuddyMangroveRoots),
            _ => Some(block),
        }
    }

    fn get_blockmaterial(&mut self, mat: BlockMaterial, amount: f32) -> Option<BlockMaterial> {
        match mat {
            Wood(_) => self.get_log(amount).map(Wood),
            Cobble
            | Stone
            | Granite
            | Diorite
            | Andesite
            | PolishedGranite
            | PolishedDiorite
            | PolishedAndesite
            | MossyCobble
            | StoneBrick
            | MossyStonebrick
            | Blackstone
            | PolishedBlackstone
            | PolishedBlackstoneBrick
            | Sandstone
            | SmoothSandstone
            | RedSandstone
            | SmoothRedSandstone => self.get(Mat::Stone, amount).then_some(mat),
            MudBrick => self.get(Mat::Soil, amount).then_some(mat),
            Brick => Some(Brick),
        }
    }

    fn get_log(&mut self, amount: f32) -> Option<TreeSpecies> {
        all::<TreeSpecies>().find(|&s| self.get(Mat::Wood(s), amount))
    }

    fn get(&mut self, mat: Mat, amount: f32) -> bool {
        assert!(amount > 0.);
        let stored = self.0.entry(mat).or_default();
        if *stored >= amount {
            *stored -= amount;
            true
        } else {
            false
        }
    }
}
