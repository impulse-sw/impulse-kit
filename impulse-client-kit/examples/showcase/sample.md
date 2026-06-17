# Loaded from a file

This document was **fetched at runtime** from `/sample.md` by the
`<Markdown>` block using `MarkdownSource::url(..)`.

## How it works

1. The block issues an HTTP `GET` for the URL.
2. While the request is in flight, a spinner is shown.
3. On success the Markdown is parsed and rendered with the default styles.
4. On failure a themed error message is shown instead.

> Try editing `sample.md` in the showcase and rebuilding to see the change.

Back to the [showcase home](/).
