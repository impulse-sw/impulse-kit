# Impulse Client Kit Showcase

Components gallery for Impulse Client Kit: one page per component family from
[`impulse-client-kit-components`](../../components/README.md) and
[`impulse-client-kit-blocks`](../../blocks/README.md). It is also the reference
consumer setup for the Tailwind wiring (`build.rs` +
`impulse-tailwind-sources` + `input.css`).

## Build

From the **repository root**, run:

```bash
./impulse-client-kit/examples/showcase/build.sh
```

(the script `cd`s into the example itself, so it must be started from the repo
root). It lints, formats, builds the wasm bundle with `trunk` in release mode,
strips and optimizes it with `wasm-strip` / `wasm-opt`, and compiles Tailwind.
The result lands in `impulse-client-kit/examples/showcase/dist`.

## Run

To run, you need to build [`impulse-static-server`](./../../../impulse-static-server/README.md)
first — it produces the `iks` binary; place it next to the generated `dist`
folder and start it. Also, you can use any static server instead.
