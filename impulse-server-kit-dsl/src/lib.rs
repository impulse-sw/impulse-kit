#![feature(iterator_try_collect, let_chains)]
#![deny(warnings, clippy::todo, clippy::unimplemented)]

mod entities;

pub use entities::*;

use impulse_utils::prelude::{MResult, ServerError};
use smart_patcher::{AreaRule, FilePath, Patch, PatchFile, RegexIR, Replacer};
use std::fs;
use std::path::{Path, PathBuf};

use crate::entities::evolution::evolve;
use crate::entities::file::File;

pub fn generate(version: Option<String>, api_desc: File, regenerate: bool, output_dir: &Path) -> MResult<()> {
  let version = version
    .map(|version| {
      println!("Selected API version: {version}.");
      version
    })
    .unwrap_or_else(|| {
      let version = crate::entities::versions::decide_version(output_dir, &api_desc, regenerate);
      if regenerate {
        println!("Regenerating existing API version: {version}.");
      } else {
        println!("Decided API version: {version}.");
      }
      version
    });

  let generated_code = api_desc.generate_from_scratch()?;
  let old_desc = fs::read_to_string(output_dir.join(&version).join(".api.json"))
    .map_err(ServerError::from_private)
    .and_then(|s| sonic_rs::from_str::<File>(&s).map_err(ServerError::from_private))
    .ok();

  let _ = fs::create_dir_all(PathBuf::from(&output_dir).join(&version));
  for file in generated_code {
    let filename = output_dir.join(&version).join(&file.filepath);
    if !fs::exists(&filename).is_ok_and(|v| v) {
      println!("Writing file {:?}...", file.filepath);
      fs::write(filename, file.content).map_err(ServerError::from_private)?;
    } else if regenerate {
      println!("Rewriting file {:?}...", file.filepath);
      fs::write(&filename, file.content.as_str()).map_err(ServerError::from_private)?;
    } else if fs::read_to_string(&filename)
      .map_err(ServerError::from_private)?
      .as_str()
      .ne(file.content.as_str())
      && let Some(tag) = file.api_tag.as_deref()
      && let Some(old_desc) = old_desc.as_ref()
    {
      let ctx = evolve(old_desc, &api_desc, tag)?;
      if ctx.is_none() {
        continue;
      }
      let ctx = ctx.unwrap();
      println!("Trying to update file {:?}...", file.filepath);

      let patchfile = PatchFile {
        patches: vec![
          Patch {
            files: vec![FilePath::Just(filename.to_owned())],
            patch_area: vec![AreaRule::After(
              RegexIR::new("use cc_server_kit::prelude::\\*;").unwrap(),
            )],
            insert: Some(ctx.new_types),
            ..Default::default()
          },
          Patch {
            files: vec![FilePath::Just(filename.to_owned())],
            patch_area: vec![],
            replace: Some(Replacer::FromTo(ctx.old_router, ctx.new_router)),
            ..Default::default()
          },
          Patch {
            files: vec![FilePath::Just(filename.to_owned())],
            patch_area: vec![AreaRule::CursorAtEnd],
            insert: Some(ctx.new_api_endpoints),
            ..Default::default()
          },
        ],
      };
      let patched = patchfile
        .patch(&PathBuf::from("."), &PathBuf::from("."))
        .map_err(|e| ServerError::from_public(e.to_string()))?;
      if patched == 0 {
        ServerError::from_public("Can't patch!").bail()?;
      }
    }
  }
  fs::write(
    output_dir.join(&version).join(".api.json"),
    sonic_rs::to_string_pretty(&api_desc).map_err(ServerError::from_private)?,
  )
  .map_err(ServerError::from_private)?;

  println!("Generated API code written. Version: {version}.");

  Ok(())
}
