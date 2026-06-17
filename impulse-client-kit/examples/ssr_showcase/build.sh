#!/bin/sh

set -e
clear

BLUE='\033[34;3m'
GREEN='\033[32m'
RESET='\033[0m'

printf '[1/7] Action `%bLint%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%b{flags}cargo clippy --package {package} --target {target}{features}%b`...\n' "$GREEN" "$RESET"
cargo clippy --package ssr-showcase --target x86_64-unknown-linux-gnu

printf '[2/7] Action `%bFormat%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%bcargo fmt -- --config tab_spaces=2,max_width=120 $(find . -name \"*.rs\" -not -path \"*/target/*\")%b`...\n' "$GREEN" "$RESET"
cargo fmt -- --config tab_spaces=2,max_width=120 $(find . -name "*.rs" -not -path "*/target/*")

printf '[3/7] Action `%bPatch \`Cargo.toml\`%b`...\n' "$BLUE" "$RESET"
#!/usr/bin/env bash
#
# Auto-generated bash patch script by smart-patcher (convert_to_bash)
#
# Usage: ./patch.sh <target_root> [<patch_dir>]
#        (patch_dir defaults to current directory)
#
# FULL SUPPORT:
#   • FilePath::Just and FilePath::Re
#   • All AreaRule variants (Contains, NotContains, Before, After,
#     CursorAtBegin, CursorAtEnd, FindBySh) – fully chained exactly
#     as in Rust find_section()
#   • Replacer::FromTo / FromToVar / RegexTo / BySh (applied to patch area)
#   • Insert (at cursor position)
#   • DecodeBy::Sh / EncodeBy::Sh
#
# Lua / Rhai are NOT supported (warning printed at runtime).
# Requires: jq, grep, sed, cut, wc, tr (standard unix tools + jq)

set -euo pipefail

TARGET_ROOT="${1:-.}"
PATCH_DIR="${2:-.}"

# ----------------------------------------------------------------------
# Regex position helpers (unix tools only: grep + cut + wc)
# ----------------------------------------------------------------------
function find_match_start {
  local suffix="$1"
  local re="$2"
  printf '%s' "$suffix" | grep -o -E -b -m1 -- "$re" | cut -d: -f1 2>/dev/null || echo -1
}

function find_match_end {
  local suffix="$1"
  local re="$2"
  local start=$(find_match_start "$suffix" "$re")
  if [[ "$start" == "-1" ]]; then
    echo -1
    return
  fi
  local matched=$(printf '%s' "$suffix" | grep -o -E -m1 -- "$re" || echo "")
  local len=$(printf '%s' "$matched" | wc -c)
  echo $((start + len))
}

# ----------------------------------------------------------------------
# Shell-script protocol helpers (identical to Rust sh_exec)
# ----------------------------------------------------------------------
function _uuid() {
  uuidgen 2>/dev/null || echo "tmp_$(date +%s)_$$_$RANDOM"
}

function call_find_by_sh() {
  local sh_script="$1"
  local content="$2"
  local start_pos="${3:-null}"
  local end_pos="${4:-null}"
  local req_file="$PATCH_DIR/find_by.req.$(_uuid).json"
  local res_file="$PATCH_DIR/find_by.res.$(_uuid).json"

  cat > "$req_file" << 'JSON_START'
{
  "content": 
JSON_START
  printf '%s' "$content" | jq -Rs . >> "$req_file"
  cat >> "$req_file" << 'JSON_MIDDLE'
,
  "start_pos": 
JSON_MIDDLE
  [[ "$start_pos" == "null" ]] && echo -n "null" >> "$req_file" || echo -n "$start_pos" >> "$req_file"
  cat >> "$req_file" << 'JSON_END'
,
  "end_pos": 
JSON_END
  [[ "$end_pos" == "null" ]] && echo -n "null" >> "$req_file" || echo -n "$end_pos" >> "$req_file"
  echo '}' >> "$req_file"

  # Redirect script stdout to stderr so it stays visible to the user but does not
  # contaminate this function's stdout (which is consumed by `read` in the caller).
  (cd "$PATCH_DIR" && "./$sh_script" "$(basename "$req_file")" "$(basename "$res_file")" 1>&2) || true

  if [[ ! -f "$res_file" ]]; then
    echo "ERROR: find_by_sh did not produce a response file" >&2
    return 1
  fi

  local start=$(jq -r '.start_pos // "null"' "$res_file")
  local end=$(jq -r '.end_pos // "null"' "$res_file")
  local cursor_at_end=$(jq -r '.cursor_at_end' "$res_file")

  rm -f "$req_file" "$res_file" 2>/dev/null
  echo "$start" "$end" "$cursor_at_end"
}

function call_replace_by_sh() {
  local sh_script="$1"
  local content="$2"
  local req_file="$PATCH_DIR/replace.req.$(_uuid).json"
  local res_file="$PATCH_DIR/replace.res.$(_uuid).json"

  jq -n --arg content "$content" '{content: $content}' > "$req_file"

  # Same stdout isolation as call_find_by_sh (the caller uses `$(call_replace_by_sh ...)`).
  (cd "$PATCH_DIR" && "./$sh_script" "$(basename "$req_file")" "$(basename "$res_file")" 1>&2) || true

  if [[ ! -f "$res_file" ]]; then
    echo "ERROR: replace_by_sh did not produce a response file" >&2
    return 1
  fi

  local new_content=$(jq -r '.content' "$res_file")
  rm -f "$req_file" "$res_file" 2>/dev/null
  echo "$new_content"
}

function call_decode_by_sh() {
  local sh_script="$1"
  local encoded_file="$2"
  local req_file="$PATCH_DIR/decode.req.$(_uuid).json"
  local res_file="$PATCH_DIR/decode.res.$(_uuid).json"

  jq -n --arg encoded_file "$encoded_file" '{encoded_file: $encoded_file}' > "$req_file"

  (cd "$PATCH_DIR" && "./$sh_script" "$(basename "$req_file")" "$(basename "$res_file")" 1>&2) || true

  if [[ ! -f "$res_file" ]]; then
    echo "ERROR: decode_by_sh did not produce a response file" >&2
    return 1
  fi

  local content=$(jq -r '.content' "$res_file")
  rm -f "$req_file" "$res_file" 2>/dev/null
  echo "$content"
}

function call_encode_by_sh() {
  local sh_script="$1"
  local content="$2"
  local write_to_file="$3"
  local req_file="$PATCH_DIR/encode.req.$(_uuid).json"

  jq -n --arg write_to_file "$write_to_file" --arg content "$content" \
    '{write_to_file: $write_to_file, content: $content}' > "$req_file"

  (cd "$PATCH_DIR" && "./$sh_script" "$(basename "$req_file")" 1>&2) || true

  rm -f "$req_file" 2>/dev/null
}

# ----------------------------------------------------------------------
# Main patching loop
# ----------------------------------------------------------------------
find "$TARGET_ROOT" -type f | while read -r filepath; do
  # Patch #0 (index 0)
  match=false
  [[ "$filepath" == *'Cargo.toml' ]] && match=true
  if [[ "$match" == true ]]; then
    content=$(cat "$filepath" 2>/dev/null | tr -d '\0' || echo '')
    patch_ok=true
    current_pos=0
    start_pos=""
    end_pos=""
    cursor="Start"

    if [[ "$patch_ok" == true ]]; then
      # compute range exactly as in Rust find_section
      range_start=0
      range_end=${#content}

      if [[ -n "$start_pos" && -n "$end_pos" ]]; then
        # Safe numeric comparison (prevents "integer expression expected" / "не заданы границы переменной")
        if (( ${start_pos:-0} <= ${end_pos:-0} )); then
          range_start="$start_pos"
          range_end="$end_pos"
        fi
      elif [[ -n "$start_pos" ]]; then
        range_start="$start_pos"
      elif [[ -n "$end_pos" ]]; then
        range_end="$end_pos"
      fi

      middle="${content:$range_start:$((range_end - range_start))}"

      # replace (only if middle non-empty)
      if [[ -n "$middle" ]]; then
        middle="${middle//'cargo-features = ["panic-immediate-abort"]'/''}"
      fi
      before="${content:0:$range_start}"
      after="${content:$range_end}"
      new_content="${before}${middle}${after}"

      # encoder or direct write
      printf '%s' "$new_content" > "$filepath"
      echo "    Applied patch #0 to $filepath"
    fi
  fi

  # Patch #1 (index 1)
  match=false
  [[ "$filepath" == *'Cargo.toml' ]] && match=true
  if [[ "$match" == true ]]; then
    content=$(cat "$filepath" 2>/dev/null | tr -d '\0' || echo '')
    patch_ok=true
    current_pos=0
    start_pos=""
    end_pos=""
    cursor="Start"

    if [[ "$patch_ok" == true ]]; then
      # compute range exactly as in Rust find_section
      range_start=0
      range_end=${#content}

      if [[ -n "$start_pos" && -n "$end_pos" ]]; then
        # Safe numeric comparison (prevents "integer expression expected" / "не заданы границы переменной")
        if (( ${start_pos:-0} <= ${end_pos:-0} )); then
          range_start="$start_pos"
          range_end="$end_pos"
        fi
      elif [[ -n "$start_pos" ]]; then
        range_start="$start_pos"
      elif [[ -n "$end_pos" ]]; then
        range_end="$end_pos"
      fi

      middle="${content:$range_start:$((range_end - range_start))}"

      # replace (only if middle non-empty)
      if [[ -n "$middle" ]]; then
        middle="${middle//'immediate-abort'/'abort'}"
      fi
      before="${content:0:$range_start}"
      after="${content:$range_end}"
      new_content="${before}${middle}${after}"

      # encoder or direct write
      printf '%s' "$new_content" > "$filepath"
      echo "    Applied patch #1 to $filepath"
    fi
  fi

done

echo "
=== bash patch script finished ===
"


printf '[4/7] Action `%bBuild (release mode)%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%b{flags}cargo build --release --package {package} --target {target}{features}%b`...\n' "$GREEN" "$RESET"
cargo build --release --package ssr-showcase --target x86_64-unknown-linux-gnu

printf '[5/7] Action `%bCompress%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%bupx %%af%%%b`...\n' "$GREEN" "$RESET"
upx target/x86_64-unknown-linux-gnu/release/ssr-showcase

printf '[6/7] Action `%bMake \`dist/pkg\` folder%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%bmkdir -p {folder}%b`...\n' "$GREEN" "$RESET"
mkdir -p ./impulse-client-kit/examples/ssr_showcase/dist/pkg

printf '[7/7] Action `%bCompile TailwindCSS%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%btailwindcss -i {tw-input} -o {tw-output} --minify%b`...\n' "$GREEN" "$RESET"
tailwindcss -i ./impulse-client-kit/examples/ssr_showcase/input.css -o ./impulse-client-kit/examples/ssr_showcase/dist/pkg/ssr_showcase.css --minify

mkdir -p artifacts
cp -rf target/x86_64-unknown-linux-gnu/release/ssr-showcase artifacts/ssr-showcase/ssr-showcase || true
cp -rf impulse-client-kit/examples/ssr_showcase/server-example.yaml artifacts/ssr-showcase/server-example.yaml || true
cp -rf impulse-client-kit/examples/ssr_showcase/dist artifacts/ssr-showcase/dist || true
