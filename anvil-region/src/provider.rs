use crate::position::RegionPosition;
use crate::region::Region;
use std::fs::{File, OpenOptions, read_dir};
use std::path::Path;
use std::{fs, io};
use std::str::FromStr;

pub trait RegionProvider<S> {
    fn get_region(&self, region_pos: RegionPosition) -> Result<Region<S>, io::Error>;
}

pub struct FolderRegionProvider<'a> {
    /// Folder where region files located.
    folder_path: &'a Path,
}

impl<'a> FolderRegionProvider<'a> {
    pub fn new(folder: &'a str) -> FolderRegionProvider<'a> {
        let folder_path = Path::new(folder);

        FolderRegionProvider { folder_path }
    }

    // leave implementing this to the specific provider,
    // makes function declaration bearable for now
    pub fn iter_positions(&self) -> Result<impl Iterator<Item=RegionPosition>, io::Error> {
        let positions: Vec<_> = read_dir(self.folder_path)?
            .filter_map(|dir| dir.ok())
            .filter_map(|dir| region_pos_from_filename(&dir.path()).ok())
            .collect();

        Ok(positions.into_iter())
    }
}

impl<'a> RegionProvider<File> for FolderRegionProvider<'a> {
    fn get_region(&self, position: RegionPosition) -> Result<Region<File>, io::Error> {
        if !self.folder_path.exists() {
            fs::create_dir(self.folder_path)?;
        }

        let region_name = region_position_filename(position);
        let region_path = self.folder_path.join(region_name);

        let file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(region_path)?;

        Region::load(position, file)
    }
}

fn region_pos_from_filename(path: &Path) -> Result<RegionPosition, io::Error> {
    // we can use lossy because of the bound check later
    let filename = path.file_name().unwrap_or_default().to_string_lossy();
    let parts: Vec<_> = filename.split('.').collect();

    let (x, z) = parse_coords(parts).ok_or_else(|| io::ErrorKind::InvalidInput)?;

    Ok(RegionPosition::new(x, z))
}

fn region_position_filename(pos: RegionPosition) -> String {
    format!("r.{}.{}.mca", pos.x, pos.z)
}

fn parse_coords(parts: Vec<&str>) -> Option<(i32, i32)> {
    let incorrect_format =
        parts.len() != 4 ||
            parts[0] != "r" ||
            parts[3] != "mca";

    if incorrect_format {
        return None;
    }

    Some((i32::from_str(parts[1]).ok()?,
          i32::from_str(parts[2]).ok()?))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::position::RegionPosition;
    use crate::provider::region_pos_from_filename;

    #[test]
    fn test_position_parse() {
        let mut path = PathBuf::new();
        path.set_file_name("r.-1.1.mca");

        let pos = region_pos_from_filename(&path).unwrap();
        assert_eq!(RegionPosition{ x: -1, z: 1}, pos)
    }

    #[test]
    #[should_panic]
    fn test_position_parse_invalid_format() {
        let mut path = PathBuf::new();
        path.set_file_name("this is not a valid region.filename");

        region_pos_from_filename(&path).unwrap();
    }
}