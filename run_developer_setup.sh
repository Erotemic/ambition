#!/usr/bin/env bash
# Idempotently prepare a local Ambition development checkout.
#
# This script:
#   - installs common host packages when apt-get is available;
#   - installs Rust through rustup when needed;
#   - initializes git submodules;
#   - uses the active virtualenv, or creates .venv at the repo root;
#   - installs the Python asset-generator packages in editable mode.
#
# Usage:
#   ./run_developer_setup.sh
#   ./run_developer_setup.sh --skip-system-packages
#   ./run_developer_setup.sh --skip-rust
#   ./run_developer_setup.sh --skip-submodules
#   ./run_developer_setup.sh --skip-python
#
# Environment overrides:
#   AMBITION_VENV_DIR=.venv
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

skip_system_packages=0
skip_rust=0
skip_submodules=0
skip_python=0

usage() {
    awk '
        NR == 1 { next }
        /^set -euo pipefail$/ { exit }
        /^#$/ { print ""; next }
        /^# / { sub(/^# /, ""); print }
    ' "$0"
}

log() {
    printf '[developer-setup] %s\n' "$*"
}

warn() {
    printf '[developer-setup] warning: %s\n' "$*" >&2
}

fatal() {
    printf '[developer-setup] error: %s\n' "$*" >&2
    exit 1
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --skip-system-packages) skip_system_packages=1 ;;
        --skip-rust) skip_rust=1 ;;
        --skip-submodules) skip_submodules=1 ;;
        --skip-python) skip_python=1 ;;
        -h|--help) usage; exit 0 ;;
        *) fatal "unknown option: $1" ;;
    esac
    shift
done

have() {
    command -v "$1" >/dev/null 2>&1
}

pkg_installed() {
    dpkg-query -W -f='${Status}' "$1" 2>/dev/null | grep -q "install ok installed"
}

install_system_packages() {
    if [ "$skip_system_packages" -eq 1 ]; then
        log "skipping host package install"
        return 0
    fi
    if ! have apt-get; then
        warn "apt-get not found; skipping host package install"
        return 0
    fi
    if ! have dpkg-query; then
        warn "dpkg-query not found; skipping host package install"
        return 0
    fi

    local -a required_pkgs=(
        build-essential
        ca-certificates
        # clang + mold are the Rust linker pair `.cargo/config.toml`
        # pins (`linker = "clang"`, `rustflags = [ … -fuse-ld=mold]`).
        # Without these, cargo errors out with `linker `clang` not
        # found` or `cannot find 'ld'` deep inside compilation. Pulled
        # in here so a fresh checkout can compile without manual setup.
        clang
        mold
        # Bevy's windowing / input / audio crates link against these
        # system libraries via pkg-config. Missing any of them surfaces
        # as `Package 'X' not found in the pkg-config search path`
        # deep in the cargo build output.
        libwayland-dev
        libudev-dev
        libasound2-dev
        libxkbcommon-dev
        libfontconfig1-dev
        curl
        ffmpeg
        fluid-soundfont-gm
        fluid-soundfont-gs
        fluidsynth
        git
        libsndfile1
        pkg-config
        python3-dev
        python3-venv
        timgm6mb-soundfont
    )

    local -a optional_pkgs=(musescore-general-soundfont)

    local -a missing_pkgs=()
    local pkg
    for pkg in "${required_pkgs[@]}"; do
        if ! pkg_installed "$pkg"; then
            missing_pkgs+=("$pkg")
        fi
    done

    local -a missing_optional=()
    for pkg in "${optional_pkgs[@]}"; do
        pkg_installed "$pkg" && continue
        missing_optional+=("$pkg")
    done

    if [ "${#missing_pkgs[@]}" -eq 0 ] && [ "${#missing_optional[@]}" -eq 0 ]; then
        log "host packages already installed; skipping apt-get"
        return 0
    fi

    local -a apt_cmd
    if [ "$(id -u)" -eq 0 ]; then
        apt_cmd=(apt-get)
    elif have sudo; then
        apt_cmd=(sudo apt-get)
    else
        warn "sudo not found; skipping host package install (missing: ${missing_pkgs[*]} ${missing_optional[*]})"
        return 0
    fi

    log "installing host packages via apt-get: ${missing_pkgs[*]} ${missing_optional[*]}"
    "${apt_cmd[@]}" update

    if [ "${#missing_pkgs[@]}" -gt 0 ]; then
        DEBIAN_FRONTEND=noninteractive "${apt_cmd[@]}" install -y "${missing_pkgs[@]}"
    fi

    for pkg in "${missing_optional[@]}"; do
        if apt-cache show "$pkg" >/dev/null 2>&1; then
            DEBIAN_FRONTEND=noninteractive "${apt_cmd[@]}" install -y "$pkg"
        else
            warn "$pkg not available from apt; continuing"
        fi
    done
}

ensure_rust() {
    if [ "$skip_rust" -eq 1 ]; then
        log "skipping Rust setup"
        return 0
    fi

    if ! have rustup; then
        have curl || fatal "curl is required to install rustup"
        log "installing rustup and the stable Rust toolchain"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
            | sh -s -- -y --profile default --default-toolchain stable
    else
        log "rustup already installed"
    fi

    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        . "$HOME/.cargo/env"
    fi

    have rustup || fatal "rustup is still not on PATH; source \$HOME/.cargo/env and rerun"
    rustup toolchain install stable
    rustup default stable
    rustup component add rustfmt clippy
    rustup component add llvm-tools-preview

    have cargo || fatal "cargo is not on PATH after Rust setup"

    cargo install cargo-llvm-cov
    cargo install cargo-modules
    cargo install cargo-sweep

    log "Rust ready: $(rustc --version)"
}

ensure_submodules() {
    if [ "$skip_submodules" -eq 1 ]; then
        log "skipping git submodule setup"
        return 0
    fi
    have git || fatal "git is required for submodule setup"
    if [ ! -f "$repo_root/.gitmodules" ]; then
        log "no .gitmodules file; skipping submodules"
        return 0
    fi

    log "syncing and initializing git submodules"
    git submodule sync --recursive
    git submodule update --init --recursive
}

python_version_ok() {
    "$1" - "$2" <<'PY'
import sys

required = tuple(int(part) for part in sys.argv[1].split("."))
raise SystemExit(0 if sys.version_info[:2] >= required else 1)
PY
}

find_base_python() {
    local candidate
    for candidate in python3.12 python3.11 python3 python; do
        if have "$candidate" && python_version_ok "$candidate" "3.11"; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done
    return 1
}

ensure_uv() {
    if have uv; then
        log "uv already installed: $(uv --version)"
    else
        have curl || fatal "curl is required to install uv"
        log "installing uv via astral.sh installer"
        curl -LsSf https://astral.sh/uv/install.sh | sh
        if [ -f "$HOME/.local/bin/env" ]; then
            # shellcheck disable=SC1091
            . "$HOME/.local/bin/env"
        fi
        if ! have uv && [ -x "$HOME/.local/bin/uv" ]; then
            export PATH="$HOME/.local/bin:$PATH"
        fi
        have uv || fatal "uv install did not put uv on PATH; restart shell and retry"
        log "uv ready: $(uv --version)"
    fi

    # Supply-chain floor: ignore packages published in the last 14 days unless
    # the caller has already set UV_EXCLUDE_NEWER. Gives the community time to
    # catch malicious releases before we resolve them.
    if [ -z "${UV_EXCLUDE_NEWER:-}" ]; then
        local cutoff
        if cutoff="$(date -u -d '14 days ago' +%Y-%m-%d 2>/dev/null)"; then
            export UV_EXCLUDE_NEWER="$cutoff"
            log "UV_EXCLUDE_NEWER=$UV_EXCLUDE_NEWER (14-day supply-chain floor)"
        else
            warn "date -d unavailable; falling back to uv.toml's static exclude-newer"
        fi
    else
        log "UV_EXCLUDE_NEWER=$UV_EXCLUDE_NEWER (inherited from environment)"
    fi
}

ensure_python() {
    if [ "$skip_python" -eq 1 ]; then
        log "skipping Python package setup"
        return 0
    fi

    ensure_uv

    local python_bin venv_dir
    if [ -n "${VIRTUAL_ENV:-}" ]; then
        python_bin="$VIRTUAL_ENV/bin/python"
        [ -x "$python_bin" ] || fatal "VIRTUAL_ENV is set but no python exists at $python_bin"
        python_version_ok "$python_bin" "3.11" || fatal "active virtualenv must use Python >= 3.11"
        log "using active virtualenv: $VIRTUAL_ENV"
        venv_dir="$VIRTUAL_ENV"
    else
        venv_dir="${AMBITION_VENV_DIR:-$repo_root/.venv}"
        if [ ! -x "$venv_dir/bin/python" ]; then
            log "creating virtualenv via uv: $venv_dir"
            uv venv --python ">=3.11" "$venv_dir"
        else
            log "using existing virtualenv: $venv_dir"
        fi
        python_bin="$venv_dir/bin/python"
    fi

    local projects=(
        "$repo_root/tools/ambition_sprite2d_renderer"
        "$repo_root/tools/ambition_music_renderer"
        "$repo_root/tools/ambition_parallax_renderer"
        "$repo_root/tools/ambition_background_renderer"
        "$repo_root/tools/ambition_ldtk_tools"
        "$repo_root/tools/ambition_sfx_renderer"
    )

    local project
    for project in "${projects[@]}"; do
        install_python_project "$python_bin" "$venv_dir" "$project"
    done

    log "Python ready: $("$python_bin" --version)"
    log "activate with: source ${venv_dir}/bin/activate"
    log "or run asset scripts with: PYTHON=$python_bin ./regen_assets.sh"
}

project_requires_python() {
    local python_bin="$1"
    local pyproject="$2"
    [ -f "$pyproject" ] || return 0
    "$python_bin" - "$pyproject" <<'PY' 2>/dev/null
import sys, tomllib
with open(sys.argv[1], "rb") as f:
    print(tomllib.load(f).get("project", {}).get("requires-python", ""))
PY
}

python_compatible_with_spec() {
    local python_bin="$1"
    local spec="$2"
    [ -n "$spec" ] || return 0
    "$python_bin" - "$spec" <<'PY' 2>/dev/null
import sys
try:
    from packaging.specifiers import SpecifierSet
    from packaging.version import Version
except ModuleNotFoundError:
    sys.exit(0)
current = Version(".".join(str(p) for p in sys.version_info[:3]))
sys.exit(0 if current in SpecifierSet(sys.argv[1]) else 1)
PY
}

install_in_dedicated_venv() {
    local project="$1"
    local req="$2"
    local project_venv="$project/.venv"

    if [ ! -x "$project_venv/bin/python" ]; then
        log "creating dedicated venv for ${project#$repo_root/} (requires-python='${req}')"
        uv venv --python "$req" "$project_venv"
    else
        log "using existing dedicated venv: ${project_venv#$repo_root/}"
    fi

    log "uv pip install -e ${project#$repo_root/} (into ${project_venv#$repo_root/})"
    VIRTUAL_ENV="$project_venv" uv pip install -e "$project"
    log "activate dedicated venv: source ${project_venv}/bin/activate"
}

install_python_project() {
    local python_bin="$1"
    local venv_dir="$2"
    local project="$3"

    if [ ! -d "$project" ]; then
        warn "missing Python project directory: ${project#$repo_root/}"
        return 0
    fi

    if [ -f "$project/pyproject.toml" ] || [ -f "$project/setup.py" ] || [ -f "$project/setup.cfg" ]; then
        local req=""
        if [ -f "$project/pyproject.toml" ]; then
            req="$(project_requires_python "$python_bin" "$project/pyproject.toml")"
        fi
        if [ -n "$req" ] && ! python_compatible_with_spec "$python_bin" "$req"; then
            install_in_dedicated_venv "$project" "$req"
            return 0
        fi
        log "uv pip install -e ${project#$repo_root/}"
        VIRTUAL_ENV="$venv_dir" uv pip install -e "$project"
        return 0
    fi

    if [ -f "$project/requirements.txt" ]; then
        log "uv pip install -r ${project#$repo_root/}/requirements.txt"
        VIRTUAL_ENV="$venv_dir" uv pip install -r "$project/requirements.txt"
        return 0
    fi

    warn "no Python install metadata found in ${project#$repo_root/}"
}

install_system_packages
ensure_rust
ensure_submodules
ensure_python

echo
log "developer setup complete"
log "try: cargo check --workspace"
log "try: ./regen_assets.sh --help"
