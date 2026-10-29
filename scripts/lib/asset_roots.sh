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

ambition_library_producer() {
    python3 - "$(ambition_repo_root)" "$@" <<'PYTHON'
from pathlib import Path
import json, sys
repo = Path(sys.argv[1])
sys.path.insert(0, str(repo / "scripts"))
from lib.content_libraries import producer, selected_manifest
if sys.argv[2] == "--list":
    declared = json.loads(selected_manifest(repo).read_text()).get("producers", {})
    order = ("fonts", "backgrounds", "sprites", "variants", "music", "sfx")
    unknown = set(declared) - set(order)
    if unknown:
        raise SystemExit(f"Unsupported producers: {sorted(unknown)}")
    print("\n".join(name for name in order if name in declared))
else:
    selected = producer(repo, sys.argv[2])
    value = selected[sys.argv[3]] if len(sys.argv) > 3 else selected
    print(value if isinstance(value, str) else json.dumps(value))
PYTHON
}
