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
#
# With --watch it stays running and rebuilds each time a source the modules
# are built from changes (the module crate, the port value crates, the SDK).
# A running game reloads the new file by itself: save the file, see the
# change in the game. A build that fails prints its errors and keeps watching;
# the game keeps the last good build.
#
#   scripts/build_extension_modules.sh --watch
set -euo pipefail
cd "$(dirname "$0")/.."
scripts/setup/target_bindmount.sh --check >/dev/null
watch=0
if [ "${1:-}" = "--watch" ]; then
    watch=1
    shift
fi
package="${1:-ambition_content_modules}"
if ! [ -d "$(rustc --print sysroot)/lib/rustlib/wasm32-unknown-unknown/lib" ]; then
    echo "the rust target wasm32-unknown-unknown is not installed:" >&2
    echo "    rustup target add wasm32-unknown-unknown" >&2
    exit 1
fi
out="target/extension-modules/wasm32-unknown-unknown/release/${package}.wasm"
build() {
    local start=$(date +%s.%N)
    if cargo rustc --quiet -p "$package" --target wasm32-unknown-unknown --release \
        --crate-type cdylib --target-dir target/extension-modules; then
        printf 'built %s (%s bytes) in %s s\n' "$out" "$(wc -c < "$out")" \
            "$(awk "BEGIN { printf \"%.2f\", $(date +%s.%N) - $start }")"
        return 0
    fi
    return 1
}
if [ "$watch" = 0 ]; then
    build
    exit $?
fi

# The sources a module build reads. A change anywhere under them rebuilds.
sources=(game/ambition_content_modules crates/ambition_extension_sdk
    crates/ambition_boss_special_port crates/ambition_combat_port crates/ambition_projectile_spec)
stamp="$(mktemp)"
trap 'rm -f "$stamp"' EXIT
build || echo "build failed; watching for the next change"
touch "$stamp"
echo "watching ${sources[*]} (Ctrl-C to stop)"
while true; do
    sleep 0.5
    if [ -n "$(find "${sources[@]}" -newer "$stamp" \( -name '*.rs' -o -name 'Cargo.toml' \) -print -quit)" ]; then
        touch "$stamp"
        build || echo "build failed; the running game keeps its last good build"
    fi
done
