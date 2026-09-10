"""The source-text-guard exposure sweep classifies the shapes it claims to.

⛔⛔ THIS SWEEP'S OWN VERDICT IS A CLASSIFICATION, SO IT IS THE THING TO TEST.
The classifier was wrong three times while it was being written, and each time
in the same direction — reporting a well-floored guard as EXPOSED:

* a floor asserted on a count variable (`count = len(..)` then `assert count >=
  10`) read as no floor at all;
* a floor living in a SIBLING test rather than the same function read as no
  floor;
* a positive control spelled `assert checker.main() == 1` against a constructed
  fixture — the strongest control shape in this repo — read as an equality.

⇒ Each is an arm below, because a sweep whose false-positive rate is unpinned
produces a list nobody reads.
"""

from __future__ import annotations

import ast
import importlib.util
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts/measure_source_text_guard_exposure.py"


def _module():
    spec = importlib.util.spec_from_file_location("exposure", SCRIPT)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _verdict(module, src: str) -> str:
    fn = ast.parse(src).body[0]
    assert isinstance(fn, ast.FunctionDef)
    return module.verdict_for(fn)[0]


def test_a_ceiling_with_no_floor_is_exposed():
    module = _module()
    assert (
        _verdict(module, "def test_x():\n    assert len(found) <= 7\n")
        == module.Verdict.EXPOSED
    )
    assert (
        _verdict(module, "def test_x():\n    assert not offenders\n")
        == module.Verdict.EXPOSED
    )


def test_a_floor_spelled_on_a_count_variable_is_a_floor():
    """The `test_actor_construction_inversion` case."""
    module = _module()
    assert (
        _verdict(module, "def test_x():\n    assert count >= 10\n")
        == module.Verdict.FLOORED
    )


def test_a_fixture_positive_control_is_a_floor():
    """The `test_an_emptied_corpus_fails` case — run the checker, require it to fail."""
    module = _module()
    assert (
        _verdict(module, "def test_x():\n    assert module.main() == 1\n")
        == module.Verdict.FLOORED
    )
    # ⚠ AND `== 0` IS NOT ONE. A checker required to SUCCEED on a fixture says
    # nothing about whether it can still see anything.
    assert (
        _verdict(module, "def test_x():\n    assert module.main() == 0\n")
        != module.Verdict.FLOORED
    )


def test_prose_and_file_paths_are_not_anchors():
    """⛔ THE FIRST TWO RUNS OF THE SHORTLIST WERE DOCSTRINGS, THEN FILENAMES.

    A guard's docstring and its multi-paragraph failure message both join word
    characters with a `.`, and so does `Cargo.toml`. Neither is a spelling a
    formatter or a refactor can change.
    """
    module = _module()
    prose = 'x = "A rule stated in one binary. The other does not have it."\n'
    assert module.rigid_literals(prose) == []
    assert module.rigid_literals('x = "Cargo.toml"\n') == []
    assert module.rigid_literals('x = "crates/a/src/lib.rs"\n') == []
    # And a real one still is.
    assert module.rigid_literals('x = "commands.insert_resource("\n')


def test_the_sweep_refuses_an_empty_corpus(monkeypatch, tmp_path):
    """⛔ ANTI-VACUITY ON THE SWEEP ITSELF — it is a scan like the ones it judges."""
    module = _module()
    monkeypatch.setattr(module, "TEST_DIR", tmp_path)
    # ⚠ `main()` PARSES sys.argv, which under pytest is pytest's own command
    # line. Point it at an empty argument list so the arm exercises the corpus
    # refusal rather than argparse.
    monkeypatch.setattr("sys.argv", ["measure_source_text_guard_exposure.py"])
    try:
        module.main()
    except AssertionError as error:
        assert "lost its own corpus" in str(error)
    else:
        raise AssertionError(
            "the sweep reported on an EMPTY guard directory instead of refusing; "
            "an empty population is the failure this whole file is about"
        )


def test_the_rust_half_reads_the_assertion_shapes():
    module = _module()
    floored, _ = module.classify_rust_file('assert!(scaled.len() >= 2, "broken");')
    assert floored == module.Verdict.FLOORED
    exposed, _ = module.classify_rust_file("assert!(offenders.is_empty());")
    assert exposed == module.Verdict.EXPOSED
    presence, _ = module.classify_rust_file('assert!(source.contains("Signal"));')
    assert presence == module.Verdict.FLOORED, (
        "a PRESENCE assertion over scanned source is an anti-vacuity floor: the "
        "scan going blind fails it"
    )
    absence, _ = module.classify_rust_file('assert!(!source.contains("Banned"));')
    assert absence == module.Verdict.EXPOSED, (
        "an ABSENCE assertion passes on a blind scan, which is the silent direction"
    )


def test_cross_evidence_is_not_an_exposure():
    """⛔⛔ THE `*_it_sync` SHAPE, which the shape classifier called exposed ×4.

    One set from source text, one from a directory listing, each difference
    asserted empty. Blinding either side leaves the OTHER full, so it reddens.
    That is a stronger anti-vacuity than a count floor, and reporting it as an
    exposure would have sent someone to "fix" the best guard shape in the tree.
    """
    module = _module()
    src = (
        "let missing: Vec<_> = on_disk.difference(&declared).collect();\n"
        "assert!(missing.is_empty(), \"not included\");\n"
        "let orphaned: Vec<_> = declared.difference(&on_disk).collect();\n"
        "assert!(orphaned.is_empty(), \"no source\");\n"
    )
    verdict, _ = module.classify_rust_file(src)
    assert verdict == module.Verdict.CROSS_CHECKED
    # ⚠ ONE difference call is not cross-evidence -- only one side is checked.
    one_way = (
        "let missing: Vec<_> = on_disk.difference(&declared).collect();\n"
        "assert!(missing.is_empty(), \"not included\");\n"
    )
    assert module.classify_rust_file(one_way)[0] == module.Verdict.EXPOSED


def test_a_file_with_no_assertions_is_not_called_a_guard():
    module = _module()
    verdict, shapes = module.classify_rust_file("fn bake() { let _ = 1; }")
    assert verdict == module.Verdict.NOT_A_GUARD and shapes == []


def test_the_rust_sweep_refuses_a_shrunken_corpus(monkeypatch):
    module = _module()
    monkeypatch.setattr(module, "rust_guard_files", lambda: [])
    try:
        module.report_rust()
    except AssertionError as error:
        assert "lost its corpus" in str(error)
    else:
        raise AssertionError("the Rust sweep reported on an empty corpus")
