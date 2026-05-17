#!/usr/bin/env bash
set -euo pipefail

# Safe cleanup helper for docs KB chunk 3.
# Default mode is DRY RUN. Pass --apply to remove only clean, tracked
# compatibility stubs from the old flat docs/*.md layout.
#
# This script never removes arbitrary untracked scratch files. It only considers
# the explicit docs/*.md paths listed below.

mode="dry-run"
if [[ "${1:-}" == "--apply" ]]; then
  mode="apply"
elif [[ "${1:-}" != "" ]]; then
  echo "Usage: $0 [--apply]" >&2
  exit 2
fi

paths=(
  docs/PROGRESSION_LOG.md
  docs/STEAM_DECK_DEPLOY_NOTES.md
  docs/ability_subset.md
  docs/ability_system.md
  docs/adding_a_showcase_room.md
  docs/ai_generation_contract.md
  docs/android_build.md
  docs/android_power_plan.md
  docs/asset_manager.md
  docs/audio_particles.md
  docs/audio_underwater.md
  docs/avian2d_physics_foundation.md
  docs/bevy_math_engine_refactor.md
  docs/blink_and_fastfall.md
  docs/blink_motion_policy.md
  docs/boss_behavior_profiles.md
  docs/boss_encounter_architecture.md
  docs/camera_and_visual_profiles.md
  docs/character_ai_refactor.md
  docs/code_structure.md
  docs/core_and_bevy_boundary.md
  docs/crate_foundation_seldom_state_assets_tests.md
  docs/crate_split_plan.md
  docs/crate_strategy.md
  docs/data_driven_manifest.md
  docs/developer_tools.md
  docs/display_modes.md
  docs/endgame_sandbox.md
  docs/enemy_collision.md
  docs/engine_architecture.md
  docs/events_refactor_plan.md
  docs/factions.md
  docs/feature_basement_wave.md
  docs/fly_and_room_hub.md
  docs/flying_door_activation.md
  docs/fundsp_audio.md
  docs/game_mode_pause.md
  docs/gameplay_effects.md
  docs/gameplay_trace_recorder.md
  docs/glam_migration.md
  docs/headless_simulation.md
  docs/input_buffering_feel.md
  docs/input_model.md
  docs/interaction_hazard_actor_skeleton.md
  docs/intro_autonomous_followup_prompt.md
  docs/intro_autonomous_followup_prompt_v3.md
  docs/intro_followup_roadmap.md
  docs/intro_handoff_to_next_agent.md
  docs/ldtk_authoring.md
  docs/ldtk_hot_reload.md
  docs/ldtk_runtime_spine.md
  docs/ldtk_world_composition.md
  docs/mechanics_checklist.md
  docs/menu_navigation.md
  docs/mob_lab.md
  docs/mobile_touch_controls.md
  docs/moving_platforms.md
  docs/music_generation_balance_notes.md
  docs/music_generation_pipeline_notes.md
  docs/music_transition_lab.md
  docs/music_transition_notes.md
  docs/parallax_backgrounds.md
  docs/parry2d_geometry.md
  docs/path_forward.md
  docs/pause_menu_settings.md
  docs/procedural_ambience.md
  docs/procedural_tune_authoring.md
  docs/profiling.md
  docs/progression_systems_2026-05-05.md
  docs/room_graph_data_model.md
  docs/room_layout_refactor.md
  docs/rooms_and_camera.md
  docs/save_and_settings.md
  docs/settings_system.md
  docs/tech_debt_log.md
  docs/technical_debt_large_file_refactors.md
  docs/testing_strategy.md
  docs/time_reference_platform.md
  docs/transition_spawn_validation.md
  docs/two_clock_simulation.md
  docs/web_audio_manual_test.md
  docs/web_build.md
)

tracked_clean=()
skipped_dirty=()
skipped_missing_or_untracked=()

for path in "${paths[@]}"; do
  if ! git ls-files --error-unmatch -- "$path" >/dev/null 2>&1; then
    skipped_missing_or_untracked+=("$path")
    continue
  fi
  # Skip anything with unstaged or staged changes.
  if [[ -n "$(git status --porcelain -- "$path")" ]]; then
    skipped_dirty+=("$path")
    continue
  fi
  tracked_clean+=("$path")
done

echo "Candidate clean tracked compatibility stubs: ${#tracked_clean[@]}"
if ((${#tracked_clean[@]})); then
  printf '  %s\n' "${tracked_clean[@]}"
fi

if ((${#skipped_dirty[@]})); then
  echo
  echo "Skipped because they have local changes: ${#skipped_dirty[@]}"
  printf '  %s\n' "${skipped_dirty[@]}"
fi

if ((${#skipped_missing_or_untracked[@]})); then
  echo
  echo "Skipped because missing or not tracked: ${#skipped_missing_or_untracked[@]}"
fi

if [[ "$mode" == "dry-run" ]]; then
  echo
  echo "Dry run only. Re-run with --apply to remove the clean tracked stubs."
  echo "No files were removed."
  exit 0
fi

if ((${#tracked_clean[@]})); then
  git rm -f -- "${tracked_clean[@]}"
fi

python scripts/generate_agent_index.py
python scripts/check_agent_kb.py
