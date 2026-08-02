#!/usr/bin/env python3
"""A crate that was renamed must not still be named anywhere that is LIVE.

Written 2026-08-01, immediately after the `platformer2d` rename, because that
rename hit the same defect three separate times and each occurrence was found by
a test failure whose message was about something else entirely.

## Why a substring search, and not a smarter one

The rename itself used a word-boundary rule — `\\bambition_portal\\b` cannot eat
`ambition_portal_presentation`, which is exactly right for rewriting Rust paths.
It is exactly WRONG for verifying the result, because the character before a
crate name is not always what it looks like:

    "relation\\tambition.limb\\tambition_actors\\tlimb-rig\\t"
    re.compile(r"\\bambition::([a-z_][a-z0-9_]*)")

In both, the character preceding the name is a letter — the `t` of `\\t`, the `b`
of `\\b` — so a boundary-anchored sweep skips them, AND SO DOES THE GREP THAT
CHECKS THE SWEEP. Three real cases survived that way: two construction-registry
assertions comparing owner strings as DATA, and the SDK-docs guard's module
pattern, which failed with "these modules are a compatibility PROMISE and the SDK
never mentions them".

So this check is deliberately the dumbest possible search. A plain substring has
no blind spot to share with the tool that did the renaming.

## Why an explicit list, and not "any name that is not a workspace member"

That was tried first and is unusable: 128 distinct `ambition_*` tokens in this
tree name Python packages, shell functions, JSON keys, LDtk layers, asset
manifests and identifiers. A ratchet with an explicit list has no false
positives, and the maintenance it asks for is one line at the moment somebody
renames a crate — which is the moment they are already editing everything else.

## Scope

Historical records are EXEMPT and must stay exempt. A linker transcript that
says `libambition_actors.so`, a review from July, a `# was:` line recording a
policy id's former name — those are records of what happened, and rewriting them
would make the record wrong.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]

# name-that-is-gone -> what it is now. Add a row the moment a crate is renamed.
RETIRED_CRATE_NAMES = {
    "ambition_engine_core": "ambition_platformer2d_core",
    "ambition_platformer_primitives": "ambition_platformer2d_shared_tangle",
    "ambition_world": "ambition_platformer2d_world",
    "ambition_ldtk_map": "ambition_platformer2d_ldtk",
    "ambition_portal_presentation": "ambition_portal2d_presentation",
    "ambition_platformer_provider": "ambition_platformer2d_provider",
    "ambition_runtime": "ambition_platformer2d_runtime",
    "ambition_host": "ambition_platformer2d_host",
    "ambition_actors": "ambition_platformer2d_actor_monolith",
    # `ambition_portal` is deliberately ABSENT: it is a prefix of the live
    # `ambition_portal2d`, so a substring search for it matches every correct
    # use. The prefix relation is what makes it safe to omit — a stale
    # `ambition_portal` on its own cannot survive `cargo check`, because unlike
    # an owner string in a test fixture it is always a real crate path.
    "ambition_engine": "ambition_platformer2d_core",
    "ambition_pulse": "examples/capability_demo",
    # ── retired TYPE names ──
    #
    # ⚠ the same rule, one level down, and it earns its place: `SandboxAction`
    # was renamed with the crates and left THIRTEEN live mentions in
    # `queue-72h-2026-07-31.md`, all describing current architecture. The row
    # that mattered told a reader to migrate `InputMap<SandboxAction>` — a grep
    # for which returns nothing, so the honest conclusion from that row was "this
    # is already done". A stale crate name breaks a build; a stale TYPE name in a
    # planning doc quietly retires a piece of work.
    "SandboxAction": "Platformer2dInputActionMonolith",
    # Found 2026-08-02 by asking a different question of the same file: which
    # type-like names in the LIVE queues exist nowhere in `*.rs`? 27 did, and
    # most were fine — a queue legitimately names things to BUILD and things
    # DELETED. ⭐ the filter that separates them is whether the mention sits under
    # an OPEN row: only there does a dead name send somebody after work.
    #
    # These two failed that filter. They sat in a sentence asserting what the
    # code does TODAY — "every teardown system keys on `SandboxResetCommitted`"
    # — and both had been renamed out from under it.
    "SandboxResetCommitted": "NewGameResetCommitted",
    "process_sandbox_reset_request": "process_new_game_reset_request",
}

# Records of what happened. Never rewritten.
HISTORICAL_PREFIXES = (
    "docs/archive/",
    "docs/brainstorms/",
    "docs/reviews/",
    "dev/journals/",
    "dev/benchmark-candidates/",
    ".agent/",
    ".llm_resource_tally/",
    "docs/planning/engine_rename_campaign.md",
    # ⛔ the QUEUE files were exempt here until 2026-08-02, and the exemption was
    # backwards. They are not a record of the past — they are the LIVE WORKLIST
    # this repository is driven from, so a dead name in one does not misdescribe
    # history, it hands the next reader a task they cannot find.
    #
    # It cost exactly that: `SandboxAction` (renamed with the crates) survived
    # THIRTEEN times in `queue-72h-2026-07-31.md`, and the row that mattered told
    # a reader to migrate `InputMap<SandboxAction>` — a grep for which returns
    # nothing, so the honest conclusion from that row was "already done".
    #
    # ⚠ and the exemption was buying almost nothing: lifting it flagged TWO
    # lines in the whole tree, both genuinely historical, both fixed by saying so
    # on the line. A blanket waiver whose real population is two is a waiver
    # nobody priced.
)

# ⚠ the guard's own test file is skipped because its FIXTURES are retired names —
# that is what it tests. It went unnoticed at first because `git ls-files` did not
# yet list the file: the live-tree ratchet passed while its own counter-example was
# untracked, which is the "green at minute zero" trap one level in.
SKIP_NAMES = (
    "Cargo.lock",
    "check_retired_crate_names.py",
    "test_retired_crate_names.py",
)

# Whole files whose CONTENT is a quotation of old source. Listed individually,
# with the reason, rather than pattern-matched.
WAIVED_FILES = {
    "tools/optimization_report/apply_headless_hygiene_patch.py":
        "a frozen one-off migration; its payload is a literal snippet of the "
        "source it used to patch, so the retired name IS the data",
}

# A line may keep a retired name when it is explicitly recording history. These
# are the phrasings real historical prose in this tree already uses; the rule
# they encode is "if you are describing what USED to be, say so on the line."
# ⚠ deliberately NOT a generic waiver comment: a marker somebody has to remember
# to add is a marker nobody adds, whereas these words are already there when the
# sentence is genuinely about the past.
HISTORICAL_MARKERS = (
    "# was:",
    "was: ",
    "libambition_",
    "former",
    "formerly",
    "no longer",
    "was collapsed",
    "was renamed",
    "renamed to",
    "is gone",
    "deletion of",
    "was deleted",
    "collapsed long ago",
    "named here before",
    "when this was authored",
    "then called",
    "Earlier notes",
    # ⚠ added 2026-08-02 with the queue exemption removal below. A sentence that
    # says a name is RETIRED is the clearest possible statement that it is
    # describing the past, and this tree already writes it that way.
    "retired",
)


# ⚠ UNTRACKED files the guard must still read.
#
# `.goal/active.json` holds the goal harness's own check COMMANDS, and it is not
# in git. When the platformer2d rename retired `ambition_actors` and `ambition`,
# two of those commands kept naming them — so `cargo check -p ambition` and
# `cargo test -p ambition_actors` failed on an unknown package, and the harness
# reported "S1 slice H is not done" and "S2 match activation is not done" about
# work that was finished and green. A broken instrument reading as unfinished
# WORK is the most expensive failure mode this repository has, and `git ls-files`
# is exactly why nothing caught it.
#
# `done-*.json` are archived goals — records, skipped like any other history.
def extra_paths() -> list[str]:
    goal = REPO / ".goal"
    if not goal.is_dir():
        return []
    return [
        f".goal/{p.name}"
        for p in sorted(goal.glob("*.json"))
        if not p.name.startswith("done-")
    ]


def tracked_files() -> list[str]:
    raw = subprocess.run(
        ["git", "ls-files", "-z"], cwd=REPO, capture_output=True, text=True, check=True
    ).stdout
    return [f for f in raw.split("\0") if f] + extra_paths()


def offences(files: list[str]) -> list[tuple[str, int, str, str]]:
    found: list[tuple[str, int, str, str]] = []
    for f in files:
        if f in WAIVED_FILES:
            continue
        if f.startswith(HISTORICAL_PREFIXES) or Path(f).name in SKIP_NAMES:
            continue
        path = REPO / f
        if not path.is_file():
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        if not any(old in text for old in RETIRED_CRATE_NAMES):
            continue
        for number, line in enumerate(text.splitlines(), 1):
            for old, new in retired_names_in_line(line):
                found.append((f, number, old, new))
    return found


def retired_names_in_line(line: str) -> list[tuple[str, str]]:
    """Every retired crate name this line still uses as a NAME.

    Split out from the file walk so it can be tested directly — the escape cases
    this exists for are one-liners, and a test that had to build a file tree to
    reach them would not get written.
    """
    if any(marker in line for marker in HISTORICAL_MARKERS):
        return []
    hits: list[tuple[str, str]] = []
    for old, new in RETIRED_CRATE_NAMES.items():
        # Plain substring, then a TRAILING boundary only. The trailing side is
        # safe to test — an escape sequence sits BEFORE a name
        # (`\tambition_actors`), never after — and it is what keeps
        # `ambition_world_entity`, a local variable meaning Ambition's world,
        # from reading as the retired `ambition_world` crate.
        start = 0
        while (at := line.find(old, start)) != -1:
            start = at + 1
            tail = line[at + len(old):at + len(old) + 1]
            if tail and (tail.isalnum() or tail == "_"):
                continue
            hits.append((old, new))
            break
    return hits


def main() -> int:
    found = offences(tracked_files())
    if found:
        print("Retired crate names are still live:\n")
        for f, number, old, new in found:
            print(f"  {f}:{number}: `{old}` was renamed to `{new}`")
        print(
            "\nIf the line is a RECORD of something that happened (a linker error,\n"
            "a dated review, a `# was:` note), move it under a historical path or\n"
            "mark the line, rather than rewriting history."
        )
        return 1
    print(f"No retired crate name is live ({len(RETIRED_CRATE_NAMES)} tracked).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
