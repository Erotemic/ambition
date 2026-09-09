"""D-BUILD-GRAPH-BLINDNESS: the three answers must stay three answers.

⛔⛔ DECLARATION COUNT IS NOT CAPABILITY REACHABILITY, and this repository has
paid for the confusion twice in one day: a `cargo metadata` walk of the facade's
closure reported 61 ambition crates where `cargo tree -e normal
--no-default-features` reports 49, because the resolve graph counts every
OPTIONAL edge no feature enables. A guard built on the first number would demand
the deletion of a dependency that already costs nothing.

⇒ Three separate facts, three separate instruments:

  DECLARED-BUT-UNUSED   this file's subject. Text: the manifest names it and no
                        source line does.
  FEATURE-GATED         the manifest forwards `"dep/feature"`, which IS naming
                        it — the one file where naming it is the whole point.
  ACTUALLY LINKED       `cargo tree`, in
                        `check_absence_contracts.py`'s featureless-facade
                        contract and its own red probes.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

from measure_unreferenced_workspace_dependencies import (  # noqa: E402
    unreferenced_in,
)

DECLARED = {"ambition_used", "ambition_forwarded", "ambition_idle"}


def test_a_dependency_the_source_names_is_not_reported():
    assert "ambition_used" not in unreferenced_in(
        DECLARED, "use ambition_used::Thing;", ""
    )


def test_a_dependency_only_a_feature_forwards_is_not_reported():
    """⭐ THE CASE THAT MAKES THIS TOOL HONEST. `causal = ["ambition_x/causal"]`
    is a real use of the dependency; reporting it would send an agent to delete a
    feature edge the composition depends on."""
    manifest = 'causal = ["ambition_forwarded/causal"]'
    assert "ambition_forwarded" not in unreferenced_in(DECLARED, "", manifest)


def test_a_weakly_forwarded_dependency_is_not_reported():
    """`dep?/feature` is the WEAK form and forwards just the same."""
    manifest = 'input = ["ambition_forwarded?/input"]'
    assert "ambition_forwarded" not in unreferenced_in(DECLARED, "", manifest)


def test_a_dependency_nothing_names_is_reported():
    assert unreferenced_in(DECLARED, "", "") == sorted(DECLARED)


def test_a_substring_of_another_crate_name_is_not_a_use():
    """⛔ THE FALSE NEGATIVE THAT WOULD MAKE THE COUNT ALWAYS ZERO. Without a word
    boundary, `ambition_used_elsewhere` in the source would satisfy
    `ambition_used`, and a crate whose sibling shares a prefix would never be
    reported."""
    reported = unreferenced_in({"ambition_use"}, "use ambition_used::Thing;", "")
    assert reported == ["ambition_use"]


def test_the_live_tree_has_no_unreferenced_declarations():
    """⭐ THE FLOOR, and the reason the fixtures above are not the whole test: a
    classifier that reported nothing for every input would pass all five."""
    import json
    import subprocess

    sys.path.insert(0, str(REPO / "scripts" / "lib"))
    from cargo_bin import cargo_binary

    meta = json.loads(
        subprocess.run(
            [cargo_binary(), "metadata", "--no-deps", "--format-version", "1"],
            cwd=REPO,
            capture_output=True,
            text=True,
            check=True,
        ).stdout
    )
    members = {package["name"] for package in meta["packages"]}
    assert len(members) > 50, f"only {len(members)} workspace members"
    offenders: dict[str, list[str]] = {}
    for package in meta["packages"]:
        manifest = Path(package["manifest_path"])
        source = "\n".join(
            path.read_text(errors="ignore") for path in manifest.parent.rglob("*.rs")
        )
        declared = {
            dependency["name"]
            for dependency in package["dependencies"]
            if dependency["name"] in members and dependency["kind"] is None
        }
        found = unreferenced_in(declared, source, manifest.read_text())
        if found:
            offenders[package["name"]] = found
    assert not offenders, (
        "crates declare an ambition dependency nothing names — establish WHY "
        f"each is declared before removing it: {offenders}"
    )
