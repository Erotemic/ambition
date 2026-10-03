#!/usr/bin/env bash
# Shared Python selection helpers for Ambition's isolated authoring tools.
#
# Resolution order: a tool-specific override, `AMBITION_PYTHON`, the legacy
# generic PYTHON override when allowed, this MACHINE's venv for the tool, the
# tool-local `.venv` in the checkout, then a bare `python3`.
#
# ⭐ `AMBITION_PYTHON` IS "USE THE INTERPRETER I ALREADY HAVE". A machine that
# keeps one global environment for everything (Jon's desk does: disk is limited
# and a venv per tool duplicates every package) sets it once, for example in
# `~/.bashrc`, and every tool on that machine resolves to it.
# `scripts/setup/python_tools.sh` then installs the tools into it instead of
# creating venvs. It is namespaced, so unlike `PYTHON` it is not ambient state
# from some other program, and it applies to every caller. A tool-specific
# override still wins, for a tool that cannot run on that interpreter (the SFX
# renderer requires Python < 3.13).
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

# Print the interpreter for a tool. Arguments: the tool's project dir, its
# override variable name ("" for none), and whether the generic `PYTHON` counts
# (default 1).
ambition_select_tool_python() {
    ambition_resolve_tool_python "$@"
    printf '%s\n' "$AMBITION_RESOLVED_PYTHON"
}

# The resolution itself: sets `AMBITION_RESOLVED_PYTHON` and, in words,
# `AMBITION_RESOLVED_FROM` — which road chose it. The preflight names that road
# when the interpreter is unfit, so a user learns WHY it was picked.
ambition_resolve_tool_python() {
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

    # ⛔⛔ THE STORE IS KEYED BY DIRECTORY BASENAME, AND A GIT WORKTREE'S
    # BASENAME IS NOT THE REPOSITORY'S. In `.worktrees/agent-worktree1` this
    # resolved `$store/agent-worktree1/bin/python`, which does not exist, so the
    # runner fell back to a bare `python3` and REFUSED the whole suite: "this
    # interpreter cannot run the Python lane … missing: soundfile,
    # tree_sitter_rust". ⇒ The full gate could not run in ANY agent worktree,
    # which is exactly where the agents work. Found 2026-09-03.
    #
    # ⚠ A FALLBACK, NOT A REPLACEMENT. A tool venv is legitimately keyed by its
    # own directory (`tools/ambition_music_renderer` → that venv), so the
    # basename lookup must stay first and win when it exists; this only fires
    # when it does not. `--git-common-dir` names the MAIN worktree's `.git`
    # whichever worktree you are in, so its parent is the repository.
    if [[ ! -x "$tool_venv" ]] && command -v git >/dev/null 2>&1; then
        local common_dir repo_name
        if common_dir="$(git -C "$project_dir" rev-parse --git-common-dir 2>/dev/null)"; then
            common_dir="$(cd -- "$project_dir" && cd -- "$common_dir" && pwd)" || common_dir=""
            if [[ -n "$common_dir" ]]; then
                repo_name="$(basename -- "$(dirname -- "$common_dir")")"
                if [[ -x "$store/$repo_name/bin/python" ]]; then
                    tool_venv="$store/$repo_name/bin/python"
                fi
            fi
        fi
    fi

    if [[ -n "$override_value" ]]; then
        AMBITION_RESOLVED_PYTHON="$override_value"
        AMBITION_RESOLVED_FROM="$override_name"
    elif [[ -n "${AMBITION_PYTHON:-}" ]]; then
        AMBITION_RESOLVED_PYTHON="$AMBITION_PYTHON"
        AMBITION_RESOLVED_FROM="AMBITION_PYTHON"
    elif [[ "$allow_generic_python" == "1" && -n "${PYTHON:-}" ]]; then
        AMBITION_RESOLVED_PYTHON="$PYTHON"
        AMBITION_RESOLVED_FROM="PYTHON"
    elif [[ -x "$tool_venv" ]]; then
        AMBITION_RESOLVED_PYTHON="$tool_venv"
        AMBITION_RESOLVED_FROM="this machine's tool venv (AMBITION_PYTHON unset)"
    elif [[ -x "$project_dir/.venv/bin/python" ]]; then
        AMBITION_RESOLVED_PYTHON="$project_dir/.venv/bin/python"
        AMBITION_RESOLVED_FROM="the tool's in-repo .venv (AMBITION_PYTHON unset, no machine tool venv)"
    elif command -v python3 >/dev/null 2>&1; then
        AMBITION_RESOLVED_PYTHON=python3
        AMBITION_RESOLVED_FROM="python3 on PATH (no AMBITION_PYTHON, no tool venv)"
    else
        AMBITION_RESOLVED_PYTHON=python
        AMBITION_RESOLVED_FROM="python on PATH (no AMBITION_PYTHON, no tool venv)"
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

# Check, before any work, that each tool's interpreter has what the tool
# DECLARES, and when one does not, say which interpreter was picked and why,
# what it lacks, and whether the environment the user already has active would
# do — with the command to use it.
#
#   ambition_preflight_tool_pythons <rerun-command> \
#       <project_dir> <override_name> <allow_generic_python> [...more triples]
#
# Returns 0 when every tool is fit. Writes only to stderr.
#
# ⛔ IT SUGGESTS, IT DOES NOT SWITCH. An active venv is ambient state, and a
# pipeline that quietly picked whatever was activated would publish from a
# different interpreter depending on the terminal. The fix it names is
# `AMBITION_PYTHON`, which the user sets on purpose.
ambition_preflight_tool_pythons() {
    local rerun="$1"
    shift
    local checker
    checker="$(dirname -- "${BASH_SOURCE[0]}")/tool_requirements.py"

    local -a projects=() overrides=() chosen=()
    local failed=0 report="" project override allow out
    while [[ "$#" -ge 3 ]]; do
        project="$1" override="$2" allow="$3"
        shift 3
        projects+=("$project")
        overrides+=("$override")
        ambition_resolve_tool_python "$project" "$override" "$allow"
        chosen+=("$AMBITION_RESOLVED_PYTHON")
        local label="${project#"$PWD"/}"
        if ! ambition_python_exists "$AMBITION_RESOLVED_PYTHON"; then
            report+="  tool        : $label"$'\n'
            report+="  interpreter : $AMBITION_RESOLVED_PYTHON — not found"$'\n'
            report+="  chosen by   : $AMBITION_RESOLVED_FROM"$'\n\n'
            failed=1
            continue
        fi
        if out="$("$AMBITION_RESOLVED_PYTHON" "$checker" "$project" 2>&1)"; then
            continue
        fi
        report+="  tool        : $label"$'\n'
        report+="  interpreter : $AMBITION_RESOLVED_PYTHON"$'\n'
        report+="  chosen by   : $AMBITION_RESOLVED_FROM"$'\n'
        report+="  lacks       : $(ambition_summarize_requirement_problems "$out")"$'\n\n'
        failed=1
    done
    [[ "$failed" -eq 0 ]] && return 0

    printf '⛔ Python preflight failed — nothing was run.\n\n%s' "$report" >&2

    # Candidates the user already has: the active venv first.
    local -a candidates=()
    local cand seen c
    for cand in \
        "${VIRTUAL_ENV:+$VIRTUAL_ENV/bin/python}" \
        "${CONDA_PREFIX:+$CONDA_PREFIX/bin/python}" \
        "$(command -v python3 2>/dev/null)" \
        "$(command -v python 2>/dev/null)"; do
        [[ -n "$cand" && -x "$cand" ]] || continue
        seen=0
        for c in "${candidates[@]}" "${chosen[@]}"; do
            [[ "$c" == "$cand" ]] && seen=1
        done
        [[ "$seen" -eq 0 ]] && candidates+=("$cand")
    done

    local -a rel_projects=()
    for project in "${projects[@]}"; do
        rel_projects+=("${project#"$PWD"/}")
    done

    local first_unfit="" first_unfit_out=""
    for cand in "${candidates[@]}"; do
        # ⚠ Captured this way so a caller under `set -e` survives a non-zero.
        local status=0
        out="$("$cand" "$checker" "${projects[@]}" 2>&1)" || status=$?
        local prefix="AMBITION_PYTHON=$cand" v
        for v in "${overrides[@]}"; do
            [[ -n "$v" && -n "${!v:-}" ]] && prefix+=" $v=$cand"
        done
        if [[ "$status" -eq 0 ]]; then
            printf 'Your environment at %s has everything these tools need. Run:\n\n' "$cand" >&2
            printf '    %s %s\n\n' "$prefix" "$rerun" >&2
            printf 'To make it the default here, put this in your shell profile:\n\n' >&2
            printf '    export AMBITION_PYTHON=%s\n' "$cand" >&2
            return 1
        fi
        if [[ "$status" -eq 1 ]] && ! grep -qv '^uninstalled' <<<"$out"; then
            local install
            if command -v uv >/dev/null 2>&1; then
                install="uv pip install --python $cand"
            else
                install="$cand -m pip install"
            fi
            for project in "${rel_projects[@]}"; do
                install+=" -e $project"
            done
            printf 'Your environment at %s has every dependency, but not the tools themselves.\n' "$cand" >&2
            printf 'Install them (editable; nothing else changes), then rerun with it:\n\n' >&2
            printf '    %s\n' "$install" >&2
            printf '    %s %s\n\n' "$prefix" "$rerun" >&2
            printf 'To make it the default here, put this in your shell profile:\n\n' >&2
            printf '    export AMBITION_PYTHON=%s\n' "$cand" >&2
            return 1
        fi
        if [[ -z "$first_unfit" ]]; then
            first_unfit="$cand"
            first_unfit_out="$out"
        fi
    done

    printf 'Fix: run ./run_developer_setup.sh to provision the tool venvs' >&2
    if [[ -n "$first_unfit" ]]; then
        printf '.\n\nYour environment at %s would not do as it is; it lacks:\n    %s\n' \
            "$first_unfit" "$(ambition_summarize_requirement_problems "$first_unfit_out")" >&2
        printf 'To install every tool and its dependencies into it instead:\n\n' >&2
        printf '    AMBITION_PYTHON=%s ./scripts/setup/python_tools.sh\n' "$first_unfit" >&2
    else
        printf ', or install every tool into one interpreter with\n' >&2
        printf '    AMBITION_PYTHON=<python> ./scripts/setup/python_tools.sh\n' >&2
    fi
    return 1
}

# One line from `tool_requirements.py` output: "numpy>=1.24, Pillow>=9, …".
ambition_summarize_requirement_problems() {
    local out="$1" line kind detail summary=""
    while IFS= read -r line; do
        [[ -n "$line" ]] || continue
        if [[ "$line" == *$'\t'*$'\t'* ]]; then
            kind="${line%%$'\t'*}"
            detail="${line##*$'\t'}"
            case "$kind" in
                uninstalled) detail="the $detail package itself" ;;
            esac
        else
            detail="$line"
        fi
        summary+="${summary:+, }$detail"
    done <<<"$out"
    printf '%s' "$summary"
}
