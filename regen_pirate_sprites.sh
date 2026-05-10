#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
python "$repo_root/tools/ambition_sprite2d_renderer/publish_pirate_spritesheets.py" "$@"
