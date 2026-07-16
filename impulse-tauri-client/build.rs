//! Turns the `tauri` Cargo feature into a plain `cfg(tauri)` so the transport can
//! gate the IPC backend on `#[cfg(tauri)]`. Mirrors `impulse-client-kit`'s switch
//! so the same feature flips both the REST client and the WebSocket bridge.

fn main() {
  // Declare the custom cfg so stable check-cfg doesn't warn.
  println!("cargo::rustc-check-cfg=cfg(tauri)");
  if std::env::var_os("CARGO_FEATURE_TAURI").is_some() {
    println!("cargo::rustc-cfg=tauri");
  }
}
