#![feature(iterator_try_collect, let_chains)]
#![deny(warnings, clippy::todo, clippy::unimplemented)]

use cc_utils::prelude::*;
use clap::{Arg, Command};
use std::path::PathBuf;

use skdsl::generate;

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
        .help("Don't bump the version and rewrite all generated files (destructive)")
        .action(clap::ArgAction::SetTrue),
    )
    .get_matches();

  let input_file = matches.get_one::<String>("input").unwrap();
  let output_dir = matches.get_one::<String>("output").unwrap();
  let regenerate = matches.get_flag("regenerate");

  let api_desc = skdsl::file::parse_file(input_file)?;
  let output_dir = PathBuf::from(&output_dir);
  let version = matches.get_one::<String>("version").cloned();

  generate(version, api_desc, regenerate, &output_dir)?;

  Ok(())
}
