#!/bin/sh

set -e
clear

BLUE='\033[34;3m'
GREEN='\033[32m'
RESET='\033[0m'

printf '[1/9] Action `%bLint \`impulse-utils\` on x86-64%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%b{flags}cargo clippy --package {package} --target {target}{features}%b`...\n' "$GREEN" "$RESET"
RUSTFLAGS='--cfg reqwest_unstable' cargo clippy --package impulse-utils --target x86_64-unknown-linux-gnu

printf '[2/9] Action `%bLint \`impulse-utils\` on wasm32%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%b{flags}cargo clippy --package {package} --target {target}{features}%b`...\n' "$GREEN" "$RESET"
RUSTFLAGS='--cfg reqwest_unstable' cargo clippy --package impulse-utils --target wasm32-unknown-unknown --no-default-features --features=reqwest

printf '[3/9] Action `%bLint \`impulse-ui-kit\`%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%b{flags}cargo clippy --package {package} --target {target}{features}%b`...\n' "$GREEN" "$RESET"
cargo clippy --package impulse-ui-kit --target wasm32-unknown-unknown

printf '[4/9] Action `%bLint \`showcase\`%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%b{flags}cargo clippy --package {package} --target {target}{features}%b`...\n' "$GREEN" "$RESET"
cargo clippy --package ui-kit-showcase --target wasm32-unknown-unknown

printf '[5/9] Action `%bLint \`impulse-error-pages\`%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%b{flags}cargo clippy --package {package} --target {target}{features}%b`...\n' "$GREEN" "$RESET"
cargo clippy --package impulse-error-pages --target wasm32-unknown-unknown

printf '[6/9] Action `%bLint \`impulse-server-kit\`%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%b{flags}cargo clippy --package {package} --target {target}{features}%b`...\n' "$GREEN" "$RESET"
RUSTFLAGS='--cfg reqwest_unstable' cargo clippy --package impulse-server-kit --target x86_64-unknown-linux-gnu

printf '[7/9] Action `%bLint \`impulse-server-kit-dsl\`%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%b{flags}cargo clippy --package {package} --target {target}{features}%b`...\n' "$GREEN" "$RESET"
cargo clippy --package impulse-skdsl --target x86_64-unknown-linux-gnu

printf '[8/9] Action `%bLint \`impulse-static-server\`%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%b{flags}cargo clippy --package {package} --target {target}{features}%b`...\n' "$GREEN" "$RESET"
cargo clippy --package impulse-static-server --target x86_64-unknown-linux-gnu

printf '[9/9] Action `%bTest docs%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%bcargo test --doc%b`...\n' "$GREEN" "$RESET"
cargo test --doc

