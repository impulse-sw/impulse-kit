use impulse_utils::prelude::{MResult, ServerError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::entities::api::{TaggedApi, generate_api_tag, parse_api_endpoint, parse_api_tag};
use crate::entities::requirements::{Requirement, parse_requirement, unite_requirements};
use crate::entities::types::{Type, TypeUsage, parse_typedef};

#[derive(Deserialize, Serialize)]
pub struct File {
  pub types: Vec<Type>,
  pub requirements: Vec<Requirement>,
  pub tags: Vec<TaggedApi>,
}

impl File {
  pub fn no_breaking_changes(&self, new_version: &Self) -> MResult<()> {
    for old_tag in &self.tags {
      let new_tag = new_version
        .tags
        .iter()
        .find(|t| t.tag.as_str().eq(old_tag.tag.as_str()))
        .ok_or(ServerError::from_public("There is no such tag!"))?;

      for old_endp_desc in &old_tag.endpoints {
        let new_endp_desc = new_tag
          .endpoints
          .iter()
          .find(|e| e.name.as_str().eq(old_endp_desc.name.as_str()))
          .ok_or(ServerError::from_public("There is no such endpoint!"))?;

        let (old_incoming, old_outgoing) = unite_requirements(old_endp_desc, old_tag, self)?;
        let (new_incoming, new_outgoing) = unite_requirements(new_endp_desc, new_tag, new_version)?;

        for indata in new_incoming.iter() {
          if !old_incoming.contains(indata) {
            ServerError::from_public("New requirement provided! Breaking change!").bail()?;
          }
        }

        for outdata in old_outgoing.iter() {
          if !new_outgoing.contains(outdata) {
            ServerError::from_public("There is no such requirement! Breaking change!").bail()?;
          }
        }
      }
    }

    Ok(())
  }
}

impl File {
  pub fn generate_from_scratch(&self) -> MResult<Vec<Generated>> {
    let mut generated = vec![];

    for tag in &self.tags {
      generated.push(Generated {
        api_tag: Some(tag.tag.clone()),
        filepath: PathBuf::from(format!("{}.rs", tag.tag)),
        content: generate_api_tag(tag, self)?,
      });
    }

    generated.push(Generated {
      api_tag: None,
      filepath: PathBuf::from("mod.rs"),
      content: self
        .tags
        .iter()
        .map(|t| format!("pub mod {};\n", t.tag))
        .collect::<Vec<_>>()
        .join(""),
    });

    Ok(generated)
  }

  pub fn generate_client_rs(&self) -> MResult<Vec<Generated>> {
    let mut generated = vec![];

    for tag in &self.tags {
      generated.push(Generated {
        api_tag: Some(tag.tag.clone()),
        filepath: PathBuf::from(format!("{}.rs", tag.tag)),
        content: crate::entities::client_rs::generate_client_rs_tag(tag, self)?,
      });
    }

    generated.push(Generated {
      api_tag: None,
      filepath: PathBuf::from("mod.rs"),
      content: self
        .tags
        .iter()
        .map(|t| format!("pub mod {};\n", t.tag))
        .collect::<Vec<_>>()
        .join(""),
    });

    Ok(generated)
  }

  pub fn generate_client_js(&self) -> MResult<Vec<Generated>> {
    let mut generated = vec![];

    for tag in &self.tags {
      generated.push(Generated {
        api_tag: Some(tag.tag.clone()),
        filepath: PathBuf::from(format!("{}.js", tag.tag)),
        content: crate::entities::client_js::generate_client_js_tag(tag, self)?,
      });
    }

    generated.push(Generated {
      api_tag: None,
      filepath: PathBuf::from("index.js"),
      content: self
        .tags
        .iter()
        .map(|t| format!("export * from './{}.js';\n", t.tag))
        .collect::<Vec<_>>()
        .join(""),
    });

    Ok(generated)
  }
}

#[derive(Deserialize, Serialize)]
pub struct Generated {
  pub api_tag: Option<String>,
  pub filepath: PathBuf,
  pub content: String,
}

pub fn parse_file(filepath: impl AsRef<Path>) -> MResult<File> {
  let f = fs::read_to_string(filepath).map_err(|e| ServerError::from_private(e).with_public("Can't read file!"))?;
  let lines = f.split('\n').filter(|l| !l.is_empty()).collect::<Vec<_>>();

  let mut types = vec![Type::Usage(TypeUsage {
    name: "HashMap".to_string(),
    rust_type: "std::collections::HashMap".to_string(),
  })];
  let mut requirements = vec![];

  let mut current_tag = None;
  let mut current_tag_requirements = None;
  let mut current_tag_apis = None;

  let mut tags = vec![];

  for line in lines {
    if line.starts_with("type") {
      types.push(parse_typedef(line)?);
    } else if line.starts_with("req") {
      requirements.push(parse_requirement(line)?);
    } else if line.starts_with("api tag") {
      let (tag, reqs) = parse_api_tag(line)?;

      if let Some(current_tag) = current_tag.take()
        && let Some(current_tag_requirements) = current_tag_requirements.take()
        && let Some(current_tag_apis) = current_tag_apis.take()
      {
        tags.push(TaggedApi {
          tag: current_tag,
          tag_requirements: current_tag_requirements,
          endpoints: current_tag_apis,
        });
      }

      current_tag = Some(tag);
      current_tag_requirements = Some(reqs);
      current_tag_apis = Some(vec![]);
    } else if line.starts_with("api")
      && let Some(apis) = current_tag_apis.as_mut()
    {
      apis.push(parse_api_endpoint(line)?);
    }
  }

  if let Some(current_tag) = current_tag.take()
    && let Some(current_tag_requirements) = current_tag_requirements.take()
    && let Some(current_tag_apis) = current_tag_apis.take()
  {
    tags.push(TaggedApi {
      tag: current_tag,
      tag_requirements: current_tag_requirements,
      endpoints: current_tag_apis,
    });
  }

  Ok(File {
    types,
    requirements,
    tags,
  })
}
