#!/bin/sh

set -e
clear

BLUE='\033[34;3m'
GREEN='\033[32m'
RESET='\033[0m'

printf '[1/8] Action `%bLint%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%b{flags}cargo clippy --package {package} --target {target}{features}%b`...\n' "$GREEN" "$RESET"
cargo clippy --package client-kit-showcase --target wasm32-unknown-unknown

printf '[2/8] Action `%bFormat #1%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%bleptosfmt -t 2 ./**/*.rs%b`...\n' "$GREEN" "$RESET"
leptosfmt -t 2 ./**/*.rs

printf '[3/8] Action `%bFormat #2%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%bcargo fmt -- --config tab_spaces=2,max_width=120 $(find . -name \"*.rs\" -not -path \"*/target/*\")%b`...\n' "$GREEN" "$RESET"
cargo fmt -- --config tab_spaces=2,max_width=120 $(find . -name "*.rs" -not -path "*/target/*")

printf '[4/8] Action `%bBuild WASM%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%bcd {project-folder} && RUSTFLAGS=\"-Zlocation-detail=none -Zfmt-debug=none -Zunstable-options -Cpanic=immediate-abort\" CARGO_UNSTABLE_BUILD_STD=\"std,panic_abort\" CARGO_UNSTABLE_BUILD_STD_FEATURES=\"optimize_for_size\" trunk build --release%b`...\n' "$GREEN" "$RESET"
cd impulse-client-kit/examples/showcase/ && RUSTFLAGS="-Zlocation-detail=none -Zfmt-debug=none -Zunstable-options -Cpanic=immediate-abort" CARGO_UNSTABLE_BUILD_STD="std,panic_abort" CARGO_UNSTABLE_BUILD_STD_FEATURES="optimize_for_size" trunk build --release

printf '[5/8] Action `%bStrip WASM%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%bwasm-strip {file}%b`...\n' "$GREEN" "$RESET"
wasm-strip ./impulse-client-kit/examples/showcase/dist/client-kit-showcase_bg.wasm

printf '[6/8] Action `%bOptimize WASM%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%bwasm-opt -Oz --all-features --strip-debug --strip-producers -o {file} {file}%b`...\n' "$GREEN" "$RESET"
wasm-opt -Oz --all-features --strip-debug --strip-producers -o ./impulse-client-kit/examples/showcase/dist/client-kit-showcase_bg.wasm ./impulse-client-kit/examples/showcase/dist/client-kit-showcase_bg.wasm

printf '[7/8] Action `%bCompile TailwindCSS%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%btailwindcss -i {tw-input} -o {tw-output} --minify%b`...\n' "$GREEN" "$RESET"
tailwindcss -i ./impulse-client-kit/examples/showcase/input.css -o ./impulse-client-kit/examples/showcase/public/tailwind.css --minify

printf '[8/8] Action `%bAdd files to \`dist\`%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%bcp {tw-output} {favicon} impulse-client-kit/examples/showcase/dist/%b`...\n' "$GREEN" "$RESET"
cp ./impulse-client-kit/examples/showcase/public/tailwind.css ./impulse-client-kit/examples/showcase/public/favicon.ico impulse-client-kit/examples/showcase/dist/

mkdir -p artifacts
cp -rf impulse-client-kit/examples/showcase/dist artifacts/showcase/dist || true
