"""Modal CLI for ambition_music_renderer.

Subcommands:

    render <cue>            Render a single cue YAML to local generated/<cue>/.
    publish <cue>           Publish newest preview into the sandbox asset tree.
    render-publish <cue>    Render then publish.
    sandbox render-publish  Render+publish the sandbox single-track cues
                            (lofi_study_loop, long_lofi_drift, pulse_drift_voyage).
    sandbox render          Render-only for sandbox cues.
    sandbox publish         Publish-only for sandbox cues (--skip-render alias).
    radio render-publish    Render+publish every cue exposed on the in-game
                            radio: SANDBOX_CUES plus auto-discovered
                            scores/active/* plus EXTRA_RADIO_CUES.
    radio render            Render-only for radio cues.
    radio publish           Publish-only for radio cues.

Pinning a specific render: drop a file named ``published.ogg`` (or a symlink)
into ``output/<cue>/preview/`` (or ``generated/<cue>/preview/``) and publish
will copy that exact file instead of the auto-named full mix. Used when a
cue's mastered preview lives under a manual filename.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

# Soft import: ubelt.ProgIter gives the bulk loop a count + ETA. If it's not
# installed in the active venv (older setup), fall back to a plain iterator
# so the renderer still works.
try:
    import ubelt as _ub  # type: ignore[import-not-found]

    def _progress(iterable, *, total, desc):
        return _ub.ProgIter(
            iterable,
            total=total,
            desc=desc,
            verbose=3,
            freq=1,
            adjust=False,
        )
except ImportError:  # pragma: no cover — graceful fallback

    def _progress(iterable, *, total, desc):  # noqa: ARG001
        return iterable


# Single-track cues authored under scores/active that ship via the radio.
# These render with --simple-mix and publish the mastered preview.
SANDBOX_CUES = ("lofi_study_loop", "long_lofi_drift", "pulse_drift_voyage")

# Adaptive cues handled by the dedicated pipeline
# (scripts/regen_first_goblin_tune_v2.sh).
# This module deliberately skips them in the bulk `radio` pass; their
# multi-stem layout is owned elsewhere.
ADAPTIVE_CUES = ("first_goblin_tune_v2",)

# Curated extras drawn from scores/examples that we expose on the radio.
# We keep this explicit (rather than scanning examples wholesale) because
# the examples tree also holds debug / fixture / archive scores that should
# not auto-publish. Add new entries here as content lands in examples/.
EXTRA_RADIO_CUES = (
    "crooked_ascent_boss",
    "dinosaur_liberators",
    "dinosaur_liberators_long",
    "env_advocacy_solace",
    "fast_paced_violin_boss",
    "glasswood",
    "military_iron_resolve",
    "moonlit_canal",
    "solo_soar",
    "solo_soar_9m08_loud",
    "tech_bros_disruption",
    "violin_boss_relentless",
)

SCORE_DIRS = ("active", "examples", "archive")

# Filename treated as a manual override inside any preview/ directory. When
# present, this is the file that gets copied to assets/.../<cue>/full.ogg
# instead of the renderer's auto-named full_soundtrack_preview.ogg.
PINNED_FILENAME = "published.ogg"


def package_dir() -> Path:
    return Path(__file__).resolve().parent.parent


def repo_root() -> Path:
    # tools/ambition_music_renderer/ambition_music_renderer/cli.py -> repo
    return Path(__file__).resolve().parents[3]


def generated_root() -> Path:
    return package_dir() / "generated"


def output_root() -> Path:
    """Legacy hashed output root used by the underlying renderer."""
    return package_dir() / "output"


def find_score(cue: str) -> Path | None:
    """Locate a cue YAML by name. Searches scores/{active,examples,archive}/.

    Accepts a bare cue id (e.g. ``lofi_study_loop``) or a relative/absolute
    path to a YAML.
    """
    p = Path(cue)
    if p.suffix in (".yaml", ".yml") and p.exists():
        return p.resolve()
    candidates = [
        package_dir() / "scores" / sub / f"{cue}.music.yaml" for sub in SCORE_DIRS
    ]
    candidates += [package_dir() / "scores" / sub / f"{cue}.yaml" for sub in SCORE_DIRS]
    for c in candidates:
        if c.exists():
            return c
    return None


def find_full_mix(preview_dir: Path, cue: str) -> Path | None:
    """Locate the OGG to publish for ``cue``.

    Order of preference:
      1. ``preview/published.ogg`` — manual pin (file or symlink). Lets a
         human elect a specific render (e.g. a renamed favorite) without
         renaming it back to the auto pattern.
      2. The most-recent ``{cue}_*.full_soundtrack_preview.ogg`` — the
         renderer's standard mastered preview output.
    Returns ``None`` if neither is present.
    """
    pinned = preview_dir / PINNED_FILENAME
    if pinned.exists():
        return pinned
    candidates = sorted(
        preview_dir.glob(f"{cue}_*.full_soundtrack_preview.ogg"),
        key=lambda p: p.stat().st_mtime,
        reverse=True,
    )
    return candidates[0] if candidates else None


def discover_active_radio_cues() -> tuple[str, ...]:
    """List cues from scores/active/ that should appear on the radio.

    Excludes:
      - Cues already in ``SANDBOX_CUES`` (handled by that path).
      - Cues in ``ADAPTIVE_CUES`` (handled by
        ``scripts/regen_first_goblin_tune_v2.sh``).
    Returns a sorted, deduped tuple so the order is stable across runs.
    """
    active = package_dir() / "scores" / "active"
    if not active.is_dir():
        return ()
    cues: set[str] = set()
    for path in active.iterdir():
        name = path.name
        for suffix in (".music.yaml", ".yaml"):
            if name.endswith(suffix):
                cue = name[: -len(suffix)]
                if cue and cue not in SANDBOX_CUES and cue not in ADAPTIVE_CUES:
                    cues.add(cue)
                break
    return tuple(sorted(cues))


def radio_cues() -> tuple[str, ...]:
    """All cues we expect to publish into the in-game radio asset tree."""
    seen: set[str] = set()
    ordered: list[str] = []
    for cue in (*SANDBOX_CUES, *discover_active_radio_cues(), *EXTRA_RADIO_CUES):
        if cue not in seen:
            seen.add(cue)
            ordered.append(cue)
    return tuple(ordered)


def needs_render(cue: str, yaml_path: Path, outdir: Path) -> bool:
    preview_dir = outdir / "preview"
    latest = find_full_mix(preview_dir, cue)
    if latest is None:
        return True
    return yaml_path.stat().st_mtime > latest.stat().st_mtime


def python_exe() -> str:
    """Prefer the package venv if it exists, else current interpreter."""
    venv_python = package_dir() / ".venv" / "bin" / "python"
    if venv_python.exists():
        return str(venv_python)
    return sys.executable


def render_cue(
    cue: str,
    yaml_path: Path,
    outdir: Path,
    *,
    backend: str = "pretty-midi",
    simple_mix: bool = True,
    extra_args: list[str] | None = None,
) -> bool:
    cmd = [
        python_exe(),
        "-m",
        "ambition_music_renderer.render_isolated",
        str(yaml_path),
        "--outdir",
        str(outdir),
        "--backend",
        backend,
    ]
    if simple_mix:
        cmd.append("--simple-mix")
    if extra_args:
        cmd.extend(extra_args)
    print(f"render {cue}: {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=package_dir())
    return result.returncode == 0


def default_publish_dest_root() -> Path:
    return (
        repo_root()
        / "crates"
        / "ambition_sandbox"
        / "assets"
        / "audio"
        / "music"
        / "generated"
    )


def publish_cue(cue: str, outdir: Path, dest_root: Path) -> bool:
    preview_dir = outdir / "preview"
    src = find_full_mix(preview_dir, cue)
    if src is None:
        print(
            f"skip publish {cue}: no full_soundtrack_preview.ogg in {preview_dir}",
            file=sys.stderr,
        )
        return False
    dest_dir = dest_root / cue
    dest_dir.mkdir(parents=True, exist_ok=True)
    dest = dest_dir / "full.ogg"
    shutil.copy2(src, dest)
    try:
        src_rel = src.relative_to(repo_root())
    except ValueError:
        src_rel = src
    try:
        dest_rel = dest.relative_to(repo_root())
    except ValueError:
        dest_rel = dest
    print(f"publish {cue}: {src_rel} -> {dest_rel}")
    return True


def cmd_render(args: argparse.Namespace) -> int:
    yaml_path = find_score(args.cue)
    if yaml_path is None:
        print(f"error: cue not found: {args.cue}", file=sys.stderr)
        return 2
    outdir = generated_root() / args.cue
    if not args.simple_mix and outdir == generated_root() / args.cue:
        # nothing special; just leaving the simple-mix off
        pass
    ok = render_cue(
        args.cue,
        yaml_path,
        outdir,
        backend=args.backend,
        simple_mix=args.simple_mix,
    )
    return 0 if ok else 1


def cmd_publish(args: argparse.Namespace) -> int:
    outdir = generated_root() / args.cue
    # Fallback to legacy output/ tree if generated/ is empty.
    if not (outdir / "preview").exists():
        legacy = output_root() / args.cue
        if (legacy / "preview").exists():
            outdir = legacy
    ok = publish_cue(args.cue, outdir, args.dest_root)
    return 0 if ok else 1


def cmd_render_publish(args: argparse.Namespace) -> int:
    yaml_path = find_score(args.cue)
    if yaml_path is None:
        print(f"error: cue not found: {args.cue}", file=sys.stderr)
        return 2
    outdir = generated_root() / args.cue
    if args.force_render or needs_render(args.cue, yaml_path, outdir):
        if not render_cue(
            args.cue,
            yaml_path,
            outdir,
            backend=args.backend,
            simple_mix=args.simple_mix,
        ):
            return 1
    else:
        print(f"skip render {args.cue}: YAML unchanged since last render")
    return 0 if publish_cue(args.cue, outdir, args.dest_root) else 1


def _process_simple_mix_cue(
    cue: str,
    *,
    action: str,
    backend: str,
    force_render: bool,
    dest_root: Path,
) -> str | None:
    """Run render/publish/render-publish for one simple-mix cue.

    Returns ``None`` on success, otherwise a short failure-stage label
    (``"resolve"`` / ``"render"`` / ``"publish"``) for the caller to
    aggregate. ``action`` is one of ``"render"``, ``"publish"``,
    ``"render-publish"``.
    """
    yaml_path = find_score(cue)
    if yaml_path is None:
        # Some cues only exist as legacy renders under output/ with no
        # active YAML (e.g. archived examples). Permit publish-only in
        # that case so existing previews still ship.
        if action == "publish":
            outdir = output_root() / cue
            if (outdir / "preview").exists():
                return None if publish_cue(cue, outdir, dest_root) else "publish"
        print(f"skip {cue}: missing YAML", file=sys.stderr)
        return "resolve"
    outdir = generated_root() / cue
    if action in ("render", "render-publish"):
        if force_render or needs_render(cue, yaml_path, outdir):
            if not render_cue(
                cue,
                yaml_path,
                outdir,
                backend=backend,
                simple_mix=True,
            ):
                return "render"
        else:
            print(f"skip render {cue}: YAML unchanged since last render")
    if action in ("publish", "render-publish"):
        if not publish_cue(cue, outdir, dest_root):
            # Fall back to the legacy output/ tree (older renders or
            # adaptive cues whose mastered preview lives there).
            legacy = output_root() / cue
            if (legacy / "preview").exists():
                if not publish_cue(cue, legacy, dest_root):
                    return "publish"
            else:
                return "publish"
    return None


def _run_bulk(args: argparse.Namespace, cues: tuple[str, ...]) -> int:
    failed: list[str] = []
    desc = f"music {args.action}"
    for cue in _progress(cues, total=len(cues), desc=desc):
        stage = _process_simple_mix_cue(
            cue,
            action=args.action,
            backend=args.backend,
            force_render=args.force_render,
            dest_root=args.dest_root,
        )
        if stage is not None:
            failed.append(f"{stage} {cue}")
    if failed:
        print(f"FAILED: {', '.join(failed)}", file=sys.stderr)
        return 1
    print(f"OK: {len(cues)} cue(s) ready")
    return 0


def cmd_sandbox(args: argparse.Namespace) -> int:
    """Render+publish the sandbox single-track cues.

    Mirrors the legacy ``tools/audio/render_sandbox_music.py`` behavior:
    skip the renderer when the YAML mtime is older than the latest preview,
    use --simple-mix for these single-track cues, publish the newest
    full_soundtrack_preview.ogg into the bevy asset tree.
    """
    cues = tuple(args.cue) if args.cue else SANDBOX_CUES
    return _run_bulk(args, cues)


def cmd_radio(args: argparse.Namespace) -> int:
    """Render+publish every cue we expose on the in-game radio.

    Covers ``SANDBOX_CUES`` plus auto-discovered ``scores/active/*`` cues
    plus the curated ``EXTRA_RADIO_CUES`` list. Skips ``ADAPTIVE_CUES``
    (those go through ``scripts/regen_first_goblin_tune_v2.sh``). Honors
    ``preview/published.ogg`` pins for cues whose mastered file lives
    under a manual filename.
    """
    cues = tuple(args.cue) if args.cue else radio_cues()
    return _run_bulk(args, cues)


def add_render_args(p: argparse.ArgumentParser) -> None:
    p.add_argument(
        "--backend",
        default="pretty-midi",
        help="renderer backend (pretty-midi / fluidsynth-cli / fallback / auto)",
    )
    p.add_argument(
        "--simple-mix",
        dest="simple_mix",
        action="store_true",
        default=True,
        help="emit only the mastered preview (default for sandbox cues)",
    )
    p.add_argument(
        "--no-simple-mix",
        dest="simple_mix",
        action="store_false",
        help="emit the full adaptive stem set (per-section per-group OGGs)",
    )


def add_publish_args(p: argparse.ArgumentParser) -> None:
    p.add_argument(
        "--dest-root",
        type=Path,
        default=default_publish_dest_root(),
        help="install destination root (default: bevy asset tree)",
    )


def build_parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(
        prog="ambition_music_renderer",
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    sub = ap.add_subparsers(dest="command", required=True)

    p_render = sub.add_parser("render", help="Render a single cue YAML")
    p_render.add_argument("cue", help="cue id (e.g. lofi_study_loop) or YAML path")
    add_render_args(p_render)
    p_render.set_defaults(func=cmd_render)

    p_publish = sub.add_parser(
        "publish", help="Publish newest preview to sandbox assets"
    )
    p_publish.add_argument("cue")
    add_publish_args(p_publish)
    p_publish.set_defaults(func=cmd_publish)

    p_rp = sub.add_parser("render-publish", help="Render then publish a single cue")
    p_rp.add_argument("cue")
    add_render_args(p_rp)
    add_publish_args(p_rp)
    p_rp.add_argument("--force-render", action="store_true")
    p_rp.set_defaults(func=cmd_render_publish)

    p_sb = sub.add_parser(
        "sandbox",
        help="Sandbox-cue presets (lofi_study_loop, long_lofi_drift, pulse_drift_voyage)",
    )
    sb_sub = p_sb.add_subparsers(dest="action", required=True)
    for action in ("render", "publish", "render-publish"):
        sp = sb_sub.add_parser(action)
        sp.add_argument(
            "--cue",
            action="append",
            choices=SANDBOX_CUES,
            help="restrict to the named sandbox cue(s); repeat to select multiple",
        )
        sp.add_argument("--backend", default="pretty-midi")
        sp.add_argument("--force-render", action="store_true")
        # publish-only convenience: the user typing `publish` already implies skip-render.
        sp.add_argument(
            "--skip-render",
            action="store_true",
            help="alias: ignored when action is publish; treats render-publish as publish",
        )
        add_publish_args(sp)
        sp.set_defaults(func=cmd_sandbox)
    p_sb.set_defaults(func=cmd_sandbox)

    p_radio = sub.add_parser(
        "radio",
        help="All radio cues: SANDBOX_CUES + scores/active/* + EXTRA_RADIO_CUES",
    )
    radio_choices = radio_cues()
    radio_sub = p_radio.add_subparsers(dest="action", required=True)
    for action in ("render", "publish", "render-publish"):
        sp = radio_sub.add_parser(action)
        sp.add_argument(
            "--cue",
            action="append",
            choices=radio_choices,
            help="restrict to the named radio cue(s); repeat to select multiple",
        )
        sp.add_argument("--backend", default="pretty-midi")
        sp.add_argument("--force-render", action="store_true")
        sp.add_argument(
            "--skip-render",
            action="store_true",
            help="alias: ignored when action is publish; treats render-publish as publish",
        )
        add_publish_args(sp)
        sp.set_defaults(func=cmd_radio)
    p_radio.set_defaults(func=cmd_radio)

    return ap


def main(argv: list[str] | None = None) -> int:
    ap = build_parser()
    args = ap.parse_args(argv)
    if args.command in ("sandbox", "radio"):
        # Map --skip-render onto action.
        if getattr(args, "skip_render", False) and args.action == "render-publish":
            args.action = "publish"
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
