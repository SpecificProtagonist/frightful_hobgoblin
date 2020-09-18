use Biome::*;

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Biome {

    Other(u8)
}

impl Biome {
    pub fn from_bytes(id: u8) -> Self {
        Other(id)
    }

    pub fn to_bytes(self) -> u8 {
        match self {
            Other(id) => id
        }
    }
}