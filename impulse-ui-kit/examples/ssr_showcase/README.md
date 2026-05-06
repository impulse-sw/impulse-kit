# SSR Showcase

Minimal server-rendered example built on top of `impulse-server-kit` (with the
`leptos-ssr` feature) and `impulse-ui-kit` (with the `ssr` feature).

This iteration delivers SEO-grade HTML rendering only — no client hydration,
no `<Suspense>` streaming, no `#[server]` functions. Those features are
reserved for the next iteration.

## Run

```sh
cd impulse-ui-kit/examples/ssr_showcase
mkdir -p dist/pkg                # placeholder; SSR works without a real bundle
cargo +nightly run --release
```

Then `curl -i http://127.0.0.1:8802/` and inspect the HTML — the `<head>`
contains `<title>`, `<meta name="description">`, `<link rel="canonical">`,
OpenGraph and Twitter Cards, and the `<html lang>` attribute.

## Configuration

All settings live in `server-example.yaml`. The relevant fields:

- `frontend_dist_path` — directory containing the built front-end assets.
- `leptos_output_name` — bundle name; controls the URL of the JS/CSS bundle.
- `leptos_site_pkg_dir` — sub-directory of `frontend_dist_path` containing
  the wasm/JS/CSS bundle (defaults to `pkg`).
- `leptos_seo` — server-side SEO defaults injected into the rendered HTML.
