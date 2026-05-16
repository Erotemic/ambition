#!/usr/bin/env bash
set -euo pipefail

# Build a browser (wasm32) bundle for the Ambition sandbox.
#
# This is the web-build counterpart to ./build_for_android.sh. It
# compiles the sandbox crate for `wasm32-unknown-unknown` with the
# `web` feature composite, runs `wasm-bindgen --target web` to emit
# the JS/wasm pair into `crates/ambition_sandbox/web/pkg/`, and
# optionally serves the directory so a browser can load it.
#
# Default: Rust release build + wasm-bindgen output, no auto-serve.
# Pass --serve to also start a static file server on http://localhost:8000/.
#
# See docs/web_build.md for the per-subsystem scope of the first-pass
# web build (audio / dev tools / file watcher / mobile touch / physics
# debris are intentionally OFF; LDtk loads via static_map).

usage() {
    cat <<'EOF'
Usage: ./build_for_web.sh [options]

Options:
  --release             Build the wasm artifact with Cargo --release (default).
  --debug               Build the wasm artifact with the dev profile (much larger, faster compile).
  --features LIST       Cargo features to enable. Default: web
  --use-default-features  Also enable ambition_sandbox default features. Off by default for web.
  --no-default-features Disable default features (default for web builds).
  --bindgen-target T    Pass-through to wasm-bindgen --target. Default: web
                        Other supported values: bundler, no-modules, nodejs, deno.
  --out-dir DIR         Where wasm-bindgen writes the JS/wasm pair.
                        Default: crates/ambition_sandbox/web/pkg
  --skip-bindgen        Compile the wasm but skip the wasm-bindgen step.
  --skip-build          Skip the cargo build (re-run wasm-bindgen against an existing artifact).
  --serve [PORT]        After building, serve `crates/ambition_sandbox/web/` on PORT (default 8000).
  --open                Open the served URL in the default browser. Implies --serve.
  --clean               Delete the wasm-bindgen output dir before building.
  --doctor              Check tools/environment and print what would be used.
  -h, --help            Show this help.

Environment overrides:
  CARGO                 Cargo command. Default: cargo
  WASM_BINDGEN          wasm-bindgen command. Default: wasm-bindgen
  AMBITION_WEB_PORT     Default port for --serve. Default: 8000

Examples:
  ./build_for_web.sh
  ./build_for_web.sh --serve
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

PROFILE="release"
FEATURES="web"
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

while [[ $# -gt 0 ]]; do
    case "$1" in
        --release) PROFILE="release" ;;
        --debug) PROFILE="debug" ;;
        --features) shift; [[ $# -gt 0 ]] || fatal "--features needs a value"; FEATURES=$1 ;;
        --use-default-features) USE_DEFAULT_FEATURES=true ;;
        --no-default-features) USE_DEFAULT_FEATURES=false ;;
        --bindgen-target) shift; [[ $# -gt 0 ]] || fatal "--bindgen-target needs a value"; BINDGEN_TARGET=$1 ;;
        --out-dir) shift; [[ $# -gt 0 ]] || fatal "--out-dir needs a path"; OUT_DIR=$1 ;;
        --skip-bindgen) SKIP_BINDGEN=true ;;
        --skip-build) SKIP_BUILD=true ;;
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

ROOT=$(repo_root)
cd "$ROOT"

CARGO_CMD=${CARGO:-cargo}
WASM_BINDGEN_CMD=${WASM_BINDGEN:-wasm-bindgen}
SERVE_PORT=${SERVE_PORT:-${AMBITION_WEB_PORT:-8000}}

WEB_DIR="$ROOT/crates/ambition_sandbox/web"
if [[ -z "$OUT_DIR" ]]; then
    OUT_DIR="$WEB_DIR/pkg"
fi
[[ -d "$WEB_DIR" ]] || fatal "$WEB_DIR not found; expected the web bootstrap directory"

LOCK="$ROOT/Cargo.lock"
WANT_BINDGEN_VERSION=$(detect_wasm_bindgen_version "$LOCK")

case "$PROFILE" in
    release) WASM_BUILD_DIR="$ROOT/target/wasm32-unknown-unknown/release" ;;
    debug)   WASM_BUILD_DIR="$ROOT/target/wasm32-unknown-unknown/debug" ;;
    *) fatal "unknown profile: $PROFILE (expected release or debug)" ;;
esac
WASM_ARTIFACT="$WASM_BUILD_DIR/ambition_sandbox.wasm"

log "repo: $ROOT"
log "profile: $PROFILE"
log "default features: $USE_DEFAULT_FEATURES"
log "features: ${FEATURES:-<default only>}"
log "wasm artifact: $WASM_ARTIFACT"
log "wasm-bindgen target: $BINDGEN_TARGET  out dir: $OUT_DIR"
if [[ -n "$WANT_BINDGEN_VERSION" ]]; then
    log "Cargo.lock pins wasm-bindgen $WANT_BINDGEN_VERSION (wasm-bindgen-cli must match)"
fi
if [[ "$SERVE" == true ]]; then
    log "serve: http://localhost:$SERVE_PORT/ from $WEB_DIR"
fi

need_cmd "$CARGO_CMD" "Install Rust via rustup."
if [[ "$SKIP_BINDGEN" != true ]]; then
    need_cmd "$WASM_BINDGEN_CMD" "Run: ./scripts/setup_web_prereq.sh"
    if [[ -n "$WANT_BINDGEN_VERSION" ]]; then
        have=$("$WASM_BINDGEN_CMD" --version 2>/dev/null | awk '{print $2}')
        if [[ -n "$have" && "$have" != "$WANT_BINDGEN_VERSION" ]]; then
            warn "wasm-bindgen-cli is $have but Cargo.lock pins $WANT_BINDGEN_VERSION; the browser will refuse to load the module on mismatch"
            warn "fix with: ./scripts/setup_web_prereq.sh --force-bindgen"
        fi
    fi
fi
if ! command -v rustup >/dev/null 2>&1; then
    warn "rustup not on PATH; assuming wasm32-unknown-unknown is already installed"
elif ! rustup target list --installed | grep -qx 'wasm32-unknown-unknown'; then
    fatal "missing rust target wasm32-unknown-unknown. Run: ./scripts/setup_web_prereq.sh"
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
    CARGO_ARGS=(build -p ambition_sandbox --lib --target wasm32-unknown-unknown)
    case "$PROFILE" in
        release) CARGO_ARGS+=(--release) ;;
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
        --target "$BINDGEN_TARGET" \
        --no-typescript
    OUT_WASM="$OUT_DIR/ambition_sandbox_bg.wasm"
    OUT_JS="$OUT_DIR/ambition_sandbox.js"
    if [[ -f "$OUT_WASM" ]]; then
        log "wasm-bindgen output: $(human_size "$OUT_WASM") wasm, $(human_size "$OUT_JS") js"
    else
        warn "wasm-bindgen finished but expected $OUT_WASM was not produced"
    fi
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
    # Open in the background so we don't race the server before it
    # accepts connections. xdg-open is best-effort; failure is non-fatal.
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
    fatal "no static file server found. Install python3, or run: ./scripts/setup_web_prereq.sh --with-server"
fi
