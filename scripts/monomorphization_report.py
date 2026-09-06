#!/usr/bin/env python3
"""Measure Rust monomorphization pressure and Cargo target-directory bloat.

This tool deliberately separates four observations that are easy to conflate:

1. ``target`` inventory: how many bytes are retained as Cargo artifacts and how
   much hashed-variant multiplicity exists for each logical artifact;
2. emitted text symbols: how much machine code an already-built ELF/rlib carries,
   grouped into conservative generic symbol families using GNU ``nm`` or ``llvm-nm``;
3. section/archive composition: how much of one artifact is text, debug data,
   relocations, symbol/string tables, Rust/compiler metadata, or container overhead;
4. rustc mono items: an optional nightly capture of the compiler's own
   monomorphization collector (``-Zprint-mono-items=lazy``).

The first two modes are read-only and do not compile anything.  The nightly mode
is explicit because changing rustc flags can create a large second artifact set;
it uses an isolated target directory by default so a diagnostic cannot poison the
normal development cache.

Examples, from the repository root::

    python3 scripts/monomorphization_report.py target
    python3 scripts/monomorphization_report.py symbols --find app_it
    python3 scripts/monomorphization_report.py symbols \
        --find libambition_platformer2d_runtime --find app_it
    python3 scripts/monomorphization_report.py sections --find app_it

    # Heavy/optional: asks nightly rustc what it actually monomorphizes.
    python3 scripts/monomorphization_report.py capture \
        --package ambition_platformer2d_runtime --lib

    python3 scripts/monomorphization_report.py mono-log path/to/capture.log

Reports are JSON plus Markdown under ``target/monomorphization_reports`` unless
``--output`` chooses another directory.  No report is a build gate: the point is
to find a small number of expensive generic families worth investigating.
"""

from __future__ import annotations

import argparse
import dataclasses
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import statistics
import subprocess
import sys
from typing import Iterable, Sequence

ROOT = Path(__file__).resolve().parents[1]
_HEX_HASH = r"[0-9a-f]{8,32}"
_ARTIFACT_RE = re.compile(
    rf"^(?P<lib>lib)?(?P<stem>.+?)-(?P<hash>{_HEX_HASH})(?P<suffix>\.(?:rlib|rmeta|so|a|dylib))?$"
)
_TRAILING_RUST_HASH = re.compile(r"::h[0-9a-f]{16}$")
_MONO_ITEM = re.compile(r"^\s*MONO_ITEM\s+(?P<item>.+?)(?:\s+@@.*)?\s*$")
_NM_LINE = re.compile(
    r"^(?:(?P<container>.*?):)?"
    r"(?P<address>[0-9A-Fa-f]+)\s+"
    r"(?P<size>[0-9A-Fa-f]+)\s+"
    r"(?P<kind>\S)\s+"
    r"(?P<name>.+)$"
)
_READELF_FILE = re.compile(r"^File:\s+(?P<member>.+)$")
_READELF_SECTION = re.compile(
    r"^\s*\[\s*\d+\]\s+"
    r"(?P<name>\S+)\s+"
    r"(?P<type>\S+)\s+"
    r"[0-9A-Fa-f]+\s+"
    r"[0-9A-Fa-f]+\s+"
    r"(?P<size>[0-9A-Fa-f]+)\s+"
)
_CODE_SYMBOL_KINDS = frozenset("TtWw")


@dataclasses.dataclass(frozen=True)
class ArtifactFile:
    path: Path
    logical: str
    profile: str
    size: int


@dataclasses.dataclass(frozen=True)
class Symbol:
    artifact: Path
    size: int
    kind: str
    name: str
    family: str
    owner: str


def human_bytes(value: int | float) -> str:
    value = float(value)
    for suffix in ("B", "KiB", "MiB", "GiB", "TiB"):
        if abs(value) < 1024.0 or suffix == "TiB":
            if suffix == "B":
                return f"{int(value)} {suffix}"
            return f"{value:.2f} {suffix}"
        value /= 1024.0
    raise AssertionError("unreachable")


def cargo_target_directory(root: Path = ROOT) -> Path:
    env = os.environ.get("CARGO_TARGET_DIR")
    if env:
        path = Path(env)
        return path if path.is_absolute() else (root / path).resolve()
    cargo = shutil.which("cargo")
    if cargo:
        proc = subprocess.run(
            [cargo, "metadata", "--format-version=1", "--no-deps"],
            cwd=root,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        if proc.returncode == 0:
            try:
                return Path(json.loads(proc.stdout)["target_directory"]).resolve()
            except (KeyError, json.JSONDecodeError):
                pass
    return (root / "target").resolve()


def file_size(path: Path) -> int:
    try:
        return path.stat().st_size
    except OSError:
        return 0


def tree_size(path: Path) -> int:
    if not path.exists():
        return 0
    total = 0
    for base, _dirs, names in os.walk(path):
        for name in names:
            total += file_size(Path(base) / name)
    return total


def logical_artifact_name(name: str) -> str | None:
    match = _ARTIFACT_RE.match(name)
    if not match:
        return None
    suffix = match.group("suffix") or "<link-product>"
    lib = "lib" if match.group("lib") else ""
    return f"{lib}{match.group('stem')}{suffix}"


def artifact_files(target: Path) -> list[ArtifactFile]:
    rows: list[ArtifactFile] = []
    if not target.exists():
        return rows
    # Hashed Cargo artifacts live in PROFILE/deps. Scanning only these avoids
    # counting build-script output payloads as if they were crate variants.
    for deps in sorted(target.glob("*/deps")):
        if not deps.is_dir():
            continue
        profile = deps.parent.name
        for path in deps.iterdir():
            if not path.is_file():
                continue
            logical = logical_artifact_name(path.name)
            if logical is None:
                continue
            rows.append(ArtifactFile(path=path, logical=logical, profile=profile, size=file_size(path)))
    return rows


def target_inventory(target: Path) -> dict[str, object]:
    rows = artifact_files(target)
    groups: dict[tuple[str, str], list[ArtifactFile]] = {}
    for row in rows:
        groups.setdefault((row.profile, row.logical), []).append(row)

    variants = []
    for (profile, logical), members in groups.items():
        if len(members) < 2:
            continue
        sizes = [member.size for member in members]
        total = sum(sizes)
        # This is deliberately named RETAINED EXCESS, not duplicate bytes.  The
        # variants may correspond to genuinely different feature/profile shapes;
        # it answers how much disk all-but-one current variant costs, not how much
        # code the compiler could safely merge.
        retained_excess = total - max(sizes)
        variants.append(
            {
                "profile": profile,
                "artifact": logical,
                "variants": len(members),
                "total_bytes": total,
                "largest_bytes": max(sizes),
                "retained_excess_bytes": retained_excess,
                "paths": [str(member.path) for member in sorted(members, key=lambda x: x.size, reverse=True)],
            }
        )
    variants.sort(key=lambda row: int(row["retained_excess_bytes"]), reverse=True)

    profile_regions: list[dict[str, object]] = []
    for profile in sorted(path for path in target.iterdir() if path.is_dir()) if target.exists() else []:
        profile_regions.append(
            {
                "profile": profile.name,
                "total_bytes": tree_size(profile),
                "deps_bytes": tree_size(profile / "deps"),
                "incremental_bytes": tree_size(profile / "incremental"),
                "build_bytes": tree_size(profile / "build"),
            }
        )

    return {
        "target_directory": str(target),
        "total_bytes": tree_size(target),
        "artifact_files": len(rows),
        "profile_regions": profile_regions,
        "variant_groups": variants,
        "variant_retained_excess_bytes": sum(int(row["retained_excess_bytes"]) for row in variants),
    }


def _replace_balanced_turbofish(text: str) -> str:
    """Replace ``::<...>`` groups while preserving impl-qualified ``<T as Trait>``.

    This conservative family is intentionally an UNDER-grouping.  It catches the
    common monomorphization spelling (``foo::<A>``) without pretending that
    ``<A as Trait>::method`` and ``<B as Trait>::method`` are necessarily copies
    of one generic definition.
    """
    out: list[str] = []
    i = 0
    while i < len(text):
        if text.startswith("::<", i):
            out.append("::<...>")
            depth = 1
            i += 3
            while i < len(text) and depth:
                if text[i] == "<":
                    depth += 1
                elif text[i] == ">":
                    depth -= 1
                i += 1
            continue
        out.append(text[i])
        i += 1
    return "".join(out)


def generic_family(name: str) -> str:
    return _replace_balanced_turbofish(_TRAILING_RUST_HASH.sub("", name))


def symbol_owner(name: str) -> str:
    cleaned = _TRAILING_RUST_HASH.sub("", name).lstrip("<")
    # Normal functions start crate::..., while trait-qualified methods start
    # <crate::Type as dep::Trait>::... . In either case the first Rust path
    # segment is a useful ownership hint, not an authoritative DefId.
    match = re.search(r"([A-Za-z_][A-Za-z0-9_]*)::", cleaned)
    return match.group(1) if match else "<unknown>"


def parse_nm_output(text: str, artifact: Path) -> list[Symbol]:
    symbols: list[Symbol] = []
    for line in text.splitlines():
        match = _NM_LINE.match(line.strip())
        if not match or match.group("kind") not in _CODE_SYMBOL_KINDS:
            continue
        try:
            size = int(match.group("size"), 16)
        except ValueError:
            continue
        if size <= 0:
            continue
        name = match.group("name")
        symbols.append(
            Symbol(
                artifact=artifact,
                size=size,
                kind=match.group("kind"),
                name=name,
                family=generic_family(name),
                owner=symbol_owner(name),
            )
        )
    return symbols


def nm_executable() -> str | None:
    # Prefer GNU nm when both are installed because it can request the Rust
    # demangling style explicitly. LLVM nm is equally usable, but its --demangle
    # option is a boolean and rejects GNU's --demangle=rust spelling.
    for candidate in ("nm", "llvm-nm"):
        path = shutil.which(candidate)
        if path:
            return path
    return None


def _nm_flavor(nm: str) -> str:
    """Best-effort GNU-vs-LLVM classification without running the tool."""
    name = Path(nm).name.lower()
    return "llvm" if "llvm-nm" in name else "gnu"


def _nm_command(nm: str, path: Path, *, flavor: str | None = None) -> list[str]:
    """Build a size-bearing, defined-text-symbol command for GNU or LLVM nm.

    Both implementations support print-size, size-sort and defined-only. Their
    demangling command lines differ: GNU accepts a style value, while LLVM's
    option is boolean. Keeping this difference here prevents tool discovery order
    from changing whether symbol collection works.
    """
    flavor = flavor or _nm_flavor(nm)
    demangle = "--demangle" if flavor == "llvm" else "--demangle=rust"
    return [nm, "--print-size", "--size-sort", "--defined-only", demangle, str(path)]


def symbols_for_artifact(path: Path, nm: str | None = None) -> tuple[list[Symbol], str | None]:
    nm = nm or nm_executable()
    if nm is None:
        return [], "neither nm nor llvm-nm is installed"

    # File names are normally enough to identify llvm-nm. This retry is cheap compared with
    # scanning the artifact and makes the reader robust to distro alternatives/symlinks.
    first_flavor = _nm_flavor(nm)
    flavors = (first_flavor, "gnu" if first_flavor == "llvm" else "llvm")
    failures: list[str] = []
    for flavor in flavors:
        proc = subprocess.run(
            _nm_command(nm, path, flavor=flavor),
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        if proc.returncode == 0:
            return parse_nm_output(proc.stdout, path), None
        failures.append(proc.stderr.strip() or f"{Path(nm).name} exited {proc.returncode}")

    detail = " | fallback: ".join(failures)
    return [], detail


def symbol_report(paths: Sequence[Path], *, top: int = 40, nm: str | None = None) -> dict[str, object]:
    all_symbols: list[Symbol] = []
    errors: list[str] = []
    artifacts: list[dict[str, object]] = []
    for path in paths:
        symbols, error = symbols_for_artifact(path, nm)
        if error:
            errors.append(f"{path}: {error}")
            continue
        all_symbols.extend(symbols)
        artifacts.append(
            {
                "path": str(path),
                "file_bytes": file_size(path),
                "code_symbols": len(symbols),
                "named_text_bytes": sum(symbol.size for symbol in symbols),
            }
        )

    families: dict[str, list[Symbol]] = {}
    owners: dict[str, list[Symbol]] = {}
    for symbol in all_symbols:
        families.setdefault(symbol.family, []).append(symbol)
        owners.setdefault(symbol.owner, []).append(symbol)

    family_rows = []
    for family, members in families.items():
        # A family with one concrete symbol says nothing about monomorphization
        # multiplication, so it stays out of the pressure ranking.
        concrete = {member.name for member in members}
        if len(concrete) < 2:
            continue
        sizes = [member.size for member in members]
        total = sum(sizes)
        family_rows.append(
            {
                "family": family,
                "instances": len(concrete),
                "symbols": len(members),
                "total_text_bytes": total,
                "largest_instance_bytes": max(sizes),
                "median_instance_bytes": int(statistics.median(sizes)),
                # Pressure proxy only.  Different instantiations may genuinely
                # need different machine code, so this must never be presented
                # as automatically recoverable bytes.
                "single_body_pressure_bytes": total - max(sizes),
                "examples": sorted(concrete)[:5],
            }
        )
    family_rows.sort(key=lambda row: int(row["total_text_bytes"]), reverse=True)

    owner_rows = [
        {
            "owner": owner,
            "symbols": len(members),
            "named_text_bytes": sum(member.size for member in members),
        }
        for owner, members in owners.items()
    ]
    owner_rows.sort(key=lambda row: int(row["named_text_bytes"]), reverse=True)

    largest = sorted(all_symbols, key=lambda symbol: symbol.size, reverse=True)[:top]
    return {
        "artifacts": artifacts,
        "errors": errors,
        "named_text_bytes": sum(symbol.size for symbol in all_symbols),
        "code_symbols": len(all_symbols),
        "generic_family_count": len(family_rows),
        "family_pressure_bytes": sum(int(row["single_body_pressure_bytes"]) for row in family_rows),
        "families": family_rows[:top],
        "owners": owner_rows[:top],
        "largest_symbols": [
            {
                "artifact": str(symbol.artifact),
                "size": symbol.size,
                "owner": symbol.owner,
                "name": symbol.name,
            }
            for symbol in largest
        ],
    }


def find_artifacts(target: Path, names: Sequence[str]) -> list[Path]:
    found: list[Path] = []
    for name in names:
        candidates: list[Path] = []
        for deps in target.glob("*/deps"):
            for path in deps.glob(f"{name}*"):
                if not path.is_file():
                    continue
                # Skip metadata/depfiles when the caller asked for a link product.
                if path.suffix in {".d", ".rmeta"}:
                    continue
                candidates.append(path)
        if not candidates:
            raise SystemExit(f"no artifact matching {name!r} under {target}/*/deps")
        # The newest artifact is usually the current fingerprint variant and is
        # far more useful than silently analysing every stale copy.
        found.append(max(candidates, key=lambda path: path.stat().st_mtime_ns))
    return found



def section_category(name: str, section_type: str) -> str:
    """Bucket ELF sections by what makes an artifact large on disk.

    The categories are diagnostic rather than ABI semantics. The point is to
    distinguish machine code from the packaging costs that monomorphization can
    amplify indirectly: debug lines, relocations, long symbol names, archive
    metadata, and compiler metadata.
    """
    if name.startswith((".debug", ".zdebug", ".gnu_debug")) or name == ".gdb_index":
        return "debug"
    if section_type in {"REL", "RELA", "RELR"} or name.startswith((".rel.", ".rela.")):
        return "relocations"
    if section_type in {"SYMTAB", "DYNSYM", "SYMTAB_SHNDX"} or name in {".symtab", ".dynsym"}:
        return "symbol tables"
    if section_type == "STRTAB" or name in {".strtab", ".shstrtab", ".dynstr"}:
        return "string tables"
    if name.startswith((".text", ".init", ".fini", ".plt", ".iplt")):
        return "code"
    if name.startswith((".rodata", ".eh_frame", ".gcc_except_table", ".data.rel.ro")):
        return "read-only data/unwind"
    if name.startswith((".data", ".bss", ".tdata", ".tbss", ".got", ".init_array", ".fini_array", ".ctors", ".dtors")):
        return "writable/TLS data"
    if name.startswith((".rustc", ".llvm", ".llvmbc")):
        return "compiler metadata"
    if name.startswith((".gnu.hash", ".hash", ".dynamic", ".gnu.version", ".interp")):
        return "dynamic/link metadata"
    if name.startswith(".note") or name == ".comment":
        return "notes/build metadata"
    return "other"


def parse_readelf_sections(text: str, artifact: Path) -> list[dict[str, object]]:
    """Parse ``readelf -SW`` output for an ELF file or every ELF member of an archive."""
    rows: list[dict[str, object]] = []
    member = str(artifact)
    for raw in text.splitlines():
        line = raw.rstrip()
        file_match = _READELF_FILE.match(line)
        if file_match:
            member = file_match.group("member")
            continue
        match = _READELF_SECTION.match(line)
        if not match:
            continue
        name = match.group("name")
        section_type = match.group("type")
        size = int(match.group("size"), 16)
        # NOBITS (normally .bss/.tbss) consumes virtual address space but has no
        # payload bytes in the ELF/archive file. Keeping both quantities avoids
        # falsely explaining disk use with zero-filled runtime memory.
        disk_bytes = 0 if section_type == "NOBITS" else size
        rows.append(
            {
                "member": member,
                "name": name,
                "type": section_type,
                "declared_bytes": size,
                "disk_bytes": disk_bytes,
                "category": section_category(name, section_type),
            }
        )
    return rows


def readelf_executable() -> str | None:
    for candidate in ("readelf", "llvm-readelf"):
        path = shutil.which(candidate)
        if path:
            return path
    return None


def sections_for_artifact(path: Path, readelf: str | None = None) -> tuple[list[dict[str, object]], str | None]:
    readelf = readelf or readelf_executable()
    if readelf is None:
        return [], "neither readelf nor llvm-readelf is installed"
    proc = subprocess.run(
        [readelf, "-SW", str(path)],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    rows = parse_readelf_sections(proc.stdout, path)
    # Rust rlibs can contain non-ELF metadata members. GNU readelf may complain
    # about those members while still printing useful section tables for every
    # object member, so parsed rows win over the process return code.
    if rows:
        warning = proc.stderr.strip() if proc.returncode != 0 and proc.stderr.strip() else None
        return rows, warning
    detail = proc.stderr.strip() or f"{Path(readelf).name} exited {proc.returncode} without section rows"
    return [], detail


def section_report(paths: Sequence[Path], *, top: int = 30, readelf: str | None = None) -> dict[str, object]:
    artifacts: list[dict[str, object]] = []
    errors: list[str] = []
    warnings: list[str] = []
    for path in paths:
        rows, warning = sections_for_artifact(path, readelf)
        if not rows:
            errors.append(f"{path}: {warning or 'no section rows'}")
            continue
        if warning:
            warnings.append(f"{path}: {warning}")

        categories: dict[str, dict[str, int]] = {}
        names: dict[tuple[str, str], dict[str, int | str]] = {}
        members: set[str] = set()
        for row in rows:
            members.add(str(row["member"]))
            category = str(row["category"])
            bucket = categories.setdefault(category, {"sections": 0, "declared_bytes": 0, "disk_bytes": 0})
            bucket["sections"] += 1
            bucket["declared_bytes"] += int(row["declared_bytes"])
            bucket["disk_bytes"] += int(row["disk_bytes"])

            key = (str(row["name"]), category)
            item = names.setdefault(
                key,
                {"name": key[0], "category": category, "occurrences": 0, "declared_bytes": 0, "disk_bytes": 0},
            )
            item["occurrences"] = int(item["occurrences"]) + 1
            item["declared_bytes"] = int(item["declared_bytes"]) + int(row["declared_bytes"])
            item["disk_bytes"] = int(item["disk_bytes"]) + int(row["disk_bytes"])

        file_bytes = file_size(path)
        accounted = sum(bucket["disk_bytes"] for bucket in categories.values())
        virtual_nobits = sum(
            int(row["declared_bytes"]) for row in rows if str(row["type"]) == "NOBITS"
        )
        category_rows = [
            {"category": category, **values}
            for category, values in categories.items()
        ]
        category_rows.sort(key=lambda row: int(row["disk_bytes"]), reverse=True)
        section_rows = sorted(names.values(), key=lambda row: int(row["disk_bytes"]), reverse=True)[:top]
        artifacts.append(
            {
                "path": str(path),
                "file_bytes": file_bytes,
                "elf_members": len(members),
                "section_rows": len(rows),
                "accounted_section_disk_bytes": accounted,
                "residual_container_bytes": max(0, file_bytes - accounted),
                "virtual_nobits_bytes": virtual_nobits,
                "categories": category_rows,
                "largest_section_names": section_rows,
            }
        )

    return {"artifacts": artifacts, "errors": errors, "warnings": warnings}


def parse_mono_items(text: str) -> list[str]:
    items = []
    for line in text.splitlines():
        match = _MONO_ITEM.match(line)
        if match:
            items.append(match.group("item"))
    return items


def mono_report(items: Sequence[str], top: int = 60) -> dict[str, object]:
    families: dict[str, set[str]] = {}
    kinds: dict[str, int] = {}
    for item in items:
        kind = item.split(" ", 1)[0] if " " in item else "other"
        kinds[kind] = kinds.get(kind, 0) + 1
        # MONO_ITEM prefixes are normally ``fn`` / ``static``. Family formation
        # only strips concrete turbofish arguments, matching the symbol census.
        family = generic_family(item)
        families.setdefault(family, set()).add(item)
    rows = [
        {"family": family, "instances": len(members), "examples": sorted(members)[:5]}
        for family, members in families.items()
        if len(members) >= 2
    ]
    rows.sort(key=lambda row: int(row["instances"]), reverse=True)
    return {
        "mono_items": len(items),
        "kinds": dict(sorted(kinds.items())),
        "multi_instance_families": len(rows),
        "families": rows[:top],
    }


def markdown_report(data: dict[str, object]) -> str:
    mode = str(data.get("mode", "report"))
    lines = [f"# Rust monomorphization report — {mode}", ""]
    lines.append(f"Generated: {data['generated_at']}")
    lines.append("")
    if mode == "target":
        inv = data["target"]
        assert isinstance(inv, dict)
        lines.extend(
            [
                f"Target directory: `{inv['target_directory']}`",
                f"Total retained: **{human_bytes(int(inv['total_bytes']))}**",
                f"Hashed artifact files: {inv['artifact_files']}",
                f"All-but-largest bytes across multi-variant artifact groups: **{human_bytes(int(inv['variant_retained_excess_bytes']))}**",
                "",
                "## Profile regions",
                "",
                "| profile | total | deps | incremental | build |",
                "|---|---:|---:|---:|---:|",
            ]
        )
        for row in inv["profile_regions"]:
            lines.append(
                f"| {row['profile']} | {human_bytes(row['total_bytes'])} | {human_bytes(row['deps_bytes'])} | {human_bytes(row['incremental_bytes'])} | {human_bytes(row['build_bytes'])} |"
            )
        lines.extend(["", "## Largest retained variant groups", "", "| artifact | variants | total | all but largest |", "|---|---:|---:|---:|"])
        for row in inv["variant_groups"][:40]:
            lines.append(
                f"| `{row['artifact']}` | {row['variants']} | {human_bytes(row['total_bytes'])} | {human_bytes(row['retained_excess_bytes'])} |"
            )
        lines.extend(
            [
                "",
                "> Variant excess is disk pressure, not proof of duplicate machine code. Different hashes can represent legitimately different feature/profile configurations.",
            ]
        )
    elif mode == "symbols":
        rep = data["symbols"]
        assert isinstance(rep, dict)
        lines.extend(
            [
                f"Named text represented by parsed code symbols: **{human_bytes(int(rep['named_text_bytes']))}** across {rep['code_symbols']} symbols.",
                f"Generic-family pressure proxy: **{human_bytes(int(rep['family_pressure_bytes']))}**.",
                "",
                "## Generic families by emitted text",
                "",
                "| family | instances | total text | all but largest | median instance |",
                "|---|---:|---:|---:|---:|",
            ]
        )
        for row in rep["families"]:
            lines.append(
                f"| `{row['family']}` | {row['instances']} | {human_bytes(row['total_text_bytes'])} | {human_bytes(row['single_body_pressure_bytes'])} | {human_bytes(row['median_instance_bytes'])} |"
            )
        lines.extend(
            [
                "",
                "> The all-but-largest column is a pressure proxy, not predicted savings. Concrete instantiations can require genuinely different code.",
                "",
                "## Code owner prefixes",
                "",
                "| owner hint | symbols | named text |",
                "|---|---:|---:|",
            ]
        )
        for row in rep["owners"]:
            lines.append(f"| `{row['owner']}` | {row['symbols']} | {human_bytes(row['named_text_bytes'])} |")
        if rep["errors"]:
            lines.extend(["", "## Symbol-reader errors", ""])
            lines.extend(f"- {error}" for error in rep["errors"])
    elif mode == "sections":
        rep = data["sections"]
        assert isinstance(rep, dict)
        for artifact in rep["artifacts"]:
            file_bytes = int(artifact["file_bytes"])
            lines.extend(
                [
                    f"## `{artifact['path']}`",
                    "",
                    f"File size: **{human_bytes(file_bytes)}**",
                    f"ELF/archive members observed: {artifact['elf_members']}",
                    f"File-backed section bytes observed: **{human_bytes(artifact['accounted_section_disk_bytes'])}**",
                    f"Residual container/header/non-ELF bytes: **{human_bytes(artifact['residual_container_bytes'])}**",
                    f"NOBITS virtual bytes (not file payload): {human_bytes(artifact['virtual_nobits_bytes'])}",
                    "",
                    "| category | file-backed bytes | declared bytes | sections | % of file |",
                    "|---|---:|---:|---:|---:|",
                ]
            )
            for row in artifact["categories"]:
                pct = (100.0 * int(row["disk_bytes"]) / file_bytes) if file_bytes else 0.0
                lines.append(
                    f"| {row['category']} | {human_bytes(row['disk_bytes'])} | {human_bytes(row['declared_bytes'])} | {row['sections']} | {pct:.1f}% |"
                )
            lines.extend(
                [
                    "",
                    "### Largest section-name aggregates",
                    "",
                    "| section | category | occurrences | file-backed bytes |",
                    "|---|---|---:|---:|",
                ]
            )
            for row in artifact["largest_section_names"]:
                lines.append(
                    f"| `{row['name']}` | {row['category']} | {row['occurrences']} | {human_bytes(row['disk_bytes'])} |"
                )
            lines.extend(
                [
                    "",
                    "> Residual bytes are not automatically waste. They include ELF/archive headers and alignment, and for rlibs can include non-ELF rustc metadata members that readelf does not expose as sections.",
                    "",
                ]
            )
        if rep["warnings"]:
            lines.extend(["## Section-reader warnings", ""])
            lines.extend(f"- {warning}" for warning in rep["warnings"])
            lines.append("")
        if rep["errors"]:
            lines.extend(["## Section-reader errors", ""])
            lines.extend(f"- {error}" for error in rep["errors"])
            lines.append("")
    elif mode in {"mono-log", "capture"}:
        rep = data["mono"]
        assert isinstance(rep, dict)
        lines.extend(
            [
                f"rustc mono items: **{rep['mono_items']}**",
                f"Families with more than one concrete mono item: **{rep['multi_instance_families']}**",
                "",
                "## Families by instantiation count",
                "",
                "| family | instances |",
                "|---|---:|",
            ]
        )
        for row in rep["families"]:
            lines.append(f"| `{row['family']}` | {row['instances']} |")
    lines.append("")
    return "\n".join(lines)


def output_directory(base: Path | None) -> Path:
    if base is not None:
        return base
    stamp = dt.datetime.now().astimezone().strftime("%Y%m%dT%H%M%S%z")
    return cargo_target_directory() / "monomorphization_reports" / stamp


def write_report(data: dict[str, object], destination: Path, *, raw_log: str | None = None) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    (destination / "report.json").write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    (destination / "report.md").write_text(markdown_report(data), encoding="utf-8")
    if raw_log is not None:
        (destination / "rustc-mono.log").write_text(raw_log, encoding="utf-8")
    print(markdown_report(data))
    print(f"Report: {destination / 'report.md'}")
    print(f"Raw data: {destination / 'report.json'}")


def envelope(mode: str) -> dict[str, object]:
    return {
        "schema": 1,
        "mode": mode,
        "generated_at": dt.datetime.now().astimezone().isoformat(timespec="seconds"),
    }


def capture_command(args: argparse.Namespace, destination: Path) -> tuple[str, list[str]]:
    cargo = shutil.which("cargo")
    if cargo is None:
        raise SystemExit("cargo is required for capture mode")
    # Probe rustc itself. Cargo also has unstable ``-Z`` flags, so asking Cargo
    # would be ambiguous about which tool accepted the option.
    rustc_probe = subprocess.run(
        ["rustc", "+nightly", "-Z", "help"],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    ) if shutil.which("rustc") else None
    if rustc_probe is None or rustc_probe.returncode != 0 or "print-mono-items" not in rustc_probe.stdout:
        raise SystemExit(
            "nightly rustc with -Zprint-mono-items is required for capture mode; "
            "install/update the nightly toolchain, or use target/symbols/mono-log modes"
        )

    target_dir = args.capture_target_dir or (destination / "cargo-target")
    cmd = [cargo, "+nightly", "rustc", "--target-dir", str(target_dir), "-p", args.package]
    if args.lib:
        cmd.append("--lib")
    if args.bin:
        cmd.extend(["--bin", args.bin])
    if args.features:
        cmd.extend(["--features", args.features])
    cmd.extend(["--", "-Zprint-mono-items=lazy"])
    return "nightly rustc mono-item capture", cmd


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--output", type=Path, help="report directory (default: target/monomorphization_reports/<timestamp>)")
    sub = parser.add_subparsers(dest="command", required=True)

    target = sub.add_parser("target", help="read-only target-directory artifact inventory")
    target.add_argument("--target-dir", type=Path, help="override Cargo target directory")

    symbols = sub.add_parser("symbols", help="read-only emitted machine-code symbol census")
    symbols.add_argument("--artifact", action="append", type=Path, default=[], help="ELF/rlib to inspect; repeatable")
    symbols.add_argument("--find", action="append", default=[], help="find newest matching artifact under PROFILE/deps; repeatable")
    symbols.add_argument("--target-dir", type=Path, help="target directory used by --find")
    symbols.add_argument("--top", type=int, default=40)
    symbols.add_argument("--nm", help="explicit nm/llvm-nm path")

    sections = sub.add_parser("sections", help="read-only ELF/rlib section and archive-composition census")
    sections.add_argument("--artifact", action="append", type=Path, default=[], help="ELF/rlib to inspect; repeatable")
    sections.add_argument("--find", action="append", default=[], help="find newest matching artifact under PROFILE/deps; repeatable")
    sections.add_argument("--target-dir", type=Path, help="target directory used by --find")
    sections.add_argument("--top", type=int, default=30)
    sections.add_argument("--readelf", help="explicit readelf/llvm-readelf path")

    mono = sub.add_parser("mono-log", help="parse an existing rustc -Zprint-mono-items log")
    mono.add_argument("log", type=Path)
    mono.add_argument("--top", type=int, default=60)

    capture = sub.add_parser("capture", help="HEAVY: compile one crate with nightly rustc mono-item tracing")
    capture.add_argument("--package", required=True)
    shape = capture.add_mutually_exclusive_group(required=True)
    shape.add_argument("--lib", action="store_true")
    shape.add_argument("--bin")
    capture.add_argument("--features", default="")
    capture.add_argument("--capture-target-dir", type=Path, help="isolated cargo target for the diagnostic build")
    capture.add_argument("--top", type=int, default=60)

    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    destination = output_directory(args.output)
    if args.command == "target":
        target = (args.target_dir or cargo_target_directory()).resolve()
        data = envelope("target")
        data["target"] = target_inventory(target)
        write_report(data, destination)
        return 0

    if args.command == "symbols":
        target = (args.target_dir or cargo_target_directory()).resolve()
        paths = [path.resolve() for path in args.artifact]
        paths.extend(find_artifacts(target, args.find))
        # Stable de-duplication in case an explicit artifact and --find point to
        # the same file.
        paths = list(dict.fromkeys(paths))
        if not paths:
            raise SystemExit("symbols mode requires --artifact and/or --find")
        data = envelope("symbols")
        data["symbols"] = symbol_report(paths, top=args.top, nm=args.nm)
        write_report(data, destination)
        return 0

    if args.command == "sections":
        target = (args.target_dir or cargo_target_directory()).resolve()
        paths = [path.resolve() for path in args.artifact]
        paths.extend(find_artifacts(target, args.find))
        paths = list(dict.fromkeys(paths))
        if not paths:
            raise SystemExit("sections mode requires --artifact and/or --find")
        data = envelope("sections")
        data["sections"] = section_report(paths, top=args.top, readelf=args.readelf)
        write_report(data, destination)
        return 0

    if args.command == "mono-log":
        text = args.log.read_text(encoding="utf-8", errors="replace")
        items = parse_mono_items(text)
        if not items:
            raise SystemExit(f"no MONO_ITEM rows found in {args.log}")
        data = envelope("mono-log")
        data["source"] = str(args.log)
        data["mono"] = mono_report(items, top=args.top)
        write_report(data, destination)
        return 0

    if args.command == "capture":
        label, cmd = capture_command(args, destination)
        print(f"{label}: {' '.join(cmd)}", file=sys.stderr)
        env = os.environ.copy()
        env["CARGO_INCREMENTAL"] = "0"
        proc = subprocess.run(
            cmd,
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        text = proc.stdout
        items = parse_mono_items(text)
        data = envelope("capture")
        data["command"] = cmd
        data["returncode"] = proc.returncode
        data["mono"] = mono_report(items, top=args.top)
        write_report(data, destination, raw_log=text)
        if proc.returncode != 0:
            print("nightly capture build failed; raw compiler output was retained", file=sys.stderr)
            return proc.returncode
        if not items:
            print("capture succeeded but rustc emitted no MONO_ITEM rows", file=sys.stderr)
            return 2
        return 0

    raise AssertionError(args.command)


if __name__ == "__main__":
    raise SystemExit(main())
