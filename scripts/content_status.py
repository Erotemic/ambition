#!/usr/bin/env python3
"""Tell which generated content is present, missing or stale. Fast, and it does not refuse.

This script loads no game, needs no cargo, no renderer toolchain and no
submodule. It uses only the Python standard library. It reads:

* the library manifest (`library.json`) and its producer families,
* the accepted publication (`published.json` in the store) and the manifest of
  each selected bundle,
* the library sources that each game pack (`game/*/assets/pack.ron`) declares.

A family is present when the store has an accepted unit for it, or when each
root that its producer declares has a loose copy under a generated asset root.
A present family is stale when an input that its bundle recipe records has
changed since the bundle was accepted. The digest cache of the producers
(`producer-inputs.json`) is read, not written, so a file that did not change is
not hashed again.

The exit status is 0, also when content is missing: missing content does not
stop the game. Use `--require-complete` to get status 1 when a family is
missing or stale.

This is the fast question. The exact producer check is
`python3 scripts/generate_library_content.py --check`, which needs each
renderer toolchain. To make what is missing, run
`scripts/setup/generated_content.sh`.

Examples:
    python3 scripts/content_status.py
    python3 scripts/content_status.py --family music --json
    python3 scripts/content_status.py --require-complete
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_LIBRARY = "game/ambition_content_library/library.json"
SCHEMA = "ambition.content_status.v1"
MAKE = "scripts/setup/generated_content.sh"
# The roots that the sprite renderer owns in a publication
# (`publication_store.MANAGED_ROOTS`). Loose copies under them have no authority.
MANAGED_ROOTS = ("sprites", "sprites_0_5x", "sprites_0_25x", "sprites_potato", "sprite_packs")


def relative(value: str) -> str:
    path = PurePosixPath(value)
    if not value or path.is_absolute() or ".." in path.parts or "\\" in value:
        raise ValueError(f"Invalid library path: {value!r}")
    return value


def repository_of(path: Path) -> Path:
    """The checkout that holds `path`. Recipe inputs are relative to it."""
    for parent in path.resolve().parents:
        if (parent / ".git").exists():
            return parent
    return ROOT


def display(path: Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def declared_roots(config: dict) -> list[str]:
    """The logical roots a producer declares: its `roots` and its `registry`."""
    roots = list(config.get("roots") or [])
    if config.get("registry"):
        roots.append(config["registry"])
    return list(dict.fromkeys(roots))


def publishes(family: str, unit: str) -> bool:
    """True when publication unit `unit` is a product of `family`.

    The same rule as `ContentLibrary::missing_families` in Rust. The sprite
    renderer publishes the `sprites` family as `target:` and `atlas:` units.
    """
    producer = "producer:" + family
    return (unit == producer or unit.startswith(producer + ":")
            or (family == "sprites" and unit.startswith(("target:", "atlas:"))))


class Digests:
    """Read-only use of the producers' digest cache: hash only a changed file."""

    def __init__(self, path: Path):
        try:
            self.cache = json.loads(path.read_text())
        except (OSError, ValueError):
            self.cache = {}
        self.seen: dict[str, str | None] = {}

    def file(self, path: Path) -> str | None:
        key = str(path.resolve())
        if key in self.seen:
            return self.seen[key]
        try:
            st = os.stat(key)
        except OSError:
            self.seen[key] = None
            return None
        # Size and mtime only: the tree is shared between machines (virtiofs), where
        # device, inode and ctime differ for one file and would rehash every input.
        signature = [st.st_size, st.st_mtime_ns]
        old = self.cache.get(key)
        if old and old[0] == signature:
            digest = old[1]
        else:
            with open(key, "rb") as stream:
                digest = hashlib.file_digest(stream, "sha256").hexdigest()
        self.seen[key] = digest
        return digest


def recorded_inputs(manifest: dict) -> list[dict] | None:
    """The `{path, sha256}` inputs that a bundle recipe records, or None."""
    recipe = manifest.get("recipe") or {}
    for inputs in ((recipe.get("generation") or {}).get("inputs"), recipe.get("inputs")):
        if isinstance(inputs, list) and all(isinstance(row, dict) and "path" in row and "sha256" in row for row in inputs):
            return inputs
    return None


def read_selection(store: Path | None) -> dict | None:
    if store is None:
        return None
    path = store / "published.json"
    if not path.is_file():
        return None
    return json.loads(path.read_text())


def library_status(manifest_path: Path, families: set[str] | None, check_stale: bool) -> dict:
    manifest = json.loads(manifest_path.read_text())
    base = manifest_path.parent
    repository = repository_of(manifest_path)
    store = base / relative(manifest["publication"]) if manifest.get("publication") else None
    selection = read_selection(store)
    bundles = (selection or {}).get("bundles", {})
    generated = [base / relative(root) for role, root in manifest["asset_roots"].items() if role != "authored"]
    digests = Digests(store / "producer-inputs.json") if store and check_stale else None
    result = {
        "id": manifest["id"],
        "manifest": display(manifest_path),
        "store": {
            "path": display(store / "published.json") if store else None,
            "present": selection is not None,
            "generation": (selection or {}).get("generation"),
        },
        "families": {},
    }
    for family, config in manifest.get("producers", {}).items():
        if families is not None and family not in families:
            continue
        units = sorted(unit for unit in bundles if publishes(family, unit))
        roots = declared_roots(config)
        loose = bool(roots) and all(any((root_dir / root).exists() for root_dir in generated) for root in roots)
        row = {"state": "present" if units or loose else "missing", "units": len(units), "loose": loose,
               "stale_units": [], "unchecked_units": 0}
        if digests is not None:
            for unit in units:
                try:
                    bundle = json.loads((store / "bundles" / bundles[unit] / "manifest.json").read_text())
                except (OSError, ValueError):
                    row["stale_units"].append({"unit": unit, "reason": "bundle manifest is absent or not valid"})
                    continue
                inputs = recorded_inputs(bundle)
                if inputs is None:
                    row["unchecked_units"] += 1
                    continue
                changed = [item["path"] for item in inputs if digests.file(repository / item["path"]) != item["sha256"]]
                if changed:
                    row["stale_units"].append({"unit": unit, "reason": f"{len(changed)} of {len(inputs)} recorded inputs changed",
                                               "changed": changed[:5]})
            if row["state"] == "present" and row["stale_units"]:
                row["state"] = "stale"
        result["families"][family] = row
    return result


SOURCE = re.compile(r'\(\s*path:\s*"([^"]+)"\s*,\s*library:\s*Some\(\s*"([^"]+)"\s*\)')
LIBRARIES = re.compile(r'libraries:\s*\[([^\]]*)\]')


def source_status(pack: Path, logical: str, library_id: str) -> dict:
    """Where one declared library source of a pack is."""
    text = pack.read_text()
    row = {"pack": display(pack), "path": logical, "library": library_id, "state": "missing", "producer": None}
    match = LIBRARIES.search(text)
    for declared in re.findall(r'"([^"]+)"', match.group(1) if match else ""):
        manifest_path = (pack.parent / declared).resolve()
        if not manifest_path.is_file():
            continue
        manifest = json.loads(manifest_path.read_text())
        if manifest.get("id") != library_id:
            continue
        base = manifest_path.parent
        roots = manifest["asset_roots"]
        selection = read_selection(base / relative(manifest["publication"])) if manifest.get("publication") else None
        managed = set(MANAGED_ROOTS) | set((selection or {}).get("roots", [])) if selection else set()
        if (base / relative(roots["authored"]) / logical).is_file() if "authored" in roots else False:
            row["state"] = "authored"
        elif selection and logical in selection.get("files", {}):
            row["state"] = "published"
        elif not any(logical == root or logical.startswith(root + "/") for root in managed) and any(
                (base / relative(root) / logical).is_file() for role, root in roots.items() if role != "authored"):
            row["state"] = "loose"
        row["producer"] = next((family for family, config in manifest.get("producers", {}).items()
                                if any(logical == root or logical.startswith(root + "/") for root in declared_roots(config))), None)
        if row["state"] == "missing" and row["producer"] is None:
            row["state"] = "missing-authored"
        return row
    row["state"] = "undeclared-library"
    return row


def pack_sources(repository: Path, families: set[str] | None) -> list[dict]:
    rows = []
    for pack in sorted(repository.glob("game/*/assets/pack.ron")):
        for logical, library_id in SOURCE.findall(pack.read_text()):
            row = source_status(pack, logical, library_id)
            if families is None or row["producer"] in families:
                rows.append(row)
    return rows


def status(library: Path, families: set[str] | None, check_stale: bool) -> dict:
    lib = library_status(library, families, check_stale)
    sources = pack_sources(repository_of(library), families)
    missing = [name for name, row in lib["families"].items() if row["state"] == "missing"]
    stale = [name for name, row in lib["families"].items() if row["state"] == "stale"]
    broken = [f"{row['pack']}: {row['path']}" for row in sources if row["state"] in ("missing-authored", "undeclared-library")]
    return {"schema": SCHEMA, "libraries": [lib], "sources": sources, "missing": missing, "stale": stale,
            "authoring_errors": broken, "complete": not (missing or stale or broken), "make": MAKE}


def render(report: dict) -> str:
    lines = []
    for lib in report["libraries"]:
        store = lib["store"]
        lines.append(f"content library {lib['id']} ({lib['manifest']})")
        lines.append(f"  store: {store['path']} ({'present' if store['present'] else 'absent'})")
        for family, row in lib["families"].items():
            detail = []
            if row["units"]:
                detail.append(f"{row['units']} accepted unit(s)")
            if row["loose"]:
                detail.append("loose roots")
            if row["unchecked_units"]:
                detail.append(f"{row['unchecked_units']} unit(s) without a recipe to check")
            for stale in row["stale_units"][:3]:
                detail.append(f"stale {stale['unit']}: {stale['reason']}")
            if row["state"] == "missing":
                detail.append("no accepted unit and no loose copy")
            lines.append(f"  {family:<12} {row['state']:<8} {'; '.join(detail)}")
    if report["sources"]:
        lines.append("library sources that packs declare")
        for row in report["sources"]:
            note = {"missing": f"not generated ({row['producer']}); the pack compiles without it",
                    "missing-authored": "ABSENT AUTHORED FILE: the pack does not compile",
                    "undeclared-library": "the pack does not import this library"}.get(row["state"], "")
            lines.append(f"  {row['pack']}: {row['path']:<28} {row['state']:<9} {note}".rstrip())
    if report["missing"] or report["stale"]:
        parts = []
        if report["missing"]:
            parts.append(f"{len(report['missing'])} generated famil{'y' if len(report['missing']) == 1 else 'ies'} missing ({', '.join(report['missing'])})")
        if report["stale"]:
            parts.append(f"{len(report['stale'])} stale ({', '.join(report['stale'])})")
        lines.append("summary: " + "; ".join(parts) + f". The game runs without them. Make them: {report['make']}")
    else:
        lines.append("summary: the generated content is present and current")
    for error in report["authoring_errors"]:
        lines.append(f"error: {error}")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--json", action="store_true", help="write the report as JSON")
    parser.add_argument("--family", action="append", help="report only this producer family (repeatable)")
    parser.add_argument("--library", default=os.environ.get("AMBITION_CONTENT_LIBRARY", DEFAULT_LIBRARY),
                        help="library manifest, relative to the repository root")
    parser.add_argument("--no-stale", action="store_true", help="do not compare recorded recipe inputs")
    parser.add_argument("--require-complete", action="store_true",
                        help="exit 1 when a family is missing or stale, or an authored source is absent")
    args = parser.parse_args(argv)
    report = status((ROOT / args.library).resolve(), set(args.family) if args.family else None, not args.no_stale)
    print(json.dumps(report, indent=1, sort_keys=True) if args.json else render(report))
    return int(args.require_complete and not report["complete"])


if __name__ == "__main__":
    raise SystemExit(main())
