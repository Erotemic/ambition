#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"
# `python` first, so an environment that already works keeps using the exact
# interpreter it always did (a conda/venv `python` may carry deps that the
# system `python3` does not). `python3` is a fallback only: most distros no
# longer ship a bare `python`, and without this the wrapper is unrunnable there.
if command -v python >/dev/null 2>&1; then
    exec python scripts/archive_agent_source.py "$@"
fi
exec python3 scripts/archive_agent_source.py "$@"
