"""Which generator made each product, and how sure the index is.

The evidence, strongest first:

* `verified`: a provenance record (`ambition_sprite2d_renderer/provenance.py`)
  or an accepted bundle names the file with the digest it has now.
* `declared`: a generator's record or registry claims the file, but nothing
  vouches for its bytes (a `plan` record, or a digest that has since changed).
* `derived`: a reduced-quality copy of another product, made by
  `scripts/generate_visual_quality_variants.py`; its generator is the source's.
* `inferred`: only a name points at a generator (an actor sidecar's
  `renderer_target`, or a file stem equal to a registered target).
* `reference`: authored or imported art that no generator makes (`assets/`).
* `unknown`: nothing names a generator. A diagnostic.
"""

from __future__ import annotations

import json
import subprocess
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .paths import provenance_root
from .scan import Product
from .sheets import sheet_role

CONFIDENCE = ("verified", "declared", "derived", "inferred", "reference", "unknown")

#: Generators that are not sprite targets, background themes or terrain skins.
FIXED = {
    "script:quality_variants": {
        "family": "quality_variants",
        "id": "quality_variants",
        "source": "scripts/generate_visual_quality_variants.py",
        "command": "./scripts/regen/quality_variants.sh",
        "note": "Downsamples each frame of the full-quality product and repacks it.",
    },
    "tool:legacy_background_profiles": {
        "family": "legacy_background_profile",
        "id": "legacy_background_profiles",
        "source": "tools/ambition_background_renderer/ambition_background_renderer/profiles.py",
        "command": "python -m ambition_background_renderer --out <assets>/backgrounds --profile all",
        "note": "Placeholder profiles; no runtime reader was found (2026-10-24).",
    },
    "tool:ultrapack": {
        "family": "sprite_pack",
        "id": "ultrapack",
        "source": "tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/authoring/ultrapack.py",
        "command": "./scripts/regen/sprites.sh",
    },
    "tool:ldtk_editor_art": {
        "family": "ldtk_editor_art",
        "id": "ldtk_editor_art",
        "source": "tools/ambition_ldtk_tools",
        "command": "./scripts/regen/sprites.sh  # ambition_ldtk_tools asset editor-art / generate-editor-icons",
    },
}


@dataclass
class Evidence:
    records: dict[str, dict[str, Any]] = field(default_factory=dict)  # generator id -> record
    by_path: dict[str, tuple[str, str, dict[str, Any]]] = field(default_factory=dict)  # product path -> (generator id, sha, record row)
    basis: dict[str, str] = field(default_factory=dict)  # generator id -> record basis
    claimed: dict[str, str] = field(default_factory=dict)  # product path -> target
    generators: dict[str, dict[str, Any]] = field(default_factory=dict)
    warnings: list[str] = field(default_factory=list)


def _gen_id(generator: dict[str, Any]) -> str:
    family = generator.get("family", "?")
    prefix = {"sprite_target": "target", "parallax_theme": "theme", "terrain_skin": "skin", "sky_part": "sky"}.get(family, family)
    return f"{prefix}:{generator.get('id')}"


def renderer_targets(repo: Path, python: str | None) -> tuple[dict[str, Any], list[str]]:
    """The renderer registry (`list --json`), run in the renderer's own
    interpreter. Empty, with the reason, when it cannot be run."""
    renderer = repo / "tools" / "ambition_sprite2d_renderer"
    if python is None:
        done = subprocess.run(
            ["bash", "-c", 'source "$1/scripts/lib/tool_python.sh"; ambition_select_tool_python "$2" AMBITION_SPRITE_PYTHON', "_", str(repo), str(renderer)],
            capture_output=True, text=True,
        )
        python = done.stdout.strip() or "python3"
    done = subprocess.run([python, "-m", "ambition_sprite2d_renderer", "list", "--json"], cwd=renderer, capture_output=True, text=True)
    if done.returncode != 0:
        return {}, [f"renderer registry unavailable: `list --json` exited {done.returncode}: {done.stderr.strip()[-300:]}"]
    data = json.loads(done.stdout)
    return data.get("targets", {}), [f"renderer discovery: {w}" for w in data.get("warnings", [])]


def load_evidence(repo: Path, targets: dict[str, Any], units=None) -> Evidence:
    ev = Evidence()
    for name, row in targets.items():
        ev.generators[f"target:{name}"] = {k: v for k, v in row.items() if k != "outputs"}
        for rel in row.get("outputs", []):
            ev.claimed.setdefault(f"{row.get('root', 'sprites')}/{rel}", name)
    ev.generators.update(FIXED)
    import sys
    sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "scripts"))
    from lib.art_publication import store, MANAGED_ROOTS
    from .paths import asset_root
    publication = store(asset_root(repo))
    selected = publication.selected()
    managed = set(MANAGED_ROOTS) | set(selected.get("roots", []))
    root = provenance_root(repo)
    for path in sorted(root.rglob("*.json")) if root.is_dir() else []:
        try:
            record = json.loads(path.read_text())
        except (OSError, ValueError):
            ev.warnings.append(f"unreadable provenance record {path}")
            continue
        if record.get("schema") != "ambition.art_provenance.v1":
            continue
        if record.get("root", "").split("/", 1)[0] in managed or record.get("generator", {}).get("family") == "sprite_target":
            continue
        gid = _gen_id(record["generator"])
        ev.records[gid] = record
        ev.basis[gid] = record.get("basis", "?")
        ev.generators.setdefault(gid, dict(record["generator"]))
        for out in record.get("outputs", []):
            ev.by_path[f"{record['root']}/{out['path']}"] = (gid, out["sha256"], out)
    for uid, key in selected["bundles"].items():
        bundle = publication.bundle(key)
        recipe = bundle["recipe"]
        generator = ev.generators.get(uid, {"id":uid,"family":"bundle"})
        record = {"basis":"rendered" if recipe else "imported", "generator":generator,
                  "bundle":key,"inputs":[],"outputs":[],"status":(units or {}).get(uid,{}).get("status",{})}
        ev.records[uid] = record
        ev.basis[uid] = record["basis"]
        for logical, member in bundle["files"].items():
            if member["role"] == "runtime":
                ev.by_path[logical] = (uid,member["sha256"],member)
    return ev


def resolve(products: dict[str, Product], ev: Evidence, actor_targets: dict[str, str]) -> None:
    """Set `provenance` on each product: generator id, confidence, evidence."""
    for product in products.values():
        p = product.path
        if product.root.origin == "reference":
            product.provenance = {"generator": None, "confidence": "reference", "evidence": ["authored or imported art under assets/ (pinned by its .ipfs sidecar)"]}
            continue
        if product.root.origin == "unmanaged":
            target = ev.claimed.get(p)
            product.provenance = {"generator": f"target:{target}" if target else None,
                "confidence": "declared" if target else "unknown", "publication": "unmanaged",
                "evidence": ["Loose file outside the accepted publication"]}
            continue
        if product.root.of and p not in ev.by_path:
            source = f"{product.root.of}/{product.rel}"
            product.provenance = {"generator": "script:quality_variants", "confidence": "derived", "derived_from": source,
                                  "evidence": [f"a {product.root.tier} copy of {source}"]}
            continue
        if p in ev.by_path:
            gid, sha, row = ev.by_path[p]
            basis = ev.basis.get(gid)
            if sha == product.sha256 and basis in ("rendered", "cache"):
                product.provenance = {"generator": gid, "confidence": "verified", "evidence": [f"provenance record ({basis}): digest matches"]}
            elif sha == product.sha256:
                product.provenance = {"generator": gid, "confidence": "declared", "evidence": [f"provenance record ({basis}): the generator's plan names this file"]}
            else:
                product.provenance = {"generator": gid, "confidence": "declared", "stale_record": True,
                                      "evidence": ["provenance record names this file, but its digest has changed since"]}
            if row.get("made_by"):
                product.provenance["made_by"] = row["made_by"]
            continue
        if p in ev.claimed:
            product.provenance = {"generator": f"target:{ev.claimed[p]}", "confidence": "declared", "evidence": ["registry: the target claims this file"]}
            continue
        if product.root.name.startswith("backgrounds/") and product.root.name.split("/")[1] not in ("parallax_layers", "sky_parts"):
            product.provenance = {"generator": "tool:legacy_background_profiles", "confidence": "declared",
                                  "evidence": ["placeholder profile folder of tools/ambition_background_renderer"]}
            continue
        if product.root.name == "sprite_packs":
            product.provenance = {"generator": "tool:ultrapack", "confidence": "declared", "evidence": ["sprite_packs/ is written by the ultrapack phase"]}
            continue
        if product.root.name == "sprites" and product.rel.startswith(("ldtk_editor_art_", "editor_icons")):
            product.provenance = {"generator": "tool:ldtk_editor_art", "confidence": "declared", "evidence": ["LDtk editor art of ambition_ldtk_tools"]}
            continue
        role = sheet_role(Path(product.rel).name)
        stem = role[0] if role else Path(product.rel).stem
        target = actor_targets.get(stem) or (stem if f"target:{stem}" in ev.generators else None)
        if target and f"target:{target}" in ev.generators:
            product.provenance = {"generator": f"target:{target}", "confidence": "inferred", "evidence": [f"file name names target {target}"]}
            continue
        product.provenance = {"generator": None, "confidence": "unknown", "evidence": ["no record, registry claim or matching name"]}
