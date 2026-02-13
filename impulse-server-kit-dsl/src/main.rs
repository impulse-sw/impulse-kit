#![feature(iterator_try_collect)]
#![deny(warnings, clippy::todo, clippy::unimplemented)]

use clap::{Arg, Command};
use impulse_utils::prelude::*;
use std::path::PathBuf;

use impulse_skdsl::ClientTarget;
use impulse_skdsl::generate;

fn main() -> MResult<()> {
  let matches = Command::new("Server Kit DSL Translator")
    .disable_version_flag(true)
    .about("Translates DSL to Server Kit's API code")
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
    .arg(
      Arg::new("client")
        .short('c')
        .long("client")
        .value_name("TARGET")
        .help("Generate client code (rust, js). Can be specified multiple times.")
        .action(clap::ArgAction::Append),
    )
    .get_matches();

  let input_file = matches.get_one::<String>("input").unwrap();
  let output_dir = matches.get_one::<String>("output").unwrap();
  let regenerate = matches.get_flag("regenerate");

  let client_targets: Vec<ClientTarget> = matches
    .get_many::<String>("client")
    .map(|vals| {
      vals
        .map(|v| match v.as_str() {
          "rust" | "rs" => ClientTarget::Rust,
          "js" | "javascript" => ClientTarget::Js,
          other => panic!("Unknown client target: `{other}`. Use `rust` or `js`."),
        })
        .collect()
    })
    .unwrap_or_default();

  let api_desc = impulse_skdsl::file::parse_file(input_file)?;
  let output_dir = PathBuf::from(&output_dir);
  let version = matches.get_one::<String>("version").cloned();

  generate(version, api_desc, regenerate, &output_dir, &client_targets)?;

  Ok(())
}
