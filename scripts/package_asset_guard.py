#!/usr/bin/env python3
"""Compose and verify Ambition's distributable asset tree.

Desktop development exposes two asset roots with a content-first fallback. A
shipped build exposes one flat ``assets/`` tree. This tool is the single seam
that collapses those roots and proves that the package contains the same bytes
that desktop could resolve.

The generated contract records every regular file from the two source roots,
plus paths declared by runtime-facing catalogs/manifests. Android verifies the
contract against the finished APK ZIP. Steam Deck deployment verifies the same
contract with ``sha256sum`` after rsync.
"""

from __future__ import annotations

import argparse
import dataclasses
import fnmatch
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import sys
import tempfile
import zipfile
from collections import defaultdict
from typing import Iterable, Iterator, Sequence


ACTOR_ASSET_ROOT = Path("crates/ambition_actors/assets")
CONTENT_ASSET_ROOT = Path("game/ambition_content/assets")
CONTRACT_VERSION = 1

TEXT_MANIFEST_SUFFIXES = {".ron", ".ldtk", ".json", ".toml", ".yaml", ".yml"}
RUNTIME_ASSET_SUFFIXES = {
    ".bank",
    ".flac",
    ".jpeg",
    ".jpg",
    ".json",
    ".ldtk",
    ".mp3",
    ".ogg",
    ".otf",
    ".png",
    ".ron",
    ".ttf",
    ".wav",
    ".webp",
    ".yaml",
    ".yml",
}

# Runtime-facing field names used in authored RON / Rust catalog fragments.
# Do not scan every string literal: generated audit sidecars legitimately name
# authoring inputs that are not runtime package assets.
DECLARATION_FIELD_RE = re.compile(
    r"(?P<key_quote>[\"']?)(?P<field>spritesheet|manifest|image|asset_path|path|relPath)"
    r"(?P=key_quote)\s*:\s*(?:Some\s*\(\s*)?\"(?P<value>[^\"\n]+)\""
)
RUST_ASSET_CONST_RE = re.compile(
    r"(?:pub\s+)?(?:const|static)\s+"
    r"(?P<name>[A-Z0-9_]*(?:ASSET|SPRITE|MUSIC|SFX|FONT|WORLD)[A-Z0-9_]*)"
    r"\s*:\s*(?:&'static\s+)?&str\s*=\s*\"(?P<value>[^\"\n]+)\""
)
QUOTED_RUNTIME_PATH_RE = re.compile(
    r"\"(?P<value>[^\"\n]+\.(?:png|jpe?g|webp|ogg|wav|flac|mp3|ttf|otf|bank|ron|ldtk|json|ya?ml))\"",
    re.IGNORECASE,
)

COMMON_EXCLUDES = (
    ".git/**",
    ".*",
    "**/.*",
    "*.ipfs",
    "**/*.ipfs",
)
PROFILE_EXCLUDES = {
    "android": (),
    "steamdeck": ("fonts/local/**",),
}


class AssetContractError(RuntimeError):
    """A package cannot reproduce the desktop asset view."""


@dataclasses.dataclass
class ContractEntry:
    path: str
    sha256: str | None = None
    size: int | None = None
    origins: set[str] = dataclasses.field(default_factory=set)

    def as_json(self) -> dict[str, object]:
        return {
            "path": self.path,
            "sha256": self.sha256,
            "size": self.size,
            "origins": sorted(self.origins),
        }


@dataclasses.dataclass
class SourceFile:
    path: str
    source: Path
    root_name: str
    sha256: str
    size: int


@dataclasses.dataclass
class AssetContract:
    profile: str
    entries: dict[str, ContractEntry]
    source_roots: tuple[str, str]

    def as_json(self) -> dict[str, object]:
        return {
            "version": CONTRACT_VERSION,
            "profile": self.profile,
            "source_roots": list(self.source_roots),
            "entries": [self.entries[path].as_json() for path in sorted(self.entries)],
        }

    @classmethod
    def from_json(cls, data: dict[str, object]) -> "AssetContract":
        if data.get("version") != CONTRACT_VERSION:
            raise AssetContractError(
                f"unsupported asset contract version {data.get('version')!r}; "
                f"expected {CONTRACT_VERSION}"
            )
        raw_entries = data.get("entries")
        if not isinstance(raw_entries, list):
            raise AssetContractError("asset contract has no entries list")
        entries: dict[str, ContractEntry] = {}
        for raw in raw_entries:
            if not isinstance(raw, dict) or not isinstance(raw.get("path"), str):
                raise AssetContractError("asset contract contains a malformed entry")
            entry = ContractEntry(
                path=raw["path"],
                sha256=raw.get("sha256") if isinstance(raw.get("sha256"), str) else None,
                size=raw.get("size") if isinstance(raw.get("size"), int) else None,
                origins=set(raw.get("origins", [])) if isinstance(raw.get("origins"), list) else set(),
            )
            entries[entry.path] = entry
        roots = data.get("source_roots", [str(ACTOR_ASSET_ROOT), str(CONTENT_ASSET_ROOT)])
        if not isinstance(roots, list) or len(roots) != 2:
            raise AssetContractError("asset contract source_roots is malformed")
        return cls(
            profile=str(data.get("profile", "unknown")),
            entries=entries,
            source_roots=(str(roots[0]), str(roots[1])),
        )


def sha256_path(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def matches_any(path: str, patterns: Sequence[str]) -> bool:
    return any(fnmatch.fnmatchcase(path, pattern) for pattern in patterns)


def profile_excludes(profile: str) -> tuple[str, ...]:
    try:
        return tuple(COMMON_EXCLUDES) + tuple(PROFILE_EXCLUDES[profile])
    except KeyError as error:
        raise AssetContractError(f"unknown package profile: {profile}") from error


def normalize_package_path(raw: str, *, relative_to: str | None = None) -> str | None:
    value = raw.strip().replace("\\", "/")
    if not value or value.startswith(("http://", "https://", "ipfs://", "embedded://")):
        return None
    if value.startswith("game://"):
        value = value[len("game://") :]
    if value.startswith("assets/"):
        value = value[len("assets/") :]
    if re.match(r"^[A-Za-z][A-Za-z0-9+.-]*://", value):
        return None
    if value.startswith("/") or re.match(r"^[A-Za-z]:/", value):
        raise AssetContractError(f"absolute asset path is not package-safe: {raw!r}")

    base = PurePosixPath(relative_to).parent if relative_to else PurePosixPath()
    if value.startswith(("./", "../")):
        parts = list(base.parts)
        for part in PurePosixPath(value).parts:
            if part in ("", "."):
                continue
            if part == "..":
                if not parts:
                    raise AssetContractError(
                        f"asset path escapes the package root: {raw!r} from {relative_to!r}"
                    )
                parts.pop()
            else:
                parts.append(part)
        normalized = PurePosixPath(*parts)
    else:
        normalized = PurePosixPath(value)

    if any(part in ("", ".", "..") for part in normalized.parts):
        raise AssetContractError(f"non-normalized package asset path: {raw!r}")
    result = normalized.as_posix()
    if not result or result == ".":
        raise AssetContractError(f"empty package asset path after normalization: {raw!r}")
    return result


def is_runtime_asset_path(value: str) -> bool:
    clean = value.split("?", 1)[0].split("#", 1)[0]
    return PurePosixPath(clean).suffix.lower() in RUNTIME_ASSET_SUFFIXES


def iter_regular_files(root: Path, *, excludes: Sequence[str]) -> Iterator[tuple[str, Path]]:
    if not root.is_dir():
        raise AssetContractError(f"asset source root does not exist: {root}")
    for current, dirnames, filenames in os.walk(root, topdown=True, followlinks=False):
        current_path = Path(current)
        kept_dirs: list[str] = []
        for dirname in sorted(dirnames):
            path = current_path / dirname
            rel = path.relative_to(root).as_posix()
            if path.is_symlink() or matches_any(rel + "/", excludes) or matches_any(rel, excludes):
                continue
            kept_dirs.append(dirname)
        dirnames[:] = kept_dirs
        for filename in sorted(filenames):
            path = current_path / filename
            rel = path.relative_to(root).as_posix()
            if matches_any(rel, excludes):
                continue
            if path.is_symlink():
                continue
            if not path.is_file():
                raise AssetContractError(f"unsupported non-regular asset entry: {path}")
            yield rel, path


def assert_no_case_collisions(paths: Iterable[str], *, label: str) -> None:
    by_folded: dict[str, list[str]] = defaultdict(list)
    for path in paths:
        by_folded[path.casefold()].append(path)
    collisions = [values for values in by_folded.values() if len(set(values)) > 1]
    if collisions:
        lines = [f"case-colliding paths in {label}:"]
        for values in collisions:
            lines.append("  - " + " <> ".join(sorted(set(values))))
        raise AssetContractError("\n".join(lines))


def collect_source_files(repo: Path, profile: str) -> dict[str, SourceFile]:
    excludes = profile_excludes(profile)
    roots = (("actors", repo / ACTOR_ASSET_ROOT), ("content", repo / CONTENT_ASSET_ROOT))
    by_path: dict[str, SourceFile] = {}
    for root_name, root in roots:
        root_paths: list[str] = []
        for rel, path in iter_regular_files(root, excludes=excludes):
            normalized = normalize_package_path(rel)
            if normalized != rel:
                raise AssetContractError(f"non-canonical source asset path: {path}")
            root_paths.append(rel)
            digest = sha256_path(path)
            item = SourceFile(
                path=rel,
                source=path,
                root_name=root_name,
                sha256=digest,
                size=path.stat().st_size,
            )
            previous = by_path.get(rel)
            if previous is not None and previous.sha256 != item.sha256:
                raise AssetContractError(
                    "implicit two-root asset override is forbidden; the same package path "
                    "has different bytes:\n"
                    f"  {rel}\n"
                    f"  lower-priority: {previous.source}\n"
                    f"  higher-priority: {item.source}\n"
                    "Move the asset to one authoritative root or make the duplicate bytes identical."
                )
            by_path[rel] = item
        assert_no_case_collisions(root_paths, label=str(root))
    assert_no_case_collisions(by_path, label="composed source asset view")
    return by_path


def source_relative_path(repo: Path, source: Path) -> str:
    try:
        return source.relative_to(repo).as_posix()
    except ValueError:
        return source.as_posix()


def add_declaration(
    declarations: dict[str, set[str]],
    raw: str,
    origin: str,
    *,
    relative_to: str | None = None,
    resolve_plain_relative_to: str | None = None,
) -> None:
    if not is_runtime_asset_path(raw):
        return
    value = raw
    if (
        resolve_plain_relative_to is not None
        and not value.startswith(("./", "../", "/", "assets/", "game://"))
        and not re.match(r"^[A-Za-z][A-Za-z0-9+.-]*://", value)
    ):
        value = f"./{value}"
        relative_to = resolve_plain_relative_to
    normalized = normalize_package_path(value, relative_to=relative_to)
    if normalized is None:
        return
    declarations[normalized].add(origin)


def scan_manifest_declarations(repo: Path, source_files: dict[str, SourceFile]) -> dict[str, set[str]]:
    declarations: dict[str, set[str]] = defaultdict(set)

    # Authored package manifests. Scan only runtime-facing fields rather than
    # arbitrary string literals, so generator/audit sidecars may name source
    # inputs without turning those inputs into shipped runtime dependencies.
    for rel, source_file in source_files.items():
        suffix = PurePosixPath(rel).suffix.lower()
        if suffix not in TEXT_MANIFEST_SUFFIXES:
            continue
        try:
            text = source_file.source.read_text(encoding="utf8")
        except UnicodeDecodeError:
            continue
        sheet_manifest = rel.endswith("_spritesheet.ron") or rel.endswith("_portraits.ron")
        for match in DECLARATION_FIELD_RE.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            raw = match.group("value")
            add_declaration(
                declarations,
                raw,
                f"manifest:{rel}:{line}:{match.group('field')}",
                relative_to=rel,
                resolve_plain_relative_to=(
                    rel if sheet_manifest and match.group("field") == "image" else None
                ),
            )
        if sheet_manifest:
            # Multi-page sheet manifests place secondary images in a list, not
            # under a named field captured by DECLARATION_FIELD_RE.
            for match in QUOTED_RUNTIME_PATH_RE.finditer(text):
                raw = match.group("value")
                if PurePosixPath(raw).suffix.lower() != ".png":
                    continue
                line = text.count("\n", 0, match.start()) + 1
                add_declaration(
                    declarations,
                    raw,
                    f"sheet:{rel}:{line}",
                    relative_to=rel,
                    resolve_plain_relative_to=rel,
                )

    # Provider catalogs can be compile-time raw RON strings in Rust rather than
    # loose files. Scan production source only, and only explicit asset fields
    # and asset-named constants.
    game_root = repo / "game"
    if game_root.is_dir():
        for source in sorted(game_root.glob("*/src/**/*.rs")):
            rel_source = source_relative_path(repo, source)
            parts = source.relative_to(game_root).parts
            if "tests" in parts or source.name in {"tests.rs"}:
                continue
            text = source.read_text(encoding="utf8")
            # Inline unit-test fixtures often use realistic field names with
            # intentionally fake paths. Production modules place their cfg(test)
            # module at the end, so exclude that suffix from package declarations.
            cfg_test = re.search(r"(?m)^\s*#\[cfg\(test\)\]", text)
            if cfg_test is not None:
                text = text[: cfg_test.start()]
            for regex in (DECLARATION_FIELD_RE, RUST_ASSET_CONST_RE):
                for match in regex.finditer(text):
                    raw = match.group("value")
                    line = text.count("\n", 0, match.start()) + 1
                    label = match.groupdict().get("field") or match.groupdict().get("name") or "asset"
                    add_declaration(
                        declarations,
                        raw,
                        f"rust:{rel_source}:{line}:{label}",
                    )

    return declarations


def build_contract(repo: Path, profile: str) -> tuple[AssetContract, dict[str, SourceFile]]:
    repo = repo.resolve()
    source_files = collect_source_files(repo, profile)
    declarations = scan_manifest_declarations(repo, source_files)
    entries: dict[str, ContractEntry] = {}

    for rel, item in source_files.items():
        entries[rel] = ContractEntry(
            path=rel,
            sha256=item.sha256,
            size=item.size,
            origins={f"source:{item.root_name}:{source_relative_path(repo, item.source)}"},
        )
    for rel, origins in declarations.items():
        entry = entries.setdefault(rel, ContractEntry(path=rel))
        entry.origins.update(origins)

    missing_source = [entry for entry in entries.values() if entry.sha256 is None]
    if missing_source:
        lines = [
            "runtime-declared assets are absent from the composed desktop source roots:",
        ]
        for entry in sorted(missing_source, key=lambda item: item.path):
            lines.append(f"  - {entry.path}")
            for origin in sorted(entry.origins):
                lines.append(f"      declared by {origin}")
        raise AssetContractError("\n".join(lines))

    assert_no_case_collisions(entries, label="asset contract")
    contract = AssetContract(
        profile=profile,
        entries=entries,
        source_roots=(str(ACTOR_ASSET_ROOT), str(CONTENT_ASSET_ROOT)),
    )
    return contract, source_files


def write_json_atomic(path: Path, data: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile("w", encoding="utf8", dir=path.parent, delete=False) as file:
        json.dump(data, file, indent=2, sort_keys=True)
        file.write("\n")
        temporary = Path(file.name)
    temporary.replace(path)


def write_hash_manifest(path: Path, contract: AssetContract) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    lines = []
    for rel in sorted(contract.entries):
        entry = contract.entries[rel]
        if entry.sha256 is None:
            raise AssetContractError(f"cannot emit a hash for unresolved asset {rel}")
        if "\n" in rel or "\r" in rel:
            raise AssetContractError(f"newline in asset path is not supported: {rel!r}")
        lines.append(f"{entry.sha256}  {rel}\n")
    path.write_text("".join(lines), encoding="utf8")


def copy_composed_tree(source_files: dict[str, SourceFile], output: Path) -> None:
    output = output.resolve()
    if output == Path("/"):
        raise AssetContractError("refusing to use filesystem root as asset output")
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)
    for rel in sorted(source_files):
        item = source_files[rel]
        destination = output / PurePosixPath(rel)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(item.source, destination, follow_symlinks=False)


def walk_package_tree(root: Path) -> dict[str, Path]:
    if not root.is_dir():
        raise AssetContractError(f"packaged asset tree does not exist: {root}")
    files: dict[str, Path] = {}
    for current, dirnames, filenames in os.walk(root, topdown=True, followlinks=False):
        current_path = Path(current)
        for dirname in list(dirnames):
            path = current_path / dirname
            if path.is_symlink():
                raise AssetContractError(f"symlink is forbidden in packaged assets: {path}")
        for filename in filenames:
            path = current_path / filename
            rel = path.relative_to(root).as_posix()
            if path.is_symlink():
                raise AssetContractError(f"symlink is forbidden in packaged assets: {path}")
            if not path.is_file():
                raise AssetContractError(f"non-regular packaged asset: {path}")
            files[rel] = path
    assert_no_case_collisions(files, label=str(root))
    return files


def audit_tree(contract: AssetContract, root: Path, *, reject_extras: bool = False) -> None:
    actual = walk_package_tree(root)
    expected = contract.entries
    missing = sorted(set(expected) - set(actual))
    extras = sorted(set(actual) - set(expected))
    mismatched: list[str] = []
    for rel in sorted(set(expected) & set(actual)):
        entry = expected[rel]
        if entry.sha256 is None:
            continue
        digest = sha256_path(actual[rel])
        if digest != entry.sha256:
            mismatched.append(
                f"{rel}: expected {entry.sha256}, found {digest} ({actual[rel]})"
            )
    problems: list[str] = []
    if missing:
        problems.append("missing packaged assets:\n" + "\n".join(f"  - {path}" for path in missing))
    if mismatched:
        problems.append("packaged assets differ from the desktop source bytes:\n" + "\n".join(f"  - {item}" for item in mismatched))
    if reject_extras and extras:
        problems.append("unexpected packaged assets:\n" + "\n".join(f"  - {path}" for path in extras))
    if problems:
        raise AssetContractError("\n\n".join(problems))


def load_contract(path: Path) -> AssetContract:
    with path.open("r", encoding="utf8") as file:
        data = json.load(file)
    if not isinstance(data, dict):
        raise AssetContractError(f"asset contract root must be an object: {path}")
    return AssetContract.from_json(data)


def normalize_zip_prefix(prefix: str) -> str:
    value = prefix.replace("\\", "/").strip("/")
    return value + "/" if value else ""


def audit_zip(contract: AssetContract, archive: Path, prefix: str) -> None:
    prefix = normalize_zip_prefix(prefix)
    if not archive.is_file():
        raise AssetContractError(f"package archive does not exist: {archive}")
    with zipfile.ZipFile(archive) as zfile:
        members: dict[str, zipfile.ZipInfo] = {}
        for info in zfile.infolist():
            name = info.filename.replace("\\", "/")
            if not name.startswith(prefix) or name.endswith("/"):
                continue
            rel = name[len(prefix) :]
            normalized = normalize_package_path(rel)
            if normalized != rel:
                raise AssetContractError(f"non-canonical asset entry in {archive}: {name}")
            mode = (info.external_attr >> 16) & 0xFFFF
            if stat.S_ISLNK(mode):
                raise AssetContractError(f"symlink asset entry in {archive}: {name}")
            if rel in members:
                raise AssetContractError(f"duplicate asset entry in {archive}: {name}")
            members[rel] = info
        assert_no_case_collisions(members, label=f"{archive}!/{prefix}")

        missing = sorted(set(contract.entries) - set(members))
        mismatched: list[str] = []
        for rel in sorted(set(contract.entries) & set(members)):
            expected = contract.entries[rel]
            if expected.sha256 is None:
                continue
            with zfile.open(members[rel]) as file:
                digest = hashlib.sha256()
                for chunk in iter(lambda: file.read(1024 * 1024), b""):
                    digest.update(chunk)
            actual_hash = digest.hexdigest()
            if actual_hash != expected.sha256:
                mismatched.append(
                    f"{rel}: expected {expected.sha256}, found {actual_hash}"
                )
    problems: list[str] = []
    if missing:
        problems.append("assets missing from final package:\n" + "\n".join(f"  - {path}" for path in missing))
    if mismatched:
        problems.append("final package contains wrong asset bytes:\n" + "\n".join(f"  - {item}" for item in mismatched))
    if problems:
        raise AssetContractError("\n\n".join(problems))


def artifact_links(paths: Sequence[Path]) -> None:
    resolved = [path.resolve() for path in paths]
    try:
        from rich import print as rich_print
    except ImportError:
        for path in resolved:
            print(path)
        for parent in sorted({path.parent for path in resolved}):
            print(parent)
        return
    for path in resolved:
        rich_print(f"[link=file://{path}]{path}[/link]")
    for parent in sorted({path.parent for path in resolved}):
        rich_print(f"[link=file://{parent}]{parent}[/link]")


def command_compose(args: argparse.Namespace) -> None:
    repo = args.repo.resolve()
    contract, source_files = build_contract(repo, args.profile)
    copy_composed_tree(source_files, args.output)
    audit_tree(contract, args.output, reject_extras=True)
    write_json_atomic(args.contract, contract.as_json())
    write_hash_manifest(args.hash_manifest, contract)
    print(
        f"asset contract OK: {len(contract.entries)} files, profile={args.profile}, "
        f"tree={args.output}"
    )
    artifact_links([args.output, args.contract, args.hash_manifest])


def command_audit_tree(args: argparse.Namespace) -> None:
    contract = load_contract(args.contract)
    audit_tree(contract, args.asset_root, reject_extras=args.reject_extras)
    print(f"asset tree matches contract: {args.asset_root} ({len(contract.entries)} files)")


def command_audit_zip(args: argparse.Namespace) -> None:
    contract = load_contract(args.contract)
    audit_zip(contract, args.archive, args.prefix)
    print(
        f"final package matches asset contract: {args.archive} "
        f"({len(contract.entries)} files under {normalize_zip_prefix(args.prefix)})"
    )


def make_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    compose = subparsers.add_parser("compose", help="compose the two roots and emit a byte contract")
    compose.add_argument("--repo", type=Path, required=True)
    compose.add_argument("--profile", choices=sorted(PROFILE_EXCLUDES), required=True)
    compose.add_argument("--output", type=Path, required=True)
    compose.add_argument("--contract", type=Path, required=True)
    compose.add_argument("--hash-manifest", type=Path, required=True)
    compose.set_defaults(func=command_compose)

    tree = subparsers.add_parser("audit-tree", help="verify a composed asset directory")
    tree.add_argument("--contract", type=Path, required=True)
    tree.add_argument("--asset-root", type=Path, required=True)
    tree.add_argument("--reject-extras", action="store_true")
    tree.set_defaults(func=command_audit_tree)

    archive = subparsers.add_parser("audit-zip", help="verify assets inside an APK or ZIP")
    archive.add_argument("--contract", type=Path, required=True)
    archive.add_argument("--archive", type=Path, required=True)
    archive.add_argument("--prefix", default="assets/")
    archive.set_defaults(func=command_audit_zip)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = make_parser()
    args = parser.parse_args(argv)
    try:
        args.func(args)
    except AssetContractError as error:
        print(f"asset contract failed:\n{error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
