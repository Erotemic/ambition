#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--dry-run" ]]; then
  DRY_RUN=1
else
  DRY_RUN=0
fi

# Fixed cleanup list for doc KB chunk 4. This script never searches for or
# removes arbitrary scratch files; it only removes known legacy documentation
# paths that were archived, consolidated, or replaced by routed docs.
remove_paths=(
  docs/AGENT_HANDOFF.md
  docs/CURRENT_STATE.md
  docs/GOAL_STATE.md
  docs/lessons_learned.md
  docs/redirects.md
  docs/agent_states/gpt_5_5_20260430.md
  docs/agent_states/gpt_5_5_20260501-v1.md
  docs/agent_states/gpt_5_5_20260501-v2.md
  docs/agent_states
  docs/recipes/bevy-math-engine-refactor.md
  docs/recipes/crate-foundation-seldom-state-assets-tests.md
  docs/recipes/crate-split-plan.md
  docs/recipes/crate-strategy.md
  docs/recipes/events-refactor-plan.md
  docs/recipes/feature-basement-wave.md
  docs/recipes/fly-and-room-hub.md
  docs/recipes/flying-door-activation.md
  docs/recipes/glam-migration.md
  docs/recipes/input-buffering-feel.md
  docs/recipes/intro-followup-roadmap.md
  docs/recipes/mechanics-checklist.md
  docs/recipes/room-layout-refactor.md
  docs/systems/room-graph-data-model.md
  docs/systems/rooms-and-camera.md
  scripts/apply_docs_kb_chunk3_cleanup.sh
)

run_rm() {
  local path="$1"
  if [[ -d "$path" ]]; then
    if [[ $DRY_RUN -eq 1 ]]; then
      printf 'would remove directory %s\n' "$path"
    else
      if git ls-files --error-unmatch "$path" >/dev/null 2>&1; then
        git rm -r -f --ignore-unmatch "$path"
      else
        rm -rf "$path"
      fi
    fi
  elif [[ -e "$path" ]]; then
    if [[ $DRY_RUN -eq 1 ]]; then
      printf 'would remove file %s\n' "$path"
    else
      if git ls-files --error-unmatch "$path" >/dev/null 2>&1; then
        git rm -f --ignore-unmatch "$path"
      else
        rm -f "$path"
      fi
    fi
  fi
}

for path in "${remove_paths[@]}"; do
  run_rm "$path"
done

if [[ $DRY_RUN -eq 1 ]]; then
  printf 'dry run complete; no files removed\n'
  exit 0
fi

python scripts/generate_agent_index.py
python scripts/check_agent_kb.py
