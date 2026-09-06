"""Prerequisite E: a ruleset that overrides shared policy must restore ALL of it.

⭐⭐ THE PATTERN THIS GUARDS, and it already exists — the program's question is
*"how does an active experience provide policy to a shared capability without
becoming permanent process-global authority?"*, and the answer shipping today is a
**PRIOR SNAPSHOT**: on activation the ruleset captures each shared resource it is
about to override into one `…Prior` struct; on deactivation it puts each one back,
or removes it if there was none.

⛔⛔ AND THE FAILURE MODE IS NOT THE PATTERN, IT IS THE FIFTH FIELD. Adding a new
override means editing THREE places — the struct, the capture, the restore — and
the restore is the one a compiler never asks for: a `…Prior` field that nobody
reads is a warning at most, and the ruleset silently keeps its policy installed
after it deactivates. `SmashPresentationPrior`'s own comment records the tally:
*"Jon's `99ab15e32` and this morning's `PlayerManaRegen` fix are the other two;
three fixes and no guard is how a shape stays broken."*

⇒ This is that guard, written from the file's own sentence. Every field declared
must be CAPTURED and must be RESTORED; the three lists have to agree.

⚠ IT IS DELIBERATELY SHAPE-BASED, NOT NAME-BASED. Any `struct …Prior` in a game
crate is checked, so a second ruleset adopting the pattern is covered on the day
it is written rather than on the day somebody remembers to add it here.
"""

from __future__ import annotations

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
GAME = REPO / "game"

PRIOR_STRUCT = re.compile(
    r"(?:pub(?:\([^)]*\))?\s+)?struct\s+(\w*Prior)\s*\{(.*?)\n\}", re.DOTALL
)
FIELD = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(\w+)\s*:", re.MULTILINE)


def _strip_comments(text: str) -> str:
    return "\n".join(line.split("//")[0] for line in text.split("\n"))


def _prior_structs() -> list[tuple[pathlib.Path, str, list[str], str]]:
    found = []
    for path in sorted(GAME.rglob("*.rs")):
        if "tests" in path.name or "/tests/" in str(path):
            continue
        raw = path.read_text(encoding="utf-8", errors="ignore")
        for match in PRIOR_STRUCT.finditer(raw):
            name, body = match.group(1), match.group(2)
            fields = FIELD.findall(_strip_comments(body))
            found.append((path, name, fields, _strip_comments(raw)))
    return found


def test_every_captured_policy_is_also_restored() -> None:
    """⛔ THE THREE LISTS MUST AGREE — declared, captured, restored.

    A field the ruleset captures and never reads back is policy it installed on a
    shared capability and left there, which is precisely the "permanent
    process-global authority" the prerequisite exists to prevent."""
    offenders: list[str] = []
    for path, name, fields, source in _prior_structs():
        for field in fields:
            captured = re.search(rf"\b{field}\s*:", source.split(f"struct {name}")[-1])
            restored = re.search(rf"\bprior\s*\.\s*{field}\b", source)
            if not restored:
                offenders.append(
                    f"{path.relative_to(REPO)}  {name}.{field} is captured and never read back"
                )
            if not captured:
                offenders.append(
                    f"{path.relative_to(REPO)}  {name}.{field} is declared and never captured"
                )
    assert not offenders, (
        "a ruleset's prior-policy snapshot does not round-trip. Each of these is "
        "an override the ruleset installs on a SHARED capability and leaves "
        "behind when it deactivates:\n  " + "\n  ".join(offenders)
    )


def test_the_guard_found_a_prior_snapshot_to_check() -> None:
    """⭐ THE POSITIVE CONTROL. The assertion above iterates a discovered set; a
    regex that stops matching `struct …Prior` — a visibility change, a derive
    moving above it, the struct gaining a lifetime — makes it pass against
    nothing at all, and there is exactly ONE instance in the tree today, so the
    set collapsing is a single edit away."""
    structs = _prior_structs()
    assert structs, (
        "no `…Prior` policy snapshot found in game/; the scoped-ruleset pattern "
        "has either been renamed or this matcher has stopped seeing it"
    )
    names = {name for _, name, _, _ in structs}
    assert "SmashPresentationPrior" in names, (
        f"the reference instance is gone; found {sorted(names)}"
    )
    fields = {f for _, name, fs, _ in structs if name == "SmashPresentationPrior" for f in fs}
    assert len(fields) >= 4, (
        f"only {len(fields)} fields parsed from the reference snapshot "
        f"({sorted(fields)}); it held four when this was written, so the field "
        "parser has stopped matching rather than the struct having shrunk"
    )
