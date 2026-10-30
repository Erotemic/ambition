"""Every asset kind beside the sprite sheets: music, sound effects, fonts and worlds.

Each provider reads the selected library manifest (`library.json`) and game pack,
so any engine game is indexed the same way. A provider that cannot read its
sources gives its kind a notice; the other kinds still index. Every item has
one shape, which the page shows the same way:

    {id, kind, name, display, state, reasons, fix, sources, files, media,
     uses, reviews, tags, related, details}

`state` is `present`, `missing` (declared but not generated), `unregistered`
(a source no game entry names) or `unknown`. Sprites, backgrounds and other
images stay in the richer art index (`assets`, `inventory`).
"""

from __future__ import annotations

import json
import os
import re
import struct
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Callable

import yaml

from .paths import asset_root, library, library_path
from .sheets import SidecarCache

_Loader = getattr(yaml, "CSafeLoader", yaml.SafeLoader)


def _rel(repo: Path, path: Path) -> str:
    try:
        return path.resolve().relative_to(repo).as_posix()
    except ValueError:
        return str(path)


def _item(kind: str, name: str, **fields) -> dict:
    item = {"id": f"{kind}:{name}", "kind": kind, "name": name, "display": name, "state": "unknown",
            "reasons": [], "fix": None, "sources": [], "files": [], "media": None, "uses": [],
            "reviews": None, "tags": [], "related": [], "details": {}}
    item.update(fields)
    return item


def _file(repo: Path, out: Path, path: Path) -> dict:
    st = path.stat()
    return {"path": _rel(repo, path), "href": os.path.relpath(path, out), "bytes": st.st_size, "mtime": int(st.st_mtime)}


def _stat(path: Path) -> list:
    try:
        st = path.stat()
        return [st.st_size, st.st_mtime_ns]
    except OSError:
        return [None, None]


def fix_for(repo: Path, family: str, make: str | None) -> str:
    """The command that makes a family's content: its regen script, else the whole setup."""
    script = repo / "scripts" / "regen" / f"{family}.sh"
    return f"./scripts/regen/{family}.sh" if script.is_file() else (make or "scripts/setup/generated_content.sh")


# --- music -------------------------------------------------------------------

_TRACK = re.compile(r'id:\s*"([^"]+)",\s*display_name:\s*"((?:[^"\\]|\\.)*)"(?:,\s*asset_path:\s*Some\("([^"]+)"\))?')
#: The engine's music registry convention: a track without an explicit path plays this file.
_TRACK_PATH = "audio/music/generated/{id}/full.ogg"


def _music_review(path: Path) -> dict:
    data = yaml.load(path.read_text(), Loader=_Loader) or {}
    current, subject = data.get("current") or {}, data.get("subject") or {}
    return {"score": current.get("score"), "label": current.get("label"), "notes": current.get("notes") or "",
            "reviewed_at": current.get("reviewed_at") or current.get("updated_at"),
            "score_sha256": subject.get("score_sha256_at_review"), "version": subject.get("version_id")}


def music(ctx: dict) -> list[dict]:
    repo, out, config = ctx["repo"], ctx["out"], ctx["config"]
    root = ctx["root"]
    tool = repo / "tools" / config["tool"]
    scores = tool / "scores" / "active"
    policy = {}
    if config.get("policy"):
        try:
            policy = json.loads((ctx["library_dir"] / config["policy"]).read_text())
        except (OSError, ValueError):
            pass
    denied = set(policy.get("DENY_IDS", []))
    registry = root / config["registry"] if config.get("registry") else None
    tracks = {}
    if registry and registry.is_file():
        for tid, display, path in _TRACK.findall(registry.read_text()):
            tracks[tid] = (display.replace('\\"', '"'), path or _TRACK_PATH.format(id=tid))
    else:
        ctx["notices"].append(f"The music registry {registry} is missing; tracks are listed from their scores only.")
    items = []
    seen_dirs = set()
    for tid, (display, logical) in sorted(tracks.items()):
        parts = logical.split("/")
        folder = parts[3] if len(parts) > 4 and parts[:3] == ["audio", "music", "generated"] else tid
        seen_dirs.add(folder)
        items.append(_track(ctx, tid, display, root / logical, folder, scores, tool, registered=True))
    if scores.is_dir():
        for score in sorted(scores.glob("*.music.yaml")):
            folder = score.name.removesuffix(".music.yaml")
            if folder in seen_dirs:
                continue
            item = _track(ctx, folder, folder.replace("_", " ").title(), root / _TRACK_PATH.format(id=folder), folder, scores, tool, registered=False)
            if folder in denied:
                item["tags"].append("excluded by policy")
            items.append(item)
    return items


def _track(ctx, tid, display, audio: Path, folder, scores: Path, tool: Path, *, registered: bool) -> dict:
    repo, out = ctx["repo"], ctx["out"]
    item = _item("music", tid, display=display)
    score = scores / f"{folder}.music.yaml"
    if score.is_file():
        item["sources"].append(_rel(repo, score))
    if audio.is_file():
        item["files"].append(_file(repo, out, audio))
        item["media"] = {"type": "audio", "href": os.path.relpath(audio, out)}
        manifest = audio.parent / f"{folder}.adaptive_manifest.json"
        if manifest.is_file():
            item["files"].append(_file(repo, out, manifest))
        adaptive = audio.parent / "adaptive"
        if adaptive.is_dir():
            item["details"]["sections"] = sorted(p.name for p in adaptive.iterdir() if p.is_dir())
    # The music tool's newest render, which publishing copies into the game.
    preview = next(iter(sorted((tool / "generated" / folder / "latest" / "preview").glob("*.ogg"))), None)
    if preview and not audio.is_file():
        item["media"] = {"type": "audio", "href": os.path.relpath(preview, out), "unpublished": True}
        item["files"].append(_file(repo, out, preview))
    if registered:
        item["state"] = "present" if audio.is_file() else "missing"
        if not audio.is_file():
            item["reasons"].append(f"The game has no {audio.name} for it." + (
                " The music tool rendered it, but the render is not published." if preview else " It is not rendered."))
            item["fix"] = ctx["fix"]
    else:
        item["state"] = "unregistered"
        item["reasons"].append("A score that no registry entry names: the game cannot play it.")
    if not score.is_file():
        item["reasons"].append("No score in the music tool: the source of this audio is unknown.")
    reviews = sorted((tool / "reviews" / "music" / folder).glob("*.review.yaml"))
    if reviews:
        rows = [ctx["sidecars"].read(_music_review, path) for path in reviews]
        rows.sort(key=lambda row: str(row.get("reviewed_at") or ""))
        latest = rows[-1]
        current_sha = ctx["hashes"].digest(score) if score.is_file() else None
        item["reviews"] = {"count": len(rows), "score": latest["score"], "label": latest["label"], "notes": latest["notes"],
                           "reviewed_at": latest["reviewed_at"], "history": rows,
                           "score_changed": bool(current_sha and latest["score_sha256"] and latest["score_sha256"] != current_sha),
                           "tool": "music", "command": f"./{_rel(repo, tool)}/review_music.sh"}
    item["details"]["folder"] = folder
    return item


# --- sound effects ------------------------------------------------------------

_HEADER = struct.Struct("<8sIIQQQ")
_ENTRY = struct.Struct("<QQIBBHIIfffI16s")
_CODECS = {0: "wav", 1: "ogg", 2: "flac"}


def read_bank(path: Path) -> list[dict]:
    """The entries of an `AMBNDSFX` sound bank (`tools/ambition_sfx_pack/pack.py`)."""
    data = path.read_bytes()
    magic, version, count, entries_at, _payloads_at, names_at = _HEADER.unpack_from(data, 0)
    if magic != b"AMBNDSFX" or version != 1:
        raise ValueError(f"{path} is not a version 1 sound bank")
    rows, cursor = [], names_at
    for index in range(count):
        (_hash, offset, length, codec, channels, _pad, rate, ms, gain, peak, rms, flags, _r) = _ENTRY.unpack_from(data, entries_at + index * _ENTRY.size)
        (size,) = struct.unpack_from("<H", data, cursor)
        name = data[cursor + 2: cursor + 2 + size].decode()
        cursor += 2 + size
        rows.append({"id": name, "offset": offset, "length": length, "codec": _CODECS.get(codec, "wav"), "channels": channels,
                     "sample_rate": rate, "ms": ms, "gain_db": round(gain, 2), "peak_db": round(peak, 2), "rms_db": round(rms, 2),
                     "looping": bool(flags & 2)})
    return rows


def _sfx_source(path: Path) -> dict:
    data = yaml.load(path.read_text(), Loader=_Loader) or {}
    return {"duration_ms": data.get("duration_ms"), "layers": [layer.get("name") for layer in data.get("layers") or [] if isinstance(layer, dict)],
            "description": data.get("description") or data.get("summary")}


def sfx(ctx: dict) -> list[dict]:
    repo, out, config, root = ctx["repo"], ctx["out"], ctx["config"], ctx["root"]
    tool = repo / "tools" / config["tool"]
    bank = next((root / r for r in config.get("roots", []) if r.endswith(".bank")), None)
    entries = {}
    if bank and bank.is_file():
        rows = read_bank(bank)
        entries = {row["id"]: row for row in rows}
        _extract(bank, rows, out / "audio" / "sfx")
    else:
        ctx["notices"].append(f"The sound bank {bank} is missing; sound effects are listed from their sources only.")
    sources = {p.name.removesuffix(".sfx.yaml"): p for p in (tool / "sounds" / "active").glob("*.sfx.yaml")}
    items = []
    for name in sorted(entries.keys() | sources.keys()):
        item = _item("sfx", name, tags=[name.split(".")[0]])
        entry, source = entries.get(name), sources.get(name)
        if source:
            item["sources"].append(_rel(repo, source))
            try:
                item["details"].update(ctx["sidecars"].read(_sfx_source, source))
            except (OSError, yaml.YAMLError, ValueError) as error:
                item["reasons"].append(f"The source does not read: {error}")
        if entry:
            item["state"] = "present"
            clip = out / "audio" / "sfx" / f"{name}.{entry['codec']}"
            item["media"] = {"type": "audio", "href": os.path.relpath(clip, out)}
            item["files"].append({"path": _rel(repo, bank) + "#" + name, "href": os.path.relpath(clip, out), "bytes": entry["length"]})
            item["details"].update({k: v for k, v in entry.items() if k not in ("id", "offset", "length")})
        else:
            item["state"] = "missing"
            item["reasons"].append("A source that the sound bank does not hold: render and pack it.")
            item["fix"] = ctx["fix"]
        if not source:
            item["reasons"].append("No source in the sound tool: the origin of this sound is unknown.")
        items.append(item)
    return items


def _extract(bank: Path, rows: list[dict], into: Path) -> None:
    """Write each clip as its own file for the page's player, once per bank version."""
    st = bank.stat()
    stamp = into / ".bank_stamp"
    key = f"{st.st_size}:{st.st_mtime_ns}"
    if stamp.is_file() and stamp.read_text() == key:
        return
    into.mkdir(parents=True, exist_ok=True)
    with bank.open("rb") as stream:
        for row in rows:
            stream.seek(row["offset"])
            (into / f"{row['id']}.{row['codec']}").write_bytes(stream.read(row["length"]))
    stamp.write_text(key)


# --- fonts --------------------------------------------------------------------

def fonts(ctx: dict) -> list[dict]:
    repo, out, config, root = ctx["repo"], ctx["out"], ctx["config"], ctx["root"]
    policy = json.loads((ctx["library_dir"] / config["policy"]).read_text()) if config.get("policy") else []
    files = {p.name: p for r in config.get("roots", []) for p in (root / r).rglob("*") if p.suffix.lower() in (".ttf", ".otf", ".woff", ".woff2")}
    items, named = [], set()
    for family in policy if isinstance(policy, list) else []:
        for _archive_name, name in family.get("extracts", []):
            named.add(name)
            path = files.get(name)
            item = _item("font", name, display=name.rsplit(".", 1)[0], tags=[family.get("license_name") or "unknown license"],
                         details={"family": family.get("name"), "license": family.get("license_name"), "homepage": family.get("homepage"), "url": family.get("url")})
            item["sources"].append(_rel(repo, ctx["library_dir"] / config["policy"]))
            if path:
                item["state"] = "present"
                item["aliases"] = [path.relative_to(root).as_posix()]
                item["files"].append(_file(repo, out, path))
                item["media"] = {"type": "font", "href": os.path.relpath(path, out)}
            else:
                item["state"] = "missing"
                item["reasons"].append("The font policy names this file; it is not downloaded.")
                item["fix"] = ctx["fix"]
            items.append(item)
    for name, path in sorted(files.items()):
        if name not in named:
            item = _item("font", name, display=name.rsplit(".", 1)[0], state="unregistered")
            item["reasons"].append("A font file that the font policy does not name.")
            item["files"].append(_file(repo, out, path))
            item["media"] = {"type": "font", "href": os.path.relpath(path, out)}
            items.append(item)
    return items


# --- worlds -------------------------------------------------------------------

_ROOM_FIELDS = ("music_track", "fight_music_track", "parallax_theme", "biome", "visual_theme", "ambient_profile")


def worlds(ctx: dict) -> list[dict]:
    repo, out, pack = ctx["repo"], ctx["out"], ctx["pack"]
    manifest = pack / "data" / "worlds.ron"
    names = re.findall(r'"([^"]+\.ldtk)"', manifest.read_text()) if manifest.is_file() else []
    files = [pack / "worlds" / Path(name).name for name in names] or sorted((pack / "worlds").glob("*.ldtk"))
    items = []
    for path in dict.fromkeys(files):
        item = _item("world", path.stem)
        if not path.is_file():
            item.update(state="missing", fix=None)
            item["reasons"].append(f"{_rel(repo, manifest)} names {path.name}, which does not exist.")
            items.append(item)
            continue
        item["state"] = "present"
        item["sources"].append(_rel(repo, path))
        item["files"].append(_file(repo, out, path))
        try:
            rooms = ctx["sidecars"].read(_ldtk_rooms, path)
        except (OSError, ValueError) as error:
            item["reasons"].append(f"The world file does not read: {error}")
            rooms = []
        item["details"]["rooms"] = rooms
        for room in rooms:
            for field in ("music_track", "fight_music_track"):
                if room.get(field):
                    ctx["uses"][f"music:{room[field]}"].append({"where": f"{path.stem} / {room['id']}", "field": field, "file": _rel(repo, path), "item": item["id"]})
            if room.get("parallax_theme"):
                item["tags"].append(room["parallax_theme"])
        item["tags"] = sorted(set(item["tags"]))
        item["related"] = sorted({f"music:{room[f]}" for room in rooms for f in ("music_track", "fight_music_track") if room.get(f)})
        items.append(item)
    return items


def _ldtk_rooms(path: Path) -> list[dict]:
    document = json.loads(path.read_text())
    rooms = []
    for level in document.get("levels", []):
        fields = {f["__identifier"]: f.get("__value") for f in level.get("fieldInstances", []) if f.get("__identifier") in _ROOM_FIELDS}
        rooms.append({"id": level.get("identifier"), "size": [level.get("pxWid"), level.get("pxHei")],
                      **{k: v for k, v in fields.items() if v not in (None, "")}})
    return rooms


# --- uses in game text ----------------------------------------------------------

_TEXT = ("*.ron", "*.rs", "*.yarn", "*.json", "*.yaml")


def text_uses(repo: Path, items: list[dict], uses: dict) -> None:
    """Where game and engine text names an item, by its quoted id (`"ability.fire.charge"`)."""
    # An item is named by its id (music, sound) or by its path under the asset root (`"fonts/bundled/X.ttf"`).
    names = {item["name"]: item["id"] for item in items if item["kind"] in ("music", "sfx")}
    names |= {alias: item["id"] for item in items for alias in item.pop("aliases", [])}
    if not names:
        return
    patterns = "\n".join(f'"{name}"' for name in names)
    done = subprocess.run(["git", "grep", "-n", "-o", "-F", "--no-color", "-f", "-", "--", "game", "crates", *_TEXT],
                          cwd=repo, input=patterns, capture_output=True, text=True)
    for line in done.stdout.splitlines():
        try:
            file, number, match = line.split(":", 2)
        except ValueError:
            continue
        item = names.get(match.strip('"'))
        if item and len(uses[item]) < 60 and "/generated/" not in file:
            uses[item].append({"where": f"{file}:{number}", "file": file, "line": int(number)})


# --- collection -----------------------------------------------------------------

PROVIDERS: dict[str, tuple[str, Callable]] = {
    # library producer family -> (item kind, provider)
    "music": ("music", music),
    "sfx": ("sfx", sfx),
    "fonts": ("font", fonts),
}


def family_status(repo: Path) -> dict:
    """The library's producer families, present, missing or stale (`scripts/content_status.py`)."""
    sys.path.insert(0, str(repo / "scripts"))
    import content_status
    report = content_status.status(library_path(repo).resolve(), None, True)
    return report


def collect(repo: Path, out: Path, hashes, *, pack: Path, families: dict | None = None, log=print) -> dict:
    """The non-sprite items of the selected game. Never raises for a kind: its notice says why it is absent."""
    try:
        lib = library(repo)
    except (OSError, ValueError) as error:
        return {"kinds": {}, "items": {}, "kinds_notice": f"The library manifest does not read: {error}"}
    lib_dir = library_path(repo).parent
    root = asset_root(repo)
    sidecars = SidecarCache(out / "kind_cache.json")
    uses: dict[str, list] = defaultdict(list)
    make = (families or {}).get("make")
    kinds: dict[str, dict] = {}
    items: list[dict] = []
    jobs = [(family, *PROVIDERS[family]) for family in lib.get("producers", {}) if family in PROVIDERS]
    jobs.append(("worlds", "world", worlds))
    for family, kind, provider in jobs:
        notices: list[str] = []
        ctx = {"repo": repo, "out": out, "config": lib.get("producers", {}).get(family, {}), "library_dir": lib_dir, "root": root,
               "pack": pack, "hashes": hashes, "sidecars": sidecars, "uses": uses, "notices": notices, "fix": fix_for(repo, family, make)}
        try:
            found = provider(ctx)
        except Exception as error:  # noqa: BLE001 - one kind's failure must not hide the others
            found = []
            notices.append(f"{type(error).__name__}: {error}")
            log(f"NOTICE: the {kind} items are unavailable: {error}")
        items += found
        kinds[kind] = {"family": family, "count": len(found), "states": dict(Counter(i["state"] for i in found)),
                       "notices": notices, "fix": ctx["fix"]}
    try:
        text_uses(repo, items, uses)
    except OSError as error:
        for row in kinds.values():
            row["notices"].append(f"Uses in game text are unknown: {error}")
    for item in items:
        # The files behind an item, for review notes: a note names the version it reviewed.
        real = {f["path"].split("#", 1)[0] for f in item["files"]}
        item["stamp"] = [[path, *_stat(repo / path)] for path in sorted(real)]
    by_id = {item["id"]: item for item in items}
    for item_id, rows in uses.items():
        if item_id in by_id:
            by_id[item_id]["uses"] = rows
            by_id[item_id]["related"] = sorted(set(by_id[item_id]["related"]) | {row["item"] for row in rows if row.get("item")})
    sidecars.save()
    return {"kinds": kinds, "items": by_id}
