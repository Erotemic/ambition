"""A9's static profile guard must run in a lane and must be able to FAIL."""

from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import check_engine_profiles as guard  # noqa: E402

WITNESS = "SUPPORTED_PROFILES.iter() is_installed the_control_installs_every_capability"


def registry(names, marked=None):
    marked = names if marked is None else marked
    return "\n".join(f"// profile-contract: {n}" for n in marked) + "\n" + "\n".join(
        f'    name: "{n}",' for n in names
    )


NAMES = [f"p{i}" for i in range(guard.MIN_PROFILES)]


def tree(*crates, extra=60):
    return "\n".join([f"{c} v0.1.0" for c in crates] + [f"filler{i} v1.0.0" for i in range(extra)])


def test_the_repo_is_green_statically_for_the_registry():
    profile = (guard.REPO / guard.PROFILE_RS).read_text(encoding="utf-8")
    witness = (guard.REPO / guard.WITNESS_RS).read_text(encoding="utf-8")
    assert guard.check_registry(profile, witness) == []


def test_a_profile_without_a_marker_and_a_marker_without_a_profile_are_red():
    problems = guard.check_registry(registry(NAMES, NAMES[:-1] + ["ghost"]), WITNESS)
    text = " ".join(problems)
    assert "has no `// profile-contract:` marker" in text and "names no declared profile" in text


def test_an_empty_registry_cannot_pass():
    assert any("lost the registry" in p for p in guard.check_registry("", WITNESS))


def test_a_witness_that_stopped_iterating_or_lost_its_control_is_red():
    assert any("iterates" in p for p in guard.check_registry(registry(NAMES), "is_installed the_control_installs_every_capability"))
    assert any("control arm" in p for p in guard.check_registry(registry(NAMES), "SUPPORTED_PROFILES.iter()"))


def test_the_closure_contract_holds_and_fails_each_way():
    linked = guard.LINKED_NOT_ABSENT
    ok_headless = tree(*linked)
    ok_windowed = tree(*linked, "ambition_render")
    assert guard.check_closure(ok_headless, ok_windowed) == []
    assert any("promises it is absent" in p for p in guard.check_closure(tree(*linked, "ambition_render"), ok_windowed))
    assert any("not drawing" in p for p in guard.check_closure(ok_headless, ok_headless))
    assert any("left the headless closure" in p for p in guard.check_closure(tree(*linked[1:]), ok_windowed))
    assert any("lost its shape" in p for p in guard.check_closure("", ""))
