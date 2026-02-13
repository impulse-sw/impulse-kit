use impulse_utils::prelude::MResult;
use std::collections::BTreeSet;

use crate::entities::api::TaggedApi;
use crate::entities::file::File;
use crate::entities::incoming::{InBody, Incoming};
use crate::entities::outgoing::{OutBody, Outgoing};
use crate::entities::requirements::unite_requirements;
use crate::entities::types::{STD_TYPES, convert_typedefs, select_typedef, typename_import};

pub fn generate_client_rs_tag(tag: &TaggedApi, api_desc: &File) -> MResult<String> {
  let mut lines = vec![];

  lines.push("use impulse_utils::prelude::*;".to_string());

  let endp_defs = tag
    .endpoints
    .iter()
    .map(|e| generate_client_rs_endpoint(e, tag, api_desc))
    .collect::<Result<Vec<_>, _>>()?;

  // Collect unique types across all endpoints
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

  for endp in &endp_defs {
    lines.extend_from_slice(&endp.1);
  }

  Ok(lines.join("\n"))
}

fn client_return_type(out_body: &OutBody) -> String {
  match out_body {
    OutBody::Ok => "CResult<()>".to_string(),
    OutBody::Plain | OutBody::Html => "CResult<String>".to_string(),
    OutBody::File => "CResult<Vec<u8>>".to_string(),
    OutBody::Json { rust_type } => format!("CResult<{rust_type}>"),
    OutBody::MsgPack { rust_type } => format!("CResult<{rust_type}>"),
  }
}

fn param_type_for_signature(rust_type: &str) -> String {
  match rust_type {
    "String" => "&str".to_string(),
    _ if STD_TYPES.contains(&rust_type) => rust_type.to_string(),
    _ => format!("&{rust_type}"),
  }
}

fn path_for_format(path: &str) -> String {
  // Replace {**key} with {key} for Rust format strings
  let mut result = path.to_string();
  while let Some(start) = result.find("{**") {
    if let Some(end) = result[start..].find('}') {
      let key = &result[start + 3..start + end];
      result = format!("{}{{{key}}}{}", &result[..start], &result[start + end + 1..]);
    } else {
      break;
    }
  }
  result
}

fn generate_client_rs_endpoint(
  endpoint: &crate::entities::api::Api,
  tag: &TaggedApi,
  api_desc: &File,
) -> MResult<(Vec<String>, Vec<String>)> {
  let (incoming, outgoing) = unite_requirements(endpoint, tag, api_desc)?;

  let scopts = stringcase::Options {
    separate_before_non_alphabets: false,
    separate_after_non_alphabets: false,
    ..Default::default()
  };

  // Extract path info
  let path_info = endpoint
    .incoming
    .iter()
    .find(|i| i.is_path())
    .and_then(|i| if let Incoming::Path(p) = i { Some(p) } else { None });

  let (http_method, url_path, path_params) = match path_info {
    Some(path) => (path.request_type.as_str(), path.path.as_str(), &path.params),
    None => return Ok((vec![], vec![])),
  };

  // Collect params in order: path, headers, queries, cookies, body, form
  struct Param {
    name: String,
    param_type: String,
  }

  let mut params: Vec<Param> = vec![];

  // 1. Path params
  for pp in path_params {
    params.push(Param {
      name: stringcase::snake_case_with_options(&pp.key, &scopts),
      param_type: param_type_for_signature(&pp.r#type),
    });
  }

  // 2. Non-hidden headers
  let mut non_hidden_headers = vec![];
  for inc in incoming.iter() {
    if let Incoming::Header(h) = inc
      && !h.hidden
    {
      let var_name = stringcase::snake_case_with_options(&h.name, &scopts);
      params.push(Param {
        name: var_name.clone(),
        param_type: param_type_for_signature(&h.r#type),
      });
      non_hidden_headers.push((h.name.clone(), var_name));
    }
  }

  // 3. Non-hidden query params
  let mut non_hidden_queries = vec![];
  for inc in incoming.iter() {
    if let Incoming::QueryParam(q) = inc
      && !q.hidden
    {
      let var_name = stringcase::snake_case_with_options(&q.query, &scopts);
      params.push(Param {
        name: var_name.clone(),
        param_type: param_type_for_signature(&q.r#type),
      });
      non_hidden_queries.push((q.query.clone(), var_name));
    }
  }

  // 4. Non-hidden cookies
  let mut non_hidden_cookies = vec![];
  for inc in incoming.iter() {
    if let Incoming::CookieParam(c) = inc
      && !c.hidden
    {
      let var_name = stringcase::snake_case_with_options(&c.key, &scopts);
      params.push(Param {
        name: var_name.clone(),
        param_type: "&str".to_string(),
      });
      non_hidden_cookies.push((c.key.clone(), var_name));
    }
  }

  // 5. Body
  let body_info = incoming
    .iter()
    .find(|i| i.is_body())
    .and_then(|i| if let Incoming::Body(b) = i { Some(b) } else { None });

  if let Some(body) = body_info {
    match body {
      InBody::File { .. } => {
        params.push(Param {
          name: "body".to_string(),
          param_type: "Vec<u8>".to_string(),
        });
      }
      InBody::Json { rust_type } | InBody::MsgPack { rust_type } => {
        params.push(Param {
          name: "body".to_string(),
          param_type: format!("&{rust_type}"),
        });
      }
    }
  }

  // 6. Form params
  let mut form_params = vec![];
  for inc in incoming.iter() {
    if let Incoming::FormParam(f) = inc {
      let var_name = stringcase::snake_case_with_options(&f.key, &scopts);
      params.push(Param {
        name: var_name.clone(),
        param_type: f.r#type.clone(),
      });
      form_params.push((f.key.clone(), var_name));
    }
  }

  // Determine return type
  let out_body = outgoing
    .iter()
    .find_map(|o| if let Outgoing::Body(b) = o { Some(b) } else { None });

  let return_type = out_body
    .map(client_return_type)
    .unwrap_or_else(|| "CResult<()>".to_string());

  // Collect used types (non-hidden params + response body)
  let mut used_types = vec![];
  for inc in incoming.iter() {
    match inc {
      Incoming::Header(h) if !h.hidden => {
        if let Ok(tns) = typename_import(&h.r#type) {
          used_types.extend(tns);
        }
      }
      Incoming::QueryParam(q) if !q.hidden => {
        if let Ok(tns) = typename_import(&q.r#type) {
          used_types.extend(tns);
        }
      }
      Incoming::Body(InBody::Json { rust_type } | InBody::MsgPack { rust_type }) => {
        if let Ok(tns) = typename_import(rust_type) {
          used_types.extend(tns);
        }
      }
      Incoming::FormParam(f) => {
        if let Ok(tns) = typename_import(&f.r#type) {
          used_types.extend(tns);
        }
      }
      Incoming::Path(p) => {
        for pp in &p.params {
          if let Ok(tns) = typename_import(&pp.r#type) {
            used_types.extend(tns);
          }
        }
      }
      _ => {}
    }
  }
  for out in outgoing.iter() {
    if let Outgoing::Body(OutBody::Json { rust_type } | OutBody::MsgPack { rust_type }) = out
      && let Ok(tns) = typename_import(rust_type)
    {
      used_types.extend(tns);
    }
  }

  // Build function lines
  let mut lines = vec![];

  // Function signature
  let params_str = params
    .iter()
    .map(|p| format!("{}: {}", p.name, p.param_type))
    .collect::<Vec<_>>()
    .join(", ");

  lines.push(format!(
    "pub async fn {}({params_str}) -> {return_type} {{",
    endpoint.name
  ));

  // Multipart form handling (file body or form params)
  let has_file_body = matches!(body_info, Some(InBody::File { .. }));
  let has_form = !form_params.is_empty();

  if has_file_body || has_form {
    lines.push("  use reqwest::multipart::{Form, Part};".to_string());
    lines.push(String::new());

    if has_file_body {
      let file_key = if let Some(InBody::File { key }) = body_info {
        key.as_str()
      } else {
        "file"
      };
      lines.push(format!(
        "  let form = Form::new()\n    .part(\"{file_key}\", Part::bytes(body));"
      ));
    } else {
      let mut form_line = "  let form = Form::new()".to_string();
      for (key, var_name) in &form_params {
        form_line.push_str(&format!("\n    .part(\"{key}\", Part::bytes({var_name}))"));
      }
      form_line.push(';');
      lines.push(form_line);
    }
    lines.push(String::new());
  }

  // Build request chain
  let fmt_path = path_for_format(url_path);
  let has_path_params = !path_params.is_empty();

  let endpoint_arg = if has_path_params {
    format!("endpoint(&format!(\"{fmt_path}\"))")
  } else {
    format!("endpoint(\"{fmt_path}\")")
  };

  lines.push(format!("  reqwest::Client::new()\n    .{http_method}({endpoint_arg})"));

  // Add headers
  for (header_name, var_name) in &non_hidden_headers {
    lines.push(format!("    .header(\"{header_name}\", {var_name})"));
  }

  // Add cookies via Cookie header
  if !non_hidden_cookies.is_empty() {
    let cookie_str = non_hidden_cookies
      .iter()
      .map(|(key, var)| format!("{key}={{{var}}}"))
      .collect::<Vec<_>>()
      .join("; ");
    lines.push(format!("    .header(\"Cookie\", format!(\"{cookie_str}\"))"));
  }

  // Add query params
  if !non_hidden_queries.is_empty() {
    let query_pairs = non_hidden_queries
      .iter()
      .map(|(key, var)| format!("(\"{key}\", {var}.to_string())"))
      .collect::<Vec<_>>()
      .join(", ");
    lines.push(format!("    .query(&[{query_pairs}])"));
  }

  // Add body
  match body_info {
    Some(InBody::Json { .. }) => {
      lines.push("    .json(body)".to_string());
    }
    Some(InBody::MsgPack { .. }) => {
      lines.push("    .msgpack(body)?".to_string());
    }
    Some(InBody::File { .. }) | None if has_form || has_file_body => {
      lines.push("    .multipart(form)".to_string());
    }
    _ => {}
  }

  // Send and handle response
  lines.push("    .send()".to_string());
  lines.push("    .await".to_string());
  lines.push("    .map_err(ClientError::from)?".to_string());
  lines.push("    .collect_server_error()".to_string());
  lines.push("    .await?".to_string());

  // Response parsing based on out body type
  match out_body {
    Some(OutBody::Ok) | None => {
      // Terminate the chain, return Ok(())
      let last = lines.last_mut().unwrap();
      last.push(';');
      lines.push("  Ok(())".to_string());
    }
    Some(OutBody::Plain) | Some(OutBody::Html) => {
      lines.push("    .text()".to_string());
      lines.push("    .await".to_string());
      lines.push("    .map_err(ClientError::from)".to_string());
    }
    Some(OutBody::File) => {
      lines.push("    .bytes()".to_string());
      lines.push("    .await".to_string());
      lines.push("    .map(|b| b.to_vec())".to_string());
      lines.push("    .map_err(ClientError::from)".to_string());
    }
    Some(OutBody::Json { .. }) => {
      lines.push("    .json()".to_string());
      lines.push("    .await".to_string());
      lines.push("    .map_err(ClientError::from)".to_string());
    }
    Some(OutBody::MsgPack { .. }) => {
      lines.push("    .msgpack()".to_string());
      lines.push("    .await".to_string());
    }
  }

  lines.push("}\n".to_string());

  Ok((used_types, lines))
}
