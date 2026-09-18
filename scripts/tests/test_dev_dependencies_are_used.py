"""What `scripts/check_dev_dependencies_are_used.py` must not get wrong.

The checker is TEXTUAL by necessity — the compiler's own answer
(`-W unused_crate_dependencies`) costs a full workspace rebuild — so the ways a
text search lies are the whole risk surface, and each one below was met for
real while the sweep was being written.
"""

from __future__ import annotations

import importlib.util
import pathlib
import subprocess
import sys

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
_SPEC = importlib.util.spec_from_file_location(
    "check_dev_dependencies_are_used",
    REPO_ROOT / "scripts" / "check_dev_dependencies_are_used.py",
)
check = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(check)


def test_the_shipped_tree_has_no_stranded_dev_dependency():
    """The ratchet itself. Seven were removed on 2026-09-18; this holds the zero."""
    done = subprocess.run(
        [sys.executable, "scripts/check_dev_dependencies_are_used.py"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )
    assert done.returncode == 0, done.stdout + done.stderr
    assert "ok: every dev-dependency is named" in done.stdout


def test_a_substring_of_a_real_word_is_not_a_reference():
    """⛔ THE ONE THAT WOULD HAVE MADE THIS SWEEP LIE.

    A loose search for `insta` returns 562 hits in `game/ambition_app` and 443
    in the actor monolith — every one inside `install`, `installs`, `installed`.
    Both crates declared `insta` and neither used it. If the pattern did not
    require `insta::` or `insta!`, this checker would have reported the tree
    clean while two stranded entries sat in it.
    """
    blob = "fn install(app: &mut App) { app.install_plugins(); } // installed"
    assert not check.is_named("insta", "1", blob)
    assert check.is_named("insta", "1", 'insta::assert_ron_snapshot!(value);')
    assert check.is_named("insta", "1", "insta!(x)")


def test_a_hyphenated_crate_is_reached_by_its_underscore_spelling():
    """`bevy-inspector-egui` is named `bevy_inspector_egui` in code."""
    assert check.is_named(
        "bevy-inspector-egui", "0.1", "use bevy_inspector_egui::quick::WorldInspectorPlugin;"
    )
    assert not check.is_named("bevy-inspector-egui", "0.1", "// bevy-inspector-egui is nice")


def test_a_renamed_dependency_counts_under_both_spellings():
    """`foo = { package = "bar" }` is reached in code by the KEY, not the package.

    Counting only the package name would report every renamed dependency as
    stranded; counting only the key would miss a crate that happens to name the
    real package. Both are accepted, which is the safe direction for a checker
    whose failure mode is a false accusation.
    """
    spec = {"package": "bar", "version": "1"}
    assert check.is_named("foo", spec, "use foo::Thing;")
    assert check.is_named("foo", spec, "use bar::Thing;")
    assert not check.is_named("foo", spec, "// neither is named here")


def test_a_target_specific_dev_dependency_table_is_not_missed():
    """`[target.'cfg(..)'.dev-dependencies]` is a dev-dependency table too.

    A sweep reading only the plain table reports a platform-gated entry as
    absent rather than as unused, which is a silent pass.
    """
    doc = {
        "dev-dependencies": {"plain": "1"},
        "target": {'cfg(target_arch = "wasm32")': {"dev-dependencies": {"gated": "1"}}},
    }
    assert set(check.dev_dependencies(doc)) == {"plain", "gated"}


def test_a_mention_of_the_table_in_a_comment_does_not_become_the_table():
    """⛔ HOW THE FIRST VERSION OF THIS SWEEP FAILED.

    It split the manifest text on the string `"[dev-dependencies]"`, which also
    matches a COMMENT mentioning the table — so it took the text after the
    comment and reported five dependencies out of the wrong crate's
    `[dependencies]` block. A TOML parser cannot make that mistake, and this
    pins that the parser is what is used.
    """
    manifest = (
        '[package]\nname = "x"\nversion = "0.1.0"\n\n'
        "# Some of these belong in [dev-dependencies] one day.\n"
        '[dependencies]\nreal_dependency = "1"\n\n'
        '[dev-dependencies]\nactual_dev = "1"\n'
    )
    import tomllib

    doc = tomllib.loads(manifest)
    assert set(check.dev_dependencies(doc)) == {"actual_dev"}, (
        "the comment's mention of the table must not contribute entries"
    )
