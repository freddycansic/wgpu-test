use color_eyre::Result;

pub fn load_binary(path: &str) -> Result<Vec<u8>> {
    let path = std::path::Path::new(path);

    Ok(std::fs::read(path)?)
}
