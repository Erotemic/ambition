"""Each character's lineage and the authorship of its generator, from the
sprite renderer's design history
(`tools/ambition_sprite2d_renderer/design_history/`).

The design history owns the lineages (`lineages.yaml`), the join from a
catalog id or a target to a lineage, and the authorship measure (`git blame`
of the generator files, each line attributed by its commit). This module only
calls it and puts the result on asset rows and the lineage index. It copies
cached historical images and frames their thumbnails. It does not run an old
renderer (`python -m design_history build <lineage>` prepares that cache).

The history store (`python -m design_history fetch`) holds the commits from
before the active renderer repository. Without it, the renderer checkout is
the only source: eras from the older stretches show their commit and label
only, and lines that came through an import count as unknown.
"""

from __future__ import annotations

import json
import hashlib
import shutil
import sys
from pathlib import Path
from typing import Any

from . import thumbs


def _design_history(repo: Path):
    renderer = repo / "tools" / "ambition_sprite2d_renderer"
    if not (renderer / "design_history" / "lineages.yaml").is_file():
        return None, renderer
    if str(renderer) not in sys.path:
        sys.path.insert(0, str(renderer))
    import design_history.authorship as authorship  # noqa: PLC0415
    import design_history.characters as characters  # noqa: PLC0415
    import design_history.stills as stills  # noqa: PLC0415
    import design_history.store as store  # noqa: PLC0415
    import design_history.variations as variations  # noqa: PLC0415
    import design_history.view as view  # noqa: PLC0415

    return (authorship, characters, stills, store, view, variations), renderer


def attach(repo: Path, out: Path, assets: dict[str, Any], rows: dict[str, dict], targets: dict[str, Any],
           *, authorship: bool = False, previews: bool = True, log=print) -> tuple[dict, list[str]]:
    """Set `lineages` (the lineages of each sheet asset, with eras) and
    `authorship` (who wrote its generator now) on the catalog `rows`.
    Return visual lineage records and warnings."""
    modules, renderer = _design_history(repo)
    if modules is None:
        return {}, ["design history unavailable: tools/ambition_sprite2d_renderer/design_history is absent"]
    authorship_mod, characters, stills, store_mod, view, variations_mod = modules
    import yaml  # noqa: PLC0415

    warnings: list[str] = []
    lineages = yaml.safe_load((renderer / "design_history" / "lineages.yaml").read_text(encoding="utf-8"))["lineages"]
    join = characters.join(lineages, repo)
    current = set(targets)

    store = store_mod.default_store()
    has_store = bool(store_mod.git(store, "for-each-ref", "refs/active", check=False).strip()) if store.exists() else False
    if has_store and authorship:
        # The eras of the last few weeks are commits of the active checkouts:
        # bring the store's active refs up to them (a local fetch).
        try:
            store_mod.fetch(store, active_ambition=repo if (repo / ".git").exists() else None,
                            active_renderer=renderer if (renderer / ".git").exists() else None)
        except Exception as ex:  # noqa: BLE001 - an older store still shows the older eras
            warnings.append(f"design history: could not refresh the history store: {ex}")
    local = renderer if (renderer / ".git").exists() else None
    if not has_store:
        warnings.append("design history: no history store; eras before the active renderer show no authorship "
                        "(run `python -m design_history fetch` in tools/ambition_sprite2d_renderer)")
    git_dir = store if has_store else local
    ledger = store_mod.load_ledger(store) if has_store and authorship else {}
    cache = stills.default_cache()
    blamer = authorship_mod.Blamer(git_dir, cache, ledger, local=local) if git_dir and authorship else None
    head = store_mod.git(local, "rev-parse", "HEAD", check=False).strip() if local else ""

    records = {}
    record_dir = repo / "crates" / "ambition_platformer2d_actor_monolith" / "asset_provenance" / "sprites"
    for path in sorted(record_dir.glob("*.json")) if authorship and record_dir.is_dir() else []:
        try:
            records[path.stem] = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            continue

    views: dict[str, dict] = {}
    still_dir = out / "lineage"
    surveyed = variations_mod.load()

    copied: dict[Path, tuple[str, str | None]] = {}

    def copy_still(src: Path) -> tuple[str, str | None]:
        if src in copied:
            return copied[src]
        still_dir.mkdir(parents=True, exist_ok=True)
        digest = hashlib.sha256(src.read_bytes()).hexdigest()[:24]
        dest = still_dir / f"{src.stem}-{digest}.png"
        if not dest.exists():
            shutil.copyfile(src, dest)
        preview = still_dir / f"{digest}-subject-v1.webp"
        if previews and not preview.exists():
            from PIL import Image
            from design_history.progression import character_bbox
            with Image.open(src) as source:
                image = source.convert("RGBA")
            box = character_bbox(image)
            if box:
                image = image.crop(box)
            camera = thumbs.subject_camera(image, None, None)
            thumbs._save(thumbs.subject_frame(image, camera, thumbs.THUMB), preview)
        result = (f"lineage/{dest.name}", f"lineage/{preview.name}" if preview.exists() else None)
        copied[src] = result
        return result

    def lineage(name: str) -> dict:
        if name not in views:
            record = view.lineage_view(name, lineages[name], store=git_dir, cache=cache, blamer=blamer,
                                       current=current, ledger=ledger, include_attribution=authorship)
            for era in record["eras"]:
                src = era.get("still")
                if src:
                    era["still"], era["preview"] = copy_still(Path(src))
            # Every variation the survey drew (`variations.yaml`), by track.
            tracks = []
            for track, row in ((surveyed.get(name) or {}).get("tracks") or {}).items():
                looks = []
                for v in row.get("variations") or []:
                    png = stills.cached_still(cache, v["commit"], row["ids"], lineages[name].get("pick"))
                    original, preview = copy_still(png) if png else (None, None)
                    looks.append(dict(v, still=original, preview=preview))
                tracks.append({"track": track, "candidates": row.get("candidates"), "failed": row.get("failed"), "variations": looks})
            record["tracks"] = tracks
            views[name] = record
        return views[name]

    linked = 0
    for aid, asset in assets.items():
        row = rows.get(aid)
        gid = asset.generator or ""
        if row is None or not gid.startswith("target:"):
            continue
        target = gid.split(":", 1)[1]
        found = [(n, f"target {target}") for n in join.targets.get(target, [])]
        if row.get("character_id"):
            found += [h for h in join.find(row["character_id"]) if h[0] not in {n for n, _ in found}]
        if found:
            row["lineages"] = [dict(lineage(n), found_via=via) for n, via in found]
            linked += 1
        if blamer is not None and head and targets.get(target, {}).get("category") == "characters":
            files = authorship_mod.specific_inputs(records, target)
            how = "record"
            if not files:
                spec = {"ids": [target]}
                _, files, how = authorship_mod.era_generator(local, head, "", spec, {}, current)
            who = blamer.authorship(head, files, "named" if how == "record" else how).as_dict()
            who["files_from"] = "provenance record inputs" if how == "record" else f"file names ({how})"
            row["authorship"] = who
    for name in lineages:
        lineage(name)
    log(f"lineages: {linked} assets link to a lineage ({len(views)} lineages, {len(copied)} historical images)")
    return views, warnings
