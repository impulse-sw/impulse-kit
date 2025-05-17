# cc-error-pages

Error pages for CC Services.

Works with 400, 401, 403, 404, 405 & 500 status codes. Just redirect to `/{status-code}` or return `dist/index.html` instead of requested resource.

## Build

```bash
cd tailwindcss -i ./input.css -o ./public/tailwind.css --minify
trunk build --release
cp public/tailwind.css dist/error_pages_tailwind.css
cp public/favicon.ico dist/
```

or:

```bash
# in repo root
deployer run build-error-pages
```
