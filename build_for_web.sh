#!/usr/bin/env bash
set -euo pipefail

# Build a browser (wasm32) bundle for the Ambition sandbox.
#
# This is the web-build counterpart to ./build_for_android.sh. It
# compiles the sandbox crate for `wasm32-unknown-unknown` with the
# `web` feature composite, runs `wasm-bindgen --target web` to emit
# the JS/wasm pair into `game/ambition_app/web/pkg/`, and
# optionally serves the directory so a browser can load it.
#
# Default: Rust release build + wasm-bindgen output, no auto-serve.
# Pass --serve to also start a static file server on http://localhost:8000/.
#
# See docs/recipes/web-build.md for the per-subsystem scope of the first-pass
# web build (audio / dev tools / file watcher / mobile touch / physics
# debris are intentionally OFF; LDtk loads via static_map).

usage() {
    cat <<'EOF'
Usage: ./build_for_web.sh [options]

Options:
  --release             Build the wasm artifact with Cargo --release (default).
  --debug               Build the wasm artifact with the dev profile (much larger, faster compile).
  --optimize            Build with the `web-release` profile (fat LTO, opt-level=s,
                        stripped). Much smaller module, much slower build. Use this
                        when the browser sits on "starting..." and the console shows
                        no Rust output at all.
  --features LIST       Cargo features to enable. Default: web (or web_served_assets when --served is passed)
  --use-default-features  Also enable ambition_app default features. Off by default for web.
  --no-default-features Disable default features (default for web builds).
  --bindgen-target T    Pass-through to wasm-bindgen --target. Default: web
                        Other supported values: bundler, no-modules, nodejs, deno.
  --out-dir DIR         Where wasm-bindgen writes the JS/wasm pair.
                        Default: game/ambition_app/web/pkg
  --skip-bindgen        Compile the wasm but skip the wasm-bindgen step.
  --skip-build          Skip the cargo build (re-run wasm-bindgen against an existing artifact).
  --served              Build the served-assets browser persona:
                        switches the default feature to `web_served_assets`,
                        and publishes the composed asset tree into
                        game/ambition_app/web/assets/ (scripts/package_asset_guard.py,
                        profile=web) so the page-served `/assets/...` URLs Bevy's
                        wasm HTTP reader fetches actually resolve. Selects
                        `AssetProfile::WebServedAssets` at runtime via the
                        `web_served` feature.
  --serve [PORT]        After building, serve `game/ambition_app/web/` on PORT (default 8000).
  --open                Open the served URL in the default browser. Implies --serve.
  --clean               Delete the wasm-bindgen output dir before building.
  --doctor              Check tools/environment and print what would be used.
  -h, --help            Show this help.

Environment overrides:
  CARGO                 Cargo command. Default: cargo
  WASM_BINDGEN          wasm-bindgen command. Default: wasm-bindgen
  AMBITION_WEB_PORT     Default port for --serve. Default: 8000

Examples:
  ./build_for_web.sh                          # WebStatic (embedded core assets)
  ./build_for_web.sh --serve
  ./build_for_web.sh --served --serve         # WebServedAssets (full game via served /assets/)
  ./build_for_web.sh --serve 9000 --open
  ./build_for_web.sh --debug --serve
  ./build_for_web.sh --doctor
EOF
}

log() { printf '[web-build] %s\n' "$*"; }
warn() { printf '[web-build] warning: %s\n' "$*" >&2; }
fatal() { printf '[web-build] error: %s\n' "$*" >&2; exit 1; }

repo_root() {
    local root
    root=$(git rev-parse --show-toplevel 2>/dev/null || true)
    if [[ -z "$root" ]]; then
        fatal "run this script from inside the Ambition git checkout"
    fi
    printf '%s\n' "$root"
}

need_cmd() {
    local cmd=$1
    local hint=$2
    if ! command -v "$cmd" >/dev/null 2>&1; then
        fatal "missing '$cmd'. $hint"
    fi
}

human_size() {
    local path=$1
    if [[ -e "$path" ]]; then
        du -h "$path" | awk '{print $1}'
    else
        printf 'missing'
    fi
}

detect_wasm_bindgen_version() {
    local lock=$1
    if [[ ! -f "$lock" ]]; then
        printf ''
        return 0
    fi
    awk '
        /^name = "wasm-bindgen"$/ { in_pkg=1; next }
        in_pkg && /^version = "/ {
            gsub(/[^0-9.]/, "", $0)
            print
            exit
        }
    ' "$lock"
}

# Cargo may be configured to put target artifacts outside the repo
# (this checkout's .cargo/config.toml does exactly that). Ask Cargo
# where the target directory is instead of assuming ./target.
cargo_target_dir() {
    local metadata
    if metadata=$("$CARGO_CMD" metadata --format-version 1 --no-deps 2>/dev/null); then
        if command -v python3 >/dev/null 2>&1; then
            python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])' <<<"$metadata"
            return 0
        fi
        printf '%s\n' "$metadata" | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p'
        return 0
    fi
    printf '%s/target\n' "$ROOT"
}

PROFILE="release"
FEATURES=""
FEATURES_EXPLICIT=false
USE_DEFAULT_FEATURES=false
BINDGEN_TARGET="web"
OUT_DIR=""
SKIP_BINDGEN=false
SKIP_BUILD=false
SERVE=false
SERVE_PORT=""
OPEN_BROWSER=false
CLEAN=false
DOCTOR=false
SERVED_MODE=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --release) PROFILE="release" ;;
        --debug) PROFILE="debug" ;;
        --optimize) PROFILE="web-release" ;;
        --features) shift; [[ $# -gt 0 ]] || fatal "--features needs a value"; FEATURES=$1; FEATURES_EXPLICIT=true ;;
        --use-default-features) USE_DEFAULT_FEATURES=true ;;
        --no-default-features) USE_DEFAULT_FEATURES=false ;;
        --bindgen-target) shift; [[ $# -gt 0 ]] || fatal "--bindgen-target needs a value"; BINDGEN_TARGET=$1 ;;
        --out-dir) shift; [[ $# -gt 0 ]] || fatal "--out-dir needs a path"; OUT_DIR=$1 ;;
        --skip-bindgen) SKIP_BINDGEN=true ;;
        --skip-build) SKIP_BUILD=true ;;
        --served) SERVED_MODE=true ;;
        --serve)
            SERVE=true
            # --serve optionally takes a numeric port; only consume the next
            # arg if it parses as a port number.
            if [[ ${2:-} =~ ^[0-9]+$ ]]; then
                SERVE_PORT=$2
                shift
            fi
            ;;
        --open) OPEN_BROWSER=true; SERVE=true ;;
        --clean) CLEAN=true ;;
        --doctor) DOCTOR=true ;;
        -h|--help) usage; exit 0 ;;
        *) fatal "unknown option: $1" ;;
    esac
    shift
done

# --served picks the right Cargo feature when one wasn't passed
# explicitly. Specifying --features ... wins so callers can compose
# with other features.
if [[ "$FEATURES_EXPLICIT" != true ]]; then
    if [[ "$SERVED_MODE" == true ]]; then
        FEATURES="web_served_assets"
    else
        FEATURES="web"
    fi
fi

ROOT=$(repo_root)
cd "$ROOT"

CARGO_CMD=${CARGO:-cargo}
WASM_BINDGEN_CMD=${WASM_BINDGEN:-wasm-bindgen}
SERVE_PORT=${SERVE_PORT:-${AMBITION_WEB_PORT:-8000}}

WEB_DIR="$ROOT/game/ambition_app/web"
if [[ -z "$OUT_DIR" ]]; then
    OUT_DIR="$WEB_DIR/pkg"
fi
[[ -d "$WEB_DIR" ]] || fatal "$WEB_DIR not found; expected the web bootstrap directory"

LOCK="$ROOT/Cargo.lock"
WANT_BINDGEN_VERSION=$(detect_wasm_bindgen_version "$LOCK")

TARGET_DIR=$(cargo_target_dir)
case "$PROFILE" in
    release) WASM_BUILD_DIR="$TARGET_DIR/wasm32-unknown-unknown/release" ;;
    debug)   WASM_BUILD_DIR="$TARGET_DIR/wasm32-unknown-unknown/debug" ;;
    web-release) WASM_BUILD_DIR="$TARGET_DIR/wasm32-unknown-unknown/web-release" ;;
    *) fatal "unknown profile: $PROFILE (expected release, web-release or debug)" ;;
esac
WASM_ARTIFACT="$WASM_BUILD_DIR/ambition_app.wasm"

log "repo: $ROOT"
log "profile: $PROFILE"
log "default features: $USE_DEFAULT_FEATURES"
log "features: ${FEATURES:-<default only>}"
log "wasm artifact: $WASM_ARTIFACT"
log "wasm-bindgen target: $BINDGEN_TARGET  out dir: $OUT_DIR"

# Audio is only in the build when the feature set includes web_audio.
# `web` (--serve without --served) is a visual smoke build with no
# audio backend: `bevy_kira_audio` isn't even compiled in, so the
# browser will boot silent and the `[ambition-audio] AudioContext
# created` log line will never fire. Surface this loudly because
# silent web audio after a build is otherwise indistinguishable from
# "Kira refused to resume" and easy to misdiagnose.
audio_feature_active=false
case ",$FEATURES," in
    *,web_audio,*|*,web_served_assets,*|*,visible_web_served,*|*,audio,*)
        audio_feature_active=true ;;
esac
if [[ "$audio_feature_active" == true ]]; then
    log "audio: ENABLED in build (bevy_kira_audio in wasm). The browser must show 'AssetProfile = web_served_assets' in the boot banner."
else
    warn "audio: DISABLED in build. This is a visual-smoke build only."
    warn "audio: bevy_kira_audio is NOT in the wasm; the browser will boot silent."
    warn "audio: for audible web audio rebuild with: ./build_for_web.sh --served --serve"
fi
if [[ -n "$WANT_BINDGEN_VERSION" ]]; then
    log "Cargo.lock pins wasm-bindgen $WANT_BINDGEN_VERSION (wasm-bindgen-cli must match)"
fi
if [[ "$SERVE" == true ]]; then
    log "serve: http://localhost:$SERVE_PORT/ from $WEB_DIR"
fi

need_cmd "$CARGO_CMD" "Install Rust via rustup."
if [[ "$SKIP_BINDGEN" != true ]]; then
    need_cmd "$WASM_BINDGEN_CMD" "Run: ./scripts/setup/web_prereq.sh"
    if [[ -n "$WANT_BINDGEN_VERSION" ]]; then
        have=$("$WASM_BINDGEN_CMD" --version 2>/dev/null | awk '{print $2}')
        if [[ -n "$have" && "$have" != "$WANT_BINDGEN_VERSION" ]]; then
            warn "wasm-bindgen-cli is $have but Cargo.lock pins $WANT_BINDGEN_VERSION; the browser will refuse to load the module on mismatch"
            warn "fix with: ./scripts/setup/web_prereq.sh --force-bindgen"
        fi
    fi
fi
if ! command -v rustup >/dev/null 2>&1; then
    warn "rustup not on PATH; assuming wasm32-unknown-unknown is already installed"
elif ! rustup target list --installed | grep -qx 'wasm32-unknown-unknown'; then
    fatal "missing rust target wasm32-unknown-unknown. Run: ./scripts/setup/web_prereq.sh"
fi

if [[ "$DOCTOR" == true ]]; then
    log "doctor completed; no build performed"
    exit 0
fi

if [[ "$CLEAN" == true && -d "$OUT_DIR" ]]; then
    log "cleaning $OUT_DIR"
    rm -rf "$OUT_DIR"
fi

if [[ "$SKIP_BUILD" != true ]]; then
    CARGO_ARGS=(build -p ambition_app --lib --target wasm32-unknown-unknown)
    case "$PROFILE" in
        release) CARGO_ARGS+=(--release) ;;
        web-release) CARGO_ARGS+=(--profile web-release) ;;
        debug) ;;
    esac
    if [[ "$USE_DEFAULT_FEATURES" != true ]]; then
        CARGO_ARGS+=(--no-default-features)
    fi
    if [[ -n "$FEATURES" ]]; then
        CARGO_ARGS+=(--features "$FEATURES")
    fi
    log "building wasm artifact: $CARGO_CMD ${CARGO_ARGS[*]}"
    "$CARGO_CMD" "${CARGO_ARGS[@]}"
fi

[[ -f "$WASM_ARTIFACT" ]] || fatal "wasm artifact not found at $WASM_ARTIFACT after build; check cargo output"
log "wasm artifact size: $(human_size "$WASM_ARTIFACT")"

if [[ "$SKIP_BINDGEN" != true ]]; then
    mkdir -p "$OUT_DIR"
    log "running wasm-bindgen --target $BINDGEN_TARGET --out-dir $OUT_DIR"
    "$WASM_BINDGEN_CMD" \
        "$WASM_ARTIFACT" \
        --out-dir "$OUT_DIR" \
        --out-name ambition_app \
        --target "$BINDGEN_TARGET" \
        --no-typescript
    OUT_WASM="$OUT_DIR/ambition_app_bg.wasm"
    OUT_JS="$OUT_DIR/ambition_app.js"
    if [[ -f "$OUT_WASM" ]]; then
        log "wasm-bindgen output: $(human_size "$OUT_WASM") wasm, $(human_size "$OUT_JS") js"
        # SIZE IS A BOOT FAILURE MODE, not a nicety. The browser must
        # download AND COMPILE this before a single line of Rust runs, and a
        # module in the hundreds of MB can take a minute or hang a tab outright —
        # which presents as "the page loads and nothing happens", indistinguishable
        # from a panic. Say the number and the fix, because the symptom does not.
        OUT_WASM_BYTES=$(stat -c%s "$OUT_WASM" 2>/dev/null || echo 0)
        if [[ "$OUT_WASM_BYTES" -gt 104857600 && "$PROFILE" != "web-release" ]]; then
            warn "the wasm module is $(human_size "$OUT_WASM") — very large for a browser to compile."
            warn "  If the page sits on 'starting…' and the console shows no Rust output,"
            warn "  this is the first suspect, not the composition."
            warn "  ⭐ RUN --optimize. It is not a suggestion, it is measured (2026-08-15,"
            warn "  --served at HEAD): 164M -> 84M wasm, 26.4M -> 14.3M gzipped, and the"
            warn "  37M debug-symbol name section disappears entirely. Costs ~9 minutes."
            warn "  The default \`release\` profile has strip = \"none\", which is where a"
            warn "  fifth of this module comes from."
        elif [[ "$OUT_WASM_BYTES" -gt 104857600 ]]; then
            warn "the wasm module is $(human_size "$OUT_WASM") even under web-release."
            warn "  The profile levers are spent; what is left is genuine code and data."
            warn "  See docs/planning/engine/portable-preparation-and-load-explainability.md."
        fi
    else
        warn "wasm-bindgen finished but expected $OUT_WASM was not produced"
    fi
fi

# --served publishes the browser persona's asset tree: the page fetches
# `/assets/<path>` over HTTP, so `python3 -m http.server` must expose the same
# composed view the game resolves on a desktop.
#
# It named a crate the ongoing Bevy decomposition is actively dismantling — rename it and the web
# build breaks for a reason nothing here explains — and it published only ONE of the two roots, so
# everything the content crate owns (every world addressed `game://worlds/...`, the vanity card) was
# simply absent from the served tree.
#
# The composed tree is not this script's to invent. `package_asset_guard.py
# compose` is the single seam that collapses the roots, forbids implicit
# overrides between them, and emits a byte contract — the same one Android
# verifies against its APK and the Steam Deck deploy verifies after rsync. Web
# is now a fourth consumer of that one publication rather than a fourth opinion
# about where assets live, and it names no crate at all.
#
# `--materialize link` because this is a DEVELOPMENT loop: the composed tree
# is ~1.1 GB and would otherwise be duplicated on every wasm build. The contract
# and its full hash audit run identically either way; only the bytes are shared.
# A shipped package keeps the default `copy` — an APK cannot carry a link.
SERVED_ASSETS_DIR="$WEB_DIR/assets"
if [[ "$SERVED_MODE" == true ]]; then
    log "served assets: composing the published tree (package_asset_guard.py, profile=web)"
    if ! python3 "$ROOT/scripts/package_asset_guard.py" compose \
        --repo "$ROOT" \
        --profile web \
        --materialize link \
        --output "$SERVED_ASSETS_DIR" \
        --contract "$WEB_DIR/.assets_contract.json" \
        --hash-manifest "$WEB_DIR/.assets_sha256"; then
        fatal "asset composition failed; the served page would fetch an incomplete tree"
    fi
    log "served assets: published $SERVED_ASSETS_DIR (both roots, contract verified)"
fi

if [[ "$SERVE" != true ]]; then
    echo
    log "done. Open the bundle by serving $WEB_DIR with any static file server."
    log "Examples:"
    log "  python3 -m http.server -d $WEB_DIR $SERVE_PORT"
    log "  basic-http-server $WEB_DIR -a 127.0.0.1:$SERVE_PORT"
    exit 0
fi

URL="http://localhost:$SERVE_PORT/"

if [[ "$OPEN_BROWSER" == true ]]; then
    # xdg-open is best-effort; failure is non-fatal.
    if command -v xdg-open >/dev/null 2>&1; then
        ( sleep 1 && xdg-open "$URL" >/dev/null 2>&1 ) &
    elif command -v open >/dev/null 2>&1; then
        ( sleep 1 && open "$URL" >/dev/null 2>&1 ) &
    else
        warn "no xdg-open / open found; visit $URL manually"
    fi
fi

if command -v python3 >/dev/null 2>&1; then
    log "serving $WEB_DIR at $URL (Ctrl-C to stop)"
    exec python3 -m http.server -d "$WEB_DIR" "$SERVE_PORT"
elif command -v basic-http-server >/dev/null 2>&1; then
    log "serving $WEB_DIR at $URL via basic-http-server (Ctrl-C to stop)"
    exec basic-http-server "$WEB_DIR" -a "127.0.0.1:$SERVE_PORT"
else
    fatal "no static file server found. Install python3, or run: ./scripts/setup/web_prereq.sh --with-server"
fi
