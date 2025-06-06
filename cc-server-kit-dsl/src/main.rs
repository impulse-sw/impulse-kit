#![feature(iterator_try_collect, let_chains)]
#![deny(warnings, clippy::todo, clippy::unimplemented)]

use cc_utils::prelude::*;
use clap::{Arg, Command};
use std::{fs, path::PathBuf};

mod entities;

fn main() -> MResult<()> {
  let matches = Command::new("Server Kit DSL Translator")
    .disable_version_flag(true)
    .about("Translates DSL to CC Server Kit's API code")
    .arg(
      Arg::new("input")
        .short('i')
        .long("input")
        .value_name("FILE")
        .help("Input DSL file")
        .required(true),
    )
    .arg(
      Arg::new("output")
        .short('o')
        .long("output")
        .value_name("FOLDER")
        .help("Output folder")
        .required(true),
    )
    .arg(
      Arg::new("version")
        .short('v')
        .long("version")
        .value_name("VERSION")
        .help("API version (optional)"),
    )
    .arg(
      Arg::new("regenerate")
        .short('r')
        .long("regenerate")
        .help("Don't bump the version and rewrite all generated files")
        .action(clap::ArgAction::SetTrue),
    )
    .get_matches();

  let input_file = matches.get_one::<String>("input").unwrap();
  let output_dir = matches.get_one::<String>("output").unwrap();
  let regenerate = matches.get_flag("regenerate");

  let api_desc = crate::entities::file::parse_file(input_file)?;
  let output_dir = PathBuf::from(&output_dir);

  let version = matches
    .get_one::<String>("version")
    .cloned()
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
