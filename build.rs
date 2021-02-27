include!("config_local.rs");

// Copy the templates into a local folder
pub fn main() -> Result<(), std::io::Error> {
    for file in std::fs::read_dir(TEMPLATE_PATH)? {
        let file = file?;
        println!("cargo:rerun-if-changed={}", file.path().display());
        let dest = std::path::Path::new("templates").join(file.file_name());
        std::fs::copy(file.path(), dest)?;
    }

    Ok(())
}
