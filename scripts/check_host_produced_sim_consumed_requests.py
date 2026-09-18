#!/usr/bin/env python3
"""Which INTENTS are raised outside the rewinding schedule and spent inside it?

⛔⛔ **THIS IS `Q136`'s POPULATION, AND IT HAD NEVER BEEN ENUMERATED.** A request
written by the host and consumed by the simulation is lost, and the player's
press does nothing. Two witnesses exist for two resources; what nobody could say
before this script is whether those two were the whole population or the two
somebody happened to look at.

⛤ **AND THE LOSS HAPPENS BY TWO OPPOSITE MECHANISMS, WHICH IS WHY ONE SENTENCE
COULD NEVER DESCRIBE IT.** Both end with the intent gone:

    NOT rollback-registered   The sim consumes it on a speculative frame. The
                              rewind does NOT put it back -- it is not registered
                              -- so the consumption stands and nothing re-produces
                              the request. `CutsceneAdvanceRequest`.
    rollback-registered       The rewind DOES put it back, to the value it held
                              before the host wrote it. The host's write is
                              erased. `NewGameResetRequested`.

⇒ A repair that registers an unregistered request moves it from the first row to
the second. It does not fix anything. The fix has to give the intent a
deterministic replay source -- an agreed tick -- which is what `Q136` is about.

⭐⭐ **IT ATTRIBUTES SCHEDULES, AND ITS SIBLING SAYS THAT IS IMPOSSIBLE.**
`check_sim_consumed_request_writers.py` states: *"a schedule's per-system access
set is `pub(crate)` in Bevy 0.19 and `System::name()` is the debug placeholder in
this build, so neither a name filter nor an access query is available here"* --
and concludes that a human must record each writer's schedule by hand. That is
true of a RUNTIME query and false of a SOURCE one: `add_systems(schedule, ..)`
names both halves in the text, which is exactly how
`check_rollback_mutators_run_in_sim.py` has attributed schedules all along. This
script imports that module's parser rather than growing a third copy.

⚠ **WHAT "CONSUMED" MEANS HERE IS SPENDING, NOT WRITING**, and the distinction is
the whole filter: 37 unregistered resources are written from a host schedule and
read inside the sim, and almost all of them are standing state -- a HUD readout,
a settings value, a camera tuning -- which a rewind may freely leave alone
because the next frame republishes it. Only a value that is TAKEN can be lost.
The spend shapes recognised are `mem::take`, `mem::replace`, `drain`, `clear`,
`take()`, assignment to `default()`, and assignment of a flag to `false`.

⚠ **AND THAT LIST IS A FLOOR.** `pop()`, a nested `Option::take` on a field
reached through a method, and any spend that happens inside a helper the system
calls are all outside it. A type this pattern cannot see is not a type that is
safe -- the same sentence its sibling carries, for the same reason.

⛔ **PRODUCERS ARE FOUND THROUGH `SystemParam` BUNDLES, AND THE FIRST VERSION OF
THIS SCRIPT WAS NOT.** `NewGameResetRequested` -- the defect that prompted it --
is written by the menu as `ResMut<'w, ..>` inside a bundle, so a scan of `pub fn`
signatures reported ZERO host producers for it and the script's own subject
passed. Bundle expansion comes from the same sibling module.

    python3 scripts/check_host_produced_sim_consumed_requests.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import check_rollback_mutators_run_in_sim as sim  # noqa: E402

REPO = Path(__file__).resolve().parent.parent

#: `ResMut<T>` as a parameter, with its BINDING, because the spend patterns below
#: are about what the body does to that binding.
_RESMUT = re.compile(
    r"\b(?:mut\s+)?(\w+)\s*:\s*(?:[\w:]*::)?ResMut\s*<\s*(?:'[a-z_]+\s*,\s*)?([\w:]+)\s*>"
)


def _spends(body: str, bind: str) -> bool:
    """Does this body SPEND `bind`, rather than merely update it?

    ⛔ A HANDLED REQUEST IS EMPTIED; A STANDING VALUE IS OVERWRITTEN. Only the
    first can be lost, because only the first has a state that means "already
    dealt with".
    """
    b = re.escape(bind)
    return any(
        re.search(pattern, body)
        for pattern in (
            rf"(?:std::)?mem::take\(\s*&mut\s*\*?{b}",
            rf"(?:std::)?mem::replace\(\s*&mut\s*\*?{b}",
            rf"{b}\.drain\(",
            rf"{b}\.clear\(\)",
            rf"{b}\.take\(\)",
            rf"{b}\.\w+\.take\(\)",
            rf"\*{b}\s*=\s*[\w:]*\s*::\s*default\(\)",
            rf"{b}\.\w+\s*=\s*false\b",
            rf"\*\*?{b}\s*=\s*false\b",
        )
    )


def schedules_by_system(repo: Path = REPO) -> dict[str, set[str]]:
    """`{system name: {schedule label, ..}}` from every `add_systems` in the tree."""
    found: dict[str, set[str]] = {}
    for _src, text in sim._production_sources(repo):
        for body in sim.add_systems_bodies(text):
            schedule, _, rest = body.partition(",")
            schedule = schedule.strip()
            rest = sim.strip_run_conditions(rest)
            for name in re.findall(r"\b([a-z_][a-z0-9_]*)\b", rest):
                found.setdefault(name, set()).add(schedule)
    return found


#: An assignment statement and what it assigns: `*x = Foo::default();`.
_ASSIGNMENT = re.compile(r"=\s*([^;=]*?);")
#: A call passing exactly one argument, used to reach a same-file helper's body.
_ONE_ARG_CALL = re.compile(r"\b([a-z_][a-z0-9_]*)\s*\(\s*([a-z_][a-z0-9_]*)\s*\)\s*;")


def _inline_one_level(text: str, body: str) -> str:
    """Append the bodies of same-file single-argument helpers this body calls.

    ⛔⛤ **WITHOUT THIS THE INSTRUMENT CALLS A CLEAR A PRODUCTION, five times.**
    `reset_session_scoped_resources_on_activation` holds thirty session-scoped
    resources through a `SystemParam` bundle and its own body writes NONE of
    them -- it does `reset(resources);`, and every write lives in that helper as
    `*field = Type::default()`. Reading the system body alone saw a mutable
    holder with no visible write and, having nothing to demote it with, reported
    it as producing an intent. `BaseGravity`, `CutsceneTriggerQueue`,
    `QuestRegistry` and `SwitchActivationQueue` were all this, and
    `CutsceneAdvanceRequest`'s producer list carried two phantom entries beside
    its real one.

    ⚠ ONE LEVEL, AND THAT IS A REAL LIMIT rather than a conservative choice: a
    clear that is two calls deep still reads as a production. The direction is
    the safe one -- an unresolved helper leaves the holder IN the population,
    where it is loud -- but a reader should not take "no false positives" from
    this. Following the chain properly needs a call graph.
    """
    seen: set[str] = set()
    for call in _ONE_ARG_CALL.finditer(body):
        name = call.group(1)
        if name in seen or name in ("return", "if", "match", "while", "for"):
            continue
        seen.add(name)
        decl = re.search(rf"\bfn\s+{name}\s*\(", text)
        if not decl:
            continue
        brace = text.find("{", decl.end())
        if brace < 0:
            continue
        body += "\n" + sim._braced(text, brace)
    return body


def _only_clears(body: str, ty: str) -> bool:
    """Does every assignment in `body` that names `ty` just write its default?

    ⭐ A WRITE OF `T::default()` IS THE ABSENCE OF AN INTENT, NOT ONE. This guard
    asks which intents are raised on the host side and spent inside the rewind;
    a session-edge reset writing a default is CLEARING the slot, and a cleared
    slot cannot be an intent a rewind loses.

    ⚠ IT RETURNS FALSE WHEN IT SEES NOTHING, which is the load-bearing half. A
    real producer writes through the BINDING (`request.0 = true`) and never
    names the type at all, so "no assignment mentions `T`" must mean "cannot
    demote", not "clears". Only a body that names the type AND only ever
    defaults it is demoted.
    """
    naming = _assignments_naming(body, ty)
    return bool(naming) and not _raises(body, ty)


def _assignments_naming(body: str, ty: str) -> list[str]:
    return [
        rhs.strip()
        for rhs in _ASSIGNMENT.findall(body)
        if re.search(rf"\b{re.escape(ty)}\b", rhs)
    ]


def _raises(body: str, ty: str) -> bool:
    """Does `body` write `ty` a value that is NOT its default?

    ⛔⛤ THIS OUTRANKS `_spends`, AND A TEST FOUND OUT WHY. `_spends` recognises
    an assignment to `default()` as a consumption, so a system that CLEARS a
    slot and then RAISES it -- `*req = Flag::default(); ... *req =
    Flag::raised();` -- was classified as a consumer and removed from the host
    side entirely, taking a real crossing with it. Net of the two writes the
    intent is raised, so the raise decides.

    ⚠ It cannot see a raise written through the BINDING (`req.0 = true`), which
    names no type. That costs nothing here: such a system is not recognised as
    a spend either, so it stays a producer by the other road.
    """
    return any(
        not rhs.endswith("::default()") for rhs in _assignments_naming(body, ty)
    )


def consumers_and_producers(
    repo: Path = REPO,
) -> tuple[dict[str, set[str]], dict[str, set[str]], dict[str, set[str]]]:
    """`({type: spenders}, {type: mutable holder}, {type: non-default writer})`."""
    bundles = sim.system_param_mutables(repo)
    spenders: dict[str, set[str]] = {}
    holders: dict[str, set[str]] = {}
    raisers: dict[str, set[str]] = {}
    for _src, text in sim._production_sources(repo):
        for match in sim._PUB_FN.finditer(text):
            name = match.group(1)
            params = sim._params(text, match.end())
            direct = _RESMUT.findall(params)
            reached = {ty.split("::")[-1] for _bind, ty in direct}
            # ⛔ THE BUNDLE HALF. Without it this script cannot see the writer
            # that prompted it: the menu holds `NewGameResetRequested` as a field.
            for ident in re.findall(r"\b([A-Z][A-Za-z_0-9]*)\b", params):
                reached |= bundles.get(ident, frozenset())
            brace = text.find("{", match.end())
            body = _inline_one_level(text, sim._braced(text, brace)) if brace >= 0 else ""
            for ty in reached:
                # A holder that only ever writes `ty`'s default is clearing the
                # slot, not raising an intent -- see `_only_clears`.
                if not _only_clears(body, ty):
                    holders.setdefault(ty, set()).add(name)
            if not body:
                continue
            for bind, ty in direct:
                short = ty.split("::")[-1]
                if _spends(body, bind):
                    spenders.setdefault(short, set()).add(name)
            for ty in reached:
                if _raises(body, ty):
                    raisers.setdefault(ty, set()).add(name)
    return spenders, holders, raisers


#: type -> the reading, dated. An entry says a human looked at this crossing.
#: ⛔ A crossing here is NOT waived: every one of these is a live defect or a
#: filed one. The table exists so a NEW crossing is loud.
ADJUDICATED: dict[str, str] = {
    "CutsceneAdvanceRequest": (
        "⛔ LIVE DEFECT, Q136. NOT rollback-registered. Produced by "
        "`apply_menu_frame_to_cutscene_request` in `Update`, spent by "
        "`tick_active_cutscene` with `mem::take` in the sim's `Cutscene` phase. A "
        "dismiss pressed on the host side does nothing: the take stands through the "
        "rewind and nothing re-produces the press. Held by "
        "`a_cutscene_dismiss_raised_outside_the_simulation_is_lost` (read 2026-09-18)"
    ),
    "NewGameResetRequested": (
        "⛔ LIVE DEFECT, Q136, AND BY THE OPPOSITE MECHANISM. IS "
        "rollback-registered, so the rewind restores it to `false` and ERASES the "
        "menu's write. Produced from `Update` by seven kaleidoscope systems through a "
        "`SystemParam` bundle, spent by `process_new_game_reset_request` in the sim. "
        "New Game can be pressed successfully at the UI and vanish before the "
        "simulation sees it (read 2026-09-18)"
    ),
    # ✅ `SpawnPlayerCloneRequest` was adjudicated here on 2026-09-18 and is GONE
    # because it was FIXED, and the distinction was checked rather than assumed:
    # `spawn_requested_player_clone` still exists and still spends the flag, so
    # this script has not lost sight of it — the spend now runs in
    # `MechanicalEditSet::Publish` (`PreUpdate`), on the HOST side, so there is no
    # host-produced/sim-consumed crossing left to lose. It was Q136's first
    # landed road; the witness is
    # `a_dev_clone_survives_a_rewind::a_clone_asked_for_outside_the_simulation_is_not_lost_to_a_rewind`,
    # and poisoning it back into `app.sim_schedule()` reddens both of that file's
    # arms while the fixed-tick control stays green.
    "VersusMatch": (
        "⛔ FILED ELSEWHERE, NOT A NEW FINDING: banked as MENU-RESET-MIDSESSION in "
        "`check_rollback_mutators_run_in_sim.py` and blocked on `Q140`. "
        "`track_versus_roster` writes `VersusMatch::opening()` from `Update` and stays "
        "there for a stated reason — the teardown it is chained with must outlive "
        "`GameplaySimulationRoot` — so this is a claim to stop making rather than a "
        "line to move. `settle_versus_round` spends it in the sim (read 2026-09-18)"
    ),
}

#: ⛔ THE FLOOR. Populations this script derives, below which it is measuring
#: something other than this tree. A regex that stops matching reports an empty
#: set, and an empty set satisfies "every crossing is adjudicated" vacuously.
FLOORS = {"spent types": 40, "systems with a schedule": 400}


def crossings(repo: Path = REPO) -> dict[str, tuple[list[str], list[str]]]:
    """`{type: (host producers, sim consumers)}` for every intent crossing."""
    spenders, holders, raisers = consumers_and_producers(repo)
    by_system = schedules_by_system(repo)

    found: dict[str, tuple[list[str], list[str]]] = {}
    for ty, spent_by in spenders.items():
        in_sim = sorted(
            fn
            for fn in spent_by
            if by_system.get(fn) and any(not sim.is_non_rewinding(s) for s in by_system[fn])
        )
        if not in_sim:
            continue
        # ⛔ A SPENDER THAT ALSO RAISES IS NOT DISQUALIFIED AS A PRODUCER.
        # `*req = T::default(); ... *req = T::raised();` reads as a consumption
        # to `_spends`, and subtracting it here took a real crossing with it.
        # ⚠ The subtraction stays for the ordinary case: a system that only
        # spends is the consumer, not the producer, even when it runs on the
        # host side.
        consuming_only = spent_by - raisers.get(ty, set())
        host = sorted(
            fn
            for fn in holders.get(ty, set()) - consuming_only
            if by_system.get(fn) and all(sim.is_non_rewinding(s) for s in by_system[fn])
        )
        if host:
            found[ty] = (host, in_sim)
    return found


def population_sizes(repo: Path = REPO) -> dict[str, int]:
    spenders, _holders, _raisers = consumers_and_producers(repo)
    return {
        "spent types": len(spenders),
        "systems with a schedule": len(schedules_by_system(repo)),
    }


def main() -> int:
    spenders, _holders, _raisers = consumers_and_producers()
    rollback = sim.rollback_types()

    sizes = population_sizes()
    short = [
        f"`{what}`: {sizes[what]}, below the recorded floor of {floor}. This script is "
        "looking at less than it was built against, so a clean result here means nothing."
        for what, floor in FLOORS.items()
        if sizes[what] < floor
    ]

    found = crossings()

    print(
        f"{len(spenders)} resource type(s) are SPENT somewhere; "
        f"{len(found)} are spent inside the rewinding schedule and produced only outside it.\n"
    )
    for ty in sorted(found):
        host, in_sim = found[ty]
        print(f"`{ty}`  [{'rollback-registered' if ty in rollback else 'NOT registered'}]")
        print(f"    host producers: {', '.join(host)}")
        print(f"    sim consumers : {', '.join(in_sim)}")
        print(f"    {ADJUDICATED.get(ty, '⛔ UNADJUDICATED')}\n")

    failures = list(short)
    new = sorted(set(found) - set(ADJUDICATED))
    if new:
        failures.append(
            f"{len(new)} host->sim intent crossing(s) nobody has read:\n    "
            + "\n    ".join(new)
            + "\n  ⇒ An intent raised outside the rewinding schedule and SPENT inside it is "
            "lost, and the player's press does nothing. Read the producer and the consumer, "
            "decide which of Q136's two mechanisms applies, and add the reading here. "
            "⛔ Registering an unregistered request does not fix it — it moves the loss from "
            "'the take survives the rewind' to 'the rewind erases the write'."
        )
    gone = sorted(set(ADJUDICATED) - set(found))
    if gone:
        failures.append(
            f"{len(gone)} adjudicated crossing(s) no longer exist:\n    "
            + "\n    ".join(gone)
            + "\n  ⇒ Either it was repaired — delete the entry in the same change — or this "
            "script can no longer see it, which is the more likely and the worse of the two."
        )

    if failures:
        print("⛔ FAILED\n")
        for row in failures:
            print(f"  {row}\n")
        return 1
    print("ok: every host-produced, sim-consumed intent is adjudicated")
    return 0


if __name__ == "__main__":
    sys.exit(main())
