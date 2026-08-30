#!/usr/bin/env bash
# Re-render and publish the sprite suite, quality variants, and shared atlases.
# Registered targets come from the sprite renderer config; entity and standalone
# sheets are published into the actor-monolith sprite assets.
#
# Usage:
#   ./scripts/regen/sprites.sh
#   ./scripts/regen/sprites.sh --force
#   ./scripts/regen/sprites.sh --list
#   ./scripts/regen/sprites.sh --target <name>   # repeatable
#
# Environment:
#   AMBITION_SPRITE_PYTHON=/path/to/python
#   AMBITION_LDTK_PYTHON=/path/to/python
#   AMBITION_ULTRAPACK=0
#   AMBITION_ULTRAPACK_DEBUG=1
#   AMBITION_QUALITY_VARIANTS=0
#   LINE_PROFILE=1
#   AMBITION_LINE_PROFILE_DIR=/path
#   AMBITION_LINE_PROFILE_TEXT=1
#
# The renderer/config fingerprint plus expected published outputs form the
# incremental cache. --force bypasses it.
set -euo pipefail

# ⚠ TWO LEVELS UP: this script lives in `scripts/regen/`, not the repo root.
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

renderer_dir="$repo_root/tools/ambition_sprite2d_renderer"
ldtk_tools_dir="$repo_root/tools/ambition_ldtk_tools"
content_assets_dir="$repo_root/game/ambition_content/assets"
worlds_dir="$content_assets_dir/worlds"
character_catalog="$content_assets_dir/data/character_catalog.ron"
sandbox_ldtk="$worlds_dir/sandbox.ldtk"
hall_ldtk="$worlds_dir/hall_of_characters.ldtk"
sprites_dir="$repo_root/crates/ambition_platformer2d_actor_monolith/assets/sprites"
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
check_toolchain_only=0
# `--target` ACCUMULATES.
target_names=()
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
            target_names+=("$2")
            shift 2
            ;;
        --target=*)
            one="${1#--target=}"
            if [ -z "$one" ]; then
                echo "--target requires a target name" >&2
                exit 2
            fi
            target_names+=("$one")
            shift
            ;;
        --check-toolchain) check_toolchain_only=1; shift ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

if [ "$list_targets" -eq 1 ] && [ "${#target_names[@]}" -gt 0 ]; then
    echo "--list and --target are mutually exclusive" >&2
    exit 2
fi

if [ "$make_gifs" -eq 1 ] && [ "${#target_names[@]}" -eq 0 ]; then
    echo "--gif requires --target <name>" >&2
    exit 2
fi

# Capture the opt-in flag before clearing it from the orchestration process.
# Only selected expensive renderer subprocesses receive LINE_PROFILE. Leaving it
# set globally would make helper/import probes emit profiler summaries into
# command substitutions and corrupt their machine-readable stdout.
line_profile_requested="${LINE_PROFILE:-}"
unset LINE_PROFILE AMBITION_LINE_PROFILE_OUTPUT LINE_PROFILER_OWNER_PID

python_bin="$(ambition_select_tool_python "$renderer_dir" AMBITION_SPRITE_PYTHON)"
# ⭐ THE SAME PROBE, ANSWERED AS A YES/NO SO OTHER SCRIPTS CAN GATE ON IT.
# `--check-toolchain` exits 0 when this machine can render for real and non-zero
# when it would silently substitute. `regen/assets.sh` asks before running any
# category that reads this renderer.
check_toolchain() {
    local missing=0
    printf 'sprite renderer toolchain — %s\n' "$(hostname)"
    printf '  python : %s\n' "$python_bin"
    if (cd "$renderer_dir" && "$python_bin" -c 'import resvg_py' >/dev/null 2>&1); then
        printf '  resvg_py: present — SVG-rigged targets render for real\n'
    else
        printf '  resvg_py: ⛔ ABSENT — SVG-rigged targets (player_robot_v3) cannot render\n'
        printf '            declared in %s\n' "tools/ambition_sprite2d_renderer/pyproject.toml"
        missing=1
    fi
    # ⚠ THE USUAL CAUSE IS NOT A MISSING PACKAGE. It is a tool venv inside this
    # shared checkout whose interpreter belongs to another user.
    #
    # ⛔ BUT A BROKEN IN-REPO VENV IS ONLY A PROBLEM IF IT IS THE ONE IN USE.
    # `AMBITION_SPRITE_PYTHON` exists precisely so a second user on a shared
    # checkout can bring their own interpreter, and reporting "not usable" while
    # holding a working one would send them to fix something that does not
    # matter. Diagnose the venv; judge the INTERPRETER.
    local cfg="$renderer_dir/.venv/pyvenv.cfg" home
    if [ -f "$cfg" ]; then
        home="$(sed -n 's/^home = //p' "$cfg" | head -1)"
        if [ -n "$home" ] && [ ! -d "$home" ]; then
            # ⭐ THE QUESTION IS WHETHER THIS BROKEN VENV IS THE ONE IN USE, not
            # whether some particular env var was set. `tool_python.sh` resolves
            # a per-machine store BEFORE the in-repo `.venv`, so a second user
            # on a shared checkout gets a working interpreter with no env var at
            # all — and reporting "not usable" then would send them to fix
            # something they had already stopped depending on.
            if [ "$python_bin" != "$renderer_dir/.venv/bin/python" ]; then
                printf '  venv    : ⚠ %s/.venv is another user'"'"'s (%s)\n' \
                    "tools/ambition_sprite2d_renderer" "$home"
                printf '            ignored — this machine resolves its own interpreter above\n'
            else
                printf '  venv    : ⛔ %s/.venv points at %s, which does not exist here\n' \
                    "tools/ambition_sprite2d_renderer" "$home"
                printf '            the checkout is shared; that interpreter is another user'"'"'s\n'
                printf '            set AMBITION_SPRITE_PYTHON to a venv of your own, outside the repo\n'
                missing=1
            fi
        fi
    fi
    [ "$missing" -eq 0 ] && printf '  → usable\n' || printf '  → NOT usable; do not publish from this machine\n'
    return "$missing"
}


# ⭐ ANSWERED BEFORE ANY WORK. `--check-toolchain` is what `regen/assets.sh` asks
# before running a category that reads this renderer, so a machine that would
# silently substitute art refuses instead of publishing.
if [ "$check_toolchain_only" -eq 1 ]; then
    check_toolchain
    exit $?
fi
ldtk_python="$(ambition_select_tool_python "$ldtk_tools_dir" AMBITION_LDTK_PYTHON 0)"
ambition_require_python_module \
    "$python_bin" ambition_sprite2d_renderer \
    "run ./run_developer_setup.sh or set AMBITION_SPRITE_PYTHON=/path/to/python"
ambition_require_python_module \
    "$ldtk_python" ambition_ldtk_tools \
    "run ./run_developer_setup.sh or set AMBITION_LDTK_PYTHON=/path/to/python"
ambition_require_python_module \
    "$ldtk_python" PIL \
    "run ./run_developer_setup.sh so the LDtk tool installs its Pillow dependency"

shell_value_is_true() {
    case "${1,,}" in
        ""|0|false|no|off) return 1 ;;
        *) return 0 ;;
    esac
}

line_profile_enabled=0
line_profile_run_dir=""
if shell_value_is_true "$line_profile_requested"; then
    if "$python_bin" -c 'import line_profiler' >/dev/null 2>&1; then
        line_profile_enabled=1
        profile_root="${AMBITION_LINE_PROFILE_DIR:-$renderer_dir/.profiles}"
        profile_run_dir_name="regen-$(date -u +%Y%m%dT%H%M%SZ)-$$"
        line_profile_run_dir="$profile_root/$profile_run_dir_name"
        mkdir -p "$line_profile_run_dir"
        echo "==> line profiling enabled: $line_profile_run_dir"
    else
        echo "warning: LINE_PROFILE requested, but line_profiler is not installed in $python_bin" >&2
        echo "         install it once in the sprite renderer .venv; regeneration will continue unprofiled" >&2
    fi
fi

run_renderer_python() {
    local label="$1"
    shift
    if [ "$line_profile_enabled" -eq 1 ]; then
        local safe_label marker report_prefix started_at finished_at elapsed rc
        safe_label="$(printf '%s' "$label" | tr -c 'A-Za-z0-9._-' '_')"
        marker="$(mktemp "$line_profile_run_dir/${safe_label}.XXXXXX")"
        rm -f "$marker"
        report_prefix="$marker"
        started_at="$(date +%s)"
        echo "    [profile] start $label"
        if (
            cd "$renderer_dir"
            PYTHONUNBUFFERED=1 \
            AMBITION_RENDER_PROGRESS=1 \
            LINE_PROFILE="$line_profile_requested" \
            AMBITION_LINE_PROFILE_OUTPUT="$report_prefix" \
                "$python_bin" "$@"
        ); then
            rc=0
        else
            rc=$?
        fi
        finished_at="$(date +%s)"
        elapsed=$((finished_at - started_at))
        if [ "$rc" -eq 0 ]; then
            echo "    [profile] finish $label (${elapsed}s) -> ${report_prefix}.lprof"
            "$python_bin" "$repo_root/scripts/render_line_profiles.py" "${report_prefix}" || true
        else
            echo "    [profile] failed $label after ${elapsed}s (exit $rc)" >&2
        fi
        return "$rc"
    else
        # every renderer subprocess is TIMED, profiler or not.
        #
        # A number costs one `date` call per subprocess and turns both questions into arithmetic.
        local started_at finished_at rc
        started_at="$(date +%s)"
        if (cd "$renderer_dir" && "$python_bin" "$@"); then
            rc=0
        else
            rc=$?
        fi
        finished_at="$(date +%s)"
        regen_timings+=("$((finished_at - started_at)) $label")
        return "$rc"
    fi
}

# `<seconds> <label>` for every renderer subprocess this run has finished.
regen_timings=()

# checks RECORD here; only the end of the script exits. A postcondition that exits where it
# stands cancels every stage after it, and this pipeline's most expensive stages — ultrapack, and
# now the quality variants — are last. Failures are printed where they are found, the remaining
# stages still run, and the exit code is settled at the bottom.
regen_failures=()

# What the run COST, slowest first. Printed at the end of a full batch and after
# `--target`, so a one-target check answers the same question a full run does.
print_regen_timings() {
    [ "${#regen_timings[@]}" -eq 0 ] && return 0
    local total=0 entry seconds
    for entry in "${regen_timings[@]}"; do
        seconds="${entry%% *}"
        total=$((total + seconds))
    done
    echo ""
    echo "==> render cost: ${#regen_timings[@]} subprocess(es), ${total}s total"
    # Not observed here — bash's short input never made `sort` write past the close — but the cost
    # of not depending on that is one word.
    printf '%s\n' "${regen_timings[@]}" | sort -rn | awk 'NR <= 12' | while read -r seconds label; do
        printf '    %5ss  %s\n' "$seconds" "$label"
    done
}


# --- Readable line-profile reports -----------------------------------------
# Per-phase profiling uses ``kernprof -o``, which intentionally writes only a
# binary .lprof database. Convert databases created by this invocation to text
# on normal exit and after Ctrl-C. The binary database remains the source of
# truth and can still be reopened interactively.
sprite_profile_marker=""
finish_sprite_profile_reports() {
    local status="$?"
    trap - EXIT
    if [ -n "$sprite_profile_marker" ]; then
        "$python_bin" "$repo_root/scripts/render_line_profiles.py" \
            --newer-than "$sprite_profile_marker" \
            "$renderer_dir/.profiles" || true
        rm -f "$sprite_profile_marker"
    fi
    exit "$status"
}

case "${LINE_PROFILE:-0}" in
    ""|0|false|False|no|No|off|Off) ;;
    *)
        mkdir -p "$renderer_dir/.profiles"
        sprite_profile_marker="$(mktemp "$renderer_dir/.profiles/.regen-start.XXXXXX")"
        trap finish_sprite_profile_reports EXIT
        ;;
esac

# A full regeneration emits hundreds of file paths, so it stays collapsed to a
# count and a directory. A FOCUSED run is the opposite case: you asked for two
# targets because you are about to go and look at what they wrote, and the
# summary alone makes you guess the filenames. `both` prints the paths AND the
# count. Either is still overridable from the environment.
export AMBITION_SPRITE_PROGRESS="${AMBITION_SPRITE_PROGRESS:-1}"
if [ "${#target_names[@]}" -gt 0 ]; then
    export AMBITION_SPRITE_PATH_OUTPUT="${AMBITION_SPRITE_PATH_OUTPUT:-both}"
else
    export AMBITION_SPRITE_PATH_OUTPUT="${AMBITION_SPRITE_PATH_OUTPUT:-summary}"
fi

list_sprite_targets() {
    echo "==> registered sprite targets"
    echo "    Use: ./scripts/regen/sprites.sh --target <target>"
    echo
    (cd "$renderer_dir" && "$python_bin" -m ambition_sprite2d_renderer list)
}

validate_sprite_targets() {
    # Validate the complete focused batch before publishing anything. This
    # prevents `--target valid --target typo` from installing the first target
    # and only then failing on the misspelled second target.
    (
        cd "$renderer_dir"
        "$python_bin" - "${target_names[@]}" <<'PY'
import difflib
import sys

from ambition_sprite2d_renderer.registry import discover_all_targets

requested = sys.argv[1:]
available = sorted(discover_all_targets().targets)
available_set = set(available)
unknown = [name for name in requested if name not in available_set]
if not unknown:
    raise SystemExit(0)

for name in unknown:
    print(f"unknown sprite target: {name}", file=sys.stderr)
    matches = difflib.get_close_matches(name, available, n=1, cutoff=0.60)
    if matches:
        print(f"Did you mean '{matches[0]}'?", file=sys.stderr)
    else:
        print("Run ./scripts/regen/sprites.sh --list to see registered targets.", file=sys.stderr)

raise SystemExit(2)
PY
    )
}

regen_one_target() {
    local target="$1"
    local dest_root="$sprites_dir"

    # Every target installs to assets/sprites/ and owns any further subdirectory
    # behavior in its own Python `Target.install` hook — gnu_ton_boss,
    # gnu_ton_apple, interdimensional_gate, pirate_heavy, mockingbird_boss,
    # sanic_support_entities, and `entities`.
    #
    # That made this script right and the CLI wrong: anyone running `python3 -m
    # ambition_sprite2d_renderer publish entities` installed a directory too high, where the runtime
    # loader never looks, and nothing in the chain said so. A rule that lives in one of two callers
    # is a rule the other caller breaks.

    echo "==> sprite target: $target → $dest_root"
    run_renderer_python "publish-$target" -m ambition_sprite2d_renderer publish "$target" --dest-root "$dest_root"
    if [ "$make_gifs" -eq 1 ]; then
        echo "==> animation GIFs: $target → $renderer_dir/generated/gifs/$target"
        run_renderer_python "gifs-$target" -m ambition_sprite2d_renderer gifs "$target"
    fi
}

if [ "$list_targets" -eq 1 ]; then
    list_sprite_targets
    exit 0
fi

if [ "${#target_names[@]}" -gt 0 ]; then
    validate_sprite_targets
    for one in "${target_names[@]}"; do
        regen_one_target "$one"
    done
    print_regen_timings
    exit 0
fi

# --- Publish roster -------------------------------------------------------
# ONE list of what this script publishes, and it is a list of TARGETS.
#
# In the other direction `patent_clerk` was published and never listed, so a run whose patent-clerk
# render failed reported every expected file present.
#
# The filenames are now DERIVED from these arrays by asking the renderer what
# each target declares it installs (`Target.claimed_install_names`). A target
# added below is covered the moment it is added, and a filename that no target
# produces cannot be named at all.

review_cues=(
    # Toon-target NPC variants already promoted.
    absurd_general architect kernel_guide vault_keeper
    # oiler is NOT here any more — see `tackon_targets`. His body comes from
    # the direct-SVG rig; leaving the review cue in place would have this loop
    # overwrite the rig's sheet with the toon render on every full run.
    merchant_prototype erdish raid_enforcer fascist_enforcer
    # Named characters whose YAML manifests already live in $sprites_dir.
    alice bob craig eve general_hero judy mallory olivia
    peggy sybil trent trudy victor walter
    # Phase 6 + bonus follow-up: every review config is now an
    # actual catalog character. Install the rest so the Hall of
    # Characters has a sprite for each.
    #
    # ⛔⛔ THE GOBLIN WEAPON VARIANTS WERE THE "REST" AND WERE NEVER ADDED, and
    # a TEST depended on one of them. `ambition_render`'s
    # `a_left_drawn_character_faces_the_way_they_are_going_like_a_right_drawn_one`
    # uses `goblin_cave_dagger` as its canonical RIGHT-drawn sheet, and
    # `record_for_sheet_key` returned `None` because no roster line ever
    # published it — so `cargo test --workspace --lib` failed on a missing
    # ASSET while reading like a handedness regression. Found 2026-08-30, the
    # first time that gate was run to completion (D-QTT-1).
    #
    # ⭐ THE LESSON IS THE ONE THIS BLOCK ALREADY TEACHES ONE COMMENT UP: a
    # target that no batch publishes does not exist for anybody who did not
    # render it by hand, and generated art is gitignored, so "it works on my
    # checkout" is the expected symptom rather than a surprising one.
    goblin_brute_hammer goblin_cave_dagger goblin_desert_bow
    goblin_forest_spear goblin_frost_sword goblin_shaman_staff
)

# Faction-leader cues copied out of the `draw-factions` scratch render. Their
# YAML lives in configs/factions/, which no discovery surface registers, so
# these are named products rather than registered targets.
faction_cues=(goblin_cantina_chieftain pulse_voyager_captain tech_bro_disruptor)

tackon_targets=(
    # That file is deleted; this line is what keeps her published.
    noether
    oiler
    # The two Fighting Polygons are named here because a `--target` render is
    # not a PUBLISH ROSTER. Both were rendered into this checkout one target at
    # a time (`scripts/regen/sprites.sh --target <name>`), which works and is the right
    # surgical tool — but generated art is gitignored, so a target that no batch
    # names exists only on the machine that once rendered it and is ABSENT from a
    # fresh clone. `character_catalog.ron` names both sheets, so a clone would
    # come up with two catalog rows pointing at files nothing can produce.
    # `test_every_catalog_character_names_a_sheet_regen_publishes` is what said
    # so, and it is the second time this session that a check caught art existing
    # only by accident of local history.
    pointed_polygon
    pugnacious_polygon
    # The two easter-egg fighters, for the same reason as the polygons above and
    # by the same test: `character_catalog.ron` names their sheets, so a roster
    # that omits them is a fresh clone with two rows pointing at nothing.
    author
    officer
    # ⛔ AND THE NEXT PAIR REPEATED IT, four lines under the comment explaining
    # it. `performer` (then `actor`) and `medic` arrived with catalog rows and no
    # roster entry, so the same test failed the same way on 2026-08-27. A
    # hand-kept roster beside a hand-kept catalog is two lists that agree only
    # when somebody remembers both — which is exactly what this check exists to
    # notice, and it did. ⚠ AND IT IS A THIRD PLACE A RENAME HAS TO REACH: this
    # list is keyed by TARGET NAME, so `actor` -> `performer` had to land here
    # too or a fresh clone regenerates a sheet the catalog no longer names.
    performer
    medic
    # NAMED HERE, not only reachable by `--target`. The game loads
    # `sprites/hud_stock_icon.png` by path from `STOCK_ICON_ASSET`, so a clone
    # that cannot produce it has a match HUD with holes where the stocks go —
    # the same trap the two polygons hit, one comment down.
    hud_icons
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
    # The Projectile Polygon's charge shot. A projectile is its own sheet
    # because it OUTLIVES the pose that fired it — five tiers plus spawn and
    # two impacts, none of which a character row can carry.
    polygon_charge_shot
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
    # Both must publish or a fresh clone draws the powered-up player as a colored rectangle.
    super_mary_o_tall
    super_mary_o_fire
    # Mary-O's gameplay provider binds these generated pickups through
    # WorldItemArt at sprites/props/<name>.png. Publish the source targets
    # here, then copy their canonical poses into props/.
    super_mary_o_star_wand
    super_mary_o_cinder_beacon
    super_mary_o_cosmic_quasar
    # Fixed-canvas construction pieces. The pipe and pole body targets repeat
    # vertically; their top/finial and flag stay separate so level code can
    # build arbitrary heights without stretching any sprite.
    super_mary_o_pipe_body
    super_mary_o_pipe_top
    super_mary_o_flag_pole_body
    super_mary_o_flag_pole_top
    super_mary_o_flag
    # Reusable hand-authored presentation marks: impacts, dust/poofs, glints,
    # directional release flashes, and charge/release effects. The generated
    # authoring sidecar carries placement/orientation/loop intent for the
    # presentation integration layer.
    generic_action_fx
    # Complementary generic marks for motion, shields/status, teleport/phase,
    # water/ambient accents, and elemental electricity/ice. Kept as a separate
    # sheet so the generic action catalog does not grow past friendly texture
    # dimensions as the authored VFX vocabulary expands.
    generic_world_fx
    # Third reusable catalog: smoke/gas, goo/corrosion, sonic/psychic, time,
    # ritual magic, mechanical debris, nature/spores, sand, and shadow effects.
    generic_exotic_fx
    # Detached character-specific presentation catalogs. These complement the
    # body-integrated character renderers with world/target/projectile-space
    # effects while preserving authored timing, anchors, and semantic intent.
    carl_stargan_vfx
    noether_vfx
    patent_clerk_vfx
    pca_vfx
    projectile_polygon_vfx
    # Detached leader-specific effects for the Pirate Admiral and Shadow Oni
    # Leader. Their metadata stays character-contextual while the runtime
    # presentation seam remains generic.
    pirate_admiral_vfx
    ninja_shadow_oni_leader_vfx
    # these two were AUTHORED IN THE SUBMODULE AND NEVER LISTED HERE
    # . `george_booul_vfx` ships a published sheet only because
    # someone ran it with a focused `--target`, so a fresh clone's regen would
    # silently drop it; `oiler_vfx` has a renderer target, no published sheet at
    # all, and no cues in `sfx.bank`. A regen roster that omits a real target is
    # exactly the fresh-clone hazard this list exists to close.
    george_booul_vfx
    oiler_vfx
    generic_explosions
    smirking_behemoth_boss
    solid_snake
    snakes_on_a_paper_plane
    snakes_on_a_cartesian_plane
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
    hand_saint
    helpful_liar
    mantis_lancer
    ninja_heavy
    pirate_cutlass_viper
    president_portrait
    puppy_slug_variant2
    raptor_stalker
    # robot_heavy is a multi-variant rig whose publisher doesn't
    # install (renders only to generated/, no install method).
    # Skipping it here keeps the working tree clean. Catalog
    # entry was dropped along with the publisher work.
    smart_house
    spaghetti_event
    synthetic_friend
    trex_enemy
    viking_heavy_shieldmaiden
    viking_heavy_warrior
    viking_shieldmaiden
    viking_warrior
    weird_hermit
    willson
    ramen_nujan
    jeff_hinter
    jeff_hinter_armored
    m_leblanc
    puppy_slug_velvet
    player_robot_fable
    # FOUR surfaces name them: the Mary-O demo's three forms, `pocket_runner`, `twintrack_traveler`,
    # and Ambition's own versus arena (`arena_duelist_close`).
    #
    # how it was NOTICED is the part worth keeping: not by a clone failing,
    # but by six blank faces in a new character-select grid. `publish` emits a
    # target's PORTRAIT products as well as its sheet, so a target no batch
    # publishes has neither — and the portrait half is what was visible, because
    # `super_mary_o_portraits.png` sat next door looking like coverage.
    mary_o_v2
    mary_o_v2_fire
    mary_o_v2_tall
    # If there are we need to fix that."* There were TWENTY-SIX — every catalog sheet stem this
    # script never named, cross-checked against `sprite2d_renderer list`, and all 26 are registered
    # targets that simply nothing ran.
    #
    # they are almost all ONE cast: the mathematician / scientist NPCs of the
    # Hall. Their art has existed on developer machines for months and would
    # have been absent from a fresh clone, which is the same defect the five
    # rows above were added for and the same one `mary_o_v2` was.
    admiral_grass_hopper
    anne_druid
    carl_runga
    carl_stargan
    data_lovelace
    davy_hylbert
    genghis_can
    genghis_cant
    georg_canter
    george_booul
    hunny_horror_boss
    hypatia_prime
    joseph_furrier
    le_beast
    leib_knives
    mami_marzakhani
    marie_curry
    martin_cutta
    neil_ongras_turfson
    busy_beaver
    charley_beagle_svg
    niels_boar
    vera_ruin
    paradox_barber
    patent_clerk
    paul_diracula
    player_robot_v2
    player_robot_v3
    projectile_polygon
    python_goras
    richard_duckling
    yuclid
)

# Rigged characters authored as GUI `.rig.json` documents auto-register as
# targets named after the file stem. Include them in the same explicit batch as
# the ordinary tack-ons so registry discovery is paid once for the whole group.
rig_targets=()
for rig in "$renderer_dir"/ambition_sprite2d_renderer/targets/characters/rigged/*.rig.json; do
    [ -f "$rig" ] || continue
    rig_targets+=("$(basename "$rig" .rig.json)")
done

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

# render-target-name -> runtime props/ basename
held_prop_map=(
    "pirate_heavy_axe:axe"
    "throwing_javelin:javelin"
    "lasersword_with_guns:gunsword"
    "portal_gun_blue:portal_gun_blue"
    "portal_gun_orange:portal_gun_orange"
    "super_mary_o_star_wand:super_mary_o_star_wand"
    "super_mary_o_cinder_beacon:super_mary_o_cinder_beacon"
    "super_mary_o_cosmic_quasar:super_mary_o_cosmic_quasar"
)

construction_prop_map=(
    "super_mary_o_pipe_body:mary_o_pipe_body"
    "super_mary_o_pipe_top:mary_o_pipe_top"
    "super_mary_o_flag_pole_body:mary_o_flag_pole_body"
    "super_mary_o_flag_pole_top:mary_o_flag_pole_top"
    "super_mary_o_flag:mary_o_flag"
)

# Every registered target this script publishes. `draw-all` owns a declared
# runtime subset of the main CharacterJob configs; the expected-file helper below
# imports that SAME renderer contract rather than assuming every configs/*.yaml
# document is a runtime publication.
publish_targets=(
    entities
    "${review_cues[@]}"
    "${tackon_targets[@]}"
    "${rig_targets[@]}"
    "${pirate_targets[@]}"
    puppy_slug
    mockingbird_boss
)

# The runtime-required file list. Consumed twice: by the cache fast-path below
# (a deleted asset re-triggers a render) and by the postcondition at the end.
#
# diagnostics are excluded because `sweep_runtime_diagnostics.py` MOVES them
# out of the runtime root at the end of every run — requiring them would make
# the fast path unsatisfiable and force a full re-render every time.
# `*_actor.ron` and the tileset/manifest `.ron` sidecars are excluded because
# the installer copies them opportunistically: 21 registered targets declare one
# and do not ship it.
declare_expected_files() {
    (
        cd "$renderer_dir"
        "$python_bin" - "$@" <<'DECLARE_EXPECTED'
import sys
from pathlib import Path

from ambition_sprite2d_renderer.registry import (
    RUNTIME_ADAPTER_CONFIG_STEMS,
    discover_all_targets,
    load_jobs,
)

DIAGNOSTIC_SUFFIXES = (
    "_canonical.png",
    "_canonical_transparent.png",
    "_preview_labeled.png",
    "_parts_debug.png",
    "_debug.png",
)


def runtime_required(rel: str) -> bool:
    name = Path(rel).name
    if name == "canonicals_contact_sheet.png" or name.endswith(DIAGNOSTIC_SUFFIXES):
        return False
    if rel.endswith(".ron") and not (
        rel.endswith("_spritesheet.ron") or rel.endswith("_portraits.ron")
    ):
        return False
    return True


report = discover_all_targets()
names = list(sys.argv[1:])
names += [
    job.output_stem(path)
    for path, job in load_jobs(Path("ambition_sprite2d_renderer/configs"))
    if path.stem in RUNTIME_ADAPTER_CONFIG_STEMS
]

unknown = sorted({n for n in names if n not in report.targets})
if unknown:
    for name in unknown:
        print(f"publish roster names an unregistered target: {name}", file=sys.stderr)
    print("Run ./scripts/regen/sprites.sh --list to see registered targets.", file=sys.stderr)
    raise SystemExit(2)

emitted = []
seen = set()
for name in names:
    for rel in report.targets[name].claimed_install_names():
        if runtime_required(rel) and rel not in seen:
            seen.add(rel)
            emitted.append(rel)
print("\n".join(emitted))
DECLARE_EXPECTED
    )
}

expected_list="$(declare_expected_files "${publish_targets[@]}")"
mapfile -t expected_files <<< "$expected_list"
# Products this script copies by hand rather than installing through a target.
for cue in "${faction_cues[@]}"; do
    expected_files+=(
        "${cue}_spritesheet.png" "${cue}_spritesheet.yaml" "${cue}_spritesheet.ron"
        "${cue}_portraits.png" "${cue}_portraits.ron"
    )
done
for pair in "${held_prop_map[@]}" "${construction_prop_map[@]}"; do
    expected_files+=("props/${pair##*:}.png")
done

# The roster has published 800+ files for a year; anything under half that means the helper broke,
# not that the roster shrank.
if [ "${#expected_files[@]}" -lt 400 ]; then
    echo "expected-file derivation produced only ${#expected_files[@]} entries" >&2
    echo "— the roster or the renderer registry is broken; refusing to run" >&2
    exit 1
fi

# --- Fingerprint cache ----------------------------------------------------
# Hash every .py and .yaml under the renderer module + the boss generator
# script. If the hash matches the cached value AND every expected sheet
# is already present in $sprites_dir, skip the whole regen.
#
# The expected-files list is the same one the postcondition validates at
# the end. Keeping a single source of truth means deleting one published
# sheet, manifest, or portrait trips both the fast-path and postcondition.

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
            sha256sum "$repo_root/scripts/regen/sprites.sh" \
                | awk -v root="$repo_root/" '{sub(root, "", $2); print}'
            # ⛔⛔ THE TOOLCHAIN IS AN INPUT. The same source art renders
            # DIFFERENTLY, or not at all, depending on whether the native SVG
            # rasteriser is importable — `resvg-py` is declared in the
            # renderer's pyproject, and v3 of the player robot is SVG-rigged
            # where v2 is not. A checkout whose tool venv is missing or points
            # at another user's Python silently resolves to a bare `python3`
            # without it.
            #
            # ⚠ MEASURED 2026-08-29: all three `tools/*/.venv/pyvenv.cfg` in
            # this shared checkout name `/home/joncrall/.local/share/uv/...`,
            # so the venv is real for one user and interpreter-less for anyone
            # else on the same filesystem. Without this line both produce
            # artifacts under one fingerprint and each treats the other's as
            # current.
            renderer_capability_fingerprint
        }
    ) | sha256sum | awk '{print $1}'
}


# What the renderer can actually DO on this machine, as a hashable line.
# Reports the reason rather than a bare boolean so a cache miss is explicable
# from the fingerprint file alone.
renderer_capability_fingerprint() {
    local probe
    probe="$(run_renderer_python capability-probe -c '
import importlib.util, sys
print("python", ".".join(map(str, sys.version_info[:2])))
for mod in ("resvg_py", "cairosvg", "PIL"):
    spec = importlib.util.find_spec(mod)
    print(mod, "present" if spec is not None else "ABSENT")
' 2>/dev/null || printf 'capability-probe FAILED\n')"
    printf 'capability %s\n' "$(printf '%s' "$probe" | tr '\n' ';')"
}

all_outputs_present() {
    # `expected_files` is the single source of truth shared with the
    # postcondition (see its comment above) — including the portrait PNGs that
    # actually ship. Hall portrait coverage is deliberately NOT consulted here:
    # it asks about all 128 catalog rows, a dozen of which no render list
    # produces, so `inspect_hall_portraits` always exits 1 and folding it in
    # made this fast path unsatisfiable — every regen re-rendered every sheet.
    local rel
    for rel in "${expected_files[@]}"; do
        if [ ! -f "$sprites_dir/$rel" ]; then
            return 1
        fi
    done
    return 0
}

# --- Per-sheet cache ------------------------------------------------------
# The global cache commits only after a fully successful run. Per-sheet keys
# allow an interrupted run to resume without rebuilding current targets. Each
# key combines shared renderer infrastructure with that target's leaf generator;
# shared helper changes invalidate every target, leaf changes only that target.
# Target modules must not import sibling non-underscore leaf modules, because
# those siblings are intentionally outside another target's cache key.
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
        # NB: this orchestrator (`scripts/regen/sprites.sh`) is deliberately NOT
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

# The digests of everything this unit published, in the order the glob yields
# them. Stored beside the key so freshness can ask whether the files on disk are
# THE ONES THIS CACHE PRODUCED, rather than whether some file is present.
sheet_output_digests() {
    local glob="$1"
    compgen -G "$glob" >/dev/null 2>&1 || return 1
    # shellcheck disable=SC2086
    sha256sum $glob 2>/dev/null \
        | awk -v root="$sprites_dir/" '{sub(root, "", $2); print $1, $2}' \
        | sort
}

# Fresh iff the stored key matches AND every published output still hashes to
# what this cache recorded when it wrote them.
#
# ⛔⛔ IT USED TO ASK ONLY WHETHER A FILE EXISTED (`compgen -G "$glob"`), AND
# THAT IS HOW A CHARACTER SHIPPED WEARING ANOTHER CHARACTER'S ART. On
# 2026-08-29 `sprites/player_robot_v3_spritesheet.png` was byte-identical to
# `player_robot_v2`'s — 3.9MB of the wrong robot, published under v3's name. The
# file existed, so this function said "fresh", so every later run skipped it. It
# took a hand-run `md5sum` across the whole directory to notice, and the game had
# been drawing the wrong body for a day.
#
# ⭐ THE INPUTS WERE ALREADY CONTENT-ADDRESSED — `compute_fingerprint` hashes
# the renderer, `leaf_hash` hashes each target's generator. Only the OUTPUTS
# were taken on trust. A build that hashes what it reads and not what it wrote
# cannot tell "I made this" from "something is there".
sheet_cache_fresh() {
    local unit="$1" key="$2" glob="$3" stored stored_key stored_digests now_digests
    [ "$force_regen" -ne 1 ] || return 1
    [ -f "$sheets_cache_dir/$unit" ] || return 1
    stored="$(cat "$sheets_cache_dir/$unit")"
    stored_key="$(printf '%s\n' "$stored" | head -1)"
    [ "$stored_key" = "$key" ] || return 1

    now_digests="$(sheet_output_digests "$glob")" || return 1
    stored_digests="$(printf '%s\n' "$stored" | tail -n +2)"
    # An entry written before outputs were recorded has no digest block. Treat
    # it as stale rather than trusting it: re-rendering one sheet is cheap and
    # this is exactly the population that may be carrying the defect above.
    [ -n "$stored_digests" ] || return 1
    [ "$stored_digests" = "$now_digests" ] || return 1
    return 0
}

sheet_cache_store() {
    local unit="$1" key="$2" glob="${3-}"
    mkdir -p "$sheets_cache_dir"
    {
        printf '%s\n' "$key"
        [ -n "$glob" ] && sheet_output_digests "$glob"
    } > "$sheets_cache_dir/$unit"
}

# Publish registered targets in explicit batches. Registry discovery and YAML
# config loading are expensive, so cache inspection resolves every target's
# declared portrait products in one Python process and stale targets are then
# rendered by one `publish-many` process. This preserves per-target cache keys
# while avoiding one interpreter + discovery pass per character.
_target_sheet_glob() {
    local target="$1"
    case "$target" in
        gnu_ton_boss|mockingbird_boss)
            printf '%s\n' "$sprites_dir/$target/${target}*_spritesheet.png"
            ;;
        *)
            printf '%s\n' "$sprites_dir/${target}*_spritesheet.png"
            ;;
    esac
}

publish_cached_batch() {
    local label="$1"
    shift
    local -a candidates=("$@")
    local -a stale=()
    local target key glob rel records portraits_ok records_ok=1
    local -A portrait_records=()

    [ "${#candidates[@]}" -gt 0 ] || return 0

    if [ "$force_regen" -ne 1 ]; then
        if ! records="$(
            cd "$renderer_dir" && "$python_bin" \
                -m ambition_sprite2d_renderer portrait-files \
                --with-target "${candidates[@]}"
        )"; then
            echo "  warning: could not resolve portrait products for batch '$label'; republishing it" >&2
            records=""
            records_ok=0
        fi
        while IFS=$'\t' read -r target rel; do
            [ -n "$target" ] || continue
            portrait_records["$target"]="${portrait_records[$target]-}$rel"$'\n'
        done <<< "$records"
    fi

    for target in "${candidates[@]}"; do
        key="$(unit_key "$target")"
        glob="$(_target_sheet_glob "$target")"
        if [ "$records_ok" -eq 1 ] \
            && [ -n "${portrait_records[$target]-}" ] \
            && sheet_cache_fresh "$target" "$key" "$glob"; then
            portraits_ok=1
            while IFS= read -r rel; do
                [ -n "$rel" ] || continue
                if [ ! -f "$sprites_dir/$rel" ]; then
                    portraits_ok=0
                    break
                fi
            done <<< "${portrait_records[$target]-}"
            if [ "$portraits_ok" -eq 1 ]; then
                echo "  [cache] $target up to date — skipped"
                continue
            fi
        fi
        stale+=("$target")
    done

    [ "${#stale[@]}" -gt 0 ] || return 0

    echo "  publishing ${#stale[@]} target(s) in one process"
    if run_renderer_python "publish-batch-$label" \
        -m ambition_sprite2d_renderer publish-many \
        --quiet --dest-root "$sprites_dir" "${stale[@]}"; then
        for target in "${stale[@]}"; do
            sheet_cache_store "$target" "$(unit_key "$target")" \
                "$(_target_sheet_glob "$target")"
        done
    else
        echo "  [warn] batch '$label' had one or more publish failures; cache keys were not advanced" >&2
    fi
}

# --- Reduced-resolution quality variants ----------------------------------
# a sprite regen that does not run this leaves the phone on full-res art.
# The half / quarter / potato roots are what the runtime loads under the Low /
# Medium / Potato quality profiles, and a sheet with no variant silently falls
# back to full resolution. `scripts/regen/assets.sh` chained backgrounds → sprites →
# variants and this script did not, so every standalone `./scripts/regen/sprites.sh`
# re-opened that drift; 25 sheets — Mary-O's and the player's among them — had
# no half-res sibling when it was measured.
#
# THE FINGERPRINT CANNOT ANSWER THIS QUESTION. It covers renderer sources and
# the presence of full-res outputs; it says nothing about whether the reduced
# tiers match them, and something that publishes art without reaching the bottom
# of this script (an interrupted run, `AMBITION_QUALITY_VARIANTS=0`, a publish
# through the renderer directly) leaves a gap the next run then declares fresh.
# So the variant stage runs on BOTH paths — it is the only stage whose staleness
# the cache key does not describe.
#
# It is cheap to chain because the generator is incremental: a run where nothing
# changed costs ~4s, and one changed character ~7s (a full rebuild is ~2m15s).
#   AMBITION_QUALITY_VARIANTS=0  skip it (same idiom as AMBITION_ULTRAPACK=0)
run_quality_variants() {
    echo "==> reduced-resolution quality variants (sprites):"
    if [ "${AMBITION_QUALITY_VARIANTS:-1}" = "0" ]; then
        echo "  (skipped — AMBITION_QUALITY_VARIANTS=0)"
        return 0
    fi
    "$python_bin" "$repo_root/scripts/generate_visual_quality_variants.py" \
        --asset-root "$repo_root/crates/ambition_platformer2d_actor_monolith/assets" \
        --sprites-only 2>&1 | sed 's/^/  /'
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
    # NOT `exit 0` — see `run_quality_variants`. Skipping publication is what
    # the cache key licenses; skipping the tier the key says nothing about is how
    # the reduced-resolution roots fell four days behind the art.
    if ! run_quality_variants; then
        echo "" >&2
        echo "==> regen FAILED — quality variants: generator reported a failure" >&2
        exit 1
    fi
    exit 0
fi

# Cheap structural preflight before the first expensive render. The adapter config surface is small
# enough to validate up front.
echo "==> validate sprite character configs"
run_renderer_python validate-configs -m ambition_sprite2d_renderer validate-configs

echo "==> config-driven targets (robot / goblin / boss) → $sprites_dir"
run_renderer_python draw-all -m ambition_sprite2d_renderer draw-all --out-dir "$sprites_dir"

echo "==> entity sprites → $entities_dir"
# `$sprites_dir`, not `$entities_dir`: the target's own `install` hook adds the
# `entities/` leg now. Passing the subdirectory here as well would nest it
# (`sprites/entities/entities/`) — see the note in `regen_one_target`.
run_renderer_python publish-entities -m ambition_sprite2d_renderer publish entities --dest-root "$sprites_dir"

echo "==> review NPC sheets (toon-target NPCs) → $sprites_dir"
# `draw-review` renders configs/review/*.yaml (toon-target NPC
# variants such as absurd_general, architect, kernel_guide). We
# render to a scratch dir, then copy the specific sheets we use
# in-game into $sprites_dir. Promoting a review config to a
# permanent runtime sheet means: add the cue id to `review_cues` in
# the publish roster AND give it a `character_catalog.ron` entry (specs are built
# from the sheet RON at load; the old `*_SHEET` statics in
# character_sprites are gone).
review_scratch="$renderer_dir/generated/review"
mkdir -p "$review_scratch"
run_renderer_python draw-review -m ambition_sprite2d_renderer draw-review --out-dir "$review_scratch"
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
    for ext in png ron; do
        src="$review_scratch/${cue}_portraits.$ext"
        if [ -f "$src" ]; then
            cp "$src" "$sprites_dir/${cue}_portraits.$ext"
            echo "  installed ${cue}_portraits.$ext"
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
run_renderer_python draw-factions -m ambition_sprite2d_renderer draw-factions --out-dir "$factions_scratch"
for cue in "${faction_cues[@]}"; do
    for ext in png yaml ron; do
        src="$factions_scratch/${cue}_spritesheet.$ext"
        if [ -f "$src" ]; then
            cp "$src" "$sprites_dir/${cue}_spritesheet.$ext"
            echo "  installed ${cue}_spritesheet.$ext"
        else
            echo "  WARN: $src missing — skipped"
        fi
    done
    for ext in png ron; do
        src="$factions_scratch/${cue}_portraits.$ext"
        cp "$src" "$sprites_dir/${cue}_portraits.$ext"
        echo "  installed ${cue}_portraits.$ext"
    done
done

echo "==> tack-on targets (render-publish into $sprites_dir)"
# `tackon_targets` in the publish roster is every registered module target
# whose manifest the runtime loads. The registry is `registry/discovery.py`
# (auto-discovered from targets/<category>/ — run `list` to see it); keep that
# roster covering every target the game references (mockingbird_boss and the
# pirates ride the late-targets batch below).
publish_cached_batch tackons "${tackon_targets[@]}" "${rig_targets[@]}"

echo "==> held-item prop canonicals (single-pose → $sprites_dir/props)"
# A few props are shown in-game as STATIC held / ground items, which load
# the single-pose `*_canonical_transparent.png` (not the animated sheet the
# tack-on publish installs). Copy those canonicals flat into props/ so the
# runtime asset paths (`sprites/props/<name>.png`) resolve on a fresh clone.
props_dir="$sprites_dir/props"
mkdir -p "$props_dir"
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

echo "==> Mary-O construction-piece canonicals (fixed canvas → $props_dir)"
# These are level-construction sprites rather than held items. Keep their
# transparent fixed canvases: seam coordinates and attachment anchors are part
# of the authoring contract, so they must never be auto-cropped here.
for pair in "${construction_prop_map[@]}"; do
    src_target="${pair%%:*}"
    dst_name="${pair##*:}"
    canon="$renderer_dir/generated/$src_target/${src_target}_canonical_transparent.png"
    if [ -f "$canon" ]; then
        cp "$canon" "$props_dir/${dst_name}.png"
        echo "    $src_target -> props/${dst_name}.png"
    else
        echo "    warning: missing $canon (Mary-O construction prop not rendered)" >&2
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

echo "==> pirate, small-enemy, and multipart-boss target batch → $sprites_dir"
# Pirates are registered as tack-on `[characters]` targets and publish through
# the same machinery as the other tack-ons. These late targets share one
# publishing contract, so they render together and the registry + YAML configs
# are loaded once rather than once per target.
publish_cached_batch late-targets \
    "${pirate_targets[@]}" \
    puppy_slug \
    mockingbird_boss

echo "==> postcondition: every runtime-required sprite file present"
# Walk the derived list of files the runtime actually loads and REPORT any that
# are missing. The list comes from the publish roster near the top of this
# script (it's also consumed by the cache-skip check), so adding a target is
# all it takes to cover its products.
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
    regen_failures+=("postcondition: ${#missing[@]} runtime-required file(s) missing")
else
    echo "  ok: ${#expected_files[@]} expected files present"
fi

# Coverage is a REPORT, not a gate. `inspect_hall_portraits` exits 1 whenever any
# catalog row lacks a portrait — useful for a `--check` caller, fatal here under
# `set -euo pipefail`, which aborted the whole regen (and every phase after it)
# over Hall rows that no render list produces. The postcondition above is the
# gate for runtime-required files; this tells the author what else is missing.
echo "==> Hall-of-Characters portrait coverage:"
if ! "$ldtk_python" -m ambition_ldtk_tools.inspect_hall_portraits \
    --catalog "$character_catalog" --sprites-dir "$sprites_dir" \
    --only-issues 2>&1 | sed 's/^/  /'; then
    echo "  note: portrait coverage is incomplete (reported above); not fatal."
fi

echo "==> portrait review gallery:"
run_renderer_python portrait-gallery -m ambition_sprite2d_renderer portrait-gallery \
    --source-dir "$sprites_dir" \
    --out "$renderer_dir/generated/portrait_gallery.png" 2>&1 | sed 's/^/  /'

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

# --- Ultrapacked quality-tier sprite atlases (runtime install) ------------ Pool every
# published per-target sheet into shared, uniformly-sized atlas pages at each quality tier, then
# write pages + a SpritePackCatalog into the RUNTIME pack root assets/sprite_packs/<tier>/
# (gitignored, generated). Tier names match the runtime `TextureResolutionScale` enum (full /
# half / quarter / potato) — the game's pack consumer selects the tier dir from the active
# quality budget. `build.rs` bakes each tier's ultrapack.json.
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
pack_root="$repo_root/crates/ambition_platformer2d_actor_monolith/assets/sprite_packs"
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
        run_renderer_python "ultrapack-$tname" -m ambition_sprite2d_renderer ultrapack \
            --from-rendered "$sprites_dir" \
            --out "$pack_root/$tname" \
            --scale "$tscale" --min-frame-px "$tmin" --page-size "$tpage" \
            --name ultrapack "${ultrapack_plan[@]}" "${ultrapack_debug[@]}" 2>&1 | sed 's/^/  /' || \
            echo "  WARN: ultrapack tier '$tname' failed (non-fatal)"
    done
    echo "  packs installed under $pack_root/{full,half,quarter,potato}/"
    # Postcondition. Two questions, and the second one is why this exists:
    #
    #   1. Do the tiers agree with EACH OTHER? A transient IO flake once
    #      silently dropped 59 targets from one tier — scale must never change
    #      coverage, so unequal sets are a hard failure.
    #   2. Do they agree with WHAT WAS RENDERED? (1) alone cannot see
    #      staleness: four equally-old tiers agree perfectly. The packs sat ten
    #      days old at 167 targets while a fresh pack held 181, and this check
    #      passed happily on every one of those days. So each catalog is also
    #      compared against the published sheets it claims to pool — by name
    #      (a packed target whose sheet is gone is a stale pack) and by MTIME
    #      (a catalog older than the newest sheet is a stale pack, whatever the
    #      counts say).
    #
    # the sheets a pack does NOT cover are reported, not failed: ultrapack
    # skips manifests with no standard frame rows (bespoke targets, which it
    # names on stderr as it goes), and that set is content, not a defect.
    if ! "$python_bin" - "$pack_root" "$sprites_dir" <<'PYEOF'
import json, sys
from pathlib import Path
root = Path(sys.argv[1])
sheets = Path(sys.argv[2])

# What ultrapack pools: the top-level published sheets (see
# `ultrapack_rendered`, which globs `*_spritesheet.yaml` at the sheet root).
rendered = {p.name[: -len("_spritesheet.yaml")] for p in sheets.glob("*_spritesheet.yaml")}
newest_sheet = max(
    (p.stat().st_mtime for p in sheets.glob("*_spritesheet.*")), default=0.0
)

sets = {}
stamps = {}
for tier in ("full", "half", "quarter", "potato"):
    cat = root / tier / "ultrapack.json"
    if cat.exists():
        sets[tier] = set(json.loads(cat.read_text())["targets"])
        stamps[tier] = cat.stat().st_mtime
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
    orphaned = sorted(s - rendered)
    if orphaned:
        bad = True
        print(f"  ERROR: tier '{tier}' packs {len(orphaned)} target(s) with no "
              f"published sheet — the pack is stale: {orphaned[:5]}…", file=sys.stderr)
    if stamps[tier] < newest_sheet:
        bad = True
        age = (newest_sheet - stamps[tier]) / 3600.0
        print(f"  ERROR: tier '{tier}' catalog is {age:.1f}h older than the newest "
              f"published sheet — the pack is stale", file=sys.stderr)
if bad:
    sys.exit(1)
unpacked = sorted(rendered - ref)
note = f"; {len(unpacked)} sheet(s) not pooled: {', '.join(unpacked)}" if unpacked else ""
print(f"  ok: {len(sets)} tiers x {len(ref)} targets — identical, current with "
      f"{len(rendered)} published sheet(s){note}")
PYEOF
    then
        regen_failures+=("ultrapack postcondition: tier coverage disagrees or is stale")
    fi
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
    run_renderer_python ldtk-manifest -m ambition_sprite2d_renderer \
        ldtk-manifest --out "$sprite_manifest" 2>&1 | sed 's/^/  /' || true
    # hall_of_characters carries the same PlayerStart icon wiring as the other
    # three; leaving it out let its tileset def drift onto a sheet name the
    # renderer had stopped publishing. Pruning drops the def that repointing
    # orphans, so no world keeps naming a PNG that no longer ships.
    for world in sandbox intro you_have_to_cut_the_rope hall_of_characters; do
        ldtk_path="$worlds_dir/$world.ldtk"
        [ -f "$ldtk_path" ] || continue
        "$ldtk_python" \
            -m ambition_ldtk_tools.edit.visual_manifest apply-manifest \
            "$ldtk_path" "$sprite_manifest" --in-place --prune-unused-tilesets \
            2>&1 | sed 's/^/  /' || true
    done
else
    echo "  (skipped — sprite renderer or ambition_ldtk_tools not importable)"
fi

# --- LDtk editor art (the engine's own tiles, in the editor) -------------
# Rebuild the per-world atlas that `asset editor-art` composes out of the
# engine's entity sprites, so a fresh clone opens the world with masonry and
# blocks rather than grey cells. The WIRING (auto-layer rules, entity
# tileRects) is committed in the .ldtk; only the atlas PNG is gitignored, and
# re-running is a no-op on the .ldtk when nothing about the art moved.
echo "==> LDtk editor art:"
if ambition_python_exists "$ldtk_python" && \
    "$ldtk_python" \
        -c "import ambition_ldtk_tools" 2>/dev/null
then
    for world_ldtk in \
        "$repo_root/game/ambition_demo_mary_o/assets/worlds/mary_o.ldtk"
    do
        [ -f "$world_ldtk" ] || continue
        "$ldtk_python" \
            -m ambition_ldtk_tools asset editor-art "$world_ldtk" --in-place \
            2>&1 | sed 's/^/  /' || true
    done
else
    echo "  (skipped — ambition_ldtk_tools not importable from $ldtk_python)"
fi

# The author vanity card's part sheet. Its manifest
# (game/ambition_content/assets/data/vanity_card_made_this_meme.ron) is TRACKED
# and `include_str!`d into ambition_content, so the frame table is always
# present — but the sheet it names is a `.png`, and .gitignore ignores `*.png`
# repo-wide. Nothing else generated it, so every fresh clone got a complete
# flipbook of placements pointing at an image that could never arrive.
#
# The exporter is pure PIL over the committed rig JSON (no Blender), and it
# rewrites the manifest from the same bake, so the two cannot drift.
echo "==> author vanity card (part sheet + baked placements → $content_assets_dir/vanity_card_made_this_meme)"
if ! run_renderer_python "vanity-card" scripts/export_author_vanity_card.py 2>&1 | sed 's/^/  /'; then
    regen_failures+=("author vanity card: exporter reported a failure")
fi

if ! run_quality_variants; then
    regen_failures+=("quality variants: generator reported a failure")
fi

# --- Verdict --------------------------------------------------------------
# Every stage has now run. Fail here, once, for anything any of them recorded —
# and leave the fingerprint uncached so the next run re-renders rather than
# reporting a cache hit over a broken tree.
if [ "${#regen_failures[@]}" -gt 0 ]; then
    print_regen_timings
    echo "" >&2
    echo "==> regen FAILED — ${#regen_failures[@]} problem(s):" >&2
    for failure in "${regen_failures[@]}"; do
        echo "    $failure" >&2
    done
    echo "    (fingerprint not cached; the next run re-renders)" >&2
    exit 1
fi

# --- Write fingerprint on success ----------------------------------------
mkdir -p "$cache_dir"
echo "$current_fingerprint" > "$fingerprint_file"
echo "  cached regen fingerprint at $fingerprint_file"

print_regen_timings

if [ "$line_profile_enabled" -eq 1 ]; then
    echo "==> line profile reports: $line_profile_run_dir"
    echo "    Inspect one with: $python_bin -m line_profiler -rtmz <report>.lprof"
fi

echo "==> done"
echo "    file://$sprites_dir"
echo "    file://$repo_root/crates/ambition_platformer2d_actor_monolith/assets"
