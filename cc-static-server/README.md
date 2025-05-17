# cc-static-server

Simple static server. Features:

- running on top of `cc-server-kit` (see [configuration example](./../cc-server-kit/README.md))
- serves all your files from `dist` or `/usr/local/frontend-dist/` folder
- when receives any request other than `/`, it returns `index.html`, excluding files

## Build

This project is supporting [Deployer](https://github.com/impulse-sw/deployer). You can build server with:

```bash
deployer build
```

Or, alternatively, just build with `cargo`:

```bash
cargo build --release
```

## Usage

1. Place your files inside `dist` folder.
2. Place `cc-static-server` executable near `dist` folder.
3. Start `./cc-static-server`.

> [!NOTE]
> There is no need to specify working dir, static server must work with distribution files placed nearly.
