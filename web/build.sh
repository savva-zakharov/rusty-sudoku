#!/usr/bin/env bash
# Build the web version into dist/:
#   dist/index.html, gl.js, audio.js, net.js, rusty-sudoku.wasm,
#   dist/assets/palettes.txt, dist/assets/palettes/*.png, dist/assets/sounds/*.wav
# Serve dist/ with any static file server, e.g.:
#   npx --yes serve@14 dist --listen tcp://0.0.0.0:8080
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build --release --target wasm32-unknown-unknown

rm -rf dist
mkdir -p dist/assets/palettes dist/assets/sounds
cp web/index.html web/gl.js web/audio.js web/net.js dist/
cp target/wasm32-unknown-unknown/release/rusty-sudoku.wasm dist/
cp assets/palettes.txt dist/assets/
sed -e 's/#.*//' -e 's/[[:space:]]//g' assets/palettes.txt | grep -E '^[A-Za-z0-9_-]+$' | while read -r pal; do
    if [ -f "assets/palettes/$pal.png" ]; then cp "assets/palettes/$pal.png" dist/assets/palettes/; fi
done
cp assets/sounds/*.wav dist/assets/sounds/

echo "built dist/ ($(du -sh dist | cut -f1))"
