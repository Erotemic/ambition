#!/usr/bin/env python3
"""Download the selected library fonts. Generated files stay ignored by Git."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import sys
import tempfile
import urllib.request
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

try:
    from rich import print as rich_print
except Exception:  # pragma: no cover - convenience fallback for fresh systems
    rich_print = None


REPO_ROOT = Path(__file__).resolve().parents[1]
from lib.content_libraries import generated_root
DEFAULT_OUT_DIR = generated_root(REPO_ROOT) / "fonts/bundled"
DEFAULT_CACHE_DIR = REPO_ROOT / "target" / "font_download_cache"


@dataclass(frozen=True)
class FontSource:
    name: str
    url: str
    license_name: str
    homepage: str
    extracts: tuple[tuple[str, str], ...]
    license_candidates: tuple[str, ...]
    archive_sha256: str


def font_sources() -> tuple[FontSource, ...]:
    from lib.content_libraries import producer, selected_manifest, relative
    policy = selected_manifest(REPO_ROOT).parent / relative(producer(REPO_ROOT, "fonts")["policy"])
    return tuple(FontSource(**{**row, "extracts": tuple((name, relative(destination)) for name, destination in row["extracts"]),
                              "license_candidates": tuple(row["license_candidates"])})
                 for row in json.loads(policy.read_text()))


def console(message: str) -> None:
    if rich_print is not None:
        rich_print(message)
    else:
        print(strip_rich_markup(message))


def strip_rich_markup(message: str) -> str:
    return re.sub(r"\[/?[A-Za-z0-9_#=:/.,~+ -]+\]", "", message)


def path_link(path: Path) -> str:
    resolved = path.resolve()
    return f"[link=file://{resolved}]{resolved}[/link]"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download(url: str, destination: Path, *, force: bool) -> None:
    if destination.exists() and not force:
        console(f"[dim]using cached archive[/dim] {path_link(destination)}")
        return
    destination.parent.mkdir(parents=True, exist_ok=True)
    console(f"[bold]downloading[/bold] {url}")
    tmp = destination.with_suffix(destination.suffix + ".tmp")
    with urllib.request.urlopen(url) as response, tmp.open("wb") as file:
        shutil.copyfileobj(response, file)
    tmp.replace(destination)
    console(f"[green]wrote[/green] {path_link(destination)}")


def iter_zip_names(zip_file: zipfile.ZipFile) -> Iterable[str]:
    for info in zip_file.infolist():
        if not info.is_dir():
            yield info.filename


def find_zip_member(zip_file: zipfile.ZipFile, basename: str) -> str:
    matches = [name for name in iter_zip_names(zip_file) if Path(name).name == basename]
    if not matches:
        raise FileNotFoundError(f"archive does not contain a file named {basename}")
    matches.sort(key=lambda name: (len(Path(name).parts), name))
    return matches[0]


def write_bytes(destination: Path, data: bytes) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(dir=destination.parent, delete=False) as file:
        temporary = Path(file.name)
        file.write(data)
    temporary.replace(destination)


def extract_member(
    zip_file: zipfile.ZipFile, basename: str, destination: Path, *, force: bool
) -> dict:
    member = find_zip_member(zip_file, basename)
    data = zip_file.read(member)
    if not force and destination.is_file() and destination.read_bytes() == data:
        return {"path": str(destination), "sha256": sha256_file(destination), "status": "kept", "archive_member": member}
    write_bytes(destination, data)
    console(f"[green]font[/green] {member} -> {path_link(destination)}")
    return {
        "path": str(destination),
        "sha256": sha256_file(destination),
        "status": "wrote",
        "archive_member": member,
    }


def copy_license(
    zip_file: zipfile.ZipFile, candidates: Iterable[str], destination: Path
) -> dict | None:
    names = list(iter_zip_names(zip_file))
    for candidate in candidates:
        matches = [name for name in names if Path(name).name == candidate]
        if matches:
            matches.sort(key=lambda name: (len(Path(name).parts), name))
            member = matches[0]
            write_bytes(destination, zip_file.read(member))
            console(f"[green]license[/green] {member} -> {path_link(destination)}")
            return {
                "path": str(destination),
                "sha256": sha256_file(destination),
                "archive_member": member,
            }
    return None


def grab_fonts(out_dir: Path, cache_dir: Path, *, force: bool, dry_run: bool) -> int:
    manifest: dict[str, object] = {
        "version": 1,
        "generated_by": "scripts/grab_font_assets.py",
        "output_dir": str(out_dir),
        "sources": [],
    }
    if dry_run:
        console(
            f"[yellow]dry-run[/yellow] would write fonts under {path_link(out_dir)}"
        )
        for source in font_sources():
            console(f"  - {source.name}: {source.url}")
        return 0

    out_dir.mkdir(parents=True, exist_ok=True)
    cache_dir.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ambition-fonts-") as _tmp:
        for source in font_sources():
            archive_path = cache_dir / Path(source.url).name
            download(source.url, archive_path, force=force)
            if sha256_file(archive_path) != source.archive_sha256:
                raise ValueError(f"Font archive hash differs from the authored policy: {archive_path}")
            source_entry: dict[str, object] = {
                "name": source.name,
                "url": source.url,
                "homepage": source.homepage,
                "license": source.license_name,
                "archive_sha256": sha256_file(archive_path),
                "files": [],
            }
            with zipfile.ZipFile(archive_path) as zip_file:
                for basename, relative_output in source.extracts:
                    font_entry = extract_member(
                        zip_file,
                        basename,
                        out_dir / relative_output,
                        force=force,
                    )
                    source_entry["files"].append(font_entry)
                license_entry = copy_license(
                    zip_file,
                    source.license_candidates,
                    out_dir / "licenses" / f"{safe_name(source.name)}-OFL.txt",
                )
                if license_entry is not None:
                    source_entry["license_file"] = license_entry
                else:
                    console(
                        f"[yellow]warning[/yellow] no license file found in {source.name} archive"
                    )
            manifest["sources"].append(source_entry)

    manifest_path = out_dir / "FONT_ASSET_MANIFEST.json"
    write_bytes(manifest_path, (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode())
    console(f"[green]manifest[/green] {path_link(manifest_path)}")
    return 0


def safe_name(name: str) -> str:
    return re.sub(r"[^A-Za-z0-9]+", "-", name).strip("-")


def main(argv: list[str] | None = None) -> int:
    import os
    import subprocess
    arguments = list(sys.argv[1:] if argv is None else argv)
    if not os.environ.get("AMBITION_PRODUCER_WORKER") and not any(arg == "--dry-run" or arg.startswith("--out-dir") for arg in arguments):
        return subprocess.call([sys.executable, str(REPO_ROOT / "scripts/generate_library_content.py"), "fonts", *arguments])
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--cache-dir", type=Path, default=DEFAULT_CACHE_DIR)
    parser.add_argument(
        "--force", action="store_true", help="redownload archives and overwrite fonts"
    )
    parser.add_argument(
        "--dry-run", action="store_true", help="print planned downloads without writing"
    )
    args = parser.parse_args(argv)
    return grab_fonts(
        args.out_dir.resolve(),
        args.cache_dir.resolve(),
        force=args.force,
        dry_run=args.dry_run,
    )


if __name__ == "__main__":
    raise SystemExit(main())
