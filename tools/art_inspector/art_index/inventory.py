"""Join owner facts by declared references. Keep each evidence axis separate."""
from __future__ import annotations

import json
import hashlib
from collections import Counter, defaultdict
from pathlib import Path

import yaml

from . import sources
from .paths import asset_root, roots

NEEDS = {"missing", "partial", "stale"}
TRAITS = ("concept", "identity_kind", "visual_family", "species", "body_base", "lineage")


def fact(value, source: str, kind="authored") -> dict:
    return {"value": value, "source": source, "evidence": kind, "state": "unknown" if value is None or value == "unknown" else "known"}


def history(repo: Path) -> dict:
    path = repo / "tools/ambition_sprite2d_renderer/design_history/lineages.yaml"
    try:
        return yaml.safe_load(path.read_text()).get("lineages", {})
    except (OSError, ValueError, yaml.YAMLError):
        return {}


def supports(source: dict, field: str, action: str, semantics: dict):
    if field not in source:
        return None
    return action in source[field] or bool(semantics.get(action) and any(semantics.get(name) == semantics[action] for name in source[field]))


def assemble(repo: Path, product_report: dict, prepared: dict, *, local_files=None, lineages=None, runtime=None) -> dict:
    units = {uid: dict(row) for uid, row in product_report.get("units", {}).items()}
    lineages = history(repo) if lineages is None else lineages
    known_history = defaultdict(list)
    for name, row in lineages.items():
        ids = list(row.get("ids", []))
        ids += [era["id"] for era in row.get("eras", []) if era.get("id")]
        ids += [target for group in row.get("forms", {}).values() for target in group]
        for target in ids:
            known_history[target].append(name)
    archive = defaultdict(list)
    for receipt in product_report.get("receipts", []):
        generator = receipt.get("generator") or {}
        for item in receipt.get("outputs") or []:
            archive[(asset_root(repo) / receipt["root"] / item["path"]).resolve()].append({
                "generator": generator, "sha256": item.get("sha256"), "basis": receipt.get("basis"),
                "renderer": receipt.get("renderer"), "written_at": receipt.get("written_at"), "evidence": "declared",
                "lineages": known_history.get(generator.get("id"), [])})
    sheets = defaultdict(list)
    manifests = defaultdict(list)
    for path, records in prepared.get("sheets", {}).items():
        for record in records:
            published = dict(record, path=path, tier=next((r.tier for r in roots(repo) if Path(path).is_relative_to(r.path)), "unknown"))
            sheets[record["product"]].append(published)
            manifests[Path(path).name].append(published)
    claims = defaultdict(list)
    for uid, unit in units.items():
        unit["identities"] = []
        unit["game_use"] = "no-known-use" if prepared.get("state") == "available" else "unavailable"
        unit["registration"] = "registered"
        source = unit.get("source") or {}
        unit["facts"] = {}
        for name in TRAITS:
            candidates = [fact(data[name], owner) for data, owner in ((source, unit.get("generator", {}).get("source") or uid),
                (unit.get("metadata") or {}, uid + "/metadata")) if name in data and data[name] is not None]
            selected = dict(candidates[0]) if candidates else fact(None, uid)
            if len({json.dumps(item["value"], sort_keys=True) for item in candidates}) > 1:
                selected["state"] = "conflicting"
            selected["candidates"] = candidates
            unit["facts"][name] = selected
        unit["lineages"] = known_history.get(unit.get("generator", {}).get("id"), [])
        unit["published"] = [sheet for rel in unit.get("required", []) if rel.endswith("_spritesheet.ron")
                             for sheet in manifests.get(Path(rel).name, [])]
        full_key = next((sheet["key"] for sheet in unit["published"] if sheet["tier"] == "full"), None)
        preview = prepared.get("previews", {}).get(full_key, {"state": "unknown"})
        unit["preview_comparison"] = dict(preview)
        full_sheet = next((sheet for sheet in unit["published"] if sheet["tier"] == "full"), None)
        if preview.get("state") == "available" and full_sheet:
            runtime_rows = {r["name"]: r["frames"] for r in full_sheet["rows"]}
            unit["preview_comparison"].update(yaml_only=sorted(preview["rows"].keys() - runtime_rows.keys()),
                ron_only=sorted(runtime_rows.keys() - preview["rows"].keys()),
                frame_count_mismatch=sorted(name for name in preview["rows"].keys() & runtime_rows.keys() if preview["rows"][name] != runtime_rows[name]))
        for rel in unit.get("required", []) + unit.get("optional", []):
            claims[(asset_root(repo) / unit["root"] / rel).resolve()].append(uid)
        unit["runtime_defects"] = [{"tier": sheet["tier"], "path": sheet["path"],
            "missing_pages": [page["path"] for page in sheet["pages"] if not page["exists"]],
            "invalid_rows": [row["name"] for row in sheet["rows"] if not row["valid"]]}
            for sheet in unit["published"] if any(not page["exists"] for page in sheet["pages"]) or any(not row["valid"] for row in sheet["rows"])]
        unit["dependents"] = []
    pack_root = Path(prepared.get("pack_root", repo))
    for asset in prepared.get("assets", []):
        path = asset["path"]
        candidates = [(pack_root / path).resolve(), (asset_root(repo) / path).resolve()]
        owners = {uid for candidate in candidates for uid in claims.get(candidate, [])}
        art = Path(path)
        if not owners and (art.suffix.lower() in (".png", ".webp", ".svg", ".jpg", ".jpeg") or art.name.endswith(("_spritesheet.ron", "_parts.ron", "_portraits.ron"))):
            uid = "requirement:asset:" + path
            domain = next(iter(asset.get("required_by", [])), "").split("/")[0].split(":")[-1]
            kind = {"character": "actor", "held_item_art": "held_item", "wielded_item_art": "held_item"}.get(domain, domain or "unknown")
            known = archive.get((asset_root(repo) / path).resolve(), [])
            members = [art.name]
            units[uid] = {"id": uid, "kind": kind, "root": art.parent.as_posix(), "generator": {"id": path, "family": "unknown"},
                "required": members, "optional": [], "source": None, "metadata": {}, "facts": {name: fact(None, path) for name in TRAITS},
                "identities": [], "published": list(manifests.get(art.name, [])), "runtime_defects": [],
                "lineages": sorted({name for receipt in known for name in receipt["lineages"]}), "provenance": known,
                "game_use": "referenced", "registration": "unregistered" if product_report.get("complete") else "unknown",
                "dependents": [], "repair": "Register a producer for " + path,
                "status": {"state": "unknown" if any(candidate.is_file() for candidate in candidates) else "missing", "required": members,
                    "tiers": {}, "reasons": [{"code": "no-registered-producer"}]}}
            claims[(asset_root(repo) / path).resolve()].append(uid)
            owners.add(uid)
        for uid in owners:
            units[uid]["game_use"] = "referenced"
            units[uid]["dependents"].extend(asset.get("required_by", []))
    world_sources = []
    for world in prepared.get("worlds") or []:
        path = pack_root / "worlds" / world["file"]
        try:
            data = path.read_bytes()
            document = json.loads(data)
        except (OSError, ValueError) as error:
            world_sources.append({"path": str(path), "state": "unavailable", "error": str(error)})
            continue
        world_sources.append({"path": str(path), "state": "available", "sha256": hashlib.sha256(data).hexdigest()})
        for level in document.get("levels", []):
            for field in level.get("fieldInstances", []):
                if field.get("__identifier") != "parallax_theme" or not field.get("__value"):
                    continue
                theme = field["__value"]
                for uid in ("theme:" + theme, "skin:" + theme):
                    if uid in units:
                        units[uid]["game_use"] = "referenced"
                        units[uid]["dependents"].append({"world": world["id"], "room": level.get("identifier"), "field": "parallax_theme", "source": str(path)})
    identities = {}
    identities = {}
    for row in prepared.get("characters", []):
        cid = f"{row['provider']}:{row['id']}"
        catalog = row["catalog"]
        target = row.get("sheet_target")
        matched = sorted(set(claims.get((asset_root(repo) / catalog.get("manifest", "")).resolve(), [])))
        identity = dict(row, key=cid, units=matched, display=catalog.get("display_name") or catalog.get("name") or row["id"],
                        relations={name: catalog.get(name) for name in ("form_of", "part_of", "derived_from")}, coverage=[])
        for uid in matched:
            unit = units[uid]
            unit["identities"].append(cid)
            unit["game_use"] = "available-look" if catalog.get("form_of") else "referenced"
            for action in row.get("required_actions", []):
                source = unit.get("source") or {}
                for published in [sheet for sheet in unit["published"] if Path(sheet["path"]).name == Path(catalog.get("manifest", "")).name] or [{"tier": "full", "actions": {}, "rows": [], "pages": []}]:
                    resolution = published.get("actions", {}).get(action, {"requested": action, "state": "unknown", "row": None, "steps": []})
                    resolved_row = next((r for r in published["rows"] if r["name"] == resolution.get("row")), None)
                    pages_valid = bool(published["pages"]) and all(p["exists"] for p in published["pages"])
                    identity["coverage"].append({"unit": uid, "action": action, "tier": published["tier"],
                        "source_available": supports(source, "available", action, prepared.get("semantics", {})),
                        "profile_selected": supports(source, "selected", action, prepared.get("semantics", {})),
                        "resolution": resolution, "valid": bool(resolved_row and resolved_row["valid"] and pages_valid),
                        "corpse": published.get("corpse", "unknown")})
        if not matched:
            uid = "requirement:" + cid
            manifest = catalog.get("manifest", "")
            exists = any(sheet.get("tier") == "full" for sheet in sheets.get(target, []))
            units[uid] = {"id": uid, "kind": "actor", "root": "sprites", "generator": {"id": target, "family": "unknown"},
                "required": [manifest] if manifest else [], "optional": [], "metadata": {}, "source": None,
                "facts": {name: fact(None, cid) for name in TRAITS}, "published": sheets.get(target, []), "lineages": known_history.get(target, []),
                "game_use": "available-look" if catalog.get("form_of") else "referenced", "identities": [cid], "dependents": [cid],
                "registration": "unregistered" if product_report.get("complete") else "unknown",
                "repair": "Register a producer for " + str(target),
                "status": {"state": "unknown" if exists else "missing", "required": [manifest], "tiers": {},
                    "reasons": [{"code": "no-registered-producer"}]}}
            identity["units"] = [uid]
            identity["coverage"] = [{"action": action, "tier": "full", "valid": False,
                "resolution": {"state": "missing-product"}, "source_available": None, "profile_selected": None} for action in row.get("required_actions", [])]
        for move in row.get("moves", []):
            for uid in matched:
                unit = units[uid]
                source = unit.get("source") or {}
                for published in [sheet for sheet in unit["published"] if Path(sheet["path"]).name == Path(catalog.get("manifest", "")).name] or [{"tier": "full", "bindings": {}, "rows": [], "pages": []}]:
                    resolution = published.get("bindings", {}).get(move["key"], {"state": "unknown", "row": None, "steps": []})
                    resolved = next((r for r in published["rows"] if r["name"] == resolution.get("row")), None)
                    identity["coverage"].append({"unit": uid, "action": move["chain"][0], "move": move["move"], "tier": published["tier"],
                        "resolution": resolution, "valid": bool(resolved and resolved["valid"] and published["pages"] and all(p["exists"] for p in published["pages"])),
                        "source_available": move["chain"][0] in source["available"] if "available" in source else None,
                        "profile_selected": move["chain"][0] in source["selected"] if "selected" in source else None})
        identities[cid] = identity
    for uid, unit in units.items():
        unit["dependents"] = list({json.dumps(value, sort_keys=True): value for value in unit["dependents"]}.values())
        for item in (unit.get("receipt") or {}).get("outputs", []):
            claims[(asset_root(repo) / unit["root"] / item["path"]).resolve()].append(uid)
    local = []
    for path in local_files or []:
        path = Path(path).resolve()
        owners = claims.get(path, [])
        local.append({"path": str(path), "units": owners, "registration": "reference" if path.is_relative_to(repo / "assets") else "registered" if any(units[uid]["registration"] == "registered" for uid in owners) else "unregistered" if product_report.get("complete") else "unknown",
            "provenance": archive.get(path, []), "lineages": sorted({name for receipt in archive.get(path, []) for name in receipt["lineages"]})})
    concepts = defaultdict(list)
    for uid, unit in units.items():
        concept = unit["facts"]["concept"]["value"]
        if concept:
            concepts[str(concept)].append(uid)
    warnings = product_report.get("warnings", []) + prepared.get("warnings", [])
    return {"schema": "ambition.art_inspection.v1", "profile": {k: prepared.get(k) for k in ("state", "provider", "pack", "profile", "fingerprint", "cache", "sources")},
        "complete": product_report.get("complete", False) and prepared.get("state") == "available" and all(w["state"] == "available" for w in world_sources),
        "world_sources": world_sources,
        "units": units, "identities": identities, "concepts": dict(concepts), "lineages": lineages, "local_products": local,
        "runtime": runtime, "warnings": warnings, "sheet_failures": prepared.get("failures", [])}


def collect(repo: Path, out: Path, *, profile="ambition", python=None, binary=None, deep=False, runtime_path=None) -> dict:
    product_report = sources.products(repo, python, deep=deep)
    prepared = sources.content(repo, profile, out, binary)
    prepared["previews"] = sources.previews(prepared, out)
    files = [p for root in roots(repo) if root.tier == "full" for p in root.path.rglob("*") if p.is_file()]
    runtime = json.loads(runtime_path.read_text()) if runtime_path else None
    report = assemble(repo, product_report, prepared, local_files=files, runtime=runtime)
    import subprocess
    revisions = {}
    for name, path in (("game", repo), ("renderer", repo / "tools/ambition_sprite2d_renderer")):
        sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=path, capture_output=True, text=True)
        dirty = subprocess.run(["git", "status", "--porcelain", "--untracked-files=no"], cwd=path, capture_output=True, text=True)
        revisions[name] = {"commit": sha.stdout.strip() or None, "local_changes": dirty.stdout.splitlines()}
    report["revisions"] = revisions
    return report


def select(report: dict, *, query="", kind="", needs=False, game_only=False, lineage="", missing="") -> list[dict]:
    kinds = set(kind.split(",")) if kind else set()
    result = []
    for uid, unit in report["units"].items():
        searchable = {k: unit.get(k) for k in ("id", "kind", "facts", "generator", "lineages", "source")}
        if query and query.lower() not in json.dumps(searchable, sort_keys=True).lower() and not any(query.lower() in json.dumps(report["identities"][cid]).lower() for cid in unit["identities"]):
            continue
        if kinds and unit["kind"] not in kinds:
            continue
        if game_only and unit["game_use"] not in ("referenced", "available-look"):
            continue
        status = unit.get("status", {})
        if needs and not unit.get("runtime_defects") and status.get("state") not in NEEDS and not any(t.get("state") in NEEDS for t in status.get("tiers", {}).values()):
            continue
        if lineage and lineage not in unit["lineages"]:
            continue
        if missing and not any(c["action"] == missing and (not c["valid"] or c["resolution"]["state"] not in ("direct", "alias")) for cid in unit["identities"] for c in report["identities"][cid]["coverage"]):
            continue
        result.append(unit)
    return result


def summary(rows: list[dict]) -> dict:
    return dict(Counter(row.get("status", {}).get("state", "unknown") for row in rows))
