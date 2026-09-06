"""`#[cfg(test)]` must skip the ITEM, not the rest of the file.

⛔⛔ THE BUG THIS PINS WAS LIVE AND IT MADE THE CENSUS FALSE.
`scripts/durable_fact_writers.py` originally truncated each file at its first
`#[cfg(test)]`. `crates/ambition_encounter/src/switches.rs` opens an inline
`mod queue_checksum_tests` at line 123 and then continues with real systems —
`drain_switch_activations` holds the canonical persisted switch writes at :401,
:406 and :410. The census reported `switch` had TWO writers when it has FIVE,
and the writer it dropped is the system that calls itself the single author of
the toggle.

⚠ The script even printed a "possible misses" line and exited 0. Ambiguity is
now an ERROR, because a diagnostic that does not fail is one nobody reads.
"""

import importlib.util
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
_spec = importlib.util.spec_from_file_location(
    "durable_fact_writers", REPO / "scripts/durable_fact_writers.py"
)
_mod = importlib.util.module_from_spec(_spec)
sys.modules["durable_fact_writers"] = _mod
_spec.loader.exec_module(_mod)

FIXTURE = '''\
fn before() {
    save.data_mut().set_flag("production_a", true);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_test_that_writes() {
        // Nested braces so the scan cannot stop at the first inner `}`.
        if true {
            for _ in 0..1 {
                save.data_mut().set_flag("test_only", true);
            }
        }
    }
}

fn after() {
    save.data_mut().set_switch("production_b", true);
}
'''


def _families(text):
    lines, unclosed = _mod.production_lines(text)
    assert unclosed == [], f"scanner could not close: {unclosed}"
    found = []
    for _number, line in lines:
        for verb, family in _mod.CALL.findall(_mod._code(line)):
            found.append((verb, family))
    return found, "\n".join(line for _n, line in lines)


def test_production_before_and_after_a_test_module_are_both_counted():
    found, kept = _families(FIXTURE)
    assert ("set", "flag") in found, "the write BEFORE the test module was dropped"
    assert ("set", "switch") in found, (
        "the write AFTER the test module was dropped — this is the exact defect: "
        "truncating at the first `#[cfg(test)]` is not the same as excluding tests"
    )
    assert len(found) == 2, f"expected exactly the two production writes, got {found}"


def test_the_test_only_write_is_not_counted():
    _found, kept = _families(FIXTURE)
    assert "test_only" not in kept, "a write inside `#[cfg(test)]` was kept"
    assert "production_a" in kept and "production_b" in kept


def test_a_bodyless_cfg_test_item_does_not_swallow_the_file():
    text = 'fn a() { save.data_mut().set_flag("x", true); }\n#[cfg(test)]\nuse foo::bar;\nfn b() { save.data_mut().set_boss("y", z); }\n'
    found, _kept = _families(text)
    assert ("set", "boss") in found, (
        "`#[cfg(test)] use …;` has no braces; treating it as a braced item eats "
        "the rest of the file"
    )


def test_braces_inside_a_string_literal_do_not_open_a_block():
    """⛔ `collision.rs` asserts on `"not ron at all {{{"` — three unmatched `{`
    in a test string left the counter permanently open and reported the whole
    file as unclosed."""
    text = (
        'fn a() { save.data_mut().set_flag("x", true); }\n'
        "#[cfg(test)]\nmod t {\n    fn f() { parse(\"not ron at all {{{\"); }\n}\n"
        'fn b() { save.data_mut().set_quest("q", s, 0); }\n'
    )
    found, _kept = _families(text)
    assert ("set", "quest") in found


def test_the_shipped_corpus_really_contains_the_shape():
    """⛔ ANTI-VACUITY: the fixtures above are synthetic. If no shipped file has
    production code after an inline test module, they guard nothing real."""
    path = REPO / "crates/ambition_encounter/src/switches.rs"
    text = path.read_text(encoding="utf-8")
    lines, unclosed = _mod.production_lines(text)
    assert unclosed == []
    numbers = [n for n, line in lines if "set_switch" in _mod._code(line)]
    first_test = next(
        n for n, line in enumerate(text.splitlines(), 1) if "#[cfg(test)]" in line
    )
    assert numbers, "switches.rs no longer writes a switch; re-aim this fixture"
    assert max(numbers) > first_test, (
        f"switches.rs no longer has a production write ({numbers}) AFTER its "
        f"first `#[cfg(test)]` at :{first_test}, so this file no longer "
        "demonstrates the defect and the fixture above is the only cover left"
    )


def test_the_census_runs_clean_against_the_live_tree():
    """⛔⛔ THE EXIT CODE ONLY MEANS SOMETHING IF A LANE READS IT.

    `durable_fact_writers.py` returns 1 when it cannot close a `#[cfg(test)]`
    item, because a census that reports its own doubt and exits 0 trains every
    reader — starting with its author — to treat the doubt as decoration. That
    was the exact failure it shipped with: it printed *"files whose writes are
    ALL after a `#[cfg(test)]`: 4 — read them if non-zero"* and exited 0, and the
    miss it was warning about was real (`switches.rs` lost three production
    writes, reporting 2 switch writers where there are 5).

    ⇒ But an exit code nothing runs is the same decoration one layer down. The
    fixtures above test `production_lines` as a FUNCTION; this arm runs the
    SCRIPT, so a Rust file whose shape defeats the scanner fails a lane rather
    than waiting for somebody to run a census by hand. ~2 s.
    """
    import subprocess

    proc = subprocess.run(
        [sys.executable, str(REPO / "scripts/durable_fact_writers.py")],
        capture_output=True,
        text=True,
        cwd=REPO,
    )
    assert proc.returncode == 0, (
        "the durable-fact census could not classify part of the tree, so its "
        "writer counts are NOT authoritative:\n" + proc.stdout[-2000:]
    )
    assert "durable families named by production code:" in proc.stdout, (
        "the census produced no report at all — it may have failed before "
        "scanning, which exit 0 alone would not distinguish"
    )
