//! Material You (MD3) dynamic-colour provider for Android.
//!
//! Android 12 (API 31) and later publish the user's wallpaper-derived tonal
//! palettes as framework colour resources: `android.R.color.system_neutral1_*`
//! and `system_neutral2_*` (plus accent ramps this crate deliberately ignores —
//! the app keeps its own brand accent).
//!
//! # Getting at the JVM without `ndk-context`
//!
//! The usual way to reach JNI from Rust on Android is `ndk_context`, but its
//! `android_context()` **panics** when nothing has initialised it — and nothing
//! does here: Tauri's Android backend (tao/wry) talks to `ndk`/`jni` directly
//! and never calls `initialize_android_context`. Under `panic = "abort"` that
//! panic takes the whole app down at startup, so this provider avoids the crate
//! entirely and takes the `JavaVM` straight from the JVM, in [`JNI_OnLoad`] —
//! no Activity, no initialisation order to rely on. (Asking the JNI invocation
//! API by symbol is kept only as a fallback: the Android runtime's own symbols
//! are not necessarily visible from an app's linker namespace.) Every step
//! yields an `Option`, so a device that can't answer simply leaves the app with
//! its own palette.
//!
//! # Which `Resources` we read
//!
//! Dynamic colour reaches an app through a resource overlay on **its** resource
//! table, so the palette is read from the application context's `Resources`
//! (via `ActivityThread.currentApplication()`). `Resources.getSystem()` is kept
//! as a fallback: it always resolves, but being overlay-free it can hand back
//! the stock, un-personalised ramp.
//!
//! # Shades vs MD3 tones
//!
//! The resource names are *shades*, where `0` is white and `1000` is black —
//! the inverse of the MD3 tone scale (`tone = 100 - shade / 10`). Available
//! shades are 0, 10, 50, 100, 200 … 900, 1000, so the MD3 roles are mapped to
//! the nearest published shade:
//!
//! | MD3 role (light / dark)      | shade used     |
//! |------------------------------|----------------|
//! | surface                      | n1 10 / n1 900 |
//! | on-surface                   | n1 900 / n1 100|
//! | surface-variant              | n2 100 / n2 700|
//! | on-surface-variant           | n2 700 / n2 200|
//! | outline                      | n2 500 / n2 400|
//! | outline-variant              | n2 200 / n2 700|
//!
//! On API < 31 the resources don't exist, `getIdentifier` answers `0`, and the
//! provider reports nothing.

use std::ffi::{CString, c_char, c_int, c_void};
use std::sync::atomic::{AtomicPtr, Ordering};

use jni::objects::{JObject, JValue};
use jni::{JNIEnv, JavaVM};

use crate::dynsym;
use crate::{BaseNeutrals, NativeBaseTheme};

/// `jint JNI_GetCreatedJavaVMs(JavaVM **vmBuf, jsize bufLen, jsize *nVMs)`
type FnGetCreatedJavaVMs = unsafe extern "C" fn(*mut *mut jni::sys::JavaVM, i32, *mut i32) -> i32;

const JNI_OK: i32 = 0;

/// The `JavaVM` handed to us when the app's library was loaded. See [`JNI_OnLoad`].
static VM: AtomicPtr<jni::sys::JavaVM> = AtomicPtr::new(std::ptr::null_mut());

/// The JVM calls this when the app loads the native library, which is the
/// earliest and most reliable way to get hold of the `JavaVM` — no Activity, no
/// initialisation order to depend on, and none of the linker-namespace
/// uncertainty that surrounds looking up the JNI invocation API by symbol.
///
/// Tauri's Android stack (tao/wry) defines no `JNI_OnLoad` of its own, so this
/// entry point is free to claim. It only records the pointer; returning the JNI
/// version the app is built against keeps the library load succeeding.
///
/// # Safety
///
/// Called by the JVM with a valid `JavaVM` pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn JNI_OnLoad(vm: *mut jni::sys::JavaVM, _reserved: *mut c_void) -> jni::sys::jint {
  VM.store(vm, Ordering::Release);
  jni::sys::JNI_VERSION_1_6
}

/// Keeps [`JNI_OnLoad`] in the final library. A `#[no_mangle]` function that
/// lives in a dependency and is called by nobody can be dropped when the linker
/// garbage-collects sections; referencing it from a `#[used]` static in the same
/// crate — which *is* linked in, since the app calls
/// [`crate::install_native_base_theme`] — pins it.
#[used]
static KEEP_JNI_ON_LOAD: unsafe extern "C" fn(*mut jni::sys::JavaVM, *mut c_void) -> jni::sys::jint = JNI_OnLoad;

/// `int __android_log_write(int prio, const char *tag, const char *text)`
type FnLogWrite = unsafe extern "C" fn(c_int, *const c_char, *const c_char) -> c_int;

const ANDROID_LOG_INFO: c_int = 4;
const LOG_TAG: &str = "impulse-native-themes";

/// Writes a line to logcat. Android apps rarely install a `tracing` subscriber,
/// so the platform's own log is what actually reaches `adb logcat`.
fn log(message: &str) {
  // SAFETY: matches `__android_log_write`'s signature; liblog is always loaded.
  let Some(write) = (unsafe { dynsym::symbol::<FnLogWrite>("__android_log_write") }) else {
    return;
  };
  let (Ok(tag), Ok(text)) = (CString::new(LOG_TAG), CString::new(message)) else {
    return;
  };
  // SAFETY: both pointers are valid NUL-terminated strings for the call.
  unsafe { write(ANDROID_LOG_INFO, tag.as_ptr(), text.as_ptr()) };
}

/// `WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS` — dark icons, for use
/// over a light background.
const APPEARANCE_LIGHT_STATUS_BARS: i32 = 0x0000_0008;

/// Matches the status bar icons to the background the app is drawing under them.
///
/// A Tauri app draws edge-to-edge, so the status bar sits over the app's own
/// background. Android picks the icon colour once, from the app's theme, and
/// never revisits it — so an app whose theme is switched at runtime ends up with
/// (say) white icons on a white background. Pass `dark_background = true` for a
/// dark app background (light icons) and `false` for a light one (dark icons).
///
/// Must run on the UI thread with the real Activity — i.e. from
/// `PlatformWebview::jni_handle().exec(..)`. Failures are logged and otherwise
/// ignored: a mis-tinted status bar is not worth taking the app down for.
pub fn apply_status_bar_appearance(env: &mut JNIEnv<'_>, activity: &JObject<'_>, dark_background: bool) {
  let appearance = if dark_background {
    0
  } else {
    APPEARANCE_LIGHT_STATUS_BARS
  };
  if set_status_bar_appearance(env, activity, appearance).is_none() {
    clear_exception(env);
    log("could not set the status bar appearance (needs Android 11 / API 30)");
  }
}

fn set_status_bar_appearance(env: &mut JNIEnv<'_>, activity: &JObject<'_>, appearance: i32) -> Option<()> {
  let window = call_object(env, activity, "getWindow", "()Landroid/view/Window;")?;
  // `Window.getInsetsController()` is API 30+; older systems keep the icon
  // colour the theme gave them.
  let controller = call_object(
    env,
    &window,
    "getInsetsController",
    "()Landroid/view/WindowInsetsController;",
  )?;
  env
    .call_method(
      &controller,
      "setSystemBarsAppearance",
      "(II)V",
      // The mask restricts the change to the status-bar bit, leaving the
      // navigation bar's appearance alone.
      &[JValue::Int(appearance), JValue::Int(APPEARANCE_LIGHT_STATUS_BARS)],
    )
    .ok()?;
  Some(())
}

/// Reads the Material You tonal palettes and derives both colour schemes.
pub(crate) fn capture() -> Option<NativeBaseTheme> {
  let vm = java_vm()?;
  let mut env = vm.attach_current_thread().ok()?;
  let context = app_context(&mut env);
  let resources = app_resources(&mut env)?;

  let mut neutral = |palette: u8, shade: u16| {
    let colour = color_hex(&mut env, &resources, palette, shade);
    if colour.is_none() {
      log(&format!(
        "system_neutral{palette}_{shade} unavailable — dynamic colour needs Android 12 (API 31)"
      ));
    }
    colour
  };

  // Neutral 1 — surfaces and the text on them.
  let n1_0 = neutral(1, 0)?;
  let n1_10 = neutral(1, 10)?;
  let n1_100 = neutral(1, 100)?;
  let n1_800 = neutral(1, 800)?;
  let n1_900 = neutral(1, 900)?;
  // Neutral 2 — surface variants, outlines.
  let n2_100 = neutral(2, 100)?;
  let n2_200 = neutral(2, 200)?;
  let n2_400 = neutral(2, 400)?;
  let n2_500 = neutral(2, 500)?;
  let n2_700 = neutral(2, 700)?;

  let light = BaseNeutrals {
    background: n1_10,
    foreground: n1_900.clone(),
    // Cards/popovers sit above the surface, so they go one step lighter.
    card: n1_0.clone(),
    card_foreground: n1_900.clone(),
    popover: n1_0,
    popover_foreground: n1_900.clone(),
    muted: n2_100.clone(),
    muted_foreground: n2_700.clone(),
    secondary: n2_100.clone(),
    secondary_foreground: n2_700.clone(),
    accent: n2_100,
    accent_foreground: n2_700.clone(),
    border: n2_200.clone(),
    input: n2_200.clone(),
    ring: n2_500,
  };

  let dark = BaseNeutrals {
    background: n1_900,
    foreground: n1_100.clone(),
    // In dark mode elevation reads as *lighter*, so cards go up a step.
    card: n1_800.clone(),
    card_foreground: n1_100.clone(),
    popover: n1_800,
    popover_foreground: n1_100,
    muted: n2_700.clone(),
    muted_foreground: n2_200.clone(),
    secondary: n2_700.clone(),
    secondary_foreground: n2_200.clone(),
    accent: n2_700.clone(),
    accent_foreground: n2_200,
    border: n2_700.clone(),
    input: n2_700,
    ring: n2_400,
  };

  let mut theme = NativeBaseTheme {
    light: Some(light),
    dark: Some(dark),
    // Filled in per request by `native_base_theme`, since it can change while
    // the app runs and the palette itself cannot.
    system_dark: None,
  };
  // The tonal palettes describe Material You, but not an OEM's deep/AMOLED dark
  // mode, which lives in the platform theme.
  if let Some(context) = &context {
    adopt_system_background(&mut env, context, &resources, &mut theme);
  }
  log(&format!("captured Material You palette:\n{}", theme.to_css()));
  Some(theme)
}

/// The running JVM: the one [`JNI_OnLoad`] recorded, falling back to asking the
/// JNI invocation API. The fallback covers the case where the linker dropped our
/// `JNI_OnLoad` from the final library; it may itself find nothing, since the
/// Android runtime's symbols aren't necessarily visible from an app's linker
/// namespace.
fn java_vm() -> Option<JavaVM> {
  let recorded = VM.load(Ordering::Acquire);
  if !recorded.is_null() {
    // SAFETY: the pointer came from the JVM itself via `JNI_OnLoad`.
    return unsafe { JavaVM::from_raw(recorded) }.ok();
  }

  // SAFETY: matches `jint JNI_GetCreatedJavaVMs(JavaVM**, jsize, jsize*)`.
  let Some(get_vms) = (unsafe { dynsym::symbol::<FnGetCreatedJavaVMs>("JNI_GetCreatedJavaVMs") }) else {
    log("no JavaVM: JNI_OnLoad did not run and JNI_GetCreatedJavaVMs is not visible");
    return None;
  };
  let mut vms: [*mut jni::sys::JavaVM; 1] = [std::ptr::null_mut()];
  let mut found: i32 = 0;
  // SAFETY: `vms` has room for the one VM we ask for, and `found` is writable.
  let status = unsafe { get_vms(vms.as_mut_ptr(), 1, &mut found) };
  if status != JNI_OK || found < 1 || vms[0].is_null() {
    log("no JavaVM: the runtime reported none");
    return None;
  }
  // SAFETY: the pointer came from the runtime's own VM list.
  unsafe { JavaVM::from_raw(vms[0]) }.ok()
}

/// The `Resources` to read the palette from: the application's (which carry the
/// dynamic-colour overlay) falling back to the framework's.
fn app_resources<'local>(env: &mut JNIEnv<'local>) -> Option<JObject<'local>> {
  if let Some(application) = app_context(env)
    && let Some(resources) = call_object(env, &application, "getResources", "()Landroid/content/res/Resources;")
  {
    return Some(resources);
  }
  // Overlay-free framework resources: always available, but may answer with the
  // stock ramp rather than the user's palette.
  log("no application context; falling back to the framework's own resources (may be the stock ramp)");
  call_static_object(
    env,
    "android/content/res/Resources",
    "getSystem",
    "()Landroid/content/res/Resources;",
  )
}

/// The process's `Application`, which is a `Context` carrying the dynamic-colour
/// resource overlay.
fn app_context<'local>(env: &mut JNIEnv<'local>) -> Option<JObject<'local>> {
  call_static_object(
    env,
    "android/app/ActivityThread",
    "currentApplication",
    "()Landroid/app/Application;",
  )
}

/// `Configuration.UI_MODE_NIGHT_MASK` / `UI_MODE_NIGHT_YES`.
const UI_MODE_NIGHT_MASK: i32 = 0x30;
const UI_MODE_NIGHT_YES: i32 = 0x20;

/// Whether the system is currently showing its dark scheme, read fresh.
///
/// The Activity declares `configChanges="uiMode"`, so a system theme change
/// neither recreates it nor reaches the WebView's `prefers-color-scheme`. This
/// is therefore the only trustworthy answer on Android, and it is read on demand
/// rather than cached so that asking again after the user has been away in the
/// system settings gives the new value.
pub(crate) fn system_prefers_dark() -> Option<bool> {
  let vm = java_vm()?;
  let mut env = vm.attach_current_thread().ok()?;
  let resources = app_resources(&mut env)?;
  system_prefers_dark_with(&mut env, &resources)
}

/// Whether the system is currently showing its dark scheme.
fn system_prefers_dark_with(env: &mut JNIEnv<'_>, resources: &JObject<'_>) -> Option<bool> {
  let configuration = call_object(
    env,
    resources,
    "getConfiguration",
    "()Landroid/content/res/Configuration;",
  )?;
  let ui_mode = env.get_field(&configuration, "uiMode", "I").ok()?.i().ok()?;
  clear_exception(env);
  Some(ui_mode & UI_MODE_NIGHT_MASK == UI_MODE_NIGHT_YES)
}

/// `TypedValue.TYPE_INT_COLOR_*`, the four types a resolved colour can come back
/// as.
const TYPE_INT_COLOR_RANGE: std::ops::RangeInclusive<i32> = 0x1c..=0x1f;

/// The background colour the **system's own** theme is using right now.
///
/// This is what answers "is the deep/AMOLED dark mode on?", for which Android
/// has no API: such a mode is an OEM overlay on the platform theme, so instead
/// of asking whether it is enabled we ask the system what its background
/// actually is. `Theme.DeviceDefault.DayNight` is the platform's own theme (the
/// app's is a MaterialComponents one, which carries Material's colours rather
/// than the device's), and being DayNight it resolves to whichever scheme the
/// system is in.
fn system_window_background(env: &mut JNIEnv<'_>, context: &JObject<'_>, resources: &JObject<'_>) -> Option<i32> {
  let style = identifier(env, resources, "Theme.DeviceDefault.DayNight", "style")?;
  let attribute = identifier(env, resources, "colorBackground", "attr")?;

  let themed = env
    .new_object(
      "android/view/ContextThemeWrapper",
      "(Landroid/content/Context;I)V",
      &[JValue::Object(context), JValue::Int(style)],
    )
    .ok()?;
  let theme = call_object(env, &themed, "getTheme", "()Landroid/content/res/Resources$Theme;")?;
  let value = env.new_object("android/util/TypedValue", "()V", &[]).ok()?;

  let resolved = env
    .call_method(
      &theme,
      "resolveAttribute",
      "(ILandroid/util/TypedValue;Z)Z",
      &[JValue::Int(attribute), JValue::Object(&value), JValue::Bool(1)],
    )
    .ok()?
    .z()
    .ok()?;
  if !resolved {
    return None;
  }
  // A colour attribute resolves to a literal; anything else (a reference to a
  // state list, say) isn't a plain background we can use.
  let kind = env.get_field(&value, "type", "I").ok()?.i().ok()?;
  if !TYPE_INT_COLOR_RANGE.contains(&kind) {
    return None;
  }
  env.get_field(&value, "data", "I").ok()?.i().ok()
}

/// Adopts the system's own background for the scheme the system is currently
/// in, when it is more extreme than the one derived from the tonal palettes.
///
/// Normally the two agree and nothing changes, so the Material You tint is kept.
/// When the device is in a deep/AMOLED dark mode its background is *darker* than
/// the palette's `neutral1` step — that difference is the signal, and adopting
/// it is what makes the app go black alongside the rest of the system. Only the
/// page background is taken: cards stay on the tonal ramp, which is exactly how
/// such modes look natively.
fn adopt_system_background(
  env: &mut JNIEnv<'_>,
  context: &JObject<'_>,
  resources: &JObject<'_>,
  theme: &mut NativeBaseTheme,
) {
  let (Some(night), Some(argb)) = (
    system_prefers_dark_with(env, resources),
    system_window_background(env, context, resources),
  ) else {
    clear_exception(env);
    log("could not read the system's own background; keeping the tonal-palette one");
    return;
  };
  clear_exception(env);

  let system = rgb_hex(argb);
  let system_luminance = luminance(argb);
  let scheme = if night {
    theme.dark.as_mut()
  } else {
    theme.light.as_mut()
  };
  let Some(scheme) = scheme else { return };

  let Some(current_luminance) = hex_luminance(&scheme.background) else {
    return;
  };
  // Darker than the ramp in dark mode, or lighter than it in light mode.
  let more_extreme = if night {
    system_luminance < current_luminance
  } else {
    system_luminance > current_luminance
  };
  if more_extreme {
    log(&format!(
      "system background {system} is beyond the tonal ramp's {} — adopting it (deep dark or equivalent)",
      scheme.background
    ));
    scheme.background = system;
  } else {
    log(&format!(
      "system background {system} matches the tonal ramp; keeping it"
    ));
  }
}

/// Resolves a framework resource id, or `None` when there is no such resource.
fn identifier(env: &mut JNIEnv<'_>, resources: &JObject<'_>, name: &str, kind: &str) -> Option<i32> {
  let j_name = env.new_string(name).ok()?;
  let j_kind = env.new_string(kind).ok()?;
  let j_package = env.new_string("android").ok()?;
  let id = env
    .call_method(
      resources,
      "getIdentifier",
      "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
      &[
        JValue::Object(&j_name),
        JValue::Object(&j_kind),
        JValue::Object(&j_package),
      ],
    )
    .ok()?
    .i()
    .ok()?;
  (id != 0).then_some(id)
}

/// Perceptual luminance of an ARGB colour int, in `0.0..=1.0`.
fn luminance(argb: i32) -> f64 {
  let c = argb as u32;
  let channel = |shift: u32| f64::from((c >> shift) & 0xff) / 255.0;
  0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
}

/// Perceptual luminance of a `#rrggbb` string, for comparing against a colour we
/// already formatted.
fn hex_luminance(hex: &str) -> Option<f64> {
  let value = i32::from_str_radix(hex.strip_prefix('#')?, 16).ok()?;
  Some(luminance(value))
}

/// Resolves `android.R.color.system_neutral{palette}_{shade}` to `#rrggbb`.
fn color_hex(env: &mut JNIEnv<'_>, resources: &JObject<'_>, palette: u8, shade: u16) -> Option<String> {
  let name = format!("system_neutral{palette}_{shade}");
  let result = lookup(env, resources, &name);
  clear_exception(env);
  result
}

fn lookup(env: &mut JNIEnv<'_>, resources: &JObject<'_>, name: &str) -> Option<String> {
  // A missing id means the system predates dynamic colour.
  let id = identifier(env, resources, name, "color")?;

  // `getColor(int, Resources.Theme)`; a null theme means "no theme attributes",
  // which is right for a plain colour resource.
  let argb = env
    .call_method(
      resources,
      "getColor",
      "(ILandroid/content/res/Resources$Theme;)I",
      &[JValue::Int(id), JValue::Object(&JObject::null())],
    )
    .ok()?
    .i()
    .ok()?;
  Some(rgb_hex(argb))
}

/// Calls a no-argument static method returning an object, or `None` if it isn't
/// there or threw. Never leaves a Java exception pending.
fn call_static_object<'local>(
  env: &mut JNIEnv<'local>,
  class: &str,
  method: &str,
  signature: &str,
) -> Option<JObject<'local>> {
  let result = env
    .call_static_method(class, method, signature, &[])
    .ok()
    .and_then(|value| value.l().ok())
    .filter(|object| !object.is_null());
  clear_exception(env);
  result
}

/// Calls a no-argument instance method returning an object, with the same
/// guarantees as [`call_static_object`].
fn call_object<'local>(
  env: &mut JNIEnv<'local>,
  object: &JObject<'_>,
  method: &str,
  signature: &str,
) -> Option<JObject<'local>> {
  let result = env
    .call_method(object, method, signature, &[])
    .ok()
    .and_then(|value| value.l().ok())
    .filter(|object| !object.is_null());
  clear_exception(env);
  result
}

/// Drops a pending Java exception so it can't poison later JNI calls.
fn clear_exception(env: &mut JNIEnv<'_>) {
  if env.exception_check().unwrap_or(false) {
    let _ = env.exception_clear();
  }
}

/// Formats an Android ARGB colour int as CSS `#rrggbb`. The alpha byte is
/// dropped: these palette entries are always opaque, and the tokens they feed
/// (backgrounds, text, borders) want an opaque colour.
fn rgb_hex(argb: i32) -> String {
  let c = argb as u32;
  format!("#{:02x}{:02x}{:02x}", (c >> 16) & 0xff, (c >> 8) & 0xff, c & 0xff)
}
