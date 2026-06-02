#!/usr/bin/env bash
# Re-render every sprite asset and install into the sandbox crate.
#
# Covers:
#   - Adapter targets (robot / goblin / boss): re-renders every registered
#     target (run `ambition_sprite2d_renderer list`) — the adapter rigs are
#     driven by YAML in the renderer package's config dir
#     tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/configs/*.yaml
#     — straight into crates/ambition_sandbox/assets/sprites/.
#   - Entity sprites (chest, breakable, door zone, etc.): re-rendered into
#     crates/ambition_sandbox/assets/sprites/entities/.
#   - Standalone pirate sheets: rendered and published into
#     crates/ambition_sandbox/assets/sprites/.
#   - Tack-on targets (sandbag, mockingbird): rendered into the renderer's
#     generated/ dir then installed into crates/ambition_sandbox/assets/sprites/.
#
# Usage:
#   ./regen_sprites.sh                  # render + install everything (cache-skipped if fresh)
#   ./regen_sprites.sh --force          # bypass the cache, re-render unconditionally
#   ./regen_sprites.sh --list           # show registered targets for focused regen
#   ./regen_sprites.sh --target <name>  # render + install one registered target
#
# Caching:
#   The renderer's Python sources + configs are fingerprinted into
#   `tools/ambition_sprite2d_renderer/.cache/regen-fingerprint`. On the
#   next run, if the fingerprint matches AND every expected output sheet
#   already exists in assets/sprites/, the script exits early with no
#   rendering work. Fingerprint mismatch (a renderer source edit) or
#   a missing expected output (someone deleted a sheet) triggers a full
#   re-render. `--force` always re-renders.
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

force_regen=0
list_targets=0
target_name=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) print_help; exit 0 ;;
        --force|-f) force_regen=1; shift ;;
        --list|--list-targets) list_targets=1; shift ;;
        --target|-t)
            if [ "$#" -lt 2 ] || [ -z "${2:-}" ]; then
                echo "--target requires a target name" >&2
                exit 2
            fi
            target_name="$2"
            shift 2
            ;;
        --target=*)
            target_name="${1#--target=}"
            if [ -z "$target_name" ]; then
                echo "--target requires a target name" >&2
                exit 2
            fi
            shift
            ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

if [ "$list_targets" -eq 1 ] && [ -n "$target_name" ]; then
    echo "--list and --target are mutually exclusive" >&2
    exit 2
fi

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

list_sprite_targets() {
    echo "==> registered sprite targets"
    echo "    Use: ./regen_sprites.sh --target <target>"
    echo
    (cd "$renderer_dir" && "$python_bin" -m ambition_sprite2d_renderer list)
}

regen_one_target() {
    local target="$1"
    local dest_root="$sprites_dir"

    # Most targets install directly under assets/sprites/. The entity target
    # is the historical exception: runtime entity sprites live under
    # assets/sprites/entities/, so keep focused regen consistent with the
    # full regen path. Custom targets such as gnu_ton_boss, gnu_ton_apple,
    # interdimensional_gate, pirate_heavy, and mockingbird_boss own any
    # further subdirectory behavior via their Python Target.install hooks.
    if [ "$target" = "entities" ]; then
        dest_root="$entities_dir"
    fi

    echo "==> sprite target: $target → $dest_root"
    (cd "$renderer_dir" && "$python_bin" -m ambition_sprite2d_renderer publish "$target" --dest-root "$dest_root")
}

if [ "$list_targets" -eq 1 ]; then
    list_sprite_targets
    exit 0
fi

if [ -n "$target_name" ]; then
    regen_one_target "$target_name"
    exit 0
fi

# --- Fingerprint cache ----------------------------------------------------
# Hash every .py and .yaml under the renderer module + the boss generator
# script. If the hash matches the cached value AND every expected sheet
# is already present in $sprites_dir, skip the whole regen.
#
# The expected-files list is the same one the postcondition validates at
# the end. Keeping a single source of truth means a hand-deletion of one
# sheet trips both the fast-path check and the postcondition.
expected_files=(
    # Adapter targets (draw-all).
    boss_spritesheet.png boss_spritesheet.yaml boss_spritesheet.ron
    raid_enforcer_spritesheet.png raid_enforcer_spritesheet.yaml raid_enforcer_spritesheet.ron
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
    # No flat `pirate_heavy_spritesheet.png` ships: pirate_heavy is
    # a multi-variant rig (broadside_bess/iron_mary/salt_annet are the
    # real characters). Per Jon's 2026-05-24 feedback the catalog
    # dropped its bare `npc_pirate_heavy` entry rather than
    # shoehorning a placeholder.
    # Small enemy sprites.
    puppy_slug_spritesheet.png puppy_slug_spritesheet.yaml puppy_slug_spritesheet.ron
    # Phase 6 + bonus follow-up: every catalog-referenced tackon
    # character sprite, published by the tackon_targets loop below.
    agent_swarm_spritesheet.png agent_swarm_spritesheet.ron
    ai_slop_spritesheet.png ai_slop_spritesheet.ron
    bear_mauler_spritesheet.png bear_mauler_spritesheet.ron
    colonial_statesman_spritesheet.png colonial_statesman_spritesheet.ron
    dark_lord_spritesheet.png dark_lord_spritesheet.ron
    flying_spaghetti_monster_boss_spritesheet.png flying_spaghetti_monster_boss_spritesheet.ron
    galwah_spritesheet.png galwah_spritesheet.ron
    ghoul_skulker_spritesheet.png ghoul_skulker_spritesheet.ron
    girdle_spritesheet.png girdle_spritesheet.ron
    goblin_forest_spear_spritesheet.png goblin_forest_spear_spritesheet.ron
    hand_saint_spritesheet.png hand_saint_spritesheet.ron
    helpful_liar_spritesheet.png helpful_liar_spritesheet.ron
    mantis_lancer_spritesheet.png mantis_lancer_spritesheet.ron
    ninja_heavy_spritesheet.png ninja_heavy_spritesheet.ron
    pirate_cutlass_viper_spritesheet.png pirate_cutlass_viper_spritesheet.ron
    player_extended_spritesheet.png player_extended_spritesheet.ron
    president_portrait_spritesheet.png president_portrait_spritesheet.ron
    puppy_slug_variant2_spritesheet.png puppy_slug_variant2_spritesheet.ron
    raptor_stalker_spritesheet.png raptor_stalker_spritesheet.ron
    robot_guardian_spritesheet.png robot_guardian_spritesheet.ron
    # robot_heavy publishes as variants (bastion/arsenal/...); main
    # spritesheet doesn't ship until a publisher like pirate_heavy
    # lands. Catalog entry falls back to colored rectangle.
    robot_runner_spritesheet.png robot_runner_spritesheet.ron
    smart_house_spritesheet.png smart_house_spritesheet.ron
    spaghetti_event_spritesheet.png spaghetti_event_spritesheet.ron
    synthetic_friend_spritesheet.png synthetic_friend_spritesheet.ron
    trex_enemy_spritesheet.png trex_enemy_spritesheet.ron
    viking_heavy_shieldmaiden_spritesheet.png viking_heavy_shieldmaiden_spritesheet.ron
    viking_heavy_warrior_spritesheet.png viking_heavy_warrior_spritesheet.ron
    viking_shieldmaiden_spritesheet.png viking_shieldmaiden_spritesheet.ron
    viking_warrior_spritesheet.png viking_warrior_spritesheet.ron
    weird_hermit_spritesheet.png weird_hermit_spritesheet.ron
    # Review-config NPCs added to review_cues for full hall coverage.
    goblin_brute_hammer_spritesheet.png goblin_brute_hammer_spritesheet.ron
    goblin_cave_dagger_spritesheet.png goblin_cave_dagger_spritesheet.ron
    goblin_desert_bow_spritesheet.png goblin_desert_bow_spritesheet.ron
    goblin_frost_sword_spritesheet.png goblin_frost_sword_spritesheet.ron
    goblin_shaman_staff_spritesheet.png goblin_shaman_staff_spritesheet.ron
    player_combat_review_spritesheet.png player_combat_review_spritesheet.ron
    player_social_review_spritesheet.png player_social_review_spritesheet.ron
    player_traversal_review_spritesheet.png player_traversal_review_spritesheet.ron
    robot_archivist_spritesheet.png robot_archivist_spritesheet.ron
    robot_caster_spritesheet.png robot_caster_spritesheet.ron
    robot_diver_spritesheet.png robot_diver_spritesheet.ron
    robot_engineer_spritesheet.png robot_engineer_spritesheet.ron
    robot_medic_spritesheet.png robot_medic_spritesheet.ron
    robot_miner_spritesheet.png robot_miner_spritesheet.ron
    sandbag_armored_review_spritesheet.png sandbag_armored_review_spritesheet.ron
    sandbag_full_review_spritesheet.png sandbag_full_review_spritesheet.ron
    # Boss subdirectories (custom install paths).
    gnu_ton_boss/gnu_ton_boss_spritesheet.png
    gnu_ton_boss/gnu_ton_boss_body_spritesheet.png
    gnu_ton_boss/gnu_ton_boss_hands_spritesheet.png
    gnu_ton_boss/gnu_ton_boss_spritesheet.ron
    gnu_ton_boss/gnu_ton_boss_actor.ron
    mockingbird_boss/mockingbird_boss_spritesheet.png
)

cache_dir="$renderer_dir/.cache"
fingerprint_file="$cache_dir/regen-fingerprint"

compute_fingerprint() {
    # `cd` into renderer dir so the file paths in `sha256sum` output
    # are relative; absolute paths would make the hash depend on the
    # filesystem location.
    (
        cd "$renderer_dir" || exit 1
        {
            find ambition_sprite2d_renderer -type f \( -name '*.py' -o -name '*.yaml' \) -print0 \
                | sort -z \
                | xargs -0 sha256sum
            find . -maxdepth 1 -type f \( -name '*.py' -o -name '*.sh' \) -print0 \
                | sort -z \
                | xargs -0 sha256sum
            # The orchestrator script itself: changes to the install
            # loops, expected-files list, or the cache logic must
            # invalidate the cache too. Hash relative to repo root
            # to keep stability across filesystem locations.
            sha256sum "$repo_root/regen_sprites.sh" \
                | awk -v root="$repo_root/" '{sub(root, "", $2); print}'
        }
    ) | sha256sum | awk '{print $1}'
}

all_outputs_present() {
    local rel
    for rel in "${expected_files[@]}"; do
        if [ ! -f "$sprites_dir/$rel" ]; then
            return 1
        fi
    done
    return 0
}

# --- Per-sheet cache ------------------------------------------------------
# The global fingerprint above is all-or-nothing: it only stores on a
# fully-successful run, so an interrupted (partial) run re-renders every
# sheet next time. The per-sheet cache below lets the per-target publish
# loops skip individual sheets that are already current, so a resumed
# partial run only renders what's actually left.
#
# Each cache unit (one publishable target) is keyed on:
#   CORE_SHARED  — a hash of the *shared* renderer infra that any target
#                  can depend on (top-level package modules, every
#                  `_*.py` family helper, every `__init__.py`, and this
#                  orchestrator script). Editing shared infra changes
#                  CORE_SHARED, which invalidates ALL per-sheet units —
#                  the conservative, never-stale choice.
#   leaf hash    — a hash of the target's OWN module file (or package
#                  dir). Editing one leaf generator changes only that
#                  unit's key, so only that sheet re-renders.
#
# This relies on the codebase convention that shared drawing logic lives
# in a top-level module or a `_`-prefixed helper (e.g. tackon_sheet.py,
# lasersword_common.py, _pirate_common.py, _held_prop_common.py) — a
# target must never import a sibling non-`_` leaf module, or a change to
# that sibling would not invalidate this unit. The renderer already
# follows this convention.
sheets_cache_dir="$cache_dir/sheets"

compute_core_shared() {
    # NOTE: each `find` gets its OWN `| sort -z | xargs -0 sha256sum`
    # pipeline. Piping several `find … -print0` from one `{ … }` block
    # into a single `xargs` silently drops all but the first find's
    # output, so keep them separate (same structure as
    # `compute_fingerprint`).
    (
        cd "$renderer_dir" || exit 1
        # Top-level shared package modules (tackon_sheet, sheet,
        # adapters, actor_contract, lasersword_common, …).
        find ambition_sprite2d_renderer -maxdepth 1 -type f -name '*.py' -print0 \
            | sort -z | xargs -0 sha256sum
        # Family helpers + package markers under targets/.
        find ambition_sprite2d_renderer/targets -type f \( -name '_*.py' -o -name '__init__.py' \) -print0 \
            | sort -z | xargs -0 sha256sum
        # Renderer-dir top-level scripts (e.g. the mockingbird generator).
        find . -maxdepth 1 -type f \( -name '*.py' -o -name '*.sh' \) -print0 \
            | sort -z | xargs -0 sha256sum
        # NB: this orchestrator (`regen_sprites.sh`) is deliberately NOT
        # hashed into CORE_SHARED. It only chooses *which* targets to
        # publish and *how* to loop — it never affects a sheet's pixels.
        # Folding it in here meant that wiring a new sprite (adding its
        # name to `tackon_targets`) changed CORE_SHARED and invalidated
        # EVERY per-sheet key, forcing a full regen just to render the
        # one new sheet. The global fingerprint above (`compute_fingerprint`)
        # still includes this script, so the all-or-nothing fast-path
        # correctly re-checks when the script changes.
    ) | sha256sum | awk '{print $1}'
}

# Hash a target's own source (single-file module or package dir).
# Empty (constant) when no leaf file is found — such units fall back to
# CORE_SHARED-only keying, which is still correct (they re-render on any
# shared change and are gated by their output existence).
leaf_hash() {
    local name="$1"
    (
        cd "$renderer_dir" || exit 1
        local f d
        for f in ambition_sprite2d_renderer/targets/*/"$name".py; do
            if [ -f "$f" ]; then sha256sum "$f"; return 0; fi
        done
        for d in ambition_sprite2d_renderer/targets/*/"$name"; do
            if [ -d "$d" ]; then
                find "$d" -type f -name '*.py' -print0 | sort -z | xargs -0 sha256sum
                return 0
            fi
        done
    ) | sha256sum | awk '{print $1}'
}

unit_key() {
    printf '%s:%s' "$core_shared_fingerprint" "$(leaf_hash "$1")" \
        | sha256sum | awk '{print $1}'
}

# Fresh iff the stored key matches AND at least one output matching the
# glob already exists on disk (a hand-deletion re-renders that sheet).
sheet_cache_fresh() {
    local unit="$1" key="$2" glob="$3" stored
    [ "$force_regen" -ne 1 ] || return 1
    [ -f "$sheets_cache_dir/$unit" ] || return 1
    stored="$(cat "$sheets_cache_dir/$unit")"
    [ "$stored" = "$key" ] || return 1
    compgen -G "$glob" >/dev/null 2>&1 || return 1
    return 0
}

sheet_cache_store() {
    mkdir -p "$sheets_cache_dir"
    printf '%s\n' "$2" > "$sheets_cache_dir/$1"
}

# Publish one registered target with per-sheet caching. Skips when the
# sheet is already current; stores the key only on a successful publish
# so a failure retries next run.
publish_cached() {
    local target="$1"
    local key glob
    key="$(unit_key "$target")"
    glob="$sprites_dir/${target}"*"_spritesheet.png"
    if sheet_cache_fresh "$target" "$key" "$glob"; then
        echo "  [cache] $target up to date — skipped"
        return 0
    fi
    if (cd "$renderer_dir" && "$python_bin" -m ambition_sprite2d_renderer publish "$target" --dest-root "$sprites_dir"); then
        sheet_cache_store "$target" "$key"
    else
        echo "  [skip] tack-on target '$target' publish failed (publisher not implemented?)"
    fi
}

core_shared_fingerprint="$(compute_core_shared)"

cached_fingerprint=""
if [ -f "$fingerprint_file" ]; then
    cached_fingerprint="$(cat "$fingerprint_file")"
fi
current_fingerprint="$(compute_fingerprint)"

if [ "$force_regen" -ne 1 ] \
    && [ -n "$cached_fingerprint" ] \
    && [ "$cached_fingerprint" = "$current_fingerprint" ] \
    && all_outputs_present
then
    echo "==> regen cache hit: renderer sources + outputs unchanged — skipping ${#expected_files[@]} sheet renders."
    echo "    Cache key: $fingerprint_file"
    echo "    Pass --force to re-render anyway."
    exit 0
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
    merchant_prototype oiler erdish raid_enforcer
    # Named characters whose YAML manifests already live in $sprites_dir.
    alice bob craig eve general_hero judy mallory olivia
    peggy sybil trent trudy victor walter
    # Phase 6 + bonus follow-up: every review config is now an
    # actual catalog character. Install the rest so the Hall of
    # Characters has a sprite for each.
    goblin_brute_hammer goblin_cave_dagger goblin_desert_bow
    goblin_frost_sword goblin_shaman_staff
    player_combat_review player_social_review player_traversal_review
    robot_archivist robot_caster robot_diver robot_engineer
    robot_medic robot_miner
    sandbag_armored_review sandbag_full_review
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
    # Hand-held weapon props + attack-effect overlays. Authored
    # pointing right (+X); the game pins them at the `grip`/`origin`
    # anchor and rotates to the swing/aim direction at runtime.
    pirate_heavy_axe
    throwing_javelin
    hunting_bow
    bow_arrow
    robot_slash
    news_board
    town_tileset
    # Phase 6 + bonus follow-up: every tack-on character listed by
    # `list-targets` now has a catalog entry; publish them all so the
    # Hall of Characters has a sprite for each.
    agent_swarm
    ai_slop
    bear_mauler
    colonial_statesman
    dark_lord
    flying_spaghetti_monster_boss
    galwah
    ghoul_skulker
    girdle
    goblin_forest_spear
    hand_saint
    helpful_liar
    mantis_lancer
    ninja_heavy
    pirate_cutlass_viper
    player_extended
    president_portrait
    puppy_slug_variant2
    raptor_stalker
    robot_guardian
    # robot_heavy is a multi-variant rig whose publisher doesn't
    # install (renders only to generated/, no install method).
    # Skipping it here keeps the working tree clean. Catalog
    # entry was dropped along with the publisher work.
    robot_runner
    smart_house
    spaghetti_event
    synthetic_friend
    trex_enemy
    viking_heavy_shieldmaiden
    viking_heavy_warrior
    viking_shieldmaiden
    viking_warrior
    # weird_hermit's publisher was fixed 2026-05-24 to emit the
    # canonical `<target>_spritesheet.{png,ron,yaml}` filenames + the
    # runtime's standard SheetRow schema. Catalog entry now resolves.
    weird_hermit
)
for target in "${tackon_targets[@]}"; do
    # Per-target failure is non-fatal — some targets (e.g.
    # robot_heavy / viking_warrior variants) don't have a publish
    # path implemented yet and exit non-zero. The postcondition
    # check below catches anything that actually needs to ship.
    # `publish_cached` skips targets already current (per-sheet cache)
    # and only stores the cache key on a successful publish.
    publish_cached "$target"
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
    publish_cached "$target"
done

echo "==> small enemy sprites (puppy_slug → $sprites_dir)"
publish_cached puppy_slug

echo "==> tack-on: mockingbird boss (render-publish into $sprites_dir/mockingbird_boss)"
# Custom generator script + subdir output, so it gets a bespoke
# per-sheet check rather than `publish_cached`. Its source is the
# top-level generator script, which is folded into CORE_SHARED.
mockingbird_key="$(unit_key mockingbird_boss)"
if sheet_cache_fresh mockingbird_boss "$mockingbird_key" \
    "$sprites_dir/mockingbird_boss/mockingbird_boss"*"_spritesheet.png"; then
    echo "  [cache] mockingbird_boss up to date — skipped"
else
    "$python_bin" "$renderer_dir/mockingbird_boss_sprite_generator.py" render-publish \
        --install-dir "$sprites_dir/mockingbird_boss"
    sheet_cache_store mockingbird_boss "$mockingbird_key"
fi

echo "==> postcondition: every runtime-required sprite file present"
# Walk the list of files the sandbox crate actually loads at runtime
# and fail loudly if any are missing after regen. Keeps the regen
# pipeline honest as new sprite consumers are added.
#
# The expected-files list is defined near the top of this script
# (it's also consumed by the cache-skip check). When adding a new
# sprite consumer, update that list.
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

# --- Hall-of-Characters sprite census ------------------------------------
# Quick check of which catalog entries the Hall will render vs fall
# back to the colored-rectangle placeholder. Helpful as a final
# "did the regen actually fix the Hall?" signal.
echo "==> Hall-of-Characters sprite census:"
if command -v "$python_bin" >/dev/null 2>&1 && \
    PYTHONPATH="$repo_root/tools/ambition_ldtk_tools" "$python_bin" \
        -c "import ambition_ldtk_tools" 2>/dev/null
then
    PYTHONPATH="$repo_root/tools/ambition_ldtk_tools" "$python_bin" \
        -m ambition_ldtk_tools.inspect_hall_sprites --only-issues \
        2>&1 | sed 's/^/  /' || true
else
    echo "  (skipped — ambition_ldtk_tools not importable from $python_bin)"
fi

# --- Write fingerprint on success ----------------------------------------
mkdir -p "$cache_dir"
echo "$current_fingerprint" > "$fingerprint_file"
echo "  cached regen fingerprint at $fingerprint_file"

echo "==> done"
