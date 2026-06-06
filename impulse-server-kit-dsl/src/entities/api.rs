use impulse_utils::prelude::{MResult, ServerError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::entities::file::File;
use crate::entities::incoming::{Incoming, parse_incoming_data};
use crate::entities::outgoing::{Outgoing, parse_outgoing_data};
use crate::entities::requirements::{parse_requirement_usage, unite_requirements};
use crate::entities::types::{convert_typedefs, select_typedef, typename_import};

#[derive(Deserialize, Serialize)]
pub struct TaggedApi {
  pub tag: String,
  pub tag_requirements: Vec<String>,
  pub endpoints: Vec<Api>,
}

#[derive(Deserialize, Serialize, PartialEq)]
pub struct Api {
  pub name: String,
  pub hidden: bool,
  pub used_types: Vec<String>,
  pub requirements: Vec<String>,
  pub incoming: Vec<Incoming>,
  pub outcoming: Vec<Outgoing>,
}

pub fn parse_api_endpoint(api_line: &str) -> MResult<Api> {
  let parts = api_line.split_whitespace().collect::<Vec<_>>();
  if parts.len() < 4 {
    return ServerError::from_public(format!("Invalid API description - insufficient data: `{api_line}`")).bail();
  }

  let hidden = match parts[0] {
    "api" => false,
    "api/hidden" => true,
    _ => ServerError::from_public(format!("Unknown API format: `{}`", parts[0])).bail()?,
  };

  let mut requirements = vec![];
  let mut i = 1;
  while parts[i].starts_with("req") && i < parts.len() {
    requirements.push(parse_requirement_usage(parts[i])?);
    i += 1;
  }

  if i + 1 >= parts.len() {
    ServerError::from_public("Provided no returning type!").bail()?;
  }

  let inout = parts[i..].split(|v| v.eq(&"->")).collect::<Vec<_>>();
  let incoming = inout[0];
  let outcoming = if inout.len() > 1 { inout[1] } else { &[] };

  let incoming = incoming
    .iter()
    .map(|i| parse_incoming_data(i))
    .try_collect::<Vec<_>>()?;
  let outcoming = outcoming
    .iter()
    .map(|o| parse_outgoing_data(o))
    .try_collect::<Vec<_>>()?;

  let name = if let Some(Incoming::Path(path)) = incoming.iter().find(|v| matches!(v, Incoming::Path(_))) {
    format!(
      "{}{}",
      path.request_type,
      path
        .path
        .replace(['/', '-'], "_")
        .replace("{**", "by_")
        .replace('{', "by_")
        .replace('}', "")
    )
  } else {
    return ServerError::from_public("No path provided!").bail();
  };

  if incoming.iter().filter(|i| i.is_path()).count() > 1 {
    ServerError::from_public("You provide several paths for single API!").bail()?;
  }
  if incoming.iter().filter(|i| i.is_body()).count() > 1 {
    ServerError::from_public("You provide several body requirements for single API!").bail()?;
  }
  if incoming.iter().any(|i| i.is_body()) && incoming.iter().any(|i| i.is_form()) {
    ServerError::from_public("You provide both body and form requirements for single API!").bail()?;
  }
  if outcoming.iter().filter(|o| o.is_body()).count() == 0 {
    ServerError::from_public("Provide any response body for single API!").bail()?;
  }
  if outcoming.iter().filter(|o| o.is_body()).count() > 1 {
    ServerError::from_public("You provide several body responses for single API!").bail()?;
  }

  let mut used_types = BTreeSet::<String>::new();

  for i in &incoming {
    for used_type in i.get_used_types() {
      used_types.insert(used_type);
    }
  }

  for o in &outcoming {
    for used_type in o.get_used_types() {
      used_types.insert(used_type);
    }
  }

  Ok(Api {
    used_types: used_types.into_iter().collect(),
    name,
    hidden,
    requirements,
    incoming,
    outcoming,
  })
}

pub fn parse_api_tag(api_tag_line: &str) -> MResult<(String, Vec<String>)> {
  let parts = api_tag_line.split_whitespace().collect::<Vec<_>>();
  if parts.len() < 3 {
    return ServerError::from_public(format!("Invalid API tag definition: `{api_tag_line}`")).bail();
  }

  let mut requirements = vec![];
  let mut i = 3;
  while i < parts.len() && parts[i].starts_with("req") {
    requirements.push(parse_requirement_usage(parts[i])?);
    i += 1;
  }

  if parts[2].eq("mod") {
    ServerError::from_public("Can't use `mod` for API tag! `mod` is the Rust keyword and used internally.").bail()?;
  }

  Ok((parts[2].to_string(), requirements))
}

pub fn generate_api_tag(tag: &TaggedApi, api_desc: &File) -> MResult<String> {
  use std::collections::BTreeSet;

  let mut lines = vec![];

  lines.push("use impulse_server_kit::prelude::*;".to_string());

  let endp_defs = tag
    .endpoints
    .iter()
    .map(|e| generate_api_endpoint(e, tag, api_desc))
    .try_collect::<Vec<_>>()?;

  let mut unique_types = BTreeSet::new();
  for endp in &endp_defs {
    endp.0.iter().for_each(|t| {
      unique_types.insert(t.to_owned());
    });
  }
  let mut typedefs = vec![];
  for utype in unique_types.iter().flat_map(|ut| select_typedef(ut, &api_desc.types)) {
    typedefs.push(utype);
  }

  let typedefs = convert_typedefs(&typedefs);
  if !typedefs.is_empty() {
    lines.push(typedefs);
  }

  if !endp_defs.is_empty() {
    lines.push(generate_router(tag)?);
  }

  for endp in &endp_defs {
    lines.extend_from_slice(&endp.1);
  }

  Ok(lines.join("\n"))
}

pub fn generate_router(tag: &TaggedApi) -> MResult<String> {
  let mut lines = vec![
    format!("pub fn {}_router() -> Router {{", tag.tag),
    "  Router::new()".to_string(),
  ];

  for endp in &tag.endpoints {
    let path = if let Some(Incoming::Path(path)) = endp.incoming.iter().find(|i| i.is_path()) {
      path
    } else {
      return ServerError::from_public(format!("No path for `{}` API endpoint found!", endp.name)).bail();
    };

    lines.push(format!(
      "    .push(Router::with_path(\"{}\").{}({}))",
      path.path, path.request_type, endp.name,
    ));
  }

  lines.push(String::from("}\n"));

  Ok(lines.join("\n"))
}

pub fn generate_api_endpoint(endpoint: &Api, tag: &TaggedApi, api_desc: &File) -> MResult<(Vec<String>, Vec<String>)> {
  let mut lines = vec![];
  let (incoming, outgoing) = unite_requirements(endpoint, tag, api_desc)?;

  let mut used_types = incoming
    .iter()
    .flat_map(|i| i.get_used_types())
    .map(|t| typename_import(t.as_str()))
    .try_collect::<Vec<_>>()?
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
  used_types.extend_from_slice(
    &outgoing
      .iter()
      .flat_map(|o| o.get_used_types())
      .map(|t| typename_import(t.as_str()))
      .try_collect::<Vec<_>>()?
      .into_iter()
      .flatten()
      .collect::<Vec<_>>(),
  );

  lines.push(format!("/// {}", tag_name_normalize(&endpoint.name)));

  lines.push(
    "#[instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]".to_string(),
  );

  if endpoint.hidden {
    lines.push("#[handler]".to_string());
  } else {
    let mut endp_macro = String::new();
    endp_macro.push_str("#[endpoint(\n  tags(\"");
    endp_macro.push_str(tag_name_normalize(&tag.tag).as_str());
    endp_macro.push_str("\")");

    if let Some(Incoming::Body(body)) = incoming.iter().find(|i| i.is_body()) {
      endp_macro.push_str(",\n  request_body(content = ");
      match body {
        crate::entities::incoming::InBody::File { .. } => {
          endp_macro.push_str("Vec<u8>, content_type = \"application/octet-stream\", description = \"\")")
        }
        crate::entities::incoming::InBody::Json { rust_type } => {
          endp_macro.push_str(rust_type);
          endp_macro.push_str(", content_type = \"application/json\", description = \"\")");
        }
        crate::entities::incoming::InBody::MsgPack { rust_type } => {
          endp_macro.push_str(rust_type);
          endp_macro.push_str(", content_type = \"application/msgpack\", description = \"\")");
        }
      }
    } else if incoming.iter().any(|i| i.is_form()) {
      endp_macro
        .push_str(",\n  request_body(content = Vec<u8>, content_type = \"multipart/form-data\", description = \"\")");
    }

    if incoming.iter().any(|i| i.is_header() || i.is_query() || i.is_path()) {
      endp_macro.push_str(&generate_endp_macro_params(&incoming.iter().collect::<Vec<_>>()));
    }

    endp_macro.push_str(&generate_endp_macro_resp(&outgoing.iter().collect::<Vec<_>>()));

    endp_macro.push_str("\n)]");
    lines.push(endp_macro);
  }

  let req_use = incoming.iter().any(|i| i.require_salvo_req());
  let res_use = outgoing.iter().any(|o| o.require_salvo_res());
  lines.push(format!(
    "pub async fn {}({}{}{}) -> {} {{",
    endpoint.name,
    if req_use { "req: &mut Request" } else { "" },
    if req_use && res_use { ", " } else { "" },
    if res_use { "res: &mut Response" } else { "" },
    if let Some(Outgoing::Body(body)) = outgoing.iter().find(|o| o.is_body()) {
      body.return_type_str()
    } else {
      String::new()
    },
  ));

  if outgoing.iter().any(|o| o.is_cookie()) {
    lines.push(String::from(
      "  use impulse_server_kit::salvo::http::cookie::CookieBuilder;\n",
    ));
  }

  let scopts = stringcase::Options {
    separate_before_non_alphabets: false,
    separate_after_non_alphabets: false,
    ..Default::default()
  };

  let mut was_indata = false;
  for indata in incoming.iter() {
    match indata {
      Incoming::Header(header) if !header.hidden => {
        lines.push(format!(
          "  let {} = req\n    .header::<{}>(\"{}\")\n    .ok_or(ServerError::from_public(\"Can't find `{}` header!\").with_400())?;",
          stringcase::snake_case_with_options(header.name.as_str(), &scopts),
          header.r#type,
          header.name,
          header.name,
        ));
        was_indata = true;
      }
      Incoming::Path(path) => {
        for path_param in &path.params {
          lines.push(format!(
          "  let {} = req\n    .param::<{}>(\"{}\")\n    .ok_or(ServerError::from_public(\"Can't find `{}` parameter!\").with_400())?;",
          stringcase::snake_case_with_options(path_param.key.as_str(), &scopts),
          path_param.r#type,
          path_param.key,
          path_param.key,
        ));
          was_indata = true;
        }
      }
      Incoming::QueryParam(query) if !query.hidden => {
        lines.push(format!(
          "  let {} = req\n    .query::<{}>(\"{}\")\n    .ok_or(ServerError::from_public(\"Can't find `{}` query parameter!\").with_400())?;",
          stringcase::snake_case_with_options(query.query.as_str(), &scopts),
          query.r#type,
          query.query,
          query.query,
        ));
        was_indata = true;
      }
      Incoming::CookieParam(cookie) if !cookie.hidden => {
        lines.push(format!(
          "  let {} = req\n    .cookie(\"{}\")\n    .ok_or(ServerError::from_public(\"Can't find `{}` cookie!\").with_400())?;",
          stringcase::snake_case_with_options(cookie.key.as_str(), &scopts),
          cookie.key,
          cookie.key,
        ));
        was_indata = true;
      }
      Incoming::Body(body) => {
        match body {
          super::incoming::InBody::File { key } => lines.push(format!(
            "  let body = req\n    .file(\"{key}\")\n    .await\n    .ok_or(ServerError::from_public(\"Can't find `{key}` file!\").with_400())?;"
          )),
          super::incoming::InBody::Json { rust_type } => lines.push(format!(
            "  let body = req\n    .parse_json_simd::<{rust_type}>()\n    .await\n    .map_err(|e| ServerError::from_private(e).with_public(\"Can't find JSON with `{rust_type}` type!\").with_400())?;"
          )),
          super::incoming::InBody::MsgPack { rust_type } => lines.push(format!(
            "  let body = req\n    .parse_msgpack::<{rust_type}>()\n    .await\n    .map_err(|e| ServerError::from_private(e).with_public(\"Can't find MsgPack with `{rust_type}` type!\").with_400())?;"
          )),
        }
        was_indata = true;
      }
      Incoming::FormParam(form_param) => {
        lines.push(format!(
          "  let {} = req\n    .form::<{}>(\"{}\")\n    .await\n    .ok_or(ServerError::from_public(\"Can't find `{}` form key!\").with_400())?;",
          stringcase::snake_case_with_options(form_param.key.as_str(), &scopts),
          form_param.r#type,
          form_param.key,
          form_param.key,
        ));
        was_indata = true;
      }
      _ => {}
    }
  }

  if was_indata {
    lines.push(String::new());
  }
  lines.push(String::from("  todo!();\n"));

  let mut out_params = false;
  for outdata in outgoing.iter() {
    match outdata {
      Outgoing::Header(header) if !header.hidden => {
        lines.push(format!(
          "  // res\n  //   .add_header(\"{}\", {}, true)\n  //   .map_err(|e| ServerError::from_private(e).with_500())?;",
          header.name,
          stringcase::snake_case_with_options(header.name.as_str(), &scopts),
        ));
        out_params = true;
      }
      Outgoing::Cookie(cookie) if !cookie.hidden => {
        lines.push(format!(
          "  // res.add_cookie(CookieBuilder::new(\"{}\", {}).build());",
          cookie.key,
          stringcase::snake_case_with_options(cookie.key.as_str(), &scopts),
        ));
        out_params = true;
      }
      _ => {}
    }
  }

  if out_params {
    lines.push(String::new());
  }

  if let Some(Outgoing::Body(body)) = outgoing.iter().find(|o| o.is_body()) {
    match body {
      super::outgoing::OutBody::Ok => lines.push(String::from("  // ok!()")),
      super::outgoing::OutBody::Plain => lines.push(String::from("  // plain!(text)")),
      super::outgoing::OutBody::Html => lines.push(String::from("  // html!(text)")),
      super::outgoing::OutBody::File => lines.push(String::from("  // file_upload!(filepath, filename)")),
      super::outgoing::OutBody::Json { .. } => lines.push(String::from("  // json!(data)")),
      super::outgoing::OutBody::MsgPack { .. } => lines.push(String::from("  // msgpack!(data)")),
    }
  }

  lines.push(String::from("}\n"));

  Ok((used_types, lines))
}

fn tag_name_normalize(tag_name: impl AsRef<str>) -> String {
  let mut tag_name = tag_name.as_ref().chars();
  let tag_name = match tag_name.next() {
    None => String::new(),
    Some(f) => f.to_uppercase().chain(tag_name).collect(),
  };

  tag_name.replace(['_', '-'], " ")
}

pub fn generate_endp_macro_params(incoming: &[&Incoming]) -> String {
  let param_cntr = incoming
    .iter()
    .filter(|i| i.is_header() || i.is_cookie() || i.is_query() || i.path_params_contains_any())
    .count();

  match param_cntr {
    0 => String::new(),
    1 => {
      if let Some(Incoming::Header(header)) = incoming.iter().find(|i| i.is_header()) {
        format!(
          ",\n  parameters((\"{}\" = {}, Header, description = \"\"))",
          header.name, header.r#type
        )
      } else if let Some(Incoming::Path(path)) = incoming.iter().find(|i| i.is_path())
        && !path.params.is_empty()
      {
        if path.params.len() == 1 {
          format!(
            ",\n  parameters((\"{}\" = {}, Path, description = \"\"))",
            path.params[0].key, path.params[0].r#type
          )
        } else {
          format!(
            ",\n  parameters(\n    {}\n  )",
            path
              .params
              .iter()
              .map(|p| format!("(\"{}\" = {}, Path, description = \"\")", p.key, p.r#type))
              .collect::<Vec<_>>()
              .join(",\n    ")
          )
        }
      } else if let Some(Incoming::QueryParam(query)) = incoming.iter().find(|i| i.is_query()) {
        format!(
          ",\n  parameters((\"{}\" = {}, Query, description = \"\"))",
          query.query, query.r#type
        )
      } else if let Some(Incoming::CookieParam(cookie)) = incoming.iter().find(|i| i.is_cookie()) {
        format!(
          ",\n  parameters((\"{}\" = String, Cookie, description = \"\"))",
          cookie.key
        )
      } else {
        unreachable!()
      }
    }
    _ => {
      format!(
        ",\n  parameters(\n    {}\n  )",
        incoming
          .iter()
          .filter_map(|p| match p {
            Incoming::Header(header) => Some(format!(
              "(\"{}\" = {}, Header, description = \"\")",
              header.name, header.r#type
            )),
            Incoming::QueryParam(query) => Some(format!(
              "(\"{}\" = {}, Query, description = \"\")",
              query.query, query.r#type
            )),
            Incoming::CookieParam(cookie) => Some(format!("(\"{}\" = String, Cookie, description = \"\")", cookie.key)),
            Incoming::Path(path) if path.params.len() == 1 => Some(format!(
              "(\"{}\" = {}, Path, description = \"\")",
              path.params[0].key, path.params[0].r#type
            )),
            Incoming::Path(path) if path.params.len() > 1 => Some(
              path
                .params
                .iter()
                .map(|p| format!("(\"{}\" = {}, Path, description = \"\")", p.key, p.r#type))
                .collect::<Vec<_>>()
                .join(",\n    ")
            ),
            _ => None,
          })
          .collect::<Vec<_>>()
          .join(",\n    ")
      )
    }
  }
}

pub fn generate_endp_macro_resp(outgoing: &[&Outgoing]) -> String {
  format!(
    ",\n  responses((\n    status_code = 200,\n    description = \"\"{}{}\n  ))",
    if let Some(Outgoing::Body(body)) = outgoing.iter().find(|o| o.is_body()) {
      match body {
        crate::entities::outgoing::OutBody::Ok => String::new(),
        crate::entities::outgoing::OutBody::Plain => {
          String::from(",\n    body = String,\n    content_type = [\"text/plain\"]")
        }
        crate::entities::outgoing::OutBody::Html => {
          String::from(",\n    body = String,\n    content_type = [\"text/html\"]")
        }
        crate::entities::outgoing::OutBody::File => {
          String::from(",\n    body = Vec<u8>,\n    content_type = [\"application/octet-stream\"]")
        }
        crate::entities::outgoing::OutBody::Json { rust_type } => {
          format!(",\n    body = {rust_type},\n    content_type = [\"application/json\"]")
        }
        crate::entities::outgoing::OutBody::MsgPack { rust_type } => {
          format!(",\n    body = {rust_type},\n    content_type = [\"application/msgpack\"]")
        }
      }
    } else {
      String::new()
    },
    {
      let headers = outgoing
        .iter()
        .filter_map(|o| {
          if let Outgoing::Header(header) = o {
            Some(header)
          } else {
            None
          }
        })
        .collect::<Vec<_>>();
      match headers.len() {
        0 => String::new(),
        1 => format!(
          ",\n    headers((\"{}\" = {}, description = \"\"))",
          headers[0].name, headers[0].r#type
        ),
        _ => format!(
          ",\n    headers(\n      {}\n    )",
          headers
            .iter()
            .map(|h| format!("(\"{}\" = {}, description = \"\")", h.name, h.r#type))
            .collect::<Vec<_>>()
            .join(",\n      "),
        ),
      }
    }
  )
}
