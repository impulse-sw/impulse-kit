//! Publishes the component sources to the Tailwind CSS scanner.
//!
//! All the logic lives in `impulse-tailwind-sources`. This crate sits at the
//! bottom of the stack, so it has no upstream component libraries to forward.
//! The crate declares `links = "impulse-client-kit-components"`, so the published
//! paths reach dependents as `DEP_IMPULSE_UI_KIT_COMPONENTS_{STYLES,SOURCE_DIR}`.

fn main() {
  impulse_tailwind_sources::export(&[]);
}
