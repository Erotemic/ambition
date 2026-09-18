"""The one test-file rule in `lib.test_paths`, now that five gates share it.

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

from lib.test_paths import (  # noqa: E402
    TEST_DIR_PARTS,
    TEST_FILE_NAMES,
    file_is_test_only,
    is_test_path,
)


def named_as_test(path: Path) -> bool:
    """The NAME half alone. The arms below compare it against the fact, so they
    must be able to ask the two questions separately."""
    return bool(TEST_DIR_PARTS & set(path.parts)) or (
        path.name in TEST_FILE_NAMES or path.name.endswith("_tests.rs")
    )

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
    assert named_as_test(Path("crates/c/src/npc_flight_tests.rs"))
    assert named_as_test(Path("crates/c/src/tests.rs"))
    assert named_as_test(Path("crates/c/tests/it.rs"))
    assert not named_as_test(Path("crates/c/src/attestation.rs"))


def test_the_name_half_still_matches_something_in_the_tree() -> None:
    """⛔ AN EMPTY POPULATION IS A BROKEN SCAN, NOT A CLEAN TREE."""
    named = [path for path in sources() if named_as_test(path)]
    assert len(named) > 400, f"only {len(named)} test files by name — the rule stopped matching"


def test_the_fact_half_still_matches_something_in_the_tree() -> None:
    found = [
        p
        for p in sources()
        if not named_as_test(p) and file_is_test_only(p.read_text(errors="replace"))
    ]
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
        if "tests" in path.parts or not named_as_test(path):
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
    # ⛔⛤ **THE LIST IS EMPTY AND MUST STAY EMPTY.** Its one entry,
    # `ambition_boss_encounter/src/pattern/tests.rs`, was declared on 2026-09-16
    # and its 36 arms passed on their first ever run. An entry here is a file
    # whose arms `cargo test` reports as neither passed nor failed, because it
    # has never heard of them — so a reader counting test files sees coverage
    # that does not exist.
    assert orphans == [], (
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


def test_the_stripper_keeps_everything_after_an_inline_test_module() -> None:
    """⛔⛤ THE HALF THAT MOVED HERE 2026-09-17, AND WHY.

    Three consumers held three answers to *"strip the test part of this source"*:
    the rollback-mutator guard's brace-balanced version (now this one),
    `multi_writer_resource_census.py`'s own copy, and
    `architecture_census.py`'s `text.split("#[cfg(test)]", 1)[0]` — which
    discards the FILE TAIL. A module declares its tests near the top in this
    tree, so that third one was hiding 12% of the census's own population:
    optional `Res`/`ResMut` read 731 with the tail cut and 820 per item, and
    `.before`/`.after` edges read 475 against 588.
    """
    from lib.test_paths import strip_test_modules

    src = (
        "fn before() {}\n"
        "#[cfg(test)]\nmod tests;\n"
        "fn after_a_declaration() {}\n"
        "#[cfg(test)]\nmod inline {\n    fn fixture() {}\n"
        "    mod deeper { fn nested_fixture() {} }\n}\n"
        "fn after_a_block() {}\n"
    )
    kept = strip_test_modules(src)
    assert "fn before()" in kept
    assert "fn after_a_declaration()" in kept, "the file TAIL is the defect this replaces"
    assert "fn after_a_block()" in kept
    assert "fixture" not in kept and "nested_fixture" not in kept


def test_the_stripper_leaves_a_cfg_test_item_that_has_no_block() -> None:
    """⚠ THE RESIDUAL, STATED RATHER THAN DISCOVERED. `#[cfg(test)] use ...;` and
    `#[cfg(test)] fn helper() { .. }` are not `mod NAME {`, so they survive. No
    consumer's counts change for it today — measured across
    `architecture_census.py`'s corpus, per-item stripping and no stripping at all
    differ by three occurrences — and widening this reaches every consumer at
    once, in the direction that makes each of them report cleaner."""
    from lib.test_paths import strip_test_modules

    assert "helper" in strip_test_modules("#[cfg(test)]\nfn helper() {}\n")
