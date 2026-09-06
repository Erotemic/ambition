"""The facade census, and the ways a 0% could be a broken scan instead of a win.

`measure_facade_reexport_coupling.py` asks how much of the actor kernel's
apparent coupling to `features` is really a RE-EXPORT of a lower crate's name.
Carve sizings are chosen from those counts — `queue.md` sized the `items/` carve
as "multi-day" partly on references that resolve out of the crate entirely.

⛔⛔ IT NOW REPORTS `RE-EXPORT 0 (0%)`, AND THAT IS THE ANSWER A TOTALLY BROKEN
SCAN ALSO GIVES. A regex that matches no re-export block returns an empty facade
and every downstream percentage becomes 0 — indistinguishable from the real
result, which is that the carve landed and the laundering is gone
(`queue.md`: *"RE-EXPORT 45 (27%) → 0 (0%)"*, closed 2026-09-02).

⭐ So the tests that matter are the POSITIVE CONTROLS: the facade must still be
found, it must still classify names into both buckets, and UNRESOLVED must stay
its own bucket rather than being folded into either.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/measure_facade_reexport_coupling.py"


def load():
    spec = importlib.util.spec_from_file_location("facade", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_the_facade_is_still_found_and_is_not_empty():
    """⛔ THE CONTROL THE 0% DEPENDS ON. An empty facade makes every ratio 0 and
    reads exactly like the carve having succeeded."""
    module = load()
    if not module.FACADE.exists():
        pytest.skip("the features facade is absent from this checkout")
    names = module.facade_names()
    assert len(names) > 50, (
        f"the facade re-export scan found only {len(names)} names; the report's "
        "percentages are computed against this and a broken block regex makes "
        "every one of them 0"
    )


def test_visibility_is_reported_and_not_all_one_value():
    """A `pub` re-export is a road a consumer can learn; a `pub(crate)` one is a
    local alias nobody outside can reach, and re-pointing it is churn. Collapsing
    the two is what flagged an unreachable alias as a finding."""
    module = load()
    if not module.FACADE.exists():
        pytest.skip("the features facade is absent")
    values = set(module.facade_names().values())
    assert "pub" in values, "no `pub` re-export found; the visibility scan broke"
    assert len(values) > 1, (
        "every re-export reports the same visibility, so the pub/pub(crate) "
        "distinction this script exists to make is not being read"
    )


def test_definitions_finds_both_a_local_and_a_lower_crate_name(tmp_path):
    """The classification rests entirely on this map. On fixtures, so it cannot
    pass by accident of the live tree."""
    module = load()
    (tmp_path / "crates" / "lower" / "src").mkdir(parents=True)
    (tmp_path / "crates" / "lower" / "src" / "lib.rs").write_text(
        "pub struct HeldItem;\npub fn can_damage() {}\n"
    )
    (tmp_path / "game").mkdir()
    defs = module.definitions(tmp_path)
    assert defs["HeldItem"] == {"lower"}
    assert defs["can_damage"] == {"lower"}
    assert "NeverDefinedAnywhere" not in defs, (
        "a name with no definition must be ABSENT so the caller can count it as "
        "UNRESOLVED, not present-with-an-empty-set"
    )


def test_a_name_defined_in_the_kernel_is_local_and_one_below_is_a_reexport(tmp_path):
    """⭐ THE DISTINCTION THE WHOLE SCRIPT IS FOR, on a fixture where I control
    both sides."""
    module = load()
    for crate, body in (
        ("ambition_platformer2d_actor_monolith", "pub struct KernelOwned;"),
        ("ambition_combat", "pub struct BelowOwned;"),
    ):
        d = tmp_path / "crates" / crate / "src"
        d.mkdir(parents=True)
        (d / "lib.rs").write_text(body + "\n")
    (tmp_path / "game").mkdir()
    defs = module.definitions(tmp_path)

    def owners(name):
        return defs.get(name, set())

    assert "ambition_platformer2d_actor_monolith" in owners("KernelOwned"), (
        "a kernel-defined name is LOCAL coupling and must resolve to the kernel"
    )
    assert owners("BelowOwned") == {"ambition_combat"}, (
        "a name defined below the monolith is a RE-EXPORT — naming it through "
        "`crate::features::` is not coupling to the kernel at all"
    )


def test_the_live_tree_reproduces_the_row_that_closed_this_work():
    """⭐ The queue row says `RE-EXPORT 45 (27%) → 0 (0%)`, closed 2026-09-02.
    That claim is only meaningful while the scan still sees a facade and still
    classifies into two buckets — which the controls above pin. This asserts the
    shape of the result, not the exact counts, so an ordinary carve does not
    make it red."""
    module = load()
    if not module.FACADE.exists():
        pytest.skip("the features facade is absent")
    names = module.facade_names()
    defs = module.definitions(REPO)
    local = [n for n in names if any("actor_monolith" in c for c in defs.get(n, set()))]
    reexport = [
        n for n in names if n in defs and not any("actor_monolith" in c for c in defs[n])
    ]
    assert local and reexport, (
        f"expected the facade to hold BOTH kernel-owned and lower-crate names; "
        f"got {len(local)} local and {len(reexport)} re-export. If either is "
        "zero the classification is not running, whatever the percentages say"
    )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
