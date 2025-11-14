use impulse_utils::prelude::{MResult, ServerError};
use serde::{Deserialize, Serialize};

use crate::entities::incoming::{Cookie, Header};
use crate::entities::types::parse_typename;

#[derive(Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[serde(rename_all = "snake_case", tag = "data_into")]
pub enum Outgoing {
  Header(Header),
  Cookie(Cookie),
  Body(OutBody),
}

impl Outgoing {
  pub fn get_used_types(&self) -> Vec<String> {
    match self {
      Self::Header(header) => vec![header.r#type.to_string()],
      Self::Body(body) => match body {
        OutBody::Json { rust_type } | OutBody::MsgPack { rust_type } => vec![rust_type.to_owned()],
        _ => vec![],
      },
      Self::Cookie(_) => vec![],
    }
  }

  pub fn require_salvo_res(&self) -> bool {
    self.is_cookie() || self.is_header()
  }

  pub fn is_body(&self) -> bool {
    matches!(self, Outgoing::Body(_))
  }
  pub fn is_cookie(&self) -> bool {
    matches!(self, Outgoing::Cookie(_))
  }
  pub fn is_header(&self) -> bool {
    matches!(self, Outgoing::Header(_))
  }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[serde(rename_all = "snake_case", tag = "body_type")]
pub enum OutBody {
  Ok,
  Plain,
  Html,
  File,
  Json { rust_type: String },
  MsgPack { rust_type: String },
}

impl OutBody {
  pub fn return_type_str(&self) -> String {
    match self {
      Self::Ok => String::from("MResult<OK>"),
      Self::Plain => String::from("MResult<Plain>"),
      Self::Html => String::from("MResult<Html>"),
      Self::File => String::from("MResult<File>"),
      Self::Json { rust_type } => format!("MResult<Json<{rust_type}>>"),
      Self::MsgPack { rust_type } => format!("MResult<MsgPack<{rust_type}>>"),
    }
  }
}

pub fn parse_outgoing_data(outgoing_data: &str) -> MResult<Outgoing> {
  let parts = outgoing_data.split("/").collect::<Vec<_>>();

  match parts[0] {
    "ok" => {
      if parts.len() != 1 {
        return ServerError::from_public(format!(
          "Invalid HTTP 200 OK (without body) outgoing data: `{outgoing_data}`"
        ))
        .bail();
      }
      Ok(Outgoing::Body(OutBody::Ok))
    }
    "h" => {
      if parts.len() != 3 {
        return ServerError::from_public(format!("Invalid header outgoing data: `{outgoing_data}`")).bail();
      }
      Ok(Outgoing::Header(Header {
        r#type: parse_typename(parts[1]),
        name: parts[2].to_owned(),
        hidden: false,
      }))
    }
    "c" => {
      if parts.len() != 2 {
        return ServerError::from_public(format!("Invalid cookie outgoing data: `{outgoing_data}`")).bail();
      }
      Ok(Outgoing::Cookie(Cookie {
        key: parts[1].to_owned(),
        hidden: false,
      }))
    }
    "b" => match parts[1] {
      "plain" => {
        if parts.len() != 2 {
          return ServerError::from_public(format!("Invalid plain body outgoing data: `{outgoing_data}`")).bail();
        }
        Ok(Outgoing::Body(OutBody::Plain))
      }
      "html" => {
        if parts.len() != 2 {
          return ServerError::from_public(format!("Invalid HTML body outgoing data: `{outgoing_data}`")).bail();
        }
        Ok(Outgoing::Body(OutBody::Html))
      }
      "file" => {
        if parts.len() != 2 {
          return ServerError::from_public(format!("Invalid file body outgoing data: `{outgoing_data}`")).bail();
        }
        Ok(Outgoing::Body(OutBody::File))
      }
      "json" => {
        if parts.len() != 3 {
          return ServerError::from_public(format!("Invalid JSON body outgoing data: `{outgoing_data}`")).bail();
        }
        Ok(Outgoing::Body(OutBody::Json {
          rust_type: parse_typename(parts[2]),
        }))
      }
      "msgpack" => {
        if parts.len() != 3 {
          return ServerError::from_public(format!("Invalid MsgPack body outgoing data: `{outgoing_data}`")).bail();
        }
        Ok(Outgoing::Body(OutBody::MsgPack {
          rust_type: parse_typename(parts[2]),
        }))
      }
      _ => ServerError::from_public(format!(
        "Invalid body type `{}` in outgoing data `{outgoing_data}`",
        parts[1]
      ))
      .bail(),
    },
    _ => ServerError::from_public(format!("Invalid outgoing data: `{outgoing_data}`")).bail(),
  }
}

pub fn parse_outgoing_data_except_body(outgoing_data: &str) -> MResult<Outgoing> {
  let outgoing = parse_outgoing_data(outgoing_data)?;
  if let Outgoing::Body(_) = &outgoing {
    ServerError::from_public(format!("Can't construct requirement with body: `{outgoing_data}`")).bail()
  } else {
    Ok(outgoing)
  }
}
