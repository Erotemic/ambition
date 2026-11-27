"""Make small edits to the selected pack's authored content."""

from __future__ import annotations

import hashlib
import json
import re
import stat
import sys
from pathlib import Path
from tempfile import NamedTemporaryFile

from . import kinds
from .ron_spans import Document

IMPORTANCE = ("Unclassified", "Primary", "Secondary", "Tertiary", "Background")
MUSIC_FIELDS = ("music_track", "fight_music_track")


class Conflict(ValueError):
    """The source changed after the editor read it."""


def _source_path(path, repo, pack):
    path = path.resolve()
    if not any(path.is_relative_to(root.resolve()) for root in (repo, pack)) or not path.is_file():
        raise ValueError("The selected pack source is unavailable.")
    return path


def _revision(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


def _check_revision(expected, actual):
    if expected != actual:
        raise Conflict("The source changed. Close the editor and open it again.")


def _replace(path, before, write):
    mode = stat.S_IMODE(path.stat().st_mode)
    with NamedTemporaryFile(dir=path.parent, suffix=path.suffix, delete=False) as file:
        staged = Path(file.name)
    try:
        write(staged)
        staged.chmod(mode)
        if path.read_bytes() != before:
            raise Conflict("The source changed during the save. Open the editor again.")
        staged.replace(path)
    finally:
        staged.unlink(missing_ok=True)


def _list(doc, span):
    start, end = span
    if doc[start] != "[" or doc.ends.get(start) != end:
        raise ValueError("Expected a RON list.")
    return [index for index in doc.children(start + 1, end) if doc[index] != ","]


def _pack_sources(pack):
    manifest = Document((pack / "pack.ron").read_text())
    fields = manifest.entries(manifest.root())
    namespace = manifest.string(fields["namespace"][0]) if "namespace" in fields else manifest.string(fields["id"][0])
    libraries = [manifest.string(at) for at in _list(manifest, fields["libraries"])] if "libraries" in fields else []
    sources = []
    for at in _list(manifest, fields["sources"]):
        if manifest[at] != "(":
            raise ValueError("Expected a source record in pack.ron.")
        source = manifest.entries(at)
        lib = source.get("library")
        library = None
        if lib is not None and manifest[lib[0]] == "Some":
            library = manifest.string(lib[0] + 2)
        sources.append((manifest.string(source["schema"][0]), manifest.string(source["path"][0]), library))
    return namespace, libraries, sources


def _cast_fields(doc, span):
    if span is None:
        return {"importance": "Unclassified", "roles": {}}
    start, end = span
    if doc[start] != "(" or doc.ends.get(start) != end:
        raise ValueError("Expected a cast record.")
    fields = doc.entries(start)
    return {
        "importance": doc[fields["importance"][0]] if "importance" in fields else "Unclassified",
        "roles": {group: [doc.string(at) for at in _list(doc, values)]
                  for group, values in doc.entries(fields["roles"][0]).items()} if "roles" in fields else {},
    }


def _normalize_cast(cast):
    if not isinstance(cast, dict) or cast.get("importance") not in IMPORTANCE or not isinstance(cast.get("roles"), dict):
        raise ValueError("Choose an importance level and supply role groups.")

    def label(value):
        if not isinstance(value, str) or not value.strip() or not value.strip().isprintable():
            raise ValueError("Each role group and role needs a name.")
        return value.strip()

    roles = {}
    for group, labels in cast["roles"].items():
        if not isinstance(labels, list):
            raise ValueError("Supply a list of roles for each group.")
        roles.setdefault(label(group), set()).update(label(value) for value in labels)
    return {"importance": cast["importance"], "roles": {key: sorted(labels) for key, labels in sorted(roles.items()) if labels}}


def _cast_ron(cast):
    groups = ", ".join(f"{json.dumps(group, ensure_ascii=False)}: {json.dumps(labels, ensure_ascii=False)}"
                       for group, labels in cast["roles"].items())
    return f"(importance: {cast['importance']}, roles: {{{groups}}})"


def character_cast(repo, pack, data, *, save=False):
    key = data.get("id")
    namespace, _, sources = _pack_sources(pack)
    if not isinstance(key, str) or not key.startswith(namespace + ":"):
        raise ValueError("Select a character from this pack.")
    character = key.split(":", 1)[1]
    matches = []
    for schema, name, library in sources:
        if schema != "character_catalog" or library is not None:
            continue
        path = _source_path(pack / name, repo, pack)
        raw = path.read_bytes()
        doc = Document(raw.decode("utf-8"))
        start, _ = doc.entries(doc.root())["characters"]
        if doc[start] != "{":
            raise ValueError("Expected a characters map.")
        if row := doc.entries(start).get(character):
            matches.append((path, raw, doc, row[0]))
    if len(matches) != 1:
        raise ValueError("This character needs one authored catalog row in the selected pack.")
    path, raw, doc, opening = matches[0]
    if doc[opening] != "(":
        raise ValueError("Expected an authored character row.")
    span = doc.entries(opening).get("cast")
    current = _normalize_cast(_cast_fields(doc, span))
    begin = end = doc.tokens[opening].end()
    if span:
        begin, end = doc.tokens[span[0]].start(), doc.tokens[span[1]].end()
    fingerprint = doc.text[begin:end]
    revision = _revision([str(path), key, fingerprint])
    if save:
        _check_revision(data.get("revision"), revision)
        desired = _normalize_cast(data.get("cast"))
        if desired != current:
            replacement = _cast_ron(desired)
            fingerprint = replacement
            if not span:
                first = doc.tokens[opening + 1].start()
                newline = "\r\n" if "\r\n" in doc.text[begin:first] else "\n"
                indent = re.match(r"[ \t]*", doc.text[doc.text.rfind("\n", 0, first) + 1:first]).group()
                replacement = f"{newline}{indent}cast: {replacement}," if "\n" in doc.text[begin:first] else f"cast: {replacement}, "
            updated = doc.text[:begin] + replacement + doc.text[end:]
            _replace(path, raw, lambda staged: staged.write_bytes(updated.encode("utf-8")))
            current = desired
            revision = _revision([str(path), key, fingerprint])
    return {"id": key, "cast": current, "revision": revision}


def _tracks(pack):
    _, manifests, sources = _pack_sources(pack)
    scripts = str(Path(__file__).resolve().parents[3] / "scripts")
    if scripts not in sys.path:
        sys.path.insert(0, scripts)
    from lib.content_libraries import read_libraries

    tracks = {}
    for schema, logical, library in sources:
        if schema != "music_registry":
            continue
        if library is None:
            path = pack / logical
        else:
            lib = read_libraries([pack / manifest for manifest in manifests]).get(library)
            if lib is None:
                continue
            selected = lib.selection()
            if selected and logical in selected["files"]:
                path = lib.publication_root() / selected["files"][logical]["path"]
            else:
                row = lib.selected_files().get(logical)
                if row is None:
                    continue
                path = row.path
        if path.is_file():
            tracks.update({key: title for key, (title, _) in kinds.read_music_registry(path).items()})
    return tracks


def _level_field(level, name):
    return next((field.get("__value") for field in level.get("fieldInstances", [])
                 if field.get("__identifier") == name), None)


def _level_music(level, field):
    value = _level_field(level, field)
    return value.strip() if isinstance(value, str) else ""


def _active_area(level):
    return _level_music(level, "activeArea") or level["identifier"]


def room_music(repo, pack, data, *, save=False):
    world, room = data.get("world"), data.get("room")
    candidates = [path for path in kinds.world_files(pack) if f"world:{path.stem}" == world]
    if len(candidates) != 1 or not isinstance(room, str):
        raise ValueError("Select a room in a world from this pack.")
    path = _source_path(candidates[0], repo, pack)
    tool = str(Path(__file__).resolve().parents[2] / "ambition_ldtk_tools")
    if tool not in sys.path:
        sys.path.insert(0, tool)
    from ambition_ldtk_tools import generated_record
    from ambition_ldtk_tools.edit.level_set_field import apply_level_field_edit
    from ambition_ldtk_tools.ldtk.io import write_project

    raw = path.read_bytes()
    project = json.loads(raw)
    matches = [level for level in project.get("levels", []) if level.get("identifier") == room]
    if len(matches) != 1:
        raise ValueError("The selected room is absent from the world.")
    area = _active_area(matches[0])
    levels = [level for level in project["levels"] if _active_area(level) == area]
    try:
        record = generated_record.load(path)
    except SystemExit as error:
        raise ValueError(str(error)) from error
    generated = set(record["generated"]) - set(record["ldtk_owned"])
    definitions = {field["identifier"]: field for field in project["defs"]["levelFields"]}
    tracks = _tracks(pack)
    fields = {}
    targets = {}
    for field in MUSIC_FIELDS:
        owners = [level for level in levels if _level_music(level, field)]
        editable = [level for level in levels if level["identifier"] not in generated]
        definition = definitions.get(field, {})
        reason = ""
        if (definition.get("__type") or definition.get("type")) != "String":
            reason = "The world needs an LDtk string field for this assignment."
        elif not editable or any(level["identifier"] in generated for level in owners):
            reason = "This assignment belongs to the room generator."
        fields[field] = {"value": _level_music(owners[0], field) if owners else "", "reason": reason}
        targets[field] = owners or editable[:1]

    def response():
        values = {level["identifier"]: {field: _level_music(level, field) for field in MUSIC_FIELDS} for level in levels}
        revision = _revision([str(path), area, values, sorted(set(values) & generated)])
        return {"area": area, "fields": fields, "levels": values, "revision": revision}

    if not save:
        return dict(response(), tracks=tracks)
    _check_revision(data.get("revision"), response()["revision"])
    desired = data.get("tracks")
    if not isinstance(desired, dict) or not desired or not set(desired).issubset(MUSIC_FIELDS):
        raise ValueError("Choose room music or fight music.")
    changed = False
    for field, value in desired.items():
        state = fields[field]
        if state["reason"]:
            raise ValueError(state["reason"])
        if not isinstance(value, str) or (value and value not in tracks):
            raise ValueError("Choose a track from the selected pack's music registry.")
        if value == state["value"]:
            continue
        empty = None if definitions[field].get("canBeNull") else ""
        for offset, level in enumerate(targets[field]):
            apply_level_field_edit(project, level, field, value if value and offset == 0 else empty)
        fields[field]["value"] = value
        changed = True
    if changed:
        _replace(path, raw, lambda staged: write_project(staged, project))
    return response()
