# SSR Showcase

Server-rendered + hydrated example built on top of `impulse-server-kit`
(feature `leptos-server-fn`) and `impulse-client-kit` (features `ssr` for the
server, `hydrate` for the wasm bundle).

It demonstrates:

- streaming SSR with `<Suspense>` boundaries (server functions are awaited
  during render and their results are inlined into the HTML),
- `#[server]` functions dispatched through `server_fn::axum::handle_server_fn`
  bridged to Salvo,
- client-side hydration via the `hydrate` entrypoint exported from the wasm
  bundle.

## Build

The project ships two artefacts:

- a host-target binary that runs the SSR + server-functions HTTP server,
- a `wasm32-unknown-unknown` cdylib that hydrates the page on the client.

A minimal manual build:

```sh
cd impulse-client-kit/examples/ssr_showcase

# 1) wasm bundle (hydration)
cargo +nightly build --lib --release \
  --target wasm32-unknown-unknown --features hydrate --no-default-features

# 2) wasm-bindgen — emits dist/pkg/{ssr_showcase.js, ssr_showcase_bg.wasm}
mkdir -p dist/pkg
wasm-bindgen --target web \
  --out-dir dist/pkg --out-name ssr_showcase \
  ../../../target/wasm32-unknown-unknown/release/ssr_showcase.wasm

# 3) tailwind — emits dist/pkg/ssr_showcase.css
tailwindcss -i input.css -o dist/pkg/ssr_showcase.css --minify

# 4) host server
cargo +nightly build --release \
  --bin ssr-showcase --features ssr --no-default-features \
  --target x86_64-unknown-linux-gnu

# 5) run
../../../target/x86_64-unknown-linux-gnu/release/ssr-showcase
```

The `.depl/config.yaml` `deploy-ssr-showcase` pipeline does the same in CI.

## Verify

```sh
curl -i http://127.0.0.1:8802/
```

Expected:

- `Content-Type: text/html; charset=utf-8`, `Transfer-Encoding: chunked`,
- a fully populated `<head>` (title, description, canonical, OpenGraph,
  Twitter Cards),
- the rendered body with both `<Suspense>` server-function results inlined,
- two `<script>__RESOLVED_RESOURCES[…]…</script>` blocks carrying the
  serialized resource data for hydration,
- a final `<script type="module">import init,{hydrate} from "/pkg/ssr_showcase.js"; init(...).then(()=>hydrate());</script>`.

Open the page in a browser and the wasm bundle takes over from there:
existing DOM is hydrated in place (no flicker), reactive signals become
interactive.

## Configuration

`server-example.yaml` controls runtime configuration via Server Kit's
`GenericValues`:

- `frontend_dist_path` — directory containing the front-end build artefacts
  (the `dist/` folder). Falls back to `IMPULSE_FRONTEND_DIST` env var,
  then `./dist`, then `/usr/local/frontend-dist` when not set. All files
  under this directory are served with logging and in-memory caching;
  unknown paths fall through to the SSR renderer.
- `leptos_output_name` — bundle name; controls `/pkg/<name>.{js,wasm,css}`.
- `leptos_server_fn_prefix` — URL prefix where `#[server]` functions are
  mounted (defaults to `/api/leptos`).
- `leptos_seo` — SEO defaults injected into the rendered HTML. The full
  field list and which `<head>` tag each one emits is documented in
  [`impulse-server-kit`'s README](../../../impulse-server-kit/README.md#8).
  The short version:
  - `default_title` + `title_template` — `<title>` fallback and the format
    used to combine page-specific titles with the brand.
  - `description` — emits both `<meta name="description">` and
    `<meta property="og:description">`.
  - `canonical_base` — appended with the request path to form
    `<link rel="canonical">` and `<meta property="og:url">`.
  - `og_image` / `og_logo` — social preview and brand mark (both absolute
    URLs).
  - `site_name` — `<meta property="og:site_name">`; falls back to
    `default_title` when unset.
  - `locale` — drives `<html lang>` and `<meta property="og:locale">`
    (the latter normalised to `lang_REGION`, e.g. `en` → `en_US`).
  - `robots` — `<meta name="robots">` (e.g. `noindex,nofollow` for staging).
  - `twitter_handle` — `<meta name="twitter:site">`.

> [!IMPORTANT]
> Don't also declare `description` / `canonical` / OG / Twitter tags inside
> the Leptos `App` view — the SSR prefix already emits them from
> `leptos_seo`, and a second copy from `leptos_meta` would create two
> identical tags in `<head>`. Use `leptos_meta`'s `<Title>`, `<Meta>` and
> `<Link>` only for per-page overrides.
