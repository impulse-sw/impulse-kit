//! Generates the `@source` partial that `input.css` imports, from the path
//! published by `impulse-ui-kit-components`' build script. See
//! `impulse-tailwind-sources` and the components crate README for the pattern.

use std::env;
use std::path::Path;

fn main() {
  let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by Cargo");
  let partial = Path::new(&manifest_dir).join(".tailwind-sources.css");
  impulse_tailwind_sources::write_source_partial(partial, &["DEP_IMPULSE_UI_KIT_COMPONENTS_STYLES"]);
}
