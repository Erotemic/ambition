"""The gated-test survey must not under-report a hidden population.

⛔ **the tool exists because "the suite is green" was never a complete
sentence** (queue D57: the sanic spike tests passed 4/4 for a day in the one
configuration where input does not exist). A survey that misses a gate
reintroduces exactly that.

Only the fire paths are pinned — the ways this could report a comfortable number
that is false:

* a `#[cfg(feature)]` on the test itself;
* a `#[cfg(feature)] mod x { … }` block, which needs brace tracking;
* a `#[cfg(feature)] mod x;` DECLARATION whose tests live in another file —
  the case that made the first draft say `ambition_touch_input` runs 10 of 45
  when cargo says 4.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO_ROOT / 'scripts'))

import feature_gated_tests as fgt  # noqa: E402


def test_a_gate_on_the_test_itself_is_seen(tmp_path):
    path = tmp_path / 'a.rs'
    path.write_text(
        '#[test]\nfn plain() {}\n\n#[cfg(feature = "input")]\n#[test]\nfn gated() {}\n',
        encoding='utf8',
    )
    total, gated, features = fgt.scan_file(path)
    assert (total, gated) == (2, 1)
    assert features == {'input'}


def test_a_gated_mod_block_is_seen(tmp_path):
    """Brace tracking, not line matching: the attribute is three lines above the
    test it guards."""
    path = tmp_path / 'b.rs'
    path.write_text(
        '#[test]\nfn outside() {}\n'
        '#[cfg(feature = "audio")]\nmod inner {\n'
        '  #[test]\n  fn one() {}\n  #[test]\n  fn two() {}\n'
        '}\n'
        '#[test]\nfn after() {}\n',
        encoding='utf8',
    )
    total, gated, _ = fgt.scan_file(path)
    assert (total, gated) == (4, 2), 'the gated block must not leak, and must not over-reach'


def test_a_gated_mod_declaration_pulls_in_its_whole_file(tmp_path):
    """⛔ **the case the first draft got wrong.** `ambition_touch_input` gates
    `bevy_plugin` as a bare declaration, so its tests live in a file the
    attribute never touches — the scan called them ungated and reported 10 of 45
    where cargo measures 4."""
    crate = tmp_path / 'crate'
    (crate / 'src' / 'bevy_plugin').mkdir(parents=True)
    (crate / 'src' / 'lib.rs').write_text(
        '#[cfg(feature = "mobile_touch")]\npub mod bevy_plugin;\n#[test]\nfn bare() {}\n',
        encoding='utf8',
    )
    (crate / 'src' / 'bevy_plugin' / 'mod.rs').write_text(
        '#[test]\nfn hidden_one() {}\n#[test]\nfn hidden_two() {}\n', encoding='utf8'
    )
    total, gated, features = fgt.scan_crate(crate)
    assert total == 3
    assert gated == 2, f'the declared module\'s tests must count as gated, got {gated}'
    assert 'mobile_touch' in features


def test_an_ungated_crate_reports_nothing_hidden(tmp_path):
    """The poison. A survey that flags every crate is a list, not a finding."""
    crate = tmp_path / 'crate'
    (crate / 'src').mkdir(parents=True)
    (crate / 'src' / 'lib.rs').write_text('#[test]\nfn a() {}\n#[test]\nfn b() {}\n', encoding='utf8')
    total, gated, features = fgt.scan_crate(crate)
    assert (total, gated) == (2, 0)
    assert not features
