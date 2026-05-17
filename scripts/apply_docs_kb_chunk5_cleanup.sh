#!/usr/bin/env bash
set -euo pipefail

# Destructive documentation cleanup for chunk 5.
# This script only removes a fixed list of known stale documentation paths.
# It never glob-removes arbitrary untracked user files.

APPLY=0
if [[ "${1:-}" == "--apply" ]]; then
  APPLY=1
fi

REMOVE_PATHS=(
  docs/AGENT_HANDOFF.md
  docs/CURRENT_STATE.md
  docs/GOAL_STATE.md
  docs/lessons_learned.md
  docs/redirects.md
  docs/agent_states/gpt_5_5_20260430.md
  docs/agent_states/gpt_5_5_20260501-v1.md
  docs/agent_states/gpt_5_5_20260501-v2.md
  docs/adr/0002-engine-may-be-bevy-native.md
  docs/systems/interaction-hazard-actor-skeleton.md
  docs/systems/data-driven-manifest.md
  docs/systems/rooms-and-camera.md
  docs/systems/room-graph-data-model.md
  docs/systems/ldtk-runtime-spine.md
  docs/recipes/glam-migration.md
  docs/recipes/bevy-math-engine-refactor.md
  docs/recipes/events-refactor-plan.md
  docs/recipes/room-layout-refactor.md
  docs/recipes/mechanics-checklist.md
  docs/recipes/steam-deck-deploy.md
  scripts/apply_docs_kb_chunk3_cleanup.sh
  scripts/apply_docs_kb_chunk4_cleanup.sh
)

if [[ "$APPLY" -eq 0 ]]; then
  echo "Dry run. Pass --apply to remove the fixed stale doc paths below."
  printf '  %s\n' "${REMOVE_PATHS[@]}"
  exit 0
fi

for path in "${REMOVE_PATHS[@]}"; do
  if git ls-files --error-unmatch "$path" >/dev/null 2>&1; then
    git rm -f --ignore-unmatch "$path"
  elif [[ -e "$path" ]]; then
    rm -rf -- "$path"
  fi
done

# Remove empty compatibility directory if present.
rmdir docs/agent_states 2>/dev/null || true

python scripts/generate_agent_index.py
python scripts/check_agent_kb.py
