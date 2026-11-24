"""Asset reviews: one feedback format for every asset kind.

The format is the music review of the music tool
(`ambition_music_renderer.music_reviews`, schema `ambition.music_review.v2`):

* a score from 1 to 10, with the label of its rubric band;
* notes and issue tags;
* the exact version that was reviewed, one file for each version;
* an edit replaces `current` and does not add a new review.

Music reviews stay with the music tool, so its review GUI and this page read and
write the same files. This module writes the other kinds (characters and
bosses, images such as props and effects, sound effects, fonts and worlds)
with the same keys under the schema `ambition.asset_review.v1`. It reads both.

A review can name a part of its subject: an animation of a character or a
sheet, or a room of a world. Each (subject, version, part) has one file.
"""

from __future__ import annotations

import datetime as dt
import hashlib
import json
import math
import re
import sys
from pathlib import Path

import yaml

_Loader = getattr(yaml, "CSafeLoader", yaml.SafeLoader)

SCHEMA = "ambition.asset_review.v1"
MUSIC_SCHEMA_V1 = "ambition.music_review.v1"
MUSIC_SCHEMA = "ambition.music_review.v2"
SNAPSHOT_SCHEMA = "ambition.asset_reviews.v1"
ASSETS = "game/ambition_content_library/generated/runtime"

#: The music tool's scale and rubric. `tests/test_reviews.py` compares them with
#: `ambition_music_renderer.music_reviews`, so the two tools cannot drift apart.
MIN_SCORE = 1.0
MAX_SCORE = 10.0
POLISH_THRESHOLD = 6.0
RUBRIC_BANDS = (
    (1.0, 2.99, "Replace", "Placeholder or actively weak; prefer a rewrite."),
    (3.0, 4.99, "Major polish", "Works only provisionally; prioritize substantial revision."),
    (5.0, 6.99, "Acceptable", "Good enough for now, but an obvious later polish candidate."),
    (7.0, 8.99, "Strong", "Ship-quality; change only for a concrete reason."),
    (9.0, 10.0, "Standout", "Benchmark/favorite; preserve its identity and use as a quality reference."),
)

_VISUAL = ("design", "silhouette-readability", "proportions-anatomy", "palette-lighting",
           "animation-motion", "style-consistency", "game-fit", "technical-render")
#: Issue tags of each kind. Music uses the music tool's own tags (`kinds.music`).
ISSUE_TAGS = {
    "character": _VISUAL,
    "image": _VISUAL,
    "sfx": ("sound-design", "timing-envelope", "mix-level", "repetition-variety", "game-fit", "technical-render"),
    "font": ("legibility", "style-fit", "glyph-coverage", "game-fit", "technical-render"),
    "world": ("layout-flow", "visual-composition", "challenge", "navigation", "game-fit", "technical-render"),
}
#: Kinds whose page plays the subject: their review keeps how far it was heard.
AUDIO_KINDS = {"sfx", "music"}
MAX_NOTES = 20000


class ConflictError(ValueError):
    """The reviewed asset changed after the page was built."""


def normalize_score(value) -> float:
    try:
        score = float(value)
    except (TypeError, ValueError) as error:
        raise ValueError(f"The score must be a number, not {value!r}.") from error
    if not math.isfinite(score) or not MIN_SCORE <= score <= MAX_SCORE:
        raise ValueError(f"The score must be from {MIN_SCORE:g} to {MAX_SCORE:g}, not {value!r}.")
    return round(score, 2)


def score_label(value) -> str:
    score = normalize_score(value)
    return next((label for low, high, label, _ in RUBRIC_BANDS if low <= score <= high), RUBRIC_BANDS[-1][2])


def utc_now_iso() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def _digest(identity) -> str:
    return hashlib.sha256(json.dumps(identity, sort_keys=True).encode()).hexdigest()


# --- subjects: what can be reviewed, and its current version --------------------

def character_subjects(catalog: dict) -> dict:
    """Characters (bosses too), identified by the bytes of their published art.
    Unrelated catalog edits do not change the version."""
    inventory = catalog.get("inventory") or {}
    result = {}
    for key, actor in inventory.get("identities", {}).items():
        units = [inventory["units"][uid] for uid in actor["units"]]
        paths = {unit["root"] + "/" + path for unit in units for path in unit["required"] + unit.get("optional", [])}
        products = [{"path": path, "sha256": catalog["products"].get(path, {}).get("sha256")} for path in sorted(paths)]
        identity = {"manifest": actor["catalog"].get("manifest"), "products": products,
                    "bundle_ids": {unit["id"]: unit["bundle"] for unit in units if unit.get("bundle")}}
        result[key] = dict(identity, kind="character", id=key, title=actor["display"], version_id=_digest(identity),
                           lineages=sorted({name for unit in units for name in unit["lineages"]}),
                           bundles=[{"id": unit["id"], "source": unit["generator"].get("source"), "repair": unit["repair"]} for unit in units])
    return result


def image_subjects(catalog: dict) -> dict:
    """Props, effects, backgrounds, tiles, icons and the other images, by the
    bytes of their files. A sheet of a known character is reviewed as that character."""
    result = {}
    for key, asset in (catalog.get("assets") or {}).items():
        if asset.get("identities"):
            continue
        products = [{"path": path, "sha256": catalog["products"].get(path, {}).get("sha256")} for path in sorted(asset["products"])]
        identity = {"asset": key, "products": products}
        result[key] = dict(identity, kind="image", id=key, title=asset["display"], version_id=_digest(identity),
                           family=asset.get("family"), category=asset.get("category"), generator=asset.get("generator"))
    return result


def item_subjects(catalog: dict) -> dict:
    """Sound effects, fonts and worlds, by the bytes of the files the page shows.
    An item with no such file has nothing to review. Music versions are the
    music tool's renders (`review_targets`)."""
    result = {}
    for key, item in (catalog.get("items") or {}).items():
        files = [{"path": path, "sha256": sha} for path, sha in item.get("version_files") or []]
        if item["kind"] == "music" or not any(row["sha256"] for row in files):
            continue
        identity = {"item": key, "files": files}
        result[key] = dict(identity, kind=item["kind"], id=key, title=item["display"], version_id=_digest(identity),
                           sources=list(item.get("sources") or []))
    return result


def all_subjects(catalog: dict) -> dict:
    """Every subject this module writes: items, images, then characters."""
    return item_subjects(catalog) | image_subjects(catalog) | character_subjects(catalog)


def _sha256(path: Path) -> str | None:
    if not path.is_file():
        return None
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def check_current(repo: Path, subject: dict) -> None:
    """Refuse a review when the files differ from the ones the page showed."""
    changed = ConflictError("The asset changed after the inspector was built. Reload the inspector, then review it again.")
    if subject["kind"] in ("sfx", "font", "world"):
        if any(_sha256(repo / row["path"]) != row["sha256"] for row in subject["files"]):
            raise changed
        return
    sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "scripts"))
    from lib.art_publication import resolve, store
    from .paths import asset_root, library_path, roots
    if subject["kind"] == "character":
        publication = store(library_path(repo))
        current = publication.selected() if publication else {"bundles": {}}
        if any(current["bundles"].get(unit) != key for unit, key in subject.get("bundle_ids", {}).items()):
            raise changed
    scanned = sorted(roots(repo), key=lambda root: -len(root.name))
    for product in subject["products"]:
        if subject.get("bundle_ids"):
            candidates = [resolve(library_path(repo), product["path"])]
        else:
            candidates = [asset_root(repo) / product["path"]]
            candidates += [root.path / product["path"][len(root.name) + 1:] for root in scanned if product["path"].startswith(root.name + "/")]
        found = [_sha256(path) for path in candidates if path is not None]
        if product["sha256"] not in found and not (product["sha256"] is None and not any(found)):
            raise changed


# --- documents ---------------------------------------------------------------------

def _safe(text: str) -> str:
    return re.sub(r"[^A-Za-z0-9._-]+", "_", text).strip("._") or "_"


class Store:
    """The review files of every kind except music: `reviews/<kind>/<subject>/`."""

    def __init__(self, repo: Path):
        self.root = repo / "tools" / "ambition_editor" / "reviews"

    def path(self, subject: dict, part: str) -> Path:
        name = subject["version_id"][:24] + (f"--{_safe(part)}" if part else "")
        return self.root / subject["kind"] / _safe(subject["id"]) / f"{name}.review.yaml"

    def load(self, problems: list | None = None) -> list[tuple[Path, dict]]:
        """Every review file. One that does not read goes to `problems`, or raises without it."""
        found = []
        for path in sorted(self.root.glob("*/*/*.review.yaml")):
            try:
                data = yaml.load(path.read_text(encoding="utf8"), Loader=_Loader) or {}
                if data.get("schema") != SCHEMA:
                    raise ValueError(f"the schema is {data.get('schema')!r}, not {SCHEMA}")
            except (OSError, ValueError, yaml.YAMLError) as error:
                if problems is None:
                    raise ValueError(f"{path}: {error}") from error
                problems.append(f"{path}: {error}")
                continue
            found.append((path, data))
        return found

    def save(self, subject: dict, *, score, notes="", issues=(), part="", furthest_played_seconds=0.0,
             furthest_played_fraction=0.0) -> tuple[Path, dict]:
        value = normalize_score(score)
        notes, part = str(notes).rstrip(), str(part or "").strip()
        if len(notes) > MAX_NOTES:
            raise ValueError(f"Notes may have at most {MAX_NOTES} characters.")
        if len(part) > 200:
            raise ValueError("A part name may have at most 200 characters.")
        issues = [str(tag) for tag in issues]
        unknown = [tag for tag in issues if tag not in ISSUE_TAGS[subject["kind"]]]
        if unknown:
            raise ValueError(f"Unknown issue tags for a {subject['kind']}: {unknown}")
        path = self.path(subject, part)
        existing = yaml.load(path.read_text(encoding="utf8"), Loader=_Loader) if path.is_file() else {}
        existing = existing or {}
        if existing and (existing.get("subject") or {}).get("id") != subject["id"]:
            raise ValueError(f"{path} holds a review of another subject.")
        previous = existing.get("current") or {}
        now = utc_now_iso()
        current = {"score": value, "label": score_label(value), "notes": notes, "issues": issues}
        if subject["kind"] in AUDIO_KINDS:
            current["furthest_played_seconds"] = round(max(0.0, float(furthest_played_seconds or 0)), 3)
            current["furthest_played_fraction"] = round(min(1.0, max(0.0, float(furthest_played_fraction or 0))), 4)
        current.update(created_at=str(previous.get("created_at") or now), updated_at=now, reviewed_at=now)
        data = {"schema": SCHEMA, "score_scale": {"min": MIN_SCORE, "max": MAX_SCORE},
                "subject": dict(subject, part=part), "current": current, "history": list(existing.get("history") or [])}
        path.parent.mkdir(parents=True, exist_ok=True)
        temp = path.with_suffix(".tmp")
        temp.write_text(yaml.safe_dump(data, sort_keys=False, allow_unicode=True, width=110), encoding="utf8")
        temp.replace(path)
        return path, data


def _music_documents(repo: Path, catalog: dict, problems: list) -> list[tuple[str, Path, dict]]:
    """The music tool's review files of each track of the index."""
    found = []
    for key, item in (catalog.get("items") or {}).items():
        details = item.get("details") or {}
        if item["kind"] != "music" or not details.get("music_tool"):
            continue
        for path in sorted((repo / details["music_tool"] / "reviews" / "music" / details["folder"]).glob("*.review.yaml")):
            try:
                data = yaml.load(path.read_text(encoding="utf8"), Loader=_Loader) or {}
            except (OSError, yaml.YAMLError) as error:
                problems.append(f"{path}: {error}")
                continue
            if data.get("schema") in (MUSIC_SCHEMA_V1, MUSIC_SCHEMA):
                found.append((key, path, data))
    return found


def _row(repo: Path, path: Path, data: dict, *, key: str, kind: str, title: str, relation: str) -> dict | None:
    current, subject = data.get("current") or {}, data.get("subject") or {}
    try:
        score = float(current["score"]) * (2.0 if data.get("schema") == MUSIC_SCHEMA_V1 else 1.0)
        score = normalize_score(score)
    except (KeyError, TypeError, ValueError):
        return None
    try:
        where = path.relative_to(repo).as_posix()
    except ValueError:
        where = str(path)
    return {"id": key, "kind": kind, "title": title, "version_id": str(subject.get("version_id") or ""),
            "part": str(subject.get("part") or ""), "relation": relation, "score": score, "label": score_label(score),
            "notes": str(current.get("notes") or ""), "issues": list(current.get("issues") or []),
            "furthest_played_seconds": current.get("furthest_played_seconds"),
            "furthest_played_fraction": current.get("furthest_played_fraction"),
            "created_at": str(current.get("created_at") or current.get("reviewed_at") or ""),
            "reviewed_at": str(current.get("updated_at") or current.get("reviewed_at") or ""), "path": where}


def snapshot(repo: Path, catalog: dict) -> dict:
    """Every review, whether it rated the current version, and the work queue."""
    subjects = all_subjects(catalog)
    rows, problems = [], []
    for path, data in Store(repo).load(problems):
        subject = data.get("subject") or {}
        now = subjects.get(subject.get("id"))
        relation = "unavailable" if now is None else "current" if now["version_id"] == subject.get("version_id") else "older"
        row = _row(repo, path, data, key=str(subject.get("id")), kind=str(subject.get("kind")),
                   title=(now or subject).get("title") or str(subject.get("id")), relation=relation)
        if row:
            rows.append(row)
    items = catalog.get("items") or {}
    for key, path, data in _music_documents(repo, catalog, problems):
        version = (data.get("subject") or {}).get("version_id")
        targets = {t["version_id"]: t for t in items[key].get("review_targets") or []}
        # The current version is the render the game plays; with none published, the newest render.
        current = [v for v, t in targets.items() if "what the game plays" in t["roles"]] or list(targets)
        relation = "current" if version in current else "newest render" if version in targets else "older"
        row = _row(repo, path, data, key=key, kind="music", title=items[key]["display"], relation=relation)
        if row:
            rows.append(row)
    rows.sort(key=lambda row: (row["reviewed_at"], row["id"]), reverse=True)
    return {"schema": SNAPSHOT_SCHEMA, "catalog_built_at": catalog.get("built_at"),
            "scale": {"min": MIN_SCORE, "max": MAX_SCORE}, "polish_threshold": POLISH_THRESHOLD,
            "rubric": [{"low": low, "high": high, "label": label, "description": text} for low, high, label, text in RUBRIC_BANDS],
            "issue_tags": {kind: list(tags) for kind, tags in ISSUE_TAGS.items()} | (
                {"music": (catalog.get("kinds") or {}).get("music", {}).get("issue_tags", [])}),
            "versions": {key: subject["version_id"] for key, subject in subjects.items()},
            "reviews": rows, "backlog": backlog(rows), "problems": problems}


def backlog(rows: list[dict]) -> dict:
    """The newest review of each (subject, part) that scored below the polish
    threshold. `fix` when it rated the current version; `re-review` when the asset
    changed since, so a person must listen or look again."""
    newest = {}
    for row in rows:  # newest first
        newest.setdefault((row["id"], row["part"]), row)
    queue = [dict(row, action="fix" if row["relation"] == "current" else "re-review")
             for row in newest.values() if row["score"] < POLISH_THRESHOLD and row["relation"] != "unavailable"]
    queue.sort(key=lambda row: (row["action"] != "fix", row["score"], row["kind"], row["id"], row["part"]))
    return {"schema": "ambition.asset_review_backlog.v1", "polish_threshold": POLISH_THRESHOLD,
            "count": len(queue), "rows": queue}


# --- saving from the page and the CLI ------------------------------------------------

def save(repo: Path, catalog: dict, data: dict) -> dict:
    """Save one review. Music goes to the music tool's review owner; every other
    kind to `Store`. `version_id` must be the version the page showed."""
    key = str(data.get("id") or "")
    item = (catalog.get("items") or {}).get(key)
    fields = {name: data.get(name) for name in ("notes", "furthest_played_seconds", "furthest_played_fraction")}
    if item and item["kind"] == "music":
        from . import music_review
        try:
            return music_review.save(repo, catalog, dict(fields, item=key, version_id=data.get("version_id"),
                                                         score=data.get("score"), issues=data.get("issues") or []))
        except LookupError as error:
            raise ConflictError(str(error)) from error
    subject = all_subjects(catalog).get(key)
    if subject is None:
        raise ValueError(f"No reviewable asset {key!r} in this index. Use a full id, such as ambition:npc_general or sfx:ui.confirm.")
    if data.get("version_id") != subject["version_id"]:
        raise ConflictError("The page and the server show different versions. Reload the inspector.")
    check_current(repo, subject)
    path, document = Store(repo).save(subject, score=data.get("score"), notes=fields["notes"] or "", issues=data.get("issues") or [],
                                      part=data.get("part") or "", furthest_played_seconds=fields["furthest_played_seconds"],
                                      furthest_played_fraction=fields["furthest_played_fraction"])
    return {"id": key, "version_id": subject["version_id"], "path": path.relative_to(repo).as_posix(), "review": document["current"]}


def read_catalog(out: Path) -> dict:
    path = out / "catalog.js"
    if not path.is_file():
        raise ValueError("Build the inspector first: python3 scripts/asset_inspector.py build")
    return json.loads(path.read_text().removeprefix("window.ART_CATALOG = ").rstrip().removesuffix(";"))


def cli(repo: Path, out: Path, args) -> int:
    try:
        catalog = read_catalog(out)
        if args.action == "add":
            if not args.id:
                raise ValueError("Name the asset to review, such as ambition:npc_general.")
            version = snapshot_versions(catalog).get(args.id)
            result = save(repo, catalog, {"id": args.id, "version_id": version, "score": args.score, "notes": args.notes,
                                          "issues": args.issue or [], "part": args.part})
        else:
            report = snapshot(repo, catalog)
            result = report["backlog"] if args.action == "backlog" else [
                row for row in report["reviews"] if (not args.kind or row["kind"] == args.kind) and (not args.id or row["id"] == args.id)]
        if args.json:
            print(json.dumps(result, indent=2, ensure_ascii=False))
        elif args.action == "backlog":
            print(f"{result['count']} reviews below {POLISH_THRESHOLD:g} (fix: the current version; re-review: the asset changed since)")
            for row in result["rows"]:
                print(f"  [{row['action']}] {row['score']:g} {row['label']} · {row['kind']} {row['id']}"
                      f"{' · ' + row['part'] if row['part'] else ''}: {row['notes'].splitlines()[0] if row['notes'] else ''}")
        elif args.action == "add":
            print(f"Saved {result['review']['score']:g} {result['review']['label']} to {result['path']}")
        else:
            for row in result:
                print(f"{row['score']:>5g} {row['label']:<12} {row['relation']:<13} {row['kind']:<9} {row['id']}"
                      f"{' · ' + row['part'] if row['part'] else ''}")
        return 0
    except (ValueError, OSError) as error:
        print(f"asset reviews: {error}", file=sys.stderr)
        return 2


def snapshot_versions(catalog: dict) -> dict:
    """The version a review saved now would rate: the subject's, or for music the
    render the game plays."""
    versions = {key: subject["version_id"] for key, subject in all_subjects(catalog).items()}
    for key, item in (catalog.get("items") or {}).items():
        targets = item.get("review_targets") or [] if item["kind"] == "music" else []
        if targets:  # `kinds` sorts the render the game plays first
            versions[key] = targets[0]["version_id"]
    return versions
