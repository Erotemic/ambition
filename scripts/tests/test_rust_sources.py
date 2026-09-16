"""The one test-file rule, now that four gates share it.

⛔ A DEFECT HERE IS INVISIBLE AND GREEN. Every caller uses this to REMOVE files
from its corpus, so a rule that says "test" too often makes four gates report
cleaner at once, in the one direction nobody inspects.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

from lib.rust_sources import file_is_test_only, is_test_path  # noqa: E402

SOURCE_ROOTS = ("crates", "game")


def sources() -> list[Path]:
    return [path for root in SOURCE_ROOTS for path in (REPO / root).rglob("*.rs")]


def test_a_module_level_attribute_after_docs_compiles_the_file_out() -> None:
    assert file_is_test_only("//! What this proves.\n//!\n#![cfg(test)]\n\nfn arm() {}\n")


def test_the_same_spelling_inside_a_mod_is_not_the_file() -> None:
    """⛔ THE ONE WAY THIS RULE DROPS PRODUCTION CODE. `#![cfg(test)]` inside a
    `mod` block compiles out that block only. A line-anchored match reads it as
    the whole file and the file leaves the corpus with nothing to show for it."""
    assert not file_is_test_only(
        "pub fn ships() {}\n\nmod fixtures {\n    #![cfg(test)]\n    fn arm() {}\n}\n"
    )


def test_both_test_file_conventions_are_named() -> None:
    """`*_tests.rs` is the convention one copy of this rule missed for all 51
    of them, and it is declared `#[cfg(test)] mod foo_tests;` — sometimes with
    a `#[path]` attribute, so the parent directory does not give it away."""
    assert is_test_path(Path("crates/c/src/npc_flight_tests.rs"))
    assert is_test_path(Path("crates/c/src/tests.rs"))
    assert is_test_path(Path("crates/c/tests/it.rs"))
    assert not is_test_path(Path("crates/c/src/attestation.rs"))


def test_the_name_half_still_matches_something_in_the_tree() -> None:
    """⛔ AN EMPTY POPULATION IS A BROKEN SCAN, NOT A CLEAN TREE."""
    named = [path for path in sources() if is_test_path(path)]
    assert len(named) > 400, f"only {len(named)} test files by name — the rule stopped matching"


def test_the_fact_half_still_matches_something_in_the_tree() -> None:
    found = [p for p in sources() if not is_test_path(p) and file_is_test_only(p.read_text(errors="replace"))]
    assert found, "no file carries a whole-file `#![cfg(test)]` — the attribute rule is dead"


# ── the proxy, measured against the fact ───────────────────────────────────
#
# Everything above trusts a NAME. What actually removes a file from a release
# build is `#[cfg(test)]` on its `mod` line, which lives in the parent, or
# `#![cfg(test)]` at the top of the file. These two arms compare the one to the
# other, so the name stays a shortcut rather than becoming a belief.

CRATE_ROOTS = ("crates", "game")


def _crate_src(src: Path) -> Path:
    for parent in src.parents:
        if (parent / "Cargo.toml").exists():
            return parent / "src"
    return src.parent


def _cfg_on_the_item_before(text: str, end: int) -> str | None:
    match = re.search(r'#\[cfg\(([^\n]+?)\)\]\s*(?:#\[path\s*=\s*"[^"]*"\]\s*)?$', text[:end])
    return match.group(1) if match else None


def declaration(src: Path) -> tuple[Path | None, str | None]:
    """The `mod` line that pulls `src` into a build, and its `#[cfg]`."""
    plain = re.compile(r"(?:pub(?:\([^)]*\))?\s+)?mod\s+" + re.escape(src.stem) + r"\s*;")
    for parent in (
        src.parent / "mod.rs",
        src.parent / "lib.rs",
        src.parent / "main.rs",
        src.parent.parent / (src.parent.name + ".rs"),
    ):
        if parent.exists() and parent != src:
            text = parent.read_text(errors="replace")
            found = plain.search(text)
            if found:
                return parent, _cfg_on_the_item_before(text, found.start())
    # `#[path]` reaches a file the module tree would not otherwise name, and it
    # must spell the file out, so this cannot adopt somebody else's module.
    pathed = re.compile(
        r'#\[path\s*=\s*"' + re.escape(src.name) + r'"\]\s*\n\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*;'
    )
    for parent in sorted(_crate_src(src).rglob("*.rs")):
        if parent == src or parent.parent != src.parent:
            continue
        text = parent.read_text(errors="replace")
        found = pathed.search(text)
        if found:
            return parent, _cfg_on_the_item_before(text, found.start())
    return None, None


#: `test` as a bare element of the predicate: `test`, `all(test, ..)`,
#: `any(test, ..)`. The `any` form also compiles when its other arm is on, and
#: the only such arm in the tree is `feature = "test-support"`, which the second
#: arm below pins to `[dev-dependencies]`.
_BARE_TEST = re.compile(r"(?:^|[(,])test(?:$|[,)])")


def compiles_without_test(cfg: str | None) -> bool:
    if cfg is None:
        return True
    return not _BARE_TEST.search(cfg.replace(" ", ""))


def test_no_file_excluded_by_name_is_compiled_into_a_release_build() -> None:
    """⛔ THE EXCLUSION IS ONLY AS TRUE AS THE `mod` LINE BEHIND IT.

    MEASURED 2026-09-16: `enemy_projectile/tests.rs` and its `test_support.rs`
    were declared with bare `mod` lines while the module's own doc comment
    called them "test-only", so four gates dropped shipping code from their
    corpora and every one of them reported cleaner for it. Gated at
    HEAD; this arm is what keeps the next one from being silent.
    """
    shipping: list[str] = []
    orphans: list[str] = []
    for path in sources():
        if "tests" in path.parts or not is_test_path(path):
            continue
        if file_is_test_only(path.read_text(errors="replace")):
            continue
        parent, cfg = declaration(path)
        rel = path.relative_to(REPO)
        if parent is None:
            orphans.append(str(rel))
        elif compiles_without_test(cfg):
            shipping.append(f"{rel} (declared in {parent.relative_to(REPO)} as cfg={cfg})")
    assert not shipping, (
        "these files are excluded by NAME but compile into a release build, so "
        "every gate that skips them is skipping shipping code:\n  "
        + "\n  ".join(shipping)
    )
    assert orphans == ["crates/ambition_boss_encounter/src/pattern/tests.rs"], (
        "a test file no `mod` line declares never compiles, so its arms never "
        f"run and nothing says so: {orphans}"
    )


def test_the_test_support_feature_never_reaches_a_shipping_build() -> None:
    """The one gate spelled `#[cfg(any(test, feature = "test-support"))]`, which
    admits a non-test build by itself. It is test-only because every crate that
    turns the feature on does so from `[dev-dependencies]`."""
    enabled: list[str] = []
    for manifest in sorted(REPO.rglob("Cargo.toml")):
        if "target" in manifest.parts:
            continue
        section = ""
        for number, line in enumerate(manifest.read_text(errors="replace").splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith("["):
                section = stripped.strip("[]")
            elif (
                "test-support" in stripped
                and not stripped.startswith("#")
                and "dependencies" in section
                and "dev-dependencies" not in section
            ):
                enabled.append(f"{manifest.relative_to(REPO)}:{number} [{section}] {stripped}")
    assert not enabled, (
        "`test-support` is enabled outside `[dev-dependencies]`, so the modules "
        "it gates now ship and the name rule no longer excludes only test code:"
        "\n  " + "\n  ".join(enabled)
    )
