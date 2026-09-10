"""The starter item roster is inserted by exactly one production site.

⛔⛔ THIS GUARD EXISTS BECAUSE THE DUPLICATE WAS INVISIBLE TO EVERY READER.
`OwnedItems::starter()` was `insert_resource`d twice — by `ambition_app`'s
`init_sandbox_resources` and by `AmbitionItemRosterPlugin`, which the app
installs from `install_menu_setup_and_hotkeys`. The windowed app ran both and the
headless harness ran only the first, so **each composition saw exactly one** and
no test could see two. The tree even said so out loud: `plugins.rs` carried
*"the later content plugin registration is byte-identical and therefore
idempotent"* — a sync standing in for an authority.

⚠ NO VALUE EVER DIVERGED, and this guard does not claim one did: there is one
`fn starter`, both calls ran at plugin-build time, and the later won with an
identical value. What it holds is that a SECOND build-time authority for "what
the game starts owning" cannot come back — because the day the two spellings
differ, the composition picks by plugin order and nothing says which one lost.

⚠ TEXTUAL, and the alternative is worse. A behavioural test would have to
compose both roads and compare, which is what the two compositions already do
separately and why nobody noticed.
"""

from __future__ import annotations

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
ROOTS = ("crates", "game", "examples")
STARTER = re.compile(r"insert_resource\s*\(\s*(?:[\w:]*::)?OwnedItems::starter\s*\(")


TEST_MOD = re.compile(
    r"#\[cfg\(test\)\]\s*(?:(?:///?[^\n]*|//![^\n]*|#\[[^\]]*\])\s*)*"
    r"(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{"
)


def _without_test_modules(text: str) -> str:
    """The file with every `#[cfg(test)] mod ..` body BLANKED — same height.

    ⛔⛔ **EXCLUDING BY PATH IS NOT EXCLUDING TESTS, and this guard's first run
    proved it.** Filtering on `"test" in rel` found THREE production inserts;
    the third was `items/conditions.rs:109`, inside a `#[cfg(test)] mod tests`
    that begins thirteen lines above it. A fixture seeding its own bag is not a
    second authority over the shipped one, and counting it would have made this
    guard unsatisfiable by any correct tree.

    ⚠ BLANKED, NOT SPLICED, because this guard REPORTS LINE NUMBERS and a
    removed block moves every citation below it.
    """
    out, i = [], 0
    while True:
        match = TEST_MOD.search(text, i)
        if not match:
            out.append(text[i:])
            break
        out.append(text[i : match.start()])
        depth, j = 0, match.end() - 1
        while j < len(text):
            if text[j] == "{":
                depth += 1
            elif text[j] == "}":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        span = text[match.start() : j + 1]
        out.append("\n" * span.count("\n"))
        i = j + 1
    return "".join(out)


def _production_sites() -> list[str]:
    sites: list[str] = []
    for root in ROOTS:
        base = REPO / root
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.rs")):
            rel = str(path.relative_to(REPO))
            if "test" in rel:
                continue
            raw = path.read_text(encoding="utf-8", errors="replace")
            if re.search(r"^\s*#!\[\s*cfg\s*\(\s*test\s*\)\s*\]", raw, re.MULTILINE):
                continue
            for number, line in enumerate(_without_test_modules(raw).split("\n"), start=1):
                if STARTER.search(line) and not line.strip().startswith("//"):
                    sites.append(f"{rel}:{number}")
    return sites


def test_the_starter_roster_has_one_production_authority():
    sites = _production_sites()
    # ⛔ ANTI-VACUITY FIRST. A renamed constructor makes this scan find nothing,
    # and "0 sites" satisfies "not more than 1" without anyone noticing that the
    # roster is now installed by a spelling this guard cannot see.
    assert sites, (
        "no production site inserts `OwnedItems::starter()` at all. Either the "
        "starter roster is gone — in which case delete this guard and say why — "
        "or it is spelled some way this scan cannot see, which is the failure "
        "this file is about."
    )
    assert len(sites) == 1, (
        f"{len(sites)} production sites insert the starter item roster: {sites}.\n"
        "  ⇒ Two build-time inserts of the same value is one authority too many. "
        "They agree today only because there is one `fn starter`; the day the "
        "spellings differ the composition picks by plugin order and nothing says "
        "which one lost.\n"
        "  fix: keep the insert in `AmbitionContentPlugin`, which every "
        "composition installs, and let the other site read the resource."
    )
