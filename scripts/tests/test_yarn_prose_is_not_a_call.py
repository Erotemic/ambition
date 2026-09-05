"""The Python side of one rule: only `<<…>>` is evaluated.

⭐ The Rust authority is `ambition_content::dialogue::yarn::executable_regions`;
`scripts/lib/yarn_source.py` is its Python mirror, because a Python checker
cannot call into the crate. These fixtures are the SAME PAIR the Rust seam
tests, so the mirror cannot drift into the whole-file scan the census and the
alias guard both used to do.
"""

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO / "scripts"))
from lib.yarn_source import executable_regions, executable_source  # noqa: E402

BOSS = re.compile(r'\bboss_cleared\(\s*"([^"]+)"')
GENERIC = re.compile(r'condition\(\s*"([^"]+)"')


def test_a_character_saying_a_call_is_not_a_call():
    spoken = 'Kernel Guide: boss_cleared("not_a_boss") returned TRUE.'
    assert executable_regions(spoken) == []
    assert BOSS.findall(executable_source(spoken)) == [], (
        "prose scanned as code: a misspelling in DIALOGUE would redden CI over "
        "text the interpreter never evaluates"
    )


def test_the_identical_spelling_inside_a_region_is_validated():
    evaluated = '<<if boss_cleared("not_a_boss")>>'
    assert BOSS.findall(executable_source(evaluated)) == ["not_a_boss"]


def test_the_generic_verb_has_the_same_pair():
    spoken = "Guide: condition() reads the world-fact domain directly."
    evaluated = '<<if condition("world.flag_set", "lamp")>>'
    assert GENERIC.findall(executable_source(spoken)) == []
    assert GENERIC.findall(executable_source(evaluated)) == ["world.flag_set"]


def test_an_unclosed_marker_in_prose_does_not_swallow_the_lines_below():
    text = 'Guide: I said <<loudly, and then\nhe left.\n<<if quest_active("a")>>'
    assert executable_regions(text) == ['if quest_active("a")']


def test_the_shipped_corpus_really_does_contain_prose_that_looks_like_a_call():
    """⛔ ANTI-VACUITY: these fixtures are synthetic, so prove the trap is REAL.

    If `kernel.yarn` ever stops explaining calls in prose, the fixtures above
    still pass while guarding nothing anyone writes. Measured 2026-09-05: raw 5
    `boss_cleared` / executable 3.
    """
    corpus = REPO / "game/ambition_content/assets/dialogue"
    raw = executable = 0
    for path in sorted(corpus.rglob("*.yarn")):
        text = path.read_text(encoding="utf-8", errors="replace")
        raw += len(BOSS.findall(text)) + len(GENERIC.findall(text))
        source = executable_source(text)
        executable += len(BOSS.findall(source)) + len(GENERIC.findall(source))
    assert executable >= 8, f"the corpus this rule protects went empty ({executable})"
    assert raw > executable, (
        f"no shipped .yarn line SPEAKS a call any more (raw {raw} == executable "
        f"{executable}), so these fixtures no longer describe real content"
    )
