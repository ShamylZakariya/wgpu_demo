use anyhow::*;
use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;
use std::env;

fn main() -> Result<()> {
    // tells cargo to run only on changes to res/
    println!("cargo:rerun-if-changed=res/*");

    let out_dir = env::var("OUT_DIR")?;

    let mut copy_options = CopyOptions::new();
    copy_options.overwrite = true;

    let paths_to_copy = vec!["res/"];

    copy_items(&paths_to_copy, out_dir, &copy_options)?;

    Ok(())
}
