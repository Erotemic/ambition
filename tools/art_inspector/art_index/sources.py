"""Read owner reports. Inspection never builds a binary or publishes art."""
from __future__ import annotations

import hashlib
import json
import os
import subprocess
from pathlib import Path

from .paths import asset_root, roots


def renderer_python(repo: Path, python: str | None) -> str:
    if python:
        return python
    result = subprocess.run(["bash", "-c", 'source "$1/scripts/lib/tool_python.sh"; ambition_select_tool_python "$2" AMBITION_SPRITE_PYTHON',
                             "_", str(repo), str(repo / "tools/ambition_sprite2d_renderer")], capture_output=True, text=True)
    return result.stdout.strip() or "python3"


def products(repo: Path, python: str | None = None, *, deep=False) -> dict:
    command = [renderer_python(repo, python), "-m", "ambition_sprite2d_renderer.products"]
    if deep:
        command.append("--deep")
    result = subprocess.run(command, cwd=repo / "tools/ambition_sprite2d_renderer", capture_output=True, text=True)
    try:
        if result.returncode:
            raise ValueError(result.stderr[-1500:])
        return json.loads(result.stdout)
    except ValueError as error:
        return {"units": {}, "complete": False, "warnings": [f"Product owner unavailable: {error}"]}


def _stamp(path: Path):
    try:
        stat = path.stat()
        return [str(path), stat.st_size, stat.st_mtime_ns, stat.st_ctime_ns]
    except OSError:
        return [str(path), None]


def content(repo: Path, profile: str, out: Path, binary: Path | None = None) -> dict:
    pack = (repo / "game/ambition_content/assets") if profile == "ambition" else Path(profile).resolve()
    binary = binary or Path(os.environ.get("AMBITION_ART_CONTENT_BIN", repo / "target/debug/ambition_content_check"))
    cache = out / ("content-" + hashlib.sha256(str(pack).encode()).hexdigest()[:16] + ".json")
    try:
        old = json.loads(cache.read_text())
    except (OSError, ValueError):
        old = {}
    sheet_roots = [r.path for r in roots(repo) if r.name.startswith("sprites") and r.origin == "generated"]
    files = [pack / "pack.ron", binary]
    files += [pack / row["path"] for row in old.get("report", {}).get("sources", [])]
    # Page membership and metadata changes invalidate the Rust owner's loose-sheet view.
    files += [p for root in sheet_roots for p in root.rglob("*") if p.suffix in (".ron", ".png")]
    signature = [_stamp(p) for p in sorted(set(files))]
    if old.get("signature") == signature and binary.is_file() and old.get("report", {}).get("schema") == "ambition.art_content.v3":
        return old["report"] | {"cache": "matching-inputs"}
    if not binary.is_file():
        return {"state": "unavailable", "characters": [], "sheets": {}, "assets": [],
                "warnings": [f"Build the content inspection binary once: cargo build -p ambition_content_schemas --bin ambition_content_check. Expected {binary}"]}
    command = [str(binary), str(pack), "--inspect-art", "--no-asset-check"]
    for root in sheet_roots:
        command += ["--sheet-root", str(root)]
    result = subprocess.run(command, capture_output=True, text=True, cwd=repo)
    try:
        if result.returncode:
            raise ValueError(result.stderr[-1800:])
        report = json.loads(result.stdout)
        if report.get("schema") != "ambition.art_content.v3":
            raise ValueError("The content inspection protocol changed. Rebuild the selected content check binary.")
    except ValueError as error:
        return {"state": "unavailable", "characters": [], "sheets": {}, "assets": [], "warnings": [f"Content export failed: {error}"]}
    report.update(state="available", profile=profile, pack_root=str(pack), cache="refreshed")
    files += [pack / row["path"] for row in report.get("sources", [])]
    out.mkdir(parents=True, exist_ok=True)
    temp = cache.with_suffix(".partial")
    temp.write_text(json.dumps({"signature": [_stamp(p) for p in sorted(set(files))], "report": report}))
    temp.replace(cache)
    return report


def previews(prepared: dict, out: Path) -> dict:
    """Read YAML row metadata without decoding pages. Cache unchanged projections."""
    import yaml
    cache = out / "preview_metadata.json"
    try:
        saved = json.loads(cache.read_text())
    except (OSError, ValueError):
        saved = {}
    report = {}
    for ron, records in prepared.get("sheets", {}).items():
        if not any(parent.name == "sprites" for parent in Path(ron).parents):
            continue
        path = Path(ron).with_suffix(".yaml")
        stamp = _stamp(path)
        if saved.get(str(path), {}).get("stamp") != stamp:
            try:
                data = yaml.load(path.read_text(), Loader=getattr(yaml, "CSafeLoader", yaml.SafeLoader))
                rows = {row["animation"]: row["frame_count"] for row in data["rows"]}
                saved[str(path)] = {"stamp": stamp, "state": "available", "rows": rows}
            except (OSError, ValueError, KeyError, TypeError, yaml.YAMLError) as error:
                saved[str(path)] = {"stamp": stamp, "state": "unavailable", "error": str(error)}
        for record in records:
            report[record["key"]] = dict(saved[str(path)], path=str(path))
    out.mkdir(parents=True, exist_ok=True)
    cache.write_text(json.dumps(saved))
    return report
