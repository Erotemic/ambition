"""Build the art index and the offline inspector page.

    python -m art_index build [--out DIR] [--jobs N] [--no-previews] [--open]

Writes `DIR` (default `.agent/art_inspector/`, git-ignored): `index.html`,
`app.js`, `app.css`, `catalog.js` (the index, as a script so that the page
works from `file://`) and the previews. The page links to the art and the
source files by relative paths, so it opens in place, with no server.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import shutil
import subprocess
import sys
import time
import webbrowser
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from . import SCHEMA, assets as assets_mod, diagnostics, generators, lineage, related, thumbs, usage
from .paths import repo_root, roots
from .scan import HashCache, scan

WEB = Path(__file__).resolve().parents[1] / "web"


def _git(repo: Path, *args: str) -> str:
    done = subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True)
    return done.stdout.strip()


def build(
    repo: Path,
    out: Path,
    *,
    jobs: int = 4,
    previews: bool = True,
    prune: bool = False,
    python: str | None = None,
    targets: dict[str, Any] | None = None,
    lineages: bool = True,
    log=print,
) -> dict[str, Any]:
    """Index the art of `repo` into `out`. `targets` stands in for the
    renderer registry (`list --json`), for a test."""
    started = time.perf_counter()
    out.mkdir(parents=True, exist_ok=True)

    def step(message: str) -> None:
        log(f"[{time.perf_counter() - started:6.1f}s] {message}")

    hashes = HashCache(out / "hash_cache.json")
    step("scanning the art roots")
    products = scan(roots(repo), hashes)
    step(f"{len(products)} files; reading the generators")
    warnings: list[str] = []
    if targets is None:
        targets, warnings = generators.renderer_targets(repo, python)
    ev = generators.load_evidence(repo, targets)
    ev.warnings += warnings
    actor_targets: dict[str, str] = {}
    step("grouping assets and reading sidecars")
    # Sheet keys named by actor sidecars (`renderer_target`) inform resolution.
    generators.resolve(products, ev, actor_targets)
    assets = assets_mod.group(products, ev.generators)
    assets_mod.read_sidecars(assets, products)
    for asset in assets.values():
        actor = asset.data.get("actor") or {}
        if actor.get("renderer_target"):
            actor_targets[asset.name] = actor["renderer_target"]
    if actor_targets:
        unresolved = {k: v for k, v in products.items() if v.provenance.get("confidence") == "unknown"}
        generators.resolve(unresolved, ev, actor_targets)
        for asset in assets.values():
            assets_mod.rate(asset, products)

    step("reading where the game uses each asset")
    index = usage.Index(assets, products)
    content, tables = usage.content_uses(repo, index)
    placements, unresolved_ldtk = usage.ldtk_uses(repo, index, tables)
    literals = usage.rust_literals(repo, index)
    used = {u["asset"] for u in content} | {p["asset"] for p in placements} | {u["asset"] for u in literals}

    preview_errors: list[str] = []
    step("previews" + ("" if previews else " (named only; --no-previews)"))
    preview_errors = thumbs.plan_and_draw(out, products, assets, jobs, draw=previews)
    if prune:
        step(f"pruned {thumbs.prune(out, products, assets)} previews the index no longer names")

    step("diagnostics")
    found = diagnostics.findings(repo, products, assets, ev, hashes, used)
    found += [{"code": "preview-failed", "severity": "info", "subject": e.split(":", 1)[0], "message": e} for e in preview_errors]
    for asset in assets.values():
        for error in asset.data.get("errors", []):
            found.append({"code": "sidecar-error", "severity": "warn", "subject": asset.id, "message": error, "asset": asset.id})
    found += [{"code": "ldtk-unresolved", "severity": "info", "subject": f"{u['field']}={u['value']}", "message": f"{u['count']} placement(s) in {u['world']} name no asset the index can resolve.", **u} for u in unresolved_ldtk]

    catalog = _catalog(repo, out, products, assets, ev, content + literals, placements, found)
    if lineages:
        step("lineages and authorship (design history)")
        for warning in lineage.attach(repo, out, assets, catalog["assets"], targets, log=log):
            catalog["findings"].append({"code": "lineage-warning", "severity": "info", "subject": "design history", "message": warning})
    step("related assets")
    related.relate(catalog["assets"], catalog["generators"], repo)
    related.generations(catalog["assets"])
    hashes.save()
    catalog["build_seconds"] = round(time.perf_counter() - started, 1)
    text = json.dumps(catalog, separators=(",", ":"), sort_keys=True)
    text = text.replace("<", "\\u003c").replace("\u2028", "\\u2028").replace("\u2029", "\\u2029")
    partial = out / "catalog.js.partial"
    partial.write_text(f"window.ART_CATALOG = {text};\n")
    partial.replace(out / "catalog.js")
    for name in ("index.html", "app.js", "app.css"):
        shutil.copyfile(WEB / name, out / name)
    step(f"wrote {out / 'index.html'} ({len(assets)} assets, {len(products)} files, {len(found)} findings)")
    return catalog


def _describe(value: Any) -> str | None:
    """An actor's description as text: a structured authoring description
    (`parody_of`, `concept`, ...) gives its concept and whom it parodies."""
    if isinstance(value, dict):
        text = value.get("concept") or value.get("summary") or ""
        return f"{text} (A parody of {value['parody_of']}.)" if value.get("parody_of") else text or None
    return value


def _catalog(repo: Path, out: Path, products, assets, ev, uses, placements, found) -> dict[str, Any]:
    to_repo = os.path.relpath(repo, out)
    uses_by: dict[str, list] = defaultdict(list)
    for u in uses:
        uses_by[u["asset"]].append({k: v for k, v in u.items() if k != "asset"})
    places_by: dict[str, list] = defaultdict(list)
    for p in placements:
        places_by[p["asset"]].append({k: v for k, v in p.items() if k != "asset"})
    finds_by: Counter = Counter(f.get("asset") for f in found if f.get("asset"))

    gen_assets: dict[str, set] = defaultdict(set)
    for asset in assets.values():
        if asset.generator:
            gen_assets[asset.generator].add(asset.id)
    gens = {}
    for gid, g in ev.generators.items():
        record = ev.records.get(gid)
        row = dict(g)
        if record:
            row["basis"] = record.get("basis")
            row["inputs"] = [i["path"] for i in record.get("inputs", [])]
            row["renderer"] = record.get("renderer")
            row["written_at"] = record.get("written_at")
            row["cache_key"] = record.get("cache_key")
        row["assets"] = sorted(gen_assets.get(gid, ()))
        gens[gid] = row

    asset_rows = {}
    for asset in assets.values():
        data = asset.data
        actor = data.get("actor") or {}
        layout = data.get("layout")
        if layout:
            layout = {k: v for k, v in layout.items() if k not in ("rows",)} | {
                "rows": [{k: v for k, v in r.items() if k != "rects"} for r in layout["rows"]]
            }
        portraits = data.get("portraits")
        if portraits:
            portraits = {"default": portraits["default"], "clips": {n: {"thumb": c.get("thumb"), "frames": len(c["rects"]), "ms": c["ms"]} for n, c in portraits["clips"].items()}}
        frames = sum(r["frames"] for r in (layout or {}).get("rows", []))
        asset_rows[asset.id] = {
            "id": asset.id,
            "family": asset.family,
            "name": asset.name,
            "display": actor.get("display_name") or asset.name,
            "generator": asset.generator,
            "confidence": asset.confidence,
            "weakest": data.get("weakest"),
            "cover": asset.cover,
            "products": sorted(asset.products),
            "character_id": actor.get("character_id"),
            "tags": actor.get("tags", []),
            "description": _describe(actor.get("description")) or _describe((layout or {}).get("description")),
            "lineage": actor.get("lineage"),
            "layout": layout,
            "portraits": portraits,
            "parts": data.get("parts"),
            "body_rig": data.get("body_rig"),
            "uses": uses_by.get(asset.id, []),
            "placements": places_by.get(asset.id, []),
            "findings": finds_by.get(asset.id, 0),
            "animations": len((layout or {}).get("rows", [])),
            "frames": frames,
            "bytes": sum(products[p].bytes for p in asset.products),
            "errors": data.get("errors", []),
        }

    product_rows = {}
    for path, p in products.items():
        prov = p.provenance
        if prov.get("confidence") == "derived":
            # A tier copy: the page names the source; its evidence is the same sentence each time.
            prov = {"confidence": "derived", "derived_from": prov.get("derived_from"), "generator": prov.get("generator")}
        product_rows[path] = {
            "asset": p.asset,
            "role": p.role,
            "tier": p.root.tier,
            "bytes": p.bytes,
            "size": list(p.size) if p.size else None,
            "sha256": p.sha256,
            "thumb": p.thumb,
            "href": os.path.relpath(p.file, out),
            "mtime": int(p.mtime),
            "provenance": prov,
        }

    coverage: dict[str, Counter] = defaultdict(Counter)
    for p in products.values():
        coverage[p.root.of or p.root.name][p.provenance.get("confidence", "unknown")] += 1
    return {
        "schema": SCHEMA,
        "built_at": dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat(),
        "repo_commit": _git(repo, "rev-parse", "HEAD"),
        "renderer_commit": _git(repo / "tools" / "ambition_sprite2d_renderer", "rev-parse", "HEAD"),
        "repo_href": to_repo,
        # For editor links (`vscode://file/...`); the page never reads the disk itself.
        "repo_abs": str(repo),
        "coverage": {k: dict(v) for k, v in sorted(coverage.items())},
        "generators": gens,
        "assets": asset_rows,
        "products": product_rows,
        "findings": found,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="art_index", description=__doc__.split("\n", 1)[0])
    sub = parser.add_subparsers(dest="command", required=True)
    b = sub.add_parser("build", help="index the art and write the inspector page")
    b.add_argument("--out", type=Path, default=None, help="default: <repo>/.agent/art_inspector")
    b.add_argument("--jobs", type=int, default=4, help=f"preview processes (at most {thumbs.MAX_JOBS})")
    b.add_argument("--no-previews", action="store_true", help="name previews but draw none (a fast rebuild of the index)")
    b.add_argument("--prune", action="store_true", help="delete previews the index no longer names")
    b.add_argument("--renderer-python", default=None, help="the interpreter for `ambition_sprite2d_renderer list --json`")
    b.add_argument("--no-lineage", action="store_true", help="skip the lineages and the authorship (git blame) of the design history")
    b.add_argument("--open", action="store_true", help="open the page in a browser when built")
    args = parser.parse_args(argv)
    repo = repo_root()
    out = (args.out or repo / ".agent" / "art_inspector").resolve()
    build(repo, out, jobs=args.jobs, previews=not args.no_previews, prune=args.prune, python=args.renderer_python,
          lineages=not args.no_lineage)
    if args.open:
        webbrowser.open((out / "index.html").as_uri())
    return 0


if __name__ == "__main__":
    sys.exit(main())
