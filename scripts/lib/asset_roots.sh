#!/usr/bin/env bash
# Read generated asset locations from the selected content library.

ambition_repo_root() {
    (cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
}

AMBITION_ASSETS_ROOT="$(python3 - "$(ambition_repo_root)" <<'PY'
from pathlib import Path
import sys
repo = Path(sys.argv[1])
sys.path.insert(0, str(repo / "scripts"))
from lib.content_libraries import generated_root
print(generated_root(repo))
PY
)"
AMBITION_MUSIC_PUBLISH_ROOT="${AMBITION_MUSIC_PUBLISH_ROOT:-${AMBITION_ASSETS_ROOT}/audio/music/generated}"
export AMBITION_ASSETS_ROOT AMBITION_MUSIC_PUBLISH_ROOT
