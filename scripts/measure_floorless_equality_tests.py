#!/usr/bin/env python3
"""Tests that compare two runs of one call and cannot notice an empty answer.

⭐ **THE SHAPE.** `assert_eq!(left, right)` where both sides come from the same
helper is the standard "same input, same output" guard — determinism, order
independence, replay safety. It has one failure mode: **if the helper returns
nothing, the two sides agree trivially and the test passes while measuring an
empty vector.** The subject is supplied by a fixture, and nothing asserts the
fixture supplied it.

⛔⛔ **FOUND BY A LIVE CASE, NOT INVENTED.** `the_same_seed_produces_the_same_fighter`
in `ambition_combat` guarded the fighter brain's replay determinism by running
two identically-seeded brains and comparing their press vectors. Measured
2026-09-10 before the fix: **0 presses of 90 frames, and the seed had not
moved** -- `run` hands the brain an empty `attack_kit`, so no press was ever
wanted and the noise stream was never sampled. The guard for the stream's
determinism never exercised the stream. It is the positive control below.

⚠ **A SCALAR COMPARISON CANNOT FAIL THIS WAY**, so the screen requires the
compared binding to be built as something that can be empty -- a `Vec`, a
`HashMap`, a `collect()`. Without that narrowing the screen reports 46 hits, of
which the great majority compare two numbers and have no emptiness to hide.

⚠ **AND A HIT IS A SHAPE, NOT A VERDICT.** Comparing against a non-empty
CONSTANT is safe even with no floor: if the built side came back empty, it would
differ from the constant and the test would fail. Both survivors were read
before this file named either of them.

⭐ **MEASURED 2026-09-10: 7916 test bodies across 1075 files, TWO hits, and one
of them is sound.** That is a good negative result -- the tree is broadly
disciplined about this -- and it is worth more with the two verdicts attached:

  * `gate_portal::the_phase_projection_folds_in_key_order_whatever_the_insertion_order`
    -- **SOUND.** `seen` is compared against `sorted_keys`, derived from a
    hard-coded 8-entry `vec!`. An empty `seen` would DIFFER from it and the test
    would fail, so the floor is carried by the constant.

  * `grid_backend::cross_backend_model_parity_inventory_and_system`
    -- ⛔⛔ **CANNOT FAIL, and the emptiness is the lesser half.** Both sides come
    from the SAME closure: `let cube_pages = build(); let grid_pages = build();`
    with no backend argument anywhere. It asserts `build() == build()`, which is
    the purity of one function, and its doc claims *"the active tab's
    `MenuPageModel` is built from the SAME backend-agnostic builders regardless
    of which backend renders it"* -- a claim the fixture ASSUMES by calling one
    builder twice. ⇒ If a backend ever stopped using `build_inventory_pages`
    and built its own model, this test stays green. **Reported, not fixed: the
    menu backends are not this row's subject.**

⛔⛔ **AND THE WIDER FAMILY CANNOT BE CENSUSED THIS WAY — MEASURED, NOT
ASSUMED.** The emptiness heuristic finds a member only when the compared sides
are plausibly-empty collections; a parity test comparing two scalars from one
call has the identical defect and is invisible here. **Two hits is a FLOOR on
this species, not a census of it.**

An attempt at the real question — *"tests whose two compared sides trace to one
call site"* — was made 2026-09-10 and abandoned with its numbers, because the
screen cannot answer it and a candidate list nobody triages becomes an amnesty
list:

  * both sides assigned the IDENTICAL call text: **27 of 7918** bodies;
  * requiring both bindings IMMUTABLE and never taken by `&mut`, which removes
    "two arms built from one constructor and then mutated apart": **17**;
  * of those, the ones read by hand were all legitimate — `playback(&app)` twice
    with the app advanced between, `find_portal(&apertures, ..)` twice with the
    apertures reordered between, `p.resolve_solid_hit(block)` twice where the
    call mutates `p`.

⇒ **The discriminator is not "the same call text" but "nothing between the two
calls changes what the call reads", which is a dataflow question and not a
textual one.** A regex screen can produce candidates for this family; it cannot
produce a verdict, and the one confirmed member below was found through the
emptiness door rather than this one.

    python3 scripts/measure_floorless_equality_tests.py
    python3 scripts/measure_floorless_equality_tests.py --positive-control
"""

from __future__ import annotations

import argparse
import collections
import pathlib
import re
import subprocess

REPO = pathlib.Path(__file__).resolve().parents[1]

TEST = re.compile(r"#\[test\]\s*(?:#\[[^\]]*\]\s*)*fn\s+(\w+)\s*\([^)]*\)\s*\{")
FLOOR = re.compile(
    r"\.is_empty\(\)|\.len\(\)|\.count\(\)|\.any\(|\.iter\(\)\.filter|> *0\b|>= *1\b|assert!\("
)
EQ = re.compile(r"assert_eq!\(\s*([A-Za-z_][\w.]*)\s*,\s*([A-Za-z_][\w.]*)\s*[,)]")
# Constructors and wrappers, which repeat for reasons that are not "two runs".
NOT_A_RUN = {"Some", "Ok", "Err", "vec", "String", "format", "Box", "Vec"}

# ⛔ The guard for THIS screen. `the_same_seed_produces_the_same_fighter` was the
# case that caused it; the screen must still flag the version that had the
# defect, or it has been narrowed until it recognises nothing.
CONTROL_REF = "359c8be69"
CONTROL_PATH = "crates/ambition_combat/src/brain/fighter/decision/tests.rs"
CONTROL_TEST = "the_same_seed_produces_the_same_fighter"


def body(src: str, start: int) -> str:
    """The text between the balanced braces beginning at `start`."""
    depth = 0
    for i in range(start, len(src)):
        if src[i] == "{":
            depth += 1
        elif src[i] == "}":
            depth -= 1
            if depth == 0:
                return src[start:i]
    return src[start:]


def flagged(text: str) -> list[tuple[str, list[str]]]:
    """Every `#[test]` in `text` with the shape, and the bindings that gave it."""
    out = []
    for m in TEST.finditer(text):
        b = body(text, m.end() - 1)
        calls = collections.Counter(
            re.findall(r"let\s+(?:mut\s+)?\w+\s*(?::[^=]+)?=\s*(\w+)\s*\(", b)
        )
        if not {k for k, n in calls.items() if n >= 2 and k not in NOT_A_RUN}:
            continue
        eqs = EQ.findall(b)
        if not eqs or FLOOR.search(b):
            continue
        emptiable = []
        for lhs, rhs in eqs:
            for name in (lhs.split(".")[0], rhs.split(".")[0]):
                d = re.search(
                    r"let\s+(?:mut\s+)?" + re.escape(name)
                    + r"\s*(?::\s*(?P<ty>[^=]+?))?=\s*(?P<rhs>[^;]+);",
                    b,
                    re.S,
                )
                if not d:
                    continue
                ty, val = (d.group("ty") or ""), d.group("rhs")
                if ("Vec<" in ty or "HashMap" in ty or "collect()" in val
                        or "vec![" in val or ".to_vec()" in val):
                    emptiable.append(name)
        if emptiable:
            out.append((m.group(1), sorted(set(emptiable))))
    return out


def positive_control() -> int:
    """The screen must flag the case that caused it, at the sha that had it."""
    src = subprocess.run(
        ["git", "show", f"{CONTROL_REF}:{CONTROL_PATH}"],
        cwd=REPO, capture_output=True, text=True, check=True,
    ).stdout
    names = [name for name, _ in flagged(src)]
    if CONTROL_TEST in names:
        print(f"✔ positive control: the screen flags {CONTROL_TEST} at {CONTROL_REF}")
        return 0
    print(f"⛔ POSITIVE CONTROL FAILED: {CONTROL_TEST} is not flagged at "
          f"{CONTROL_REF}. The screen has been narrowed until it recognises "
          f"nothing; its clean result is worthless.")
    return 1


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--positive-control", action="store_true")
    args = ap.parse_args()
    if args.positive_control:
        return positive_control()

    if positive_control():
        return 1

    files = subprocess.run(
        ["git", "grep", "-l", r"#\[test\]", "--", "*.rs"],
        cwd=REPO, capture_output=True, text=True, check=True,
    ).stdout.split()
    # ⛔ AN EMPTY CORPUS PRINTS A CLEAN BILL OF HEALTH. The first run of this
    # screen scanned 0 files because the pathspec preceded the pattern, and it
    # reported no findings without saying so.
    if len(files) < 500:
        print(f"⛔ only {len(files)} files matched; the corpus is wrong")
        return 1

    scanned, hits = 0, []
    for f in files:
        text = (REPO / f).read_text(encoding="utf-8")
        scanned += len(TEST.findall(text))
        for name, bindings in flagged(text):
            hits.append((f, name, bindings))

    print(f"{scanned} #[test] bodies in {len(files)} files")
    print(f"{len(hits)} compare two runs of one call with no emptiness floor\n")
    for f, name, bindings in hits:
        print(f"  {f}::{name}")
        print(f"      compared: {', '.join(bindings)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
