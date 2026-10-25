"""Findings about the art: files no generator claims, records whose bytes
changed, inputs that changed since a render, claimed files that are absent,
tier copies older than their source, and assets nothing in the game names.
Each finding names its evidence; a heuristic says so."""

from __future__ import annotations

import os
from pathlib import Path
from typing import Any

from .generators import Evidence
from .scan import HashCache, Product


def _d(code: str, severity: str, subject: str, message: str, **extra: Any) -> dict[str, Any]:
    return {"code": code, "severity": severity, "subject": subject, "message": message, **extra}


def findings(
    repo: Path,
    products: dict[str, Product],
    assets: dict[str, Any],
    ev: Evidence,
    hashes: HashCache,
    used: set[str],
) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = [_d("generator-warning", "info", "renderer", w) for w in ev.warnings]
    for product in products.values():
        prov = product.provenance
        if prov.get("confidence") == "unknown":
            out.append(_d("unknown-generator", "warn", product.path, "No generator record, cache entry, registry claim or matching name.", asset=product.asset))
        if prov.get("stale_record"):
            out.append(_d("changed-since-record", "warn", product.path, "The file's bytes differ from the digest its generator recorded: published by another road, or edited.", asset=product.asset))
        if product.root.of and prov.get("derived_from") in products:
            source = products[prov["derived_from"]]
            if product.mtime + 1 < source.mtime:
                out.append(_d("tier-older-than-source", "warn", product.path, f"This {product.root.tier} copy is older than {source.path}.", asset=product.asset))
    # Claimed by a generator, absent on disk.
    for path, (gid, _sha, _row) in ev.by_path.items():
        if path not in products:
            out.append(_d("recorded-output-missing", "warn", path, f"{gid} recorded this output; the file is gone.", generator=gid))
    for path, target in ev.claimed.items():
        # A target also claims companions it may not make (a parts table, a
        # body rig): only a missing sheet page or record is a finding.
        if path.endswith(("_spritesheet.png", "_spritesheet.ron")) and path not in products and path not in ev.by_path:
            out.append(_d("claimed-output-missing", "info", path, f"target:{target} claims this file; it is not published here.", generator=f"target:{target}"))
    # Rendered records whose inputs changed since.
    for gid, record in ev.records.items():
        if record.get("basis") != "rendered":
            continue
        changed = []
        for item in record.get("inputs", []):
            file = repo / item["path"]
            if item.get("sha256") and (not file.is_file() or hashes.digest(file) != item["sha256"]):
                changed.append(item["path"])
        if changed:
            out.append(_d("inputs-changed", "warn", gid, f"{len(changed)} input(s) changed since the render: the art is stale.", generator=gid, inputs=changed[:20]))
    # Assets nothing in the game names (a heuristic: a Rust constant built at
    # run time, or a demo that names it by a computed key, is invisible here).
    for aid, asset in assets.items():
        if asset.family in ("reference", "manifest", "review sheet", "legacy background") or aid in used:
            continue
        if aid.startswith(("sheet:", "image:sprites/", "parallax:", "skin:", "sky:")):
            out.append(_d("no-known-use", "info", aid, "No content row, LDtk placement or Rust literal names this asset (heuristic).", asset=aid))
    return out
