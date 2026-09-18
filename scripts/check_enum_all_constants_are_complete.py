#!/usr/bin/env python3
"""Every `const ALL: [Self; N]` must list every variant of its own enum.

⛔⛤ **`ALL` IS A SECOND OWNER OF "WHICH VARIANTS EXIST", AND THE FIRST OWNER IS
THE ENUM.** Nothing in Rust holds the two together: adding a variant compiles,
and the hand-written array keeps its old length happily. What the omission costs
depends on who reads `ALL` — a settings row that never renders
(`SettingsOptionId::ALL`, 59 entries), a developer toggle that cannot be reached
(`DevToggleId::ALL`, 22), an item the catalogue never enumerates
(`Item::ALL`, 24) — and in every case the symptom is an ABSENCE, which is the
shape nobody notices.

⭐ **MEASURED 2026-09-18 BEFORE THIS GUARD WAS WRITTEN: 41 of 41 agree.** That is
the argument FOR the ratchet rather than against it. There is nothing to repair,
so the whole value is that the next divergence cannot land quietly; a guard
written the day after a row goes missing is a guard written too late.

⚠ **THE LENGTH IS CHECKED TOO, AND IT IS NOT REDUNDANT.** `[Self; N]` makes the
compiler count the array, so a WRONG `N` is a build error — but only against the
array, never against the enum. `N` is checked here because the number is what a
reader quotes.

Usage::

    python3 scripts/check_enum_all_constants_are_complete.py
"""

from __future__ import annotations

import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "scripts" / "lib"))

from rust_source import strip_comments  # noqa: E402
from test_paths import strip_test_modules  # noqa: E402

#: Where production Rust lives. Submodules under `tools/` are measured and routed
#: by other instruments and are never edited from here, so they stay out.
ROOTS = ("crates", "game")

ENUM = re.compile(r"^\s*(?:pub(?:\([a-z:]+\))?\s+)?enum\s+([A-Z]\w*)", re.M)
IMPL = re.compile(r"^\s*impl\s+([A-Z]\w*)\s*\{", re.M)
ALL = re.compile(
    r"const\s+ALL\s*:\s*\[\s*(?:Self|[A-Z]\w*)\s*;\s*([A-Za-z0-9_]+)\s*\]\s*=\s*\["
)
ATTRIBUTE = re.compile(r"#\[[^\]]*\]")

#: Enums whose `ALL` is deliberately not the whole declaration, each with the
#: reason. ⛔ AN ENTRY HERE IS AN ARGUMENT, NOT A SILENCER: a variant left out of
#: `ALL` is invisible to every consumer that iterates it, so the reason has to say
#: who is allowed not to see it.
#:
#: Empty on 2026-09-18 because nothing needed one. Kept because the legitimate
#: case is real — a variant behind a `#[cfg(feature = ...)]` that `ALL` cannot
#: name unconditionally — and discovering it as a red with nowhere to put the
#: answer is how a guard gets deleted instead of amended.
PARTIAL_BY_DESIGN: dict[str, str] = {}


def _balanced(text: str, start: int, opener: str, closer: str) -> tuple[str, int]:
    """The contents of the `opener`…`closer` pair beginning at/after `start`."""
    first = text.index(opener, start)
    depth = 0
    for index in range(first, len(text)):
        if text[index] == opener:
            depth += 1
        elif text[index] == closer:
            depth -= 1
            if depth == 0:
                return text[first + 1 : index], index
    return "", len(text)


def variants(body: str) -> list[str]:
    """Variant names declared in an enum body, in order.

    ⚠ SPLIT AT DEPTH ZERO. A tuple variant (`Cycle { index, count }`,
    `Toggle(bool)`) carries its own commas, and splitting the body on every comma
    reads those as extra variants — which would make this guard report a
    DIFFERENT number from the compiler for exactly the enums most worth checking.
    """
    out: list[str] = []
    depth = 0
    current = ""
    for char in body:
        if char in "({[":
            depth += 1
        elif char in ")}]":
            depth -= 1
        if char == "," and depth == 0:
            out.append(current)
            current = ""
        else:
            current += char
    out.append(current)
    names = []
    for chunk in out:
        chunk = ATTRIBUTE.sub("", chunk).strip()
        match = re.match(r"^([A-Z]\w*)", chunk)
        if match:
            names.append(match.group(1))
    return names


def production_files() -> list[pathlib.Path]:
    found: list[pathlib.Path] = []
    for root in ROOTS:
        found.extend(sorted((REPO / root).rglob("*.rs")))
    return found


def rows() -> list[dict]:
    """One row per `const ALL` found, with what it listed and what was declared."""
    out = []
    for path in production_files():
        raw = path.read_text(errors="replace")
        if "const ALL" not in raw:
            continue
        source = strip_test_modules(strip_comments(raw))
        declared = {}
        for match in ENUM.finditer(source):
            body, _ = _balanced(source, match.end(), "{", "}")
            declared[match.group(1)] = variants(body)
        for match in IMPL.finditer(source):
            body, _ = _balanced(source, match.end() - 1, "{", "}")
            found = ALL.search(body)
            if not found:
                continue
            array, _ = _balanced(body, found.end() - 1, "[", "]")
            listed = re.findall(r"(?:Self|[A-Z]\w*)::([A-Z]\w*)", array)
            out.append(
                {
                    "file": str(path.relative_to(REPO)),
                    "type": match.group(1),
                    "declared_len": found.group(1),
                    "listed": listed,
                    "variants": declared.get(match.group(1)),
                }
            )
    return out


def faults(found: list[dict]) -> list[str]:
    problems = []
    for row in found:
        where = f"{row['file']}: `{row['type']}::ALL`"
        if row["variants"] is None:
            problems.append(
                f"  {where} — the enum is not declared in this file, so this guard "
                "cannot compare the two. Move the constant beside its enum, or add "
                "the type to PARTIAL_BY_DESIGN with the reason."
            )
            continue
        reason = PARTIAL_BY_DESIGN.get(row["type"])
        missing = [v for v in row["variants"] if v not in row["listed"]]
        extra = [v for v in row["listed"] if v not in row["variants"]]
        if missing and not reason:
            problems.append(
                f"  {where} omits {missing} — every consumer that iterates `ALL` "
                "cannot see them, and the symptom is an ABSENCE (a row that never "
                "renders, a toggle nothing reaches)."
            )
        if extra:
            problems.append(f"  {where} lists {extra}, which the enum does not declare.")
        length = row["declared_len"]
        if length.isdigit() and int(length) != len(row["listed"]):
            problems.append(
                f"  {where} declares `[Self; {length}]` and lists {len(row['listed'])}."
            )
    return problems


def main() -> int:
    found = rows()
    # ⛔ ANTI-VACUITY. Two regexes and a brace walker stand between this guard and
    # an empty population, and an empty population agrees with everything. The
    # floor is well under the 41 measured so ordinary growth does not trip it,
    # and the named members are the three whose absence would cost the most.
    if len(found) < 30:
        print(
            f"⛔ only {len(found)} `const ALL` constant(s) found; the tree had 41 on "
            "2026-09-18. The parser has stopped seeing its population, so a clean "
            "result here means nothing."
        )
        return 2
    by_type = {row["type"] for row in found}
    for required in ("SettingsOptionId", "DevToggleId", "Item"):
        if required not in by_type:
            print(f"⛔ `{required}::ALL` is no longer in the population this guard reads.")
            return 2

    problems = faults(found)
    if problems:
        print(f"⛔ {len(problems)} `ALL` constant(s) disagree with their own enum:\n")
        print("\n".join(problems))
        return 1
    print(
        f"ok: all {len(found)} `const ALL` constants list every variant their enum "
        f"declares ({sum(len(row['listed']) for row in found)} variants across "
        f"{len({row['file'] for row in found})} file(s))"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
