"""Unified discovery for every sprite target.

One concept — `Target` — covers every renderable thing in the package:

- **Tack-on targets** authored in Python under ``targets/<category>/``.
  Either a single ``.py`` file or a package directory; either form is
  auto-registered if it exposes a module-level ``render(out_dir, **opts)``
  function. The procedural-Python authoring path.
- **Adapter targets** defined by a YAML config in ``configs/*.yaml``
  that's consumed by one of the rigs in ``adapters.py``. The
  YAML-driven authoring path.
- **Review NPCs** — YAML configs under ``configs/review/*.yaml``,
  same machinery as adapter targets but a separate category since
  they're review-only (the sandbox runtime loads only the curated
  subset listed in CLI's ``RUNTIME_REVIEW_NPCS``).

`Target` is a ``Protocol`` — anything with ``name`` / ``category`` /
``sheet_files`` plus ``render_canonical`` / ``render_sheet`` / ``install``
methods qualifies. Two concrete implementations live here:

- [`TackonTarget`] — wraps a tack-on module's callables.
- [`AdapterTarget`] — wraps a YAML config + the adapter pipeline.

The registry's job is just to walk every surface and yield Target
instances. Consumers (CLI, gallery, render-publish) iterate the
returned dict without caring which surface a target came from.
"""

from __future__ import annotations

import importlib
import shutil
from pathlib import Path
from typing import (
    Any,
    Callable,
    Dict,
    Iterable,
    Iterator,
    List,
    NamedTuple,
    Mapping,
    Optional,
    Protocol,
    Sequence,
    Tuple,
    runtime_checkable,
)

from ..authoring.actor_profiles import merge_actor_metadata

CATEGORIES: Tuple[str, ...] = (
    # Tack-on categories — Python authoring under `targets/<category>/`.
    "characters",
    "props",
    "tiles",
    "icons",
    "projectiles",
)
# Note: there used to be a separate `review_npcs` category for YAML-
# config-driven characters that lived in `configs/review/*.yaml`. As
# of the Phase 6 character-catalog cleanup (2026-05-24) those targets
# merge into `characters` — the split was an internal authoring
# detail that didn't map to runtime behavior (many "review NPCs"
# ship to the runtime via the same sprite registry as everyone else).
# The `configs/review/` directory still exists for backwards-
# compat authoring, but its contents now register under `characters`.

# Modules under `targets/characters/` that are imported by
# `adapters.py` and driven by YAML configs instead of a `render()`
# function. Discovery silently skips these so they don't show up as
# warnings under `list-targets`.
ADAPTER_HELPER_STEMS: frozenset[str] = frozenset(
    {
        "alice_cryptographer",
        "bob_engineer",
        "boss_side",
        "goblin_side",
        "ninja_side",
        "robot25d",
        "robot_side",
        "toon_side",
        "trent_elder",
    }
)


# ---- Shared install helpers --------------------------------------------------


def _copy_sheet_files(
    sheet_files: "Sequence[str]",
    render_dir: Path,
    dest_root: Path,
) -> List[Path]:
    """Copy every listed sheet file plus optional generated sidecars.

    Runtime-compatible sheets have long shipped ``*_spritesheet.ron`` next to
    their YAML manifests. The renderer now also emits optional
    ``*_actor.ron`` contracts; copy them opportunistically so old targets and
    old manifests keep working while new metadata can ride along.
    """
    copied: List[Path] = []
    copied_names: set[str] = set()
    listed = set(sheet_files)

    def copy_if_exists(fname: str) -> None:
        if fname in copied_names:
            return
        src = render_dir / fname
        if not src.exists():
            return
        dst = dest_root / fname
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dst)
        copied.append(dst)
        copied_names.add(fname)

    def companions_for(fname: str) -> List[str]:
        out: List[str] = []
        if fname.endswith(".yaml"):
            out.append(fname[:-5] + ".ron")
        stem = None
        for suffix in ("_spritesheet.yaml", "_spritesheet.ron", "_spritesheet.png"):
            if fname.endswith(suffix):
                stem = fname[: -len(suffix)]
                break
        if stem:
            out.append(f"{stem}_actor.ron")
        return out

    for fname in sheet_files:
        copy_if_exists(fname)
        for companion in companions_for(fname):
            if companion not in listed:
                copy_if_exists(companion)
    return copied


# ---- Target protocol ---------------------------------------------------------


@runtime_checkable
class Target(Protocol):
    """Any sprite target the registry can render + install.

    Implemented by [`TackonTarget`] (Python authoring) and
    [`AdapterTarget`] (YAML authoring). New target types just need to
    match this shape.
    """

    name: str
    """Registry key — CLI ``choices=`` value, ``generated/<name>/`` subdir."""

    category: str
    """One of [`CATEGORIES`]. Drives section grouping in gallery + list-targets."""

    sheet_files: Tuple[str, ...]
    """Files the default installer copies into the sandbox sprites dir."""

    def render_canonical(self, out_dir: Path, **opts) -> Path:
        """Draw the canonical pose into ``out_dir``, return the saved path."""
        ...

    def render_sheet(self, out_dir: Path, **opts) -> List[Path]:
        """Draw the full sprite sheet bundle into ``out_dir``, return paths."""
        ...

    def install(self, render_dir: Path, dest_root: Path) -> List[Path]:
        """Copy the rendered sheet from ``render_dir`` to ``dest_root``."""
        ...


# ---- Optional actor-contract sidecar hook -----------------------------------


def _ensure_actor_sidecars(
    *,
    target_name: str,
    render_dir: Path,
    paths: Sequence[Path],
    actor_metadata: Mapping[str, Any] | None = None,
) -> List[Path]:
    """Ensure every rendered ``*_spritesheet.yaml`` has ``*_actor.ron``.

    Most modern targets already emit the sidecar from ``tackon_sheet``. This
    post-render hook covers older/custom tack-ons and lets module-level
    ``ACTOR_METADATA`` enrich the inferred contract without forcing every
    bespoke renderer through the generic sheet builder immediately. It also
    handles multi-file boss targets that emit ``*_spritesheet_manifest.json``
    instead of the standard YAML/RON sheet manifest.
    """
    from ..authoring.actor_contract import write_actor_contract_for_tackon
    import json
    import yaml

    extras: List[Path] = []

    def maybe_emit(
        *,
        manifest_path: Path,
        manifest: Mapping[str, Any],
        image_name: str,
        sheet_manifest_name: str,
    ) -> None:
        actor_path = manifest_path.with_name(f"{target_name}_actor.ron")
        if actor_path.exists() and not actor_metadata:
            return
        write_actor_contract_for_tackon(
            target=target_name,
            image_out=manifest_path.with_name(image_name),
            sheet_ron_out=manifest_path.with_name(sheet_manifest_name),
            manifest=manifest,
            actor_metadata=actor_metadata or {},
        )
        if actor_path.exists() and actor_path not in extras:
            extras.append(actor_path)

    for path in list(paths):
        path = Path(path)
        if path.suffix == ".yaml" and path.name.endswith("_spritesheet.yaml"):
            try:
                manifest = yaml.safe_load(path.read_text(encoding="utf8")) or {}
            except Exception:
                continue
            if not isinstance(manifest, dict):
                continue
            image_name = str(
                manifest.get("image") or path.name.replace(".yaml", ".png")
            )
            maybe_emit(
                manifest_path=path,
                manifest=manifest,
                image_name=image_name,
                sheet_manifest_name=path.with_suffix(".ron").name,
            )
        elif path.suffix == ".json" and path.name.endswith(
            "_spritesheet_manifest.json"
        ):
            try:
                manifest = json.loads(path.read_text(encoding="utf8")) or {}
            except Exception:
                continue
            if not isinstance(manifest, dict):
                continue
            image_name = str(manifest.get("image") or f"{target_name}_spritesheet.png")
            maybe_emit(
                manifest_path=path,
                manifest=manifest,
                image_name=image_name,
                sheet_manifest_name=path.name,
            )
    return extras


# ---- TackonTarget ------------------------------------------------------------


class TackonTarget:
    """A Python-authored target wrapping a module's callables."""

    def __init__(
        self,
        *,
        name: str,
        category: str,
        module_path: str,
        render: Callable,
        sheet_files: Tuple[str, ...],
        install: Optional[Callable] = None,
        render_canonical: Optional[Callable] = None,
        actor_metadata: Mapping[str, Any] | None = None,
    ) -> None:
        self.name = name
        self.category = category
        self.module_path = module_path
        self.sheet_files = sheet_files
        self._render_sheet_fn = render
        self._install_fn = install
        self._render_canonical_fn = render_canonical
        self._actor_metadata = dict(actor_metadata or {})

    def render_sheet(self, out_dir: Path, **opts) -> List[Path]:
        out_dir = Path(out_dir)
        out_dir.mkdir(parents=True, exist_ok=True)
        paths = list(self._render_sheet_fn(out_dir, **opts))
        paths.extend(
            _ensure_actor_sidecars(
                target_name=self.name,
                render_dir=out_dir,
                paths=paths,
                actor_metadata=self._actor_metadata,
            )
        )
        return paths

    def render_canonical(self, out_dir: Path, **opts) -> Path:
        """Draw just the canonical pose into ``out_dir``.

        Fast path: if the target exposes a ``render_canonical`` hook,
        invoke it. Otherwise fall back to running the full
        ``render_sheet()`` and locating the
        ``{name}_canonical_transparent.png`` it emits as a side
        effect. The slow fallback is correct but ~16× slower; targets
        built on ``tackon_sheet.build_sheet`` should expose a hook
        (3 lines wrapping ``tackon_sheet.write_canonical``).
        """
        out_dir = Path(out_dir)
        out_dir.mkdir(parents=True, exist_ok=True)
        if self._render_canonical_fn is not None:
            return Path(self._render_canonical_fn(out_dir, **opts))
        # Slow fallback.
        self._render_sheet_fn(out_dir, **opts)
        candidate = out_dir / f"{self.name}_canonical_transparent.png"
        if candidate.exists():
            return candidate
        raise FileNotFoundError(
            f"{self.category}/{self.name}: full render completed but "
            f"{candidate.name} is missing — target's render() may not go "
            f"through `tackon_sheet.build_sheet`. Add a `render_canonical` "
            f"hook (see e.g. galwah.py) to fix."
        )

    def install(self, render_dir: Path, dest_root: Path) -> List[Path]:
        """Default installer copies each path in ``sheet_files``.

        Targets that need a custom install (e.g. mockingbird_boss
        which ships a subdirectory of part files) override this by
        exposing a module-level ``install`` function.
        """
        render_dir = Path(render_dir)
        dest_root = Path(dest_root)
        if self._install_fn is not None:
            return list(self._install_fn(render_dir, dest_root))
        dest_root.mkdir(parents=True, exist_ok=True)
        return _copy_sheet_files(self.sheet_files, render_dir, dest_root)


# ---- AdapterTarget -----------------------------------------------------------


class AdapterTarget:
    """A YAML-authored target wrapping a config + the adapter pipeline.

    Implements the same [`Target`] protocol as [`TackonTarget`] so the
    CLI / gallery / install paths don't need to branch by surface.
    """

    def __init__(self, *, config_path: Path, category: str) -> None:
        # Local import — the adapter pipeline pulls in Pillow; we keep
        # registry/discovery.py importable without it by deferring.
        from .config import CharacterJob

        self._config_path = Path(config_path)
        self._job = CharacterJob.load(self._config_path)
        # The output stem is what shows up in the sheet filenames.
        self.name = self._job.output_stem(self._config_path)
        self.category = category
        self.sheet_files = (
            f"{self.name}_spritesheet.png",
            f"{self.name}_spritesheet.yaml",
            f"{self.name}_spritesheet.ron",
            f"{self.name}_actor.ron",
        )

    def render_canonical(self, out_dir: Path, **opts) -> Path:
        from ..authoring.adapters import get_adapter

        del opts  # adapter pipeline ignores tack-on **opts
        out_dir = Path(out_dir)
        out_dir.mkdir(parents=True, exist_ok=True)
        adapter = get_adapter(self._job.target)
        spec = adapter.sample_spec(self._job)
        img = adapter.render_canonical(spec, self._job)
        if img.mode != "RGBA":
            img = img.convert("RGBA")
        out = out_dir / f"{self.name}_canonical_transparent.png"
        img.save(out)
        return out

    def render_sheet(self, out_dir: Path, **opts) -> List[Path]:
        from ..authoring.sheet import write_spritesheet

        del opts
        out_dir = Path(out_dir)
        out_dir.mkdir(parents=True, exist_ok=True)
        image_out = out_dir / f"{self.name}_spritesheet.png"
        manifest_out = out_dir / f"{self.name}_spritesheet.yaml"
        paths = list(
            write_spritesheet(
                self._job, image_out, manifest_out, source_config=self._config_path
            )
        )
        actor_out = out_dir / f"{self.name}_actor.ron"
        if actor_out.exists():
            paths.append(actor_out)
        return paths

    def install(self, render_dir: Path, dest_root: Path) -> List[Path]:
        """Default copy of `sheet_files`; same default as TackonTarget."""
        render_dir = Path(render_dir)
        dest_root = Path(dest_root)
        dest_root.mkdir(parents=True, exist_ok=True)
        return _copy_sheet_files(self.sheet_files, render_dir, dest_root)


# ---- Discovery ---------------------------------------------------------------


class DiscoveryReport(NamedTuple):
    """Outcome of one discovery pass."""

    targets: Dict[str, Target]
    warnings: List[str]


def _targets_dir() -> Path:
    return Path(__file__).resolve().parent.parent / "targets"


def _configs_dir() -> Path:
    return Path(__file__).resolve().parent.parent / "configs"


def _walk_category(category: str) -> Iterator[Tuple[str, str]]:
    """Yield ``(stem, dotted_module_path)`` for each candidate under
    ``targets/<category>/`` — both single-file modules and packages."""
    cat_dir = _targets_dir() / category
    if not cat_dir.is_dir():
        return
    for path in sorted(cat_dir.iterdir()):
        name = path.name
        if name.startswith("_"):
            continue  # __init__, __main__, private helpers
        if path.is_file() and path.suffix == ".py":
            stem = path.stem
            yield stem, f"ambition_sprite2d_renderer.targets.{category}.{stem}"
        elif path.is_dir() and (path / "__init__.py").exists():
            yield name, f"ambition_sprite2d_renderer.targets.{category}.{name}"


def _with_actor_sidecar(stem: str, sheet_files: Sequence[str]) -> Tuple[str, ...]:
    """Return install file list with the optional actor sidecar included."""
    out = list(sheet_files)
    actor = f"{stem}_actor.ron"
    if actor not in out:
        out.append(actor)
    return tuple(out)


def default_sheet_files(stem: str) -> List[str]:
    """Default install set for tack-on targets that don't declare ``SHEET_FILES``."""
    return [
        f"{stem}_spritesheet.png",
        f"{stem}_spritesheet.yaml",
        f"{stem}_spritesheet.ron",
        f"{stem}_actor.ron",
    ]


def _build_tackon_single(
    mod,
    stem: str,
    category: str,
    dotted: str,
    render: Callable,
) -> TackonTarget:
    sheet_files = _with_actor_sidecar(
        stem, getattr(mod, "SHEET_FILES", default_sheet_files(stem))
    )
    install_fn = getattr(mod, "install", None)
    if not callable(install_fn):
        install_fn = None
    render_canonical_fn = getattr(mod, "render_canonical", None)
    if not callable(render_canonical_fn):
        render_canonical_fn = None
    return TackonTarget(
        name=stem,
        category=category,
        module_path=dotted,
        render=render,
        sheet_files=sheet_files,
        install=install_fn,
        render_canonical=render_canonical_fn,
        actor_metadata=getattr(mod, "ACTOR_METADATA", None),
    )


def _build_tackon_multi(
    mod,
    stem: str,
    category: str,
    dotted: str,
    warnings: List[str],
) -> List[TackonTarget]:
    """A module exposing ``TARGETS = {name: {...}}`` registers many."""
    results: List[TackonTarget] = []
    for sub_name, spec in mod.TARGETS.items():
        render = spec.get("render")
        if not callable(render):
            warnings.append(
                f"{category}/{stem}: TARGETS[{sub_name!r}] missing `render`; skipped"
            )
            continue
        sheet_files = _with_actor_sidecar(
            sub_name, spec.get("sheet_files", default_sheet_files(sub_name))
        )
        install_fn = spec.get("install")
        if install_fn is not None and not callable(install_fn):
            install_fn = None
        render_canonical_fn = spec.get("render_canonical")
        if render_canonical_fn is not None and not callable(render_canonical_fn):
            render_canonical_fn = None
        results.append(
            TackonTarget(
                name=sub_name,
                category=category,
                module_path=dotted,
                render=render,
                sheet_files=sheet_files,
                install=install_fn,
                render_canonical=render_canonical_fn,
                actor_metadata=merge_actor_metadata(
                    getattr(mod, "ACTOR_METADATA", None), spec.get("actor_metadata")
                ),
            )
        )
    return results


def discover_tackon_targets() -> DiscoveryReport:
    """Walk ``targets/<category>/`` and register every conformant module.

    Tack-on targets only; for the unified surface that also covers
    YAML adapter configs, see [`discover_all_targets`].
    """
    targets: Dict[str, Target] = {}
    warnings: List[str] = []
    tackon_categories = ("characters", "props", "tiles", "icons", "projectiles")
    for category in tackon_categories:
        for stem, dotted in _walk_category(category):
            if category == "characters" and stem in ADAPTER_HELPER_STEMS:
                continue
            try:
                mod = importlib.import_module(dotted)
            except Exception as ex:  # noqa: BLE001 - record + continue
                warnings.append(
                    f"{category}/{stem}: import failed ({type(ex).__name__}: {ex})"
                )
                continue
            multi = getattr(mod, "TARGETS", None)
            if isinstance(multi, dict):
                for tgt in _build_tackon_multi(mod, stem, category, dotted, warnings):
                    targets[tgt.name] = tgt
                continue
            render = getattr(mod, "render", None)
            if not callable(render):
                warnings.append(
                    f"{category}/{stem}: no `render(out_dir, **opts) -> Iterable[Path]` "
                    f"function (and no `TARGETS` dict) — add one to register as a "
                    f"tack-on target, or move shared helpers to the package root."
                )
                continue
            targets[stem] = _build_tackon_single(mod, stem, category, dotted, render)
    return DiscoveryReport(targets=targets, warnings=warnings)


def _discover_yaml_configs(
    config_dir: Path, category: str
) -> Tuple[Dict[str, Target], List[str]]:
    """Walk ``config_dir/*.yaml`` and wrap each as an AdapterTarget."""
    targets: Dict[str, Target] = {}
    warnings: List[str] = []
    if not config_dir.is_dir():
        return targets, warnings
    for path in sorted(config_dir.glob("*.yaml")):
        stem = path.stem
        try:
            target = AdapterTarget(config_path=path, category=category)
        except Exception as ex:  # noqa: BLE001
            warnings.append(
                f"{category}/{stem}: load failed ({type(ex).__name__}: {ex})"
            )
            continue
        targets[target.name] = target
    return targets, warnings


def discover_all_targets() -> DiscoveryReport:
    """Walk every surface (tack-ons + YAML configs) into one Target dict.

    Sources, in precedence order (later overrides earlier on name collision):

    1. ``configs/review/*.yaml`` — category ``"review_npcs"``
    2. ``configs/*.yaml`` (main) — category ``"characters"`` (collides with tack-on chars; tack-ons win)
    3. Tack-on Python modules under ``targets/<category>/`` — categories
       ``characters`` / ``props`` / ``tiles`` / ``icons``

    So a tack-on character with the same name as a YAML config (e.g.
    `sandbag` ships both) gets the tack-on. The YAML pipeline is still
    reachable via `draw-character <config>` for one-off use.
    """
    tackon_report = discover_tackon_targets()
    # `configs/review/*.yaml` joins `characters` (Phase 6 cleanup —
    # the split was internal renderer-bookkeeping). Adapter rigs and
    # tack-ons both surface under one category.
    review_targets, review_warnings = _discover_yaml_configs(
        _configs_dir() / "review",
        "characters",
    )
    main_targets, main_warnings = _discover_yaml_configs(
        _configs_dir(),
        "characters",
    )
    targets: Dict[str, Target] = {}
    # Precedence (later overrides earlier).
    targets.update(review_targets)
    targets.update(main_targets)
    targets.update(tackon_report.targets)
    return DiscoveryReport(
        targets=targets,
        warnings=tackon_report.warnings + main_warnings + review_warnings,
    )


__all__ = [
    "ADAPTER_HELPER_STEMS",
    "AdapterTarget",
    "CATEGORIES",
    "DiscoveryReport",
    "TackonTarget",
    "Target",
    "default_sheet_files",
    "discover_all_targets",
    "discover_tackon_targets",
]
