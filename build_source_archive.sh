#!/usr/bin/env bash
# Build a source archive from a clean Git clone of this repo and every
# initialized recursive submodule. Untracked files, ignored files, build
# outputs, and local edits are deliberately excluded.
set -euo pipefail

usage() {
    cat <<'USAGE'
Usage: ./build_source_archive.sh [OPTIONS]

Create a tar.gz archive containing the checked-in source for the current
repository and all initialized recursive submodules.

By default the archive includes Git metadata, so the unpacked tree is a real
Git checkout and `git log` works. Use --history-depth to make that checkout
shallow, or --no-git-history for the old source-only archive behavior.

Options:
  --output-dir DIR      Directory for the generated archive
                        default: current repository root
  --output PATH         Exact archive path to write
  --prefix NAME         Top-level directory name inside the archive
                        default: <repo>-source-<UTC timestamp>-<short sha>
  --history-depth N     Include only the most recent N commits of Git history
                        for the superproject and submodules. By default, the
                        full current-HEAD history is included. Use "full" to
                        request the default explicitly.
  --no-git-history      Do not include .git metadata. This uses git archive and
                        produces a source snapshot only.
  -h, --help            Show this help

Notes:
  - Archives committed/tracked content only.
  - Local uncommitted edits, untracked files, ignored files, and build outputs
    such as target/ are not included.
  - By default, .git metadata is included for the superproject and for each
    initialized recursive submodule.
  - With --history-depth N, the archived repositories are shallow clones whose
    histories stop at the shallow boundary.
  - Submodules must be initialized locally. Run
    `git submodule update --init --recursive` first if needed.
USAGE
}

die() {
    printf 'error: %s\n' "$*" >&2
    exit 2
}

require_value() {
    local opt="$1"
    local value="${2-}"
    if [[ -z "$value" || "$value" == --* ]]; then
        die "$opt requires a value"
    fi
}

validate_history_depth() {
    local value="$1"
    if [[ "$value" == "full" ]]; then
        printf '%s\n' ""
    elif [[ "$value" =~ ^[1-9][0-9]*$ ]]; then
        printf '%s\n' "$value"
    else
        die "--history-depth must be a positive integer or 'full'"
    fi
}

history_depth_label() {
    if [[ -z "$history_depth" ]]; then
        printf 'full\n'
    else
        printf '%s\n' "$history_depth"
    fi
}

clone_args_for_history() {
    # Emit one NUL-separated git-clone argument per output record. This helper
    # keeps --depth handling centralized while preserving spaces in paths in the
    # caller's arrays.
    printf '%s\0' clone --quiet --no-local --single-branch --no-checkout
    if [[ -n "$history_depth" ]]; then
        printf '%s\0' --depth "$history_depth"
    fi
}

checkout_commit() {
    local dst="$1"
    local src="$2"
    local commit="$3"
    local label="$4"

    if git -C "$dst" checkout -q --detach "$commit"; then
        return
    fi

    printf '[source-archive] checkout of %s failed after clone; fetching exact commit\n' "$label" >&2
    if [[ -n "$history_depth" ]]; then
        git -C "$dst" fetch --quiet --depth "$history_depth" origin "$commit" || \
            git -C "$dst" fetch --quiet origin "$commit"
    else
        git -C "$dst" fetch --quiet origin "$commit"
    fi
    git -C "$dst" checkout -q --detach "$commit"

    # Keep src referenced so shellcheck-style tooling does not flag the argument
    # in downstream local edits; the value is useful in error messages above if
    # this function is extended.
    : "$src"
}

clone_committed_checkout() {
    local src="$1"
    local dst="$2"
    local commit="$3"
    local label="$4"
    local -a clone_args

    mkdir -p "$(dirname "$dst")"
    rm -rf "$dst"

    clone_args=()
    while IFS= read -r -d '' arg; do
        clone_args+=("$arg")
    done < <(clone_args_for_history)
    clone_args+=("$src" "$dst")

    git "${clone_args[@]}"
    checkout_commit "$dst" "$src" "$commit" "$label"

    # The archive is for inspection, not local recovery. Expire the clone's
    # fresh reflogs so they do not keep extra objects alive, then repack to make
    # the archived .git directory as small and deterministic as reasonably
    # possible.
    git -C "$dst" reflog expire --expire=now --expire-unreachable=now --all >/dev/null 2>&1 || true
    git -C "$dst" gc --prune=now --quiet >/dev/null 2>&1 || true
}

append_manifest_exclude() {
    local repo="$1"
    if [[ -d "$repo/.git/info" ]]; then
        {
            printf '\n# Added by build_source_archive.sh so the archive manifest does not dirty the checkout.\n'
            printf '/SOURCE_ARCHIVE_MANIFEST.txt\n'
        } >> "$repo/.git/info/exclude"
    fi
}

output_dir=""
output_path=""
prefix=""
history_depth=""
history_depth_was_set=0
include_git_history=1

while (($#)); do
    case "$1" in
        --output-dir)
            require_value "$1" "${2-}"
            output_dir="$2"
            shift 2
            ;;
        --output)
            require_value "$1" "${2-}"
            output_path="$2"
            shift 2
            ;;
        --prefix)
            require_value "$1" "${2-}"
            prefix="$2"
            shift 2
            ;;
        --history-depth)
            require_value "$1" "${2-}"
            history_depth=$(validate_history_depth "$2")
            history_depth_was_set=1
            shift 2
            ;;
        --no-git-history)
            include_git_history=0
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            die "unknown argument: $1"
            ;;
    esac
done

if [[ "$include_git_history" -eq 0 && "$history_depth_was_set" -eq 1 ]]; then
    die "--history-depth cannot be combined with --no-git-history"
fi

repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || die "not inside a Git repository"
cd "$repo_root"

git rev-parse --verify HEAD >/dev/null 2>&1 || die "repository has no HEAD commit to archive"

repo_name=$(basename "$repo_root")
head_sha=$(git rev-parse HEAD)
short_sha=$(git rev-parse --short=12 HEAD)
timestamp=$(date -u +%Y%m%dT%H%M%SZ)

if [[ -z "$prefix" ]]; then
    prefix="${repo_name}-source-${timestamp}-${short_sha}"
fi

if [[ "$prefix" == /* || "$prefix" == *..* ]]; then
    die "prefix must be a simple relative directory name"
fi

if [[ -n "$output_path" ]]; then
    archive_path="$output_path"
    archive_dir=$(dirname "$archive_path")
else
    if [[ -n "$output_dir" ]]; then
        archive_dir="$output_dir"
        archive_path="${archive_dir}/${prefix}.tar.gz"
    else
        archive_dir="."
        archive_path="${prefix}.tar.gz"
    fi
fi

mkdir -p "$archive_dir"

tmpdir=$(mktemp -d "${TMPDIR:-/tmp}/${repo_name}-source-archive.XXXXXX")
cleanup() {
    rm -rf "$tmpdir"
}
trap cleanup EXIT

stage="$tmpdir/stage"
mkdir -p "$stage"

submodule_status_file="$tmpdir/submodule_status.txt"
git submodule status --recursive > "$submodule_status_file" || true

printf '[source-archive] repo: %s\n' "$repo_root"
printf '[source-archive] prefix: %s\n' "$prefix"
printf '[source-archive] git history: %s\n' "$([[ "$include_git_history" -eq 1 ]] && printf 'included' || printf 'omitted')"
if [[ "$include_git_history" -eq 1 ]]; then
    printf '[source-archive] history depth: %s\n' "$(history_depth_label)"
fi
printf '[source-archive] superproject HEAD: %s\n' "$short_sha"

if [[ "$include_git_history" -eq 1 ]]; then
    printf '[source-archive] cloning superproject\n'
    clone_committed_checkout "$repo_root" "$stage/$prefix" "$head_sha" "superproject"
else
    printf '[source-archive] exporting superproject with git archive\n'
    git archive --format=tar --prefix="${prefix}/" HEAD | tar -xf - -C "$stage"
fi

if [[ -s "$submodule_status_file" ]]; then
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        # Format: "<status><sha> <path> (<describe>)". Submodule paths in this
        # repo do not contain spaces; if that ever changes, prefer avoiding
        # spaces in submodule paths or extend this parser.
        path=$(printf '%s\n' "$line" | awk '{print $2}')
        status_char=${line:0:1}
        first_field=$(printf '%s\n' "$line" | awk '{print $1}')
        if [[ "$status_char" == " " ]]; then
            submodule_sha="$first_field"
        else
            submodule_sha=${first_field:1}
        fi
        if [[ -z "$path" || -z "$submodule_sha" ]]; then
            die "could not parse submodule status line: $line"
        fi
        if [[ "$status_char" == "-" ]]; then
            die "submodule '$path' is not initialized; run: git submodule update --init --recursive"
        fi
        if [[ ! -d "$path" ]]; then
            die "submodule path '$path' is missing; run: git submodule update --init --recursive"
        fi
        git -C "$path" rev-parse --verify HEAD >/dev/null 2>&1 || \
            die "submodule '$path' has no checked-out HEAD"
        printf '[source-archive] exporting submodule %s HEAD %s\n' \
            "$path" "$(git -C "$path" rev-parse --short=12 HEAD)"
        if [[ "$include_git_history" -eq 1 ]]; then
            clone_committed_checkout "$repo_root/$path" "$stage/$prefix/$path" "$submodule_sha" "submodule $path"
        else
            mkdir -p "$stage/$prefix/$path"
            git -C "$path" archive --format=tar --prefix="${prefix}/${path}/" HEAD | tar -xf - -C "$stage"
        fi
    done < "$submodule_status_file"
fi

if [[ "$include_git_history" -eq 1 ]]; then
    append_manifest_exclude "$stage/$prefix"
fi

manifest="$stage/$prefix/SOURCE_ARCHIVE_MANIFEST.txt"
{
    printf 'Source archive manifest\n'
    printf '=======================\n\n'
    printf 'Generated UTC: %s\n' "$timestamp"
    printf 'Repository: %s\n' "$repo_name"
    printf 'Repository root: %s\n' "$repo_root"
    printf 'Archive prefix: %s\n' "$prefix"
    printf 'Superproject HEAD: %s\n' "$head_sha"
    printf 'Superproject short HEAD: %s\n' "$short_sha"
    printf 'Git history included: %s\n' "$([[ "$include_git_history" -eq 1 ]] && printf 'yes' || printf 'no')"
    if [[ "$include_git_history" -eq 1 ]]; then
        printf 'Git history depth: %s\n' "$(history_depth_label)"
        if [[ -n "$history_depth" ]]; then
            printf 'Shallow clone: yes\n'
        else
            printf 'Shallow clone: no\n'
        fi
    fi
    printf '\nArchive policy:\n'
    printf '%s\n' '- Includes committed/tracked files from the superproject HEAD.'
    printf '%s\n' '- Includes committed/tracked files from each initialized recursive submodule HEAD.'
    if [[ "$include_git_history" -eq 1 ]]; then
        printf '%s\n' '- Includes .git metadata for the superproject and initialized recursive submodules.'
        printf '%s\n' '- Excludes local edits, untracked files, ignored files, and build outputs.'
    else
        printf '%s\n' '- Excludes local edits, untracked files, ignored files, build outputs, and .git directories.'
    fi
    printf '\nSuperproject status at archive time:\n'
    git status --short --branch || true
    printf '\nRecursive submodule status at archive time:\n'
    if [[ -s "$submodule_status_file" ]]; then
        cat "$submodule_status_file"
    else
        printf '(none)\n'
    fi
    printf '\nSubmodule HEADs included:\n'
    if [[ -s "$submodule_status_file" ]]; then
        while IFS= read -r line; do
            [[ -z "$line" ]] && continue
            path=$(printf '%s\n' "$line" | awk '{print $2}')
            if [[ -n "$path" && -d "$path" ]]; then
                printf '%s %s\n' "$(git -C "$path" rev-parse HEAD)" "$path"
            fi
        done < "$submodule_status_file"
    else
        printf '(none)\n'
    fi
} > "$manifest"

if [[ "$include_git_history" -eq 1 ]]; then
    # Re-run exclusion after writing the manifest so the archived checkout opens
    # cleanly with `git status`.
    append_manifest_exclude "$stage/$prefix"
fi

tar -C "$stage" -czf "$archive_path" "$prefix"

printf '[source-archive] wrote: %s\n' "$archive_path"
printf '[source-archive] list contents: tar -tzf %q | less\n' "$archive_path"
