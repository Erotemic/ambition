"""Prepare generated products from authored rooms and prepared character bindings."""
from __future__ import annotations

from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import shlex
import subprocess
import sys
import hashlib
import os

from . import sources
from .paths import library, library_path

SCHEMA = "ambition.asset_preflight.v1"


def content(repo: Path, profile: str, binary: Path | None = None) -> dict:
    binary = binary or sources.default_binary(repo)
    if not binary.is_file():
        raise ValueError(f"Content tool not found: {binary}. Build it once: {sources.BUILD_BINARY}")
    done = subprocess.run([str(binary), str(sources.pack_dir(repo, profile)), "--inspect-art", "--no-asset-check"],
                          cwd=repo, capture_output=True, text=True)
    if done.returncode:
        raise ValueError(f"Content preparation failed:\n{done.stderr[-4000:]}")
    report = json.loads(done.stdout)
    if not report.get("scope"):
        raise ValueError(f"The content tool has no asset-scope export. Build it once: {sources.BUILD_BINARY}")
    return report


def walk(value, path=""):
    if isinstance(value, dict):
        for key, child in value.items():
            site = f"{path}.{key}" if path else key
            yield key, child, site
            yield from walk(child, site)
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from walk(child, f"{path}[{index}]")


def prepare(prepared: dict, products: dict, *, worlds=(), rooms=(), characters=(), effects=(), purpose="play", owner="") -> dict:
    scope = prepared.get("scope") or {}
    known_rooms = scope.get("rooms") or {}
    known_characters = {row["id"]: row for row in prepared.get("characters", [])}
    definitions = products.get("units", {})
    diagnostics = []
    selected_rooms = set(rooms)
    for world in worlds:
        selected = [rid for rid, row in known_rooms.items()
                    if world in {row["world"], Path(row["source"]).stem, Path(row["source"]).name}]
        if not selected:
            diagnostics.append({"code": "unknown-world", "subject": world, "details": scope.get("room_failures", [])})
        selected_rooms.update(selected)
    requested_characters = defaultdict(set)
    for cid in characters:
        requested_characters[cid].add("request.character")
    wanted = defaultdict(list)
    references = []
    missing_paths = defaultdict(set)
    missing_effects = defaultdict(set)
    unverified_effects = defaultdict(set)
    paths = defaultdict(set)
    semantics = defaultdict(set)
    for uid, unit in definitions.items():
        for path in unit.get("required", []):
            paths[unit["root"] + "/" + path].add(uid)
        for name in unit.get("semantic_effects", []):
            semantics[name].add(uid)

    def require(uid, use, source, logical=None):
        if uid not in definitions:
            diagnostics.append({"code": "unknown-producer", "subject": uid, "source": source, "use": use})
            return
        row = {"source": source, "use": use, "logical_path": logical}
        if row in wanted[uid]:
            return
        wanted[uid].append(row)
        references.append(dict(row, unit=uid))
        for dependency in definitions[uid].get("dependencies", []):
            require("target:" + dependency, use, uid)
        for dependency in definitions[uid].get("gameplay_targets", []):
            require("target:" + dependency, "gameplay", uid)

    def path_ref(path, use, source):
        candidates = paths.get(path, set())
        if len(candidates) == 1:
            uid = next(iter(candidates))
            require(uid, use, source, path)
            if 'publication' in products and definitions[uid].get('status', {}).get('state') != 'missing' and path not in products['publication'].get('files', {}):
                missing_paths[uid].add(path)
        else:
            diagnostics.append({"code": "ambiguous-producer" if candidates else "unresolved-product", "subject": path,
                                "source": source, "use": use, "candidates": sorted(candidates)})

    def prop_ref(name, source, platform=False):
        target = ((scope.get("platforms") or {}).get(name) if platform else None) or (scope.get("props") or {}).get(name)
        item = (scope.get("world_items") or {}).get(name)
        if target:
            path_ref("sprites/" + target + "_spritesheet.ron", "presentation", source)
        elif item:
            path_ref(item, "presentation", source)
        else:
            diagnostics.append({"code": "unknown-prop", "subject": name, "source": source, "use": "presentation"})

    def effect_ref(name, source):
        candidates = semantics.get(name, set())
        if len(candidates) == 1:
            uid = next(iter(candidates))
            require(uid, "gameplay", source)
            if definitions[uid].get('status', {}).get('state') != 'missing':
                accepted = definitions[uid].get('accepted_effects')
                if accepted is None:
                    unverified_effects[uid].add(name)
                elif name not in accepted:
                    missing_effects[uid].add(name)
        else:
            diagnostics.append({"code": "ambiguous-effect" if candidates else "unknown-effect", "subject": name,
                                "source": source, "candidates": sorted(candidates)})

    optional_media = []
    for rid in sorted(selected_rooms):
        if rid not in known_rooms:
            diagnostics.append({"code": "unknown-room", "subject": rid, "details": scope.get("room_failures", [])})
            continue
        spec = known_rooms[rid]["spec"]
        for prop in spec.get("props", []):
            prop_ref(prop["kind"], f"room:{rid}/prop:{prop['id']}")
        for key, value, site in walk(spec):
            if not isinstance(value, str) or not value:
                continue
            source = f"room:{rid}/{site}"
            if key == "character_id":
                requested_characters[value].add(source)
            elif key in {"sprite", "look", "visual"}:
                prop_ref(value, source, platform=key == "visual")
            elif key == "held_item":
                path = (scope.get("held_items") or {}).get(value)
                if path:
                    path_ref(path, "presentation", source)
                else:
                    diagnostics.append({"code": "unknown-held-item-art", "subject": value, "source": source, "use": "presentation"})
        for trigger in spec.get("encounter_triggers", []):
            for cid in (scope.get("encounters") or {}).get(trigger["id"], []):
                requested_characters[cid].add(f"room:{rid}/encounter:{trigger['id']}")
        for spawn in spec.get("boss_spawns", []):
            brain = spawn["payload"]
            bid = (brain.get("Custom") or brain.get("PhaseScript", {}).get("script_id")) if isinstance(brain, dict) else None
            if bid:
                filename = (scope.get("bosses") or {}).get(bid)
                if filename:
                    path_ref("sprites/" + filename, "presentation", f"room:{rid}/boss:{bid}")
                else:
                    diagnostics.append({"code": "unknown-boss-art", "subject": bid, "source": rid, "use": "gameplay"})
        metadata = spec.get("metadata") or {}
        theme = known_rooms[rid].get("theme")
        for prefix, name in (("theme", theme), ("skin", theme)):
            if name:
                require(prefix + ":" + name, "presentation", f"room:{rid}/metadata")
        for field in ("music_track", "fight_music_track"):
            cue = metadata.get(field)
            if cue:
                optional_media.append({"use": "optional-media", "source": f"room:{rid}/{field}", "cue": cue,
                                       "logical_path": (scope.get("music") or {}).get(cue),
                                       "repair": "./scripts/regen/music.sh --cue " + shlex.quote(cue)})
    for cid, sites in sorted(requested_characters.items()):
        row = known_characters.get(cid)
        if not row:
            diagnostics.append({"code": "unknown-character", "subject": cid, "sources": sorted(sites)})
            continue
        for source in sorted(sites):
            path_ref(row["catalog"]["manifest"], "presentation", source)
        for move in row.get("moves", []):
            for name in move.get("vfx", []):
                effect_ref(name, move["key"] + '/vfx')
            for effect in move.get("effects", []):
                # The move owner exports these references after borrow and override resolution.
                for key, value, site in walk(effect.get("params", {})):
                    if key in {"effect", "vfx", "sustain_effect"} and isinstance(value, str):
                        effect_ref(value, move["key"] + "/" + site)
        for key, value, site in walk(row["catalog"].get("cues") or {}):
            if isinstance(value, str):
                optional_media.append({"use": "optional-media", "source": f"character:{cid}/cues.{site}", "cue": value})
    for name in effects:
        effect_ref(name, "request.effect")
    result_units = {}
    for uid, reasons in sorted(wanted.items()):
        unit = definitions[uid]
        uses = sorted({reason["use"] for reason in reasons})
        required = "gameplay" in uses or purpose != "headless"
        state = unit.get("status", {}).get("state", "unknown")
        if state != 'missing' and (missing_paths[uid] or missing_effects[uid]):
            state = 'partial'
        if state != "missing" and any(row.get("state") in {"missing", "partial"}
                                     for row in unit.get("status", {}).get("tiers", {}).values()):
            state = "partial"
        repair = unit.get("repair")
        result_units[uid] = {"id": uid, "library": owner, "uses": uses, "required": required,
                             "availability": state, "repair": repair,
                             "logical_paths": sorted(unit["root"] + "/" + path for path in unit.get("required", [])),
                             "reasons": reasons, "tiers": unit.get("tiers", []),
                             "dynamic_members": unit.get("dynamic_members", False)}
        result_units[uid].update(missing_paths=sorted(missing_paths[uid]), missing_effects=sorted(missing_effects[uid]),
                                 unverified_effects=sorted(unverified_effects[uid]))
    needed = [row for row in result_units.values() if row["required"]]
    missing = [row["id"] for row in needed if row["availability"] == "missing"]
    partial = [row["id"] for row in needed if row["availability"] == "partial"]
    for diagnostic in diagnostics:
        diagnostic['severity'] = 'warning' if purpose == 'headless' and diagnostic.get('use') == 'presentation' else 'error'
    errors = any(row['severity'] == 'error' for row in diagnostics)
    return {"schema": SCHEMA, "purpose": purpose, "content_fingerprint": prepared.get("fingerprint"),
            "scope_fingerprint": hashlib.sha256(json.dumps({rid: {key: known_rooms.get(rid, {}).get(key) for key in ('world', 'spec')}
                                                            for rid in sorted(selected_rooms)}, sort_keys=True).encode()).hexdigest(),
            "publication": products.get("publication", {}).get("generation"), "library": owner,
            "rooms": sorted(selected_rooms), "characters": sorted(requested_characters), "units": result_units,
            "references": references, "optional_media": optional_media, "diagnostics": diagnostics,
            "missing": missing, "partial": partial,
            "stale": [row["id"] for row in needed if row["availability"] == "stale"],
            "ready": not errors and not missing and not partial and all(row["availability"] != "not-checked" and not row['unverified_effects'] for row in needed),
            "complete": not errors,
            "generation_limit": "Each sprite target builds all its supported quality tiers. Gameplay-only targets publish no image payloads. Optional media is not generated."}


def collect(repo: Path, *, profile="ambition", binary=None, python=None, **selection) -> dict:
    with ThreadPoolExecutor(max_workers=2) as workers:
        needs_content = any(selection.get(key) for key in ('worlds', 'rooms', 'characters'))
        prepared = workers.submit(content, repo, profile, binary) if needs_content else None
        products = workers.submit(sources.products, repo, python, catalog=True)
        report = products.result()
        if not report.get("units"):
            raise ValueError("Product discovery failed: " + "; ".join(report.get("warnings", [])))
        prepared = prepared.result() if prepared else {}
        owner = library(repo)["id"]
        plan = prepare(prepared, report, owner=owner, **selection)
        inspect = [uid for uid, unit in plan["units"].items()
                   if unit["required"] and unit["availability"] == "not-checked"]
        if not inspect:
            return plan
        availability = sources.products(repo, python, requested=inspect)
        absent = set(inspect) - availability.get("units", {}).keys()
        if absent:
            raise ValueError(f"Product availability failed for {sorted(absent)}: {availability.get('warnings', [])}")
        report["units"].update(availability["units"])
        report["publication"] = availability["publication"]
        return prepare(prepared, report, owner=owner, **selection)


def generate_missing(repo: Path, report: dict, *, python=None) -> list[str]:
    if not report["complete"]:
        raise ValueError("Resolve all preflight diagnostics before generation.")
    targets = [uid.removeprefix("target:") for uid in report["missing"] if uid.startswith("target:")]
    commands = []
    environment = dict(os.environ)
    if python:
        environment['AMBITION_SPRITE_PYTHON'] = python
    if targets:
        command = [str(repo / "scripts/regen/sprites.sh"), "--missing-only"]
        for target in targets:
            command += ["--target", target]
        subprocess.run(command, cwd=repo, check=True, stdout=sys.stderr, env=environment)
        commands.extend("target:" + target for target in targets)
    scenery = [uid for uid in report["missing"] if uid.startswith(("theme:", "skin:"))]
    if scenery:
        command = [str(repo / "scripts/regen/backgrounds.sh"), "--missing-only"]
        for uid in scenery:
            prefix, name = uid.split(":", 1)
            command += ["--" + prefix, name]
        subprocess.run(command, cwd=repo, check=True, stdout=sys.stderr, env=environment)
        commands.extend(scenery)
    return commands
