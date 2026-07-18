#!/usr/bin/env bash
# Re-render every sprite asset and install into the sandbox crate.
#
# Covers:
#   - Config-driven targets (robot / goblin / boss): re-renders every
#     registered YAML job (run `ambition_sprite2d_renderer list`) from the
#     renderer package's config dir
#     tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/configs/*.yaml
#     — straight into crates/ambition_actors/assets/sprites/.
#   - Entity sprites (chest, breakable, door zone, etc.): re-rendered into
#     crates/ambition_actors/assets/sprites/entities/.
#   - Standalone pirate sheets: rendered and published into
#     crates/ambition_actors/assets/sprites/.
#   - Tack-on targets (sandbag, mockingbird): rendered into the renderer's
#     generated/ dir then installed into crates/ambition_actors/assets/sprites/.
#
# Usage:
#   ./regen_sprites.sh                  # render + install everything (cache-skipped if fresh)
#   ./regen_sprites.sh --force          # bypass the cache, re-render unconditionally
#   ./regen_sprites.sh --list           # show registered targets for focused regen
#   ./regen_sprites.sh --target <name>  # render + install one registered target
#
# Environment:
#   AMBITION_SPRITE_PYTHON=/path/to/python  Override the sprite tool .venv.
#   AMBITION_LDTK_PYTHON=/path/to/python    Override the LDtk tool .venv.
#
# Caching:
#   The renderer's Python sources + configs are fingerprinted into
#   `tools/ambition_sprite2d_renderer/.cache/regen-fingerprint`. On the
#   next run, if the fingerprint matches AND every expected published output
#   already exists in assets/sprites/, the script exits early with no
#   rendering work. Fingerprint mismatch (a renderer source edit) or
#   a missing expected output (someone deleted an asset) triggers a full
#   re-render. `--force` always re-renders.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

renderer_dir="$repo_root/tools/ambition_sprite2d_renderer"
ldtk_tools_dir="$repo_root/tools/ambition_ldtk_tools"
content_assets_dir="$repo_root/game/ambition_content/assets"
worlds_dir="$content_assets_dir/worlds"
character_catalog="$content_assets_dir/data/character_catalog.ron"
sandbox_ldtk="$worlds_dir/sandbox.ldtk"
hall_ldtk="$worlds_dir/hall_of_characters.ldtk"
sprites_dir="$repo_root/crates/ambition_actors/assets/sprites"
entities_dir="$sprites_dir/entities"

# shellcheck disable=SC1091
source "$repo_root/scripts/lib/tool_python.sh"

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
make_gifs=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) print_help; exit 0 ;;
        --force|-f) force_regen=1; shift ;;
        --gif|--gifs) make_gifs=1; shift ;;
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

if [ "$make_gifs" -eq 1 ] && [ -z "$target_name" ]; then
    echo "--gif requires --target <name>" >&2
    exit 2
fi

python_bin="$(ambition_select_tool_python "$renderer_dir" AMBITION_SPRITE_PYTHON)"
ldtk_python="$(ambition_select_tool_python "$ldtk_tools_dir" AMBITION_LDTK_PYTHON 0)"
ambition_require_python_module \
    "$python_bin" ambition_sprite2d_renderer \
    "run ./run_developer_setup.sh or set AMBITION_SPRITE_PYTHON=/path/to/python"
ambition_require_python_module \
    "$ldtk_python" ambition_ldtk_tools \
    "run ./run_developer_setup.sh or set AMBITION_LDTK_PYTHON=/path/to/python"

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
    if [ "$make_gifs" -eq 1 ]; then
        echo "==> animation GIFs: $target → $renderer_dir/generated/gifs/$target"
        (cd "$renderer_dir" && "$python_bin" -m ambition_sprite2d_renderer gifs "$target")
    fi
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
# the end. Keeping a single source of truth means deleting one published
# sheet, manifest, or portrait trips both the fast-path and postcondition.
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
    # Sandbox combat-variety enemies (draw-all): the kiter + the two volatile mites.
    ranged_skirmisher_spritesheet.png ranged_skirmisher_spritesheet.yaml ranged_skirmisher_spritesheet.ron
    exploding_mite_spritesheet.png exploding_mite_spritesheet.yaml exploding_mite_spritesheet.ron
    dividing_mite_spritesheet.png dividing_mite_spritesheet.yaml dividing_mite_spritesheet.ron
    # Review-config NPCs (draw-review → copied).
    absurd_general_spritesheet.png absurd_general_spritesheet.yaml absurd_general_spritesheet.ron
    alice_spritesheet.png alice_spritesheet.yaml alice_spritesheet.ron
    alice_portraits.png alice_portraits.ron
    architect_spritesheet.png architect_spritesheet.yaml architect_spritesheet.ron
    bob_spritesheet.png bob_spritesheet.yaml bob_spritesheet.ron
    erdish_spritesheet.png erdish_spritesheet.yaml erdish_spritesheet.ron
    kernel_guide_spritesheet.png kernel_guide_spritesheet.yaml kernel_guide_spritesheet.ron
    merchant_prototype_spritesheet.png merchant_prototype_spritesheet.yaml merchant_prototype_spritesheet.ron
    oiler_spritesheet.png oiler_spritesheet.yaml oiler_spritesheet.ron
    oiler_portraits.png oiler_portraits.ron
    vault_keeper_spritesheet.png vault_keeper_spritesheet.yaml vault_keeper_spritesheet.ron
    # Faction-leader sheets (draw-factions → copied).
    goblin_cantina_chieftain_spritesheet.png goblin_cantina_chieftain_spritesheet.yaml goblin_cantina_chieftain_spritesheet.ron
    pulse_voyager_captain_spritesheet.png pulse_voyager_captain_spritesheet.yaml pulse_voyager_captain_spritesheet.ron
    tech_bro_disruptor_spritesheet.png tech_bro_disruptor_spritesheet.yaml tech_bro_disruptor_spritesheet.ron
    # Tack-on targets that produce character sheets.
    burning_flying_shark_spritesheet.png burning_flying_shark_spritesheet.yaml burning_flying_shark_spritesheet.ron
    pipi_tau_spritesheet.png pipi_tau_spritesheet.yaml pipi_tau_spritesheet.ron
    pipi_tau_portraits.png pipi_tau_portraits.ron
    sanic_spritesheet.png sanic_spritesheet.yaml sanic_spritesheet.ron
    super_sanic_spritesheet.png super_sanic_spritesheet.yaml super_sanic_spritesheet.ron
    sanic_ring_prop_spritesheet.png sanic_ring_prop_spritesheet.yaml sanic_ring_prop_spritesheet.ron
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
    # Intro / cut-the-rope content + catalog characters (see the
    # matching tackon_targets block).
    cut_rope_anvil_spritesheet.png cut_rope_anvil_spritesheet.ron
    cut_rope_piano_spritesheet.png cut_rope_piano_spritesheet.ron
    cut_rope_rope_spritesheet.png cut_rope_rope_spritesheet.ron
    super_mary_o_spritesheet.png super_mary_o_spritesheet.ron
    generic_explosions_spritesheet.png generic_explosions_spritesheet.ron
    smirking_behemoth_boss_spritesheet.png smirking_behemoth_boss_spritesheet.ron
    stochastic_parrot_spritesheet.png stochastic_parrot_spritesheet.ron
    stochastic_parrot_v2_spritesheet.png stochastic_parrot_v2_spritesheet.ron
    imperfect_cellular_automaton_spritesheet.png imperfect_cellular_automaton_spritesheet.ron
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
    # Rigged (bone-toolkit) characters auto-discovered under
    # targets/characters/rigged/*.rig.json.
    noether_spritesheet.png noether_spritesheet.ron
    # Boss subdirectories (custom install paths).
    gnu_ton_boss/gnu_ton_boss_spritesheet.png
    gnu_ton_boss/gnu_ton_boss_body_spritesheet.png
    gnu_ton_boss/gnu_ton_boss_hands_spritesheet.png
    gnu_ton_boss/gnu_ton_boss_spritesheet.ron
    gnu_ton_boss/gnu_ton_boss_actor.ron
    mockingbird_boss/mockingbird_boss_spritesheet.png
    mockingbird_boss/mockingbird_boss_spritesheet.ron
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
            find ambition_sprite2d_renderer -type f \( -name '*.py' -o -name '*.yaml' -o -name '*.json' \) -print0 \
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
#                  can depend on (top-level package modules, the core/
#                  authoring/registry/cli subpackages, every `_*.py`
#                  family helper, every `__init__.py`, and the renderer
#                  dir's top-level scripts). Editing shared infra changes
#                  CORE_SHARED, which invalidates ALL per-sheet units —
#                  the conservative, never-stale choice.
#   leaf hash    — a hash of the target's OWN module file (or package
#                  dir). Editing one leaf generator changes only that
#                  unit's key, so only that sheet re-renders.
#
# This relies on the codebase convention that shared drawing logic lives
# in core/, authoring/, or a `_`-prefixed family helper (e.g.
# authoring/sheet_build.py, authoring/lasersword_common.py,
# targets/characters/_pirate_common.py, targets/props/_held_prop_common.py)
# — a target must never import a sibling non-`_` leaf module, or a change
# to that sibling would not invalidate this unit. The renderer already
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
        # Top-level package modules (__init__, __main__, ldtk_manifest).
        find ambition_sprite2d_renderer -maxdepth 1 -type f -name '*.py' -print0 \
            | sort -z | xargs -0 sha256sum
        # Shared render infra subpackages — the draw primitives, sheet
        # spines, RON emitter, packer, discovery, and CLI every target
        # renders through. These live in subpackages (core/, authoring/,
        # registry/, cli/), NOT at the package top level, so the maxdepth-1
        # find above does not see them.
        find ambition_sprite2d_renderer/core ambition_sprite2d_renderer/authoring \
            ambition_sprite2d_renderer/registry ambition_sprite2d_renderer/cli \
            -type f -name '*.py' -print0 \
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
        # Rigged characters (GUI .rig.json docs) are data, not a .py leaf —
        # hash the document so editing a rig invalidates only its sheet.
        local rig="ambition_sprite2d_renderer/targets/characters/rigged/$name.rig.json"
        if [ -f "$rig" ]; then sha256sum "$rig"; return 0; fi
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
        # A portrait-capable target's cache unit is its complete published
        # bundle, not merely the gameplay sheet. Hand-deleting the portrait
        # must force regeneration just like deleting the sheet.
        if [ "$target" != "pipi_tau" ] \
            || { [ -f "$sprites_dir/pipi_tau_portraits.png" ] \
                && [ -f "$sprites_dir/pipi_tau_portraits.ron" ]; }; then
            echo "  [cache] $target up to date — skipped"
            return 0
        fi
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
    echo "==> regen cache hit: renderer sources + outputs unchanged — skipping sprite publication."
    echo "    Cache key: $fingerprint_file"
    echo "    Pass --force to re-render anyway."
    exit 0
fi

echo "==> config-driven targets (robot / goblin / boss) → $sprites_dir"
(cd "$renderer_dir" && "$python_bin" -m ambition_sprite2d_renderer draw-all --out-dir "$sprites_dir")

echo "==> entity sprites → $entities_dir"
(cd "$renderer_dir" && "$python_bin" -m ambition_sprite2d_renderer publish entities --dest-root "$entities_dir")

echo "==> review NPC sheets (toon-target NPCs) → $sprites_dir"
# `draw-review` renders configs/review/*.yaml (toon-target NPC
# variants such as absurd_general, architect, kernel_guide). We
# render to a scratch dir, then copy the specific sheets we use
# in-game into $sprites_dir. Promoting a review config to a
# permanent runtime sheet means: add the cue id to the copy list
# below AND give it a `character_catalog.ron` entry (specs are built
# from the sheet RON at load; the old `*_SHEET` statics in
# character_sprites are gone).
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

# Overlay 1 publishes catalog-backed default portraits for one representative
# config target. Keep bulk gameplay rendering unchanged: explicitly render and
# promote only portrait products referenced by the runtime catalog.
echo "==> Alice native config-generator portrait -> $sprites_dir"
(cd "$renderer_dir" && "$python_bin" -m ambition_sprite2d_renderer portraits alice)
for ext in png ron; do
    src="$renderer_dir/generated/alice/alice_portraits.$ext"
    cp "$src" "$sprites_dir/alice_portraits.$ext"
    echo "  installed alice_portraits.$ext"
done

# Oiler is the representative direct-SVG rig target. Render only its portrait
# product here so full regeneration does not replace the established gameplay
# sheet selected by the review config. Focused `--target oiler` still publishes
# the module target's full sheet + portrait bundle.
echo "==> Oiler native rig portrait → $sprites_dir"
(cd "$renderer_dir" && "$python_bin" -m ambition_sprite2d_renderer portraits oiler)
for ext in png ron; do
    src="$renderer_dir/generated/oiler/oiler_portraits.$ext"
    cp "$src" "$sprites_dir/oiler_portraits.$ext"
    echo "  installed oiler_portraits.$ext"
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
# Every registered module target whose manifest the runtime loads.
# The registry is `registry/discovery.py` (auto-discovered from
# targets/<category>/ — run `list` to see it); keep this list covering
# every target the game references (mockingbird_boss has its own driver
# below; pirates go through the standalone publisher).
tackon_targets=(
    sandbag
    burning_flying_shark
    pipi_tau
    sanic
    super_sanic
    sanic_ring_prop
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
    portal_gun_blue
    portal_gun_orange
    hunting_bow
    bow_arrow
    robot_slash
    news_board
    town_tileset
    # Intro / cut-the-rope content (loaded by ambition_content's intro
    # sprites + cut_rope boss) and catalog-referenced characters that
    # were missing from this list — a fresh clone rendered them as
    # colored rectangles.
    cut_rope_anvil
    cut_rope_piano
    cut_rope_rope
    # Super Mary-O playable protagonist for the SMB1 demo (M-track).
    # Its catalog row (game/ambition_demo_smb1) references
    # sprites/super_mary_o_spritesheet.*; without this publish a fresh
    # clone renders the demo character as a colored rectangle.
    super_mary_o
    generic_explosions
    smirking_behemoth_boss
    stochastic_parrot
    stochastic_parrot_v2
    imperfect_cellular_automaton
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

# Rigged characters authored as GUI `.rig.json` documents
# (targets/characters/rigged/*.rig.json) auto-register as targets named after
# the file stem. They were previously checked for existence (expected_files)
# but NEVER re-published here, so rig edits silently never reached the crate.
# Discover + publish them so the rig editor's output ships like everything else.
echo "==> rigged characters (GUI .rig.json docs) → $sprites_dir"
for rig in "$renderer_dir"/ambition_sprite2d_renderer/targets/characters/rigged/*.rig.json; do
    [ -f "$rig" ] || continue
    rig_name="$(basename "$rig" .rig.json)"
    publish_cached "$rig_name"
done

echo "==> held-item prop canonicals (single-pose → $sprites_dir/props)"
# A few props are shown in-game as STATIC held / ground items, which load
# the single-pose `*_canonical_transparent.png` (not the animated sheet the
# tack-on publish installs). Copy those canonicals flat into props/ so the
# runtime asset paths (`sprites/props/<name>.png`) resolve on a fresh clone.
props_dir="$sprites_dir/props"
mkdir -p "$props_dir"
# render-target-name -> runtime props/ basename
held_prop_map=(
    "pirate_heavy_axe:axe"
    "throwing_javelin:javelin"
    "lasersword_with_guns:gunsword"
    "portal_gun_blue:portal_gun_blue"
    "portal_gun_orange:portal_gun_orange"
)
for pair in "${held_prop_map[@]}"; do
    src_target="${pair%%:*}"
    dst_name="${pair##*:}"
    canon="$renderer_dir/generated/$src_target/${src_target}_canonical_transparent.png"
    if [ -f "$canon" ]; then
        cp "$canon" "$props_dir/${dst_name}.png"
        echo "    $src_target -> props/${dst_name}.png"
    else
        echo "    warning: missing $canon (held-item prop not rendered)" >&2
    fi
done

echo "==> wielded-gauntlet prop icons (procedural → $props_dir)"
# The abstract wielded gauntlets (shockwave/volley/beam/vortex/sentry/dive/
# meteor) have no character rig, so their ground/held icons are procedural
# 64x64 PNGs from `item_icons.py::write_gauntlet_props`, consumed at runtime by
# `item_pickup::item_sprite`. (No canonical-pose copy step — drawn directly.)
(cd "$renderer_dir" && "$python_bin" -c "from ambition_sprite2d_renderer.targets.icons.item_icons import write_gauntlet_props as w; w('$props_dir')")

echo "==> heal/save shrine prop (procedural obelisk → $props_dir)"
# The world heal/save shrine is a free-standing prop (taller than the 64x64
# icons), an 88x160 obelisk from `item_icons.py::write_shrine_prop`, consumed at
# runtime by `shrine::sync_shrine_visual`.
(cd "$renderer_dir" && "$python_bin" -c "from ambition_sprite2d_renderer.targets.icons.item_icons import write_shrine_prop as w; w('$props_dir')")

echo "==> Mark/Recall world beacon prop (procedural crystal → $props_dir)"
# The recall beacon is a free-standing 48x112 crystal pillar from
# `item_icons.py::write_mark_beacon_prop`, consumed at runtime by
# `mark_recall::sync_mark_beacon_visual` (stands at the dropped recall mark).
(cd "$renderer_dir" && "$python_bin" -c "from ambition_sprite2d_renderer.targets.icons.item_icons import write_mark_beacon_prop as w; w('$props_dir')")

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
# Custom generator CLI + subdir output, so it gets a bespoke per-sheet
# check rather than `publish_cached`. Its source is the mockingbird_boss
# package dir, which leaf_hash covers.
mockingbird_key="$(unit_key mockingbird_boss)"
if sheet_cache_fresh mockingbird_boss "$mockingbird_key" \
    "$sprites_dir/mockingbird_boss/mockingbird_boss"*"_spritesheet.png"; then
    echo "  [cache] mockingbird_boss up to date — skipped"
else
    (cd "$renderer_dir" && "$python_bin" \
        -m ambition_sprite2d_renderer.targets.characters.mockingbird_boss \
        render-publish --install-dir "$sprites_dir/mockingbird_boss")
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

# --- Publish boundary: sweep diagnostics out of the runtime roots ---------
# The sprite generators emit human-only diagnostics (canonical poses, labeled
# previews, debug overlays) next to the runtime sheets. Relocate them out of
# the runtime asset roots into target/ambition_publish/diagnostics so the game
# bundle ships runtime artifacts only. This is what keeps the Rust
# `shipped_runtime_roots_have_no_leaked_diagnostics` test green after a regen.
# See docs/planning/engine/data-driven-sprites-and-characters.md.
echo "==> Publish boundary: sweeping diagnostics out of runtime roots:"
if command -v "$python_bin" >/dev/null 2>&1; then
    "$python_bin" "$repo_root/scripts/sweep_runtime_diagnostics.py" \
        --repo-root "$repo_root" 2>&1 | sed 's/^/  /' || true
else
    echo "  (skipped — no python interpreter)"
fi

# --- Ultrapacked quality-tier sprite atlases (runtime install) ------------
# Pool every published per-target sheet into shared, uniformly-sized atlas
# pages at each quality tier, then write pages + a SpritePackCatalog into the
# RUNTIME pack root assets/sprite_packs/<tier>/ (gitignored, generated).
# Tier names match the runtime `TextureResolutionScale` enum (full / half /
# quarter / potato) — the game's pack consumer selects the tier dir from the
# active quality budget. `build.rs` bakes each tier's ultrapack.json. See
# docs/planning/engine/data-driven-sprites-and-characters.md (W2).
#
# Efficient by construction: the sheets were rendered ONCE above, so each tier
# reads that pool (`--from-rendered`) and downsamples each isolated frame to
# the tier budget before repacking — never re-rendering, and never resizing an
# already-packed page (which would bleed neighbours across frame edges).
#
# Debug views (labeled page overlays + a pack report) are OFF by default and
# always land in STAGING (never the runtime pack root — the hygiene test
# would flag them there).
#   AMBITION_ULTRAPACK=0        skip the pack step entirely (fast dev regen)
#   AMBITION_ULTRAPACK_DEBUG=1  also emit per-page diagnostics into staging
echo "==> Ultrapack: shared-page atlases per quality tier → runtime pack root:"
pack_root="$repo_root/crates/ambition_actors/assets/sprite_packs"
pack_debug_root="$repo_root/target/ambition_publish/diagnostics/packs"
if [ "${AMBITION_ULTRAPACK:-1}" = "0" ]; then
    echo "  (skipped — AMBITION_ULTRAPACK=0)"
elif command -v "$python_bin" >/dev/null 2>&1 && \
    "$python_bin" -c 'import ambition_sprite2d_renderer' >/dev/null 2>&1
then
    # tier: <name> <scale> <min_frame_px> <page_size>
    #
    # Page size scales DOWN with the tier: shrunk frames pack many-per-page,
    # and MaxRects degrades badly with thousands of tiny rects in one big page
    # (potato @ 2048² takes minutes). A smaller page keeps frames-per-page
    # bounded — potato @ 256² packs in ~10s — and a potato atlas has no reason
    # to be 2048². Uniform page size still holds WITHIN each pack.
    ultrapack_tiers=(
        "full 1.0 1 2048"
        "half 0.5 1 1024"
        "quarter 0.25 1 512"
        "potato 0.0625 8 256"
    )
    # PackPlan (locality groups): authored in the renderer's data dir (NOT
    # configs/, which is reserved for CharacterJob YAMLs globbed by draw-all);
    # quality-independent, so the same plan applies to every tier.
    ultrapack_plan=()
    if [ -f "$renderer_dir/ambition_sprite2d_renderer/data/pack_plan.yaml" ]; then
        ultrapack_plan=(--pack-plan "ambition_sprite2d_renderer/data/pack_plan.yaml")
    fi
    for tier in "${ultrapack_tiers[@]}"; do
        read -r tname tscale tmin tpage <<<"$tier"
        ultrapack_debug=()
        if [ "${AMBITION_ULTRAPACK_DEBUG:-0}" = "1" ]; then
            ultrapack_debug=(--debug-views --debug-dir "$pack_debug_root/$tname")
        fi
        (cd "$renderer_dir" && "$python_bin" -m ambition_sprite2d_renderer ultrapack \
            --from-rendered "$sprites_dir" \
            --out "$pack_root/$tname" \
            --scale "$tscale" --min-frame-px "$tmin" --page-size "$tpage" \
            --name ultrapack "${ultrapack_plan[@]}" "${ultrapack_debug[@]}") 2>&1 | sed 's/^/  /' || \
            echo "  WARN: ultrapack tier '$tname' failed (non-fatal)"
    done
    echo "  packs installed under $pack_root/{full,half,quarter,potato}/"
    # Postcondition: every tier packs the SAME target set. A transient IO
    # flake once silently dropped 59 targets from one tier — scale must
    # never change coverage, so unequal sets are a hard regen failure.
    "$python_bin" - "$pack_root" <<'PYEOF'
import json, sys
from pathlib import Path
root = Path(sys.argv[1])
sets = {}
for tier in ("full", "half", "quarter", "potato"):
    cat = root / tier / "ultrapack.json"
    if cat.exists():
        sets[tier] = set(json.loads(cat.read_text())["targets"])
if not sets:
    sys.exit("  ERROR: no tier catalogs found under %s" % root)
ref_tier = "full" if "full" in sets else sorted(sets)[0]
ref = sets[ref_tier]
bad = False
for tier, s in sorted(sets.items()):
    if s != ref:
        bad = True
        missing = sorted(ref - s)[:5]
        extra = sorted(s - ref)[:5]
        print(f"  ERROR: tier '{tier}' target set differs from '{ref_tier}' "
              f"(missing {len(ref - s)}: {missing}… / extra {len(s - ref)}: {extra}…)",
              file=sys.stderr)
if bad:
    sys.exit(1)
print(f"  ok: {len(sets)} tiers x {len(ref)} targets — coverage identical")
PYEOF
else
    echo "  (skipped — sprite renderer not importable from $python_bin)"
fi

# --- Hall-of-Characters sprite census ------------------------------------
# Quick check of which catalog entries the Hall will render vs fall
# back to the colored-rectangle placeholder. Helpful as a final
# "did the regen actually fix the Hall?" signal.
echo "==> Hall-of-Characters sprite census:"
if ambition_python_exists "$ldtk_python" && \
    "$ldtk_python" \
        -c "import ambition_ldtk_tools" 2>/dev/null
then
    "$ldtk_python" \
        -m ambition_ldtk_tools.inspect_hall_sprites \
        --catalog "$character_catalog" \
        --ldtk "$hall_ldtk" \
        --sprites-dir "$sprites_dir" \
        --only-issues \
        2>&1 | sed 's/^/  /' || true
else
    echo "  (skipped — ambition_ldtk_tools not importable from $ldtk_python)"
fi

# --- LDtk editor-icon atlas ----------------------------------------------
# Regenerate the gitignored editor-icon atlas that the worlds' EditorIcons
# tileset references, so the LDtk editor shows a distinct icon per entity
# type on a fresh clone. PNG only — the per-entity tileRect wiring is
# committed in the .ldtk; only re-run `asset register-entity-icons` when the
# entity set changes (it rewrites the .ldtk).
echo "==> LDtk editor-icon atlas:"
if ambition_python_exists "$ldtk_python" && \
    "$ldtk_python" \
        -c "import ambition_ldtk_tools" 2>/dev/null
then
    "$ldtk_python" \
        -m ambition_ldtk_tools asset generate-editor-icons "$sandbox_ldtk" \
        --icons "$sprites_dir/editor_icons.png" --tile-size 32 \
        2>&1 | sed 's/^/  /' || true
else
    echo "  (skipped — ambition_ldtk_tools not importable from $ldtk_python)"
fi

# --- LDtk sprite tilesets (real sprites as editor visuals) ----------------
# Emit the LDtk-consumable visual manifest from the published sheets, then
# re-apply it so the worlds' sprite tilesets + entity tileRects stay in sync
# with the regenerated (gitignored) sheet PNGs. Unlike the fixed-grid
# editor-icon atlas, sprite frame sizes can change when a sheet is
# re-rendered, so this re-applies every run (LdtkTransaction only rewrites a
# .ldtk when something actually changed). Default is the curated entity map
# (a minimal diff); pass --all-sheets by hand to register every sheet for
# browsing in the editor.
echo "==> LDtk sprite tilesets:"
sprite_manifest="$sprites_dir/ldtk_sprite_manifest.json"
if ambition_python_exists "$python_bin" && \
    "$python_bin" -c 'import ambition_sprite2d_renderer' >/dev/null 2>&1 && \
    "$ldtk_python" \
        -c "import ambition_ldtk_tools" 2>/dev/null
then
    (cd "$renderer_dir" && "$python_bin" -m ambition_sprite2d_renderer \
        ldtk-manifest --out "$sprite_manifest") 2>&1 | sed 's/^/  /' || true
    for world in sandbox intro you_have_to_cut_the_rope; do
        ldtk_path="$worlds_dir/$world.ldtk"
        [ -f "$ldtk_path" ] || continue
        "$ldtk_python" \
            -m ambition_ldtk_tools.edit.visual_manifest apply-manifest \
            "$ldtk_path" "$sprite_manifest" --in-place 2>&1 | sed 's/^/  /' || true
    done
else
    echo "  (skipped — sprite renderer or ambition_ldtk_tools not importable)"
fi

# --- Write fingerprint on success ----------------------------------------
mkdir -p "$cache_dir"
echo "$current_fingerprint" > "$fingerprint_file"
echo "  cached regen fingerprint at $fingerprint_file"

echo "==> done"
