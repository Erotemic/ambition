#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
python draw_sandbag_spritesheet.py --copy-to-sandbox
