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
class SelectedFile:
    """One member of a library's selected file set.

    `origin` is `published` for a product of the accepted publication and
    `loose` for a library file outside the roots that publication owns.
    """
    library: str
    logical: str
    path: Path
    origin: str
    generation: str | None


@dataclass(frozen=True)
class Library:
    path: Path
    manifest: dict

    @property
    def id(self) -> str:
        return self.manifest["id"]

    def publication_root(self) -> Path | None:
        store = self.manifest.get("publication")
        return self.path.parent / relative(store) if store else None

    def selection(self) -> dict | None:
        """The accepted publication, or `None` for a library without a store.

        A store that has published nothing is an empty selection, not an error,
        and a recorded identity that does not match is a freshness note (see
        `advisories`), not a refusal. Unsafe member paths still refuse.
        """
        root = self.publication_root()
        return Store(root).selected(verify_identity=False) if root else None

    def advisories(self) -> list[str]:
        """What is usable but not current: reported, never refused."""
        root = self.publication_root()
        if root is None:
            return []
        try:
            Store(root).selected()
        except ValueError as error:
            return [f"Library {self.id!r}: {error}"]
        return []

    def requires_publication(self) -> str | None:
        """Why a package of this library cannot be formed, if it cannot: it
        declares a store that has published nothing. The runtime runs without
        those products; a package has nothing to ship for them."""
        selected = self.selection()
        if selected is not None and selected["generation"] is None and not selected["files"]:
            return f"Library {self.id!r} has no publication: {self.publication_root()}"
        return None

    def selected_files(self) -> dict[str, SelectedFile]:
        """The selected file set: published products own their roots, and loose
        copies under those roots have no authority. The runtime's
        `ContentLibrary::files` reads the same set; the shared cases in
        `crates/ambition_asset_manager/tests/data/selected_files` hold both to it."""
        selected = self.selection()
        managed = set(MANAGED_ROOTS) | set(selected.get("roots", [])) if selected is not None else set()
        def is_managed(logical: str) -> bool:
            return bool(selected is not None and logical in selected["files"]) or any(
                logical == prefix or logical.startswith(prefix + "/") for prefix in managed)
        rows: dict[str, SelectedFile] = {}
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
                if logical in rows:
                    raise ValueError(f"Library {self.id!r} declares {logical!r} twice: {rows[logical].path} and {path}")
                rows[logical] = SelectedFile(self.id, logical, path, "loose", None)
        if selected is not None:
            collision = set(rows) & set(selected["files"])
            if collision:
                raise ValueError(f"An authored file collides with a published product: {sorted(collision)}")
            store = self.publication_root()
            for logical, row in selected["files"].items():
                rows[logical] = SelectedFile(self.id, logical, store / relative(row["path"]), "published", selected["generation"])
        return rows

    def files(self) -> dict[str, Path]:
        return {logical: row.path for logical, row in self.selected_files().items()}


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
