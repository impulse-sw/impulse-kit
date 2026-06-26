//! Publishes the block sources to the Tailwind CSS scanner.
//!
//! Blocks are built on top of `impulse-client-kit-components`, so this crate folds
//! the upstream component bundle into its own published bundle. The `links` key
//! makes the result reach dependents as
//! `DEP_IMPULSE_CLIENT_KIT_BLOCKS_{STYLES,SOURCE_DIR}`; downstream consumers then
//! only ever wire up this single `@source`. See `impulse-tailwind-sources`.

fn main() {
  impulse_tailwind_sources::export(&["DEP_IMPULSE_CLIENT_KIT_COMPONENTS_STYLES"]);
}
