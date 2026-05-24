#!/usr/bin/env bash
# Re-render every sprite asset and install into the sandbox crate.
#
# Covers:
#   - Adapter targets (robot / goblin / boss): re-renders every job in
#     tools/ambition_sprite2d_renderer/configs/ straight into
#     crates/ambition_sandbox/assets/sprites/.
#   - Entity sprites (chest, breakable, door zone, etc.): re-rendered into
#     crates/ambition_sandbox/assets/sprites/entities/.
#   - Standalone pirate sheets: rendered and published into
#     crates/ambition_sandbox/assets/sprites/.
#   - Tack-on targets (sandbag, mockingbird): rendered into the renderer's
#     generated/ dir then installed into crates/ambition_sandbox/assets/sprites/.
#
# Usage:
#   ./regen_sprites.sh   # render + install everything
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

renderer_dir="$repo_root/tools/ambition_sprite2d_renderer"
sprites_dir="$repo_root/crates/ambition_sandbox/assets/sprites"
entities_dir="$sprites_dir/entities"

select_python() {
    if [ -n "${PYTHON:-}" ]; then
        printf '%s\n' "$PYTHON"
    elif [ -n "${VIRTUAL_ENV:-}" ] && [ -x "$VIRTUAL_ENV/bin/python" ]; then
        printf '%s\n' "$VIRTUAL_ENV/bin/python"
    elif [ -x "$repo_root/.venv/bin/python" ]; then
        printf '%s\n' "$repo_root/.venv/bin/python"
    else
        printf '%s\n' python
    fi
}

print_help() {
    awk '
        NR == 1 { next }
        /^set -euo pipefail$/ { exit }
        /^#$/ { print ""; next }
        /^# / { sub(/^# /, ""); print }
    ' "$0"
}

for arg in "$@"; do
    case "$arg" in
        -h|--help) print_help; exit 0 ;;
        *) echo "unknown arg: $arg" >&2; exit 2 ;;
    esac
done

python_bin="$(select_python)"
if ! command -v "$python_bin" >/dev/null 2>&1; then
    echo "python executable not found: $python_bin" >&2
    echo "run ./run_developer_setup.sh, activate a venv, or set PYTHON=/path/to/python" >&2
    exit 1
fi

if ! "$python_bin" -c 'import ambition_sprite2d_renderer' >/dev/null 2>&1; then
    echo "ambition_sprite2d_renderer is not installed in: $python_bin" >&2
    echo "run ./run_developer_setup.sh, activate the configured venv, or set PYTHON=/path/to/python" >&2
    exit 1
fi

echo "==> adapter targets (robot / goblin / boss) → $sprites_dir"
(cd "$renderer_dir" && "$python_bin" -m ambition_sprite2d_renderer draw-all --out-dir "$sprites_dir")

echo "==> entity sprites → $entities_dir"
(cd "$renderer_dir" && "$python_bin" -m ambition_sprite2d_renderer publish entities --dest-root "$entities_dir")

echo "==> review NPC sheets (toon-target NPCs) → $sprites_dir"
# `draw-review` renders configs/review/*.yaml (toon-target NPC
# variants such as absurd_general, architect, kernel_guide). We
# render to a scratch dir, then copy the specific sheets we use
# in-game into $sprites_dir. Promoting a review config to a
# permanent runtime sheet means: add the cue id to the copy list
# below AND register a CharacterSheetSpec for it in
# `crates/ambition_sandbox/src/character_sprites/sheets.rs`, plus
# wire it into `NPC_SPRITE_REGISTRY` in
# `crates/ambition_sandbox/src/character_sprites/assets.rs`.
review_scratch="$renderer_dir/generated/review"
mkdir -p "$review_scratch"
(cd "$renderer_dir" && "$python_bin" -m ambition_sprite2d_renderer draw-review --out-dir "$review_scratch")
review_cues=(
    # Toon-target NPC variants already promoted.
    absurd_general architect kernel_guide vault_keeper
    merchant_prototype oiler erdish fascist_enforcer
    # Named characters whose YAML manifests already live in $sprites_dir.
    alice bob craig eve general_hero judy mallory olivia
    peggy sybil trent trudy victor walter
)
# `ron` is included because the sandbox SheetRegistry parses RON at
# startup (see `presentation::character_sprites::registry`). Without
# the copy step the .ron in $sprites_dir would drift from the
# regenerated .yaml/.png.
for cue in "${review_cues[@]}"; do
    for ext in png yaml ron; do
        src="$review_scratch/${cue}_spritesheet.$ext"
        if [ -f "$src" ]; then
            cp "$src" "$sprites_dir/${cue}_spritesheet.$ext"
            echo "  installed ${cue}_spritesheet.$ext"
        else
            echo "  WARN: $src missing — skipped"
        fi
    done
done

echo "==> faction-leader sheets (robot-target leaders) → $sprites_dir"
# `draw-factions` renders configs/factions/*.yaml (the
# faction-leader manifest). Same pattern as draw-review: render to a
# scratch dir, then copy the named sheets into the runtime asset
# tree. Factions intentionally render to a separate scratch path so
# the lineup manifest + canonicals don't pollute review/.
factions_scratch="$renderer_dir/generated/factions"
mkdir -p "$factions_scratch"
(cd "$renderer_dir" && "$python_bin" -m ambition_sprite2d_renderer draw-factions --out-dir "$factions_scratch")
for cue in goblin_cantina_chieftain pulse_voyager_captain tech_bro_disruptor; do
    for ext in png yaml ron; do
        src="$factions_scratch/${cue}_spritesheet.$ext"
        if [ -f "$src" ]; then
            cp "$src" "$sprites_dir/${cue}_spritesheet.$ext"
            echo "  installed ${cue}_spritesheet.$ext"
        else
            echo "  WARN: $src missing — skipped"
        fi
    done
done

echo "==> tack-on targets (render-publish into $sprites_dir)"
# Every tack-on registered in _TACKON_TARGETS whose YAML manifest the
# sandbox crate loads. Keep this list in sync with cli.py's _TACKON_TARGETS
# (mockingbird_boss has its own driver below; pirates go through the
# standalone publisher).
tackon_targets=(
    sandbag
    burning_flying_shark
    creator
    creator_lab_props
    gnu_ton_boss
    interdimensional_gate
    intro_cart
    intro_lab_tileset
    lasersword
    lasersword_with_guns
    news_board
    town_tileset
)
for target in "${tackon_targets[@]}"; do
    (cd "$renderer_dir" && "$python_bin" -m ambition_sprite2d_renderer publish "$target" --dest-root "$sprites_dir")
done

echo "==> standalone pirate sheets (publish into $sprites_dir)"
# Pirates are registered as tack-on `[characters]` targets and publish
# through the same machinery as the other tack-ons above. Kept as its
# own loop so the runtime-required pirate list stays explicit.
pirate_targets=(
    pirate_admiral
    pirate_lookout
    pirate_navigator
    pirate_quartermaster
    pirate_raider
    # pirate_heavy fans out into three variants (broadside_bess, iron_mary,
    # salt_annet) — its module-level install copies all three flat into
    # $sprites_dir as `pirate_heavy_<slug>_spritesheet.{png,yaml,ron}`.
    pirate_heavy
)
for target in "${pirate_targets[@]}"; do
    (cd "$renderer_dir" && "$python_bin" -m ambition_sprite2d_renderer publish "$target" --dest-root "$sprites_dir")
done

echo "==> small enemy sprites (puppy_slug → $sprites_dir)"
(cd "$renderer_dir" && "$python_bin" -m ambition_sprite2d_renderer publish puppy_slug --dest-root "$sprites_dir")

echo "==> tack-on: mockingbird boss (render-publish into $sprites_dir/mockingbird_boss)"
"$python_bin" "$renderer_dir/mockingbird_boss_sprite_generator.py" render-publish \
    --install-dir "$sprites_dir/mockingbird_boss"

echo "==> postcondition: every runtime-required sprite file present"
# Walk the list of files the sandbox crate actually loads at runtime
# and fail loudly if any are missing after regen. Keeps the regen
# pipeline honest as new sprite consumers are added.
#
# Expected file list is derived from:
#   - every `*_SHEET` / `LAB_PROP_*` static in
#     `crates/ambition_sandbox/src/presentation/character_sprites/sheets.rs`
#     (each implies `{root}_spritesheet.{png,yaml,ron}` for single-record
#     files, or just the shared sheet for multi-record files like
#     creator_lab_props),
#   - the gnu_ton_boss / mockingbird_boss subdir PNG sets referenced
#     by `boss_encounter/sprites.rs`.
#
# If you add a new sprite consumer, append the filename here.
expected_files=(
    # Adapter targets (draw-all).
    boss_spritesheet.png boss_spritesheet.yaml boss_spritesheet.ron
    fascist_enforcer_spritesheet.png fascist_enforcer_spritesheet.yaml fascist_enforcer_spritesheet.ron
    goblin_spritesheet.png goblin_spritesheet.yaml goblin_spritesheet.ron
    ninja_shadow_duelist_spritesheet.png ninja_shadow_duelist_spritesheet.yaml ninja_shadow_duelist_spritesheet.ron
    ninja_shadow_oni_leader_spritesheet.png ninja_shadow_oni_leader_spritesheet.yaml ninja_shadow_oni_leader_spritesheet.ron
    player_robot_spritesheet.png player_robot_spritesheet.yaml player_robot_spritesheet.ron
    robot_spritesheet.png robot_spritesheet.yaml robot_spritesheet.ron
    sandbag_spritesheet.png sandbag_spritesheet.yaml sandbag_spritesheet.ron
    # Review-config NPCs (draw-review → copied).
    absurd_general_spritesheet.png absurd_general_spritesheet.yaml absurd_general_spritesheet.ron
    alice_spritesheet.png alice_spritesheet.yaml alice_spritesheet.ron
    architect_spritesheet.png architect_spritesheet.yaml architect_spritesheet.ron
    bob_spritesheet.png bob_spritesheet.yaml bob_spritesheet.ron
    erdish_spritesheet.png erdish_spritesheet.yaml erdish_spritesheet.ron
    kernel_guide_spritesheet.png kernel_guide_spritesheet.yaml kernel_guide_spritesheet.ron
    merchant_prototype_spritesheet.png merchant_prototype_spritesheet.yaml merchant_prototype_spritesheet.ron
    oiler_spritesheet.png oiler_spritesheet.yaml oiler_spritesheet.ron
    vault_keeper_spritesheet.png vault_keeper_spritesheet.yaml vault_keeper_spritesheet.ron
    # Faction-leader sheets (draw-factions → copied).
    goblin_cantina_chieftain_spritesheet.png goblin_cantina_chieftain_spritesheet.yaml goblin_cantina_chieftain_spritesheet.ron
    pulse_voyager_captain_spritesheet.png pulse_voyager_captain_spritesheet.yaml pulse_voyager_captain_spritesheet.ron
    tech_bro_disruptor_spritesheet.png tech_bro_disruptor_spritesheet.yaml tech_bro_disruptor_spritesheet.ron
    # Tack-on targets that produce character sheets.
    burning_flying_shark_spritesheet.png burning_flying_shark_spritesheet.yaml burning_flying_shark_spritesheet.ron
    creator_spritesheet.png creator_spritesheet.yaml creator_spritesheet.ron
    creator_lab_props_spritesheet.png creator_lab_props_spritesheet.yaml creator_lab_props_spritesheet.ron
    interdimensional_gate_portal_spritesheet.png interdimensional_gate_portal_spritesheet.yaml interdimensional_gate_portal_spritesheet.ron
    interdimensional_gate_ring_spritesheet.png interdimensional_gate_ring_spritesheet.yaml interdimensional_gate_ring_spritesheet.ron
    intro_cart_spritesheet.png intro_cart_spritesheet.yaml intro_cart_spritesheet.ron
    news_board_spritesheet.png news_board_spritesheet.yaml news_board_spritesheet.ron
    # Pirate sheets (standalone publisher).
    pirate_admiral_spritesheet.png pirate_admiral_spritesheet.yaml pirate_admiral_spritesheet.ron
    pirate_lookout_spritesheet.png pirate_lookout_spritesheet.yaml pirate_lookout_spritesheet.ron
    pirate_navigator_spritesheet.png pirate_navigator_spritesheet.yaml pirate_navigator_spritesheet.ron
    pirate_quartermaster_spritesheet.png pirate_quartermaster_spritesheet.yaml pirate_quartermaster_spritesheet.ron
    pirate_raider_spritesheet.png pirate_raider_spritesheet.yaml pirate_raider_spritesheet.ron
    # Pirate-heavy variants (three named bruisers sharing one rig).
    pirate_heavy_broadside_bess_spritesheet.png pirate_heavy_broadside_bess_spritesheet.yaml pirate_heavy_broadside_bess_spritesheet.ron
    pirate_heavy_iron_mary_spritesheet.png pirate_heavy_iron_mary_spritesheet.yaml pirate_heavy_iron_mary_spritesheet.ron
    pirate_heavy_salt_annet_spritesheet.png pirate_heavy_salt_annet_spritesheet.yaml pirate_heavy_salt_annet_spritesheet.ron
    # Small enemy sprites.
    puppy_slug_spritesheet.png puppy_slug_spritesheet.yaml puppy_slug_spritesheet.ron
    # Boss subdirectories (custom install paths).
    gnu_ton_boss/gnu_ton_boss_spritesheet.png
    gnu_ton_boss/gnu_ton_boss_body_spritesheet.png
    gnu_ton_boss/gnu_ton_boss_hands_spritesheet.png
    mockingbird_boss/mockingbird_boss_spritesheet.png
)
missing=()
for rel in "${expected_files[@]}"; do
    if [ ! -f "$sprites_dir/$rel" ]; then
        missing+=("$rel")
    fi
done
if [ "${#missing[@]}" -gt 0 ]; then
    echo "  ERROR: missing ${#missing[@]} expected file(s) after regen:" >&2
    for rel in "${missing[@]}"; do
        echo "    $sprites_dir/$rel" >&2
    done
    exit 1
fi
echo "  ok: ${#expected_files[@]} expected files present"

echo "==> done"
