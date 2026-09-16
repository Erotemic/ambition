"""The guard that keeps a headless arm able to observe that it stepped.

⚠ These arms exist because the SCANNER was wrong twice before the guard was
right once, and both defects reported confident, plausible numbers. A parser
that miscounts is worse than no census, because the count looks measured.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "check_headless_arms_can_fail.py"
_spec = importlib.util.spec_from_file_location("check_headless_arms_can_fail", SCRIPT)
guard = importlib.util.module_from_spec(_spec)
sys.modules[_spec.name] = guard
_spec.loader.exec_module(guard)


def names(source: str) -> list[str]:
    return [name for name, _, _ in guard.test_arms(source)]


def test_a_helper_nested_in_a_test_is_not_an_arm() -> None:
    """⛔ THE FIRST SCANNER REPORTED `arena` AND `arena_with_victim_hp` AS ARMS.

    It searched BACKWARDS from each `fn` for a nearby `#[test]`, so a helper
    declared inside a test body inherited the enclosing test's attribute. The
    census read 30; the real number was a quarter of that.
    """
    source = """
#[test]
fn the_real_arm() {
    fn arena(flag: bool) -> App {
        App::new()
    }
    arena(true).update();
}
"""
    assert names(source) == ["the_real_arm"]


def test_an_ignored_probe_is_not_an_arm() -> None:
    """A `#[ignore]`d print-only probe does not run, so it is not a silent gap.
    MEASURED in-tree: 25 of them, including
    `probe_when_the_mirror_breaks`, whose attribute says "PROBE, print-only"."""
    source = """
#[test]
#[ignore = "PROBE, print-only: first tick two mirrored CPUs diverge"]
fn probe_when_the_mirror_breaks() {
    app.update();
}
"""
    assert names(source) == []


def test_should_panic_is_an_assertion() -> None:
    """⛔ THE SECOND SCANNER COUNTED THESE AS MUTE, and they are the opposite:
    `#[should_panic(expected = ...)]` is a MESSAGE-MATCHED assertion. Four arms
    were wrongly flagged, among them
    `activating_a_provider_with_no_audio_fragment_panics`."""
    source = """
#[test]
#[should_panic(expected = "registered no audio catalog fragment")]
fn activating_a_provider_with_no_audio_fragment_panics() {
    add_headless_foundation(&mut app);
    app.update();
}
"""
    arms = list(guard.test_arms(source))
    assert [name for name, _, _ in arms] == ["activating_a_provider_with_no_audio_fragment_panics"]
    assert arms[0][2] is True, "should_panic must be reported so the caller can count it as asserting"


def test_an_attribute_separated_from_its_fn_by_real_code_binds_to_nothing() -> None:
    """`#[test]` scans FORWARD, so it must not adopt a distant `fn` when the
    thing it decorates is not one — otherwise a stray attribute silently renames
    whatever arm follows it."""
    source = """
#[test]
struct NotAFunction;

fn a_plain_helper() {
    app.update();
}
"""
    assert names(source) == []


def test_the_live_tree_population_is_not_empty() -> None:
    """⛔ THE GUARD'S OWN FAILURE MODE. It reads source, so a broken scan and a
    clean tree both produce zero offenders — and this exact check caught a real
    misconfigured `ROOT` that made the whole scan silently match nothing.

    An empty population must be a FAILURE, never a pass.
    """
    checked = 0
    for path in guard.ROOT.glob("**/*.rs"):
        if "/target/" in str(path) or "/.git/" in str(path):
            continue
        try:
            text = path.read_text(errors="replace")
        except OSError:
            continue
        if not guard.FOUNDATION.search(text):
            continue
        for _, body, _ in guard.test_arms(text):
            reach = guard.reachable(text, body)
            if guard.FOUNDATION.search(reach) and guard.STEP.search(reach):
                checked += 1
    assert checked > 0, "the scan matched no headless stepping arm at all — it is broken"


def test_the_allowlist_names_only_arms_that_still_exist() -> None:
    """A stale entry would let a genuinely new gap in under an unchanged count:
    one arm fixed and one added nets to zero."""
    for relative, arm in guard.KNOWN_GAPS:
        path = guard.ROOT / relative
        assert path.exists(), f"{relative} is gone; drop its KNOWN_GAPS entries"
        assert f"fn {arm}" in path.read_text(errors="replace"), (
            f"{relative}::{arm} no longer exists; remove it from KNOWN_GAPS"
        )
