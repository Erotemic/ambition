#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

# Prefer the active environment's `python`, but only when it contains the
# git-well archive hook API this wrapper is built on. Fall back to `python3`
# when that interpreter is the provisioned one.
for candidate in python python3; do
    if ! command -v "$candidate" >/dev/null 2>&1; then
        continue
    fi
    if "$candidate" -c \
        'from git_well.git_archive_source import ArchiveSourceContext, archive_source' \
        >/dev/null 2>&1; then
        exec "$candidate" scripts/archive_agent_source.py "$@"
    fi
done

cat >&2 <<'EOF'
archive_agent_source.sh requires git-well with programmatic archive hooks.
Install/update it in the Python environment used for this repository, e.g.:

    python -m pip install -U 'git_well>=0.3.4'

A local editable install of the updated git-well checkout also works.
EOF
exit 1
