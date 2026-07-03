# impulse-static-server

Simple static server. Features:

- running on top of `impulse-server-kit` (see [configuration example](./../impulse-server-kit/README.md))
- serves all your files from `dist` or `/usr/local/frontend-dist/` folder
- when receives any request other than `/`, it returns `index.html`, excluding files
- provides in-memory cache for files less than 16 MiB via `StaticRouter::new_with_cacher` or by default - with `ETag` and `Last-Modified`

> [!NOTE]
> Since v1.1.1 the routing logic lives in `impulse-server-kit::static_server`,
> behind the `static-server` feature. This crate is a thin re-export and the
> `iks` binary entry point. The same handlers power asset serving for the
> Leptos SSR adapter (`leptos-ssr` feature), so SSR setups get the same
> logging, caching and `ETag` / `Last-Modified` behaviour.

## Build

This project is supporting Deployer. You can build server with:

```bash
depl run build-static-server
```

Or, alternatively, just build with `cargo`:

```bash
cargo build --release
```

## Usage

1. Place your files inside `dist` folder.
2. Place the `iks` executable (the binary this crate builds) near `dist` folder.
3. Create a `static-server.yaml` next to it with Server Kit parameters (protocols, logging, …; see the [Server Kit configuration overview](./../impulse-server-kit/README.md#7)). The config is also looked up at `/etc/static-server.yaml`.
4. Start `./iks`.

> [!NOTE]
> There is no need to specify working dir, static server must work with distribution files placed nearly.

## Usage as a library

Just include it into your `Cargo.toml`:

```toml
[dependencies]
impulse-static-server = { git = "https://github.com/impulse-sw/impulse-kit.git", tag = "1.4.10" }
```

and use one of these functions:

- `frontend_router` - same behavior as the binary Static Server
- `frontend_router_from_given_dist` - you can specify any dist folder you want
- `assets_only_router_from` - serves files only, without the SPA `index.html` fallback

For finer control there are the router types themselves (all re-exported from
`impulse-server-kit`'s `static_server` module):

- `StaticRouter::new(dist)` / `StaticRouter::new_with_cacher(dist)` - the base
  SPA router, optionally with the in-memory cache (files < 16 MiB, `ETag` +
  `Last-Modified`, plus a filesystem watcher that invalidates entries on
  change)
- `.with_no_redirect()` → `NoRedirectStaticRouter` - fall through (404) for
  missing files instead of returning `index.html`
- `.with_routes_list(routes)` → `ProvidedRoutesStaticRouter` - serve only the
  files whose names are in the given list
