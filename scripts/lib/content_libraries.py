"""Resolve runtime files from explicit content library manifests."""
from __future__ import annotations

from dataclasses import dataclass
import json
import os
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
        def is_managed(logical: str) -> bool:
            return bool(selected and logical in selected["files"]) or any(logical == prefix or logical.startswith(prefix + "/") for prefix in managed)
        files: dict[str, Path] = {}
        for role, declared in self.manifest["asset_roots"].items():
            root = self.path.parent / relative(declared)
            candidates = []
            for directory, folders, names in os.walk(root):
                folders[:] = sorted(folder for folder in folders if not folder.startswith(".") and
                                    not (role != "authored" and is_managed((Path(directory) / folder).relative_to(root).as_posix())))
                candidates.extend(Path(directory) / name for name in sorted(names) if not name.startswith("."))
            for path in candidates:
                if not path.is_file():
                    continue
                logical = path.relative_to(root).as_posix()
                if role != "authored" and is_managed(logical):
                    continue
                if logical in files:
                    raise ValueError(f"Library {self.id!r} declares {logical!r} twice: {files[logical]} and {path}")
                files[logical] = path
        if selected:
            collision = set(files) & set(selected["files"])
            if collision:
                raise ValueError(f"Authored files collide with published products: {sorted(collision)}")
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
    import os
    if os.environ.get("AMBITION_GENERATION_ROOT"):
        return Path(os.environ["AMBITION_GENERATION_ROOT"])
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


def runtime_files(repo: Path) -> dict[str, Path]:
    library = next(iter(read_libraries([selected_manifest(repo)]).values()))
    return library.files()
