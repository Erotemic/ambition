"""Every `ambition::` module the SDK's CODE BLOCKS name must actually exist.

⚠ **This is here because prose failed three times in four blind runs**, and each
failure sent a third-party agent into `crates/`:

* the README's "Known gaps" advertised gaps slices B and C had already closed;
* `api-prototype.md` §5 published `ambition::experience`, which never existed;
* §5 then DENIED `ambition::world::prelude` exists, after slice C shipped it —
  costing blind run 4 two engine crates, and making its headline finding a
  documentation defect rather than an API one.

Three instances is enough to stop writing paragraphs about it. The campaign's
own rule — "name a test, never a doc marker" — applies to the campaign's own
documents, and this is that test.

## Why CODE BLOCKS only

A doc that says "`ambition::experience` never existed" must not fail the guard
that verified it. That is the exact recurrence
`scripts/check_absence_contracts.py` was built around, and its answer is the
same one: strip the prose, check the code. What a reader COPIES is a fenced
block; what a reader READS about history is not a claim about the present.

## What this does NOT catch

A doc that DENIES a real module exists, in prose. That was the worst of the
three and it is not mechanically checkable from the doc side — "not invented
here" is a sentence, not a path. The countermeasure for that one is
`the_sdk_worked_room_example_compiles_and_runs` in the minimal game, which runs
the README's snippet verbatim: if the vocabulary moves, a test fails instead of
a reader. Recorded rather than papered over, because a guard whose limits are
unstated gets trusted past them.
"""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
)

_FENCED = re.compile(r"```[a-z]*\n(.*?)```", re.DOTALL)
_MODULE = re.compile(r"\bambition::([a-z_][a-z0-9_]*)")
# Declared in `crates/ambition/src/lib.rs` as either a real module or a crate
# re-export.
_EXPORT = re.compile(r"^pub (?:mod|use ambition_[a-z_0-9]+ as) ([a-z_][a-z0-9_]*)", re.M)


def facade_modules() -> set[str]:
    lib = (REPO / "crates/ambition/src/lib.rs").read_text(encoding="utf-8")
    modules = set(_EXPORT.findall(lib))
    # `pub use bevy;` is the documented Bevy re-export and is not spelled like
    # the others.
    if re.search(r"^pub use bevy;", lib, re.M):
        modules.add("bevy")
    return modules


def named_in_sdk_code() -> dict[str, list[str]]:
    """Module -> the SDK files whose CODE names it."""
    found: dict[str, list[str]] = {}
    for doc in sorted((REPO / "docs/sdk").glob("*.md")):
        text = doc.read_text(encoding="utf-8")
        for block in _FENCED.findall(text):
            for module in _MODULE.findall(block):
                found.setdefault(module, []).append(doc.name)
    return found


def test_the_facade_exports_something():
    """Non-vacuity: a broken parser would pass every assertion below."""
    modules = facade_modules()
    assert len(modules) > 20, sorted(modules)
    assert "app" in modules, sorted(modules)


def test_the_sdk_code_blocks_name_only_modules_that_exist():
    exports = facade_modules()
    missing = {
        module: sorted(set(files))
        for module, files in named_in_sdk_code().items()
        if module not in exports
    }
    assert not missing, (
        "the SDK's code blocks name modules the facade does not export — a "
        f"reader copying them gets a compile error: {missing}. This is the "
        "defect that shipped `ambition::experience` in api-prototype.md §5, a "
        "list explicitly framed as an OUTPUT read off the call sites."
    )


def test_every_reviewed_sdk_module_is_documented():
    """A module we PROMISE must be findable in the SDK.

    The allowlist's `allowed` set is the compatibility commitment; if a consumer
    may name it, a consumer must be able to discover it without reading
    `crates/`. Blind run 4 opened two engine crates hunting for the room
    vocabulary while `world` sat in that set.
    """
    import sys

    sys.path.insert(0, str(REPO / "scripts"))
    from check_absence_contracts import MODULE_ALLOWLISTS

    promised: set[str] = set()
    for contract in MODULE_ALLOWLISTS:
        promised |= set(contract["allowed"])

    documented = set()
    for doc in (REPO / "docs/sdk").glob("*.md"):
        documented |= set(_MODULE.findall(doc.read_text(encoding="utf-8")))

    undocumented = sorted(promised - documented)
    assert not undocumented, (
        f"these modules are a compatibility PROMISE and the SDK never mentions "
        f"them: {undocumented}. A promise a consumer cannot find is a promise "
        "they will go into `crates/` to look for."
    )
