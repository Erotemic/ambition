#!/usr/bin/env bash
# It launches `rust-analyzer` directly, so it maintains real document versions
# and sends proper change notifications. This bridge never sends `didChange`:
# it opens a document once and answers every later position query from the
# content that file had when the server process first touched it, so its answers
# go stale the moment you edit — verified here, and the reason `cargo check` is
# the only compile gate. Its `diagnostics` is worse than stale on a cold start:
# before proc macros are built it returns a list of confident, entirely fake
# `E0559`/`E0599` errors (every deref through `Res`/`ResMut` reads as a missing
# field), alongside `macro-error: proc-macro not yet built` hints that are the
# tell. Discard any response containing that hint and ask again.
#
# This script remains for clients that genuinely lack native LSP integration
#. If you are fixing it rather than replacing it, the
# upstream changes worth making are: make check-on-save configurable and default
# it OFF, drop the synthetic `didSave`, drop the unconditional workspace reload,
# implement `didChange`, and pin the installed revision.

set -Eeuo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
cd "$REPO_ROOT"

CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
export CARGO_HOME
export RUSTUP_HOME
export PATH="$CARGO_HOME/bin:$HOME/.local/bin:$PATH"

SERVER_NAME="${SERVER_NAME:-rust-analyzer}"
MCP_BIN="${MCP_BIN:-rust-analyzer-mcp}"
CARGO_PACKAGE="${CARGO_PACKAGE:-rust-analyzer-mcp}"

# The server's cargo work MUST NOT share the project's target directory.
#
# `rust-analyzer-mcp` hardcodes `checkOnSave.enable = true` and `allTargets =
# true`, sends that config at initialize AND again via
# `workspace/didChangeConfiguration`, calls `rust-analyzer/reloadWorkspace` on
# startup, and manufactures a `textDocument/didSave` the first time it opens any
# file — its own source comment says "trigger cargo check". None of that is
# configurable through the bridge (its README still lists configuration as future
# work). So a `cargo check --workspace` fires on first touch of a file and holds
# the Cargo target-dir lock.
#
# The cost is duplicated artifacts, which the upstream docs call the expected trade.
#
# Outside the repo on purpose, like `.cargo/config.toml`'s own target dir: a
# second target tree inside the working copy is noise in every `git status` and
# every file walk.
RA_TARGET_DIR="${RA_TARGET_DIR:-$HOME/.cache/rust-analyzer-mcp/$(basename "$REPO_ROOT")-target}"

SCOPE="local"
DO_REGISTER=1
DO_SMOKE_TEST=1
DO_INSTALL=1
DO_UNINSTALL=0
CLAUDE_DIRS=()

ALL_SCOPES=(local project user)
MCP_BIN_PATH=""
RUST_ANALYZER_BIN_PATH=""
REGISTERED_COUNT=0

log() {
    printf '[setup_rust_analyzer_mcp] %s\n' "$*"
}

warn() {
    printf '[setup_rust_analyzer_mcp] warning: %s\n' "$*" >&2
}

fail() {
    printf '[setup_rust_analyzer_mcp] error: %s\n' "$*" >&2
    exit 1
}

usage() {
    cat <<'USAGE'
Usage: ./setup_rust_analyzer_mcp.sh [options]

Options:
  --claude-dir DIR   Directory you launch Claude Code from. Repeatable.
                     `local` scope is keyed to this directory, so the server is only
                     visible in sessions started there. Defaults to this repo root.
  --scope SCOPE      local (default), project, or user.
                       local   - private to you, per --claude-dir, not committed
                       project - writes a portable .mcp.json in --claude-dir
                       user    - visible in every project on this machine, but always
                                 analyzes this repository
  --uninstall        Unregister the server from every scope under each --claude-dir,
                     then `cargo uninstall` the binary. Combine with --no-install to
                     keep the binary, or --no-register to keep registrations.
  --no-install       Do not run cargo install (or cargo uninstall with --uninstall).
  --no-register      Do not touch Claude configuration.
  --no-smoke-test    Skip the MCP initialize + tools/list handshake check.
  -h, --help         Show this help.

Environment:
  SERVER_NAME        MCP server name registered with Claude. Default: rust-analyzer
  MCP_BIN            Binary to run. Default: rust-analyzer-mcp
  CARGO_PACKAGE      Cargo package to install. Default: rust-analyzer-mcp
  FORCE=1            Reinstall the MCP package with `cargo install --force`.

Examples:
  ./setup_rust_analyzer_mcp.sh
  ./setup_rust_analyzer_mcp.sh --claude-dir ../..
  ./setup_rust_analyzer_mcp.sh --scope project
  ./setup_rust_analyzer_mcp.sh --scope user
  FORCE=1 ./setup_rust_analyzer_mcp.sh
  ./setup_rust_analyzer_mcp.sh --uninstall
  ./setup_rust_analyzer_mcp.sh --uninstall --no-install
USAGE
}

parse_args() {
    while [ $# -gt 0 ]; do
        case "$1" in
            --claude-dir)
                [ $# -ge 2 ] || fail "--claude-dir requires a directory argument"
                CLAUDE_DIRS+=("$2")
                shift 2
                ;;
            --scope)
                [ $# -ge 2 ] || fail "--scope requires an argument"
                SCOPE="$2"
                shift 2
                ;;
            --uninstall)     DO_UNINSTALL=1; shift ;;
            --no-install)    DO_INSTALL=0; shift ;;
            --no-register)   DO_REGISTER=0; shift ;;
            --no-smoke-test) DO_SMOKE_TEST=0; shift ;;
            -h|--help)       usage; exit 0 ;;
            *)               fail "unknown argument: $1 (try --help)" ;;
        esac
    done

    case "$SCOPE" in
        local|project|user) ;;
        *) fail "invalid --scope '$SCOPE' (expected local, project, or user)" ;;
    esac

    if [ "${#CLAUDE_DIRS[@]}" -eq 0 ]; then
        CLAUDE_DIRS=("$REPO_ROOT")
    fi

    local i dir
    for i in "${!CLAUDE_DIRS[@]}"; do
        dir="${CLAUDE_DIRS[$i]}"
        [ -d "$dir" ] || fail "--claude-dir does not exist: $dir"
        CLAUDE_DIRS[$i]="$(cd "$dir" && pwd -P)"
    done
}

verify_workspace() {
    [ -f "$REPO_ROOT/Cargo.toml" ] ||
        fail "$REPO_ROOT does not look like a Rust workspace; no Cargo.toml found"
    log "Rust workspace: $REPO_ROOT"
}

verify_python() {
    command -v python3 >/dev/null 2>&1 ||
        fail "python3 is required for metadata inspection and the MCP smoke test"
    log "python3: $(python3 --version 2>&1)"
}

verify_rust_tools() {
    command -v cargo >/dev/null 2>&1 ||
        fail "cargo not found on PATH. Install Rust first: https://rustup.rs/"
    command -v rustc >/dev/null 2>&1 ||
        fail "rustc not found on PATH. Install Rust first: https://rustup.rs/"

    log "cargo: $(cargo --version)"
    log "rustc: $(rustc --version)"

    if command -v rustup >/dev/null 2>&1; then
        log "ensuring rust-analyzer is installed via rustup"
        rustup component add rust-analyzer
    elif ! command -v rust-analyzer >/dev/null 2>&1; then
        fail "rustup is not installed and rust-analyzer is not on PATH"
    fi

    command -v rust-analyzer >/dev/null 2>&1 ||
        fail "rust-analyzer is not on PATH after installation"

    RUST_ANALYZER_BIN_PATH="$(command -v rust-analyzer)"
    RUST_ANALYZER_BIN_PATH="$(cd "$(dirname "$RUST_ANALYZER_BIN_PATH")" && pwd -P)/$(basename "$RUST_ANALYZER_BIN_PATH")"
    log "rust-analyzer: $(rust-analyzer --version | head -1)"
}

check_ripgrep() {
    if command -v rg >/dev/null 2>&1; then
        log "ripgrep found; local source searches will be fast"
    else
        warn "ripgrep (rg) not found. Claude can still use rust-analyzer, but source searches will be weaker."
    fi
}

check_cargo_artifacts() {
    local target_dir

    target_dir="$(
        cd "$REPO_ROOT" &&
            cargo metadata --no-deps --format-version 1 2>/dev/null |
            python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])' 2>/dev/null
    )" || true
    [ -n "$target_dir" ] || target_dir="$REPO_ROOT/target"

    if [ -d "$target_dir/debug/.fingerprint" ] || [ -d "$target_dir/release/.fingerprint" ]; then
        log "cargo artifacts present under $target_dir"
        return
    fi

    warn "no cargo build/check artifacts found under $target_dir"
    warn "the first rust-analyzer tool call may be slow while dependencies are indexed"
    warn "recommended warmup: run 'cargo check' in $REPO_ROOT"
}

resolve_server_binary() {
    command -v "$MCP_BIN" >/dev/null 2>&1 || fail "$MCP_BIN not found on PATH"
    MCP_BIN_PATH="$(command -v "$MCP_BIN")"
    MCP_BIN_PATH="$(cd "$(dirname "$MCP_BIN_PATH")" && pwd -P)/$(basename "$MCP_BIN_PATH")"
    [ -x "$MCP_BIN_PATH" ] || fail "$MCP_BIN_PATH is not executable"
    log "server binary: $MCP_BIN_PATH"
}

install_server() {
    if [ "$DO_INSTALL" -eq 0 ]; then
        log "skipping cargo install (--no-install)"
        resolve_server_binary
        return
    fi

    if command -v "$MCP_BIN" >/dev/null 2>&1 && [ "${FORCE:-0}" != "1" ]; then
        log "$MCP_BIN is already installed: $(command -v "$MCP_BIN")"
        resolve_server_binary
        return
    fi

    log "installing $CARGO_PACKAGE with cargo"
    if [ "${FORCE:-0}" = "1" ]; then
        cargo install "$CARGO_PACKAGE" --force
    else
        cargo install "$CARGO_PACKAGE"
    fi

    resolve_server_binary
}

reinstall_server() {
    log "reinstalling $CARGO_PACKAGE with cargo install --force"
    cargo install "$CARGO_PACKAGE" --force || fail "reinstall of $CARGO_PACKAGE failed"
    resolve_server_binary
}

# local/user scope intentionally use absolute executable and workspace paths.
# project scope must be portable, so it derives the checkout root from
# CLAUDE_PROJECT_DIR and uses the executable name from the normal Cargo PATH.
build_launch_cmd() {
    local claude_dir="$1"
    local quoted_repo quoted_mcp quoted_mcp_dir quoted_ra_dir quoted_cargo_bin
    local rel_repo quoted_rel

    if [ "$SCOPE" = "project" ]; then
        case "$MCP_BIN" in
            */*) fail "project scope requires MCP_BIN to be a command name, not a path: $MCP_BIN" ;;
        esac

        rel_repo="$(python3 - "$claude_dir" "$REPO_ROOT" <<'PY'
import os
import sys
print(os.path.relpath(sys.argv[2], sys.argv[1]))
PY
)"
        case "$rel_repo" in
            ..|../*)
                fail "project scope requires the Rust workspace to be inside --claude-dir: $claude_dir"
                ;;
        esac

        printf -v quoted_rel '%q' "$rel_repo"
        printf -v quoted_mcp '%q' "$MCP_BIN"

        printf -v quoted_ra_target '%q' "$RA_TARGET_DIR"

        printf '%s' \
            'export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$HOME/.local/bin:$PATH"; ' \
            "export CARGO_TARGET_DIR=$quoted_ra_target; " \
            'project_root="${CLAUDE_PROJECT_DIR:-$PWD}"; ' \
            "cd \"\$project_root\"/$quoted_rel && exec $quoted_mcp"
        return
    fi

    printf -v quoted_repo '%q' "$REPO_ROOT"
    printf -v quoted_mcp '%q' "$MCP_BIN_PATH"
    printf -v quoted_mcp_dir '%q' "$(dirname "$MCP_BIN_PATH")"
    printf -v quoted_ra_dir '%q' "$(dirname "$RUST_ANALYZER_BIN_PATH")"
    printf -v quoted_cargo_bin '%q' "$CARGO_HOME/bin"

    printf -v quoted_ra_target '%q' "$RA_TARGET_DIR"

    printf 'export PATH=%s:%s:%s:$HOME/.local/bin:$PATH; export CARGO_TARGET_DIR=%s; cd %s && exec %s' \
        "$quoted_mcp_dir" "$quoted_ra_dir" "$quoted_cargo_bin" \
        "$quoted_ra_target" "$quoted_repo" "$quoted_mcp"
}

# Probe one exact registered command. The Python driver:
#   * uses nonblocking reads with a real wall-clock deadline;
#   * waits for initialize before sending initialized and tools/list;
#   * launches from / so an accidental dependence on the caller's cwd is exposed;
#   * supplies CLAUDE_PROJECT_DIR for portable project-scope commands.
probe_server() {
    local claude_dir="$1"
    local launch_cmd
    launch_cmd="$(build_launch_cmd "$claude_dir")"

    python3 - "$launch_cmd" "$claude_dir" <<'PY'
import json
import os
import selectors
import subprocess
import sys
import tempfile
import time

launch_cmd = sys.argv[1]
claude_dir = sys.argv[2]
env = os.environ.copy()
env["CLAUDE_PROJECT_DIR"] = claude_dir

stderr_file = tempfile.TemporaryFile(mode="w+b")
proc = subprocess.Popen(
    ["bash", "-lc", launch_cmd],
    cwd="/",
    env=env,
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=stderr_file,
    bufsize=0,
)

selector = selectors.DefaultSelector()
selector.register(proc.stdout, selectors.EVENT_READ)
buffer = bytearray()

def send(message):
    payload = json.dumps(message, separators=(",", ":")).encode("utf-8") + b"\n"
    proc.stdin.write(payload)
    proc.stdin.flush()

def read_message(deadline):
    while True:
        newline = buffer.find(b"\n")
        if newline >= 0:
            raw = bytes(buffer[:newline]).strip()
            del buffer[: newline + 1]
            if not raw:
                continue
            try:
                return json.loads(raw)
            except json.JSONDecodeError:
                continue

        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise TimeoutError("timed out waiting for MCP response")

        events = selector.select(remaining)
        if not events:
            raise TimeoutError("timed out waiting for MCP response")

        chunk = os.read(proc.stdout.fileno(), 65536)
        if not chunk:
            code = proc.poll()
            raise RuntimeError(f"server stdout closed before the expected response (exit={code})")
        buffer.extend(chunk)

def wait_for_id(request_id, deadline):
    while True:
        message = read_message(deadline)
        if message.get("id") == request_id:
            return message

def stderr_tail():
    stderr_file.flush()
    stderr_file.seek(0)
    data = stderr_file.read().decode("utf-8", errors="replace")
    return data[-4000:].strip()

try:
    deadline = time.monotonic() + 30.0

    send({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "setup_rust_analyzer_mcp",
                "version": "1",
            },
        },
    })

    initialized = wait_for_id(1, deadline)
    if "error" in initialized:
        raise RuntimeError(f"initialize failed: {initialized['error']}")
    if "result" not in initialized:
        raise RuntimeError("initialize response had neither result nor error")

    info = initialized["result"].get("serverInfo", {})
    name = info.get("name", "unknown-server")
    version = info.get("version", "unknown-version")
    print(f"[setup_rust_analyzer_mcp] connected to {name} {version}")

    send({"jsonrpc": "2.0", "method": "notifications/initialized"})
    send({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})

    listed = wait_for_id(2, deadline)
    if "error" in listed:
        raise RuntimeError(f"tools/list failed: {listed['error']}")

    tools = listed.get("result", {}).get("tools")
    if not tools:
        raise RuntimeError("server advertised no tools")

    names = sorted(tool.get("name", "") for tool in tools)
    print(f"[setup_rust_analyzer_mcp] server advertises {len(names)} tools")
    preview = ", ".join(names[:8]) + (", ..." if len(names) > 8 else "")
    print(f"[setup_rust_analyzer_mcp]   {preview}")

    required_any = {
        "rust_analyzer_diagnostics",
        "rust_analyzer_workspace_diagnostics",
    }
    if required_any.isdisjoint(names):
        raise RuntimeError("missing expected rust-analyzer diagnostic tools")
except Exception as ex:
    probe_error = ex
else:
    probe_error = None
finally:
    selector.close()
    if proc.stdin:
        try:
            proc.stdin.close()
        except BrokenPipeError:
            pass
    if proc.poll() is None:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=5)

if probe_error is not None:
    print(f"[setup_rust_analyzer_mcp] probe: {probe_error}", file=sys.stderr)
    tail = stderr_tail()
    if tail:
        print("[setup_rust_analyzer_mcp] server stderr tail:", file=sys.stderr)
        print(tail, file=sys.stderr)
    stderr_file.close()
    sys.exit(1)

stderr_file.close()
PY
}

smoke_test_all() {
    local dir
    for dir in "${CLAUDE_DIRS[@]}"; do
        log "smoke test for sessions launched from $dir"
        probe_server "$dir" || return 1
    done
}

smoke_test() {
    if [ "$DO_SMOKE_TEST" -eq 0 ]; then
        log "skipping smoke test (--no-smoke-test)"
        return
    fi

    log "smoke test: MCP initialize + tools/list"
    if smoke_test_all; then
        return
    fi

    if [ "$DO_INSTALL" -eq 0 ]; then
        fail "server is not usable, and --no-install forbids repairing it"
    fi

    if [ "${FORCE:-0}" = "1" ]; then
        fail "server is not usable even after the forced reinstall performed earlier"
    fi

    warn "server is installed but not usable; reinstalling once"
    reinstall_server

    log "smoke test retry after reinstall"
    smoke_test_all || fail "server is still not usable after reinstalling $CARGO_PACKAGE"
    log "reinstall repaired the server"
}

clear_registrations_for_dir() {
    local claude_dir="$1"
    local scope

    # Remove every same-name entry visible from this launch directory. Otherwise an
    # older local/project entry can shadow a newly added lower-precedence scope.
    for scope in "${ALL_SCOPES[@]}"; do
        (
            cd "$claude_dir"
            claude mcp remove "$SERVER_NAME" -s "$scope" >/dev/null 2>&1
        ) || true
    done
}

register_one() {
    local claude_dir="$1"
    local launch_cmd
    launch_cmd="$(build_launch_cmd "$claude_dir")"

    log "registering '$SERVER_NAME' (scope=$SCOPE) for sessions launched from $claude_dir"
    log "server launch command: bash -lc $(printf '%q' "$launch_cmd")"

    clear_registrations_for_dir "$claude_dir"

    (
        cd "$claude_dir"
        claude mcp add "$SERVER_NAME" -s "$SCOPE" -- bash -lc "$launch_cmd"
    ) || fail "claude mcp add failed for $claude_dir"

    REGISTERED_COUNT=$((REGISTERED_COUNT + 1))
}

print_manual_registration() {
    local dir launch_cmd

    warn "the 'claude' CLI is not on PATH; automatic registration was skipped"
    warn "after installing Claude Code, run the following command(s):"
    printf '\n' >&2
    for dir in "${CLAUDE_DIRS[@]}"; do
        launch_cmd="$(build_launch_cmd "$dir")"
        printf '  (cd %q && claude mcp add %q -s %q -- bash -lc %q)\n' \
            "$dir" "$SERVER_NAME" "$SCOPE" "$launch_cmd" >&2
    done
    printf '\n' >&2
}

register_server() {
    if [ "$DO_REGISTER" -eq 0 ]; then
        log "skipping registration (--no-register)"
        return
    fi

    if ! command -v claude >/dev/null 2>&1; then
        print_manual_registration
        return
    fi

    local dir
    for dir in "${CLAUDE_DIRS[@]}"; do
        register_one "$dir"
    done
}

unregister_one() {
    local claude_dir="$1"
    local scope removed=0

    for scope in "${ALL_SCOPES[@]}"; do
        if (
            cd "$claude_dir"
            claude mcp remove "$SERVER_NAME" -s "$scope" >/dev/null 2>&1
        ); then
            log "removed '$SERVER_NAME' (scope=$scope) registered from $claude_dir"
            removed=1
        fi
    done

    [ "$removed" -eq 1 ] || warn "no '$SERVER_NAME' registration found under $claude_dir"
}

unregister_server() {
    if [ "$DO_REGISTER" -eq 0 ]; then
        log "leaving Claude config alone (--no-register)"
        return
    fi

    if ! command -v claude >/dev/null 2>&1; then
        warn "the 'claude' CLI is not on PATH; cannot unregister automatically"
        warn "delete the '$SERVER_NAME' entry from your Claude MCP configuration manually"
        return
    fi

    local dir
    for dir in "${CLAUDE_DIRS[@]}"; do
        unregister_one "$dir"
    done
}

uninstall_binary() {
    if [ "$DO_INSTALL" -eq 0 ]; then
        log "leaving $MCP_BIN installed (--no-install)"
        return
    fi

    if ! command -v "$MCP_BIN" >/dev/null 2>&1; then
        log "$MCP_BIN is not installed; nothing to uninstall"
        return
    fi

    log "removing $CARGO_PACKAGE with cargo uninstall"
    cargo uninstall "$CARGO_PACKAGE" || fail "cargo uninstall $CARGO_PACKAGE failed"

    if command -v "$MCP_BIN" >/dev/null 2>&1; then
        warn "$MCP_BIN is still on PATH: $(command -v "$MCP_BIN")"
    fi
}

uninstall() {
    log "uninstalling '$SERVER_NAME'"
    unregister_server
    uninstall_binary

    cat <<EOF2

================================================================================
[setup_rust_analyzer_mcp] Uninstalled
================================================================================

A running Claude Code session keeps the MCP connections it opened at startup.
Start a new session before checking that the server has disappeared.

Deliberately not removed:
  * the rustup rust-analyzer component
  * this repository's Cargo target directory

To reinstall:
  ./$(basename "$0")

EOF2
}

print_next_steps() {
    if [ "$REGISTERED_COUNT" -eq 0 ]; then
        cat <<EOF2

================================================================================
[setup_rust_analyzer_mcp] Server installed and validated, but not registered
================================================================================

No Claude MCP registration was written. Either --no-register was supplied or the
'claude' CLI was unavailable. Use the manual command printed above, or rerun this
script after Claude Code is available.

Rust workspace validated by the smoke test:
    $REPO_ROOT

EOF2
        return
    fi

    cat <<EOF2

================================================================================
[setup_rust_analyzer_mcp] Setup completed; start a new Claude Code session
================================================================================

Claude Code binds MCP servers when a session starts. The session that ran this
script will not gain '$SERVER_NAME' dynamically.

1. Start a new Claude Code session.
2. Verify the registration:

       claude mcp list

   Expected: '$SERVER_NAME' reports OK/Connected.
3. Exercise rust-analyzer itself with hover or definition on a known Rust symbol.
   The first call may return null while indexing; retry after rust-analyzer settles.
   Use 'cargo check' as the compile gate.

Registered for sessions launched from:
$(printf '    %s\n' "${CLAUDE_DIRS[@]}")
Rust workspace analyzed by the server:
    $REPO_ROOT

Known rust-analyzer-mcp v0.2.0 limitations:
  * Files are opened once and no didChange notification is sent. Position-based
    answers become stale after edits until a new MCP server/session is started.
  * rust_analyzer_workspace_diagnostics returns an unexpected/null response.
  * rust_analyzer_format may return null when no formatting changes are needed.

EOF2
}

main() {
    parse_args "$@"

    if [ "$DO_UNINSTALL" -eq 1 ]; then
        if [ "$DO_INSTALL" -eq 1 ]; then
            command -v cargo >/dev/null 2>&1 ||
                fail "cargo not found on PATH; rerun with --no-install to unregister only"
        fi
        uninstall
        log "done"
        return
    fi

    verify_workspace
    verify_python
    verify_rust_tools
    check_ripgrep
    check_cargo_artifacts
    install_server

    smoke_test
    register_server
    print_next_steps
    log "done"
}

main "$@"
