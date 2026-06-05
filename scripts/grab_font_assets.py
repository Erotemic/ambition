#!/usr/bin/env python3
"""Download redistributable UI fonts for the sandbox asset tree.

The fonts themselves are intentionally git-ignored by default. Run this script,
review the generated manifest/licenses, then force-add or IPFS-track the assets
once you are happy with the versions.
"""

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
DEFAULT_OUT_DIR = (
    REPO_ROOT / "crates" / "ambition_sandbox" / "assets" / "fonts" / "bundled"
)
DEFAULT_CACHE_DIR = REPO_ROOT / "target" / "font_download_cache"


@dataclass(frozen=True)
class FontSource:
    name: str
    url: str
    license_name: str
    homepage: str
    extracts: tuple[tuple[str, str], ...]
    license_candidates: tuple[str, ...]


FONT_SOURCES: tuple[FontSource, ...] = (
    FontSource(
        name="Inter 4.1",
        url="https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip",
        license_name="SIL Open Font License 1.1",
        homepage="https://rsms.me/inter/",
        extracts=(
            ("InterDisplay-Regular.otf", "InterDisplay-Regular.otf"),
            ("InterDisplay-SemiBold.otf", "InterDisplay-SemiBold.otf"),
        ),
        license_candidates=("LICENSE.txt", "OFL.txt"),
    ),
    FontSource(
        name="JetBrains Mono 2.304",
        url="https://download.jetbrains.com/fonts/JetBrainsMono-2.304.zip",
        license_name="SIL Open Font License 1.1",
        homepage="https://www.jetbrains.com/lp/mono/",
        extracts=(("JetBrainsMono-Regular.ttf", "JetBrainsMono-Regular.ttf"),),
        license_candidates=("OFL.txt", "LICENSE.txt"),
    ),
)


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


def extract_member(
    zip_file: zipfile.ZipFile, basename: str, destination: Path, *, force: bool
) -> dict:
    if destination.exists() and not force:
        return {
            "path": str(destination.relative_to(REPO_ROOT)),
            "sha256": sha256_file(destination),
            "status": "kept",
        }
    member = find_zip_member(zip_file, basename)
    destination.parent.mkdir(parents=True, exist_ok=True)
    with zip_file.open(member) as src, destination.open("wb") as dst:
        shutil.copyfileobj(src, dst)
    console(f"[green]font[/green] {member} -> {path_link(destination)}")
    return {
        "path": str(destination.relative_to(REPO_ROOT)),
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
            destination.parent.mkdir(parents=True, exist_ok=True)
            with zip_file.open(member) as src, destination.open("wb") as dst:
                shutil.copyfileobj(src, dst)
            console(f"[green]license[/green] {member} -> {path_link(destination)}")
            return {
                "path": str(destination.relative_to(REPO_ROOT)),
                "sha256": sha256_file(destination),
                "archive_member": member,
            }
    return None


def grab_fonts(out_dir: Path, cache_dir: Path, *, force: bool, dry_run: bool) -> int:
    manifest: dict[str, object] = {
        "version": 1,
        "generated_by": "scripts/grab_font_assets.py",
        "output_dir": str(out_dir.relative_to(REPO_ROOT)),
        "sources": [],
    }
    if dry_run:
        console(
            f"[yellow]dry-run[/yellow] would write fonts under {path_link(out_dir)}"
        )
        for source in FONT_SOURCES:
            console(f"  - {source.name}: {source.url}")
        return 0

    out_dir.mkdir(parents=True, exist_ok=True)
    cache_dir.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ambition-fonts-") as _tmp:
        for source in FONT_SOURCES:
            archive_path = cache_dir / Path(source.url).name
            download(source.url, archive_path, force=force)
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
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    console(f"[green]manifest[/green] {path_link(manifest_path)}")
    console(
        "[bold]next[/bold] review licenses, then force-add/IPFS-track the generated assets if accepted"
    )
    console(f"       git add -f {path_link(out_dir)}")
    return 0


def safe_name(name: str) -> str:
    return re.sub(r"[^A-Za-z0-9]+", "-", name).strip("-")


def main(argv: list[str] | None = None) -> int:
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
