use std::path::Path;

include!("config_local.rs");

// Copy the templates into a local folder
pub fn main() -> Result<(), std::io::Error> {
    for file in std::fs::read_dir(TEMPLATE_PATH)? {
        let file = file?;
        if file.file_type()?.is_dir() {
            let namespace = file.file_name();
            let dir = Path::join(&file.path(), "structures");

            std::fs::create_dir_all(format!("template/{}", namespace.to_string_lossy()))?;

            for file in std::fs::read_dir(dir)? {
                let file = file?;
                println!("cargo:rerun-if-changed={}", file.path().display());

                std::fs::copy(
                    file.path(),
                    format!(
                        "templates/{}/{}",
                        namespace.to_string_lossy(),
                        file.file_name().to_string_lossy()
                    ),
                )?;
            }
        }
    }

    Ok(())
}
