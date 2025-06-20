use std::fs;
use std::path::Path;

use crate::entities::file::File;

pub fn decide_version(output_dir: &Path, api_desc: &File, regenerate: bool) -> String {
  let mut cntr = 1u16;

  while let Ok(saved_api_desc) = fs::read_to_string(output_dir.join(format!("v{cntr}")))
    && let Ok(saved_api_desc) = sonic_rs::from_str::<File>(&saved_api_desc)
    && saved_api_desc.no_breaking_changes(api_desc).is_err()
  {
    cntr += 1;
  }

  if regenerate && cntr != 1 {
    cntr -= 1;
  }

  format!("v{cntr}")
}
