#!/usr/bin/env bash
# Shared Python selection helpers for Ambition's isolated authoring tools.
#
# Resolution order: a tool-specific override, the legacy generic PYTHON override
# when allowed, this MACHINE's venv for the tool, the tool-local `.venv` in the
# checkout, then a bare `python3`.
#
# ⛔⛔ WHY A PER-MACHINE STORE COMES BEFORE THE IN-REPO `.venv`. This checkout can
# be shared — the agent VM and the desk see one filesystem over virtiofs — and a
# venv is MACHINE state: `pyvenv.cfg` names an absolute interpreter path in one
# user's home. Measured 2026-08-29: all three `tools/*/.venv/pyvenv.cfg` named
# `/home/joncrall/.local/share/uv/...`, so the venv was real for one user and
# interpreter-less for the other, and resolution fell through to a bare
# `python3` WITHOUT the renderer's dependencies. The sprite pipeline then
# published a character wearing another character's art, reporting success.
#
# ⭐ THIS REPO ALREADY SOLVED THIS CLASS ONCE. `scripts/setup/target_bindmount.sh`
# exists because `target/` is machine state in a shared tree, and AGENTS.md
# carries a ⛔⛔ block about the silent damage when it is not bound. A `.venv` is
# the same kind of object and got the opposite treatment.
#
# The store defaults to `$XDG_CACHE_HOME/ambition-tool-venvs/<tool>`; override
# with `AMBITION_TOOL_VENVS`. A single-user checkout is unaffected: no store
# exists, so the in-repo `.venv` is still found.

ambition_python_exists() {
    local python_bin="$1"
    if [[ "$python_bin" == */* ]]; then
        [[ -x "$python_bin" ]]
    else
        command -v "$python_bin" >/dev/null 2>&1
    fi
}

ambition_select_tool_python() {
    local project_dir="$1"
    local override_name="$2"
    local allow_generic_python="${3:-1}"
    # ⚠ An EMPTY override name is legitimate — a caller that only wants the
    # resolution order, with no tool-specific variable, passes "". Indirect
    # expansion on an empty name is a bash error ("invalid variable name"), not
    # an empty result, so it has to be guarded rather than defaulted.
    local override_value=""
    if [[ -n "$override_name" ]]; then
        override_value="${!override_name:-}"
    fi

    local store tool_venv
    store="${AMBITION_TOOL_VENVS:-${XDG_CACHE_HOME:-$HOME/.cache}/ambition-tool-venvs}"
    tool_venv="$store/$(basename -- "$project_dir")/bin/python"

    if [[ -n "$override_value" ]]; then
        printf '%s\n' "$override_value"
    elif [[ "$allow_generic_python" == "1" && -n "${PYTHON:-}" ]]; then
        printf '%s\n' "$PYTHON"
    elif [[ -x "$tool_venv" ]]; then
        printf '%s\n' "$tool_venv"
    elif [[ -x "$project_dir/.venv/bin/python" ]]; then
        printf '%s\n' "$project_dir/.venv/bin/python"
    elif command -v python3 >/dev/null 2>&1; then
        printf '%s\n' python3
    else
        printf '%s\n' python
    fi
}

ambition_require_python_module() {
    local python_bin="$1"
    local module="$2"
    local setup_hint="$3"

    if ! ambition_python_exists "$python_bin"; then
        printf 'python executable not found: %s\n' "$python_bin" >&2
        printf '%s\n' "$setup_hint" >&2
        return 1
    fi
    if ! "$python_bin" -c "import $module" >/dev/null 2>&1; then
        printf '%s is not installed in: %s\n' "$module" "$python_bin" >&2
        printf '%s\n' "$setup_hint" >&2
        return 1
    fi
}
