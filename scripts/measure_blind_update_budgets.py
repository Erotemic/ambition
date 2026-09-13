#!/usr/bin/env python3
"""Census the `app_it` shape that produced the gate's flaky-binary victim.

⛔⛤ WHY THIS EXISTS, AND WHAT THE MECHANISM ACTUALLY WAS.
`smash_cpu_cognition::both_emmy_seats_receive_one_cognitive_stream_in_the_real_host`
failed in the gate with *"only 48 frames had two seated bodies, so the match did
not really run"*. Instrumenting it showed the match never ended early: the FIRST
two-seated tick varied `5,5,5,5,37,35,5,32,5,45` across ten runs of ONE binary
while the LAST was 137 every time. ⇒ A non-deterministic seating latency ate the
START of a FIXED observation window, and the test reported that as a gameplay
result. The repair was structural: WAIT for the premise (bounded, failing loudly
if it never arrives), then measure a full window.

⭐⭐ THE SIGNATURE IS NOT "A FIXED BUDGET". Most fixed budgets in this suite are
fine — *"hold the stick for 20 frames"* is not a race, and a budget spent after a
converging wait measures from a known state. The fragile shape is narrower:

    A FIXED BUDGET THAT ACCUMULATES CONDITIONALLY.

A loop that cannot stop early, whose body collects a sample only when some
premise holds, followed by an assertion on how much it collected. Every frame the
premise takes to arrive is a frame subtracted from the measurement, and the
failure text cannot tell "started late" from "ended early" — which want opposite
investigations.

⛔ IT SCANS EVERY `fn`, NOT ONLY `#[test]` ONES, and that is the correction that
made it work: the original victim's loop lives in a HELPER (`play_mirror_match`),
so a scanner that reads only test bodies reports a clean suite and is wrong. A
negative result here would have been a claim about the query.

Run:  python scripts/measure_blind_update_budgets.py [--root DIR]
"""
import argparse
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent

FN = re.compile(r"\n\s*(?:pub(?:\([^)]*\))?\s+)?fn\s+(\w+)")
LOOP = re.compile(r"for\s+(?:_\w*|\w+)\s+in\s+0\.\.")
UPDATE = re.compile(r"\.update\(\)")
# Accumulation: pushing a sample, or counting one.
ACCUMULATE = re.compile(r"\.push\(|\.insert\(|\+=\s*1|\.push_str\(")
COUNTED = re.compile(r"\.len\(\)|\.count\(\)|\.is_empty\(\)")


def brace_span(src: str, open_at: int) -> int:
    depth = 0
    i = open_at
    while i < len(src):
        if src[i] == "{":
            depth += 1
        elif src[i] == "}":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return len(src) - 1


def functions(src: str):
    for match in FN.finditer(src):
        open_at = src.find("{", match.end())
        if open_at < 0:
            continue
        # A `fn` whose signature spans to a `where` clause or a return type still
        # opens its body at the first `{` after the name in practice; a generic
        # bound containing `{` does not occur in this tree.
        close_at = brace_span(src, open_at)
        yield match.group(1), match.start(), src[open_at : close_at + 1]


def fragile_loops(body: str):
    """Budget loops that accumulate conditionally and cannot stop early."""
    found = []
    for loop in LOOP.finditer(body):
        open_at = body.find("{", loop.start())
        if open_at < 0:
            continue
        close_at = brace_span(body, open_at)
        inner = body[open_at : close_at + 1]
        if not UPDATE.search(inner):
            continue
        if "break" in inner:
            continue  # a bounded WAIT, not a budget
        if not ACCUMULATE.search(inner):
            continue  # spends frames, collects nothing: not a window
        if "if " not in inner:
            continue  # collects unconditionally: the window cannot be eaten
        found.append(body[loop.start() : open_at].strip())
    return found


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--root",
        default=str(ROOT / "game" / "ambition_app" / "tests"),
        help="directory of Rust test sources to scan",
    )
    args = parser.parse_args()

    root = pathlib.Path(args.root)
    scanned_fns = 0
    rows = []
    for path in sorted(root.rglob("*.rs")):
        src = path.read_text()
        for name, _offset, body in functions(src):
            scanned_fns += 1
            loops = fragile_loops(body)
            if not loops:
                continue
            # Does anything in the file assert on how much was collected? That is
            # what turns an eaten window into a red test rather than a quiet one.
            counted = bool(COUNTED.search(src))
            rows.append((path.name, name, loops, counted))

    print(f"scanned {scanned_fns} function(s) under {root}\n")
    print(
        f"CONDITIONALLY-ACCUMULATING FIXED WINDOWS — {len(rows)} function(s). Each "
        "spends a budget it cannot cut short and collects only while a premise\n"
        "holds, so latency before the premise arrives is subtracted from the "
        "measurement:\n"
    )
    for file_name, name, loops, counted in sorted(rows):
        mark = "counted" if counted else "NOT COUNTED"
        print(f"    {file_name}::{name}  ({mark})")
        for loop in loops:
            print(f"        {loop}")
    if not rows:
        print("    (none)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
