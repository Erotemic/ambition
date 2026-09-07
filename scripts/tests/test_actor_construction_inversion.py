"""F1/P1: actor construction remains below features after the spawn carve.

The original F1 guard watched a module inside the monolith.  That subject now
has a stronger boundary: actor-spawn construction lives in its own crate.  The
guard therefore checks BOTH sides of the extraction:

* monolith construction still does not reach upward into `features`;
* the extracted spawn crate reaches neither the monolith nor `crate::features`.

The positive control pins the intended dependency: construction consumes the
spawn capability directly.  If those references disappear, this checker must be
revisited instead of becoming an always-green historical assertion.
"""

from __future__ import annotations

import pathlib
import re
import tomllib

REPO = pathlib.Path(__file__).resolve().parents[2]
MONOLITH_SRC = REPO / "crates" / "ambition_platformer2d_actor_monolith" / "src"
SPAWN_CRATE = REPO / "crates" / "ambition_platformer2d_actor_spawn"
SPAWN_SRC = SPAWN_CRATE / "src"

LINE_COMMENT = re.compile(r"//.*$", re.MULTILINE)


def _production_files(root: pathlib.Path) -> list[pathlib.Path]:
    return [
        p
        for p in sorted(root.rglob("*.rs"))
        if "tests" not in p.name and "/tests/" not in str(p)
    ]


def _code(path: pathlib.Path) -> str:
    return LINE_COMMENT.sub("", path.read_text(encoding="utf-8", errors="ignore"))


def _crate_references(path: pathlib.Path, target: str) -> list[str]:
    return re.findall(
        rf"crate::{target}\b(?:::[A-Za-z_{{][A-Za-z_0-9]*)?",
        _code(path),
    )


def test_the_construction_domain_does_not_name_the_feature_layer() -> None:
    offenders: list[str] = []
    for path in _production_files(MONOLITH_SRC / "construction"):
        for ref in _crate_references(path, "features"):
            offenders.append(f"{path.relative_to(REPO)}  {ref}")
    assert not offenders, (
        "the actor construction domain names the feature layer again; construction "
        "may consume the extracted actor-spawn capability, never feature systems:\n  "
        + "\n  ".join(offenders)
    )


def test_the_extracted_spawn_capability_does_not_reach_back_into_the_monolith() -> None:
    assert SPAWN_SRC.is_dir(), f"missing extracted spawn capability: {SPAWN_SRC}"
    offenders: list[str] = []
    for path in _production_files(SPAWN_SRC):
        text = _code(path)
        if "ambition_platformer2d_actor_monolith::" in text:
            offenders.append(
                f"{path.relative_to(REPO)} names ambition_platformer2d_actor_monolith"
            )
        for ref in _crate_references(path, "features"):
            offenders.append(f"{path.relative_to(REPO)}  {ref}")
    assert not offenders, (
        "the extracted actor-spawn capability reaches back into the monolith/feature "
        "layer, recreating the cycle across a crate boundary:\n  "
        + "\n  ".join(offenders)
    )


def test_the_spawn_manifest_has_no_monolith_dependency() -> None:
    manifest = tomllib.loads((SPAWN_CRATE / "Cargo.toml").read_text(encoding="utf-8"))
    deps: set[str] = set()
    for section in ("dependencies", "build-dependencies"):
        deps.update((manifest.get(section) or {}).keys())
    assert "ambition_platformer2d_actor_monolith" not in deps


def test_the_checker_has_a_positive_control_on_the_intended_edge() -> None:
    probe = MONOLITH_SRC / "construction" / "mod.rs"
    text = _code(probe)
    count = text.count("ambition_platformer2d_actor_spawn::")
    assert count >= 10, (
        "construction no longer has the expected direct dependency on the extracted "
        f"spawn capability (found {count} references); re-point this guard if the "
        "construction/spawn boundary moved deliberately"
    )
