"""Each character's lineage and the authorship of its generator, from the
sprite renderer's design history
(`tools/ambition_sprite2d_renderer/design_history/`).

The design history owns the lineages (`lineages.yaml`), the join from a
catalog id or a target to a lineage, and the authorship measure (`git blame`
of the generator files, each line attributed by its commit). This module only
calls it and puts the result on the asset rows. It never renders: an era's
still shows when the design-history cache has it (`python -m design_history
build <lineage>`).

The history store (`python -m design_history fetch`) holds the commits from
before the active renderer repository. Without it, the renderer checkout is
the only source: eras from the older stretches show their commit and label
only, and lines that came through an import count as unknown.
"""

from __future__ import annotations

import json
import shutil
import sys
from pathlib import Path
from typing import Any


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
    import design_history.view as view  # noqa: PLC0415

    return (authorship, characters, stills, store, view), renderer


def attach(repo: Path, out: Path, assets: dict[str, Any], rows: dict[str, dict], targets: dict[str, Any],
           *, authorship: bool = True, log=print) -> list[str]:
    """Set `lineages` (the lineages of each sheet asset, with eras) and
    `authorship` (who wrote its generator now) on the catalog `rows`.
    Returns warnings."""
    modules, renderer = _design_history(repo)
    if modules is None:
        return ["design history unavailable: tools/ambition_sprite2d_renderer/design_history is absent"]
    authorship_mod, characters, stills, store_mod, view = modules
    import yaml  # noqa: PLC0415

    warnings: list[str] = []
    lineages = yaml.safe_load((renderer / "design_history" / "lineages.yaml").read_text(encoding="utf-8"))["lineages"]
    join = characters.join(lineages, repo)
    current = set(targets)

    store = store_mod.default_store()
    has_store = bool(store_mod.git(store, "for-each-ref", "refs/active", check=False).strip()) if store.exists() else False
    local = renderer if (renderer / ".git").exists() else None
    if not has_store:
        warnings.append("design history: no history store; eras before the active renderer show no authorship "
                        "(run `python -m design_history fetch` in tools/ambition_sprite2d_renderer)")
    git_dir = store if has_store else local
    ledger = store_mod.load_ledger(store) if has_store else {}
    cache = stills.default_cache()
    blamer = authorship_mod.Blamer(git_dir, cache, ledger, local=local) if git_dir and authorship else None
    head = store_mod.git(local, "rev-parse", "HEAD", check=False).strip() if local else ""

    records = {}
    record_dir = repo / "crates" / "ambition_platformer2d_actor_monolith" / "asset_provenance" / "sprites"
    for path in sorted(record_dir.glob("*.json")) if record_dir.is_dir() else []:
        try:
            records[path.stem] = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            continue

    views: dict[str, dict] = {}
    still_dir = out / "lineage"

    def lineage(name: str) -> dict:
        if name not in views:
            record = view.lineage_view(name, lineages[name], store=git_dir, cache=cache, blamer=blamer,
                                       current=current, ledger=ledger)
            for era in record["eras"]:
                src = era.get("still")
                if src:
                    still_dir.mkdir(parents=True, exist_ok=True)
                    dest = still_dir / Path(src).name
                    if not dest.exists() or dest.stat().st_size != Path(src).stat().st_size:
                        shutil.copyfile(src, dest)
                    era["still"] = f"lineage/{dest.name}"
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
    log(f"lineages: {linked} assets link to a lineage ({len(views)} lineages)")
    return warnings
