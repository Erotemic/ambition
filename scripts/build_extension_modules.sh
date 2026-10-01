#!/usr/bin/env bash
# Build the game's procedural modules as `.wasm` files (fast-iteration I6/I7).
#
# This compiles ONLY the module crate and the pure port values it uses; the
# engine is not compiled or linked. Run the game with the result:
#
#   scripts/build_extension_modules.sh
#   AMBITION_EXTENSION_MODULES=target/extension-modules/wasm32-unknown-unknown/release \
#       cargo run -p ambition_app
#
# The loaded modules explicitly replace the linked ones of the same key; the
# log says which.
set -euo pipefail
cd "$(dirname "$0")/.."
scripts/setup/target_bindmount.sh --check >/dev/null
package="${1:-ambition_content_modules}"
cargo rustc --quiet -p "$package" --target wasm32-unknown-unknown --release \
    --crate-type cdylib --target-dir target/extension-modules
out="target/extension-modules/wasm32-unknown-unknown/release/${package}.wasm"
echo "built $out ($(wc -c < "$out") bytes)"
