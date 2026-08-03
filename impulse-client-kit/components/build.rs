//! Publishes the component sources to the Tailwind CSS scanner.
//!
//! All the logic lives in `impulse-tailwind-sources`. `impulse-client-kit` sits
//! below this crate and carries a few classes of its own (`utils::safe_area`),
//! so its bundle is folded in here — a consumer then still needs only the one
//! `DEP_IMPULSE_CLIENT_KIT_COMPONENTS_STYLES`.
//! The crate declares `links = "impulse-client-kit-components"`, so the published
//! paths reach dependents as `DEP_IMPULSE_CLIENT_KIT_COMPONENTS_{STYLES,SOURCE_DIR}`.

fn main() {
  impulse_tailwind_sources::export(&["DEP_IMPULSE_CLIENT_KIT_STYLES"]);
}
