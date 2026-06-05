"""Modal CLI for ambition_sprite2d_renderer.

Two command families:

(1) **Unified Target commands** — work with any registered target
    (tack-ons under ``targets/<category>/``, main configs under
    ``configs/``, review-NPC configs under ``configs/review/``). Each
    takes an optional ``<TARGET>`` name. With a name: act on that
    target. Without: bulk over every tack-on target.

      list                        Show every registered target, grouped by category.
      canonical [<target>]        One canonical pose, or the full gallery.
      sheet [<target>]            One full sprite sheet, or every tack-on sheet.
      install [<target>]          Copy one target's files to sandbox assets, or all.
      publish [<target>]          sheet + install (one, or every tack-on).

(2) **Adapter-pipeline commands** — take config paths instead of
    target names, scoped to the YAML adapter pipeline. Useful for
    one-off art iteration with custom configs and for the curated
    runtime-NPC publishing path.

      draw-all                    Render every config in ``configs/``.
      draw-review                 Render every config in ``configs/review/``.
      draw-character <config>     One config: canonical + spritesheet + YAML.
      draw-factions               Music-faction lineup review render.
      draw-runtime-npcs           Render + install the curated review-NPC subset.
      regenerate-all              draw-all + publish + draw-runtime-npcs.
      spritesheet <config> <out>  One config's sheet to a specific path.
      single <config> <out>       One frame from a config.

See ``target_registry.py`` for the Target protocol contract.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import List

from .adapters import TARGETS, get_adapter
from .canonical import (
    draw_canonical_of,
    render_canonical,
    write_canonicals,
    write_gallery,
)
from .console import print_canonical_outputs, print_path, print_paths
from .config import CharacterJob, load_jobs
from .faction_lineup import write_faction_lineup
from .debug_hitboxes import render_debug_overlay
from .sheet import write_spritesheet
from .target_registry import (
    CATEGORIES,
    DiscoveryReport,
    Target,
    discover_all_targets,
)


def package_dir() -> Path:
    return Path(__file__).resolve().parent.parent


def repo_root() -> Path:
    # tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/cli.py -> repo root.
    return Path(__file__).resolve().parents[3]


# Defaults are computed against the package, not the cwd, so the CLI works
# regardless of where the user runs it from.
DEFAULT_CONFIG_DIR = Path(__file__).resolve().parent / "configs"
DEFAULT_REVIEW_CONFIG_DIR = DEFAULT_CONFIG_DIR / "review"
DEFAULT_ASSET_DIR = package_dir() / "generated"
DEFAULT_FACTION_CONFIG = DEFAULT_CONFIG_DIR / "factions" / "music_factions.yaml"


# ---- Target registry ---------------------------------------------------------
#
# Unified discovery across every surface: tack-on Python modules under
# `targets/<category>/` AND YAML adapter configs under `configs/` /
# `configs/review/`. See `target_registry.discover_all_targets`.
#
# Adding a tack-on: drop a `.py` (or package dir) into the right
# category subdir. Adding an adapter target: drop a YAML config under
# `configs/` or `configs/review/`. Either way, no edit to this file
# is required.

_REPORT: DiscoveryReport = discover_all_targets()
_ALL_TARGETS: dict[str, Target] = _REPORT.targets


# Review configs whose generated spritesheets are loaded at runtime via
# the sandbox NPC sprite registry. `draw-all` skips `configs/review/`
# by design (those are art-iteration review jobs), but these specific
# ones produce assets the game needs. `draw-runtime-npcs` renders +
# installs them in one shot so a fresh checkout can boot with full
# NPC art without invoking `draw-character` ten times.
RUNTIME_REVIEW_NPCS: tuple[str, ...] = (
    "absurd_general",
    "architect",
    "erdish",
    "kernel_guide",
    "merchant_prototype",
    "oiler",
    "vault_keeper",
    # Cryptography crew batch 1 — Bob/Alice/Eve/Mallory/Trent/Judy.
    # See `docs/concepts/cryptography-crew.md` for the full canonical
    # roster. Batch 2 (Trudy/Craig/Sybil/Victor/Peggy/Walter/Olivia)
    # landed as toon-target sketches with phenotype variation; each
    # may be promoted to a bespoke template if a story room demands.
    "alice",
    "bob",
    "eve",
    "judy",
    "mallory",
    "trent",
    "trudy",
    "craig",
    "sybil",
    "victor",
    "peggy",
    "walter",
    "olivia",
)


def _get_target(name: str) -> Target:
    """Look up a target from the unified registry.

    If the name isn't registered but a matching file exists under
    ``targets/<category>/<name>.py``, surface the discovery warning for
    it (typically "no `render()` function") so the user knows *why*
    their file isn't registered, instead of just "unknown target."
    """
    if name in _ALL_TARGETS:
        return _ALL_TARGETS[name]
    # Look for a discovery warning matching this name. Warnings are
    # formatted as "<category>/<stem>: <reason>" so an `endswith` /
    # `:` split is enough to find the relevant one.
    for line in _REPORT.warnings:
        head, _, reason = line.partition(":")
        if head.endswith(f"/{name}"):
            raise SystemExit(
                f"error: target {name!r} is not registered.\n"
                f"  reason: {reason.strip()}\n"
                f"  location: {head.strip()}.py\n"
                f"  see `target_registry.py` for the Target protocol contract."
            )
    raise SystemExit(
        f"error: unknown target: {name!r}\n  run `list` to see the registered targets."
    )


def sandbox_sprites_dir() -> Path:
    return repo_root() / "crates" / "ambition_sandbox" / "assets" / "sprites"


def generated_dir(target_name: str) -> Path:
    return DEFAULT_ASSET_DIR / target_name


# ---- Adapter (character lab) commands -----------------------------------------


def draw_all(
    config_dir: str | Path = DEFAULT_CONFIG_DIR, out_dir: str | Path = DEFAULT_ASSET_DIR
) -> List[Path]:
    out_dir = Path(out_dir)
    config_dir_path = Path(config_dir)
    runtime_stems = {
        "boss",
        "raid_enforcer",
        "goblin",
        "ninja",
        "ninja_leader",
        "player_robot",
        "robot",
        "sandbag",
    }
    default_runtime_dir = (
        config_dir_path.resolve() == Path(DEFAULT_CONFIG_DIR).resolve()
    )
    outputs: List[Path] = []
    for path, job in load_jobs(config_dir_path):
        # The default configs/ directory has accumulated a few older review
        # jobs for compatibility. Keep draw-all focused on the runtime sheets
        # so it stays quick and does not unexpectedly publish review variants.
        # Custom config dirs still render every .yaml they contain.
        stem = path.stem
        if default_runtime_dir and stem not in runtime_stems:
            continue
        # Use an explicit output_name when provided, otherwise the config stem,
        # so multiple variants of the same adapter do not overwrite each other.
        stem = job.output_stem(path)
        image_out = out_dir / f"{stem}_spritesheet.png"
        manifest_out = out_dir / f"{stem}_spritesheet.yaml"
        outputs.extend(
            write_spritesheet(job, image_out, manifest_out, source_config=path)
        )
    return outputs


def draw_review(
    config_dir: str | Path = DEFAULT_REVIEW_CONFIG_DIR,
    out_dir: str | Path = DEFAULT_ASSET_DIR / "review",
) -> List[Path]:
    # Scoped to `config_dir` — use the adapter-only `write_canonicals`
    # path rather than `draw_canonicals` (which now does the full
    # adapters + tack-ons + review-NPCs gallery and would balloon a
    # review-of-this-dir into a full-roster render).
    outputs = draw_all(config_dir, out_dir)
    outputs += write_canonicals(config_dir, Path(out_dir) / "canonicals")
    return outputs


def draw_canonicals(
    config_dir: str | Path = DEFAULT_CONFIG_DIR,
    out_dir: str | Path = DEFAULT_ASSET_DIR / "canonicals",
    *,
    adapters_only: bool = False,
) -> List[Path]:
    """Draw the full canonical gallery: adapters + tack-ons + review NPCs.

    Every canonical is drawn fresh by invoking the per-target renderer
    — does NOT read from any cached ``generated/<name>/`` files. Tiles
    are composited onto a consistent gallery backdrop with per-category
    section headers (Adapter targets, Review NPCs, Tack-on
    characters/props/tiles/icons) so it reads as one unified review piece.

    Set ``adapters_only=True`` for the legacy behavior that walks
    ``configs/*.yaml`` only (adapter targets, no tack-ons or review NPCs).
    """
    if adapters_only:
        return write_canonicals(config_dir, out_dir)
    outputs, warnings = write_gallery(out_dir, _ALL_TARGETS.values())
    for line in warnings:
        print(f"warning: {line}", file=sys.stderr)
    return outputs


def resolve_config_path(value: str | Path) -> Path:
    """Resolve a config name or path to a concrete ``Path``.

    Lookup order:

    1. If ``value`` is a path that exists, return it as-is.
    2. ``configs/<value>.yaml`` (main adapter rigs).
    3. ``configs/review/<value>.yaml`` (review NPCs).

    Raises ``FileNotFoundError`` with the search paths if no match.
    Lets callers pass a short name (``boss``,
    ``robot_guardian``, ``architect``) instead of the full
    ``ambition_sprite2d_renderer/configs/boss.yaml`` path.
    """
    candidate = Path(value)
    if candidate.exists():
        return candidate
    # Strip any extension the user typed so `boss.yaml` and `boss`
    # both work.
    stem = candidate.stem if candidate.suffix else candidate.name
    candidates = [
        DEFAULT_CONFIG_DIR / f"{stem}.yaml",
        DEFAULT_REVIEW_CONFIG_DIR / f"{stem}.yaml",
    ]
    for path in candidates:
        if path.exists():
            return path
    raise FileNotFoundError(
        f"config not found: {value!r}; tried {[str(p) for p in [candidate, *candidates]]}"
    )


def draw_character(
    config: str | Path, out_dir: str | Path = DEFAULT_ASSET_DIR
) -> List[Path]:
    """Render both review artifacts for one character config.

    This is the one-shot path for art iteration: it writes the canonical still
    frame used for visual review and the runtime spritesheet + YAML manifest
    used by the game.  It deliberately shares the same `CharacterJob` adapter
    path as `single` and `spritesheet`, so the canonical pose and the sheet are
    generated from the exact same spec.

    Canonical PNGs land in ``<out_dir>/canonicals/`` so they don't visually
    mix with the per-character spritesheet PNGs in ``<out_dir>/`` when an
    artist pages through the folder. Spritesheet + manifest stay at the
    top of ``<out_dir>`` because that's where the runtime asset loader
    looks for them.
    """
    config_path = Path(config)
    out_dir = Path(out_dir)
    job = CharacterJob.load(config_path)
    stem = job.output_stem(config_path)
    out_dir.mkdir(parents=True, exist_ok=True)

    canonical_dir = out_dir / "canonicals"
    canonical_dir.mkdir(parents=True, exist_ok=True)
    canonical_out = canonical_dir / f"{stem}_canonical.png"
    render_canonical(job).save(canonical_out)

    sheet_out = out_dir / f"{stem}_spritesheet.png"
    manifest_out = out_dir / f"{stem}_spritesheet.yaml"
    image_out, yaml_out = write_spritesheet(
        job, sheet_out, manifest_out, source_config=config_path
    )
    actor_out = out_dir / f"{stem}_actor.ron"
    outputs = [canonical_out, image_out, yaml_out]
    if actor_out.exists():
        outputs.append(actor_out)
    return outputs


def draw_factions(
    config: str | Path = DEFAULT_FACTION_CONFIG,
    out_dir: str | Path = DEFAULT_ASSET_DIR / "factions",
) -> List[Path]:
    return write_faction_lineup(config, out_dir)


def _cmd_draw_all(args: argparse.Namespace) -> int:
    print_paths(draw_all(args.config_dir, args.out_dir))
    return 0


def _cmd_draw_review(args: argparse.Namespace) -> int:
    print_paths(draw_review(args.config_dir, args.out_dir))
    return 0


def _cmd_canonical(args: argparse.Namespace) -> int:
    """`canonical [<name>]` — draw one canonical, or the full gallery."""
    if args.target:
        target = _get_target(args.target)
        out = draw_canonical_of(target, args.out_dir)
        print_paths([out])
        return 0
    print_canonical_outputs(
        draw_canonicals(
            args.config_dir,
            args.out_dir,
            adapters_only=args.adapters_only,
        )
    )
    return 0


def _cmd_draw_character(args: argparse.Namespace) -> int:
    try:
        config_path = resolve_config_path(args.config)
    except FileNotFoundError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    outputs = draw_character(config_path, args.out_dir)
    print_paths(outputs)
    if getattr(args, "debug_hitboxes", False):
        # Find the YAML sidecar in the outputs to feed to the
        # overlay. `draw_character` returns it as the third path
        # (canonical, sheet PNG, sheet YAML, optional actor RON).
        yaml_out = next((p for p in outputs if p.suffix == ".yaml"), None)
        if yaml_out is None:
            print(
                "warning: --debug-hitboxes set but no YAML manifest in outputs",
                file=sys.stderr,
            )
            return 0
        try:
            written = render_debug_overlay(yaml_out)
        except FileNotFoundError as e:
            print(f"error: debug overlay failed: {e}", file=sys.stderr)
            return 1
        print_path(written, prefix="  debug overlay: ")
    return 0


def _cmd_draw_factions(args: argparse.Namespace) -> int:
    print_paths(draw_factions(args.config, args.out_dir))
    return 0


def _cmd_list_targets(args: argparse.Namespace) -> int:
    print(
        "# adapter rigs (driven by configs/*.yaml — renders via draw-character / draw-all):"
    )
    for target in sorted(TARGETS):
        adapter = get_adapter(target)
        print(f"  {target}: {', '.join(adapter.default_animations())}")
    print("# registered targets (unified — works with render/install/canonical):")
    by_category: dict[str, list[str]] = {cat: [] for cat in CATEGORIES}
    for name, tgt in _ALL_TARGETS.items():
        by_category.setdefault(tgt.category, []).append(name)
    for category in CATEGORIES:
        names = sorted(by_category.get(category, []))
        if not names:
            continue
        print(f"  [{category}]")
        for name in names:
            marker = (
                "  (runtime)"
                if category == "characters" and name in RUNTIME_REVIEW_NPCS
                else ""
            )
            print(f"    {name}{marker}")
    if _REPORT.warnings:
        print(
            "# warnings (files in targets/ that don't conform to the Target API):",
            file=sys.stderr,
        )
        for line in _REPORT.warnings:
            print(f"  {line}", file=sys.stderr)
    return 0


def _cmd_spritesheet(args: argparse.Namespace) -> int:
    job = CharacterJob.load(args.config)
    print_paths(
        write_spritesheet(
            job, args.output, args.manifest_out, source_config=args.config
        )
    )
    return 0


def _cmd_single(args: argparse.Namespace) -> int:
    job = CharacterJob.load(args.config)
    adapter = get_adapter(job.target)
    spec = adapter.sample_spec(job)
    img = adapter.render_single(spec, args.animation, args.frame_index, job)
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    img.save(output)
    print_paths([output])
    return 0


# ---- Target sheet / install / publish commands -------------------------------

# Tack-on categories that bulk operations scope to. The adapter
# surface (`characters` includes both tack-on and main-config targets;
# `review_npcs` is its own thing) has its own bulk paths via
# `draw-all` / `draw-runtime-npcs`.
_TACKON_CATEGORIES = frozenset({"characters", "props", "tiles", "icons"})


def _tackon_target_names() -> list[str]:
    """Names of every tack-on (non-AdapterTarget) target, sorted."""
    from .target_registry import TackonTarget

    return sorted(
        name for name, t in _ALL_TARGETS.items() if isinstance(t, TackonTarget)
    )


def _render_target(target_name: str) -> List[Path]:
    target = _get_target(target_name)
    out_dir = generated_dir(target_name)
    paths = list(target.render_sheet(out_dir))
    print_paths(paths)
    return paths


def _install_target(target_name: str, dest_root: Path) -> List[Path]:
    target = _get_target(target_name)
    out_dir = generated_dir(target_name)
    # Both TackonTarget and AdapterTarget implement `install` with a
    # default copy-each-SHEET_FILES; targets that need custom behavior
    # (e.g. mockingbird_boss with its subdirectory of part files)
    # override the method.
    copied = list(target.install(out_dir, dest_root))
    print_paths(copied)
    return copied


def _bulk_over(
    op_name: str,
    target_names: list[str],
    op: "callable",
) -> int:
    """Run ``op(name)`` over each ``target_names``; report failures, return rc."""
    failures: list[str] = []
    for name in target_names:
        print(f"\n# {name}")
        try:
            op(name)
        except Exception as ex:  # noqa: BLE001 - report and continue
            print(f"error: target {name!r} failed: {ex}", file=sys.stderr)
            failures.append(name)
    if failures:
        print(
            f"\n{op_name} completed with {len(failures)} failure(s): "
            + ", ".join(failures),
            file=sys.stderr,
        )
        return 1
    return 0


def _cmd_sheet(args: argparse.Namespace) -> int:
    """`sheet [<name>]` — render one sheet, or every tack-on sheet."""
    if args.target:
        _render_target(args.target)
        return 0
    return _bulk_over("sheet", _tackon_target_names(), _render_target)


def _cmd_install(args: argparse.Namespace) -> int:
    """`install [<name>]` — install one target's files, or every tack-on's."""
    if args.target:
        copied = _install_target(args.target, args.dest_root)
        return 0 if copied else 1
    return _bulk_over(
        "install",
        _tackon_target_names(),
        lambda name: _install_target(name, args.dest_root),
    )


def _cmd_publish(args: argparse.Namespace) -> int:
    """`publish [<name>]` — sheet + install for one target, or all tack-ons."""
    if args.target:
        _render_target(args.target)
        copied = _install_target(args.target, args.dest_root)
        return 0 if copied else 1

    def _publish_one(name: str) -> None:
        _render_target(name)
        _install_target(name, args.dest_root)

    return _bulk_over("publish", _tackon_target_names(), _publish_one)


def _cmd_regenerate_all(args: argparse.Namespace) -> int:
    """Single-button regen: render + install every sprite the sandbox
    runtime can consume.

    Composes three existing convenience commands so a fresh checkout
    only needs one invocation to be art-current:

    1. `draw-all --out-dir <sandbox assets>` — adapter-driven sheets
       (player_robot, robot, goblin, ninja, ninja_leader, sandbag,
       boss, raid_enforcer).
    2. `publish` (no target) — every tack-on target under `targets/`.
    3. `draw-runtime-npcs` — review-config toon NPCs that the runtime
       sprite registry expects (architect, kernel_guide, vault_keeper,
       merchant_prototype, absurd_general, oiler, erdish).

    Errors in any sub-step are reported but don't abort the others.
    """
    dest = Path(args.dest_root)
    print("# step 1/3: draw-all (adapter sheets) -> sandbox assets")
    failures: list[str] = []
    try:
        outputs = draw_all(DEFAULT_CONFIG_DIR, dest)
        print_paths(outputs)
    except Exception as ex:  # noqa: BLE001
        print(f"error: draw-all failed: {ex}", file=sys.stderr)
        failures.append("draw-all")

    print("\n# step 2/3: publish (every tack-on target)")
    publish_args = argparse.Namespace(target=None, dest_root=dest)
    rc = _cmd_publish(publish_args)
    if rc != 0:
        failures.append("publish")

    print("\n# step 3/3: draw-runtime-npcs (review-config NPCs)")
    npc_args = argparse.Namespace(
        review_dir=str(DEFAULT_REVIEW_CONFIG_DIR), out_dir=str(dest)
    )
    rc = _cmd_draw_runtime_npcs(npc_args)
    if rc != 0:
        failures.append("draw-runtime-npcs")

    if failures:
        print(
            f"\nregenerate-all completed with failure(s): {', '.join(failures)}",
            file=sys.stderr,
        )
        return 1
    print("\nregenerate-all OK")
    return 0


def _cmd_debug_hitboxes(args: argparse.Namespace) -> int:
    """Overlay per-animation hurt + hit boxes on a rendered spritesheet.

    Reads the sheet's YAML manifest, finds the matching PNG, and
    writes a sibling ``*_debug.png`` with cyan hurtbox + red hitbox
    outlines (+ legend) over every frame. Sprite authors run this
    after a render to verify the boxes line up with the visible
    body / strike pose.
    """
    yaml_path = _resolve_sheet_yaml(args.yaml_or_target)
    if yaml_path is None:
        print(
            f"error: {args.yaml_or_target!r} is neither a file nor a known target name "
            f"(searched {sandbox_sprites_dir()})",
            file=sys.stderr,
        )
        return 1
    out_path = args.out
    try:
        written = render_debug_overlay(yaml_path, out_path)
    except FileNotFoundError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    print_path(written, prefix="wrote debug overlay: ")
    return 0


def _resolve_sheet_yaml(value: str | Path) -> Path | None:
    """Resolve a YAML path or a short target name to a sheet manifest.

    Lookup order:

    1. ``value`` as a path that exists.
    2. ``<sandbox_sprites>/<value>_spritesheet.yaml`` (standard
       install location).
    3. ``<sandbox_sprites>/<stem>_spritesheet.yaml`` where
       ``stem`` is ``value`` with any extension stripped (so
       ``boss``, ``boss.yaml``, and ``boss_spritesheet.yaml``
       all resolve to the same file).
    """
    candidate = Path(value)
    if candidate.exists():
        return candidate
    sprites_dir = sandbox_sprites_dir()
    stem = candidate.stem if candidate.suffix else candidate.name
    # Strip a trailing `_spritesheet` so users can pass either
    # `boss` or `boss_spritesheet`.
    if stem.endswith("_spritesheet"):
        stem = stem[: -len("_spritesheet")]
    for path in (
        sprites_dir / f"{stem}_spritesheet.yaml",
        sprites_dir / "review" / f"{stem}_spritesheet.yaml",
    ):
        if path.exists():
            return path
    return None


def _cmd_draw_runtime_npcs(args: argparse.Namespace) -> int:
    """Render + install every review-config NPC that the runtime sprite
    registry expects at boot. These configs live under `configs/review/`
    so `draw-all` skips them by default; this one walks the
    [`RUNTIME_REVIEW_NPCS`] tuple and runs `draw-character` for each."""
    review_dir = Path(args.review_dir)
    out_dir = Path(args.out_dir)
    failures: list[str] = []
    all_outputs: List[Path] = []
    for stem in RUNTIME_REVIEW_NPCS:
        cfg = review_dir / f"{stem}.yaml"
        if not cfg.exists():
            print(
                f"error: missing review config for runtime NPC {stem!r}: {cfg}",
                file=sys.stderr,
            )
            failures.append(stem)
            continue
        try:
            paths = draw_character(cfg, out_dir)
            all_outputs.extend(paths)
        except Exception as ex:  # noqa: BLE001
            print(
                f"error: rendering runtime NPC {stem!r} failed: {ex}",
                file=sys.stderr,
            )
            failures.append(stem)
    print_paths(all_outputs)
    if failures:
        print(
            f"\ndraw-runtime-npcs completed with {len(failures)} failure(s): "
            + ", ".join(failures),
            file=sys.stderr,
        )
        return 1
    return 0


def _add_optional_target_arg(p: argparse.ArgumentParser) -> None:
    """Optional TARGET positional — empty means bulk over every tack-on.

    No ``choices=`` constraint here so that ``_get_target`` can surface
    a useful error when the name matches a file under
    ``targets/<category>/`` but the file is missing the tack-on API.
    With ``choices=`` argparse would error before our handler runs
    and we couldn't show the warning.
    """
    p.add_argument(
        "target",
        metavar="TARGET",
        nargs="?",
        default=None,
        help=(
            "target id — name from `list`. Omit to bulk over every "
            "registered tack-on target (characters/props/tiles/icons)."
        ),
    )


def _add_dest_root_arg(p: argparse.ArgumentParser) -> None:
    p.add_argument(
        "--dest-root",
        type=Path,
        default=sandbox_sprites_dir(),
        help="install destination (default: crates/ambition_sandbox/assets/sprites)",
    )


def _add_config_dir_args(
    p: argparse.ArgumentParser,
    *,
    config_default: Path,
    out_default: Path,
) -> None:
    p.add_argument("--config-dir", default=str(config_default))
    p.add_argument("--out-dir", default=str(out_default))


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="ambition_sprite2d_renderer",
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    sub = parser.add_subparsers(dest="command", required=True)

    # ---- Unified Target commands (take an optional <TARGET> name) ----------
    #
    # With a name: act on that target. Without a name: bulk over every
    # tack-on target (characters/props/tiles/icons). The YAML adapter
    # surface is bulk-rendered separately via `draw-all` /
    # `draw-runtime-npcs` because those have surface-specific semantics.

    p = sub.add_parser(
        "canonical",
        help=(
            "Draw a single target's canonical pose, or the full gallery if "
            "no target is given. Sources canonicals from every surface "
            "(tack-ons, main configs, review NPCs)."
        ),
    )
    _add_optional_target_arg(p)
    _add_config_dir_args(
        p,
        config_default=DEFAULT_CONFIG_DIR,
        out_default=DEFAULT_ASSET_DIR / "canonicals",
    )
    p.add_argument(
        "--adapters-only",
        action="store_true",
        help="(bulk mode only) skip tack-on + review NPC targets",
    )
    p.set_defaults(func=_cmd_canonical)

    p = sub.add_parser(
        "sheet",
        help=(
            "Render a single target's full sprite sheet bundle into generated/, "
            "or bulk-render every tack-on target if no name is given."
        ),
    )
    _add_optional_target_arg(p)
    p.set_defaults(func=_cmd_sheet)

    p = sub.add_parser(
        "install",
        help=(
            "Copy a single target's rendered files into the sandbox sprites "
            "dir, or bulk-install every tack-on target if no name is given."
        ),
    )
    _add_optional_target_arg(p)
    _add_dest_root_arg(p)
    p.set_defaults(func=_cmd_install)

    p = sub.add_parser(
        "publish",
        help=(
            "sheet + install for one target, or bulk for every tack-on target "
            "if no name is given."
        ),
    )
    _add_optional_target_arg(p)
    _add_dest_root_arg(p)
    p.set_defaults(func=_cmd_publish)

    p = sub.add_parser(
        "list", help="Show every registered target, grouped by category."
    )
    p.set_defaults(func=_cmd_list_targets)
    sub.add_parser("list-targets", help="alias of `list`").set_defaults(
        func=_cmd_list_targets
    )

    # ---- Adapter-pipeline commands (take config paths, not target names) ----

    p = sub.add_parser("draw-all", help="Render every main adapter config in configs/.")
    _add_config_dir_args(
        p, config_default=DEFAULT_CONFIG_DIR, out_default=DEFAULT_ASSET_DIR
    )
    p.set_defaults(func=_cmd_draw_all)

    p = sub.add_parser(
        "draw-review", help="Render every review config in configs/review/."
    )
    _add_config_dir_args(
        p,
        config_default=DEFAULT_REVIEW_CONFIG_DIR,
        out_default=DEFAULT_ASSET_DIR / "review",
    )
    p.set_defaults(func=_cmd_draw_review)

    p = sub.add_parser(
        "draw-character", help="Render one config's canonical + spritesheet + YAML."
    )
    p.add_argument(
        "config",
        help=(
            "Config to render. Either a path to a `*.yaml` file or a "
            "short name (e.g. `boss`, `robot_guardian`) — the latter "
            "resolves to `configs/<name>.yaml` or `configs/review/<name>.yaml`."
        ),
    )
    p.add_argument("--out-dir", default=str(DEFAULT_ASSET_DIR))
    p.add_argument(
        "--debug-hitboxes",
        action="store_true",
        help=(
            "After rendering, write `<sheet>_debug.png` next to the "
            "sheet PNG with per-animation hurt + hit boxes drawn over "
            "every frame. Equivalent to running `debug-hitboxes "
            "<sheet>.yaml` separately."
        ),
    )
    p.set_defaults(func=_cmd_draw_character)

    p = sub.add_parser(
        "draw-factions", help="Render music-faction leader/NPC review sprites."
    )
    p.add_argument("--config", default=str(DEFAULT_FACTION_CONFIG))
    p.add_argument("--out-dir", default=str(DEFAULT_ASSET_DIR / "factions"))
    p.set_defaults(func=_cmd_draw_factions)

    p = sub.add_parser(
        "spritesheet", help="Render one config's sheet to a specific path."
    )
    p.add_argument("config")
    p.add_argument("output")
    p.add_argument("--manifest-out", default=None)
    p.set_defaults(func=_cmd_spritesheet)

    p = sub.add_parser("single", help="Render one frame from a config.")
    p.add_argument("config")
    p.add_argument("output")
    p.add_argument("--animation", default="idle")
    p.add_argument("--frame-index", type=int, default=0)
    p.set_defaults(func=_cmd_single)

    p = sub.add_parser(
        "draw-runtime-npcs",
        help=(
            "Render + install every review-config NPC that the runtime "
            "sprite registry expects at boot (architect, kernel_guide, "
            "vault_keeper, merchant_prototype, absurd_general, oiler, "
            "erdish, plus the crypto crew batch 1: alice, bob, eve, "
            "judy, mallory, trent). These live under configs/review/ "
            "so draw-all skips them by default."
        ),
    )
    p.add_argument(
        "--review-dir",
        default=str(DEFAULT_REVIEW_CONFIG_DIR),
    )
    p.add_argument(
        "--out-dir",
        default=str(sandbox_sprites_dir()),
        help="install destination (default: crates/ambition_sandbox/assets/sprites)",
    )
    p.set_defaults(func=_cmd_draw_runtime_npcs)

    p = sub.add_parser(
        "regenerate-all",
        help=(
            "One-shot: draw-all + publish + draw-runtime-npcs, all installed "
            "into sandbox assets. Brings a fresh checkout's sprite directory "
            "up to date in one command."
        ),
    )
    p.add_argument(
        "--dest-root",
        type=Path,
        default=sandbox_sprites_dir(),
        help="install destination (default: crates/ambition_sandbox/assets/sprites)",
    )
    p.set_defaults(func=_cmd_regenerate_all)

    p = sub.add_parser(
        "debug-hitboxes",
        help=(
            "Overlay per-animation hurt + hit boxes on a rendered "
            "spritesheet. Writes a sibling `<sheet>_debug.png`."
        ),
    )
    p.add_argument(
        "yaml_or_target",
        help=(
            "Either: a path to the sheet's YAML manifest "
            "(e.g. `crates/ambition_sandbox/assets/sprites/boss_spritesheet.yaml`); "
            "OR a short target name (e.g. `boss`) that resolves to the "
            "expected `<sandbox_sprites>/<target>_spritesheet.yaml`."
        ),
    )
    p.add_argument(
        "--out",
        type=Path,
        default=None,
        help=(
            "Output PNG path. Default: sibling of the sheet PNG with "
            "`_debug.png` suffix."
        ),
    )
    p.set_defaults(func=_cmd_debug_hitboxes)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args) or 0)


if __name__ == "__main__":
    raise SystemExit(main())
