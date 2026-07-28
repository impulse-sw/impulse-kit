//! Material You (MD3) dynamic-colour provider for Android.
//!
//! Android 12 (API 31) and later publish the user's wallpaper-derived tonal
//! palettes as framework colour resources: `android.R.color.system_neutral1_*`
//! and `system_neutral2_*` (plus accent ramps this crate deliberately ignores —
//! the app keeps its own brand accent). We read them through JNI using the
//! `Activity` and `JavaVM` Tauri's Android runtime publishes via `ndk-context`.
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
//! On API < 31 the resources don't exist, `getIdentifier` answers `0`, and we
//! return `None` so the app keeps its own palette.

use jni::JNIEnv;
use jni::objects::{JObject, JValue};

use crate::{BaseNeutrals, NativeBaseTheme};

/// Reads the Material You tonal palettes and derives both colour schemes.
pub(crate) fn capture() -> Option<NativeBaseTheme> {
  let ctx = ndk_context::android_context();
  // SAFETY: `ndk-context` hands out the process-wide JavaVM and Activity that
  // Tauri's Android runtime registered at startup.
  let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
  let context = unsafe { JObject::from_raw(ctx.context().cast()) };
  let mut env = vm.attach_current_thread().ok()?;

  let resources = env
    .call_method(&context, "getResources", "()Landroid/content/res/Resources;", &[])
    .ok()?
    .l()
    .ok()?;

  let mut neutral = |palette: u8, shade: u16| color_hex(&mut env, &context, &resources, palette, shade);

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

  Some(NativeBaseTheme {
    light: Some(light),
    dark: Some(dark),
  })
}

/// Resolves `android.R.color.system_neutral{palette}_{shade}` to `#rrggbb`.
///
/// Returns `None` when the resource doesn't exist (API < 31) or any JNI call
/// fails, clearing a pending Java exception so later calls aren't poisoned.
fn color_hex(
  env: &mut JNIEnv<'_>,
  context: &JObject<'_>,
  resources: &JObject<'_>,
  palette: u8,
  shade: u16,
) -> Option<String> {
  let name = format!("system_neutral{palette}_{shade}");
  let result = lookup(env, context, resources, &name);
  if result.is_none() && env.exception_check().unwrap_or(false) {
    let _ = env.exception_clear();
  }
  result
}

fn lookup(env: &mut JNIEnv<'_>, context: &JObject<'_>, resources: &JObject<'_>, name: &str) -> Option<String> {
  let j_name = env.new_string(name).ok()?;
  let j_type = env.new_string("color").ok()?;
  let j_package = env.new_string("android").ok()?;

  let id = env
    .call_method(
      resources,
      "getIdentifier",
      "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
      &[
        JValue::Object(&j_name),
        JValue::Object(&j_type),
        JValue::Object(&j_package),
      ],
    )
    .ok()?
    .i()
    .ok()?;
  // 0 means "no such resource" — the system predates dynamic colour.
  if id == 0 {
    return None;
  }

  let argb = env
    .call_method(context, "getColor", "(I)I", &[JValue::Int(id)])
    .ok()?
    .i()
    .ok()?;
  Some(rgb_hex(argb))
}

/// Formats an Android ARGB colour int as CSS `#rrggbb`. The alpha byte is
/// dropped: these palette entries are always opaque, and the tokens they feed
/// (backgrounds, text, borders) want an opaque colour.
fn rgb_hex(argb: i32) -> String {
  let c = argb as u32;
  format!("#{:02x}{:02x}{:02x}", (c >> 16) & 0xff, (c >> 8) & 0xff, c & 0xff)
}
