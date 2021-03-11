use nbt::CompoundTag;

use crate::*;

#[derive(Clone)]
pub struct Village {
    pub center: ChunkIndex,
    pub buildings: Vec<(Rect, VillageBuildingType)>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum VillageBuildingType {
    House,
    Center,
    Farm,
    Street,
}

impl Village {
    // TODO: read old village.dat

    pub fn from_nbt(nbt: &CompoundTag) -> Self {
        let center = ChunkIndex(nbt.get("ChunkX").unwrap(), nbt.get("ChunkZ").unwrap());

        let mut buildings = Vec::new();

        for child in nbt.get_compound_tag_vec("Children").unwrap() {
            let bounds = child.get_i32_vec("BB").unwrap();
            let area = Rect {
                min: Column(bounds[0], bounds[2]),
                max: Column(bounds[3], bounds[5]),
            };

            let id = child.get_str("id").unwrap();

            // Check if 1.16
            if id == "minecraft:jigsaw" {
                // not every feature has a structure, some are simple generators
                let name = if let Ok(element) = child.get_compound_tag("pool_element") {
                    if let Ok(name) = element.get_str("location") {
                        name
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };

                if name.contains("farm") | name.contains("pen") {
                    buildings.push((area, VillageBuildingType::Farm));
                } else if name.contains("houses") {
                    buildings.push((area, VillageBuildingType::House));
                } else if name.contains("town_centers") {
                    buildings.push((area, VillageBuildingType::Center));
                } else if name.contains("streets") {
                    buildings.push((area, VillageBuildingType::Street));
                }
            } else {
                // 1.12
                if id.ends_with("H") | id.ends_with("S") | id.ends_with("T") {
                    buildings.push((area, VillageBuildingType::House));
                } else if id.ends_with("F") {
                    buildings.push((area, VillageBuildingType::Farm));
                } else if id.ends_with("Start") {
                    buildings.push((area, VillageBuildingType::Center));
                } else if id.ends_with("R") {
                    buildings.push((area, VillageBuildingType::Street));
                }
            }
        }

        Village { center, buildings }
    }
}
