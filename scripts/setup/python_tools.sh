#!/usr/bin/env bash
# Create the isolated Python environment each authoring tool renders from.
#
# Usage:
#   scripts/setup/python_tools.sh
#   scripts/setup/python_tools.sh --verify   # check existing environments only
#   scripts/setup/python_tools.sh --help
#
# Environment:
#   AMBITION_TOOL_PYTHON=3.12
#   AMBITION_TOOL_VENVS=<dir>   # moves the per-machine venv store
#   UV_EXCLUDE_NEWER=YYYY-MM-DD
#
# ⛔⛔ A VENV IS MACHINE STATE AND THIS CHECKOUT CAN BE SHARED, which is why the
# environments live in a per-machine store rather than in the tree. The full
# reasoning is in `scripts/lib/tool_python.sh`, the ONE place that knows how a
# tool's interpreter is resolved.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
AMBITION_SETUP_LABEL=python-tools
# shellcheck source=../lib/setup_common.sh
. "$repo_root/scripts/lib/setup_common.sh"
# The ONE place that knows how a tool's interpreter is resolved. Sourced rather
# than reimplemented so setup, the regen scripts and the game all agree.
# shellcheck source=../lib/tool_python.sh
. "$repo_root/scripts/lib/tool_python.sh"

skip_python=0
verify_only=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        --verify) verify_only=1 ;;
        -h|--help) setup_usage "$0"; exit 0 ;;
        *) fatal "unknown option: $1" ;;
    esac
    shift
done

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

ambition_tool_venv_dir() {
    local project="$1"
    printf '%s/%s\n' \
        "${AMBITION_TOOL_VENVS:-${XDG_CACHE_HOME:-$HOME/.cache}/ambition-tool-venvs}" \
        "$(basename -- "$project")"
}

ensure_tool_venv() {
    local project="$1"
    local requested_python="$2"
    local venv_dir
    venv_dir="$(ambition_tool_venv_dir "$project")"
    mkdir -p "$(dirname -- "$venv_dir")"
    local label="${venv_dir/#$HOME/~}"

    if [ -x "$venv_dir/bin/python" ]; then
        local current_python
        current_python="$(venv_major_minor "$venv_dir/bin/python")"
        if [ "$current_python" != "$requested_python" ]; then
            log "recreating $label ($current_python -> $requested_python)"
            uv venv --clear --python "$requested_python" "$venv_dir"
        else
            log "reusing $label (Python $current_python)"
        fi
    else
        if [ -e "$venv_dir" ]; then
            warn "replacing incomplete environment: $label"
        fi
        log "creating $label with Python $requested_python"
        uv venv --clear --python "$requested_python" "$venv_dir"
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
    local venv_python
    venv_python="$(ambition_tool_venv_dir "$project")/bin/python"
    log "installing $relative_project into ${venv_python/#$HOME/~}"
    (
        cd "$project"
        uv pip install --python "$venv_python" -e "$editable_target"
        if [ -d "$project/tests" ]; then
            uv pip install --python "$venv_python" pytest
        fi
    )
    "$venv_python" -c "import $import_name" \
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
        # ⚠ Ask `tool_python.sh`, do not reconstruct the path: it is the ONE
        # place that knows the resolution order (per-machine store, then an
        # in-repo `.venv` for checkouts that predate the store). A verifier with
        # its own idea of where the interpreter lives will fail a machine that
        # is actually fine.
        python_bin="$(ambition_select_tool_python "$project" "" 0)"
        ambition_python_exists "$python_bin" \
            || fatal "no interpreter for $relative_project; rerun without --skip-python"
        "$python_bin" -c "import $import_name" >/dev/null 2>&1 \
            || fatal "$import_name is not importable from $python_bin; rerun without --skip-python"
    done < <(tool_projects)
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
    local requested_python venv_dir venv_python
    requested_python="$(tool_python_version)"
    # Same creation policy as every tool-local environment: this used to be its
    # own copy of the logic, and being a copy is how it missed both `--clear`
    # and the interpreter-version check the tool venvs have had all along.
    ensure_tool_venv "$repo_root" "$requested_python"
    # ⛔ ASK THE HELPER FOR THE PATH; DO NOT SPELL IT AGAIN. `ensure_tool_venv`
    # creates this environment in the per-machine store, and a second literal
    # `$repo_root/.venv` here named a directory nothing had created — so every
    # fresh clone died on the next line with "No virtual environment ... for
    # path `.venv/bin/python`", before assets or the desktop check ever ran.
    venv_dir="$(ambition_tool_venv_dir "$repo_root")"
    venv_python="$venv_dir/bin/python"
    log "installing scripts/ dependencies into ${venv_python/#$HOME/~}"
    # `pytest` belongs here for the same reason `tree_sitter_rust` does, and its
    # absence was worse. `scripts/run_tests.py` runs the repo's TWO Python suites
    # as `sys.executable -m pytest` — the goal guard, the test runner, the
    # package-asset guard, the architectural absence contracts, and the whole
    # 149-test LDtk authoring toolchain — and nothing installed it. So on any
    # machine set up by THIS script both jobs failed instantly with "No module
    # named pytest", which `run_tests.py` reports as two red jobs at 0.0s among
    # twenty green ones: a suite whose first line is "if the thing that decides
    # whether the suite is honest is broken, that is the answer" could never run
    # the thing that decides it.
    # ⚠ EVERY ONE OF THESE IS A COLLECTION ERROR WHEN ABSENT, NOT A SKIPPED TEST.
    # `scripts/tests` imports the scripts it tests at module scope, so one
    # missing third-party module takes the whole detached-tools job down at
    # import time: `numpy` and `soundfile` are `scripts/audio_levels.py`, and
    # `rich` is the repository's clickable-`file://` output convention, which
    # `scripts/agent_query.py` — step 2 of the AGENTS.md cold start — needs to
    # print anything at all.
    uv pip install --python "$venv_python" \
        pytest tree_sitter tree_sitter_rust numpy soundfile rich
    # The moveset inspector is imported directly out of `tools/` by
    # `scripts/tests/test_moveset_inspector_renderer.py`, so it belongs in THIS
    # environment rather than one of its own. Installed editable so its
    # dependencies stay its own to declare (today: pyyaml).
    uv pip install --python "$venv_python" -e tools/ambition_moveset_inspector
    local module
    for module in tree_sitter_rust pytest numpy soundfile rich yaml; do
        "$venv_python" -c "import $module" \
            || fatal "$venv_dir installed but '$module' is not importable — the repo's own Python suites cannot run"
    done
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

if [ "$verify_only" -eq 1 ]; then
    verify_tool_environments
    log "existing tool environments are usable"
else
    ensure_python_tools
fi
