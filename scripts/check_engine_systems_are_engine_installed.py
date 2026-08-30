#!/usr/bin/env python3
"""Find engine systems that are installed only by the shipped game composition.

The check identifies reusable engine systems whose registration has leaked into a
game host, so headless/demo consumers cannot accidentally omit required behavior."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]

# Compositions that are a GAME rather than the engine. A system registered only
# here is registered by exactly one composition.
APP_ROOTS = ["game"]

# Where an engine plugin may register a system so that every composition gets it.
ENGINE_ROOTS = ["crates"]

# A guard with 42 waivers is a guard nobody reads — this file's sibling says so in its own
# docstring — and the class that has actually bitten is PRESENTATION BINDING, three times, all
# in the render crate: a label pass, a theme load, a layer sync. So the question is asked where
# the answer has been wrong.
#
# Widening this is a deliberate act with a cost: run `--list --all` first and
# read what comes back before deciding the rest is signal.
RENDER_PATH_PREFIXES = (
    "ambition_render::",
    "ambition_platformer2d::render::",
    # The finding is about this list rather than about any one system: the narrowing is by PATH,
    # and presentation is not only in the render crate. `dialog_reveal_tick` owns the typewriter
    # timing — its own doc says "this is presentation only" — and lives in `ambition_dialog`, so no
    # prefix here could ever see it. The class this guard exists for was hiding one module outside
    # the paths it looked at.
    "ambition_platformer2d::dialog::",
    "ambition_dialog::",
)

# The broad form, kept for `--all`: every engine crate, umbrella included.
ENGINE_PATH_ROOTS = (
    "ambition_platformer2d",
    "ambition_platformer2d_actor_monolith",
    "ambition_audio",
    "ambition_character_sprites",
    "ambition_combat",
    "ambition_encounter",
    "ambition_platformer2d_core",
    "ambition_game_shell",
    "ambition_platformer2d_host",
    "ambition_input",
    "ambition_interaction",
    "ambition_items",
    "ambition_platformer2d_ldtk",
    "ambition_load_presentation",
    "ambition_menu",
    "ambition_persistence",
    "ambition_platformer2d_shared_tangle",
    "ambition_platformer2d_provider",
    "ambition_portal2d",
    "ambition_portal2d_presentation",
    "ambition_projectiles",
    "ambition_render",
    "ambition_platformer2d_runtime",
    "ambition_settings_menu",
    "ambition_sfx",
    "ambition_sim_view",
    "ambition_sprite_sheet",
    "ambition_touch_input",
    "ambition_ui_nav",
    "ambition_vfx",
    "ambition_platformer2d_world",
)

# ── The waivers ──
#
# id → why this engine system is deliberately registered by the app alone. Every
# entry is a DECISION about who owns a behaviour, and deleting one is how you say
# "actually every composition should have this".
WAIVERS: dict[str, str] = {
    "refresh_entity_sprite_handles_on_game_assets_change": (
        "pairs with `reload_visual_quality_assets_on_scale_change` — an "
        "app-local system, because the settings menu is. A demo's GameAssets "
        "does not "
        "change after startup, so there is nothing for it to refresh."
    ),
    "sync_health_overlays": (
        "generic code, but floating health bars over every body are a GAME's "
        "choice. Installing them engine-side would decide a demo's HUD for it."
    ),
    "sync_lock_wall_visuals": (
        "encounter lock walls: same reasoning. A game without encounters would "
        "pay for a system that reconciles an empty set."
    ),
    "sync_boss_health_bar_overlay": (
        "a boss HUD is Ambition's, and no other composition ships a boss."
    ),
    "setup_capture_target": (
        "a CAPTURE BINARY's own render target. Building one is the tool's whole "
        "point, and no shipped composition should ever install it — the engine "
        "owning this would mean every game allocated a screenshot texture. "
        "⚠ AND IT IS THE GUARD'S BLIND SPOT, which is why it is written down "
        "rather than sidestepped: `capture_sanic` and `capture_mary_o` register "
        "the SAME system and neither is reported, because they import the name "
        "and this script only recognises a qualified `a::b::c` path. "
        "`capture_twintrack` spelled the path out and was caught. The class was "
        "always here; only the spelling decided whether anyone saw it."
    ),
    "adopt_cameras_into_capture_target": (
        "`setup_capture_target`'s other half, and waived for the same reason: it "
        "points a capture binary's cameras at the render target that binary just "
        "built. A shipped composition has no capture target for it to adopt. "
        "⚠ AND IT IS THE SAME BLIND SPOT the waiver above documents, now with a "
        "count: SIX binaries register this system and only the TWO that spell out "
        "`ambition_platformer2d::render::capture::…` are reported — "
        "`moveset_render` and `zero_time_capture_spike`. The other four import "
        "the bare name and are invisible to this script."
    ),
    "apply_placeholder_sprites_override": (
        "dev tool. The app owns its dev overlay and its hotkeys."
    ),
    "apply_hide_sprites_override": ("dev tool, as above."),
    "sync_portal_capture_parallax_layers": (
        "portal is Ambition's feature and is feature-gated behind "
        "`portal_render`; nothing else composes a portal rig."
    ),
    "sync_portal_sprite_visibility": ("portal, as above."),
    "sync_portal_sprite_animation": ("portal, as above."),
    "sync_portal_ring_rotation_system": ("portal, as above."),
    "hide_portal_loading_zone_visuals": ("portal, as above."),
    # The system took `Res<ResolvedVisualQuality>`, which `VisualQualityPlugin` owns and no other
    # composition installed, so it PANICKED everywhere but the shipped app. A waiver answers this
    # checker's question; this checker's question was never the dangerous one.
    #
    # The system lives in `VisualQualityPlugin` now, beside its resource, so
    # there is nothing left to waive.
    "fit_to_reading_rect": (
        "the cutscene overlay's half of it, registered in the SAME `.chain()` as "
        "`sync_cutscene_ui` below and waived on that entry's reasoning exactly: "
        "it follows the system whose root it repositions, and it moves the day "
        "that one does. The dialogue's half is NOT here — "
        "`DefaultDialogUiPlugin` installs it engine-side, which is the shape "
        "both should end up in. "
        "AND THE OTHER QUESTION, because on 2026-08-04 a waiver that answered "
        "only this file's question cost four panics (queue D16): its params are "
        "`Option<Res<ResolvedGameplayPresentation>>` and a `Query`, so there is "
        "no composition it can fail to validate in. A waiver here says nothing "
        "about that, and it has to be checked separately."
    ),
    "sync_cutscene_ui": (
        "WAIVED ON THE STANDARD, not on the architecture (2026-07-31). It is "
        "generic — `ambition_cutscene` is an engine crate and the engine group "
        "installs `CutsceneSchedulePlugin`, so a demo that played a cutscene "
        "would get the beats and no overlay. But no demo opens one, the app "
        "registers it inside a four-system `.chain()` with its own HUD, and "
        "moving it would reorder the shipped host's UI to serve a consumer that "
        "does not exist. Move it the day a demo plays a cutscene: the params "
        "must become `Option` first (`Res<ActiveCutscene>` panics rather than "
        "skips), and the app's chain loses a member."
    ),
    "report_image_census": (
        "a startup diagnostic the app prints; it draws nothing."
    ),
    "spawn_player_hud": (
        "the built-in HUD is opt-in per game — `toggle_builtin_hud_for_declared_games` "
        "is the seam a game uses to say it wants one. Installing it for every "
        "composition would give every demo Ambition's HUD."
    ),
    "place_player_hud": ("the built-in HUD, as above."),
    "update_player_hud": ("the built-in HUD, as above."),
    "toggle_builtin_hud_for_declared_games": ("the built-in HUD, as above."),
}

# ── OPEN rows: the engine SHOULD own these, and a blocker is recorded ──
#
# not the same thing as a waiver, and kept apart on purpose. A waiver says
# "this belongs to a GAME"; an entry here says "this belongs to the ENGINE, the
# move is not mechanical, and here is the specific question it waits on". Merging
# the two would let "we have not done it" hide inside "we decided not to".
#
# EMPTY, and emptied by doing the work rather than by relaxing the bar. Bevy runs `PreUpdate` →
# `RunFixedMainLoop` → `Update`, and every host puts the sim ahead of `Update` — so the
# `.after(CoreSimulation)` pin is load-bearing under `RenderFrame` (where `sim_schedule()` IS
# `Update`) and merely decorative under the other two, while being correct in all three. No rewind
# was ever involved: neither `DialogState` nor `MenuControlFrame` is rollback state.
#
# the test on this registry is what forced the issue. Once the blocker was
# retracted the entry stopped naming one — exactly the "TODO with a ratchet's
# authority" that test rejects. Both systems now live in the monolith's
# `YarnBindingsPlugin`, beside the rest of the dialogue feature.
OPEN_ROWS: dict[str, str] = {}

# ── The ratchet ──
#
# Each is a system whose absence draws nothing and says nothing.
#
# The number may not GROW, and it may not silently shrink either. A budget that is never
# tightened is a budget that rots into a permanent allowance — the footprint ratchet in
# `check_absence_contracts.py` carries the same rule for the same reason.
#
# The budget stays as the mechanism: the next one somebody adds fails this check, and the fix is
# either a move or a sentence saying why the engine should not own it.
UNCLAIMED_BUDGET = 0

_BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.S)
_LINE_COMMENT = re.compile(r"//[^\n]*")
_QUALIFIED_PATH = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)+)")
_ADD_SYSTEMS = re.compile(r"\badd_systems\s*\(")
# `run_if` is a PREDICATE and `after`/`before`/`ambiguous_with` name a system
# somebody ELSE registered. Neither is a registration, and reading them as one is
# how this script first reported `sync_morph_ball_visual` and
# `sync_bubble_shield_visual` as app-only when `ambition_render` registers both —
# the app merely orders against them.
_NOT_A_REGISTRATION = re.compile(r"\b(?:run_if|after|before|ambiguous_with)\s*\(")


def strip_comments(source: str) -> str:
    """Comment text is not a registration. Same rule, same reason, as the
    absence checker: three guards there went red on a doc comment explaining a
    removal, and a paragraph naming `sync_parallax_layers` is the opposite of
    evidence that somebody registered it."""
    return _LINE_COMMENT.sub("", _BLOCK_COMMENT.sub(" ", source))


def add_systems_bodies(source: str) -> list[str]:
    """The text inside each `add_systems( … )`, by paren balance."""
    bodies = []
    for match in _ADD_SYSTEMS.finditer(source):
        depth = 1
        index = match.end()
        while index < len(source) and depth:
            char = source[index]
            if char == "(":
                depth += 1
            elif char == ")":
                depth -= 1
            index += 1
        bodies.append(source[match.end() : index - 1])
    return bodies


def strip_run_conditions(body: str) -> str:
    """Remove every `run_if( … )`, `after( … )`, `before( … )` argument.

    A run condition is not a system: reading one as a system is how the first
    version of this script reported `in_mode`, `in_base_mode`,
    `simulation_authorized` and `phase_mark` as unregistered engine
    presentation. They are predicates the app supplies to systems it is
    registering, which is the opposite of the thing being looked for.

    An ORDERING edge is not a registration either, and that one was worse
    because it looked like signal: `.after(morph_ball::sync_morph_ball_visual)`
    made this script report two ability visuals as app-only when
    `ambition_render` registers both and the app is merely ordering the dev
    sprite overrides against them. A guard that sends you to fix something
    already correct is the expensive kind of wrong.
    """
    out = []
    index = 0
    for match in _NOT_A_REGISTRATION.finditer(body):
        if match.start() < index:
            continue
        out.append(body[index : match.start()])
        depth = 1
        cursor = match.end()
        while cursor < len(body) and depth:
            if body[cursor] == "(":
                depth += 1
            elif body[cursor] == ")":
                depth -= 1
            cursor += 1
        index = cursor
    out.append(body[index:])
    return "".join(out)


def registered_engine_systems(
    root: Path, subdirs: list[str], every_crate: bool = False
) -> dict[str, set[str]]:
    """system name → the files under `subdirs` whose `add_systems` name it
    through an engine-rooted path."""
    found: dict[str, set[str]] = {}
    for subdir in subdirs:
        for path in (root / subdir).rglob("*.rs"):
            relative = path.relative_to(root).as_posix()
            # Test files register systems to exercise them, which says nothing
            # about what a composition installs.
            if "/tests/" in relative or relative.endswith("tests.rs"):
                continue
            source = strip_comments(path.read_text(encoding="utf-8", errors="replace"))
            for body in add_systems_bodies(source):
                body = strip_run_conditions(body)
                for qualified in _QUALIFIED_PATH.findall(body):
                    segments = qualified.split("::")
                    if len(segments) < 2:
                        continue
                    if every_crate:
                        if segments[0] not in ENGINE_PATH_ROOTS:
                            continue
                    elif not qualified.startswith(RENDER_PATH_PREFIXES):
                        continue
                    name = segments[-1]
                    # `Type::method` and set names are not systems; a system's
                    # last segment is snake_case by this repo's convention.
                    if not name.islower():
                        continue
                    found.setdefault(name, set()).add(relative)
    return found


def app_only_systems(root: Path, every_crate: bool = False) -> dict[str, set[str]]:
    """Engine systems an app registers that no engine crate registers."""
    by_app = registered_engine_systems(root, APP_ROOTS, every_crate)
    by_engine = registered_engine_systems(root, ENGINE_ROOTS, every_crate)
    return {
        name: files
        for name, files in by_app.items()
        if name not in by_engine and name not in WAIVERS and name not in OPEN_ROWS
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--list",
        action="store_true",
        help="print every app-registered engine system, waived or not",
    )
    parser.add_argument(
        "--all",
        action="store_true",
        help="ask the question of every engine crate, not just presentation "
        "(reports a lot; read before believing)",
    )
    args = parser.parse_args()

    if args.list:
        by_app = registered_engine_systems(REPO, APP_ROOTS, args.all)
        by_engine = registered_engine_systems(REPO, ENGINE_ROOTS, args.all)
        for name in sorted(by_app):
            if name in by_engine:
                continue
            if name in WAIVERS:
                mark = "waived"
            elif name in OPEN_ROWS:
                mark = "  OPEN"
            else:
                mark = "UNCLAIMED"
            print(f"{mark:>9}  {name}  ({', '.join(sorted(by_app[name]))})")
        return 0

    offenders = app_only_systems(REPO, args.all)
    if args.all:
        # The wide question is a report, never a gate: most of what it returns is
        # a game composing its own sim, and pretending otherwise is how a guard
        # becomes noise.
        for name in sorted(offenders):
            print(f"  {name}  ({', '.join(sorted(offenders[name]))})")
        print(f"\n{len(offenders)} unclaimed across every engine crate (report only)")
        return 0

    if len(offenders) > UNCLAIMED_BUDGET:
        print(
            f"{len(offenders)} engine presentation systems are installed by ONE "
            f"composition; the ratchet allows {UNCLAIMED_BUDGET}. Every other "
            "composition runs an engine with these missing, and the failure is "
            "silent every time:\n"
        )
        for name in sorted(offenders):
            print(f"  {name}")
            for file in sorted(offenders[name]):
                print(f"      registered in {file}")
        print(
            "\nMove the registration into the engine plugin that owns the family, "
            "or add it to OPEN_ROWS with the specific blocker if the engine "
            "SHOULD own it and the move is not mechanical, "
            "or add the system to WAIVERS in this file with the reason it belongs "
            "to a GAME rather than to the engine."
        )
        return 1

    if len(offenders) < UNCLAIMED_BUDGET:
        print(
            f"only {len(offenders)} unclaimed, and the ratchet still allows "
            f"{UNCLAIMED_BUDGET}. Lower UNCLAIMED_BUDGET to {len(offenders)} in "
            "the commit that fixed one — a budget nobody tightens becomes a "
            "permanent allowance."
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
