#!/usr/bin/env bash
set -euo pipefail

# Install/check the Linux prerequisites for Ambition web (wasm32) builds.
#
# Mirror of `scripts/setup_android_prereqs.sh` for the browser build path.
# Installs the `wasm32-unknown-unknown` rustup target and a
# `wasm-bindgen-cli` whose version matches the `wasm-bindgen` crate
# already locked into `Cargo.lock` — a mismatched CLI is the most
# common cause of a "version mismatch" runtime error in the browser.
#
# Usage:
#   ./scripts/setup_web_prereq.sh
#   ./scripts/setup_web_prereq.sh --doctor
#   ./scripts/setup_web_prereq.sh --with-server   # also install basic-http-server
#
# Environment overrides:
#   WASM_BINDGEN_VERSION=0.2.x   Pin a specific wasm-bindgen-cli version
#                                instead of auto-detecting from Cargo.lock.

usage() {
    cat <<'EOF'
Usage: ./scripts/setup_web_prereq.sh [options]

Options:
  --doctor          Check the environment and print missing pieces; do not install.
  --with-server     Also install basic-http-server (a small Rust static file server)
                    so `./build_for_web.sh --serve` has a non-Python fallback.
  --skip-apt        Do not install host packages with apt-get.
  --force-bindgen   Reinstall wasm-bindgen-cli even if a matching version is present.
  -h, --help        Show this help.

Environment overrides:
  WASM_BINDGEN_VERSION   Override the wasm-bindgen-cli version. If unset, the
                         script reads the wasm-bindgen crate version from
                         Cargo.lock and pins the CLI to the same version.
EOF
}

log() { printf '[web-prereq] %s\n' "$*"; }
warn() { printf '[web-prereq] warning: %s\n' "$*" >&2; }
fatal() { printf '[web-prereq] error: %s\n' "$*" >&2; exit 1; }

DOCTOR=false
WITH_SERVER=false
SKIP_APT=false
FORCE_BINDGEN=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --doctor) DOCTOR=true ;;
        --with-server) WITH_SERVER=true ;;
        --skip-apt) SKIP_APT=true ;;
        --force-bindgen) FORCE_BINDGEN=true ;;
        -h|--help) usage; exit 0 ;;
        *) fatal "unknown option: $1" ;;
    esac
    shift
done

repo_root() {
    local root
    root=$(git rev-parse --show-toplevel 2>/dev/null || true)
    if [[ -z "$root" ]]; then
        fatal "run this script from inside the Ambition git checkout"
    fi
    printf '%s\n' "$root"
}

ROOT=$(repo_root)
LOCK="$ROOT/Cargo.lock"

detect_wasm_bindgen_version() {
    if [[ -n "${WASM_BINDGEN_VERSION:-}" ]]; then
        printf '%s\n' "$WASM_BINDGEN_VERSION"
        return 0
    fi
    if [[ ! -f "$LOCK" ]]; then
        fatal "Cargo.lock not found at $LOCK; run 'cargo generate-lockfile' or set WASM_BINDGEN_VERSION"
    fi
    # Cargo.lock package entries look like:
    #   [[package]]
    #   name = "wasm-bindgen"
    #   version = "0.2.120"
    # Pull the first version line that follows the wasm-bindgen name line.
    local version
    version=$(awk '
        /^name = "wasm-bindgen"$/ { in_pkg=1; next }
        in_pkg && /^version = "/ {
            gsub(/[^0-9.]/, "", $0)
            print
            exit
        }
    ' "$LOCK")
    if [[ -z "$version" ]]; then
        fatal "could not detect wasm-bindgen version from $LOCK; set WASM_BINDGEN_VERSION manually"
    fi
    printf '%s\n' "$version"
}

check_cmd() {
    local name=$1
    if command -v "$name" >/dev/null 2>&1; then
        printf 'ok      %s -> %s\n' "$name" "$(command -v "$name")"
        return 0
    fi
    printf 'missing %s\n' "$name"
    return 1
}

installed_bindgen_version() {
    if ! command -v wasm-bindgen >/dev/null 2>&1; then
        printf ''
        return 0
    fi
    # `wasm-bindgen --version` prints e.g. "wasm-bindgen 0.2.120"
    wasm-bindgen --version 2>/dev/null | awk '{print $2}'
}

run_doctor() {
    local missing=0
    local want_version
    want_version=$(detect_wasm_bindgen_version || true)

    echo "[web-prereq] environment"
    echo "  repo: $ROOT"
    echo "  desired wasm-bindgen-cli: ${want_version:-<unknown>}"
    echo
    check_cmd rustup || missing=1
    check_cmd cargo  || missing=1
    if rustup target list --installed 2>/dev/null | grep -qx 'wasm32-unknown-unknown'; then
        echo "ok      rust target wasm32-unknown-unknown"
    else
        echo "missing rust target wasm32-unknown-unknown"
        missing=1
    fi
    local have
    have=$(installed_bindgen_version)
    if [[ -n "$have" && "$have" == "$want_version" ]]; then
        echo "ok      wasm-bindgen-cli $have (matches Cargo.lock)"
    elif [[ -n "$have" ]]; then
        echo "warn    wasm-bindgen-cli $have (Cargo.lock wants $want_version)"
        missing=1
    else
        echo "missing wasm-bindgen-cli"
        missing=1
    fi
    if [[ "$WITH_SERVER" == true ]]; then
        if command -v basic-http-server >/dev/null 2>&1; then
            echo "ok      basic-http-server -> $(command -v basic-http-server)"
        else
            echo "missing basic-http-server"
            missing=1
        fi
    fi
    if command -v python3 >/dev/null 2>&1; then
        echo "ok      python3 -> $(command -v python3) (default static server for ./build_for_web.sh --serve)"
    else
        warn "python3 not found; ./build_for_web.sh --serve will fall back to basic-http-server (use --with-server)"
    fi

    echo
    if [[ $missing -eq 0 ]]; then
        echo "[web-prereq] doctor passed"
    else
        echo "[web-prereq] doctor found missing prerequisites"
    fi
    return "$missing"
}

install_host_packages() {
    if [[ "$SKIP_APT" == true ]]; then
        log "skipping apt host package install"
        return 0
    fi
    if ! command -v apt-get >/dev/null 2>&1; then
        warn "apt-get not found; assuming curl/pkg-config are already installed"
        return 0
    fi
    # Minimal host bits: curl for the rustup bootstrap (if rustup itself is
    # missing) and pkg-config for crates that probe the host C ABI during
    # build.rs even on wasm. Do NOT install python3 here; most distros
    # already ship it and the user's filesystem feedback rule says to avoid
    # `sudo apt-get` cascades.
    log "installing host packages via apt"
    sudo apt-get update
    sudo apt-get install -y \
        curl \
        ca-certificates \
        pkg-config
}

install_rust_target() {
    if ! command -v rustup >/dev/null 2>&1; then
        fatal "rustup not found; install Rust via https://rustup.rs first"
    fi
    if rustup target list --installed | grep -qx 'wasm32-unknown-unknown'; then
        log "rust target wasm32-unknown-unknown already installed"
    else
        log "installing rust target wasm32-unknown-unknown"
        rustup target add wasm32-unknown-unknown
    fi
}

install_wasm_bindgen_cli() {
    local want="$1"
    local have
    have=$(installed_bindgen_version)
    if [[ "$FORCE_BINDGEN" != true && -n "$have" && "$have" == "$want" ]]; then
        log "wasm-bindgen-cli $have already installed (matches Cargo.lock)"
        return 0
    fi
    if [[ -n "$have" && "$have" != "$want" ]]; then
        log "replacing wasm-bindgen-cli $have with $want to match Cargo.lock"
    else
        log "installing wasm-bindgen-cli $want"
    fi
    cargo install --locked --version "$want" wasm-bindgen-cli
}

install_optional_server() {
    if [[ "$WITH_SERVER" != true ]]; then
        return 0
    fi
    if command -v basic-http-server >/dev/null 2>&1; then
        log "basic-http-server already installed"
        return 0
    fi
    log "installing basic-http-server (used by ./build_for_web.sh --serve when python3 is absent)"
    cargo install basic-http-server
}

if [[ "$DOCTOR" == true ]]; then
    run_doctor
    exit $?
fi

WANT_BINDGEN=$(detect_wasm_bindgen_version)
log "repo: $ROOT"
log "desired wasm-bindgen-cli: $WANT_BINDGEN"

install_host_packages
install_rust_target
install_wasm_bindgen_cli "$WANT_BINDGEN"
install_optional_server

echo
log "versions"
rustup --version 2>/dev/null || true
cargo --version 2>/dev/null || true
wasm-bindgen --version 2>/dev/null || true
if [[ "$WITH_SERVER" == true ]]; then
    basic-http-server --version 2>/dev/null || true
fi

echo
log "done"
echo "Next:"
echo "  ./scripts/setup_web_prereq.sh --doctor   # re-check"
echo "  ./build_for_web.sh --doctor              # check the build pipeline"
echo "  ./build_for_web.sh --serve               # build + serve at http://localhost:8000/"
