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
    try:
        result = subprocess.run(command, cwd=repo / "tools/ambition_sprite2d_renderer", capture_output=True, text=True)
        if result.returncode:
            raise ValueError(result.stderr[-1500:])
        return json.loads(result.stdout)
    except (ValueError, OSError) as error:
        return {"units": {}, "complete": False, "warnings": [f"Product owner unavailable: {error}"]}


def _stamp(path: Path):
    try:
        stat = path.stat()
        return [str(path), stat.st_size, stat.st_mtime_ns, stat.st_ctime_ns]
    except OSError:
        return [str(path), None]


def pack_dir(repo: Path, profile: str) -> Path:
    """The game pack a profile selects: `ambition`, or a pack directory."""
    return (repo / "game/ambition_content/assets") if profile == "ambition" else Path(profile).resolve()


def default_binary(repo: Path) -> Path:
    """`AMBITION_ART_CONTENT_BIN`, else the newer of the release and debug builds
    (release when neither exists: that is what the inspector builds)."""
    if os.environ.get("AMBITION_ART_CONTENT_BIN"):
        return Path(os.environ["AMBITION_ART_CONTENT_BIN"])
    builds = [repo / "target" / profile / "ambition_content_check" for profile in ("release", "debug")]
    present = [path for path in builds if path.is_file()]
    return max(present, key=lambda path: path.stat().st_mtime) if present else builds[0]


def newer_source(repo: Path, binary: Path) -> Path | None:
    """A Rust source or lockfile newer than `binary`, or the binary's own path when
    it is absent. The check binary links the content schemas of many engine crates,
    so any of them can change what it accepts."""
    try:
        built = binary.stat().st_mtime
    except OSError:
        return binary
    lock = repo / "Cargo.lock"
    if lock.is_file() and lock.stat().st_mtime > built:
        return lock
    for top in sorted([*repo.glob("crates/*/src"), *repo.glob("game/*/src")]):
        for folder, _dirs, names in os.walk(top):
            for name in names:
                if name.endswith(".rs") and os.stat(os.path.join(folder, name)).st_mtime > built:
                    return Path(folder, name)
    return None


def build_binary(repo: Path, log=print) -> str | None:
    """Rebuild the content check binary (release). None on success, else why it was not built."""
    log(f"rebuilding the content check binary: {BUILD_BINARY}")
    done = subprocess.run(BUILD_BINARY.split(), cwd=repo, capture_output=True, text=True)
    return None if done.returncode == 0 else f"{BUILD_BINARY} failed:\n{done.stderr[-1500:]}"


def content(repo: Path, profile: str, out: Path, binary: Path | None = None, *, cargo: bool = False, log=print) -> dict:
    """The game catalog from the content check binary. With `cargo`, a binary
    older than the engine's Rust sources is rebuilt once, then asked again."""
    report = _content(repo, profile, out, binary)
    if not cargo or binary is not None or report.get("state") == "available" or os.environ.get("AMBITION_ART_CONTENT_BIN"):
        return report
    # The release build is the one rebuilt: a debug build newer than it is still compared.
    newer = newer_source(repo, default_binary(repo))
    if newer is None or not (repo / "Cargo.toml").is_file():
        return report
    log(f"the content check binary is older than {newer.relative_to(repo) if newer.is_relative_to(repo) else newer}")
    problem = build_binary(repo, log)
    if problem:
        report["warnings"] = [*report.get("warnings", []), problem]
        return report
    return _content(repo, profile, out, binary)


def _content(repo: Path, profile: str, out: Path, binary: Path | None = None) -> dict:
    pack = pack_dir(repo, profile)
    binary = binary or default_binary(repo)
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
                "warnings": [f"Content inspection binary not found: {binary}. Build it: {BUILD_BINARY} (the served inspector does this itself)."]}
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
        return {"state": "unavailable", "characters": [], "sheets": {}, "assets": [],
                "warnings": [f"Content export failed: {error}", *_stale_binary(repo, binary)]}
    report.update(state="available", profile=profile, pack_root=str(pack), cache="refreshed")
    files += [pack / row["path"] for row in report.get("sources", [])]
    out.mkdir(parents=True, exist_ok=True)
    temp = cache.with_suffix(".partial")
    temp.write_text(json.dumps({"signature": [_stamp(p) for p in sorted(set(files))], "report": report}))
    temp.replace(cache)
    return report


BUILD_BINARY = "cargo build --release -p ambition_content_schemas --bin ambition_content_check"


def _stale_binary(repo: Path, binary: Path) -> list[str]:
    """Name a content check binary older than an engine Rust source, and the rebuild."""
    newer = newer_source(repo, binary)
    if newer is not None and newer != binary:
        shown = newer.relative_to(repo) if newer.is_relative_to(repo) else newer
        return [f"The content check binary {binary} is older than {shown}. Rebuild it: {BUILD_BINARY} (the served inspector does this itself)."]
    return [f"If the files it names exist, the content itself has this problem; fix the file it names. Rebuild the binary after Rust edits: {BUILD_BINARY}"]


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
