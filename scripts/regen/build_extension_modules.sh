#!/usr/bin/env bash
# Build the game's procedural modules as `.wasm` files (fast-iteration I6/I7).
#
# This compiles ONLY the module crate and the pure port values it uses; the
# engine is not compiled or linked. Run the game with the result:
#
#   scripts/regen/build_extension_modules.sh
#   AMBITION_EXTENSION_MODULES=target/extension-modules/wasm32-unknown-unknown/release \
#       cargo run -p ambition_app
#
# The loaded modules explicitly replace the linked ones of the same key; the
# log says which.
#
# With --watch it stays running and rebuilds each time a source the modules
# are built from changes: the selected module crate and every local package it
# depends on (its port value crates, the SDK), as Cargo resolves them.
# A running game reloads the new file by itself: save the file, see the
# change in the game. A build that fails prints its errors and keeps watching;
# the game keeps the last good build.
#
#   scripts/regen/build_extension_modules.sh --watch
#
# An independent project can name its Cargo workspace with --manifest-path.
# The output still uses this SDK checkout's checked target storage.
set -euo pipefail
watch=0
package=""
module_manifest=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --watch) watch=1; shift ;;
        --manifest-path)
            if [ "$#" -lt 2 ]; then echo "--manifest-path needs a Cargo.toml path" >&2; exit 1; fi
            module_manifest="$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).resolve())' "$2")"
            shift 2 ;;
        --help|-h)
            echo "Usage: $0 [--watch] [--manifest-path Cargo.toml] [package]"
            exit 0 ;;
        --*) echo "unknown option: $1 (use --help)" >&2; exit 1 ;;
        *)
            if [ -n "$package" ]; then echo "select one module package" >&2; exit 1; fi
            package="$1"; shift ;;
    esac
done
package="${package:-ambition_content_modules}"
cd "$(dirname "$0")/../.."
scripts/setup/target_bindmount.sh --check >/dev/null
module_manifest="${module_manifest:-$PWD/Cargo.toml}"
if ! [ -d "$(rustc --print sysroot)/lib/rustlib/wasm32-unknown-unknown/lib" ]; then
    echo "the rust target wasm32-unknown-unknown is not installed:" >&2
    echo "    rustup target add wasm32-unknown-unknown" >&2
    exit 1
fi
out="target/extension-modules/wasm32-unknown-unknown/release/${package}.wasm"
build() {
    local start=$(date +%s.%N)
    # The stack size is ambition_extension_wasm::MODULE_STACK_BYTES (128 KiB):
    # a new instance zeroes its memory, and the 1 MiB default stack was about
    # half the cost of a call. The exported stack pointer is
    # MODULE_STACK_POINTER: with it a call can reuse an instance and set it back.
    if CARGO_INCREMENTAL=0 cargo rustc --quiet --manifest-path "$module_manifest" -p "$package" --target wasm32-unknown-unknown --release \
        --crate-type cdylib --target-dir target/extension-modules -- -Clink-arg=-zstack-size=131072 -Clink-arg=--export=__stack_pointer; then
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

# The sources a module build reads: the selected package and every local
# package in its resolved graph for the wasm target, from Cargo itself. A fixed
# list named the Ambition module tree only, so `--watch` of Mary-O's or
# Smash's modules never saw their edits (runtime authoring audit, B03).
mapfile -t sources < <(cargo metadata --manifest-path "$module_manifest" --format-version 1 --filter-platform wasm32-unknown-unknown 2>/dev/null \
    | python3 -c '
import json, os, sys
meta = json.load(sys.stdin)
root = os.getcwd()
pkgs = {p["id"]: p for p in meta["packages"]}
nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
start = [p["id"] for p in meta["packages"] if p["name"] == sys.argv[1] and p["source"] is None]
seen, todo = set(), list(start)
while todo:
    pid = todo.pop()
    if pid in seen or pkgs[pid]["source"] is not None:
        continue
    seen.add(pid)
    todo.extend(d["pkg"] for d in nodes[pid]["deps"]
                if any(k["kind"] in (None, "build") for k in d["dep_kinds"]))
for pid in sorted(seen):
    print(os.path.relpath(os.path.dirname(pkgs[pid]["manifest_path"]), root))
' "$package")
if [ "${#sources[@]}" = 0 ]; then
    echo "no local package named $package: nothing to watch" >&2
    exit 1
fi
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
