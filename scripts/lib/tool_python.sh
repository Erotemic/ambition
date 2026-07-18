#!/usr/bin/env bash
# Shared Python selection helpers for Ambition's isolated authoring tools.
#
# Each tool owns its own .venv. A tool-specific override wins, followed by the
# legacy generic PYTHON override when allowed, then the tool-local interpreter.

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
    local override_value="${!override_name:-}"

    if [[ -n "$override_value" ]]; then
        printf '%s\n' "$override_value"
    elif [[ "$allow_generic_python" == "1" && -n "${PYTHON:-}" ]]; then
        printf '%s\n' "$PYTHON"
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
