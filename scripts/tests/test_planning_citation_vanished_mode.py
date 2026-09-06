"""`--vanished REF` must report a bare citation of a name the tree has LOST.

`check_planning_citations.py` extracts only QUALIFIED names (`SYMBOL` requires a
`::`), and its own docstring says so: a bare backticked identifier -- the
commonest citation form in these docs -- is never checked. Extending the check
to bare names outright was measured and rejected on 2026-09-02 (`d3c86dc79`):
docs/planning holds 1490 distinct bare snake_case tokens of which 542 do not
resolve, and the non-resolvers are dominated by CORRECT citations of things that
are not Rust items -- content keys, directories, upstream Bevy API, CSV columns.

⭐ `--vanished` asks a narrower question that is nearly all signal: was this name
a definition at REF, and is it not one at HEAD? The name's OWN HISTORY supplies
the precision, so no heuristic decides what is "code-shaped". That is exactly
what a decomposition carve leaves behind -- a row citing an item the carve
renamed or removed, in a form `SYMBOL` cannot see.

⛔ IT IS NOT THE FABRICATED-NAME CHECK AND MUST NOT BECOME ONE. A name that
never existed is invisible here BY DESIGN: it was not defined at the baseline
either, so it is not a regression. `test_a_name_that_never_existed_is_not_reported`
pins that, because the obvious "improvement" is to report it and that would
reintroduce the 542.
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
SCRIPT = REPO / "scripts/check_planning_citations.py"


def load():
    spec = importlib.util.spec_from_file_location("citations", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=repo, capture_output=True, text=True, check=True
    ).stdout


@pytest.fixture
def tree(tmp_path, monkeypatch):
    """A two-commit repo: `fn kept` survives, `fn carved_away` does not.

    Built rather than mocked -- `source_text_at` reads a real tree through
    `git cat-file --batch`, and a stub would pin the parser to my idea of the
    format instead of git's.
    """
    repo = tmp_path / "tree"
    repo.mkdir()
    git(repo, "init", "-q")
    git(repo, "config", "user.email", "t@t")
    git(repo, "config", "user.name", "t")
    (repo / "lib.rs").write_text("fn kept() {}\nfn carved_away() {}\n")
    # ⛔ THREE FILES, NOT ONE. A single blob cannot detect a stride error in the
    # `--batch` reader, and that error fails SILENTLY (see the sync test below).
    (repo / "second.rs").write_text("fn second_file_item() {}\n")
    (repo / "third.rs").write_text("fn third_file_item() {}\n")
    git(repo, "add", "lib.rs", "second.rs", "third.rs")
    git(repo, "commit", "-qm", "before")
    base = git(repo, "rev-parse", "HEAD").strip()
    (repo / "lib.rs").write_text("fn kept() {}\n")
    git(repo, "add", "lib.rs")
    git(repo, "commit", "-qm", "the carve")

    module = load()
    monkeypatch.setattr(module, "REPO", repo)
    return module, repo, base


def test_the_baseline_index_really_differs_from_head(tree):
    """⭐ THE PREMISE. Without this the check below could pass because the two
    indexes are identical and nothing is ever reported."""
    module, repo, base = tree
    head = module.defined_names(module.source_text())
    was = module.defined_names(module.source_text_at(base))
    assert "carved_away" in was, "the baseline tree must define the carved name"
    assert "carved_away" not in head, "and HEAD must not"
    assert "kept" in was and "kept" in head


def test_a_bare_citation_of_a_carved_name_is_reported(tree, capsys):
    module, repo, base = tree
    doc = repo / "row.md"
    doc.write_text("A row citing `carved_away`, which the carve removed.\n")
    found = module.vanished_report(
        [doc], base, module.defined_names(module.source_text())
    )
    assert found == 1, "the whole point: SYMBOL cannot see this, --vanished must"
    assert "`carved_away`" in capsys.readouterr().out


def test_a_name_that_never_existed_is_not_reported(tree, capsys):
    """⛔ BY DESIGN, and the reason is a measurement: reporting these is the
    change that yields ~408 mostly-legitimate findings."""
    module, repo, base = tree
    doc = repo / "row.md"
    doc.write_text("A row citing `totally_invented_name_here`.\n")
    assert (
        module.vanished_report([doc], base, module.defined_names(module.source_text()))
        == 0
    )


def test_a_name_that_still_exists_is_not_reported(tree):
    module, repo, base = tree
    doc = repo / "row.md"
    doc.write_text("A row citing `kept`, which survived the carve.\n")
    assert (
        module.vanished_report([doc], base, module.defined_names(module.source_text()))
        == 0
    )


def test_the_marker_suppresses_a_row_that_records_the_old_name_on_purpose(tree):
    """A row whose SUBJECT is the removal must be able to name it. Without this
    the check punishes the docs that are most correct about the carve."""
    module, repo, base = tree
    doc = repo / "row.md"
    doc.write_text(f"The carve removed `carved_away`. {module.MARKER}\n")
    assert (
        module.vanished_report([doc], base, module.defined_names(module.source_text()))
        == 0
    )


def test_the_batch_stream_stays_in_sync_across_several_blobs(tree):
    """⛔ `git cat-file --batch` returns `<oid> blob <size>\\n<bytes>\\n` back to
    back, so the reader must advance by the size AND both newlines. Off by one
    and every blob after the first is read from the wrong offset -- which does
    not raise, it silently yields garbage, and garbage indexes as "nothing was
    defined at the baseline", so the check reports nothing and looks clean.

    ⭐ THIS TEST REPLACED ONE THAT COULD NOT FAIL. The first version pinned a
    `missing`-object branch that `source_text_at` cannot reach (it asks only for
    paths `ls-tree` just listed FROM THAT SAME REF); deleting the branch left it
    green. Poisoned instead by changing the stride, which reddens it.
    """
    module, repo, base = tree
    was = module.defined_names(module.source_text_at(base))
    for name in ("kept", "carved_away", "second_file_item", "third_file_item"):
        assert name in was, (
            f"`{name}` is missing from the baseline index, so the stream "
            "desynchronised -- later blobs were read at the wrong offset"
        )


def test_a_field_that_disappeared_is_not_reported(tree, capsys):
    """⛔ MEASURED ON THE FIRST REAL RUN, not reasoned about. `defined_names`
    includes a FIELD rule, and it put `ambition_demo_pocket` in the baseline
    index but not HEAD's -- reporting five rows as citing a vanished name while
    the crate is alive at `game/ambition_demo_pocket`. A field is not the thing
    a planning row cites, and fields are renamed constantly; every true positive
    in that run came from `DEFINITION`, so the differential uses `item_names`.
    """
    module, repo, base = tree
    (repo / "lib.rs").write_text(
        # ⛔ ON ITS OWN LINE: `FIELD` anchors at `^\\s*`, so an inline
        # `struct S { f: u32 }` is not indexed and the premise below would fail
        # for the wrong reason.
        "fn kept() {}\nstruct S {\n    a_vanishing_field: u32,\n}\n"
    )
    git(repo, "add", "lib.rs")
    git(repo, "commit", "-qm", "add a field")
    field_base = git(repo, "rev-parse", "HEAD").strip()
    (repo / "lib.rs").write_text(
        "fn kept() {}\nstruct S {\n    renamed: u32,\n}\n"
    )
    git(repo, "add", "lib.rs")
    git(repo, "commit", "-qm", "rename the field")

    assert "a_vanishing_field" in module.defined_names(
        module.source_text_at(field_base)
    ), "premise: the FIELD rule does index it, which is why it had to be excluded"

    doc = repo / "row.md"
    doc.write_text("A row mentioning `a_vanishing_field`.\n")
    assert module.vanished_report([doc], field_base, set()) == 0


def test_a_live_crate_is_not_reported_when_a_mod_line_for_it_goes_away(tree):
    """⛔ ALSO MEASURED: a `mod ambition_platformer2d_actor_monolith` present at
    the baseline and absent at HEAD made the differential report the MONOLITH as
    vanished, while the crate is alive and named in Cargo.toml. The manifest is
    the authority on whether a crate exists; a `mod` line is not.
    """
    module, repo, base = tree
    (repo / "Cargo.toml").write_text('[package]\nname = "a_live_crate"\n')
    (repo / "lib.rs").write_text("fn kept() {}\nmod a_live_crate;\n")
    git(repo, "add", "Cargo.toml", "lib.rs")
    git(repo, "commit", "-qm", "declare the crate and a mod line")
    crate_base = git(repo, "rev-parse", "HEAD").strip()
    (repo / "lib.rs").write_text("fn kept() {}\n")
    git(repo, "add", "lib.rs")
    git(repo, "commit", "-qm", "the mod line goes away; the crate does not")

    assert "a_live_crate" in module.crate_names(), "premise: the manifest names it"
    doc = repo / "row.md"
    doc.write_text("A row citing `a_live_crate`.\n")
    assert module.vanished_report([doc], crate_base, set()) == 0


def test_a_range_attributes_only_what_left_inside_it(tree, capsys):
    """⭐ THE PROPERTY THE RANGE FORM EXISTS FOR. `--vanished <ref>` compares
    that ref to the WORKING TREE, so once HEAD moves past the carve it sweeps up
    every LATER removal and attributes them all to the carve. The first real
    post-carve run hit exactly this: the integrator had merged past cut 1 and
    had to hand over `c761a9d80..83460e3f3`.

    Three commits, so the two cases are separable: `alpha` leaves INSIDE the
    range and `beta` leaves AFTER it. Both existed at the base, which is what
    makes the bare-ref arm below a real premise rather than a coincidence.
    """
    module, repo, _ = tree
    (repo / "pair.rs").write_text("fn alpha() {}\nfn beta() {}\n")
    git(repo, "add", "pair.rs")
    git(repo, "commit", "-qm", "both names exist")
    a = git(repo, "rev-parse", "HEAD").strip()
    (repo / "pair.rs").write_text("fn beta() {}\n")
    git(repo, "add", "pair.rs")
    git(repo, "commit", "-qm", "alpha leaves -- inside the range")
    b = git(repo, "rev-parse", "HEAD").strip()
    (repo / "pair.rs").write_text("// both gone\n")
    git(repo, "add", "pair.rs")
    git(repo, "commit", "-qm", "beta leaves -- after the range")

    doc = repo / "row.md"
    doc.write_text("rows citing `alpha` and `beta`.\n")

    assert module.vanished_report([doc], f"{a}..{b}", set()) == 1
    out = capsys.readouterr().out
    assert "`alpha`" in out
    assert "beta" not in out.split("⇒")[0], (
        "`beta` left AFTER the range ended; attributing it to this range is the "
        "defect the range form fixes"
    )

    assert module.vanished_report([doc], a, set()) == 2, (
        "premise: against the WORKING TREE both names are gone, so a bare ref "
        "cannot tell which of them this range was responsible for"
    )


def test_an_open_ended_range_means_head(tree):
    """`A..` is the same question as a bare `A`, spelled as a range."""
    module, repo, base = tree
    doc = repo / "row.md"
    doc.write_text("citing `carved_away`\n")
    assert module.vanished_report([doc], f"{base}..", set()) == 1


def test_a_doc_outside_the_repo_is_reported_by_absolute_path(tree):
    """A fixture or a poison lives outside the tree; raising on it would make
    the mode untestable from anywhere but the repo."""
    module, repo, base = tree
    outside = repo.parent / "outside.md"
    outside.write_text("citing `carved_away`\n")
    assert (
        module.vanished_report(
            [outside], base, module.defined_names(module.source_text())
        )
        == 1
    )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))


def test_an_item_that_became_a_field_is_not_reported(tree):
    """⛔ THE MIRROR OF `test_a_field_that_disappeared_is_not_reported`, and it
    was live until 2026-09-03.

    Excluding `FIELD` from the baseline index stops a field from INVENTING a
    vanished name. Excluding it from HEAD's side too was the other half of that
    change, and it created the opposite error: a name defined as an item at the
    baseline and as a FIELD now is reported vanished, although the row citing it
    resolves perfectly.

    ⚠ Measured, not imagined: on a 2026-08-13 baseline, 4 of 45 findings were
    this shape -- `attacks`, `grounded`, `conversation` and `combat`, all short
    generic names that are fields at HEAD. `still_a_field` subtracts them.
    Subtracting can only REMOVE findings, so the failure the sibling test pins
    cannot come back through this door -- which is why the asymmetry is safe.
    """
    module, repo, base = tree
    (repo / "lib.rs").write_text("fn kept() {}\nfn attacks() {}\n")
    git(repo, "add", "lib.rs")
    git(repo, "commit", "-qm", "attacks is a function")
    item_base = git(repo, "rev-parse", "HEAD").strip()

    # The function is gone; the NAME survives as a field on a struct.
    (repo / "lib.rs").write_text(
        "fn kept() {}\nstruct Profile {\n    attacks: u32,\n}\n"
    )
    git(repo, "add", "lib.rs")
    git(repo, "commit", "-qm", "attacks is now a field")

    assert "attacks" in module.item_names(
        module.source_text_at(item_base)
    ), "premise: it WAS an item at the baseline, so the differential can see it"
    assert "attacks" not in module.item_names(
        module.source_text()
    ), "premise: it is no longer an item, which is what made this a finding"

    doc = repo / "row.md"
    doc.write_text("A row citing `attacks` on the profile.\n")
    assert module.vanished_report([doc], item_base, set()) == 0
