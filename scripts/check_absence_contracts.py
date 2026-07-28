#!/usr/bin/env python3
"""Check the architectural absences this repo depends on — claims that nothing exists.

``scripts/check_roadmap_evidence.py`` verifies that a claim's CITATIONS still
exist. It cannot verify the other kind of claim, the kind whose whole content is
that a thing does *not* exist:

* "`register_character` no longer demands art" (queue A1),
* "the String-keyed sheet-row lookup is deleted" (binding-resolution boundary),
* "the rollback exit oracle is not quarantined" (queue A6),
* "the fight test drives the real damage path" (queue A2).

An absence has no citation to rot, so nothing re-reads it. The queue found this
the expensive way: a row said `with_moveset` had NO production caller, C4 gave it
two, and the row went on saying it for as long as it took somebody to notice
(queue W1). Being right when written is not a property a document keeps.

**The mechanism is a predicate, not a cleverer parser.** Do not teach the
evidence checker to read "used to" / "no longer" / "not yet" — that is prose
interpretation, and ``check_roadmap_evidence.py``'s own docstring explains why it
refuses to go there. An absence that MATTERS belongs in the table below, where it
reddens the day somebody reintroduces the thing.

⚠ **Why this is not a bare `git grep`, which is what the queue first proposed.**
Three times a goal-guard check grepped for the absence of an identifier and three
times it went red on PROSE — the phrase appeared in a doc comment *explaining the
removal*. Documenting a removal must never break the guard that verified it. So
every contract here:

* searches **production source only** — the paths are explicit, and this file,
  the planning tree and the test-support scaffolding are outside them;
* **strips comments before matching**, which is the fix for the recurrence above:
  ``//``, ``///``, ``//!``, ``#`` and block-comment bodies are not code, and a
  paragraph explaining why a symbol is gone is the opposite of evidence that it
  is back;
* uses an **exact symbol or a narrowly scoped pattern**. A broad negative grep
  generates noise, and a noisy guard gets waived, which is worse than no guard;
* carries an **id and a reason**, so a red line says what architectural property
  broke rather than just which regex matched.

A contract is meant to be DELETED or INVERTED when the architecture deliberately
changes. That is not a failure of the guard — a red line here is a conversation
about whether the absence is still wanted, and answering "no, we want the thing
now" is a legitimate answer that ends with this row removed in the same commit.

## The second table: dependency edges

``DEPENDENCY_CONTRACTS`` guards the half a grep CANNOT express. "Crate A must not
depend on crate B" is a fact about the manifest graph, not about any line of
text: a grep can find the ``use`` that proves an edge exists and miss the one
added through a re-export tomorrow, and it cannot see an edge introduced through
an intermediary at all. So that table is checked against ``cargo metadata``, and
checked TRANSITIVELY — the claim is that a foundation cannot REACH gameplay, and
a layering inversion almost never arrives as a direct dependency line.

Both tables are RED-PROBED in ``scripts/tests/test_absence_contracts.py``. Every
contract here is green against the live tree, which is the whole point of it and
also the reason running it proves nothing about whether it works.

Usage:
    python3 scripts/check_absence_contracts.py            # report every contract
    python3 scripts/check_absence_contracts.py --check    # exit 1 on a violation
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

# Every entry is one architectural absence. `paths` are git pathspecs limited to
# production source; `patterns` are Python regexes matched against comment-
# stripped lines. Keep patterns narrow — see the module docstring.
ABSENCE_CONTRACTS: list[dict] = [
    {
        "id": "registration-does-not-demand-art",
        "paths": ["crates/ambition_actors/src/character_runtime/definition.rs"],
        "patterns": [r"CharacterLoadDemand::request", r"\bdemand\.request\("],
        "reason": (
            "Registering a character DECLARES it; it does not ask for its art. "
            "Demanding at registration defeats the room/match/worn projection "
            "model — every registered character would decode whether or not "
            "anything staged it. Staging is the only demand source (queue A1). "
            "`CharacterLoadDemand` is legitimately named in this file's prose, "
            "which is why this matches code only."
        ),
    },
    {
        "id": "no-string-keyed-sheet-row-lookup",
        "paths": ["crates/", "game/", "fixtures/", "tools/"],
        "patterns": [r"\.row_index_of\(", r"\bfn row_index_of\b"],
        "reason": (
            "A sheet row addressed by an arbitrary &str returned None when the "
            "sprite called the animation `death` and the policy asked for "
            "`dead`, and NOTHING DREW — an absence rendered as an absence. The "
            "lookup was deleted in favour of resolved bindings, and the story "
            "survives only as prose in two `binding.rs` module docs "
            "(binding-resolution boundary, 2026-07-25/26)."
        ),
    },
    {
        "id": "rollback-exit-oracle-is-not-quarantined",
        "paths": ["game/ambition_app/tests/rollback_exit_oracle.rs"],
        "patterns": [
            # An `#[ignore]` here is either opt-in TOOLING (a bisection you run
            # when the oracle is red) or a DISABLED GUARD. Only the second is
            # the absence this contract is about, and the reason string is what
            # tells them apart — so the contract requires one, which also makes
            # "why is this off?" answerable without reading the test body.
            {
                "grep": r"#\[ignore",
                "match": r"#\[ignore(?!\s*=\s*\"(?:diagnostic|audit|measurement)\b)",
            },
            "known GGRS divergence",
        ],
        "reason": (
            "This oracle guards a KNOWN determinism failure, so an ordinary "
            "green run has to include it. It was `#[ignore]`d and red for a "
            "long time; the file now explains that history in prose, and the "
            "prose must not be mistaken for the attribute coming back "
            "(queue A6). Release blocker for rollback multiplayer. A new "
            "`#[ignore]` is allowed only as `diagnostic`/`audit`/`measurement` "
            "tooling — anything else is a guard being switched off, which is "
            "queue E1's rule applied where it was learned."
        ),
    },
    {
        "id": "fight-tests-do-not-hand-roll-damage",
        "paths": ["crates/", "game/", "fixtures/"],
        "patterns": [r"\b\w*_hp\s*-=", r"\bhp\s*-=\s*\d"],
        "reason": (
            "The two-provider fight test used to compute an AABB overlap by "
            "hand and subtract integers from local variables: no entities, no "
            "`MovePlayback`, no hit events, no `BodyHealth`. It proved the test "
            "could do arithmetic. Damage is asserted through the production "
            "path or not at all (queue A2)."
        ),
    },
]

# Files whose content is ABOUT the contracts rather than governed by them.
SELF_REFERENTIAL = {"scripts/check_absence_contracts.py"}

# Dependency-edge contracts, read from `cargo metadata` rather than from text.
#
# A grep cannot express "crate A must not depend on crate B". It can find the
# `use` that proves it, and miss the one added through a re-export tomorrow; it
# cannot see a dependency introduced through an intermediary at all. The manifest
# graph is the fact, so this asks the manifest (GPT 5.6's W1 note, 2026-07-28:
# "use Cargo metadata for dependency edges").
#
# `forbidden` is checked TRANSITIVELY. The claim being guarded is never "no
# direct dependency line" — it is that a foundation crate cannot REACH gameplay,
# and reaching it through one intermediary is the same architectural failure with
# an extra hop. A layering inversion almost never arrives as a direct edge.
DEPENDENCY_CONTRACTS: list[dict] = [
    {
        "id": "engine-core-is-the-floor",
        "crate": "ambition_engine_core",
        "forbidden": "*",
        "reason": (
            "The geometry, movement and body vocabulary every other crate is "
            "written in terms of. It depends on NO workspace crate, and that is "
            "what makes it the layer everything else can agree on rather than "
            "one more participant in a cycle. A single edge out of here makes "
            "the whole graph a suggestion."
        ),
    },
    {
        "id": "platformer-primitives-stays-a-foundation",
        "crate": "ambition_platformer_primitives",
        "forbidden": [
            "ambition_actors",
            "ambition_characters",
            "ambition_combat",
            "ambition_runtime",
            "ambition_content",
            "ambition",
        ],
        "reason": (
            "Session scope, binding resolution and stable ids — the vocabulary "
            "gameplay crates USE. It sits directly above `engine_core` and "
            "below everything that has opinions about actors, so a dependency "
            "on one of those inverts the layering the refactor timeline is "
            "built on (foundations < gameplay_core < content)."
        ),
    },
    {
        "id": "characters-do-not-depend-on-the-actor-integration-layer",
        "crate": "ambition_characters",
        "forbidden": ["ambition_actors", "ambition_runtime", "ambition"],
        "reason": (
            "`ambition_actors` depends on `ambition_characters`, which makes "
            "the reverse edge a cycle waiting to be discovered by the compiler "
            "at the worst moment. It also matters for the deferred "
            "`ambition_actors` decomposition: if a coherent actor kernel exists "
            "at all, `ambition_characters` is below it."
        ),
    },
    {
        "id": "engine-crates-do-not-consume-the-umbrella-facade",
        "crate": "ambition_actors",
        "forbidden": ["ambition"],
        "reason": (
            "`ambition` is the facade a CONSUMER builds a game against; it "
            "re-exports `ambition_actors` among thirty-odd others. An engine "
            "crate reaching back through it is circular by construction, and it "
            "is how a headless consumer ends up compiling the render stack. "
            "⚠ deliberately scoped to engine crates: `ambition_content` DOES "
            "depend on the facade today, and whether that should stop is a "
            "MEASUREMENT question the campaign defers rather than a rule."
        ),
    },
]

_LINE_COMMENT = re.compile(r"//.*$")
_HASH_COMMENT = re.compile(r"#(?!\[).*$")
_BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.DOTALL)


def strip_comments_for(path: str, line: str) -> str:
    """Return `line` with comment text removed, so prose cannot match a pattern.

    Deliberately line-local and deliberately crude. A multi-line `/* */` body
    survives only if it contains no `//`, and the one shape that matters — a
    `///` or `//!` paragraph naming the thing that was removed — is removed
    exactly. Being conservative in the other direction (treating code as
    comment) would HIDE a real violation, so nothing here strips code.

    `#` is a comment in shell and Python and not in Rust, where it opens an
    attribute — and `#[ignore]` is precisely what one contract looks for. So the
    hash rule is applied by file type rather than universally.
    """
    stripped = _BLOCK_COMMENT.sub(" ", line)
    stripped = _LINE_COMMENT.sub("", stripped)
    if not path.endswith((".rs", ".toml")):
        stripped = _HASH_COMMENT.sub("", stripped)
    return stripped


def git_grep(pattern: str, paths: list[str], root: Path) -> list[tuple[str, int, str]]:
    """Every `path, line number, text` match for `pattern` under `paths`."""
    command = ["git", "grep", "-n", "-I", "-E", pattern, "--", *paths]
    result = subprocess.run(
        command, cwd=root, capture_output=True, text=True, check=False
    )
    # git grep exits 1 for "no matches", which is the outcome this file wants.
    if result.returncode not in (0, 1):
        raise RuntimeError(f"git grep failed: {result.stderr.strip()}")
    hits = []
    for raw in result.stdout.splitlines():
        parts = raw.split(":", 2)
        if len(parts) != 3:
            continue
        path, number, text = parts
        try:
            hits.append((path, int(number), text))
        except ValueError:
            continue
    return hits


def violations(contract: dict, root: Path) -> list[tuple[str, int, str]]:
    """Every production line that violates `contract`, comments excluded.

    A pattern is either a string (git grep and the confirming match are the same
    expression) or a `{"grep": ..., "match": ...}` pair. The pair exists because
    `git grep -E` is POSIX ERE and has no lookaround: the coarse expression finds
    candidate lines cheaply and the precise Python one decides. Splitting them is
    what lets a contract say "an ignore WITHOUT a diagnostic reason" instead of
    settling for "an ignore", which is the difference between a contract that
    survives and one that gets waived.
    """
    found: list[tuple[str, int, str]] = []
    for pattern in contract["patterns"]:
        if isinstance(pattern, str):
            grep, confirm = pattern, pattern
        else:
            grep, confirm = pattern["grep"], pattern["match"]
        compiled = re.compile(confirm)
        for path, number, text in git_grep(grep, contract["paths"], root):
            if path in SELF_REFERENTIAL:
                continue
            if compiled.search(strip_comments_for(path, text)):
                found.append((path, number, text.strip()))
    found.sort()
    return found


def cargo_binary() -> str:
    """`cargo`, found even when PATH does not have it.

    This runs from the goal-guard hook, and the hook's PATH has no cargo — a
    lesson this repo paid for twice, because a check that can only ever report
    "command not found" can never pass and wedges the run it was supposed to
    guard. So the rustup location is tried before giving up on PATH.
    """
    rustup = Path.home() / ".cargo" / "bin" / "cargo"
    return str(rustup) if rustup.exists() else "cargo"


def workspace_graph(root: Path) -> dict[str, set[str]]:
    """Every workspace crate's DIRECT workspace dependencies, from the manifests.

    `--no-deps` keeps this to the workspace: registry crates are somebody else's
    layering problem. Dev-dependencies are included deliberately — a test that
    reaches upward compiles the upward edge, and "only in tests" is exactly the
    excuse under which a layering inversion first arrives.
    """
    raw = subprocess.run(
        [cargo_binary(), "metadata", "--no-deps", "--format-version", "1"],
        cwd=root,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    metadata = json.loads(raw)
    members = {package["name"] for package in metadata["packages"]}
    return {
        package["name"]: {
            dependency["name"]
            for dependency in package["dependencies"]
            if dependency["name"] in members
        }
        for package in metadata["packages"]
    }


def reachable(graph: dict[str, set[str]], start: str) -> dict[str, list[str]]:
    """Every crate `start` can reach, mapped to the path that gets there.

    The path is the whole value of reporting this: "A depends on B" is arguable,
    "A -> C -> B" is a fix.
    """
    found: dict[str, list[str]] = {}
    queue = [(start, [start])]
    while queue:
        crate, path = queue.pop(0)
        for dependency in sorted(graph.get(crate, ())):
            if dependency in found or dependency == start:
                continue
            found[dependency] = path + [dependency]
            queue.append((dependency, path + [dependency]))
    return found


def dependency_violations(contract: dict, graph: dict[str, set[str]]) -> list[str]:
    crate = contract["crate"]
    if crate not in graph:
        return [f"contract names `{crate}`, which is not a workspace member"]
    reached = reachable(graph, crate)
    forbidden = contract["forbidden"]
    targets = sorted(reached) if forbidden == "*" else forbidden
    return [
        " -> ".join(reached[target]) for target in targets if target in reached
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check", action="store_true", help="exit 1 when a contract is violated"
    )
    args = parser.parse_args()

    root = Path(
        subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip()
    )

    broken = 0
    for contract in ABSENCE_CONTRACTS:
        found = violations(contract, root)
        if not found:
            print(f"  ok   {contract['id']}")
            continue
        broken += 1
        print(f"  RED  {contract['id']}")
        print(f"       {contract['reason']}")
        for path, number, text in found:
            print(f"       {path}:{number}: {text}")

    graph = workspace_graph(root)
    for contract in DEPENDENCY_CONTRACTS:
        found = dependency_violations(contract, graph)
        if not found:
            print(f"  ok   {contract['id']}")
            continue
        broken += 1
        print(f"  RED  {contract['id']}")
        print(f"       {contract['reason']}")
        for path in found:
            print(f"       {path}")

    total = len(ABSENCE_CONTRACTS) + len(DEPENDENCY_CONTRACTS)
    if broken:
        print(f"\n{broken} of {total} absence contracts are violated.")
        print(
            "Either the reintroduction is a mistake, or the architecture changed "
            "on purpose — in which case DELETE or INVERT the contract in the same "
            "commit rather than waiving it."
        )
        return 1 if args.check else 0
    print(f"\n{total} of {total} absence contracts hold.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
