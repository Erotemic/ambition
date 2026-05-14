#!/usr/bin/env bash
# One-shot pipeline for the vanity card animation.
# Run from the repo root or from this directory.
#
# Usage:
#   ./run.sh           — spritesheet cutout animation  (requires display)
#   ./run.sh panels    — extract_panels → compose → demo (old 4-panel flow)
#   ./run.sh preview   — extract_panels → compose → static PNGs (no display)

set -e
cd "$(dirname "$0")"

if [ "${1}" = "panels" ]; then
  echo "=== Stage 1: chroma-key extraction ==="
  python3 extract_panels.py

  echo ""
  echo "=== Stage 2: pose detection & panel composition ==="
  python3 compose.py

  echo ""
  echo "=== Stage 3: interactive demo (4-panel) ==="
  echo "Keys:  Space/Right = advance   R = restart   Escape = quit"
  python3 demo.py

elif [ "${1}" = "preview" ]; then
  echo "=== Stage 1: chroma-key extraction ==="
  python3 extract_panels.py

  echo ""
  echo "=== Stage 2: pose detection & panel composition ==="
  python3 compose.py

  echo ""
  echo "=== Stage 3: static preview (no display needed) ==="
  python3 preview.py
  echo ""
  echo "Open assets/vanity_card/preview/ to see the frames."

else
  echo "=== Spritesheet cutout animation ==="
  echo "Keys:  Space/Right = advance   R = restart   Escape = quit"
  python3 frame_demo.py
fi
