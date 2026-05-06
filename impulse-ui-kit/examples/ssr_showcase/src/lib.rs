//! Shared application component for the SSR showcase.

use impulse_ui_kit::prelude::*;
use leptos_meta::*;

/// Root component used for SSR.
#[component]
pub fn App() -> impl IntoView {
  provide_meta_context();
  view! {
    <Title text="UI Kit Showcase | Impulse"/>
    <Meta name="description" content="Server-rendered demo of Impulse UI Kit using Impulse Server Kit"/>
    <Link rel="canonical" href="http://127.0.0.1:8802/"/>
    <main class="min-h-screen flex flex-col items-center justify-center p-8 gap-4">
      <h1 class="text-4xl font-semibold">"Impulse UI Kit"</h1>
      <p class="text-lg opacity-80">"Server-side rendered demo. SEO tags are injected at the server."</p>
      <p class="text-sm opacity-60">"This page is fully crawlable. Hydration is reserved for the next iteration."</p>
    </main>
  }
}
