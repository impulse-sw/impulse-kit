pub mod api;
pub mod client_js;
pub mod client_rs;
pub mod evolution;
pub mod file;
pub mod incoming;
pub mod outgoing;
pub mod requirements;
pub mod types;
pub mod versions;

#[derive(Debug, Clone, PartialEq)]
pub enum ClientTarget {
  Rust,
  Js,
}
