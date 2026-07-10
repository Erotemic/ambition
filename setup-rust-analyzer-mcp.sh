#!/usr/bin/env bash
# Install and register the rust-analyzer-mcp server
# (https://github.com/zeenix/rust-analyzer-mcp)
# so Claude Code can query this repository's Rust language-server state directly:
# diagnostics, hover/type info, definitions, references, completions, formatting,
# and code actions.
#
# This complements your normal Rust setup. It assumes Rust/Cargo are already
# installed and will install the rust-analyzer component plus the MCP server.
#
# The MCP server is launched from this repository root via a small `bash -lc`
# wrapper, so rust-analyzer sees the intended Cargo workspace regardless of
# where Claude Code itself was launched from.

set -Eeuo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
export CARGO_HOME
export RUSTUP_HOME
export PATH="$CARGO_HOME/bin:$HOME/.local/bin:$PATH"

SERVER_NAME="${SERVER_NAME:-rust-analyzer}"
MCP_BIN="${MCP_BIN:-rust-analyzer-mcp}"
CARGO_PACKAGE="${CARGO_PACKAGE:-rust-analyzer-mcp}"

SCOPE="local"
DO_REGISTER=1
DO_SMOKE_TEST=1
DO_INSTALL=1
CLAUDE_DIRS=()

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
    cat <<'EOF'
Usage: ./setup_rust_analyzer_mcp.sh [options]

Options:
  --claude-dir DIR   Directory you launch Claude Code from. Repeatable.
                     `local` scope is keyed to this directory, so the server is only
                     visible in sessions started there. Defaults to this repo root.
  --scope SCOPE      local (default), project, or user.
                       local   - private to you, per --claude-dir, not committed
                       project - writes .mcp.json in --claude-dir, shared via git
                       user    - visible in every project on this machine
  --no-install       Verify prerequisites only; do not run cargo install.
  --no-register      Install/verify prerequisites only; do not touch Claude config.
  --no-smoke-test    Skip the stdio handshake check.
  -h, --help         Show this help.

Environment:
  SERVER_NAME        MCP server name registered with Claude. Default: rust-analyzer
  MCP_BIN            Binary to run. Default: rust-analyzer-mcp
  CARGO_PACKAGE      Cargo package to install. Default: rust-analyzer-mcp
  FORCE=1            Reinstall the MCP package with `cargo install --force`.

Examples:
  ./setup_rust_analyzer_mcp.sh
  ./setup_rust_analyzer_mcp.sh --claude-dir ../..          # superproject checkout
  ./setup_rust_analyzer_mcp.sh --scope user
  FORCE=1 ./setup_rust_analyzer_mcp.sh
EOF
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

    # Resolve to absolute paths up front: the launch directory is what `local` scope is
    # keyed on, so it must be reported accurately even when --no-register skips the
    # code path that would otherwise resolve it.
    local i dir
    for i in "${!CLAUDE_DIRS[@]}"; do
        dir="${CLAUDE_DIRS[$i]}"
        [ -d "$dir" ] || fail "--claude-dir does not exist: $dir"
        CLAUDE_DIRS[$i]="$(cd "$dir" && pwd)"
    done
}

verify_workspace() {
    if [ ! -f "$REPO_ROOT/Cargo.toml" ]; then
        fail "$REPO_ROOT does not look like a Rust workspace; no Cargo.toml found"
    fi

    log "Rust workspace: $REPO_ROOT"
}

verify_rust_tools() {
    command -v cargo >/dev/null 2>&1 || fail "cargo not found on PATH. Install Rust first: https://rustup.rs/"
    command -v rustc >/dev/null 2>&1 || fail "rustc not found on PATH. Install Rust first: https://rustup.rs/"

    log "cargo: $(cargo --version)"
    log "rustc: $(rustc --version)"

    if command -v rustup >/dev/null 2>&1; then
        log "ensuring rust-analyzer is installed via rustup"
        rustup component add rust-analyzer
    elif ! command -v rust-analyzer >/dev/null 2>&1; then
        fail "rustup is not installed and rust-analyzer is not on PATH"
    fi

    command -v rust-analyzer >/dev/null 2>&1 || fail "rust-analyzer is not on PATH after installation"
    log "rust-analyzer: $(rust-analyzer --version | head -1)"
}

check_ripgrep() {
    if command -v rg >/dev/null 2>&1; then
        log "ripgrep found; local source searches will be fast"
    else
        warn "ripgrep (rg) not found. Claude can still use rust-analyzer, but source searches will be weaker."
    fi
}

# rust-analyzer indexes on demand. If the workspace has never been checked, the
# first MCP tool call can spend a while downloading/building dependencies, which
# can look like a broken server from inside Claude Code.
check_cargo_artifacts() {
    if [ -d "$REPO_ROOT/target/debug/.fingerprint" ] || [ -d "$REPO_ROOT/target/release/.fingerprint" ]; then
        log "cargo artifacts present under target/"
        return
    fi

    warn "no cargo build/check artifacts found under target/"
    warn "the first MCP tool call may be slow while rust-analyzer indexes dependencies"
    warn "recommended warmup: run 'cargo check' in $REPO_ROOT before using the server"
}

install_server() {
    if [ "$DO_INSTALL" -eq 0 ]; then
        log "skipping cargo install (--no-install)"
        return
    fi

    if command -v "$MCP_BIN" >/dev/null 2>&1 && [ "${FORCE:-0}" != "1" ]; then
        log "$MCP_BIN is already installed: $(command -v "$MCP_BIN")"
        return
    fi

    log "installing $CARGO_PACKAGE with cargo"
    if [ "${FORCE:-0}" = "1" ]; then
        cargo install "$CARGO_PACKAGE" --force
    else
        cargo install "$CARGO_PACKAGE"
    fi

    command -v "$MCP_BIN" >/dev/null 2>&1 || fail "$MCP_BIN was installed but is not on PATH"
}

prefetch_server() {
    command -v "$MCP_BIN" >/dev/null 2>&1 || fail "$MCP_BIN not found on PATH"

    # Do not run `$MCP_BIN --help` here. rust-analyzer-mcp currently treats its
    # first positional argument as a workspace path, so `--help` starts the MCP
    # server with workspace path "--help" and then waits forever on stdin.
    log "server binary: $(command -v "$MCP_BIN")"
}

register_one() {
    # Already validated and made absolute by parse_args.
    local claude_dir="$1"
    local quoted_repo quoted_bin launch_cmd

    printf -v quoted_repo '%q' "$REPO_ROOT"
    printf -v quoted_bin '%q' "$MCP_BIN"
    launch_cmd="cd $quoted_repo && exec $quoted_bin"

    log "registering '$SERVER_NAME' (scope=$SCOPE) for sessions launched from $claude_dir"
    log "server launch command: bash -lc '$launch_cmd'"

    # Re-registering is an error if the name is taken, so drop any prior entry first.
    # A missing entry is the normal case, hence the tolerated failure.
    ( cd "$claude_dir" && claude mcp remove "$SERVER_NAME" -s "$SCOPE" >/dev/null 2>&1 ) || true

    (
        cd "$claude_dir" &&
        claude mcp add "$SERVER_NAME" -s "$SCOPE" -- bash -lc "$launch_cmd"
    ) || fail "claude mcp add failed for $claude_dir"
}

register_server() {
    if [ "$DO_REGISTER" -eq 0 ]; then
        log "skipping registration (--no-register)"
        return
    fi

    if ! command -v claude >/dev/null 2>&1; then
        warn "the 'claude' CLI is not on PATH; skipping automatic registration."
        warn "Add this to your MCP config by hand:"
        cat >&2 <<EOF

  "mcpServers": {
    "$SERVER_NAME": {
      "command": "bash",
      "args": ["-lc", "cd $REPO_ROOT && exec $MCP_BIN"]
    }
  }

EOF
        return
    fi

    local dir
    for dir in "${CLAUDE_DIRS[@]}"; do
        register_one "$dir"
    done
}

# Drive the server over stdio and confirm it completes an MCP handshake and advertises
# tools. stdin is held open until the reply arrives; closing it early shuts the server
# down mid-request, which looks like a hang.
smoke_test() {
    if [ "$DO_SMOKE_TEST" -eq 0 ]; then
        log "skipping smoke test (--no-smoke-test)"
        return
    fi

    log "smoke test: MCP handshake + tools/list"

    python3 - "$REPO_ROOT" "$MCP_BIN" <<'PY' || fail "smoke test failed"
import json
import os
import subprocess
import sys
import time

repo_root = sys.argv[1]
mcp_bin = sys.argv[2]

proc = subprocess.Popen(
    [mcp_bin],
    cwd=repo_root,
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.DEVNULL,
    text=True,
    bufsize=1,
)

def send(msg):
    proc.stdin.write(json.dumps(msg) + "\n")
    proc.stdin.flush()

try:
    send({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name": "setup_rust_analyzer_mcp", "version": "1"},
    }})
    send({"jsonrpc": "2.0", "method": "notifications/initialized"})
    send({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})

    tools = None
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        line = proc.stdout.readline()
        if not line:
            if proc.poll() is not None:
                break
            continue
        line = line.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue
        if msg.get("id") == 1 and "result" in msg:
            info = msg["result"].get("serverInfo", {})
            name = info.get("name", "unknown-server")
            version = info.get("version", "unknown-version")
            print(f"[setup_rust_analyzer_mcp] connected to {name} {version}")
        if msg.get("id") == 2:
            tools = msg.get("result", {}).get("tools")
            break
finally:
    proc.terminate()
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()

if not tools:
    print("[setup_rust_analyzer_mcp] error: server advertised no tools", file=sys.stderr)
    sys.exit(1)

names = sorted(t["name"] for t in tools)
print(f"[setup_rust_analyzer_mcp] server advertises {len(names)} tools")
print("[setup_rust_analyzer_mcp]   " + ", ".join(names[:8]) + (", ..." if len(names) > 8 else ""))

# Current zeenix/rust-analyzer-mcp tool names. Keep this check narrow enough to
# catch a wrong server, but broad enough for harmless additions.
required_any = (
    "rust_analyzer_diagnostics",
    "rust_analyzer_workspace_diagnostics",
)
if not any(name in names for name in required_any):
    print("[setup_rust_analyzer_mcp] error: missing expected rust-analyzer diagnostic tools", file=sys.stderr)
    sys.exit(1)
PY
}

print_next_steps() {
    cat <<EOF

================================================================================
[setup_rust_analyzer_mcp] MANUAL STEPS REQUIRED — the setup is not usable yet
================================================================================

Everything above is done. The following steps cannot be automated, because a
Claude Code session connects to its MCP servers once, at startup. The session you
ran this script from will never see '$SERVER_NAME', no matter what the config says.

  STEP 1 (required) — Start a NEW Claude Code session.

      Terminal CLI:      exit and run 'claude' again.
      VS Code extension: open a new Claude Code chat/session. A full restart of
                         VS Code is normally NOT needed. If the server still does
                         not appear, reload the window:
                           Ctrl/Cmd+Shift+P -> "Developer: Reload Window"

  STEP 2 (required) — Verify the server is connected.

      Run:   claude mcp list
      Want:  $SERVER_NAME: bash -lc ... - OK Connected

      If it is missing, you are launching Claude from a directory this script did
      not register. '$SCOPE' scope is keyed to the launch directory. Re-run with:
          ./setup_rust_analyzer_mcp.sh --claude-dir /path/you/launch/claude
      or register everywhere at once:
          ./setup_rust_analyzer_mcp.sh --scope user

  STEP 3 (recommended) — Confirm rust-analyzer itself responds, not just the server.

      'OK Connected' only means the process started. It does NOT mean
      rust-analyzer has finished indexing this project. In the new session, ask
      Claude to use the MCP tools on a known Rust file:

          "use rust_analyzer_workspace_diagnostics on this workspace"
          "use rust_analyzer_hover on crates/my_crate/src/lib.rs line 10 character 5"

      Expect diagnostics or hover/type information. The FIRST call against a
      large workspace can take a while because rust-analyzer must index crates.
      If it times out, run 'cargo check' once and try again.

--------------------------------------------------------------------------------
Registered for sessions launched from:
$(printf '    %s\n' "${CLAUDE_DIRS[@]}")
Rust workspace launch directory used by the server:
    $REPO_ROOT
--------------------------------------------------------------------------------

Useful tools for Rust work:
    rust_analyzer_diagnostics            diagnostics for a specific file
    rust_analyzer_workspace_diagnostics  diagnostics across the workspace
    rust_analyzer_hover                  type/doc information at a position
    rust_analyzer_definition             go-to-definition at a position
    rust_analyzer_references             find references for a symbol
    rust_analyzer_completion             completion suggestions at a position
    rust_analyzer_code_actions           quick fixes/refactor actions
    rust_analyzer_symbols                file-level symbols

EOF
}

main() {
    parse_args "$@"
    verify_workspace
    verify_rust_tools
    check_ripgrep
    check_cargo_artifacts
    install_server
    prefetch_server
    register_server
    smoke_test
    print_next_steps
    log "done"
}

main "$@"
