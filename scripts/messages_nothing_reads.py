#!/usr/bin/env python3
r"""Which message types are DECLARED to a channel that nothing in this tree reads?

⭐ THE QUESTION, and it is not "is this dead". 2026-09-06, `ambition_load`: seven
copies of `events.push(LoadEvent::PlanChanged { .. })` collapsed into one helper,
and poisoning the helper to emit NOTHING left all 13 tests of the crate green.
`LoadEvent` has no reader anywhere in the tree — and it is not dead. That crate is
composed by EXTERNAL consumers; its own plugin test exists because *"the external
consumer, invisible to a repo grep, sat red until somebody read the panic."*

⇒ **A message with no in-tree reader is one of at least two things and they want
OPPOSITE fixes:**

    PUBLISHED  the reader is downstream, outside this repo  -> TEST THE EMISSION
               HERE, because no downstream failure can report a regression in it
    DEAD       nobody could read it, in or out               -> delete it

⛔⛔ SO THIS REPORTS AND DOES NOT FAIL ON "UNREAD". Which one a given message is
depends on whether its crate is composed from outside — an external-consumer
fixture, a `MinimalXPlugins` group, a doc comment about a stranger composing it.
That is a judgement, and a guard that made it automatically would be wrong half
the time in a direction that deletes a published API.

⛔⛔ AND THE INSTRUMENT IS THE HARD PART. A naive `git grep 'MessageReader<T>'`
was wrong THREE TIMES in a row while this file was being written:

    single-line only     missed every multi-line SystemParam declaration
    `T\s*>`              missed the TRAILING COMMA in `T,\n    >` -- which is how
                         `FreshAttempt` declares its `RoomLoaded` reader, so the
                         most-read message in the tree reported ZERO readers
    counted comments     the only "reader" of `LoadEvent` was a doc comment
                         *about* there being no reader

⇒ Angle brackets are BALANCED here (the same approach `ecs_inventory.py` uses),
lifetimes are dropped, the payload is the LAST type parameter, and line comments
are stripped before anything is matched. ⚠ It still cannot see a reader that
arrives through a wrapper `SystemParam` under a name it does not know -- see
`room_replay_reader_slots.py`, which solves that for ONE message by naming the
wrapper. A message read only through such a wrapper appears here as unread, so
CHECK ONE BEFORE BELIEVING THE LIST.
"""
from __future__ import annotations

import importlib.util
import pathlib
import re
import sys

#: ⭐ ONE AUTHORITY FOR "WHAT IS PRODUCTION CODE". `durable_fact_writers.py`
#: already excludes `#[cfg(test)]` items BY POSITION (this repo puts
#: `#[cfg(test)] mod tests` inside ordinary source files, so a path filter is
#: not enough) and it has already had its own truncation bug found and fixed.
#: Re-deriving that here would be a second, worse copy of the same rule.
_SPEC = importlib.util.spec_from_file_location(
    "durable_fact_writers", pathlib.Path(__file__).resolve().parent / "durable_fact_writers.py"
)
_DFW = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_DFW)
production_lines = _DFW.production_lines

REPO = pathlib.Path(__file__).resolve().parent.parent
#: The population must clear this or the sweep, not the tree, is broken.
MIN_DECLARED = 40


def rust_files() -> list[pathlib.Path]:
    """Every workspace source tree — INCLUDING `examples/`.

    ⛔⛔ THE BLIND SPOT THAT MISCLASSIFIED `SemanticActionPressed`. It scanned
    `crates/` and `game/` only, so the message reported no reader — while
    `examples/capability_demo` exists precisely to demonstrate that seam and says
    in its own docs that *"a press comes back as `SemanticActionPressed`"* and the
    COMPOSITION owns the last hop. ⇒ an example crate is exactly where a
    published seam's consumer lives, so leaving it out biased the sweep toward
    calling published things unread.
    """
    return sorted(
        [
            *REPO.glob("crates/**/*.rs"),
            *REPO.glob("game/**/*.rs"),
            *REPO.glob("examples/**/*.rs"),
        ],
        key=str,
    )


def strip_line_comments(text: str) -> str:
    """Drop `//` and `///` lines.

    ⛔ The first version of this file counted a doc comment as a reader: the sole
    hit for `MessageReader<LoadEvent>` was a comment SAYING there is no reader.
    """
    return "\n".join(
        "" if line.lstrip().startswith("//") else line for line in text.split("\n")
    )


def balance_angle_end(text: str, open_pos: int) -> int | None:
    depth = 0
    for index in range(open_pos, len(text)):
        if text[index] == "<":
            depth += 1
        elif text[index] == ">":
            depth -= 1
            if depth == 0:
                return index
    return None


def generic_inners(text: str, name: str) -> list[str]:
    inners: list[str] = []
    # ⚠ `add_message::<T>()` is a TURBOFISH and `MessageReader<T>` is not, so the
    # `::` has to be optional. Without it the sweep found ZERO declarations and
    # the floor below fired — which is the floor doing its job on its own author.
    for match in re.finditer(rf"\b{re.escape(name)}\s*(?:::)?\s*<", text):
        end = balance_angle_end(text, match.end() - 1)
        if end is not None:
            inners.append(text[match.end() : end].strip())
    return inners


def payload(inner: str) -> str:
    """The message type named by a generic argument list.

    Lifetimes are dropped and the LAST parameter wins, so
    `MessageReader<'w, 's, a::b::RoomLoaded,>` and `MessageReader<LoadEvent>`
    both answer with the short name.
    """
    parts = [p.strip() for p in inner.split(",")]
    parts = [p for p in parts if p and not p.startswith("'")]
    if not parts:
        return ""
    return parts[-1].split("::")[-1].strip()


#: What you can do to a `Messages<T>` handle that makes it a READER.
_READ_VERBS = ("drain", "iter", "get_cursor", "read")


def messages_read_directly(text: str) -> list[tuple[str, int]]:
    """`Messages<T>` handles that are actually READ, not written."""
    found: list[tuple[str, int]] = []
    for match in re.finditer(r"\bMessages\s*<", text):
        end = balance_angle_end(text, match.end() - 1)
        if end is None:
            continue
        name = payload(text[match.end() : end])
        if not name:
            continue
        # The verb may sit lines below through a `let` binding, so look at the
        # statement this handle belongs to rather than the same line.
        tail = text[end : end + 400]
        statement = tail.split(";", 1)[0]
        if any(f".{verb}" in statement for verb in _READ_VERBS):
            found.append((name, text[: match.start()].count("\n") + 1))
    return found


def is_generic_parameter(name: str) -> bool:
    """Rust's convention for a type parameter: one or two capitals, no lowercase.

    ⚠ A heuristic, and it is the right one only because the alternative — parsing
    the enclosing item's generics — buys nothing here. If a real message is ever
    named `Ev` this will hide it, so the count is printed alongside.
    """
    return len(name) <= 2 and name.isupper()


def main() -> int:
    declared: dict[str, list[str]] = {}
    read: dict[str, list[str]] = {}
    for path in rust_files():
        raw = path.read_text(encoding="utf-8", errors="replace")
        kept, _unclosed = production_lines(raw)
        text = strip_line_comments("\n".join(line for _number, line in kept))
        rel = str(path.relative_to(REPO))
        for inner in generic_inners(text, "add_message"):
            name = payload(inner)
            # ⚠ A GENERIC PARAMETER IS NOT A MESSAGE TYPE. `ledger.rs:246` reads
            # `app.add_message::<M>()` inside `fn register<M: Message + Clone>`,
            # so `M` is the caller's choice, registered once per instantiation.
            # A sweep that reports it names a type that does not exist.
            if name and name.isidentifier() and not is_generic_parameter(name):
                declared.setdefault(name, []).append(rel)
        # ⭐⭐ TWO ROADS, AND THE SECOND ONE COST ME A FALSE POSITIVE IN PRINT.
        # `RunAuthoredCommand` was published as unread. It IS read — by
        # `authored_logic/commands.rs:312`, which takes
        # `world.get_resource_mut::<Messages<RunAuthoredCommand>>()` and
        # `.drain()`s it, with a comment saying why a `MessageReader` cursor
        # would be wrong there (the cursor is `Local` state GGRS never rewinds).
        for inner in generic_inners(text, "MessageReader"):
            name = payload(inner)
            if name:
                read.setdefault(name, []).append(rel)
        # ⛔⛔ BUT `Messages<T>` IS NOT A READ ROAD BY ITSELF, and counting it as
        # one over-corrected in two directions at once: `select_screen.rs:2520`
        # takes `Messages<TouchInput>` to `.write(..)` it — the opposite of a
        # read — and `semantic.rs:926` drains `SemanticActionPressed` inside a
        # `#[cfg(test)]` helper. So the handle must be followed by a READ verb,
        # and the whole sweep runs on production lines only.
        for name, _where in messages_read_directly(text):
            read.setdefault(name, []).append(rel)

    if len(declared) < MIN_DECLARED:
        print(
            f"FAIL: only {len(declared)} message type(s) declared — the sweep is "
            f"broken, not the tree.\n  Expected at least {MIN_DECLARED}.",
            file=sys.stderr,
        )
        return 1

    unread = sorted(name for name in declared if name not in read)
    print(f"message types declared with `add_message`: {len(declared)}")
    print(f"  with at least one `MessageReader` in this tree: {len(declared) - len(unread)}")
    print(f"  with NONE: {len(unread)}")
    for name in unread:
        sites = sorted(set(declared[name]))
        where = sites[0] + (f" (+{len(sites) - 1} more)" if len(sites) > 1 else "")
        print(f"    {name:28} declared in {where}")
    print(
        "\n⇒ Reported, not enforced. An unread message is PUBLISHED (the reader is\n"
        "  downstream — test the EMISSION here) or DEAD (delete it). Deciding which\n"
        "  needs to know whether the crate is composed from outside this repo."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
