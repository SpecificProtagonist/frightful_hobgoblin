use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use Block::*;

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
pub enum Block {
    Air,
    Stone,
    Log(TreeSpecies, LogType),
    Other {
        id: u8,
        data: u8
    }
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
            1 => Stone,
            17 => Log(
                TreeSpecies::from_u8(data % 4).unwrap(), 
                match data >> 4 {
                    3 => LogType::FullBark,
                    dir => LogType::Normal(Axis::from_u8(dir).unwrap())
                }
            ),
            _ => Other {
                id,
                data
            }
        }
    }

    pub fn to_bytes(self) -> (u8, u8) {
        match self {
            Air => (0, 0),
            Stone => (1, 0),
            Log(species, log_type) => {
                (if (species as u8) < 4 {17} else {162},
                (match log_type {
                    LogType::Normal(dir) => dir as u8,
                    LogType::FullBark => 3
                } << 2) + (species as u8) % 4
                )
            },
            Other {id, data} => (id, data),
        }
    }
}