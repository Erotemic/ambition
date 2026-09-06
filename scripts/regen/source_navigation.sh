#!/usr/bin/env bash
set -Eeuo pipefail

# ⚠ TWO LEVELS UP: this script lives in `scripts/regen/`, not the repo root.
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

skip_ecs=0

usage() {
    cat <<'EOF'
Usage: ./scripts/regen/source_navigation.sh [OPTIONS]

Regenerate source-navigation artifacts.

Options:
  --quick       Skip the comparatively expensive ECS inventory.
  --skip-ecs    Skip the ECS inventory.
  -h, --help    Show this help.

Environment:
  PYTHON        Python interpreter for standard-library-only scripts.
                Defaults to python3, then python.

The ECS inventory uses its PEP 723 inline dependencies through uv when
available; it does not require or modify the repository .venv.
EOF
}

while (($#)); do
    case "$1" in
        --quick | --skip-ecs)
            skip_ecs=1
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
    shift
done

find_python() {
    local candidate

    if [[ -n "${PYTHON:-}" ]]; then
        if command -v "$PYTHON" >/dev/null 2>&1; then
            command -v "$PYTHON"
            return
        fi
        if [[ -x "$PYTHON" ]]; then
            printf '%s\n' "$PYTHON"
            return
        fi
        echo "PYTHON does not name an executable: $PYTHON" >&2
        return 1
    fi

    for candidate in python3 python; do
        if command -v "$candidate" >/dev/null 2>&1; then
            command -v "$candidate"
            return
        fi
    done

    echo "No Python interpreter found." >&2
    return 1
}

python_bin="$(find_python)"

check_python_version() {
    "$python_bin" - <<'PY'
import sys

required = (3, 11)
if sys.version_info < required:
    raise SystemExit(
        f"Python {required[0]}.{required[1]} or newer is required; "
        f"found {sys.version.split()[0]}"
    )
PY
}

run_python() {
    "$python_bin" "$@"
}

replace_directory() {
    local source_dir="$1"
    local destination="$2"
    local backup="${destination}.old.$$"

    rm -rf "$backup"

    if [[ -e "$destination" ]]; then
        mv "$destination" "$backup"
    fi

    if mv "$source_dir" "$destination"; then
        rm -rf "$backup"
    else
        local status=$?
        echo "Failed to install generated directory: $destination" >&2
        rm -rf "$destination"

        if [[ -e "$backup" ]]; then
            mv "$backup" "$destination"
        fi

        return "$status"
    fi
}

regenerate_ecs_inventory() {
    local destination="$repo_root/.agent/ecs_inventory"
    local temporary
    local status

    mkdir -p "$repo_root/.agent"
    temporary="$(mktemp -d "$repo_root/.agent/.ecs_inventory.tmp.XXXXXX")"

    cleanup_ecs_tmp() {
        rm -rf "$temporary"
    }
    trap cleanup_ecs_tmp RETURN

    local -a command

    if command -v uv >/dev/null 2>&1; then
        # ecs_inventory.py declares its tree-sitter dependencies with PEP 723.
        # --no-project prevents uv from syncing or using the repository .venv.
        # --isolated prevents unrelated installed packages from affecting it.
        command=(
            uv run
            --isolated
            --no-project
            --script "$repo_root/scripts/ecs_inventory.py"
        )
    elif "$python_bin" -c \
        'import tree_sitter, tree_sitter_rust' \
        >/dev/null 2>&1
    then
        command=("$python_bin" "$repo_root/scripts/ecs_inventory.py")
    else
        cat >&2 <<EOF
Cannot regenerate the ECS inventory.

Install uv, or provide a Python environment containing:
  tree-sitter>=0.25,<0.26
  tree-sitter-rust>=0.24,<0.25

To regenerate everything else:
  ./scripts/regen/source_navigation.sh --skip-ecs
EOF
        return 2
    fi

    set +e
    PYTHONFAULTHANDLER=1 "${command[@]}" \
        --repo-root "$repo_root" \
        --workspace \
        --out-dir "$temporary"
    status=$?
    set -e

    if ((status != 0)); then
        echo >&2
        if ((status >= 128)); then
            echo \
                "ECS inventory terminated by signal $((status - 128)) " \
                "(exit status $status)." >&2
        else
            echo "ECS inventory failed with status $status." >&2
        fi
        echo "The existing ECS inventory was left untouched." >&2
        return "$status"
    fi

    replace_directory "$temporary" "$destination"
    trap - RETURN
}

check_python_version

echo "Regenerating module maps..."
run_python scripts/modules_md.py --write

if ((skip_ecs)); then
    echo "Skipping ECS inventory."
else
    echo "Regenerating ECS navigation..."
    regenerate_ecs_inventory
fi

echo "Regenerating agent indexes..."
run_python scripts/generate_agent_index.py

echo "Checking generated navigation..."
run_python scripts/check_agent_kb.py

echo "Source navigation regenerated successfully."
