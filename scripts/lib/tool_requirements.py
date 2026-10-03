"""Report what an interpreter lacks to run one of this repo's Python tools.

Run BY the interpreter in question (stdlib only), from any directory:

    <python> scripts/lib/tool_requirements.py <project_dir> [<project_dir> ...]

For each project it reads `[project].dependencies` from `pyproject.toml` and
prints one tab-separated line per problem, nothing when the interpreter is fit:

    missing     <project_dir>  <requirement>
    outdated    <project_dir>  <requirement> (have <version>)
    uninstalled <project_dir>  <import name>

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


def problems(project_dir: Path) -> list[tuple[str, str]]:
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
    return found


def main(argv: list[str]) -> int:
    if tomllib is None:
        print(f"cannot read pyproject.toml on Python {sys.version.split()[0]} without tomli", file=sys.stderr)
        return 2
    # Not this script's directory: the question is what is INSTALLED.
    sys.path[:] = [p for p in sys.path if Path(p or ".").resolve() != Path(__file__).resolve().parent]
    any_problem = False
    for arg in argv:
        for kind, detail in problems(Path(arg)):
            print(f"{kind}\t{arg}\t{detail}")
            any_problem = True
    return 1 if any_problem else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
