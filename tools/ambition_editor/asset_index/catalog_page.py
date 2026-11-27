"""Write the catalog files that the browser reads."""

from __future__ import annotations

import gzip
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path
from tempfile import NamedTemporaryFile

INDEX = "catalog-index.js"
CHUNKS = "catalog-data"
PREFIX = "window.ART_CATALOG = "


def _select(row, fields):
    return {key: row[key] for key in fields.split() if key in row}


def _json(value):
    return json.dumps(value, separators=(",", ":")).replace("<", "\\u003c")


def _write(path, data):
    with NamedTemporaryFile(dir=path.parent, delete=False) as file:
        temp = Path(file.name)
        try:
            file.write(data)
            file.close()
            temp.chmod(0o644)
            temp.replace(path)
        finally:
            temp.unlink(missing_ok=True)


def _bucket(key):
    value = 0
    for char in key:
        value = (value * 31 + ord(char)) % 128
    return value


def _groups(rows):
    buckets = defaultdict(dict)
    for key, row in rows.items():
        buckets[_bucket(key)][key] = row
    return buckets.items()


def _asset_search(a, generator):
    fields = [a["id"], a["name"], a["display"], a["family"], *(a.get("character_ids") or [a.get("character_id")]),
              " ".join(a.get("tags") or []), a.get("generator"), generator.get("source"), generator.get("module"),
              " ".join(u["path"] + " " + (u.get("row") or "") for u in a.get("uses", [])),
              " ".join(p["level"] + " " + p["field"] for p in a.get("placements", [])), _json(a.get("chargen") or {}),
              " ".join(r["name"] for r in (a.get("layout") or {}).get("rows", [])), " ".join(a["products"]),
              " ".join(l["lineage"] + " " + l["name"] for l in a.get("lineages", [])),
              " ".join(s["who"] for s in (a.get("authorship") or {}).get("shares", []))]
    return " ".join(field or "" for field in fields).lower()


def write(out: Path, catalog: dict, *, prune=False):
    """Write all chunks before the index. An open page keeps its chunk URLs."""
    folder = out / CHUNKS
    folder.mkdir(exist_ok=True)
    chunks = {}

    def chunk(name, value):
        body = f"window.ArtCatalog.receive({_json(name)},{_json(value)});\n".encode()
        digest = hashlib.sha256(body).hexdigest()
        path = folder / f"{digest}.js"
        zipped = path.with_suffix(".js.gz")
        if not path.is_file():
            _write(path, body)
        if not zipped.is_file():
            _write(zipped, gzip.compress(body, compresslevel=5, mtime=0))
        chunks[name] = digest

    assets = catalog.get("assets", {})
    products = catalog.get("products", {})
    generators = catalog.get("generators", {})
    items = catalog.get("items", {})
    inventory = catalog.get("inventory") or {}
    identities = inventory.get("identities", {})
    units = inventory.get("units", {})
    findings = catalog.get("findings", [])
    review = catalog.get("reviews") or {"reviews": [], "backlog": {"count": 0, "rows": []}}
    item_kinds = defaultdict(dict)
    for key, row in items.items():
        item_kinds[row["kind"]][key] = row
    for kind, rows in item_kinds.items():
        chunk(f"items-{kind}", rows)

    unit_status = Counter(unit["status"]["state"] for unit in units.values())
    attention = {key: dict(_select(row, "id name display kind state media reviews"),
                           reasons=row["reasons"][:1], uses_count=len(row["uses"]))
                 for key, row in items.items() if row["state"] != "present"}
    core = _select(catalog, "built_at build_seconds repo_commit renderer_commit repo_href repo_abs categories coverage kinds")
    core.update(counts={"assets": len(assets), "files": len(products), "findings": len(findings), "characters": len(identities),
                        "reviews": len(review["reviews"]), "backlog": review["backlog"]["count"]},
                unit_status=dict(unit_status), attention=attention,
                item_labels={key: row["display"] for key, row in items.items()},
                inventory=dict(profile={"state": "unavailable"}, warnings=[],
                               **_select(inventory, "complete publication")))
    core["inventory"].update(_select(inventory, "profile warnings"))
    families = catalog.get("families") or {}
    core["families"] = dict(families, libraries=[
        dict(library, families={name: dict(_select(family, "state units unchecked_units"),
                                           stale_units=[dict(_select(row, "reason"), changed=row.get("changed", [])[:12])
                                                        for row in family.get("stale_units", [])[:2]])
                                for name, family in library["families"].items()})
        for library in families.get("libraries", [])])

    chunk("images", {
        key: dict(_select(a, "id name display family generator confidence cover art_label tags category tier_copy "
                             "animations frames bytes findings generation identities"),
                  file_count=len(a["products"]), used=bool(a.get("uses") or a.get("placements")),
                  lineage_ids=[row["lineage"] for row in a.get("lineages", [])],
                  placement_count=sum(p["count"] for p in a.get("placements", [])),
                  game_use=unit.get("game_use"), unit_state=(unit.get("status") or {}).get("state"))
        for key, a in assets.items() for unit in [units.get(a.get("inspection_unit"), {})]})
    chunk("image-search", {key: _asset_search(a, generators.get(a.get("generator"), {})) for key, a in assets.items()})
    chunk("generators", {key: dict(row, evidence=[
        _select(products[path]["provenance"], "made_by evidence")
        for aid in row.get("assets", []) for path in assets[aid]["products"]
        if products[path]["tier"] == "full" and any(products[path]["provenance"].get(field) for field in ("made_by", "evidence"))])
        for key, row in generators.items()})
    chunk("findings", findings)
    chunk("lineages", catalog.get("lineages") or {})
    chunk("reviews", review)
    latest = {}
    for row in review["reviews"]:
        latest.setdefault(row["id"], _select(row, "score label relation"))
    core["attention_reviews"] = {key: latest[key] for key in attention if key in latest}
    core["polish_threshold"] = review.get("polish_threshold", 6)
    chunk("review-summary", {"latest": latest, "backlog": dict(Counter(row["id"] for row in review["backlog"]["rows"])),
                             "polish_threshold": core["polish_threshold"]})

    character_rows = {}
    for key, actor in identities.items():
        actor_units = [units[uid] for uid in actor["units"]]
        art = [assets[aid] for unit in actor_units for aid in unit.get("assets", []) if key in assets[aid].get("identities", [])]
        preview = actor.get("preview") or {}
        cover = next((href for asset in art
                      if (href := preview.get("cover") if preview.get("asset") == asset["id"] else asset.get("cover"))), None)
        inventory_cover = next((assets[aid].get("cover") for aid in (actor_units[0].get("assets", []) if actor_units else [])
                                if key in assets[aid].get("identities", [])), None)
        families = [((unit["facts"].get("visual_family") or {}).get("value") or "unknown") for unit in actor_units]
        needs_art = not actor_units or any(
            unit.get("runtime_defects") or any(status["state"] in ("missing", "partial", "stale")
                                               for status in [unit["status"], *unit["status"].get("tiers", {}).values()])
            for unit in actor_units)
        character_rows[key] = dict(_select(actor, "id key provider display units relations lineages preview added_at generated_at generated_at_evidence"),
                                   catalog=_select(actor["catalog"], "tags manifest cast"),
                                   cover=cover, inventory_cover=inventory_cover, art_families=list(dict.fromkeys(families)),
                                   needs_art=needs_art,
                                   coverage_gaps=list(dict.fromkeys(c["action"] for c in actor["coverage"]
                                                                    if not c["valid"] or c["resolution"]["state"] not in ("direct", "alias"))))
    chunk("characters", dict(identities=character_rows, character_groups=inventory.get("character_groups", []),
                             lineages=inventory.get("lineages", {})))
    unit_rows = {}
    for key, unit in units.items():
        status = unit["status"]
        unit_rows[key] = dict(_select(unit, "id kind assets game_use registration generator_family identities lineages repair"),
                              generator=_select(unit["generator"], "family"),
                              runtime_defects=bool(unit.get("runtime_defects")),
                              status=dict(state=status["state"], tiers={name: {"state": val["state"]}
                                                                         for name, val in status.get("tiers", {}).items()}),
                              facts={name: _select(fact, "value state") for name, fact in unit["facts"].items()})
    chunk("inventory", unit_rows)
    chunk("inventory-search", {
        key: json.dumps([unit["id"], unit["kind"], unit["facts"], unit["generator"], unit.get("source"),
                         unit["lineages"], [identities[cid] for cid in unit["identities"]]],
                        ensure_ascii=False, separators=(",", ":")).lower()
        for key, unit in units.items()})
    chunk("concepts", inventory.get("concepts", {}))
    chunk("unregistered", inventory.get("local_products", []))
    runtime = inventory.get("runtime")
    chunk("runtime", {"runtime": runtime, "published": {
        key: [_select(p, "key fingerprint") for p in unit["published"]] for key, unit in units.items()} if runtime else {}})
    for number, rows in _groups(identities):
        chunk(f"character-{number}", rows)
    for number, rows in _groups(units):
        chunk(f"unit-{number}", rows)

    by_asset, by_subject = defaultdict(list), defaultdict(list)
    for index, finding in enumerate(findings):
        by_asset[finding.get("asset")].append(index)
        by_subject[finding["subject"]].append(index)
    folders = catalog.get("href_folders", [])
    for number, rows in _groups(assets):
        files = {}
        related_findings = {}
        for key, asset in rows.items():
            indices = set(by_asset[key])
            for path in asset["products"]:
                indices.update(by_subject[path])
                product = products[path]
                href = product.get("href") or ""
                if href.startswith("~"):
                    folder_number, _, name = href[1:].partition("/")
                    product = dict(product, href=folders[int(folder_number)] + "/" + name)
                files[path] = product
            if indices:
                related_findings[key] = [findings[i] for i in sorted(indices)]
        chunk(f"asset-{number}", {"assets": rows, "products": files, "findings": related_findings,
                                  "generators": {gid: generators[gid] for asset in rows.values()
                                                 if (gid := asset.get("generator")) in generators},
                                  "character_labels": {key: identities[key]["display"] for asset in rows.values()
                                                       for key in asset.get("identities", []) if key in identities}})

    index = out / INDEX
    previous = index.with_suffix(".previous")
    if index.is_file():
        _write(previous, index.read_bytes())
    _write(index, f"{PREFIX}{_json(dict(core, chunks=chunks))};\n".encode())
    if prune:
        keep = set(chunks.values())
        if previous.is_file():
            keep.update(json.loads(previous.read_text().removeprefix(PREFIX).rstrip().removesuffix(";"))["chunks"].values())
        for path in folder.iterdir():
            if path.name.endswith((".js", ".js.gz")) and path.name.split(".")[0] not in keep:
                path.unlink(missing_ok=True)
