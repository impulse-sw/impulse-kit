#![feature(iterator_try_collect, let_chains)]
#![deny(warnings, clippy::todo, clippy::unimplemented)]

mod entities;

pub use entities::*;

use cc_utils::prelude::{MResult, ServerError};
use std::fs;
use std::path::{Path, PathBuf};

use crate::entities::file::File;

pub fn generate(version: Option<String>, api_desc: File, regenerate: bool, output_dir: &Path) -> MResult<()> {
  let version = version
    .map(|version| {
      println!("Selected API version: {}.", version);
      version
    })
    .unwrap_or_else(|| {
      let version = crate::entities::versions::decide_version(&output_dir, &api_desc, regenerate);
      if regenerate {
        println!("Regenerating existing API version: {}.", version);
      } else {
        println!("Decided API version: {}.", version);
      }
      version
    });

  let generated_code = api_desc.generate_from_scratch()?;

  let _ = fs::create_dir_all(PathBuf::from(&output_dir).join(&version));
  for file in generated_code {
    let filename = output_dir.join(&version).join(&file.filepath);
    if !fs::exists(&filename).is_ok_and(|v| v) {
      println!("Writing file {:?}...", file.filepath);
      fs::write(filename, file.content).map_err(ServerError::from_private)?;
    } else if regenerate {
      println!("Rewriting file {:?}...", file.filepath);
      fs::write(filename, file.content).map_err(ServerError::from_private)?;
    }
  }
  fs::write(
    output_dir.join(&version).join(".api.json"),
    sonic_rs::to_string_pretty(&api_desc).map_err(ServerError::from_private)?,
  )
  .map_err(ServerError::from_private)?;

  println!("Generated API code written. Version: {}.", version);

  Ok(())
}
