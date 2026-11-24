"""Report what an interpreter lacks to run one of this repo's Python tools.

Run BY the interpreter in question (stdlib only), from any directory:

    <python> scripts/lib/tool_requirements.py <project_dir> [<project_dir> ...]

For each project it reads `[project].dependencies` from `pyproject.toml` and
prints one tab-separated line per problem, nothing when the interpreter is fit:

    missing     <project_dir>  <requirement>
    outdated    <project_dir>  <requirement> (have <version>)
    uninstalled <project_dir>  <import name>
    elsewhere   <project_dir>  <where the installed copy comes from>

`elsewhere` is reported only with `--source` first: the tool is installed, but
not editable from THIS `project_dir`. A tool venv is per machine and keyed by
the tool's directory name, so every checkout and worktree of this repository
shares one; an editable install made from another checkout (or one since
deleted, or a stale submodule tree) serves that checkout's modules here, and a
module this checkout has and that one lacks fails to import.

Exit 0 when nothing is printed, 1 when something is, 2 when a pyproject cannot
be read.

⛔ WHY NOT `import <tool>`. The old preflight imported the tool's own package
and nothing else. A stale editable venv passes that, then the run dies on the
first target that imports numpy, reported as "publish roster names an
unregistered target". The DECLARED dependencies are the contract; check them.

⚠ Versions are checked only when `packaging` is importable; without it a
present distribution counts as satisfied, and markers are assumed to apply.
"""

from __future__ import annotations

import importlib.metadata
import importlib.util
import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11
    try:
        import tomli as tomllib  # type: ignore[no-redef]
    except ModuleNotFoundError:
        tomllib = None  # type: ignore[assignment]

try:
    from packaging.requirements import Requirement
except ModuleNotFoundError:
    Requirement = None  # type: ignore[assignment,misc]

NAME = re.compile(r"\s*([A-Za-z0-9][A-Za-z0-9._-]*)")


def problems(project_dir: Path, check_source: bool = False) -> list[tuple[str, str]]:
    data = tomllib.loads((project_dir / "pyproject.toml").read_text())
    project = data.get("project", {})
    found: list[tuple[str, str]] = []
    for spec in project.get("dependencies", []):
        if Requirement is not None:
            req = Requirement(spec)
            if req.marker is not None and not req.marker.evaluate():
                continue
            name, specifier = req.name, req.specifier
        else:
            match = NAME.match(spec)
            if match is None:
                continue
            name, specifier = match.group(1), None
        try:
            version = importlib.metadata.version(name)
        except importlib.metadata.PackageNotFoundError:
            found.append(("missing", spec))
            continue
        if specifier is not None and not specifier.contains(version, prereleases=True):
            found.append(("outdated", f"{spec} (have {version})"))
    module = project.get("name", project_dir.name).replace("-", "_")
    if importlib.util.find_spec(module) is None:
        found.append(("uninstalled", module))
    elif check_source:
        where = install_source(project.get("name", project_dir.name))
        if where != project_dir.resolve():
            found.append(("elsewhere", str(where) if isinstance(where, Path) else where))
    return found


def install_source(distribution: str) -> Path | str:
    """The directory an editable install of `distribution` points at, or what it is instead."""
    import json
    from urllib.parse import unquote, urlparse
    try:
        dist = importlib.metadata.distribution(distribution)
    except importlib.metadata.PackageNotFoundError:
        return "no installed distribution (importable only through sys.path)"
    record = json.loads(dist.read_text("direct_url.json") or "{}")
    url = urlparse(record.get("url", ""))
    if url.scheme != "file" or not record.get("dir_info", {}).get("editable"):
        return "a non-editable copy"
    return Path(unquote(url.path)).resolve()


def main(argv: list[str]) -> int:
    if tomllib is None:
        print(f"cannot read pyproject.toml on Python {sys.version.split()[0]} without tomli", file=sys.stderr)
        return 2
    # Not this script's directory: the question is what is INSTALLED.
    sys.path[:] = [p for p in sys.path if Path(p or ".").resolve() != Path(__file__).resolve().parent]
    check_source = argv[:1] == ["--source"]
    if check_source:
        argv = argv[1:]
    any_problem = False
    for arg in argv:
        for kind, detail in problems(Path(arg), check_source):
            print(f"{kind}\t{arg}\t{detail}")
            any_problem = True
    return 1 if any_problem else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
