#!/usr/bin/env bash
set -euo pipefail

apply=0
if [[ "${1:-}" == "--apply" ]]; then
  apply=1
fi

# Fixed-path cleanup only. This script never globs user scratch files and never
# touches docs/brainstorms/.
paths=(
  docs/recipes/crate-split-plan.md
  docs/recipes/music-generation-balance-notes.md
  docs/recipes/music-generation-pipeline-notes.md
  docs/recipes/music-transition-lab.md
  docs/recipes/music-transition-notes.md
  docs/recipes/procedural-tune-authoring.md
  docs/systems/avian2d-physics-foundation.md
  docs/systems/parry2d-geometry.md
  docs/systems/enemy-collision.md
  docs/systems/moving-platforms.md
  docs/vision/mechanics-expressibility-checklist.md
  docs/archive/superseded-migrations/bevy-math-engine-refactor.md
  docs/archive/superseded-migrations/events-refactor-plan.md
  docs/archive/superseded-migrations/glam-migration.md
  docs/archive/superseded-migrations/room-layout-refactor.md
  docs/archive/old-system-notes/room-graph-data-model.md
  docs/archive/old-system-notes/rooms-and-camera.md
  scripts/apply_docs_kb_chunk3_cleanup.sh
  scripts/apply_docs_kb_chunk4_cleanup.sh
  scripts/apply_docs_kb_chunk5_cleanup.sh
  scripts/apply_docs_kb_chunk6b_recipe_link_cleanup.sh
)

removable=()
skipped=()
for path in "${paths[@]}"; do
  if git ls-files --error-unmatch "$path" >/dev/null 2>&1; then
    if git diff --quiet -- "$path" && git diff --cached --quiet -- "$path"; then
      removable+=("$path")
    else
      skipped+=("$path (has local changes)")
    fi
  else
    skipped+=("$path (missing or untracked)")
  fi
done

printf 'Candidate clean tracked stale docs/scripts: %s\n' "${#removable[@]}"
if ((${#removable[@]})); then
  printf '%s\n' "${removable[@]}"
fi
if ((${#skipped[@]})); then
  printf '\nSkipped: %s\n' "${#skipped[@]}"
  printf '%s\n' "${skipped[@]}"
fi

if ((apply)); then
  if ((${#removable[@]})); then
    git rm -f -- "${removable[@]}"
  fi
  python scripts/generate_agent_index.py
  python scripts/check_agent_kb.py
else
  printf '\nDry run only. Re-run with --apply to remove the fixed-path stale docs.\n'
fi
