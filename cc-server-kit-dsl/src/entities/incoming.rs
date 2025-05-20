use cc_utils::prelude::{MResult, ServerError};
use serde::{Deserialize, Serialize};

use crate::entities::types::parse_typename;

#[derive(Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[serde(rename_all = "snake_case", tag = "data_from")]
pub enum Incoming {
  Header(Header),
  Body(InBody),
  Path(Path),
  QueryParam(Query),
  FormParam(Form),
  CookieParam(Cookie),
}

impl Incoming {
  pub fn get_used_types(&self) -> Vec<String> {
    match self {
      Self::Header(header) => vec![header.r#type.to_string()],
      Self::Body(body) => match body {
        InBody::File { .. } => vec![],
        InBody::Json { rust_type } | InBody::MsgPack { rust_type } => vec![rust_type.to_owned()],
      },
      Self::Path(path) => path.params.iter().map(|v| v.r#type.to_string()).collect(),
      Self::QueryParam(query) => vec![query.r#type.to_string()],
      Self::FormParam(form) => vec![form.r#type.to_string()],
      Self::CookieParam(_) => vec![],
    }
  }

  pub fn require_salvo_req(&self) -> bool {
    self.is_body()
      || self.is_form()
      || self.is_cookie()
      || self.is_header()
      || self.is_query()
      || self.path_params_contains_any()
  }

  pub fn is_header(&self) -> bool {
    matches!(self, Incoming::Header(_))
  }
  pub fn is_query(&self) -> bool {
    matches!(self, Incoming::QueryParam(_))
  }
  pub fn is_body(&self) -> bool {
    matches!(self, Incoming::Body(_))
  }
  pub fn is_path(&self) -> bool {
    matches!(self, Incoming::Path(_))
  }
  pub fn path_params_contains_any(&self) -> bool {
    if let Incoming::Path(path) = self
      && !path.params.is_empty()
    {
      true
    } else {
      false
    }
  }
  pub fn is_form(&self) -> bool {
    matches!(self, Incoming::FormParam(_))
  }
  pub fn is_cookie(&self) -> bool {
    matches!(self, Incoming::CookieParam(_))
  }
}

pub const ALLOWED_HTTP_METHODS: [&str; 5] = ["get", "post", "put", "patch", "delete"];

#[derive(Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Header {
  pub r#type: String,
  pub name: String,
  pub hidden: bool,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[serde(rename_all = "snake_case", tag = "body_type")]
pub enum InBody {
  File { key: String },
  Json { rust_type: String },
  MsgPack { rust_type: String },
}

#[derive(Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Path {
  pub request_type: String,
  pub path: String,
  pub params: Vec<PathParam>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct PathParam {
  pub r#type: String,
  pub key: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Query {
  pub r#type: String,
  pub query: String,
  pub hidden: bool,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Form {
  pub r#type: String,
  pub key: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Cookie {
  pub key: String,
  pub hidden: bool,
}

pub fn parse_incoming_data(incoming_data: &str) -> MResult<Incoming> {
  let parts = incoming_data.split("/").collect::<Vec<_>>();

  match parts[0] {
    "h" => {
      if parts.len() != 3 {
        return ServerError::from_public(format!("Invalid header incoming data: `{}`", incoming_data)).bail();
      }
      Ok(Incoming::Header(Header {
        r#type: parse_typename(parts[1]),
        name: parts[2].to_owned(),
        hidden: false,
      }))
    }
    "b" => match parts[1] {
      "file" => {
        if parts.len() != 3 {
          return ServerError::from_public(format!("Invalid file body incoming data: `{}`", incoming_data)).bail();
        }
        Ok(Incoming::Body(InBody::File {
          key: parts[2].to_owned(),
        }))
      }
      "json" => {
        if parts.len() != 3 {
          return ServerError::from_public(format!("Invalid JSON body incoming data: `{}`", incoming_data)).bail();
        }
        Ok(Incoming::Body(InBody::Json {
          rust_type: parse_typename(parts[2]),
        }))
      }
      "msgpack" => {
        if parts.len() != 3 {
          return ServerError::from_public(format!("Invalid MsgPack body incoming data: `{}`", incoming_data)).bail();
        }
        Ok(Incoming::Body(InBody::MsgPack {
          rust_type: parse_typename(parts[2]),
        }))
      }
      _ => ServerError::from_public(format!(
        "Invalid body type `{}` in incoming data `{}`",
        parts[1], incoming_data
      ))
      .bail(),
    },
    "q" => {
      if parts.len() != 3 {
        return ServerError::from_public(format!("Invalid query incoming data: `{}`", incoming_data)).bail();
      }
      Ok(Incoming::QueryParam(Query {
        r#type: parse_typename(parts[1]),
        query: parts[2].to_owned(),
        hidden: false,
      }))
    }
    "f" => {
      if parts.len() != 3 {
        return ServerError::from_public(format!("Invalid form incoming data: `{}`", incoming_data)).bail();
      }
      Ok(Incoming::FormParam(Form {
        r#type: parse_typename(parts[1]),
        key: parts[2].to_owned(),
      }))
    }
    "c" => {
      if parts.len() != 2 {
        return ServerError::from_public(format!("Invalid cookie incoming data: `{}`", incoming_data)).bail();
      }
      Ok(Incoming::CookieParam(Cookie {
        key: parts[1].to_owned(),
        hidden: false,
      }))
    }
    method if ALLOWED_HTTP_METHODS.contains(&method) => Ok(Incoming::Path(parse_http_path(&parts)?)),
    _ => ServerError::from_public(format!("Invalid incoming data: `{}`", incoming_data)).bail(),
  }
}

pub fn parse_incoming_data_except_path(incoming_data: &str) -> MResult<Incoming> {
  let incoming = parse_incoming_data(incoming_data)?;
  if let Incoming::Path(_) = &incoming {
    ServerError::from_public(format!("Can't construct requirement with path: `{}`", incoming_data)).bail()
  } else {
    Ok(incoming)
  }
}

pub fn parse_http_path(incoming_parts: &[&str]) -> MResult<Path> {
  let mut request = Path {
    request_type: incoming_parts[0].to_owned(),
    path: String::new(),
    params: vec![],
  };

  let mut i = 1;
  while i < incoming_parts.len() {
    if !incoming_parts[i].starts_with('{') {
      request.path.push('/');
      request.path.push_str(incoming_parts[i]);
    } else if i + 1 < incoming_parts.len() && incoming_parts[i + 1].ends_with('}') {
      let sp_len = incoming_parts[i + 1].len();
      request.path.push_str("/{");
      request.path.push_str(incoming_parts[i + 1]);
      request.params.push(PathParam {
        r#type: parse_typename(&incoming_parts[i][1..]),
        key: incoming_parts[i + 1][..sp_len - 1].to_string(),
      });
      i += 1;
    } else if incoming_parts[i].starts_with("{**") && incoming_parts[i].ends_with('}') {
      let p_len = incoming_parts[i].len();
      request.path.push('/');
      request.path.push_str(incoming_parts[i]);
      request.params.push(PathParam {
        r#type: String::from("String"),
        key: incoming_parts[i][3..p_len - 1].to_string(),
      });
    } else {
      ServerError::from_public(format!(
        "Invalid `{{` found at HTTP path: `{}`",
        incoming_parts.join("/")
      ))
      .bail()?;
    }

    i += 1;
  }

  Ok(request)
}
