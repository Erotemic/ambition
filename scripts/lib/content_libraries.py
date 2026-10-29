"""Resolve runtime files from explicit content library manifests."""
from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path, PurePosixPath

from .art_publication import Store, MANAGED_ROOTS


def relative(value: str) -> str:
    path = PurePosixPath(value)
    if not value or path.is_absolute() or ".." in path.parts or "\\" in value:
        raise ValueError(f"Invalid library path: {value!r}")
    return value


@dataclass(frozen=True)
class Library:
    path: Path
    manifest: dict

    @property
    def id(self) -> str:
        return self.manifest["id"]

    def files(self) -> dict[str, Path]:
        selected = None
        store = self.manifest.get("publication")
        if store:
            root = self.path.parent / relative(store)
            selected = Store(root).selected()
            if selected["generation"] is None:
                raise ValueError(f"Library {self.id!r} has no publication: {root}")
        managed = set(MANAGED_ROOTS) | set(selected.get("roots", [])) if selected else set()
        files: dict[str, Path] = {}
        for declared in self.manifest["asset_roots"].values():
            root = self.path.parent / relative(declared)
            for path in sorted(root.rglob("*")):
                if not path.is_file() or any(part.startswith(".") for part in path.relative_to(root).parts):
                    continue
                logical = path.relative_to(root).as_posix()
                if logical.split("/", 1)[0] in managed:
                    continue
                if logical in files:
                    raise ValueError(f"Library {self.id!r} declares {logical!r} twice: {files[logical]} and {path}")
                files[logical] = path
        if selected:
            files.update({logical: self.path.parent / store / relative(row["path"]) for logical, row in selected["files"].items()})
        return files


def read_libraries(paths: list[Path]) -> dict[str, Library]:
    libraries = {}
    for path in paths:
        path = path.resolve()
        manifest = json.loads(path.read_text())
        if manifest.get("schema") != "ambition.content_library.v1":
            raise ValueError(f"Unsupported content library: {path}")
        name = manifest["id"]
        if not name or any(not (c.isascii() and (c.isalnum() or c in "_-")) for c in name):
            raise ValueError(f"Invalid library identity: {name!r}")
        library = Library(path, manifest)
        if name in libraries and libraries[name] != library:
            raise ValueError(f"Two selections of content library {name!r}: {libraries[name].path} and {path}")
        libraries[name] = library
    return libraries


def selected_manifest(repo: Path) -> Path:
    import os
    return repo / os.environ.get("AMBITION_CONTENT_LIBRARY", "game/ambition_content_library/library.json")


def generated_root(repo: Path) -> Path:
    path = selected_manifest(repo)
    library = next(iter(read_libraries([path]).values()))
    return path.parent / relative(library.manifest["asset_roots"]["generated"])


def producer(repo: Path, name: str) -> dict:
    path = selected_manifest(repo)
    manifest = json.loads(path.read_text())
    try:
        return manifest["producers"][name]
    except KeyError:
        raise ValueError(f"Library {path} declares no {name!r} producer") from None
