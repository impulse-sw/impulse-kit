use impulse_utils::prelude::{MResult, ServerError};
use std::collections::BTreeSet;

use crate::entities::api::{generate_api_endpoint, generate_router};
use crate::entities::file::File;
use crate::entities::types::{convert_typedefs_update, select_typedef};

pub struct EvolutionContext {
  pub new_types: String,
  pub new_api_endpoints: String,
  pub old_router: String,
  pub new_router: String,
}

pub fn evolve(old_desc: &File, new_desc: &File, api_tag: &str) -> MResult<Option<EvolutionContext>> {
  let mut new_api_endpoints = vec![];
  let old_desc_tag = old_desc
    .tags
    .iter()
    .find(|api| api.tag.as_str().eq(api_tag))
    .ok_or(ServerError::from_public(
      "Can't find current tag in previous iteration!",
    ))?;
  let new_desc_tag = new_desc
    .tags
    .iter()
    .find(|api| api.tag.as_str().eq(api_tag))
    .ok_or(ServerError::from_public("Can't find current tag in new iteration!"))?;
  for new_endp in &new_desc_tag.endpoints {
    if !old_desc_tag.endpoints.iter().any(|endp| endp == new_endp) {
      new_api_endpoints.push(new_endp.to_owned());
    }
  }

  let new_api_endpoints = new_api_endpoints
    .iter()
    .map(|endp| generate_api_endpoint(endp, new_desc_tag, new_desc))
    .try_collect::<Vec<_>>()?;

  let mut unique_types = BTreeSet::new();
  for endp in &new_api_endpoints {
    endp.0.iter().for_each(|t| {
      unique_types.insert(t.to_owned());
    });
  }
  let mut typedefs = vec![];
  for utype in unique_types.iter().flat_map(|ut| select_typedef(ut, &new_desc.types)) {
    typedefs.push(utype);
  }
  let new_types = typedefs
    .into_iter()
    .filter(|td| !old_desc.types.contains(*td))
    .collect::<Vec<_>>();
  let new_types = convert_typedefs_update(&new_types);

  let new_api_endpoints = new_api_endpoints
    .into_iter()
    .map(|(_, endp)| endp.join("\n"))
    .collect::<Vec<_>>();

  if new_types.is_empty() && new_api_endpoints.is_empty() {
    println!("There is no new types or endpoints in tag `{api_tag}`");
    return Ok(None);
  }

  let new_types = format!("\n\n{}", new_types.join("\n"));
  let new_api_endpoints = format!("\n{}", new_api_endpoints.join("\n"));
  let old_router = generate_router(old_desc_tag)?;
  let new_router = generate_router(new_desc_tag)?;

  Ok(Some(EvolutionContext {
    new_types,
    new_api_endpoints,
    old_router,
    new_router,
  }))
}
