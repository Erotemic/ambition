#!/usr/bin/env bash
set -euo pipefail

# Remove stale recipe fragments that are no longer part of the current recipe index.
# This script only removes this fixed list. It does not glob or touch scratch files.

APPLY=0
if [[ "${1:-}" == "--apply" ]]; then
  APPLY=1
fi

REMOVE_PATHS=(
  docs/recipes/crate-strategy.md
  docs/recipes/crate-foundation-seldom-state-assets-tests.md
  docs/recipes/intro-followup-roadmap.md
  docs/recipes/feature-basement-wave.md
  docs/recipes/fly-and-room-hub.md
  docs/recipes/flying-door-activation.md
)

if [[ "$APPLY" -eq 0 ]]; then
  echo "Dry run. Pass --apply to remove these fixed stale recipe fragments if present:"
  printf '  %s\n' "${REMOVE_PATHS[@]}"
  exit 0
fi

for path in "${REMOVE_PATHS[@]}"; do
  if git ls-files --error-unmatch "$path" >/dev/null 2>&1; then
    git rm -f --ignore-unmatch "$path"
  elif [[ -e "$path" ]]; then
    rm -f -- "$path"
  fi
done

python scripts/generate_agent_index.py
python scripts/check_agent_kb.py
