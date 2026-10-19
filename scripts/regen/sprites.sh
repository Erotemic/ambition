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
#   ./scripts/regen/sprites.sh --check           # list missing or stale outputs; render nothing
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
#
# --check answers "is anything not generated yet?" in about a second: it derives
# the same runtime-required file list the cache and the postcondition use, and
# reports which files are absent, grouped by the target that produces them.
# It also says whether the published sheets are STALE: whether the renderer,
# its configs, this script or the toolchain changed since the last full
# publish (the saved fingerprint). It does not read the files' contents, and it
# does not look at the reduced-resolution tiers. Exit 0 when complete and
# current, 1 when not.
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
asset_root="$repo_root/crates/ambition_platformer2d_actor_monolith/assets"
# Where the phases write. A full publish points these at its staging root
# (`scripts/lib/sprite_staging.sh`) and back at `asset_root` after the swap.
publish_asset_root="$asset_root"
sprites_dir="$asset_root/sprites"
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

# For the preflight's "rerun with" line: the command exactly as typed.
rerun_command="$(printf '%q ' "$0" "$@")"
rerun_command="${rerun_command% }"

force_regen=0
list_targets=0
check_toolchain_only=0
check_only=0
list_outputs=0
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
        --check) check_only=1; shift ;;
        --list-outputs) list_outputs=1; shift ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

if [ "$list_targets" -eq 1 ] && [ "${#target_names[@]}" -gt 0 ]; then
    echo "--list and --target are mutually exclusive" >&2
    exit 2
fi

if { [ "$check_only" -eq 1 ] || [ "$list_outputs" -eq 1 ]; } && { [ "$list_targets" -eq 1 ] || [ "$force_regen" -eq 1 ] \
    || [ "$make_gifs" -eq 1 ] || [ "${#target_names[@]}" -gt 0 ]; }; then
    echo "--check and --list-outputs take no other options" >&2
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
# ⭐ EVERY DECLARED DEPENDENCY, not one import. Importing only the tool's own
# package let a venv without numpy through, and the run then died on the first
# pirate as "publish roster names an unregistered target". On failure this names
# the interpreter, why it was chosen, and whether the user's active env would do.
# `--check` only asks the renderer what it installs; it needs no LDtk tools.
if [ "$check_only" -eq 1 ] || [ "$list_outputs" -eq 1 ]; then
    ambition_preflight_tool_pythons "$rerun_command" \
        "$renderer_dir" AMBITION_SPRITE_PYTHON 1 \
        || exit 1
else
    ambition_preflight_tool_pythons "$rerun_command" \
        "$renderer_dir" AMBITION_SPRITE_PYTHON 1 \
        "$ldtk_tools_dir" AMBITION_LDTK_PYTHON 0 \
        || exit 1
fi

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

# The renderer is single-threaded Python. With one process, a full regen took
# more than 2.5 h on a 14-core machine at a load average of 1.45. `publish-many`
# renders each target in its own process
# when this is above 1; each one writes only its own files. Half the cores,
# because one long publish process grew to 3.7 GB, and at most 6: the machine is
# shared (at most 6 parallel jobs, Jon 2026-10-03).
if [ -z "${AMBITION_SPRITE_JOBS:-}" ]; then
    sprite_jobs=$(( $(nproc 2>/dev/null || echo 2) / 2 ))
    [ "$sprite_jobs" -ge 1 ] || sprite_jobs=1
    [ "$sprite_jobs" -le 6 ] || sprite_jobs=6
    export AMBITION_SPRITE_JOBS="$sprite_jobs"
fi
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

# The staging root, on the same file system as `asset_root`, so the swap is
# a rename. A focused publish and a full one both write into it.
staging_root="$renderer_dir/.cache/publish_staging"
mirror_root="$renderer_dir/.cache/publish_mirror"
# shellcheck source=../lib/sprite_staging.sh
source "$repo_root/scripts/lib/sprite_staging.sh"

# Point the phases at the staging root, or back at the runtime roots.
use_publish_root() {
    publish_asset_root="$1"
    sprites_dir="$1/sprites"
    entities_dir="$sprites_dir/entities"
}

# Ultrapack tiers. tier: <name> <scale> <min_frame_px> <page_size>
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

# render-target-name -> runtime props/ basename
canonical_prop_map=(
    "shrine:shrine"
    "pirate_heavy_axe:axe"
    "throwing_javelin:javelin"
    "evil_chain_gun:evil_chain_gun"
    "coconut:coconut"
    "seam_gauge:seam_gauge"
    "pool_noodle:pool_noodle"
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

# A few props are shown in-game as STATIC held / ground items, which load
# the single-pose `*_canonical_transparent.png` (not the animated sheet the
# tack-on publish installs). They are copied flat into props/ so the runtime
# asset paths (`sprites/props/<name>.png`) resolve on a fresh clone.
#
# The Mary-O construction pieces are level-construction sprites rather than
# held items. Their transparent fixed canvases are kept: seam coordinates and
# attachment anchors are part of the authoring contract, so they must never be
# auto-cropped here.
#
# With target names, only the props of those targets are copied: a focused run
# installs the runtime files of what it published, as a full run does. It did
# not, so `--target seam_gauge` published the sheet and left
# `props/seam_gauge.png` missing, which the game's art check reported as a
# code defect (2026-10-16).
copy_prop_canonicals() {
    local only=("$@")
    local pair src_target dst_name canon wanted one
    mkdir -p "$sprites_dir/props"
    for pair in "${canonical_prop_map[@]}" "${construction_prop_map[@]}"; do
        src_target="${pair%%:*}"
        dst_name="${pair##*:}"
        if [ "${#only[@]}" -gt 0 ]; then
            wanted=0
            for one in "${only[@]}"; do
                [ "$one" = "$src_target" ] && wanted=1
            done
            [ "$wanted" -eq 1 ] || continue
        fi
        canon="$renderer_dir/generated/$src_target/${src_target}_canonical_transparent.png"
        if [ -f "$canon" ]; then
            cp "$canon" "$sprites_dir/props/${dst_name}.png"
            echo "    $src_target -> props/${dst_name}.png"
        else
            echo "    warning: missing $canon (prop not rendered)" >&2
        fi
    done
}

if [ "${#target_names[@]}" -gt 0 ]; then
    validate_sprite_targets
    # Into the staging root, as a full publish writes: a focused run that
    # stops part way leaves the runtime roots as the last whole set, and
    # the next run continues the staged copy.
    begin_staging
    use_publish_root "$staging_root"
    for one in "${target_names[@]}"; do
        regen_one_target "$one"
    done
    copy_prop_canonicals "${target_names[@]}"
    # ⛔ A FOCUSED run used to exit here, publishing the full-resolution sheet
    # and leaving the Low / Medium / Potato tiers on the previous art — the
    # player robot's new rows shipped to 1x only, and a quality setting decided
    # which robot you saw. The tiers of what was just published are part of
    # publishing it. `--force` because the sheets named here were just rewritten.
    if [ "${AMBITION_QUALITY_VARIANTS:-1}" != "0" ]; then
        variant_targets=()
        for one in "${target_names[@]}"; do
            variant_targets+=(--target "${one}*")
        done
        echo "==> reduced-resolution quality variants: ${target_names[*]}"
        if ! "$python_bin" "$repo_root/scripts/generate_visual_quality_variants.py" \
            --asset-root "$publish_asset_root" \
            --sprites-only --force "${variant_targets[@]}" 2>&1 | sed 's/^/  /'; then
            echo "==> regen FAILED — quality variants: generator reported a failure" >&2
            exit 1
        fi
    fi
    # The packs hold the frames of the runtime groups (`intro_cart` draws
    # from them). A focused run did not pack, so a focused publish of a
    # runtime-group target left its pack on the old art until the next full
    # publish. A tier packs only the runtime groups, in about a second. The
    # tier keys are removed: the next full publish runs each tier again.
    if [ "${AMBITION_ULTRAPACK:-1}" != "0" ]; then
        echo "==> Ultrapack runtime groups"
        mkdir -p "$renderer_dir/.cache/ultrapack"
        tier_log="$(mktemp "${TMPDIR:-/tmp}/sprite-ultrapack.XXXXXX")"
        for tier in "${ultrapack_tiers[@]}"; do
            read -r tname tscale tmin tpage <<<"$tier"
            rm -f "$renderer_dir/.cache/ultrapack/$tname.key"
            if ! run_renderer_python "ultrapack-$tname" -m ambition_sprite2d_renderer.authoring.ultrapack \
                --from-rendered "$sprites_dir" \
                --out "$publish_asset_root/sprite_packs/$tname" --runtime-only \
                --scale "$tscale" --min-frame-px "$tmin" --page-size "$tpage" \
                --name ultrapack "${ultrapack_plan[@]}" \
                --state "$renderer_dir/.cache/ultrapack/$tname.state.json" \
                --jobs 1 >"$tier_log" 2>&1; then
                sed 's/^/  /' "$tier_log" >&2
                rm -f "$tier_log"
                echo "==> regen FAILED — Ultrapack tier '$tname'" >&2
                exit 1
            fi
            sed -n '1s/^/  /p' "$tier_log"
        done
        rm -f "$tier_log"
    fi
    echo "==> staging root → runtime roots"
    finish_staging
    use_publish_root "$asset_root"
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

# Characters authored as configs/review/*.yaml that the game draws: config
# targets of the registry, published by the cached batch. To promote a review
# config, add its name here AND give it a `character_catalog.ron` entry.
review_cues=(
    # Toon-target NPC variants already promoted.
    architect vault_keeper
    # absurd_general is NOT here any more -- see `tackon_targets`: the General
    # is an SVG rig of his own (2026-10-13).
    # kernel_guide is NOT here any more -- see `tackon_targets`, the same move
    # as oiler and bob.
    # oiler is NOT here any more — see `tackon_targets`. His body comes from
    # the direct-SVG rig; leaving the review cue in place would have this loop
    # overwrite the rig's sheet with the toon render on every full run.
    # erdish is NOT here any more -- he is a rigged module target, in the
    # roster below.
    # raid_enforcer is NOT here any more: its config is the main one
    # (configs/raid_enforcer.yaml), a runtime config target above.
    # fascist_enforcer is NOT here any more -- he is a person of the shared
    # rig (`targets/characters/fascist_enforcer.py`), in `tackon_targets`.
    merchant_prototype
    # Named characters whose YAML manifests already live in $sprites_dir.
    # bob is NOT here any more -- see `tackon_targets`, the same move as oiler.
    # The cryptography crew but for Alice are NOT here any more -- they are
    # rigged module targets (`crypto_crew.py`), in the roster below.
    alice general_hero
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
    # goblin_forest_spear is NOT here any more: its one config is the main
    # one (configs/goblin_forest_spear.yaml), a runtime config target.
    goblin_brute_hammer goblin_cave_dagger goblin_desert_bow
    goblin_frost_sword goblin_shaman_staff goblin_bruiser_club
)

# The faction leaders the game draws: rows of the faction lineup
# (`configs/factions/music_factions.yaml`), each a registered config target.
# The lineup's other characters are review art, and nothing publishes them.
faction_cues=(goblin_cantina_chieftain goblin_cantina_bard pulse_voyager_captain tech_bro_disruptor)

tackon_targets=(
    # The three sentinel sheets the catalog draws (`boss_art_keys.ron`): the
    # gradient sentinel for a boss with no art of its own, the clockwork warden
    # and the hooded sentinel (Mode Collapse) for the two that wear their own.
    gradient_sentinel_boss
    clockwork_warden_boss
    hooded_sentinel_boss
    # That file is deleted; this line is what keeps her published.
    noether
    oiler
    # ⛔⛔ THE GNU-TON'S APPLE, WHICH NO ROSTER LINE NAMED. `ambition_content`
    # registers the `apple` projectile as
    # `ProjectileArtSource::Image { path: "sprites/gnu_ton_boss/gnu_ton_apple.png" }`,
    # and that file was absent from this checkout: the boss's apple rain fell
    # back to `ProjectileArt::generic()`, an orange-red quad, for every player.
    # ⚠ NOTHING CRASHES AND NOTHING LOGS when that happens — `resolve` falls
    # back silently, so the shot simply flies with the wrong look. Caught
    # 2026-08-30 by `declared_art_resolves::every_declared_projectile_image_names_a_file_that_exists`
    # the first time `app_it` was swept end to end (D-QTT-2), which is the same
    # shape as `goblin_cave_dagger` one array up: a target that no batch names
    # exists only on the machine that once rendered it.
    gnu_ton_apple
    # Bob, redrawn 2026-10-05 as an SVG rig (`targets/characters/bob.py`, rig
    # under `rigged/bob/`). The rig roster below globs only TOP-LEVEL
    # `rigged/*.rig.json`, and he stayed in `review_cues`, so every full run
    # installed `generated/review/bob_*` -- a toon render left over from
    # before the redraw, which `draw-review` no longer produces -- and the game
    # drew the old Bob with his new art committed and nothing red.
    bob
    # The Kernel Guide, a character on the crew rig since the renderer's
    # `bda6df5` (`targets/characters/kernel_guide.py`); its review config is
    # deleted, so `draw-review` renders it no more and the publish stopped at
    # its cue.
    kernel_guide
    # The Flying Spaghetti Monster's thrown meatball, the `meatball` projectile
    # visual, named here for the same reason as the apple above.
    fsm_meatball
    # The rocks the Tyrant King shakes from his arena's ceiling, the
    # `trex_rock` projectile visual, named here for the same reason.
    trex_rock
    # The Hunny Horror's honey, the `honey_glob` projectile visual, named here
    # for the same reason.
    honey_glob
    # The Hunny Horror's red balloon, the `red_balloon` projectile visual,
    # named here for the same reason.
    red_balloon
    # The Mockingbird's wingtip missile, the `mockingbird_missile` projectile
    # visual, named here for the same reason.
    mockingbird_missile
    # The burning lightsabers it spits from its mouth (two rows: its red fire
    # and its cold one), the `mockingbird_lightsaber` projectile visuals.
    mockingbird_lightsaber
    # The bolt the Smirking Behemoth fires from its eye, the `eye_beam`
    # projectile visual.
    behemoth_eye_bolt
    # The plain shots: the robot's fireball and Hadouken, a spit, a thrown
    # rock and a magic bolt, one row each (`projectiles.rs` registers them).
    shot_kit
    # The Officer's service round, the `pistol_round` projectile visual. It was
    # not named here, so a publish of the roster did not make it and his shot
    # was drawn as the magenta box of a look whose sheet is missing.
    pistol_round
    # The bubble shield: the field about a body that guards, and the
    # honeycomb and the cracks its shader layer draws on it.
    bubble_shield
    # The two Fighting Polygons are named here because a `--target` render is
    # not a PUBLISH ROSTER. Both were rendered into this checkout one target at
    # a time (`scripts/regen/sprites.sh --target <name>`), which works and is the right
    # surgical tool — but generated art is gitignored, so a target that no batch
    # names exists only on the machine that once rendered it and is ABSENT from a
    # fresh clone. `character_catalog.ron` names both sheets, so a clone would
    # come up with two catalog rows pointing at files nothing can produce.
    # `test_authored_assets_have_regeneration_paths` is what said
    # so, and it is the second time this session that a check caught art existing
    # only by accident of local history.
    pointed_polygon
    pugnacious_polygon
    # The two easter-egg fighters, for the same reason as the polygons above and
    # by the same test: `character_catalog.ron` names their sheets, so a roster
    # that omits them is a fresh clone with two rows pointing at nothing.
    director
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
    # The Mockingbird boss's art since 2026-10-06 (`boss_art_keys.ron`); its
    # first design, `mockingbird_boss`, still publishes below as its lineage.
    mockingbird_boss_v2
    pipi_tau
    sanic
    super_sanic
    sanic_ring_prop
    sanic_goal_post
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
    coconut
    # The seam gauge: a plate of a measured grid the seam lab throws through
    # portals (scripts/portal_seam_lab.py).
    seam_gauge
    # The Iron Trooper's chain gun (`evil_heavy`).
    evil_chain_gun
    # Scenery of the primeval trail (an LDtk `Prop`).
    coconut_palm
    # The stand of a dimension gate (an LDtk `Prop`): the Math gate square.
    gate_cradle
    # Castle Godwin of the Evil dimension: its entrance and the thorns at
    # its foot, and its sign at the road (LDtk `Prop`s).
    castle_godwin_entrance
    castle_godwin_sign
    evil_thorn_bush
    # The set dressing of the castle's rooms (`castle_godwin_dressing`).
    castle_guard_post
    castle_wanted_poster
    castle_propaganda_poster_a
    castle_propaganda_poster_b
    castle_propaganda_poster_c
    castle_searchlight
    castle_bunk_bed
    castle_locker_row
    castle_mess_table
    castle_rally_podium
    castle_tesla_coil
    castle_specimen_vat
    castle_operating_table
    castle_iron_throne
    # The house fronts of the Math town (LDtk `Prop`s, decoration).
    math_house_rose
    math_house_dome
    math_house_tower
    math_house_shop
    math_house_arcade
    math_house_gable
    math_house_flat
    math_observatory
    # The terraces of the Math town: the aqueduct, the stair, the passage
    # under the first terrace, homes in the rock, trees (LDtk `Prop`s).
    math_aqueduct_span_street
    math_aqueduct_span_t1
    math_aqueduct_span_t2
    math_switchback_stair
    math_tunnel_bay
    math_tunnel_mouth
    math_tunnel_gate
    math_cliff_home_wide
    math_cliff_home_narrow
    math_cypress
    math_olive
    # The keys and the doors of the Noodly Temple (`PickupSpawn.sprite`,
    # `LockWall.look`).
    temple_key_marinara
    temple_gate_marinara
    temple_key_pesto
    temple_gate_pesto
    temple_key_parmesan
    temple_gate_parmesan
    temple_key_colander
    temple_gate_colander
    temple_gate_iron
    temple_gate_wheel_1
    temple_gate_wheel_2
    temple_gate_wheel_3
    temple_gate_wheel_1_shut
    temple_gate_bell
    # A held item of the Noodly Temple: a foam noodle to hit with.
    pool_noodle
    portal_gun_blue
    portal_gun_orange
    hunting_bow
    bow_arrow
    # The Projectile Polygon's charge shot. A projectile is its own sheet
    # because it OUTLIVES the pose that fired it — five tiers plus spawn and
    # two impacts, none of which a character row can carry.
    polygon_charge_shot
    # The PCA's charge blast: a ball of live cells, a real Life oscillator per
    # tier (blinker to pulsar).
    pca_charge_shot
    robot_slash
    # The stroke of GNU-ton's quill: the same kind of sheet, his own art
    # (`attack_vfx` of `npc_gnu_ton`).
    gnu_ton_slash
    # The strikes of the rest of the Smash cast: one sheet each, drawn by
    # `_strike_fx_common.py` (`attack_vfx` of each fighter).
    george_booul_strike_fx
    mary_o_strike_fx
    sanic_strike_fx
    pirate_admiral_strike_fx
    ninja_shadow_oni_leader_strike_fx
    alice_strike_fx
    bob_strike_fx
    oiler_strike_fx
    perfect_cellular_automaton_strike_fx
    goblin_strike_fx
    emmy_noether_strike_fx
    carl_stargan_strike_fx
    patent_clerk_strike_fx
    ramen_nujan_strike_fx
    pointed_polygon_strike_fx
    projectile_polygon_strike_fx
    pugnacious_polygon_strike_fx
    director_strike_fx
    performer_strike_fx
    officer_strike_fx
    medic_strike_fx
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
    # A pit of lava of Mary-O's castle: a top that repeats along the pit, and
    # the body under it.
    super_mary_o_lava_top
    super_mary_o_lava_body
    # The switch at the end of Mary-O's castle: a button, up and down.
    super_mary_o_switch
    # The gate past the switch: a portcullis, one row for each position.
    super_mary_o_gate
    # The moving platform of Mary-O: a `MovingPlatform` that names it in its
    # `visual` field is drawn as it.
    super_mary_o_lift
    # Mary-O's plant in a pipe. Its catalog row (game/ambition_demo_mary_o)
    # names sprites/snake_plant_spritesheet.*.
    snake_plant
    # The boss of Mary-O's castle.
    void_lizard
    # Mary-O's enemy that no stomp can hurt.
    void_star
    # Mary-O's fireball, the spark blossom: `provider.rs` names its sheet as
    # the look of the shot of the cinder beacon.
    super_mary_o_spark
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
    # GNU-ton the fighter: the fist of his giant, out of a portal's line.
    gnu_ton_vfx
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
    # The god's lesser appendage, the minion its `lesser_appendages` summons.
    fsm_noodling
    galwah
    ghoul_skulker
    girdle
    hand_saint
    helpful_liar
    mantis_lancer
    ninja_heavy
    ninja_shadow_duelist
    ninja_shadow_oni_leader
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
    chicken
    chicken_hen
    owl
    kiwi
    emu
    ostrich
    ostrich_hen
    cassowary
    shoebill
    european_swallow
    african_swallow
    vulture
    penguin
    compsognathus
    dilophosaurus
    pachycephalosaurus
    triceratops
    viking_heavy_shieldmaiden
    viking_heavy_warrior
    viking_shieldmaiden
    viking_warrior
    weird_hermit
    willson
    ramen_nujan
    erdish
    craig eve judy mallory olivia peggy sybil trent trudy victor walter
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
    # Her ultimate form, and the transformation into it (one sheet).
    mary_o_v2_cosmic
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
    # The roaming dog of the central hub basement (`npc_companion_dog`).
    companion_dog
    # The soldier of the Evil dimension (`npc_fascist_enforcer`), redrawn on
    # the person rig (2026-10-15); its review config is deleted.
    fascist_enforcer
    # The boss of Godwin Castle (`npc_wolfenstein_boss`), on the person rig.
    wolfenstein
    # The troops of Godwin Castle, on the person rig: the iron trooper (his
    # chain gun is the held prop `evil_chain_gun`) and the lab's failed
    # prototypes of Wolfenstein.
    evil_heavy
    evil_failed_prototype
    # The guard dog of Godwin Castle: the house dog parts, its own clips.
    evil_attack_hound
    # The first and easiest baddie of Castle Godwin: a dead conscript of
    # the regime, raised in the lab, on the person rig.
    evil_zombie
    # Two more of the castle's troops, SVG rigs: the riot trooper with the
    # electric shield and baton, and the searchlight on spider legs that
    # crawls the walls.
    shock_warden
    searchlight_crawler
    # The General (`npc_general`): an SVG rig, not a preset of the toon
    # template any more.
    absurd_general
    # The three house dogs, each in a test room off the basement. They are
    # drawn on the companion dog's rig.
    dog_honey
    dog_roadie
    dog_bezoar
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


# The runtime sheets authored as CharacterJob configs: the renderer's
# `RUNTIME_ADAPTER_CONFIG_STEMS`, as registered target names. Not every
# configs/*.yaml document is a runtime sheet. A name that the registry gives to
# a module target (`sandbag`) is that module's, published by its own line.
#
# They publish through the cached batch, as the other targets do. The
# `draw-all` and `draw-review` phases rendered every config on every publish
# that was not a whole-run cache hit: about 27 minutes, because one sheet held
# each phase (`alice` 891 s, `goblin` 684 s; measured 2026-10-14). And a name
# with a main config and a review config was two products: `draw-all` wrote
# the main sheet and the review install loop wrote over it, so the full
# publish and `--target` shipped different art.
if ! config_target_list="$(
    cd "$renderer_dir" && "$python_bin" - <<'CONFIG_TARGETS'
from pathlib import Path

from ambition_sprite2d_renderer.registry import (
    RUNTIME_ADAPTER_CONFIG_STEMS,
    discover_all_targets,
    load_jobs,
)

targets = discover_all_targets().targets
for path, job in load_jobs(Path("ambition_sprite2d_renderer/configs")):
    name = job.output_stem(path)
    if path.stem in RUNTIME_ADAPTER_CONFIG_STEMS and targets[name].kind == "config":
        print(name)
CONFIG_TARGETS
)"; then
    echo "the renderer could not list its runtime config targets; refusing to run" >&2
    exit 1
fi
mapfile -t config_targets <<< "$config_target_list"

# Every registered target this script publishes.
publish_targets=(
    entities
    "${config_targets[@]}"
    "${faction_cues[@]}"
    "${review_cues[@]}"
    "${tackon_targets[@]}"
    "${rig_targets[@]}"
    "${pirate_targets[@]}"
    puppy_slug
    mockingbird_boss
)

# The runtime-required file list. Consumed three times: by `--check`, by the
# cache fast-path below (a deleted asset re-triggers a render) and by the
# postcondition at the end. Each line is `<target>\t<file>`: the target is only
# for reporting which render owns a missing file.
#
# diagnostics are excluded because `sweep_runtime_diagnostics.py` MOVES them
# out of the runtime root at the end of every run — requiring them would make
# the fast path unsatisfiable and force a full re-render every time.
# `*_actor.ron` and the tileset/manifest `.ron` sidecars are excluded because
# the installer copies them opportunistically: 21 registered targets declare one
# and do not ship it.
# `*_parts.png` is excluded for the same reason: `install_companions` claims a
# transform flipbook (`_parts.ron` + `_parts.png`) for EVERY sheet, and only the
# pirates and Mary-O publish one. The `.ron` half was already dropped by the
# rule above; the `.png` half slipped through as a PNG, so from 2026-09-30 every
# other character "missed" one and a full run could never pass its
# postcondition or cache its fingerprint. Which targets publish a flipbook is
# not declared anywhere, so it cannot be required; `--check` reports the pairs.
declare_expected_files() {
    (
        cd "$renderer_dir"
        "$python_bin" - "$@" <<'DECLARE_EXPECTED'
import sys
from pathlib import Path

from ambition_sprite2d_renderer.registry import discover_all_targets

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
    if name.endswith("_parts.png"):
        return False
    if rel.endswith(".ron") and not (
        rel.endswith("_spritesheet.ron") or rel.endswith("_portraits.ron")
    ):
        return False
    return True


report = discover_all_targets()
names = list(sys.argv[1:])

unknown = sorted({n for n in names if n not in report.targets})
if unknown:
    for name in unknown:
        print(f"publish roster names an unregistered target: {name}", file=sys.stderr)
        # Discovery records a target module that fails to import as a
        # warning, not an error. Without the warning, a broken module reads
        # as a missing one.
        for warning in report.warnings:
            if name in warning:
                print(f"  discovery: {warning}", file=sys.stderr)
    print("Run ./scripts/regen/sprites.sh --list to see registered targets.", file=sys.stderr)
    raise SystemExit(2)

emitted = []
seen = set()
for name in names:
    for rel in report.targets[name].claimed_install_names():
        if runtime_required(rel) and rel not in seen:
            seen.add(rel)
            emitted.append(f"{name}\t{rel}")
print("\n".join(emitted))
DECLARE_EXPECTED
    )
}

expected_list="$(declare_expected_files "${publish_targets[@]}")"
mapfile -t expected_rows <<< "$expected_list"
# `expected_owners[i]` produces `expected_files[i]`.
expected_files=()
expected_owners=()
for row in "${expected_rows[@]}"; do
    expected_owners+=("${row%%$'\t'*}")
    expected_files+=("${row#*$'\t'}")
done
for pair in "${canonical_prop_map[@]}" "${construction_prop_map[@]}"; do
    expected_files+=("props/${pair##*:}.png")
    expected_owners+=("${pair%%:*}")
done

# The roster has published 800+ files for a year; anything under half that means the helper broke,
# not that the roster shrank.
if [ "${#expected_files[@]}" -lt 400 ]; then
    echo "expected-file derivation produced only ${#expected_files[@]} entries" >&2
    echo "— the roster or the renderer registry is broken; refusing to run" >&2
    exit 1
fi

if [ "$list_outputs" -eq 1 ]; then
    printf '%s\n' "${expected_files[@]}"
    exit 0
fi

if [ "$check_only" -eq 1 ]; then
    missing_owners=()
    declare -A missing_by_owner=()
    missing_count=0
    for i in "${!expected_files[@]}"; do
        [ -f "$sprites_dir/${expected_files[$i]}" ] && continue
        owner="${expected_owners[$i]}"
        [ -n "${missing_by_owner[$owner]+set}" ] || missing_owners+=("$owner")
        missing_by_owner[$owner]+="${missing_by_owner[$owner]:+ }${expected_files[$i]}"
        missing_count=$((missing_count + 1))
    done
    # Transform flipbooks are optional per target (see `declare_expected_files`),
    # so they are reported rather than required — except a draw table without
    # its atlas page, which the game would wait on forever and draw baked.
    flipbooks=()
    for table in "$sprites_dir"/*_parts.ron; do
        [ -f "$table" ] || continue
        stem="$(basename "$table" _parts.ron)"
        flipbooks+=("$stem")
        if [ ! -f "$sprites_dir/${stem}_parts.png" ]; then
            owner="$stem (part flipbook)"
            missing_owners+=("$owner")
            missing_by_owner[$owner]="${stem}_parts.png"
            missing_count=$((missing_count + 1))
        fi
    done
    echo "part flipbooks: ${#flipbooks[@]} published${flipbooks[*]:+ (${flipbooks[*]})}"
    if [ "$missing_count" -eq 0 ]; then
        echo "ok: all ${#expected_files[@]} runtime-required sprite files present in ${sprites_dir#"$repo_root"/}"
    else
    echo "missing ${missing_count} of ${#expected_files[@]} runtime-required sprite files" \
        "(${#missing_owners[@]} target(s)) in ${sprites_dir#"$repo_root"/}:"
    for owner in "${missing_owners[@]}"; do
        echo "  $owner"
        for rel in ${missing_by_owner[$owner]}; do
            echo "      $rel"
        done
    done
    echo ""
    echo "Generate them with: ./scripts/regen/sprites.sh  (current sheets are reused from the per-sheet cache)"
    fi
    # The staleness half needs the fingerprint functions below; it runs
    # there and exits.
fi

# --- Fingerprint cache ----------------------------------------------------
# The key of the whole publish (`compute_fingerprint`, in
# `scripts/lib/sprite_cache_key.sh`): every file of the renderer, the files
# each recorded render read, this script and the toolchain. If it matches the
# cached value AND every expected sheet is already present in $sprites_dir,
# skip the whole regen.
#
# The expected-files list is the same one the postcondition validates at
# the end. Keeping a single source of truth means deleting one published
# sheet, manifest, or portrait trips both the fast-path and postcondition.

cache_dir="$renderer_dir/.cache"
fingerprint_file="$cache_dir/regen-fingerprint"
# What each target's last render read: `publish-many` records it under this
# variable, and both keys hash it (`leaf_hash`, `compute_fingerprint`).
target_reads_dir="$cache_dir/reads"



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
# allow an interrupted run to resume without rebuilding current targets. The
# key (`compute_unit_core`, `leaf_hash`, `unit_key`) is in
# `scripts/lib/sprite_cache_key.sh`, so its tests call the functions this
# script calls.
sheets_cache_dir="$cache_dir/sheets"
# shellcheck source=../lib/sprite_cache_key.sh
source "$repo_root/scripts/lib/sprite_cache_key.sh"

# The digests of everything this unit published, in a stable order. Stored
# beside the key so freshness can ask whether the files on disk are THE ONES
# THIS CACHE PRODUCED, rather than whether some file is present.
#
# ⛔⛔ THE PATHS ARE THE REGISTRY'S TO STATE, NOT THIS SCRIPT'S TO GUESS. This
# took a `<target>*_spritesheet.png` glob, which describes neither
# `town_tileset` (installs `town_tileset.png`), nor `gnu_ton_apple` (installs
# into the `gnu_ton_boss/` subdir), nor `hud_icons` (installs
# `hud_stock_icon.png`, a name no glob could reach). Reasoning and the one
# implementation: `scripts/lib/sprite_install_names.py`.
#
# Takes the newline-separated install-relative paths the registry declares, and
# returns non-zero only when NONE of them is on disk.
#
# One `sha256sum` for all of a target's files: one for each file (and an
# `awk` for each) was 18 s of every publish that missed the whole-run
# fingerprint, for about 2,100 files (measured 2026-10-15; now 5 s). The
# lines are the same, `<digest> <path>`, so the stored digests stay valid.
sheet_output_digests() {
    local records="$1" rel
    local -a present=()
    [ -n "$records" ] || return 1
    while IFS= read -r rel; do
        [ -n "$rel" ] || continue
        [ -f "$sprites_dir/$rel" ] || continue
        present+=("$rel")
    done <<< "$records"
    [ "${#present[@]}" -gt 0 ] || return 1
    (cd "$sprites_dir" && sha256sum -- "${present[@]}") | sed 's/^\([0-9a-f]*\)  /\1 /' | sort
}

# ⛔⛔ A TARGET WHOSE OUTPUTS CANNOT BE DIGESTED IS NOT A REASON TO KILL THE RUN.
# The group's status used to be the `[ -n "$glob" ] && sheet_output_digests ...`
# conjunction, so an unmatched glob returned 1 out of this function and `set -e`
# aborted `sprites.sh` where it stood — silently, with no message, after a
# 19½-minute publish of all 137 targets, leaving the seven targets of the later
# batches unrendered. Store the key with no digest block instead:
# `sprite_freshness.py` already treats that as stale, so the next run re-renders
# rather than trusting an entry it could not verify.
sheet_cache_store() {
    local unit="$1" key="$2" records="${3-}" digests=""
    mkdir -p "$sheets_cache_dir"
    if [ -n "$records" ]; then
        digests="$(sheet_output_digests "$records" || true)"
    fi
    if [ -z "$digests" ]; then
        echo "  [warn] $unit published nothing this cache can verify; not caching its outputs" >&2
    fi
    {
        printf '%s\n' "$key"
        [ -n "$digests" ] && printf '%s\n' "$digests"
        :
    } > "$sheets_cache_dir/$unit"
}

# Publish registered targets in explicit batches. Registry discovery and YAML
# config loading are expensive, so cache inspection resolves every target's
# declared portrait products in one Python process and stale targets are then
# rendered by one `publish-many` process. This preserves per-target cache keys
# while avoiding one interpreter + discovery pass per character.
publish_cached_batch() {
    local label="$1"
    shift
    local -a candidates=("$@")
    local -a stale=()
    local target rel records="" records_ok=1
    local -A install_records=()

    [ "${#candidates[@]}" -gt 0 ] || return 0

    # What each target DECLARES it installs, which is what the output-digest
    # cache keys on. Resolved unconditionally — a forced run still stores, and
    # storing needs the same answer freshness does.
    local install_lines=""
    if ! install_lines="$(
        cd "$renderer_dir" && "$python_bin" \
            "$repo_root/scripts/lib/sprite_install_names.py" "${candidates[@]}"
    )"; then
        echo "  warning: could not resolve installed products for batch '$label'; its output digests are not cached" >&2
        install_lines=""
    fi
    while IFS=$'\t' read -r target rel; do
        [ -n "$target" ] || continue
        install_records["$target"]="${install_records[$target]-}$rel"$'\n'
    done <<< "$install_lines"

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
    fi

    # A target with no portrait row declares no portraits (a prop, an effect):
    # when the resolution fails, `records_ok` is 0. Requiring a row made all
    # 74 such targets stale on every run (measured 2026-10-13).
    #
    # One process answers every target: the same key as `unit_key`, the same
    # digests as `sheet_output_digests`, each file hashed once. Asked one
    # target at a time it was 15 s of keys and 5 s of digests for 245 targets
    # (measured 2026-10-15); `sprite_freshness.py` says why the key is the
    # same bytes.
    local installs_tsv portraits_tsv verdict
    installs_tsv="$(mktemp "${TMPDIR:-/tmp}/sprite-installs.XXXXXX")"
    portraits_tsv="$(mktemp "${TMPDIR:-/tmp}/sprite-portraits.XXXXXX")"
    printf '%s\n' "$install_lines" >"$installs_tsv"
    printf '%s\n' "${records-}" >"$portraits_tsv"
    local -a freshness_flags=()
    [ "$force_regen" -ne 1 ] || freshness_flags+=(--force)
    [ "$records_ok" -eq 1 ] || freshness_flags+=(--portraits-failed)
    local freshness=""
    if ! freshness="$(
        cd "$renderer_dir" && "$python_bin" "$repo_root/scripts/lib/sprite_freshness.py" \
            --repo-root "$repo_root" --cache-dir "$cache_dir" --sprites-dir "$sprites_dir" \
            --core "$unit_core_fingerprint" --installs "$installs_tsv" --portraits "$portraits_tsv" \
            "${freshness_flags[@]}" "${candidates[@]}"
    )"; then
        echo "  warning: could not judge the freshness of batch '$label'; republishing it" >&2
        freshness=""
        stale=("${candidates[@]}")
    fi
    rm -f "$installs_tsv" "$portraits_tsv"
    while IFS=$'\t' read -r verdict target; do
        [ -n "$target" ] || continue
        if [ "$verdict" = fresh ]; then
            echo "  [cache] $target up to date — skipped"
        else
            stale+=("$target")
        fi
    done <<< "$freshness"

    [ "${#stale[@]}" -gt 0 ] || return 0

    # Named, because `publish-many --quiet` names none of them, and a count
    # alone cannot say why a sheet was stale.
    echo "  publishing ${#stale[@]} target(s) on ${AMBITION_SPRITE_JOBS} process(es): ${stale[*]}"
    local published_list
    published_list="$(mktemp "${TMPDIR:-/tmp}/sprite-published.XXXXXX")"
    if run_renderer_python "publish-batch-$label" \
        -m ambition_sprite2d_renderer publish-many \
        --quiet --dest-root "$sprites_dir" --published-list "$published_list" \
        "${stale[@]}"; then
        for target in "${stale[@]}"; do
            sheet_cache_store "$target" "$(unit_key "$target")" \
                "${install_records[$target]-}"
        done
    else
        # A target that published is current. Keep its key, so that the next
        # run renders only the targets that failed.
        local kept=0
        while IFS= read -r target; do
            [ -n "$target" ] || continue
            sheet_cache_store "$target" "$(unit_key "$target")" \
                "${install_records[$target]-}"
            kept=$((kept + 1))
        done < "$published_list"
        echo "  [warn] batch '$label' had one or more publish failures; cache keys advanced for the $kept of ${#stale[@]} target(s) that published" >&2
        # A failed target is a failed run: no whole-run fingerprint, and a
        # non-zero exit. The old files of a failed target can still satisfy
        # the postcondition, so it cannot see this.
        regen_failures+=("batch '$label': $(( ${#stale[@]} - kept )) of ${#stale[@]} target(s) failed to publish")
    fi
    rm -f "$published_list"
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
# The wall time of each top-level phase. The render cost above counts only the
# renderer subprocesses; the phases between them (the entity sprites, the
# atlases, the LDtk art, the variants) are not in it, and a publish must say
# where all of its time went before a cache can be aimed at the largest cost.
phase_names=()
phase_seconds=()
phase_started=""

# Close the running phase, if there is one.
close_phase() {
    [ -n "$phase_started" ] || return 0
    phase_seconds+=("$(( $(date +%s) - phase_started ))")
    phase_started=""
}

# Start the phase `$1`, and print its header.
phase() {
    close_phase
    phase_names+=("$1")
    phase_started="$(date +%s)"
    echo "==> $1"
}

# Every phase's wall time, slowest first.
print_phase_seconds() {
    close_phase
    [ "${#phase_seconds[@]}" -eq 0 ] && return 0
    local index total=0
    for index in "${!phase_seconds[@]}"; do
        total=$((total + phase_seconds[index]))
    done
    echo ""
    echo "==> phase wall time: ${#phase_seconds[@]} phase(s), ${total}s total"
    for index in "${!phase_seconds[@]}"; do
        printf '%s\t%s\n' "${phase_seconds[index]}" "${phase_names[index]%%:*}"
    done | sort -rn | while IFS=$'\t' read -r seconds name; do
        printf '    %5ss  %s\n' "$seconds" "${name:0:72}"
    done
}

# It is cheap to chain because the generator is incremental: a run where nothing
# changed costs ~4s, and one changed character ~7s (a full rebuild is ~2m15s).
#   AMBITION_QUALITY_VARIANTS=0  skip it (same idiom as AMBITION_ULTRAPACK=0)
run_quality_variants() {
    phase "reduced-resolution quality variants (sprites):"
    if [ "${AMBITION_QUALITY_VARIANTS:-1}" = "0" ]; then
        echo "  (skipped — AMBITION_QUALITY_VARIANTS=0)"
        return 0
    fi
    # On the job cap's processes: after a full re-render the pass was 554 s
    # on one, and 98 s on six (measured 2026-10-15), the same files.
    "$python_bin" "$repo_root/scripts/generate_visual_quality_variants.py" \
        --asset-root "$publish_asset_root" \
        --sprites-only --jobs "$AMBITION_SPRITE_JOBS" 2>&1 | sed 's/^/  /'
}

# `--check`, second half: are the published sheets older than the renderer?
# The fingerprint is saved only after a full publish succeeds, so a different
# one means that the renderer, its configs, this script or the toolchain moved
# since then. A file can be present and still be drawn by an old renderer: the
# game then asks a sheet for a row it does not have.
if [ "$check_only" -eq 1 ]; then
    check_fingerprint="$(compute_fingerprint)"
    check_saved=""
    [ -f "$fingerprint_file" ] && check_saved="$(cat "$fingerprint_file")"
    check_status="$(( missing_count > 0 ? 1 : 0 ))"
    if [ "$check_fingerprint" = "$check_saved" ]; then
        echo "ok: the published sheets are from this renderer (fingerprint ${check_saved})"
    else
        echo "stale: the renderer, its art or configs, this script or the toolchain changed since the last full publish" \
            "(saved ${check_saved:-none}, now ${check_fingerprint})."
        echo "The files above can be present and still be what an older renderer drew."
        echo "Publish again with: ./scripts/regen/sprites.sh  (unchanged sheets are reused from the per-sheet cache)"
        check_status=1
    fi
    exit "$check_status"
fi

# Which files each target reads, asked of the registry once for the run.
mkdir -p "$cache_dir"
target_closures_file="$cache_dir/target_closures.tsv"
export AMBITION_SPRITE_READS_DIR="$target_reads_dir"
if ! sprite_cache_key_prepare; then
    echo "==> regen FAILED — the renderer could not say which files each target reads" >&2
    echo "    (scripts/lib/sprite_target_closure.py); a key without them is not that target's key" >&2
    exit 1
fi
core_shared_fingerprint="$(compute_core_shared)"
unit_core_fingerprint="$(compute_unit_core)"

phase "staging root: copy of the runtime roots → $staging_root"
begin_staging
use_publish_root "$staging_root"

cached_fingerprint=""
if [ -f "$fingerprint_file" ]; then
    cached_fingerprint="$(cat "$fingerprint_file")"
fi
current_fingerprint="$(compute_fingerprint)"
started_authored_fingerprint="$(compute_authored_fingerprint)"

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
        echo "    (the runtime roots were not replaced; the next run continues $staging_root)" >&2
        exit 1
    fi
    "$python_bin" "$repo_root/scripts/check_regeneration_coverage.py" \
        --family sprites --asset-root "$publish_asset_root"
    finish_staging
    exit 0
fi

# Cheap structural preflight before the first expensive render. The adapter config surface is small
# enough to validate up front.
phase "validate sprite character configs"
run_renderer_python validate-configs -m ambition_sprite2d_renderer validate-configs

phase "entity sprites → $entities_dir"
# Through the per-sheet cache, as every other target: the registry declares
# the 47 textures it installs under `entities/`, so their digests key it. It
# rendered all of them on every publish before (measured 2026-10-15).
publish_cached_batch entities entities

phase "tack-on targets (render-publish into $sprites_dir)"
# `tackon_targets` in the publish roster is every registered module target
# whose manifest the runtime loads. The registry is `registry/discovery.py`
# (auto-discovered from targets/<category>/ — run `list` to see it); keep that
# roster covering every target the game references (mockingbird_boss and the
# pirates ride the late-targets batch below).
publish_cached_batch tackons \
    "${config_targets[@]}" "${faction_cues[@]}" "${review_cues[@]}" "${tackon_targets[@]}" "${rig_targets[@]}"

phase "prop canonicals (single-pose → $sprites_dir/props)"
props_dir="$sprites_dir/props"
copy_prop_canonicals

phase "wielded-gauntlet prop icons (procedural → $props_dir)"
# The abstract wielded gauntlets (shockwave/volley/beam/vortex/sentry/dive/
# meteor) have no character rig, so their ground/held icons are procedural
# 64x64 PNGs from `item_icons.py::write_gauntlet_props`, consumed at runtime by
# `item_pickup::item_sprite`. (No canonical-pose copy step — drawn directly.)
# The same call also writes the `polygon_*.png` held-item props from that
# module's HELD_ITEM_ICON_SPECS, which `HeldItemArt` resolves instead.
(cd "$renderer_dir" && "$python_bin" -c "from ambition_sprite2d_renderer.targets.icons.item_icons import write_gauntlet_props as w; w('$props_dir')")

phase "Mark/Recall world beacon prop (procedural crystal → $props_dir)"
# The recall beacon is a free-standing 48x112 crystal pillar from
# `item_icons.py::write_mark_beacon_prop`, consumed at runtime by
# `mark_recall::sync_mark_beacon_visual` (stands at the dropped recall mark).
(cd "$renderer_dir" && "$python_bin" -c "from ambition_sprite2d_renderer.targets.icons.item_icons import write_mark_beacon_prop as w; w('$props_dir')")

phase "pirate, small-enemy, and multipart-boss target batch → $sprites_dir"
# Pirates are registered as tack-on `[characters]` targets and publish through
# the same machinery as the other tack-ons. These late targets share one
# publishing contract, so they render together and the registry + YAML configs
# are loaded once rather than once per target.
publish_cached_batch late-targets \
    "${pirate_targets[@]}" \
    puppy_slug \
    mockingbird_boss

phase "postcondition: every runtime-required sprite file present"
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
phase "Hall-of-Characters portrait coverage:"
if ! "$ldtk_python" -m ambition_ldtk_tools.inspect_hall_portraits \
    --catalog "$character_catalog" --sprites-dir "$sprites_dir" \
    --only-issues 2>&1 | sed 's/^/  /'; then
    echo "  note: portrait coverage is incomplete (reported above); not fatal."
fi

phase "portrait review gallery:"
run_renderer_python portrait-gallery -m ambition_sprite2d_renderer portrait-gallery \
    --source-dir "$sprites_dir" \
    --out "$renderer_dir/generated/portrait_gallery.png" 2>&1 | sed 's/^/  /'

# --- Publish boundary: sweep diagnostics out of the runtime roots ---------
# The sprite generators emit human-only diagnostics (canonical poses, labeled
# previews, debug overlays) next to the runtime sheets. Relocate them out of
# the runtime asset roots into target/ambition_publish/diagnostics so the game
# bundle ships runtime artifacts only. This is what keeps the Rust
# `shipped_runtime_roots_have_no_leaked_diagnostics` test green after a regen.
# See docs/archive/reviews/sprite-pipeline-2026-07/data-driven-sprites-and-characters.md
# (the page was ARCHIVED, not deleted -- this comment pointed at its live path
# for months after the move, and nothing checks doc paths cited outside docs/).
phase "Publish boundary: sweeping diagnostics out of runtime roots:"
if command -v "$python_bin" >/dev/null 2>&1; then
    "$python_bin" "$repo_root/scripts/sweep_runtime_diagnostics.py" \
        --repo-root "$repo_root" --asset-root "$publish_asset_root" 2>&1 | sed 's/^/  /' || true
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
phase "Ultrapack: shared-page atlases per quality tier → runtime pack root:"
pack_root="$publish_asset_root/sprite_packs"
# Each tier packs only the groups a runtime reader loads (the plan's
# `runtime_groups`, `--runtime-only`). The packager ships every file of the
# runtime roots, and it shipped 695 MB of pages for 5.4 MB that a reader can
# reach (measured 2026-10-15). A group packs onto pages of its own, so these
# are the pages the whole pack gave the group.
pack_debug_root="$repo_root/target/ambition_publish/diagnostics/packs"
if [ "${AMBITION_ULTRAPACK:-1}" = "0" ]; then
    echo "  (skipped — AMBITION_ULTRAPACK=0)"
elif command -v "$python_bin" >/dev/null 2>&1 && \
    "$python_bin" -c 'import ambition_sprite2d_renderer' >/dev/null 2>&1
then
    # The tiers read the same published sheets and write separate roots, so
    # they run at once, at most `AMBITION_SPRITE_JOBS` of them, the slowest
    # first. Measured 2026-10-15: full 126 s, half 116 s, quarter 83 s,
    # potato 29 s; one after another they were 362 s, 61% of a publish that
    # rendered one target. Each tier's output is kept in its own log and
    # printed whole, in tier order, when the tier ends.
    # A tier is skipped when its key holds: no pooled sheet, the plan, the
    # packer or the tier's own numbers changed since it was packed. It was
    # 131 s of a publish with nothing to render (measured 2026-10-15). The
    # key is removed before a tier packs and written only when it ends
    # well, so a tier cut short is never current.
    # A tier whose key moved is patched when it can be (`--state`): when
    # only some sheets' pixels changed and their frames kept their trimmed
    # sizes, the layout is the same, so only the pages that hold those
    # frames are drawn again, to the bytes of a full pack. Measured
    # 2026-10-15 on the full tier: a recolour of one sheet 6.7 s, a full
    # pack 124 s.
    # Each tier reads its sheets and encodes its pages on threads (`--jobs`;
    # the bytes do not change). The threads of the four tiers are at most
    # `AMBITION_SPRITE_JOBS` in all, and the two slowest tiers get two each.
    # Measured 2026-10-15, the four at once: 82 s on 2/2/1/1 threads, was
    # 130 s on one each; the quarter tier is then the slowest.
    ultrapack_key_dir="$cache_dir/ultrapack"
    mkdir -p "$ultrapack_key_dir"
    ultrapack_inputs="$(ultrapack_inputs_key "$sprites_dir" "$renderer_dir/ambition_sprite2d_renderer/data/pack_plan.yaml")"
    ultrapack_logs="$(mktemp -d)"
    ultrapack_pids=()
    ultrapack_names=()
    ultrapack_keys=()
    finish_ultrapack_tier() {
        local index="$1"
        local tname="${ultrapack_names[index]}"
        if [ -z "${ultrapack_pids[index]}" ]; then
            :
        elif wait "${ultrapack_pids[index]}"; then
            printf '%s\n' "${ultrapack_keys[index]}" >"$ultrapack_key_dir/$tname.key"
        else
            echo "  WARN: ultrapack tier '$tname' failed (non-fatal)" >>"$ultrapack_logs/$tname.log"
        fi
        sed 's/^/  /' "$ultrapack_logs/$tname.log"
    }
    ultrapack_waited=0
    for tier in "${ultrapack_tiers[@]}"; do
        read -r tname tscale tmin tpage <<<"$tier"
        ultrapack_debug=()
        if [ "${AMBITION_ULTRAPACK_DEBUG:-0}" = "1" ]; then
            ultrapack_debug=(--debug-views --debug-dir "$pack_debug_root/$tname")
        fi
        tier_key="$ultrapack_inputs $tier ${ultrapack_debug[*]}"
        if [ -f "$pack_root/$tname/ultrapack.json" ] \
            && [ "$(cat "$ultrapack_key_dir/$tname.key" 2>/dev/null)" = "$tier_key" ]; then
            echo "$tname: current (no pooled sheet, the plan or the packer changed since it was packed)" >"$ultrapack_logs/$tname.log"
            ultrapack_pids+=("")
            ultrapack_names+=("$tname")
            ultrapack_keys+=("$tier_key")
            continue
        fi
        rm -f "$ultrapack_key_dir/$tname.key"
        if [ "$(( ${#ultrapack_pids[@]} - ultrapack_waited ))" -ge "$AMBITION_SPRITE_JOBS" ]; then
            finish_ultrapack_tier "$ultrapack_waited"
            ultrapack_waited=$((ultrapack_waited + 1))
        fi
        case "$tname" in
            full | half) ultrapack_threads=$(( 2 * AMBITION_SPRITE_JOBS / 6 )) ;;
            *) ultrapack_threads=$(( AMBITION_SPRITE_JOBS / 6 )) ;;
        esac
        # The module's own entry, not the `ultrapack` command: an edit to
        # the CLI makes every sheet stale (see `ultrapack.main`).
        run_renderer_python "ultrapack-$tname" -m ambition_sprite2d_renderer.authoring.ultrapack \
            --from-rendered "$sprites_dir" \
            --out "$pack_root/$tname" --runtime-only \
            --scale "$tscale" --min-frame-px "$tmin" --page-size "$tpage" \
            --name ultrapack "${ultrapack_plan[@]}" "${ultrapack_debug[@]}" \
            --state "$ultrapack_key_dir/$tname.state.json" \
            --jobs "$(( ultrapack_threads > 1 ? ultrapack_threads : 1 ))" >"$ultrapack_logs/$tname.log" 2>&1 &
        ultrapack_pids+=("$!")
        ultrapack_names+=("$tname")
        ultrapack_keys+=("$tier_key")
    done
    while [ "$ultrapack_waited" -lt "${#ultrapack_pids[@]}" ]; do
        finish_ultrapack_tier "$ultrapack_waited"
        ultrapack_waited=$((ultrapack_waited + 1))
    done
    rm -rf "$ultrapack_logs"
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
    # A runtime-group target with a published sheet that a tier does not
    # pack is a failure: its prop row reads nothing.
    if ! "$python_bin" - "$pack_root" "$sprites_dir" \
        "$renderer_dir/ambition_sprite2d_renderer/data/pack_plan.yaml" <<'PYEOF'
import json, sys
from pathlib import Path
import yaml
root = Path(sys.argv[1])
sheets = Path(sys.argv[2])
plan = yaml.safe_load(Path(sys.argv[3]).read_text()) or {}
wanted = {
    stem for group in plan.get("runtime_groups") or [] for stem in (plan.get("groups") or {})[group]
}

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
        doc = json.loads(cat.read_text())
        sets[tier] = set(doc["targets"])
        stamps[tier] = cat.stat().st_mtime
        absent = [page for page in doc["pages"] if not (root / tier / page).is_file()]
        if absent:
            sets[tier] = None
            print(f"  ERROR: tier '{tier}' names {len(absent)} absent page(s): {absent[:5]}",
                  file=sys.stderr)
if not sets:
    sys.exit("  ERROR: no tier catalogs found under %s" % root)
bad = any(s is None for s in sets.values())
sets = {tier: s for tier, s in sets.items() if s is not None}
ref_tier = "full" if "full" in sets else sorted(sets)[0]
ref = sets[ref_tier]
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
    unpacked = sorted((wanted & rendered) - s)
    if unpacked:
        bad = True
        print(f"  ERROR: tier '{tier}' does not pack {len(unpacked)} runtime-group "
              f"target(s) with a published sheet: {unpacked[:5]}", file=sys.stderr)
    stray = sorted(s - wanted)
    if stray:
        bad = True
        print(f"  ERROR: tier '{tier}' packs {len(stray)} target(s) of no runtime group: "
              f"{stray[:5]}", file=sys.stderr)
    if stamps[tier] < newest_sheet:
        bad = True
        age = (newest_sheet - stamps[tier]) / 3600.0
        print(f"  ERROR: tier '{tier}' catalog is {age:.1f}h older than the newest "
              f"published sheet — the pack is stale", file=sys.stderr)
if bad:
    sys.exit(1)
print(f"  ok: {len(sets)} tiers x {len(ref)} runtime-group targets — identical, current "
      f"with {len(rendered)} published sheet(s)")
PYEOF
    then
        regen_failures+=("ultrapack postcondition: tier coverage disagrees or is stale")
    fi
else
    echo "  (skipped — sprite renderer not importable from $python_bin)"
fi

if ! run_quality_variants; then
    regen_failures+=("quality variants: generator reported a failure")
fi

# --- Swap the staged roots into place -------------------------------------
# Only a set that every sprite phase passed replaces the runtime roots. The
# phases below read the runtime roots: the LDtk phases write paths of the
# sheets into tracked files, and those paths must be the runtime ones.
phase "staging root → runtime roots"
if ! "$python_bin" "$repo_root/scripts/check_regeneration_coverage.py" \
    --family sprites --asset-root "$publish_asset_root"; then
    regen_failures+=("sprite regeneration coverage: required sheets are absent from the plan or output")
fi
if [ "${#regen_failures[@]}" -eq 0 ]; then
    finish_staging
    echo "  moved ${staged_root_names[*]} into $asset_root"
else
    echo "  NOT replaced: a phase failed (see the verdict); the next run continues $staging_root" >&2
fi
use_publish_root "$asset_root"

# --- Hall-of-Characters sprite census ------------------------------------
# Quick check of which catalog entries the Hall will render vs fall
# back to the colored-rectangle placeholder. Helpful as a final
# "did the regen actually fix the Hall?" signal.
phase "Hall-of-Characters sprite census:"
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
phase "LDtk editor-icon atlas:"
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
# The `file:` of each world row in the pack's `worlds.ron`.
ldtk_world_files() {
    sed -n 's/^[[:space:]]*(id:.*file:[[:space:]]*"\([^"]*\)".*/\1/p' \
        "$content_assets_dir/data/worlds.ron"
}

phase "LDtk sprite tilesets:"
sprite_manifest="$sprites_dir/ldtk_sprite_manifest.json"
if ambition_python_exists "$python_bin" && \
    "$python_bin" -c 'import ambition_sprite2d_renderer' >/dev/null 2>&1 && \
    "$ldtk_python" \
        -c "import ambition_ldtk_tools" 2>/dev/null
then
    run_renderer_python ldtk-manifest -m ambition_sprite2d_renderer \
        ldtk-manifest --out "$sprite_manifest" 2>&1 | sed 's/^/  /' || true
    # Each world carries the same PlayerStart icon wiring. A world left out
    # of this loop keeps a tileset def that drifts onto a sheet name the
    # renderer stopped publishing (the hall did, and then the Math world was
    # left out of a hand list). So the worlds are the pack's own list,
    # `worlds.ron`. Pruning drops the def that repointing orphans, so no
    # world keeps naming a PNG that no longer ships.
    for world in $(ldtk_world_files); do
        ldtk_path="$worlds_dir/$world"
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
phase "LDtk editor art:"
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
phase "director vanity card (part sheet + baked placements → $content_assets_dir/vanity_card_made_this_meme)"
# The card is a render of the director's rig: about 56 s of SVG rasterizing,
# and it ran on every publish (measured 2026-10-15). It is keyed as a target
# is (`script_key`): the core, its two scripts, the robot target it imports,
# and the files its last run read, which this run records. A run with the
# same key and both outputs present is skipped.
vanity_record="$target_reads_dir/director_vanity_card.reads"
vanity_key_file="$cache_dir/director_vanity_card.key"
vanity_dir="$content_assets_dir/vanity_card_made_this_meme"
vanity_ron="$content_assets_dir/data/vanity_card_made_this_meme.ron"
vanity_key="$(script_key "$vanity_record" "player_robot_v3" \
    scripts/export_director_vanity_card.py scripts/render_director_vanity_dialog.py)"
if [ -d "$vanity_dir" ] && [ -f "$vanity_ron" ] && [ -f "$vanity_key_file" ] \
    && [ "$(cat "$vanity_key_file")" = "$vanity_key" ]; then
    echo "  current: no input changed since the last export"
else
    rm -f "$vanity_key_file"
    if run_renderer_python "vanity-card" - "$repo_root" "$vanity_record" "$vanity_dir" "$vanity_ron" \
        2>&1 <<'PYEOF' | sed 's/^/  /'
import runpy
import sys
from pathlib import Path

from ambition_sprite2d_renderer import read_record

root, record, *outputs = sys.argv[1:]
renderer = Path.cwd()
read_record.start(
    Path(root),
    [renderer / "ambition_sprite2d_renderer" / "generated", renderer / ".cache", *map(Path, outputs)],
)
sys.argv = ["scripts/export_director_vanity_card.py"]
try:
    runpy.run_path("scripts/export_director_vanity_card.py", run_name="__main__")
except SystemExit as done:
    if done.code not in (None, 0):
        raise
read_record.write(Path(record))
PYEOF
    then
        # Keyed again: the record this run wrote is part of the key.
        script_key "$vanity_record" "player_robot_v3" \
            scripts/export_director_vanity_card.py scripts/render_director_vanity_dialog.py >"$vanity_key_file"
    else
        regen_failures+=("director vanity card: exporter reported a failure")
    fi
fi

# --- Verdict --------------------------------------------------------------
# Every stage has now run. Fail here, once, for anything any of them recorded —
# and leave the fingerprint uncached so the next run re-renders rather than
# reporting a cache hit over a broken tree.
if [ "${#regen_failures[@]}" -gt 0 ]; then
    print_regen_timings
    print_phase_seconds
    echo "" >&2
    echo "==> regen FAILED — ${#regen_failures[@]} problem(s):" >&2
    for failure in "${regen_failures[@]}"; do
        echo "    $failure" >&2
    done
    echo "    (fingerprint not cached; the next run re-renders)" >&2
    if [ -d "$staging_root" ]; then
        echo "    (the runtime roots were not replaced; the next run continues $staging_root)" >&2
    fi
    exit 1
fi

# --- Write fingerprint on success ----------------------------------------
# Of the tree this run leaves (`fingerprint_after_publish`): the renders
# rewrote their read records.
mkdir -p "$cache_dir"
if final_fingerprint="$(fingerprint_after_publish "$started_authored_fingerprint")"; then
    echo "$final_fingerprint" > "$fingerprint_file"
    echo "  cached regen fingerprint at $fingerprint_file"
else
    echo "  an authored input changed while the publish ran; fingerprint not cached (the next run looks again)" >&2
fi

print_regen_timings
print_phase_seconds

if [ "$line_profile_enabled" -eq 1 ]; then
    echo "==> line profile reports: $line_profile_run_dir"
    echo "    Inspect one with: $python_bin -m line_profiler -rtmz <report>.lprof"
fi

echo "==> done"
echo "    file://$sprites_dir"
echo "    file://$repo_root/crates/ambition_platformer2d_actor_monolith/assets"
