#!/usr/bin/env bash
set -euo pipefail

ENGINE="$(cd "$(dirname "$0")/.." && pwd)"

export PATH="$HOME/.cargo/bin:$PATH"

TARGET_DIR="target/wasm32-unknown-unknown/release"
OUT="dist"
CRATE="$(sed -n 's/^name = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
LIB="${CRATE//-/_}"
TARBALL="${CRATE//_/-}-web.tar.gz"

manifest_value() {
  sed -n "s/^$1 = \"\(.*\)\"\$/\1/p" manifest.toml | head -1
}

TITLE="$(manifest_value title)"
ICON="$(manifest_value icon)"
ICON="${ICON:-🎮}"

if ! command -v wasm-bindgen >/dev/null; then
  echo "wasm-bindgen not found. install with:" >&2
  echo "  cargo install wasm-bindgen-cli --version \$(grep -A1 '^name = \"wasm-bindgen\"\$' Cargo.lock | grep version | head -1 | cut -d'\"' -f2)" >&2
  exit 1
fi

rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true

echo "==> building wasm"
cargo build --release --lib --target wasm32-unknown-unknown

echo "==> running wasm-bindgen"
rm -rf "$OUT"
mkdir -p "$OUT"
wasm-bindgen \
  --target web \
  --no-typescript \
  --out-dir "$OUT" \
  "$TARGET_DIR/$LIB.wasm"

if command -v wasm-opt >/dev/null; then
  echo "==> optimizing with wasm-opt"
  wasm-opt -Oz -o "$OUT/${LIB}_bg.wasm" "$OUT/${LIB}_bg.wasm"
fi

# Content-hash the wasm and js so they can be cached forever; index.html stays
# unhashed and must be served no-cache, since it names the current hashes.
hash_of() {
  sha256sum "$1" | cut -c1-12
}

echo "==> fingerprinting"

WASM_HASH="$(hash_of "$OUT/${LIB}_bg.wasm")"
WASM_NAME="${LIB}_bg.${WASM_HASH}.wasm"
mv "$OUT/${LIB}_bg.wasm" "$OUT/$WASM_NAME"

if ! grep -q "${LIB}_bg.wasm" "$OUT/$LIB.js"; then
  echo "error: wasm reference not found in glue js; hashing would break the build" >&2
  exit 1
fi
sed -i "s/${LIB}_bg\.wasm/${WASM_NAME}/g" "$OUT/$LIB.js"

JS_HASH="$(hash_of "$OUT/$LIB.js")"
JS_NAME="${LIB}.${JS_HASH}.js"
mv "$OUT/$LIB.js" "$OUT/$JS_NAME"

echo "==> writing index.html"
template() {
  if [ -f "web/$1" ]; then cat "web/$1"; else cat "$ENGINE/web/$1"; fi
}

GAME='<div id="loading">loading…</div>
<canvas id="game" tabindex="0"></canvas>'
WRAPPER="$(template wrapper.html)"
WRAPPER="${WRAPPER//\{\{game\}\}/$GAME}"
STYLE="$(template style.css)"
PAGE="$(template index.html)"
PAGE="${PAGE//\{\{style\}\}/$STYLE}"
PAGE="${PAGE//\{\{wrapper\}\}/$WRAPPER}"
PAGE="${PAGE//\{\{js\}\}/$JS_NAME}"
PAGE="${PAGE//\{\{title\}\}/$TITLE}"
PAGE="${PAGE//\{\{icon\}\}/$ICON}"
printf '%s\n' "$PAGE" > "$OUT/index.html"

cp "$ENGINE/web/_headers" "$OUT/_headers"

echo "==> packaging $TARBALL"
rm -f "$TARBALL"
tar -czf "$TARBALL" -C "$OUT" .

echo
echo "dist/ ready:"
du -h "$OUT"/* | sed 's/^/  /'
echo "  $TARBALL ($(du -h "$TARBALL" | cut -f1))"
echo
echo "cache-control:"
echo "  index.html                  -> no-cache (must revalidate)"
echo "  *.js / *.wasm               -> immutable, max-age=31536000"
echo
echo "serve locally:  python3 -m http.server -d $OUT 8080"
