use impulse_utils::prelude::{MResult, ServerError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::entities::api::{Api, TaggedApi};
use crate::entities::file::File;
use crate::entities::incoming::{Incoming, parse_incoming_data_except_path};
use crate::entities::outgoing::{Outgoing, parse_outgoing_data_except_body};

#[derive(Deserialize, Serialize)]
pub struct Requirement {
  pub name: String,
  pub hidden: bool,
  pub incoming: Vec<Incoming>,
  pub outcoming: Vec<Outgoing>,
}

pub fn parse_requirement(requirement_line: &str) -> MResult<Requirement> {
  assert!(requirement_line.starts_with("req"));

  let parts = requirement_line.split_whitespace().collect::<Vec<_>>();
  if parts.len() < 2 {
    return ServerError::from_public(format!("Requirement doesn't contain a name: `{requirement_line}`")).bail();
  }

  let hidden = match parts[0] {
    "req" => false,
    "req/hidden" => true,
    _ => return ServerError::from_public(format!("Invalid requirement: `{}`", parts[0])).bail(),
  };

  let inout = parts[2..].split(|v| v.eq(&"->")).collect::<Vec<_>>();
  let incoming = inout[0];
  let outcoming = if inout.len() > 1 { inout[1] } else { &[] };

  let mut incoming = incoming
    .iter()
    .map(|i| parse_incoming_data_except_path(i))
    .try_collect::<Vec<_>>()?;
  let mut outcoming = outcoming
    .iter()
    .map(|o| parse_outgoing_data_except_body(o))
    .try_collect::<Vec<_>>()?;

  for indata in incoming.iter_mut() {
    match indata {
      Incoming::Header(header) => header.hidden = true,
      Incoming::QueryParam(query) => query.hidden = true,
      Incoming::CookieParam(cookie) => cookie.hidden = true,
      _ => {}
    }
  }

  for outdata in outcoming.iter_mut() {
    match outdata {
      Outgoing::Header(header) => header.hidden = true,
      Outgoing::Cookie(cookie) => cookie.hidden = true,
      _ => {}
    }
  }

  if incoming.iter().filter(|i| i.is_body()).count() > 1 {
    return ServerError::from_public("You provide several body requirements for single API!").bail();
  }
  if incoming.iter().any(|i| i.is_body()) && incoming.iter().any(|i| i.is_form()) {
    return ServerError::from_public("You provide both body and form requirements for single API!").bail();
  }

  Ok(Requirement {
    name: parts[1].to_string(),
    hidden,
    incoming,
    outcoming,
  })
}

pub fn parse_requirement_usage(requirement_str: &str) -> MResult<String> {
  let parts = requirement_str.split('/').collect::<Vec<_>>();
  if parts.len() != 2 {
    return ServerError::from_public(format!("Invalid requirement usage: `{requirement_str}`")).bail();
  }
  Ok(parts[1].to_string())
}

pub fn unite_requirements(
  endpoint: &Api,
  tag: &TaggedApi,
  api_desc: &File,
) -> MResult<(BTreeSet<Incoming>, BTreeSet<Outgoing>)> {
  let mut incoming = BTreeSet::new();
  let mut outcoming = BTreeSet::new();

  endpoint.incoming.iter().for_each(|i| {
    incoming.insert(i.clone());
  });
  endpoint.outcoming.iter().for_each(|o| {
    outcoming.insert(o.clone());
  });

  endpoint.requirements.iter().for_each(|rn| {
    if let Some(requirement) = api_desc.requirements.iter().find(|r| r.name.as_str().eq(rn.as_str())) {
      requirement.incoming.iter().for_each(|i| {
        incoming.insert(i.clone());
      });
      requirement.outcoming.iter().for_each(|o| {
        outcoming.insert(o.clone());
      });
    }
  });

  tag.tag_requirements.iter().for_each(|rn| {
    if let Some(requirement) = api_desc.requirements.iter().find(|r| r.name.as_str().eq(rn.as_str())) {
      requirement.incoming.iter().for_each(|i| {
        incoming.insert(i.clone());
      });
      requirement.outcoming.iter().for_each(|o| {
        outcoming.insert(o.clone());
      });
    }
  });

  Ok((incoming, outcoming))
}
