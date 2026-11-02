"""Every asset kind beside the sprite sheets: music, sound effects, fonts and worlds.

Each provider reads the selected library manifest (`library.json`) and game pack,
so any engine game is indexed the same way. A provider that cannot read its
sources gives its kind a notice; the other kinds still index. Every item has
one shape, which the page shows the same way:

    {id, kind, name, display, state, reasons, fix, sources, files, media,
     uses, reviews, tags, related, details}

`state` is `present`, `missing` (declared or authored, but the game cannot
load it), `excluded` (a policy leaves it out), `unregistered` (a file no game
entry names) or `unknown`. Sprites, backgrounds and other
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


def music_reviews_module(tool: Path):
    """The music tool's review owner (`ambition_music_renderer.music_reviews`), which
    names render versions and writes review files. It needs only PyYAML."""
    import importlib
    if str(tool) not in sys.path:
        sys.path.insert(0, str(tool))
    return importlib.import_module("ambition_music_renderer.music_reviews")


def _render_versions(tool: Path, notices: list) -> dict[str, list]:
    if not (tool / "ambition_music_renderer" / "music_reviews.py").is_file():
        return {}  # this game's music tool has no review owner: the page only shows reviews
    try:
        module = music_reviews_module(tool)
        rows = module.discover_render_versions(tool, include_agent_bundles=False)
    except Exception as error:  # noqa: BLE001 - reviews from the page are optional
        notices.append(f"Music reviews from the page are unavailable: {type(error).__name__}: {error}")
        return {}
    by_cue: dict[str, list] = defaultdict(list)
    for version in rows:
        by_cue[version.cue_id].append(version)
    return by_cue


def _sound_of(row: dict) -> tuple[str, str]:
    """(source kind, sound) of one score instrument: what actually makes its sound."""
    backend = row.get("instrument_backend") if isinstance(row.get("instrument_backend"), dict) else {}
    kind = backend.get("kind")
    if kind == "sfz":
        name = backend.get("library_ref") or Path(str(backend.get("sfz_glob") or "")).name.removesuffix(".sfz")
        return "sampled (SFZ)", name or "unspecified"
    if kind == "surge":
        return "synth (Surge XT)", str(backend.get("patch") or "")
    if kind == "soundfont":
        return "sampled (SoundFont)", str(backend.get("library_ref") or backend.get("soundfont") or "")
    if kind == "procedural_fm":
        carrier = (backend.get("carrier") or {}).get("waveform", "")
        return "synth (procedural FM)", f"{carrier} carrier".strip()
    return ("drum kit (General MIDI)" if row.get("is_drum") else "General MIDI"), str(row.get("program") or "")


def _score_summary(path: Path) -> dict:
    """What a music page shows of its score: description, notes, form and instruments."""
    data = yaml.load(path.read_text(), Loader=_Loader) or {}
    notes = data.get("notes")
    notes = [str(n) for n in notes] if isinstance(notes, list) else [p for p in str(notes or "").split("\n\n") if p.strip()]
    tempo, meter = data.get("tempo"), data.get("meter")
    sections = data.get("sections")
    names = ([s.get("name") or s.get("id") for s in sections if isinstance(s, dict)] if isinstance(sections, list)
             else list(sections) if isinstance(sections, dict) else [])
    instruments = []
    for row in data.get("instruments") or []:
        if isinstance(row, dict):
            source, sound = _sound_of(row)
            instruments.append({"name": row.get("name"), "group": row.get("group"), "source": source, "sound": sound,
                                "notes": row.get("notes") if isinstance(row.get("notes"), str) else None})
    return {"title": data.get("title"), "description": data.get("description"), "notes": notes,
            "key": data.get("key") if isinstance(data.get("key"), str) else None,
            "bpm": tempo.get("bpm") if isinstance(tempo, dict) else tempo if isinstance(tempo, (int, float)) else None,
            "meter": f"{meter.get('beats_per_bar')}/{meter.get('beat_unit')}" if isinstance(meter, dict) else meter if isinstance(meter, str) else None,
            "sections": [str(n) for n in names if n], "form": data.get("form") if isinstance(data.get("form"), str) else None,
            "one_shot": bool(data.get("one_shot")), "instruments": instruments}


def _music_review(path: Path) -> dict:
    data = yaml.load(path.read_text(), Loader=_Loader) or {}
    current, subject = data.get("current") or {}, data.get("subject") or {}
    return {"score": current.get("score"), "label": current.get("label"), "notes": current.get("notes") or "", "issues": current.get("issues") or [],
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
    denied = ctx["denied"] = set(policy.get("DENY_IDS", []))
    registry = root / config["registry"] if config.get("registry") else None
    tracks = {}
    if registry and registry.is_file():
        for tid, display, path in _TRACK.findall(registry.read_text()):
            tracks[tid] = (display.replace('\\"', '"'), path or _TRACK_PATH.format(id=tid))
    else:
        ctx["notices"].append(f"The music registry {registry} is missing; tracks are listed from their scores only.")
    ctx["versions"] = _render_versions(tool, ctx["notices"])
    if ctx["versions"]:
        module = music_reviews_module(tool)
        ctx["extra"] = {"issue_tags": list(module.ISSUE_TAGS), "score_range": [module.MIN_SCORE, module.MAX_SCORE]}
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
            items.append(_track(ctx, folder, folder.replace("_", " ").title(), root / _TRACK_PATH.format(id=folder), folder, scores, tool, registered=False))
    return items


def _track(ctx, tid, display, audio: Path, folder, scores: Path, tool: Path, *, registered: bool) -> dict:
    repo, out = ctx["repo"], ctx["out"]
    item = _item("music", tid, display=display)
    score = scores / f"{folder}.music.yaml"
    if score.is_file():
        item["sources"].append(_rel(repo, score))
        try:
            item["score"] = ctx["sidecars"].read(_score_summary, score)
            if item["score"]["title"] and not registered:
                item["display"] = item["score"]["title"]  # no game name: the score names it
            sources = sorted({row["source"] for row in item["score"]["instruments"]})
            item["tags"] += sorted({"general-midi" if source.startswith(("General", "drum")) else source.split(" ")[0] for source in sources})
        except (OSError, ValueError, yaml.YAMLError) as error:
            item["reasons"].append(f"The score does not read: {error}")
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
    elif ctx.get("denied") and tid in ctx["denied"]:
        item["state"] = "excluded"
        item["reasons"].append("The music policy excludes this score (DENY_IDS), so the registry leaves it out.")
    else:
        # The registry is regenerated from the active scores at each music publish.
        item["state"] = "missing"
        item["reasons"].append("The score is newer than the last music publish: neither the music registry nor the game has it yet."
                               + (" The music tool has a render." if preview else ""))
        item["fix"] = ctx["fix"]
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
    item["details"]["music_tool"] = _rel(repo, tool)
    item["review_targets"] = _review_targets(ctx, folder, audio)
    return item


def _review_targets(ctx, cue: str, audio: Path) -> list[dict]:
    """The render versions a review on the page can rate: the one the game plays
    (its preview bytes equal the published file) and the newest render."""
    versions = ctx.get("versions", {}).get(cue, [])
    if not versions:
        return []
    published = ctx["hashes"].digest(audio) if audio.is_file() else None
    targets = {}
    for version in versions:
        roles = (["what the game plays"] if published and version.preview_sha256 == published else []) + (["newest render"] if version.is_latest else [])
        if roles:
            targets[version.version_id] = {"version_id": version.version_id, "render_hash": version.render_hash,
                "preview_sha256": version.preview_sha256, "roles": roles, "href": os.path.relpath(version.preview_path, ctx["out"])}
    return sorted(targets.values(), key=lambda t: "what the game plays" not in t["roles"])


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
        try:
            item["preview"] = _world_previews(ctx, path, rooms)
        except Exception as error:  # noqa: BLE001 - previews are optional; the rooms still list
            item["reasons"].append(f"No world preview: {type(error).__name__}: {error}")
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
    linear = document.get("worldLayout") in ("LinearHorizontal", "LinearVertical")
    cursor = 0
    for level in document.get("levels", []):
        fields = {f["__identifier"]: f.get("__value") for f in level.get("fieldInstances", []) if f.get("__identifier") in _ROOM_FIELDS}
        width, height = level.get("pxWid") or 0, level.get("pxHei") or 0
        x, y = level.get("worldX", 0), level.get("worldY", 0)
        if linear:  # a linear world stores no positions: the rooms follow each other
            x, y = (cursor, 0) if document["worldLayout"] == "LinearHorizontal" else (0, cursor)
            cursor += (width if document["worldLayout"] == "LinearHorizontal" else height) + 64
        rooms.append({"id": level.get("identifier"), "size": [width, height], "at": [x, y], "depth": level.get("worldDepth", 0),
                      **{k: v for k, v in fields.items() if v not in (None, "")}})
    return rooms


def _world_previews(ctx: dict, path: Path, rooms: list[dict]) -> dict:
    """SVG previews of a world and of each room, drawn by the LDtk tool
    (`ambition_ldtk_tools.room_support.render`): collision, water and climbable
    cells, and entities. Redrawn only when the world file changes."""
    repo, out = ctx["repo"], ctx["out"]
    tool = repo / "tools" / "ambition_ldtk_tools"
    if str(tool) not in sys.path:
        sys.path.insert(0, str(tool))
    from ambition_ldtk_tools.room_support.render import render_room_svg
    folder = out / "worlds" / path.stem
    stamp = folder / ".stamp"
    st = path.stat()
    key = f"{st.st_size}:{st.st_mtime_ns}:2"
    names = {room["id"]: f"{re.sub(r'[^A-Za-z0-9_.-]', '_', str(room['id']))}.svg" for room in rooms}
    if not (stamp.is_file() and stamp.read_text() == key):
        folder.mkdir(parents=True, exist_ok=True)
        document = json.loads(path.read_text())
        drawn = {room["id"]: render_room_svg(document, room["id"], max_width=10**9) for room in rooms}
        for room_id, svg in drawn.items():
            (folder / names[room_id]).write_text(svg)
        (folder / "_world.svg").write_text(_world_svg(rooms, drawn))
        stamp.write_text(key)
    left = min((r["at"][0] for r in rooms), default=0)
    top = min((r["at"][1] for r in rooms), default=0)
    right = max((r["at"][0] + r["size"][0] for r in rooms), default=1)
    bottom = max((r["at"][1] + r["size"][1] for r in rooms), default=1)
    margin = _MARGIN
    return {"world": os.path.relpath(folder / "_world.svg", out),
            "bounds": [left - margin, top - margin, right - left + 2 * margin, bottom - top + 2 * margin],
            "rooms": {room_id: os.path.relpath(folder / name, out) for room_id, name in names.items()}}


_MARGIN = 128


def _world_svg(rooms: list[dict], drawn: dict[str, str]) -> str:
    """Every room at its world position, each with its name. A room on another depth is dashed."""
    if not rooms:
        return '<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"/>'
    left = min(r["at"][0] for r in rooms) - _MARGIN
    top = min(r["at"][1] for r in rooms) - _MARGIN
    width = max(r["at"][0] + r["size"][0] for r in rooms) - left + _MARGIN
    height = max(r["at"][1] + r["size"][1] for r in rooms) - top + _MARGIN
    font = max(24, int(max(width, height) / 90))
    parts = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{left} {top} {width} {height}" width="{min(width, 2400)}" height="{int(height * min(width, 2400) / width)}">',
             f'<rect x="{left}" y="{top}" width="{width}" height="{height}" fill="#2b2f36"/>']
    for room in sorted(rooms, key=lambda r: r.get("depth", 0), reverse=True):
        x, y = room["at"]
        w, h = room["size"]
        inner = drawn[room["id"]].split(">", 1)[1].rsplit("</svg>", 1)[0]
        parts.append(f'<svg x="{x}" y="{y}" width="{w}" height="{h}" viewBox="0 0 {w} {h}">{inner}</svg>')
        dash = ' stroke-dasharray="24 16"' if room.get("depth") else ""
        parts.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="none" stroke="#fbbf24" stroke-width="{max(4, font // 6)}"{dash}/>')
        if w * 4 < len(str(room["id"])) * font * 2:
            continue  # too small to label legibly: the page names it on hover
        label = str(room["id"]).replace("&", "&amp;").replace("<", "&lt;")
        parts.append(f'<text x="{x + font // 3}" y="{y + font}" font-family="sans-serif" font-size="{font}" fill="#fde68a" stroke="#111827" stroke-width="{max(2, font // 8)}" paint-order="stroke">{label}</text>')
    parts.append("</svg>")
    return "\n".join(parts) + "\n"


# --- uses in game text ----------------------------------------------------------

_TEXT = ("*.ron", "*.rs", "*.yarn", "*.json", "*.yaml")
_QUOTED = re.compile(r'"([^"\n]+)"')
#: A music id is a plain word (`billions_and_billions`) that other things share, such as a
#: move clip of that name; a line names a track only when it also says music, track, cue or song.
_MUSIC_WORDS = re.compile(r"music|track|cue|song|radio|playlist", re.I)


def text_uses(repo: Path, items: list[dict], uses: dict) -> None:
    """Where game and engine text names an item, by its quoted id (`"ability.fire.charge"`)
    or quoted asset path (`"fonts/bundled/X.ttf"`)."""
    names = {item["name"]: item for item in items if item["kind"] in ("music", "sfx")}
    names |= {alias: item for item in items for alias in item.pop("aliases", [])}
    if not names:
        return
    patterns = "\n".join(f'"{name}"' for name in names)
    done = subprocess.run(["git", "grep", "-n", "-F", "--no-color", "-f", "-", "--", "game", "crates", *_TEXT],
                          cwd=repo, input=patterns, capture_output=True, text=True)
    for line in done.stdout.splitlines():
        try:
            file, number, text = line.split(":", 2)
        except ValueError:
            continue
        if "/generated/" in file:
            continue
        for quoted in _QUOTED.findall(text):
            item = names.get(quoted)
            if not item or (item["kind"] == "music" and quoted == item["name"] and not _MUSIC_WORDS.search(text)):
                continue
            rows = uses[item["id"]]
            if len(rows) < 60 and not any(row.get("file") == file and row.get("line") == int(number) for row in rows):
                rows.append({"where": f"{file}:{number}", "file": file, "line": int(number), "text": text.strip()[:160]})


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
                       "notices": notices, "fix": ctx["fix"], **ctx.get("extra", {})}
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
            rooms = [row["where"] for row in rows if row.get("field")]
            if rooms and by_id[item_id]["state"] != "present":
                by_id[item_id]["reasons"].insert(0, f"{len(rooms)} room(s) ask for it, and the game cannot play it there: {', '.join(rooms[:4])}.")
            by_id[item_id]["related"] = sorted(set(by_id[item_id]["related"]) | {row["item"] for row in rows if row.get("item")})
    sidecars.save()
    return {"kinds": kinds, "items": by_id}
