"""Modal CLI for ambition_sprite2d_renderer.

Two surfaces live here:

(a) The procedural character lab — formerly ``proc2d_character_lab``.
    Targets defined under ``targets/`` (robot_side, goblin_side, boss_side,
    toon_side, robot25d) are driven by YAML jobs in ``configs/`` and the
    ``adapters.TARGETS`` registry.

      list-targets               Show registered character adapters.
      draw-all                   Render core runtime jobs in ``configs/``.
      draw-review                Render review/variant jobs in ``configs/review/``.
      draw-canonicals            Render canonical poses + contact sheet.
      draw-character <config>    Render canonical + spritesheet + YAML for one job.
      draw-entities              Render non-character gameplay entity sprites.
      spritesheet <config> <out> Render one job's sheet.
      single <config> <out>      Render one frame.

(b) Tack-on targets (e.g. sandbag, ghoul_skulker) that ship their own
    ``render(out_dir, **opts) -> Iterable[Path]`` function. They are
    auto-discovered from ``targets/*.py`` via
    :mod:`ambition_sprite2d_renderer.target_registry` — there is no
    central registration list to edit, so dropping a new file in
    ``targets/`` is the entire integration step.

      render <target>            Render into ``generated/<target>/``.
      preview <target>           Render and print the resulting paths.
      install <target>           Copy generated files into sandbox assets.
      render-publish <target>    Render then install.

See ``target_registry.py`` for the tack-on API contract.
"""
from __future__ import annotations

import argparse
import shutil
import sys
from pathlib import Path
from typing import List

from .adapters import TARGETS, get_adapter
from .canonical import render_canonical, write_all_canonicals, write_canonicals
from .console import print_canonical_outputs, print_paths
from .config import CharacterJob, load_jobs
from .targets.props.entities import write_entity_sprites
from .targets.icons.item_icons import write_item_icons
from .faction_lineup import write_faction_lineup
from .sheet import write_spritesheet
from .target_registry import (
    CATEGORIES,
    DiscoveryReport,
    TackonTarget,
    discover_tackon_targets,
)


def package_dir() -> Path:
    return Path(__file__).resolve().parent.parent


def repo_root() -> Path:
    # tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/cli.py -> repo root.
    return Path(__file__).resolve().parents[3]


# Defaults are computed against the package, not the cwd, so the CLI works
# regardless of where the user runs it from.
DEFAULT_CONFIG_DIR = (
    Path(__file__).resolve().parent / "configs"
)
DEFAULT_REVIEW_CONFIG_DIR = DEFAULT_CONFIG_DIR / "review"
DEFAULT_ASSET_DIR = (
    package_dir() / "generated"
)
DEFAULT_FACTION_CONFIG = DEFAULT_CONFIG_DIR / "factions" / "music_factions.yaml"


# ---- Tack-on targets ----------------------------------------------------------
#
# `targets/<category>/...` is auto-discovered at import time (see
# `target_registry.discover_tackon_targets`). Adding a new tack-on is
# just dropping a `.py` file (or a package directory with `__init__.py`)
# into the right category subdir and exposing a `render()` function —
# no edit to this file is required.

_TACKON_REPORT: DiscoveryReport = discover_tackon_targets()
_TACKON_TARGETS: dict[str, TackonTarget] = _TACKON_REPORT.targets


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


def _get_tackon(name: str) -> TackonTarget:
    """Look up a discovered tack-on target.

    If the name isn't in the registry but a matching file exists under
    ``targets/<category>/<name>.py``, surface the discovery warning for
    it (typically "no `render()` function") so the user knows *why*
    their file isn't registered, instead of just "unknown target."
    """
    if name in _TACKON_TARGETS:
        return _TACKON_TARGETS[name]
    # Look for a discovery warning matching this name. Warnings are
    # formatted as "<category>/<stem>: <reason>" so an `endswith` /
    # `:` split is enough to find the relevant one.
    for line in _TACKON_REPORT.warnings:
        head, _, reason = line.partition(":")
        if head.endswith(f"/{name}"):
            raise SystemExit(
                f"error: target {name!r} is not registered.\n"
                f"  reason: {reason.strip()}\n"
                f"  location: {head.strip()}.py\n"
                f"  see `target_registry.py` for the tack-on API contract."
            )
    raise SystemExit(
        f"error: unknown tack-on target: {name!r}\n"
        f"  run `list-targets` to see the registered tack-ons."
    )


def sandbox_sprites_dir() -> Path:
    return (
        repo_root() / "crates" / "ambition_sandbox" / "assets" / "sprites"
    )


def generated_dir(target_name: str) -> Path:
    return DEFAULT_ASSET_DIR / target_name


# ---- Adapter (character lab) commands -----------------------------------------

def draw_all(config_dir: str | Path = DEFAULT_CONFIG_DIR, out_dir: str | Path = DEFAULT_ASSET_DIR) -> List[Path]:
    out_dir = Path(out_dir)
    config_dir_path = Path(config_dir)
    runtime_stems = {"boss", "fascist_enforcer", "goblin", "ninja", "ninja_leader", "player_robot", "robot", "sandbag"}
    default_runtime_dir = config_dir_path.resolve() == Path(DEFAULT_CONFIG_DIR).resolve()
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
        outputs.extend(write_spritesheet(job, image_out, manifest_out))
    return outputs


def draw_review(
    config_dir: str | Path = DEFAULT_REVIEW_CONFIG_DIR,
    out_dir: str | Path = DEFAULT_ASSET_DIR / "review",
) -> List[Path]:
    outputs = draw_all(config_dir, out_dir)
    outputs += draw_canonicals(config_dir, Path(out_dir) / "canonicals")
    return outputs


def draw_canonicals(
    config_dir: str | Path = DEFAULT_CONFIG_DIR,
    out_dir: str | Path = DEFAULT_ASSET_DIR / "canonicals",
    *,
    adapters_only: bool = False,
    render_if_missing: bool = True,
) -> List[Path]:
    """Render every adapter + tack-on canonical pose into ``out_dir``.

    Tack-on canonicals come from each target's cached
    ``generated/<name>/<name>_canonical.png`` (rendered on demand if
    missing and ``render_if_missing`` is true).

    Set ``adapters_only=True`` for the legacy behavior that walks
    ``configs/*.yaml`` only.
    """
    if adapters_only:
        return write_canonicals(config_dir, out_dir)
    outputs, warnings = write_all_canonicals(
        out_dir,
        config_dir=config_dir,
        tackons=_TACKON_TARGETS.items(),
        generated_root=DEFAULT_ASSET_DIR,
        render_if_missing=render_if_missing,
    )
    for line in warnings:
        print(f"warning: {line}", file=sys.stderr)
    return outputs


def draw_character(config: str | Path, out_dir: str | Path = DEFAULT_ASSET_DIR) -> List[Path]:
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
    image_out, yaml_out = write_spritesheet(job, sheet_out, manifest_out)
    return [canonical_out, image_out, yaml_out]


def draw_entities(out_dir: str | Path = DEFAULT_ASSET_DIR / "entities") -> List[Path]:
    return write_entity_sprites(out_dir)


def draw_icons(out_dir: str | Path = DEFAULT_ASSET_DIR / "icons") -> List[Path]:
    return write_item_icons(out_dir)


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


def _cmd_draw_canonicals(args: argparse.Namespace) -> int:
    print_canonical_outputs(draw_canonicals(
        args.config_dir,
        args.out_dir,
        adapters_only=args.adapters_only,
        render_if_missing=not args.no_render,
    ))
    return 0


def _cmd_draw_character(args: argparse.Namespace) -> int:
    print_paths(draw_character(args.config, args.out_dir))
    return 0


def _cmd_draw_entities(args: argparse.Namespace) -> int:
    print_paths(draw_entities(args.out_dir))
    return 0


def _cmd_draw_icons(args: argparse.Namespace) -> int:
    print_paths(draw_icons(args.out_dir))
    return 0


def _cmd_draw_factions(args: argparse.Namespace) -> int:
    print_paths(draw_factions(args.config, args.out_dir))
    return 0


def _cmd_list_targets(args: argparse.Namespace) -> int:
    print("# adapter targets (driven by configs/*.yaml):")
    for target in sorted(TARGETS):
        adapter = get_adapter(target)
        print(f"  {target}: {', '.join(adapter.default_animations())}")
    print("# tack-on targets (render/install/render-publish):")
    by_category: dict[str, list[str]] = {cat: [] for cat in CATEGORIES}
    for name, tgt in _TACKON_TARGETS.items():
        by_category.setdefault(tgt.category, []).append(name)
    for category in CATEGORIES:
        names = sorted(by_category.get(category, []))
        if not names:
            continue
        print(f"  [{category}]")
        for name in names:
            print(f"    {name}")
    if _TACKON_REPORT.warnings:
        print("# warnings (files in targets/ that don't conform to the tack-on API):", file=sys.stderr)
        for line in _TACKON_REPORT.warnings:
            print(f"  {line}", file=sys.stderr)
    return 0


def _cmd_spritesheet(args: argparse.Namespace) -> int:
    job = CharacterJob.load(args.config)
    print_paths(write_spritesheet(job, args.output, args.manifest_out))
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


# ---- Tack-on commands ---------------------------------------------------------

def _render_tackon(target_name: str) -> List[Path]:
    target = _get_tackon(target_name)
    out_dir = generated_dir(target_name)
    paths = list(target.render(out_dir))
    print_paths(paths)
    return paths


def _install_tackon(target_name: str, dest_root: Path) -> List[Path]:
    target = _get_tackon(target_name)
    out_dir = generated_dir(target_name)
    # Targets that need to install into a subdirectory (e.g. the
    # mockingbird boss, which ships a `mockingbird_boss/` folder with
    # a manifest + per-part frames) expose a custom
    # `install(render_dir, dest_root)` function. Falling back to the
    # default SHEET_FILES copy when absent matches the historical
    # behavior.
    if target.install is not None:
        copied = list(target.install(out_dir, dest_root))
        print_paths(copied)
        return copied

    dest_root.mkdir(parents=True, exist_ok=True)
    copied = []
    missing: List[str] = []
    for fname in target.sheet_files:
        src = out_dir / fname
        if not src.exists():
            missing.append(fname)
            continue
        dst = dest_root / fname
        shutil.copy2(src, dst)
        copied.append(dst)
    if missing:
        print(
            f"warning: {target_name} files not yet rendered: {', '.join(missing)}",
            file=sys.stderr,
        )
    print_paths(copied)
    return copied


def _cmd_render(args: argparse.Namespace) -> int:
    _render_tackon(args.target)
    return 0


def _cmd_preview(args: argparse.Namespace) -> int:
    paths = _render_tackon(args.target)
    if paths:
        print_paths([paths[0]])
    else:
        print("\npreview written: <none>")
    return 0


def _cmd_install(args: argparse.Namespace) -> int:
    copied = _install_tackon(args.target, args.dest_root)
    return 0 if copied else 1


def _cmd_render_publish(args: argparse.Namespace) -> int:
    _render_tackon(args.target)
    copied = _install_tackon(args.target, args.dest_root)
    return 0 if copied else 1


def _cmd_render_publish_all(args: argparse.Namespace) -> int:
    """Render + install every tack-on target. Used to bring a fresh
    checkout up to date in one command after a renderer change touches
    multiple targets. Each target's errors are reported but don't abort
    the rest of the run."""
    failures: list[str] = []
    for target_name in sorted(_TACKON_TARGETS):
        print(f"\n# {target_name}")
        try:
            _render_tackon(target_name)
            if not _install_tackon(target_name, args.dest_root):
                failures.append(target_name)
        except Exception as ex:  # noqa: BLE001 - report and continue
            print(
                f"error: tack-on target {target_name!r} failed: {ex}",
                file=sys.stderr,
            )
            failures.append(target_name)
    if failures:
        print(
            f"\nrender-publish-all completed with {len(failures)} failure(s): "
            + ", ".join(failures),
            file=sys.stderr,
        )
        return 1
    return 0


def _cmd_regenerate_all(args: argparse.Namespace) -> int:
    """Single-button regen: render + install every sprite the sandbox
    runtime can consume.

    Composes three existing convenience commands so a fresh checkout
    only needs one invocation to be art-current:

    1. `draw-all --out-dir <sandbox assets>` — adapter-driven sheets
       (player_robot, robot, goblin, ninja, ninja_leader, sandbag,
       boss, fascist_enforcer).
    2. `render-publish-all` — every tack-on target discovered under
       `targets/`.
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

    print("\n# step 2/3: render-publish-all (tack-on targets)")
    publish_args = argparse.Namespace(dest_root=dest)
    rc = _cmd_render_publish_all(publish_args)
    if rc != 0:
        failures.append("render-publish-all")

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


def _add_tackon_target_arg(p: argparse.ArgumentParser) -> None:
    # No `choices=` constraint here. We want argparse to accept any
    # string so `_get_tackon` can give a useful error when the name
    # matches a file under `targets/<category>/` but the file is
    # missing the tack-on API. With `choices=` argparse would error
    # before our handler runs and we couldn't surface the warning.
    p.add_argument(
        "target",
        metavar="TARGET",
        help="tack-on target id (run `list-targets` to see what's registered)",
    )


def _add_tackon_install_args(p: argparse.ArgumentParser) -> None:
    p.add_argument(
        "target",
        metavar="TARGET",
        help="tack-on target id (run `list-targets` to see what's registered)",
    )
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

    # Adapter (character lab) commands.
    p = sub.add_parser("draw-all", help="Render all default sprite sheets")
    _add_config_dir_args(p, config_default=DEFAULT_CONFIG_DIR, out_default=DEFAULT_ASSET_DIR)
    p.set_defaults(func=_cmd_draw_all)

    p = sub.add_parser("draw-review", help="Render review/variant sprite sheets")
    _add_config_dir_args(p, config_default=DEFAULT_REVIEW_CONFIG_DIR, out_default=DEFAULT_ASSET_DIR / "review")
    p.set_defaults(func=_cmd_draw_review)

    p = sub.add_parser(
        "draw-canonicals",
        help=(
            "Render every adapter + tack-on canonical pose + a grid contact sheet. "
            "Tack-on canonicals are read from generated/<name>/ if cached, or "
            "rendered on demand. Pass --adapters-only for the legacy adapter-only "
            "behavior, or --no-render to skip rendering missing tack-on canonicals."
        ),
    )
    _add_config_dir_args(p, config_default=DEFAULT_CONFIG_DIR, out_default=DEFAULT_ASSET_DIR / "canonicals")
    p.add_argument(
        "--adapters-only",
        action="store_true",
        help="Render only adapter (YAML-driven) canonicals; skip tack-on targets",
    )
    p.add_argument(
        "--no-render",
        action="store_true",
        help="Skip the on-demand render fallback for tack-on canonicals missing from generated/",
    )
    p.set_defaults(func=_cmd_draw_canonicals)

    p = sub.add_parser("draw-character", help="Render one config's canonical image, spritesheet, and YAML")
    p.add_argument("config")
    p.add_argument("--out-dir", default=str(DEFAULT_ASSET_DIR))
    p.set_defaults(func=_cmd_draw_character)

    p = sub.add_parser("draw-entities", help="Render non-character gameplay entity sprites")
    p.add_argument("--out-dir", default=str(DEFAULT_ASSET_DIR / "entities"))
    p.set_defaults(func=_cmd_draw_entities)

    p = sub.add_parser("draw-icons", help="Render ability/item icon review sprites")
    p.add_argument("--out-dir", default=str(DEFAULT_ASSET_DIR / "icons"))
    p.set_defaults(func=_cmd_draw_icons)

    p = sub.add_parser("draw-factions", help="Render music-faction leader/NPC review sprites")
    p.add_argument("--config", default=str(DEFAULT_FACTION_CONFIG))
    p.add_argument("--out-dir", default=str(DEFAULT_ASSET_DIR / "factions"))
    p.set_defaults(func=_cmd_draw_factions)

    p = sub.add_parser("list-targets", help="Show registered targets (adapter + tack-on)")
    p.set_defaults(func=_cmd_list_targets)
    sub.add_parser("list", help="alias of list-targets").set_defaults(func=_cmd_list_targets)

    p = sub.add_parser("spritesheet", help="Render one job's sheet")
    p.add_argument("config")
    p.add_argument("output")
    p.add_argument("--manifest-out", default=None)
    p.set_defaults(func=_cmd_spritesheet)

    p = sub.add_parser("single", help="Render one frame from a job")
    p.add_argument("config")
    p.add_argument("output")
    p.add_argument("--animation", default="idle")
    p.add_argument("--frame-index", type=int, default=0)
    p.set_defaults(func=_cmd_single)

    # Tack-on commands.
    p = sub.add_parser("render", help="Render a tack-on target into generated/")
    _add_tackon_target_arg(p)
    p.set_defaults(func=_cmd_render)

    p = sub.add_parser("preview", help="Render a tack-on target and report paths")
    _add_tackon_target_arg(p)
    p.set_defaults(func=_cmd_preview)

    p = sub.add_parser("install", help="Copy a tack-on target's files into sandbox assets")
    _add_tackon_install_args(p)
    p.set_defaults(func=_cmd_install)

    p = sub.add_parser("render-publish", help="Render then install a tack-on target")
    _add_tackon_target_arg(p)
    p.add_argument(
        "--dest-root",
        type=Path,
        default=sandbox_sprites_dir(),
    )
    p.set_defaults(func=_cmd_render_publish)

    p = sub.add_parser(
        "render-publish-all",
        help="Render + install every discovered tack-on target in one shot",
    )
    p.add_argument(
        "--dest-root",
        type=Path,
        default=sandbox_sprites_dir(),
        help="install destination (default: crates/ambition_sandbox/assets/sprites)",
    )
    p.set_defaults(func=_cmd_render_publish_all)

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
            "One-shot: draw-all + render-publish-all + draw-runtime-npcs, "
            "all installed into sandbox assets. Brings a fresh checkout's "
            "sprite directory up to date in one command."
        ),
    )
    p.add_argument(
        "--dest-root",
        type=Path,
        default=sandbox_sprites_dir(),
        help="install destination (default: crates/ambition_sandbox/assets/sprites)",
    )
    p.set_defaults(func=_cmd_regenerate_all)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args) or 0)


if __name__ == "__main__":
    raise SystemExit(main())
