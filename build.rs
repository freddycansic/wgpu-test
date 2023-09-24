use color_eyre::Result;
use std::env;

fn main() -> Result<()> {
    const ASSETS_PATH: &str = "assets/";

    println!("cargo:rerun-if-changed={}*", ASSETS_PATH);

    let out_dir = env::var("OUT_DIR")?;
    let mut copy_options = fs_extra::dir::CopyOptions::new();
    copy_options.overwrite = true;
    let mut paths_to_copy = Vec::new();
    paths_to_copy.push(ASSETS_PATH);
    fs_extra::copy_items(&paths_to_copy, out_dir, &copy_options)?;

    Ok(())
}
