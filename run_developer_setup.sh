#!/usr/bin/env bash
# Idempotently take a fresh Ambition checkout to a runnable desktop setup.
#
# This is a bootstrap and environment-repair command. Normal development does
# not require running it before every asset regeneration; existing tool-local
# virtualenvs are reused directly by regen_sprites.sh and the renderer CLIs.
#
# The default path is intentionally complete:
#   - install Ubuntu/Debian host libraries and offline audio tools;
#   - install Rust plus the developer Cargo utilities used by repo scripts;
#   - initialize every git submodule recursively;
#   - create one Python virtualenv per active authoring tool;
#   - install each tool from its own pyproject metadata;
#   - regenerate backgrounds, sprites, music, and SFX;
#   - fetch and check the desktop game target.
#
# Usage:
#   ./run_developer_setup.sh
#   ./run_developer_setup.sh --skip-system-packages
#   ./run_developer_setup.sh --skip-rust
#   ./run_developer_setup.sh --skip-submodules
#   ./run_developer_setup.sh --skip-python
#   ./run_developer_setup.sh --skip-assets
#   ./run_developer_setup.sh --skip-cargo-check
#
# Environment overrides:
#   AMBITION_TOOL_PYTHON=3.12   Python version used for every tool-local .venv.
#   UV_EXCLUDE_NEWER=YYYY-MM-DD Override the rolling 14-day package cutoff.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

skip_system_packages=0
skip_rust=0
skip_submodules=0
skip_python=0
skip_assets=0
skip_cargo_check=0

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
        --skip-assets) skip_assets=1 ;;
        --skip-cargo-check) skip_cargo_check=1 ;;
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
    if ! have apt-get || ! have dpkg-query; then
        warn "apt-get/dpkg-query not found; skipping Debian/Ubuntu package install"
        return 0
    fi

    local -a required_pkgs=(
        build-essential
        ca-certificates
        clang
        curl
        ffmpeg
        fluid-soundfont-gm
        fluid-soundfont-gs
        fluidsynth
        git
        libasound2-dev
        libfontconfig1-dev
        libsndfile1
        libudev-dev
        libvulkan1
        libwayland-dev
        libx11-dev
        libxcb-shape0-dev
        libxcb-xfixes0-dev
        libxcursor-dev
        libxi-dev
        libxinerama-dev
        libxkbcommon-dev
        libxkbcommon-x11-dev
        libxrandr-dev
        mesa-vulkan-drivers
        mold
        pkg-config
        python3-dev
        python3-venv
        rubberband-cli
        sox
        timgm6mb-soundfont
    )
    local -a optional_pkgs=(musescore-general-soundfont)
    local -a missing_pkgs=()
    local -a missing_optional=()
    local pkg

    for pkg in "${required_pkgs[@]}"; do
        pkg_installed "$pkg" || missing_pkgs+=("$pkg")
    done
    for pkg in "${optional_pkgs[@]}"; do
        pkg_installed "$pkg" || missing_optional+=("$pkg")
    done

    if [ "${#missing_pkgs[@]}" -eq 0 ] && [ "${#missing_optional[@]}" -eq 0 ]; then
        log "host packages already installed"
        return 0
    fi

    local -a apt_cmd
    if [ "$(id -u)" -eq 0 ]; then
        apt_cmd=(apt-get)
    elif have sudo; then
        apt_cmd=(sudo apt-get)
    else
        fatal "host packages are missing and sudo is unavailable: ${missing_pkgs[*]}"
    fi

    log "refreshing apt metadata"
    "${apt_cmd[@]}" update

    if [ "${#missing_pkgs[@]}" -gt 0 ]; then
        log "installing required host packages: ${missing_pkgs[*]}"
        DEBIAN_FRONTEND=noninteractive "${apt_cmd[@]}" install -y "${missing_pkgs[@]}"
    fi

    for pkg in "${missing_optional[@]}"; do
        if apt-cache show "$pkg" >/dev/null 2>&1; then
            log "installing optional host package: $pkg"
            DEBIAN_FRONTEND=noninteractive "${apt_cmd[@]}" install -y "$pkg"
        else
            warn "$pkg is unavailable from the configured apt repositories"
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
    fi

    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        . "$HOME/.cargo/env"
    fi

    have rustup || fatal "rustup is not on PATH after installation"
    rustup toolchain install stable
    rustup default stable
    rustup component add rustfmt clippy llvm-tools-preview
    have cargo || fatal "cargo is not on PATH after Rust setup"

    ensure_cargo_tool cargo-llvm-cov cargo-llvm-cov
    ensure_cargo_tool cargo-modules cargo-modules
    ensure_cargo_tool cargo-sweep cargo-sweep

    log "Rust ready: $(rustc --version)"
}

ensure_cargo_tool() {
    local package="$1"
    local binary="$2"
    if have "$binary"; then
        log "$binary already installed"
    else
        log "installing $package"
        cargo install --locked "$package"
    fi
}

ensure_submodules() {
    if [ "$skip_submodules" -eq 1 ]; then
        log "skipping git submodule setup"
        return 0
    fi
    have git || fatal "git is required for submodule setup"
    [ -f "$repo_root/.gitmodules" ] || return 0

    log "syncing and initializing git submodules recursively"
    git submodule sync --recursive
    git submodule update --init --recursive

    local key path
    while read -r key path; do
        [ -n "$path" ] || continue
        [ -d "$repo_root/$path" ] || fatal "submodule path was not initialized: $path"
        if [ -z "$(find "$repo_root/$path" -mindepth 1 -maxdepth 1 -print -quit)" ]; then
            fatal "submodule path is empty after update: $path"
        fi
    done < <(git config -f "$repo_root/.gitmodules" --get-regexp '^submodule\..*\.path$' || true)
}

ensure_uv() {
    if have uv; then
        log "uv already installed: $(uv --version)"
    else
        have curl || fatal "curl is required to install uv"
        log "installing uv"
        curl -LsSf https://astral.sh/uv/install.sh | sh
        if [ -f "$HOME/.local/bin/env" ]; then
            # shellcheck disable=SC1091
            . "$HOME/.local/bin/env"
        fi
        export PATH="$HOME/.local/bin:$PATH"
        have uv || fatal "uv install did not put uv on PATH"
    fi

    if [ -z "${UV_EXCLUDE_NEWER:-}" ]; then
        local cutoff
        if cutoff="$(date -u -d '14 days ago' +%Y-%m-%d 2>/dev/null)"; then
            export UV_EXCLUDE_NEWER="$cutoff"
            log "UV_EXCLUDE_NEWER=$UV_EXCLUDE_NEWER"
        else
            warn "date -d is unavailable; uv will use repository configuration"
        fi
    fi
    export UV_LINK_MODE="${UV_LINK_MODE:-copy}"
}

tool_python_version() {
    printf '%s\n' "${AMBITION_TOOL_PYTHON:-3.12}"
}

venv_major_minor() {
    "$1" - <<'PY'
import sys
print(f"{sys.version_info.major}.{sys.version_info.minor}")
PY
}

ensure_tool_venv() {
    local project="$1"
    local requested_python="$2"
    local venv_dir="$project/.venv"

    if [ -x "$venv_dir/bin/python" ]; then
        local current_python
        current_python="$(venv_major_minor "$venv_dir/bin/python")"
        if [ "$current_python" != "$requested_python" ]; then
            log "recreating ${project#$repo_root/}/.venv ($current_python -> $requested_python)"
            uv venv --clear --python "$requested_python" "$venv_dir"
        else
            log "reusing ${project#$repo_root/}/.venv (Python $current_python)"
        fi
    else
        if [ -e "$venv_dir" ]; then
            warn "replacing incomplete environment: ${venv_dir#$repo_root/}"
            rm -rf "$venv_dir"
        fi
        log "creating ${project#$repo_root/}/.venv with Python $requested_python"
        uv venv --python "$requested_python" "$venv_dir"
    fi
}

install_tool_project() {
    local relative_project="$1"
    local import_name="$2"
    local editable_target="${3:-.}"
    local project="$repo_root/$relative_project"
    local requested_python
    requested_python="$(tool_python_version)"

    [ -d "$project" ] || fatal "missing tool project: $relative_project (submodule not initialized?)"
    [ -f "$project/pyproject.toml" ] || fatal "missing $relative_project/pyproject.toml"

    ensure_tool_venv "$project" "$requested_python"
    log "installing $relative_project into its own .venv"
    (
        cd "$project"
        uv pip install --python "$project/.venv/bin/python" -e "$editable_target"
    )
    "$project/.venv/bin/python" -c "import $import_name" \
        || fatal "$relative_project installed but '$import_name' is not importable"
}

tool_projects() {
    cat <<'EOF'
tools/ambition_sprite2d_renderer ambition_sprite2d_renderer
tools/ambition_music_renderer ambition_music_renderer
tools/ambition_sfx_renderer ambition_sfx_renderer
tools/ambition_background_renderer ambition_background_renderer
tools/ambition_parallax_renderer ambition_parallax_renderer
tools/ambition_ldtk_tools ambition_ldtk_tools
EOF
}

verify_tool_environments() {
    local relative_project import_name project python_bin
    while read -r relative_project import_name; do
        project="$repo_root/$relative_project"
        python_bin="$project/.venv/bin/python"
        [ -x "$python_bin" ] \
            || fatal "missing $relative_project/.venv; rerun without --skip-python"
        "$python_bin" -c "import $import_name" >/dev/null 2>&1 \
            || fatal "$import_name is not importable from $relative_project/.venv; rerun without --skip-python"
    done < <(tool_projects)
}

ensure_python_tools() {
    if [ "$skip_python" -eq 1 ]; then
        log "skipping Python tool installation"
        return 0
    fi

    ensure_uv

    # Keep every authoring project isolated. The SFX renderer intentionally
    # caps Python below 3.13, and the audio/sprite stacks carry native wheels
    # that should not constrain unrelated tools.
    install_tool_project tools/ambition_sprite2d_renderer ambition_sprite2d_renderer
    install_tool_project tools/ambition_music_renderer ambition_music_renderer '.[all]'
    install_tool_project tools/ambition_sfx_renderer ambition_sfx_renderer
    install_tool_project tools/ambition_background_renderer ambition_background_renderer
    install_tool_project tools/ambition_parallax_renderer ambition_parallax_renderer
    install_tool_project tools/ambition_ldtk_tools ambition_ldtk_tools

    install_scripts_env

    log "Python authoring environments are ready"
}

# The repo-root `.venv` used by `scripts/*.py` (as opposed to the per-tool
# authoring environments above).
#
# `scripts/ecs_inventory.py` — which regenerates the `.agent/ecs_inventory`
# packets an agent navigates by — parses Rust with tree-sitter. Nothing
# installed it, so on a fresh clone that regeneration simply failed with
# ModuleNotFoundError and the committed navigation data silently went stale.
# That is the regen-on-a-fresh-clone invariant, so it belongs in setup.
install_scripts_env() {
    local venv_dir="$repo_root/.venv"
    local requested_python
    requested_python="$(tool_python_version)"
    if [ ! -x "$venv_dir/bin/python" ]; then
        log "creating .venv for scripts/ with Python $requested_python"
        uv venv --python "$requested_python" "$venv_dir"
    fi
    log "installing scripts/ dependencies"
    uv pip install --python "$venv_dir/bin/python" tree_sitter tree_sitter_rust
    "$venv_dir/bin/python" -c "import tree_sitter_rust" \
        || fatal "scripts/.venv installed but 'tree_sitter_rust' is not importable"
}

regenerate_assets() {
    if [ "$skip_assets" -eq 1 ]; then
        log "skipping generated assets"
        return 0
    fi
    if [ "$skip_python" -eq 1 ]; then
        log "checking existing tool-local environments"
        verify_tool_environments
    fi

    log "regenerating all runtime assets"
    "$repo_root/regen_assets.sh"
}

check_desktop_target() {
    if [ "$skip_cargo_check" -eq 1 ]; then
        log "skipping Cargo fetch/check"
        return 0
    fi
    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        . "$HOME/.cargo/env"
    fi
    have cargo || fatal "cargo is required for the desktop target check"

    log "fetching locked Cargo dependencies"
    cargo fetch --locked
    log "checking the desktop game target"
    cargo check --locked -p ambition_app --bin ambition_game_bin
}

install_system_packages
ensure_rust
ensure_submodules
ensure_python_tools
regenerate_assets
check_desktop_target

echo
if [ "$skip_assets" -eq 0 ] && [ "$skip_cargo_check" -eq 0 ]; then
    log "developer setup complete"
    log "the checkout is ready for: ./run_game.sh"
else
    log "selected developer setup phases complete"
    log "rerun without skip flags for the zero-to-runnable setup"
fi
