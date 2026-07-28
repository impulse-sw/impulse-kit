//! Resolving C entry points from the libraries **already loaded in this
//! process**, rather than linking them.
//!
//! Both native providers need a handful of functions from libraries the host app
//! already has open — GTK on the Linux desktop, the Android runtime's JNI
//! invocation API on Android. Declaring those as ordinary link-time dependencies
//! would force every consumer (headless CI lint runs included) to have the
//! matching development packages, and would make a missing library a link error
//! instead of a graceful "no palette here". Looking them up at runtime keeps the
//! crate's dependency footprint at nothing and turns absence into `None`.

use std::ffi::{CString, c_char, c_void};

unsafe extern "C" {
  /// Looks a symbol up across everything already loaded in this process.
  fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

/// Resolves an already-loaded function by name, or `None` when it isn't there.
///
/// # Safety
///
/// The caller must instantiate `F` with the symbol's real C signature.
pub(crate) unsafe fn symbol<F: Copy>(name: &str) -> Option<F> {
  debug_assert_eq!(size_of::<F>(), size_of::<*mut c_void>());
  let cname = CString::new(name).ok()?;
  // SAFETY: `cname` is a valid NUL-terminated string; a null handle means
  // "search the global scope" (`RTLD_DEFAULT`), and a missing symbol yields null.
  let addr = unsafe { dlsym(std::ptr::null_mut(), cname.as_ptr()) };
  if addr.is_null() {
    return None;
  }
  // SAFETY: `addr` is a live function pointer for as long as the process holds
  // the library open, and `F` matches its signature per this fn's contract.
  Some(unsafe { *(&addr as *const *mut c_void as *const F) })
}
