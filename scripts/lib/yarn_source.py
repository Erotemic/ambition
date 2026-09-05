"""What a `.yarn` file actually EXECUTES — the one Python definition.

⭐⭐ MIRROR OF `ambition_content::dialogue::yarn::executable_regions`, which is
the authority. Python cannot call into the crate, so this is the one place the
rule is restated, and `scripts/tests/test_yarn_prose_is_not_a_call.py` holds the
same fixture pair as the Rust seam so the two cannot drift.

⛔⛔ A `.yarn` FILE IS MOSTLY SPOKEN LINES. Only `<<…>>` is evaluated; everything
else is a character talking, and a character may say anything — including the
exact spelling of a call. `kernel.yarn` has the Kernel Guide explain
`boss_cleared("mockingbird") returned TRUE.` in dialogue. Instruments that
scanned whole files over-reported authored demand by 25% (measured 2026-09-05)
and could have reddened CI over a misspelling in the WRITING.
"""

__all__ = ["executable_regions", "executable_source"]


def executable_regions(text: str) -> list[str]:
    """Every `<<…>>` body in `text`, in order. Prose is not included.

    ⛔ REGIONS DO NOT SPAN LINES. An unclosed `<<` in prose is a typo an author
    can make; a line-spanning match would run to the next `>>` further down and
    hand a caller MORE prose than a whole-file scan did. (This mirror shipped
    with `re.DOTALL` for about an hour; the shared fixture caught it.)
    """
    regions: list[str] = []
    for line in text.splitlines():
        rest = line
        while True:
            open_at = rest.find("<<")
            if open_at < 0:
                break
            after = rest[open_at + 2 :]
            close_at = after.find(">>")
            if close_at < 0:
                break
            regions.append(after[:close_at].strip())
            rest = after[close_at + 2 :]
    return regions


def executable_source(text: str) -> str:
    """The evaluated parts of a `.yarn` file, joined — what a regex may scan."""
    return "\n".join(executable_regions(text))
