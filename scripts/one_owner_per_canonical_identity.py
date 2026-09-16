#!/usr/bin/env python3
"""One production mint per CONSTANT canonical identity.

⛔⛤ **THE DEFECT THIS EXISTS FOR, MEASURED 2026-09-16.** The gameplay session
root was minted as `SimId::singleton("session", "root")` in TWO production
places: `ambition_game_shell::session::spawn_world_for` and A10's candidate road
in `ambition_platformer2d_provider`. Nothing called the first one. The
value-level arm that closed ID-PEER's session-root provenance hole called the
one nothing calls, so re-keying the LIVE mint on the session scope counter left
the whole app suite green at 705 passed / 0 failed.

⇒ A canonical identity minted in two places is two authorities for one fact
whatever both currently spell, and the duplicate is where the guard goes to the
wrong road. This censuses the CONSTANT mints -- every `SimId::<ctor>(..)` call
whose arguments are all literals, so the call names one specific entity rather
than a family -- and reports any that more than one production site makes.

⚠ **A VARIABLE ARGUMENT IS NOT A DUPLICATE.** `SimId::placement(id)` is called
from many roads by design: the argument is what distinguishes the entity, so two
call sites mint two different identities. Only a literal argument tuple names
the SAME identity from two places, which is why the population is literals.

⚠ **WHAT IT CANNOT SEE, STATED RATHER THAN FIXED.** It matches the spelling
`SimId::`, so a mint reached through an alias (`use ...::SimId as Identity`) or
built by a wrapper function is invisible; measured 2026-09-16, the workspace has
no such alias, and this paragraph is the reason to re-measure rather than assume.
It also cannot tell a MINT from a REFERENCE — see `ADJUDICATED`.

⚠ **AND A LEGITIMATE DUPLICATE MUST STATE ITS ARGUMENT.** `ADJUDICATED` below
holds the pairs that are deliberately shared, each with the reason. A10's
candidate root deliberately carries the live root's identity while hidden -- but
that is ONE mint reached through one road today, so it is not in the list.
"""

from __future__ import annotations

import collections
import importlib.util
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]


def _load(name: str):
    spec = importlib.util.spec_from_file_location(name, REPO / "scripts" / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


#: ⭐ REUSED, NOT REIMPLEMENTED. `_production_sources` already reads every
#: `crates/` and `game/` source once with comments stripped, inline
#: `#[cfg(test)] mod` blocks removed by brace balance, test-named files skipped
#: and whole files behind an inner `#![cfg(test)]` dropped. Four of those five
#: filters were each added because a fixture had entered a production corpus.
_MUTATORS = _load("check_rollback_mutators_run_in_sim")

#: A `SimId::` constructor call. The argument text is captured raw and classified
#: afterwards, because a nested call (`SimId::child(parent.as_str(), n)`) is not
#: a literal and must not read as one.
#:
#: ⛔⛤ **SCANNED OVER THE WHOLE FILE, NOT LINE BY LINE, AND THE FIRST VERSION
#: WAS LINE BY LINE.** It reported ONE constant mint in the entire workspace and
#: that number is what exposed it: `SimId::singleton("session", "root")` — the
#: identity this whole check was built for — is written across three lines, so no
#: single line ever held the call. Nearly every mint in this repository is
#: wrapped, which made a line scan a census of the few short ones.
#: ⚠ NO `re.S`, DELIBERATELY: `[^()]` already spans newlines, so DOTALL would be
#: decoration that reads like the fix. The fix is `finditer` over the whole file
#: text instead of a loop over its lines, and the arm that holds it poisons the
#: LOOP — poisoning the flag changed nothing and 7 of 7 stayed green.
CALL = re.compile(r"\bSimId::([a-z_]+)\s*\(([^()]*)\)")

#: A Rust string or integer literal, the only two an identity segment is spelled
#: with. A `const` NAME is deliberately NOT a literal here: it is one owner
#: already, and the census is about two sites spelling the same thing twice.
LITERAL = re.compile(r'^(?:"[^"]*"|\d[\d_]*|b?\'[^\']*\')$')

#: Constructors that do not mint an identity and must not be censused.
NOT_A_MINT = {"from_snapshot", "as_str", "clone"}

#: Constant mints two production sites make on purpose, each with its argument.
#: ⛔ A ROW HERE IS A DECISION, NOT A WAIVER: it says two owners is the right
#: shape for this identity. Prefer collapsing to one mint.
#:
#: ⚠ **THE INSTRUMENT CANNOT TELL A MINT FROM A REFERENCE, and the first row
#: here is one of each.** `SimId::player_slot(0)` is MINTED in
#: `sim_identity.rs` (the fallback identity for an unidentified primary-player
#: body) and merely SPELLED in `possession.rs` (the controller a possession claim
#: names). Both read identically in source. The reason two sites is right is that
#: the whole possession feature is slot-0-only by design and says so at its
#: `home_q` parameter — the home avatar is the body slot 0 owns and returns to.
ADJUDICATED: dict[tuple[str, str], str] = {
    (
        "player_slot",
        "0",
    ): "one mint (sim_identity's fallback for an unidentified primary body) and one "
    "reference (the controller a possession claim names). Possession is slot-0-only "
    "by design; see its `home_q` parameter.",
}


def constant_mints(sources=None) -> dict[tuple[str, str], list[str]]:
    """Every production `SimId::<ctor>(literals…)` site, keyed by what it names.

    `sources` is an iterable of `(path, text)` for tests; the default is the
    repository's production corpus.
    """
    found: dict[tuple[str, str], list[str]] = collections.defaultdict(list)
    for path, text in sources if sources is not None else _MUTATORS._production_sources():
        for match in CALL.finditer(text):
            ctor, raw = match.group(1), match.group(2)
            if ctor in NOT_A_MINT:
                continue
            args = [arg.strip() for arg in raw.split(",") if arg.strip()]
            if not args or not all(LITERAL.match(arg) for arg in args):
                continue
            key = (ctor, ", ".join(args))
            line_no = text.count("\n", 0, match.start()) + 1
            try:
                shown = path.relative_to(REPO)
            except ValueError:
                shown = path
            found[key].append(f"{shown}:{line_no}")
    return dict(found)


def main() -> int:
    mints = constant_mints()
    duplicates = {
        key: sites
        for key, sites in sorted(mints.items())
        if len(sites) > 1 and key not in ADJUDICATED
    }
    print(
        f"{len(mints)} constant canonical mint(s) in production, "
        f"{len(ADJUDICATED)} adjudicated duplicate(s)"
    )
    if not mints:
        # ⛔ AN EMPTY CORPUS IS THE ONE RESULT THIS CHECK CANNOT REPORT AS CLEAN.
        # Every filter it inherits removes files, and a scan root that stopped
        # matching prints the same "no duplicates" as a healthy tree.
        print("⛔ NO constant mint found at all — the corpus or the pattern is wrong.")
        return 1
    if not duplicates:
        print("ok: every constant canonical identity has one production mint.")
        return 0
    print(f"\n{len(duplicates)} constant identity(ies) minted in more than one place:")
    for (ctor, args), sites in duplicates.items():
        print(f"\n  SimId::{ctor}({args})")
        for site in sites:
            print(f"    {site}")
    print(
        "\n⇒ TWO MINTS FOR ONE IDENTITY IS TWO AUTHORITIES FOR ONE FACT. Collapse\n"
        "  them, or add the pair to ADJUDICATED with the reason two owners is\n"
        "  right. ⚠ Before either, ask which mint the SHIPPED composition calls:\n"
        "  a value-level arm on the other one certifies nothing."
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
