//! Turns the `tauri` Cargo feature into a plain `cfg(tauri)` so downstream code
//! can gate on `#[cfg(tauri)]` (as well as `#[cfg(feature = "tauri")]`). This
//! keeps the switch that selects the Tauri IPC transport a single, readable
//! attribute across the crate.

fn main() {
  // Declare the custom cfg so `-Zcheck-cfg` / stable check-cfg doesn't warn.
  println!("cargo::rustc-check-cfg=cfg(tauri)");
  if std::env::var_os("CARGO_FEATURE_TAURI").is_some() {
    println!("cargo::rustc-cfg=tauri");
  }
}
