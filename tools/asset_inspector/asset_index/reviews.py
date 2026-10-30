"""Durable review notes for an exact version of a character or any other asset."""

from __future__ import annotations

import datetime as dt
import hashlib
import json
import sys
import uuid
from pathlib import Path

SCHEMA = "ambition.art_review.v1"
PRIORITIES = ("high", "normal", "low")
ASSETS = "game/ambition_content_library/generated/runtime"


class ConflictError(ValueError):
    """The reviewed art or note changed before a save."""


def subjects(catalog: dict) -> dict:
    """Identify art by product bytes, independent of unrelated catalog edits."""
    inventory = catalog.get("inventory") or {}
    result = {}
    for key, actor in inventory.get("identities", {}).items():
        units = [inventory["units"][uid] for uid in actor["units"]]
        paths = {unit["root"] + "/" + path for unit in units for path in unit["required"] + unit.get("optional", [])}
        products = [{"path": path, "sha256": catalog["products"].get(path, {}).get("sha256")} for path in sorted(paths)]
        identity = {"manifest": actor["catalog"].get("manifest"), "products": products,
                    "bundle_ids": {unit["id"]: unit["bundle"] for unit in units if unit.get("bundle")}}
        generation = hashlib.sha256(json.dumps(identity, sort_keys=True).encode()).hexdigest()
        result[key] = dict(identity, generation_id=generation, character=key, display=actor["display"],
                           content_fingerprint=inventory["profile"].get("fingerprint"),
                           lineages=sorted({name for unit in units for name in unit["lineages"]}),
                           bundles=[{"id": unit["id"], "source": unit["generator"].get("source"), "repair": unit["repair"]} for unit in units])
    return result


def item_subjects(catalog: dict) -> dict:
    """Music, sound, fonts and worlds: an item's version is the size and mtime of its files."""
    result = {}
    for key, item in (catalog.get("items") or {}).items():
        stamp = item.get("stamp") or []
        identity = {"item": key, "stamp": stamp}
        result[key] = {"key": key, "character": key, "kind": item["kind"], "display": item["display"], "stamp": stamp,
                       "generation_id": hashlib.sha256(json.dumps(identity, sort_keys=True).encode()).hexdigest(),
                       "products": [], "lineages": [], "bundles": []}
    return result


def all_subjects(catalog: dict) -> dict:
    """Every reviewable subject: characters, then music, sound, fonts and worlds."""
    return item_subjects(catalog) | subjects(catalog)


def check_products(repo: Path, subject: dict) -> None:
    if "stamp" in subject:
        from .kinds import _stat
        if any(_stat(repo / path) != [size, mtime] for path, size, mtime in subject["stamp"]):
            raise ConflictError("The asset changed. Rebuild the inspector before adding this note.")
        return
    sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "scripts"))
    from lib.art_publication import resolve, store
    from .paths import library_path, asset_root
    publication = store(library_path(repo))
    current = publication.selected() if publication else {"bundles": {}}
    if any(current["bundles"].get(unit) != key for unit, key in subject.get("bundle_ids", {}).items()):
        raise ConflictError("The published art changed. Rebuild the inspector before adding this note.")
    for product in subject["products"]:
        path = resolve(library_path(repo), product["path"]) if subject.get("bundle_ids") else asset_root(repo) / product["path"]
        digest = None
        if path is not None and path.is_file():
            with path.open("rb") as stream:
                digest = hashlib.file_digest(stream, "sha256").hexdigest()
        if digest != product["sha256"]:
            raise ConflictError("The published art changed. Rebuild the inspector before adding this note.")


class Store:
    def __init__(self, repo: Path):
        self.root = repo / "tools" / "asset_inspector" / "reviews" / "notes"

    def path(self, note_id: str) -> Path:
        return self.root / (str(uuid.UUID(note_id)) + ".json")

    def load(self) -> list[dict]:
        rows = []
        for path in sorted(self.root.glob("*.json")):
            row = json.loads(path.read_text())
            if row.get("schema") != SCHEMA or row.get("status") not in ("open", "resolved"):
                raise ValueError(f"Invalid art review: {path}")
            rows.append(row)
        return rows

    def write(self, note: dict) -> dict:
        self.root.mkdir(parents=True, exist_ok=True)
        path = self.path(note["id"])
        temp = path.with_suffix("." + uuid.uuid4().hex + ".tmp")
        try:
            temp.write_text(json.dumps(note, indent=2, ensure_ascii=False) + "\n")
            temp.replace(path)
        finally:
            temp.unlink(missing_ok=True)
        return note

    def add(self, subject: dict, text: str, *, priority="normal", animation="") -> dict:
        text = text.strip()
        if not text or len(text) > 20000:
            raise ValueError("A note must contain 1 to 20000 characters.")
        if priority not in PRIORITIES:
            raise ValueError("Priority must be high, normal, or low.")
        now = dt.datetime.now(dt.timezone.utc).isoformat()
        return self.write({"schema": SCHEMA, "id": str(uuid.uuid4()), "subject": subject,
                           "text": text, "animation": animation.strip(), "priority": priority,
                           "status": "open", "resolution": "", "created_at": now, "updated_at": now})

    def update(self, note_id: str, status: str, resolution="", expected_updated_at=None) -> dict:
        if status not in ("open", "resolved"):
            raise ValueError("Status must be open or resolved.")
        note = json.loads(self.path(note_id).read_text())
        if expected_updated_at and note["updated_at"] != expected_updated_at:
            raise ConflictError("The note changed. Reload its latest state before saving.")
        note.update(status=status, resolution=resolution or note["resolution"], updated_at=dt.datetime.now(dt.timezone.utc).isoformat())
        return self.write(note)


def snapshot(repo: Path, catalog: dict) -> dict:
    current = all_subjects(catalog)
    notes = Store(repo).load()
    for note in notes:
        subject = current.get(note["subject"]["character"])
        note["generation_relation"] = ("current" if subject["generation_id"] == note["subject"]["generation_id"] else "older") if subject else "unavailable"
    queue = []
    for key in sorted({note["subject"]["character"] for note in notes if note["status"] == "open"}):
        open_notes = sorted([note for note in notes if note["status"] == "open" and note["subject"]["character"] == key],
                            key=lambda note: (PRIORITIES.index(note["priority"]), note["created_at"]))
        queue.append({"character": key, "display": (current.get(key) or open_notes[0]["subject"])["display"],
                      "current_art": current.get(key), "notes": open_notes})
    queue.sort(key=lambda group: (PRIORITIES.index(group["notes"][0]["priority"]), group["character"]))
    return {"catalog_built_at": catalog.get("built_at"), "subjects": current, "notes": notes,
            "backlog": {"schema": "ambition.art_backlog.v1", "open_count": sum(len(group["notes"]) for group in queue), "characters": queue}}


def read_catalog(out: Path) -> dict:
    path = out / "catalog.js"
    if not path.is_file():
        raise ValueError("Build the inspector first: python3 scripts/asset_inspector.py build")
    return json.loads(path.read_text().removeprefix("window.ART_CATALOG = ").rstrip().removesuffix(";"))


def cli(repo: Path, out: Path, args) -> int:
    try:
        store = Store(repo)
        if args.action in ("resolve", "reopen"):
            result = store.update(args.id or "", "resolved" if args.action == "resolve" else "open", args.resolution)
        else:
            catalog = read_catalog(out)
            report = snapshot(repo, catalog)
            if args.action == "add":
                subject = report["subjects"].get(args.id)
                if not subject:
                    raise ValueError("Use a full catalog identity, such as ambition:npc_general or music:ai_dente.")
                check_products(repo, subject)
                result = store.add(subject, args.text, priority=args.priority, animation=args.animation)
            elif args.action == "backlog":
                result = report["backlog"]
            else:
                result = report["notes"]
        if args.json:
            print(json.dumps(result, indent=2, ensure_ascii=False))
        elif args.action == "backlog":
            print(f"{result['open_count']} open notes across {len(result['characters'])} characters")
            for group in result["characters"]:
                print(f"\n{group['display']} ({group['character']})")
                for note in group["notes"]:
                    print(f"  [{note['priority']}] {note['id']} ({note['generation_relation']} generation): {note['text']}")
        else:
            for note in result if isinstance(result, list) else [result]:
                print(f"{note['id']} [{note['status']}] {note['subject']['character']}: {note['text']}")
        return 0
    except (ValueError, OSError) as error:
        print(f"art reviews: {error}", file=sys.stderr)
        return 2
