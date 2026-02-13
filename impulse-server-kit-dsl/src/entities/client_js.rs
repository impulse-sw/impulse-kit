use impulse_utils::prelude::MResult;

use crate::entities::api::TaggedApi;
use crate::entities::file::File;
use crate::entities::incoming::{InBody, Incoming};
use crate::entities::outgoing::{OutBody, Outgoing};
use crate::entities::requirements::unite_requirements;

pub fn generate_client_js_tag(tag: &TaggedApi, api_desc: &File) -> MResult<String> {
  let mut lines = vec![];

  let endp_defs = tag
    .endpoints
    .iter()
    .map(|e| generate_client_js_endpoint(e, tag, api_desc))
    .collect::<Result<Vec<_>, _>>()?;

  // Check if any endpoint uses msgpack (request or response)
  let mut needs_encode = false;
  let mut needs_decode = false;

  for endp in &tag.endpoints {
    let (incoming, outgoing) = unite_requirements(endp, tag, api_desc)?;
    for inc in &incoming {
      if let Incoming::Body(InBody::MsgPack { .. }) = inc {
        needs_encode = true;
      }
    }
    for out in &outgoing {
      if let Outgoing::Body(OutBody::MsgPack { .. }) = out {
        needs_decode = true;
      }
    }
  }

  if needs_encode || needs_decode {
    let imports = match (needs_encode, needs_decode) {
      (true, true) => "encode, decode",
      (true, false) => "encode",
      (false, true) => "decode",
      _ => unreachable!(),
    };
    lines.push(format!("import {{ {imports} }} from '@msgpack/msgpack';"));
  }

  lines.push("import { endpoint } from './endpoint.js';".to_string());
  lines.push(String::new());

  for endp in &endp_defs {
    lines.extend_from_slice(endp);
  }

  Ok(lines.join("\n"))
}

fn to_camel_case(s: &str) -> String {
  let parts: Vec<&str> = s.split(['_', '-']).collect();
  let mut result = String::new();
  for (i, part) in parts.iter().enumerate() {
    if part.is_empty() {
      continue;
    }
    if i == 0 {
      result.push_str(&part.to_lowercase());
    } else {
      let mut chars = part.chars();
      if let Some(first) = chars.next() {
        result.push(first.to_uppercase().next().unwrap_or(first));
        result.extend(chars.map(|c| c.to_lowercase().next().unwrap_or(c)));
      }
    }
  }
  result
}

fn js_path_template(path: &str, path_params: &[crate::entities::incoming::PathParam]) -> String {
  let mut result = path.to_string();
  for pp in path_params {
    let js_name = to_camel_case(&pp.key);
    let from_wildcard = format!("{{**{}}}", pp.key);
    let from_normal = format!("{{{}}}", pp.key);
    if result.contains(&from_wildcard) {
      result = result.replace(&from_wildcard, &format!("${{{js_name}}}"));
    } else {
      result = result.replace(&from_normal, &format!("${{{js_name}}}"));
    }
  }
  result
}

fn generate_client_js_endpoint(
  endpoint: &crate::entities::api::Api,
  tag: &TaggedApi,
  api_desc: &File,
) -> MResult<Vec<String>> {
  let (incoming, outgoing) = unite_requirements(endpoint, tag, api_desc)?;

  // Extract path info from endpoint's own incoming
  let path_info = endpoint
    .incoming
    .iter()
    .find(|i| i.is_path())
    .and_then(|i| if let Incoming::Path(p) = i { Some(p) } else { None });

  let (http_method, url_path, path_params) = match path_info {
    Some(path) => (path.request_type.as_str(), path.path.as_str(), &path.params),
    None => return Ok(vec![]),
  };

  // Collect params in order
  let mut js_params: Vec<String> = vec![];

  // 1. Path params
  for pp in path_params {
    js_params.push(to_camel_case(&pp.key));
  }

  // 2. Non-hidden headers
  let mut non_hidden_headers: Vec<(String, String)> = vec![];
  for inc in incoming.iter() {
    if let Incoming::Header(h) = inc
      && !h.hidden
    {
      let js_name = to_camel_case(&h.name);
      js_params.push(js_name.clone());
      non_hidden_headers.push((h.name.clone(), js_name));
    }
  }

  // 3. Non-hidden query params
  let mut non_hidden_queries: Vec<(String, String)> = vec![];
  for inc in incoming.iter() {
    if let Incoming::QueryParam(q) = inc
      && !q.hidden
    {
      let js_name = to_camel_case(&q.query);
      js_params.push(js_name.clone());
      non_hidden_queries.push((q.query.clone(), js_name));
    }
  }

  // 4. Non-hidden cookies
  let mut non_hidden_cookies: Vec<(String, String)> = vec![];
  for inc in incoming.iter() {
    if let Incoming::CookieParam(c) = inc
      && !c.hidden
    {
      let js_name = to_camel_case(&c.key);
      js_params.push(js_name.clone());
      non_hidden_cookies.push((c.key.clone(), js_name));
    }
  }

  // 5. Body
  let body_info = incoming
    .iter()
    .find(|i| i.is_body())
    .and_then(|i| if let Incoming::Body(b) = i { Some(b) } else { None });

  if body_info.is_some() {
    js_params.push("body".to_string());
  }

  // 6. Form params
  let mut form_params: Vec<(String, String)> = vec![];
  for inc in incoming.iter() {
    if let Incoming::FormParam(f) = inc {
      let js_name = to_camel_case(&f.key);
      js_params.push(js_name.clone());
      form_params.push((f.key.clone(), js_name));
    }
  }

  // Determine response type
  let out_body = outgoing
    .iter()
    .find_map(|o| if let Outgoing::Body(b) = o { Some(b) } else { None });

  let fn_name = to_camel_case(&endpoint.name);
  let params_str = js_params.join(", ");

  let mut lines = vec![];
  lines.push(format!("export async function {fn_name}({params_str}) {{"));

  // Query params
  if !non_hidden_queries.is_empty() {
    lines.push("  const params = new URLSearchParams();".to_string());
    for (key, js_name) in &non_hidden_queries {
      lines.push(format!("  params.set(\"{key}\", {js_name});"));
    }
  }

  // Form data
  let has_file_body = matches!(body_info, Some(InBody::File { .. }));
  let has_form = !form_params.is_empty();

  if has_file_body || has_form {
    lines.push("  const formData = new FormData();".to_string());
    if has_file_body {
      let file_key = if let Some(InBody::File { key }) = body_info {
        key.as_str()
      } else {
        "file"
      };
      lines.push(format!("  formData.append(\"{file_key}\", new Blob([body]));"));
    }
    for (key, js_name) in &form_params {
      lines.push(format!("  formData.append(\"{key}\", new Blob([{js_name}]));"));
    }
  }

  // Build URL
  let js_path = js_path_template(url_path, path_params);
  let has_path_params = !path_params.is_empty();
  let has_queries = !non_hidden_queries.is_empty();

  let url_expr = match (has_path_params || has_queries, has_queries) {
    (false, false) => format!("endpoint(\"{js_path}\")"),
    (true, false) => format!("endpoint(`{js_path}`)"),
    (_, true) => format!("endpoint(`{js_path}?${{params}}`)"),
  };

  // Build headers object
  let mut header_entries: Vec<String> = vec![];

  // Content-Type
  match body_info {
    Some(InBody::Json { .. }) => {
      header_entries.push("\"Content-Type\": \"application/json\"".to_string());
    }
    Some(InBody::MsgPack { .. }) => {
      header_entries.push("\"Content-Type\": \"application/msgpack\"".to_string());
    }
    _ => {}
  }

  for (header_name, js_name) in &non_hidden_headers {
    header_entries.push(format!("\"{header_name}\": {js_name}"));
  }

  if !non_hidden_cookies.is_empty() {
    let cookie_parts: Vec<String> = non_hidden_cookies
      .iter()
      .map(|(key, js_name)| format!("{key}=${{{js_name}}}"))
      .collect();
    header_entries.push(format!("\"Cookie\": `{}`", cookie_parts.join("; ")));
  }

  // Build fetch options
  let method_upper = http_method.to_uppercase();
  lines.push(format!("  const response = await fetch({url_expr}, {{"));
  lines.push(format!("    method: \"{method_upper}\","));

  if !header_entries.is_empty() {
    lines.push("    headers: {".to_string());
    for (i, entry) in header_entries.iter().enumerate() {
      let comma = if i < header_entries.len() - 1 { "," } else { "" };
      lines.push(format!("      {entry}{comma}"));
    }
    lines.push("    },".to_string());
  }

  // Body
  if has_file_body || has_form {
    lines.push("    body: formData,".to_string());
  } else {
    match body_info {
      Some(InBody::Json { .. }) => {
        lines.push("    body: JSON.stringify(body),".to_string());
      }
      Some(InBody::MsgPack { .. }) => {
        lines.push("    body: encode(body),".to_string());
      }
      _ => {}
    }
  }

  lines.push("  });".to_string());

  // Error handling
  lines.push("  if (!response.ok) {".to_string());
  lines.push("    const err = await response.text();".to_string());
  lines.push("    throw new Error(err);".to_string());
  lines.push("  }".to_string());

  // Response parsing
  match out_body {
    Some(OutBody::Ok) | None => {
      // No return value
    }
    Some(OutBody::Plain) | Some(OutBody::Html) => {
      lines.push("  return await response.text();".to_string());
    }
    Some(OutBody::File) => {
      lines.push("  return await response.arrayBuffer();".to_string());
    }
    Some(OutBody::Json { .. }) => {
      lines.push("  return await response.json();".to_string());
    }
    Some(OutBody::MsgPack { .. }) => {
      lines.push("  return decode(new Uint8Array(await response.arrayBuffer()));".to_string());
    }
  }

  lines.push("}".to_string());
  lines.push(String::new());

  Ok(lines)
}
