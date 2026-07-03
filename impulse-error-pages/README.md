# impulse-error-pages

Error pages for Impulse services.

Works with 400, 401, 403, 404, 405 & 500 status codes. Just redirect to `/{status-code}` or return `dist/index.html` instead of requested resource.

## Build

```bash
cd impulse-error-pages
trunk build --release
tailwindcss -i ./input.css -o ./dist/error_pages_tailwind.css --minify
cp public/favicon.ico dist/
```

(`index.html` links the stylesheet as `/error_pages_tailwind.css`, so the
compiled Tailwind output must land in `dist/` under that name.)

or:

```bash
# in repo root
depl run build-error-pages
```

The Deployer pipeline additionally runs `wasm-opt` on the bundle and collects
`impulse-error-pages/dist` as the artifact.
