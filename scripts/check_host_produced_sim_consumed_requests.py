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

import functools
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import check_rollback_mutators_run_in_sim as sim  # noqa: E402
import measure_user_settings_in_simulation as settings  # noqa: E402

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


#: An inherent `&mut self` method, so a raise spelled as a CALL can be resolved.
_IMPL_BLOCK = re.compile(r"\bimpl(?:\s*<[^>]*>)?\s+([A-Z][A-Za-z_0-9]*)\b[^{]*\{")
_MUT_METHOD = re.compile(r"\bfn\s+([a-z_]\w*)\s*\(\s*&mut\s+self")


@functools.cache
def _mut_methods(repo: Path = REPO) -> tuple[tuple[str, str, str], ...]:
    """`(type, method, body)` for every inherent `&mut self` method in the tree.

    ⛔⛤ **THE SIXTH SPELLING OF A PRODUCTION IS A METHOD CALL, AND IT HID THE
    ONLY REAL `NewGameResetRequested` PRODUCER.** This script's raise test read
    ASSIGNMENTS. The menu does not assign: `kaleidoscope_app.rs:761` says
    `self.reset.request()`, and `NewGameResetRequested::request` is four lines
    that set `self.request = true`. So the type looked unproduced, while seven
    systems that merely HELD the bundle looked like producers — a false negative
    and seven false positives from the same gap.
    """
    out: list[tuple[str, str, str]] = []
    for _src, text in sim._production_sources(repo):
        for match in _IMPL_BLOCK.finditer(text):
            block = sim._braced(text, match.end() - 1)
            for method in _MUT_METHOD.finditer(block):
                brace = block.find("{", method.end())
                if brace < 0:
                    continue
                out.append((match.group(1), method.group(1), sim._braced(block, brace)))
    return tuple(out)


def _method_bodies(repo: Path = REPO) -> dict[tuple[str, str], list[str]]:
    found: dict[tuple[str, str], list[str]] = {}
    for ty, method, body in _mut_methods(repo):
        found.setdefault((ty, method), []).append(body)
    return found


def _inline_methods_on(body: str, bind: str, ty: str, methods) -> str:
    """Append the bodies of `&mut self` methods this body calls on `bind`.

    `self.` is rewritten to `bind.` so the appended text speaks the caller's
    access paths and the ordinary path-keyed tests apply to it unchanged.
    """
    extra: list[str] = []
    for call in re.findall(rf"{re.escape(bind)}\s*\.\s*([a-z_]\w*)\s*\(", body):
        for inner in methods.get((ty, call), ()):
            extra.append(inner.replace("self.", f"{bind}."))
    return body + "\n" + "\n".join(extra) if extra else body


def _raises_via_method(body: str, path: str, ty: str, methods) -> bool:
    """Is a non-default written by a `&mut self` method called on `path`?"""
    for call in re.findall(rf"{re.escape(path)}\s*\.\s*([a-z_]\w*)\s*\(", body):
        for inner in methods.get((ty, call), ()):
            if _raises_through(inner, "self"):
                return True
    return False


#: A parameter declaration, so a bundle's BINDING is known and not just its type.
_PARAM_DECL = re.compile(r"(?:mut\s+)?([a-z_]\w*)\s*:\s*(?:[\w:]*::)?([A-Z][A-Za-z_0-9]*)")


def _touches(body: str, path: str) -> bool:
    """Does `body` mention this access path at all?"""
    return bool(re.search(rf"{re.escape(path)}\b", body))


def _raises_through(body: str, path: str) -> bool:
    """Is a NON-default value written through this access path?

    ⚠ [`_raises`] is keyed on the TYPE NAME, so it can only see a write that
    spells the type (`*x = Foo::default()`). A producer writing through a bundle
    field says `p.req.0 = true` and names nothing, so the type-keyed test
    returns False and the field would never be credited as raising.
    """
    for rhs in re.findall(rf"{re.escape(path)}[\w\.\[\]0-9]*\s*=(?!=)([^;]*);", body):
        stripped = rhs.strip()
        if stripped and not stripped.endswith("::default()") and stripped != "false":
            return True
    return False


def unlocated_message_systems(repo: Path = REPO) -> list[str]:
    """Message writers/readers this module cannot place in any schedule.

    ⛔⛤ **IT MAKES THE CROSSING SET A LOWER BOUND, AND THE BOUND WAS 24% BEFORE
    THE WRAPPER ROAD LANDED.** MEASURED 2026-09-18: of 320 distinct message
    writers and readers, **77** appeared in no registration
    [`schedules_by_system`] could find — 24 of the 90 written message types
    exposed. Following registration wrappers [`_wrapper_registered`] took that
    to **47 and 18**, a 39% cut, and left the four crossings unchanged: the
    adjudication below did not move, the confidence in its completeness did.

    ⭐⛤ **AND A THIRD REGISTRATION SHAPE LANDED 2026-09-19, WITH THE SAME
    SIGNATURE: THE BOUND MOVED AND THE VERDICT DID NOT.** The bound had drifted
    back to **61 and 20** as the tree grew. Following a system tuple bound to a
    LOCAL before registration ([`_expand_local_tuples`]) takes it to **42 and
    15** — 13 such locals, recovering 19 systems — and the adjudication is
    byte-identical: the same three resources and the same three messages.

    ⚠ THAT THE VERDICT DOES NOT MOVE IS THE EXPECTED RESULT AND NOT A
    DISAPPOINTMENT. This number is a claim about how much of the tree the
    instrument can SEE, and the crossings it already found were never in
    dispute. A road that moved the verdict would mean the previous answer had
    been wrong, not that this one is better.

    ⛔⛔ **AND THE DIAGNOSIS THIS DOCSTRING CARRIED FOR A DAY WAS WRONG, WHICH
    IS WHY THE FIX LOOKED LIKE A CAMPAIGN.** It said `add_systems_bodies`
    truncates the `app.add_systems(sim, ..)` at
    `crates/ambition_platformer2d_runtime/src/combat_schedule.rs:640` to 272
    characters, stopping short of `apply_feature_hit_events` at `:695`, and
    therefore that widening a parser shared by several censuses was owed.
    MEASURED 2026-09-18, all three claims fail:

      * that body is 452 characters and closes correctly at `:648`;
      * across the tree, **0 of 639** `add_systems` bodies get longer when
        every comment is blanked first — no body in this workspace is truncated
        by a prose paren today;
      * and at `ac27d1718~1`, `ac27d1718` and HEAD alike, no `add_systems` body
        in that file has EVER contained `apply_feature_hit_events`.

    ⇒ **THE REAL MECHANISM IS A REGISTRATION WRAPPER, and it was already solved
    in another script.** `:695` is not inside an `add_systems` call at all; it is
    an argument to `install_technique(app, KEY, offer, (..systems..))`
    (`combat_schedule.rs:650`), whose own body is `app.add_systems(sim,
    systems)` (`:78`). A scan for the literal call answers *"not scheduled"*
    about a system that ships. `measure_user_settings_in_simulation.py` found
    this first, wrote the mechanism down, and built the fixpoint that follows it
    — `find_sim_forwarders` is a fixpoint precisely because the chain is two
    deep, the singular delegating to the plural. Two owners of one fact, and the
    blind one had the confident wrong reason. [`schedules_by_system`] now calls
    the owner's helpers; nothing was copied and no shared parser moved.

    ⚠ **47 REMAIN, AND THEY ARE A DIFFERENT SHAPE** — registration this module
    still cannot follow: a schedule assembled from a table, a `cfg`-gated
    install, a plugin whose systems are named in a `const`. Naming the next
    mechanism needs the same treatment this one got: a specimen, measured, not a
    guess about the parser. `main()` prints the remainder every run, because the
    number that matters to Q136 is the one that is still unread.
    """
    by_system = schedules_by_system(repo)
    missing: set[str] = set()
    for _ty, writers, readers in _message_sides(repo):
        for fn in list(writers) + list(readers):
            if fn not in by_system:
                missing.add(fn)
    return sorted(missing)


#: `let name = (` — a system tuple bound to a local before registration.
_LOCAL_TUPLE = r"\blet\s+{name}\s*=\s*\("


def _expand_local_tuples(text: str, rest: str) -> str:
    """Replace `app.add_systems(sim, rules)` with the contents of `rules`.

    ⛔⛤ **A THIRD REGISTRATION SHAPE, FOUND 2026-09-19, AND IT HID 19 SYSTEMS.**
    This module already followed a direct `add_systems` and a wrapper that
    forwards its systems parameter. The shape it did not follow is a system
    TUPLE bound to a local first:

        let after_the_star = (empowerment::apply_contact_harm, star::play_star_music)
            .chain()
            .in_set(..);
        app.add_systems(sim, after_the_star);

    The registration body is then the single word `after_the_star`, which
    matches no function, so every system inside the tuple read as "in no
    registration this module can find". MEASURED: 13 such locals, recovering
    **19 of the 61** systems the lower-bound warning was reporting.

    ⚠ IT IS A TEXTUAL, SAME-FILE EXPANSION and deliberately shallow: a local
    whose tuple names another local is not followed. The bound this feeds is a
    LOWER bound, so under-expanding leaves the warning conservative, which is
    the safe direction.
    """
    name = rest.strip()
    if not re.fullmatch(r"[a-z_][a-z0-9_]*", name):
        return rest
    match = re.search(_LOCAL_TUPLE.format(name=re.escape(name)), text)
    if match is None:
        return rest
    depth, index = 0, text.index("(", match.start())
    end = index
    while end < len(text):
        if text[end] == "(":
            depth += 1
        elif text[end] == ")":
            depth -= 1
            if depth == 0:
                break
        end += 1
    return text[index : end + 1]


def schedules_by_system(repo: Path = REPO) -> dict[str, set[str]]:
    """`{system name: {schedule label, ..}}` from every registration in the tree.

    Two roads, because registration takes two shapes: the literal
    `add_systems(schedule, ..)` call, and a wrapper that forwards its systems
    parameter into one [`_wrapper_registered`].

    ⚠ STILL INCOMPLETE, and by how much is measured in
    [`unlocated_message_systems`] rather than guessed at. A missing entry reads
    as *"no schedule"*, which every caller treats as *"cannot classify"* rather
    than as *"host"* or *"sim"* — the safe direction for a verdict and the
    unsafe one for a POPULATION.
    """
    found: dict[str, set[str]] = {}
    for _src, text in sim._production_sources(repo):
        bound_to_sim = sim.sim_schedule_bindings(text)
        for body in sim.add_systems_bodies(settings.without_comments(text)):
            schedule, _, rest = body.partition(",")
            schedule = schedule.strip()
            # A local bound to `app.sim_schedule()` IS the sim schedule, whatever
            # it was named. Only `sim` and `pre_collect_sim` exist today, and the
            # second is why this resolution is here — see
            # `sim.sim_schedule_bindings`.
            if schedule in bound_to_sim:
                schedule = "sim"
            rest = sim.strip_run_conditions(rest)
            rest = _expand_local_tuples(settings.without_comments(text), rest)
            for name in re.findall(r"\b([a-z_][a-z0-9_]*)\b", rest):
                found.setdefault(name, set()).add(schedule)
    # A wrapper registers into `app.sim_schedule()`, which every caller here
    # reads through `sim.is_non_rewinding` — and a schedule VARIABLE is
    # rewinding by that predicate, which is the answer this label needs.
    for name in sim.wrapper_registered_systems(repo):
        found.setdefault(name, set()).add("sim")
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
    bundles = sim.system_param_mutable_fields(repo)
    methods = _method_bodies(repo)
    spenders: dict[str, set[str]] = {}
    holders: dict[str, set[str]] = {}
    raisers: dict[str, set[str]] = {}
    for _src, text in sim._production_sources(repo):
        for match in sim._PUB_FN.finditer(text):
            name = match.group(1)
            params = sim._params(text, match.end())
            brace = text.find("{", match.end())
            body = _inline_one_level(text, sim._braced(text, brace)) if brace >= 0 else ""

            # ⛔⛤ **EVERY ACCESS IS A (PATH, TYPE) PAIR, AND REDUCING THE BUNDLE
            # HALF TO A SET OF TYPES PRODUCED ONE ERROR OF EACH SIGN.** A
            # request spent through `p.req` was invisible, because spend
            # detection only ever looked at direct `ResMut` parameters; and a
            # system that merely HELD a bundle was counted as a producer of
            # every type in it, which is where the claim of "seven kaleidoscope
            # systems" producing `NewGameResetRequested` came from. Both were
            # reproduced from a review on 2026-09-18 and are held by
            # `scripts/tests/test_host_produced_sim_consumed_requests.py`.
            accesses: list[tuple[str, str]] = [
                (bind, ty.split("::")[-1]) for bind, ty in _RESMUT.findall(params)
            ]
            direct_paths = {path for path, _ in accesses}
            for bind, bundle in _PARAM_DECL.findall(params):
                if bundle not in bundles:
                    continue
                # A bundle METHOD is where the menu's raise actually lives, so
                # its body joins the caller's before any path test runs.
                body = _inline_methods_on(body, bind, bundle, methods)
                for field, ty in bundles[bundle].items():
                    accesses.append((f"{bind}.{field}", ty))

            for path, ty in accesses:
                # A bundle FIELD counts only when the body actually touches it.
                # A direct `ResMut` parameter is a declared exclusive hold and
                # counts either way -- it is in the signature on purpose.
                if path not in direct_paths and not _touches(body, path):
                    continue
                # A holder that only ever writes `ty`'s default is clearing the
                # slot, not raising an intent -- see `_only_clears`.
                if not _only_clears(body, ty):
                    holders.setdefault(ty, set()).add(name)
                if not body:
                    continue
                if _spends(body, path):
                    spenders.setdefault(ty, set()).add(name)
                if (
                    _raises(body, ty)
                    or _raises_through(body, path)
                    or _raises_via_method(body, path, ty, methods)
                ):
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
        "menu's write. Produced from `Update` by TWO registered systems — "
        "`grid_menu_action_activated` and `kaleidoscope_menu_action_activated` — "
        "and spent by `process_new_game_reset_request` in the sim. ⚠ This said "
        "\"seven kaleidoscope systems through a `SystemParam` bundle\" until "
        "2026-09-18, which was an artefact of this script reducing a bundle to "
        "the set of types it holds: most of those systems merely POSSESS "
        "`SystemMenuParams`. See `PRODUCER_BY_INSPECTION` for the four-hop road "
        "no textual scan reaches. "

        "New Game can be pressed successfully at the UI and vanish before the "
        "simulation sees it (read 2026-09-18)"
    ),
    # ✅ `SpawnPlayerCloneRequest` IS GONE FROM THIS TABLE FOR THE SECOND REASON,
    # and the sentence here used to give the first. It was adjudicated on
    # 2026-09-18 as FIXED — the spend moved to `MechanicalEditSet::Publish`
    # (`PreUpdate`), leaving no host-produced/sim-consumed crossing — and that
    # note ended *"`spawn_requested_player_clone` still exists and still spends
    # the flag, so this script has not lost sight of it"*, which was true when
    # written and false by the end of the same day: `89d78a4a5` deleted the whole
    # clone road, its trigger having collided with two shipped input presets.
    # ⇒ The distinction the old note drew is the right one to keep — a type
    # leaving this table because it was repaired and a type leaving because the
    # scan went blind look identical from here — but a prose claim that a name
    # "still exists" rots in a day. `check_rollback_mutators_run_in_sim.py` now
    # holds the machine-checked version for its own waiver table.
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
FLOORS = {
    "spent types": 40,
    "systems with a schedule": 400,
    "message types written": 70,
}


#: type → (host producers, why no textual scan can reach the raise).
#:
#: ⛔⛤ **AN INSTRUMENT THAT CANNOT SEE SOMETHING MUST BE TOLD, VISIBLY.** This
#: table exists for exactly one shape: a raise so many hops from the system
#: signature that resolving it textually would be a guess dressed as a
#: measurement. An entry is a HAND ATTRIBUTION and reads like one; it is not a
#: waiver, because the type still appears in the output as a crossing.
#:
#: ⚠ The alternative was worse and was tried: with the bundle half reduced to
#: field paths, `NewGameResetRequested` silently left the population entirely,
#: and only this script's own "an adjudicated crossing no longer exists" guard
#: caught it. A census whose completeness depends on nobody improving its
#: precision is not a census.
PRODUCER_BY_INSPECTION: dict[str, tuple[tuple[str, ...], str]] = {
    "NewGameResetRequested": (
        ("grid_menu_action_activated", "kaleidoscope_menu_action_activated"),
        "FOUR HOPS FROM THE SIGNATURE, AND THE COUNT WAS WRONG UNTIL MEASURED. "
        "This entry said \"seven kaleidoscope systems through a `SystemParam` "
        "bundle\" until 2026-09-18; a review pointed out that most of those "
        "systems merely POSSESS `SystemMenuParams` and produce nothing, which "
        "was an artefact of reducing a bundle to the set of types it holds. "
        "Read at source, the road is: the two registered `Update` systems above "
        "→ `dispatch_menu_action` (a multi-argument free function, so no "
        "single-argument helper inlining reaches it) → "
        "`SystemMenuParams::request_reset` "
        "(`game/ambition_app/src/menu/kaleidoscope_app.rs:760-762`) → "
        "`NewGameResetRequested::request` "
        "(`crates/ambition_platformer2d_actor_monolith/src/session/reset/mod.rs:266-268`), "
        "which is the only production `self.request = true` in the tree. "
        "Nothing in the system signature or body names the type.",
    ),
}


#: A `MessageWriter`/`MessageReader` parameter and the message it carries.
_MSG_WRITER = re.compile(
    r"(?:mut\s+)?[a-z_]\w*\s*:\s*(?:[\w:]*::)?MessageWriter\s*<\s*(?:'[a-z_]+\s*,\s*)?([\w:]+)"
)
_MSG_READER = re.compile(
    r"(?:mut\s+)?[a-z_]\w*\s*:\s*(?:[\w:]*::)?MessageReader\s*<\s*(?:'[a-z_]+\s*,\s*)?([\w:]+)"
)

#: ⛔⛤ **`NarrativeInputWriter<M>` IS A WRITER SPELLING THIS CENSUS DOES NOT
#: RECOGNISE, AND ADDING IT WOULD MANUFACTURE FALSE CROSSINGS.** They are not
#: crossings, because the writer does not write a message: it records the
#: payload into `NarrativeInputLedger<M>` stamped with `SimTick + 1`, and
#: `release_narrative_inputs` raises the real message at the head of the SIM
#: schedule, so a resimulated tick re-raises it. The producer this census
#: should see IS the sim-side release, and it does.
#:
#: ⇒ Recorded here because the shape reads exactly like a gap. A future reader
#: who "closes" it by treating `NarrativeInputWriter` as a `MessageWriter` gets
#: a host producer for every one of them and a finding that is the architecture
#: working. See `Q136`'s fourth escape.
#:
#: ⚠ **THE POPULATION IS TEN, AND THIS COMMENT SAID FIVE UNTIL 2026-09-18.**
#: `ChallengeRequested`, `BrainCommand`, `ReleaseProvocation`,
#: `ItemGrantRequested`, `ShopTransactionRequested`, `ConversationEnded`,
#: `RunAuthoredCommand`, `SpawnActorRequest`, `CutRopeRoomReplayRequested`,
#: `SetFlagRequested`. The five named were the actor-feature payloads; a
#: hand-written list of an open set is a count nobody re-derives, which is why
#: the population now has an owner that does —
#: `scripts/check_narrative_writers_have_a_ledger.py`.
#:
#: ⛔⛤ **AND THE EXEMPTION HAD AN UNCHECKED PREMISE, WHICH COST A SHIPPED
#: DEFECT.** "The ledger is the safe ingress" is true of the MECHANISM and says
#: nothing about whether the mechanism was INSTALLED. `SetFlagRequested` had a
#: writer in shipped content and no `NarrativeInputPlugin<SetFlagRequested>`
#: anywhere, so its `ResMut<NarrativeInputLedger<_>>` could not resolve and the
#: authored Yarn command recorded nothing — while this census looked away from
#: it on purpose. Found by review, not by this file. The pairing guard above is
#: the premise, checked; do not widen this exemption without it.

#: The same two, as FIELDS of a `#[derive(SystemParam)]` bundle.
#:
#: ⛔⛤ **A `MessageReader` ONE LEVEL DOWN WAS INVISIBLE TO BOTH SIDES OF THIS
#: CENSUS UNTIL 2026-09-18.** The two patterns above are matched against a
#: system's own parameter list, so a bundle holding the cursor hid the reading
#: entirely: `FreshAttempt` (`crates/ambition_combat/src/events.rs:193`) carries
#: two, and `void_pending_player_hits_at_lifecycle_boundaries`
#: (`crates/ambition_damage/src/lib.rs:1249`) takes it. Found by review with a
#: production specimen, not a poison — the twin census learned the same lesson
#: the same day, which is why the bundle walk now has one owner
#: (`sim.bundle_fields_matching`) instead of three.
#:
#: ⚠ AND THE UNDERCOUNT RAN THE UNSAFE WAY. A missing READER makes a real
#: host→sim crossing look like a write nobody consumes, which is the shape this
#: script drops rather than reports.
_BUNDLE_WRITER_FIELD = sim.bundle_field_pattern("MessageWriter")
_BUNDLE_READER_FIELD = sim.bundle_field_pattern("MessageReader")

#: A bundle named as a system parameter, by type, path-qualified or not.
_BUNDLE_PARAM = r":\s*(?:[A-Za-z_][A-Za-z_0-9]*::)*{}\b"


@functools.cache
def _bundle_message_fields(
    repo: Path,
) -> tuple[tuple[str, tuple[str, ...], tuple[str, ...]], ...]:
    """`(bundle, written types, read types)` for each bundle carrying either."""
    written = dict(sim.bundle_fields_matching(_BUNDLE_WRITER_FIELD, repo))
    read = dict(sim.bundle_fields_matching(_BUNDLE_READER_FIELD, repo))
    out = []
    for bundle in sorted(set(written) | set(read)):
        ws = tuple(sorted({ty for _field, ty in written.get(bundle, ())}))
        rs = tuple(sorted({ty for _field, ty in read.get(bundle, ())}))
        if ws or rs:
            out.append((bundle, ws, rs))
    return tuple(out)


#: `add_message::<T>()` — the type universe a `.write_message(..)` argument is
#: resolved against. 103 registrations at 2026-09-18.
_MSG_REGISTERED = re.compile(r"\badd_message\s*::\s*<\s*([A-Za-z_][\w:<>\s]*?)\s*>")

#: `.write_message(EXPR` — the SECOND spelling of a message production.
#:
#: ⛔⛤ **THE FIRST VERSION OF THIS PASS SAW ONLY `MessageWriter<T>` PARAMETERS,
#: AND A PRODUCTION WRITE IS ALSO A `&mut World` CALL.** Found 2026-09-18 by
#: joining this census against the `Local`-memory one: `NewGameResetCommitted`
#: has four sim-schedule readers and read as NEVER WRITTEN, because its only
#: production write is `world.write_message(NewGameResetCommitted)`
#: (`session/reset/mod.rs:502`). Measured over the production corpus: 54 such
#: calls, 34 naming a type path, and exactly **3** registered types written ONLY
#: this way — `NewGameResetCommitted`, `RespawnRoomVisualsRequested`,
#: `RoomLoaded`.
#:
#: ⭐ **NONE OF THE THREE IS A CROSSING, AND THAT IS WHY THIS HAD TO BE FIXED
#: RATHER THAN NOTED.** `NewGameResetCommitted`'s writer
#: (`process_new_game_reset_request`) is itself sim-side, so the resimulation
#: re-raises it; the other two have no host writer with a sim reader. The answer
#: did not move — and a population that undercounts without changing the answer
#: is exactly the one that changes silently later.
_WRITE_MESSAGE = re.compile(
    r"\.\s*write_message\s*\(\s*([A-Za-z_][\w]*(?:\s*::\s*[A-Za-z_][\w]*)*)"
)


@functools.cache
def _registered_messages(repo: Path) -> frozenset[str]:
    found: set[str] = set()
    for _src, text in sim._production_sources(repo):
        for match in _MSG_REGISTERED.finditer(text):
            found.add(match.group(1).split("::")[-1].strip())
    return frozenset(found)


#: `let NAME = …;` / `for NAME in …`, used to give a bare argument a type.
_BINDS = (
    re.compile(r"\blet\s+(?:mut\s+)?{name}\s*(?::\s*([^=;]+?))?\s*=\s*([^;]+);"),
    re.compile(r"\bfor\s+{name}\s+in\s+([^\{{]+)\{{"),
    # A FUNCTION PARAMETER, which is how `stage_actor(&mut self, request:
    # SpawnActorRequest)` carries the harness's only bare write.
    re.compile(r"\b{name}\s*:\s*&?\s*(?:mut\s+)?([A-Za-z_][\w:]*)\s*[,)]"),
)
#: `fn NAME( … ) -> T`, to read a call's element type out of its signature.
_RETURNS = re.compile(r"\bfn\s+{name}\s*(?:<[^>()]*>)?\s*\([^;]*?\)\s*->\s*([^\{{;]+)")


def _resolve_binding(body: str, name: str, universe: frozenset[str], repo: Path) -> str | None:
    """The registered message type a BARE `write_message(x)` argument carries.

    ⛔⛤ **A LOCAL BINDING HID A CROSSING FROM THIS SCRIPT, AND A REVIEW PROVED IT
    BY POISON.** `let event = Heal; world.write_message(event);` took a real
    host→sim crossing out of [`message_crossings`] and into
    [`unresolved_message_writes`], and the guard stayed GREEN — an argument this
    parser cannot read was silently not part of the population. The shape is not
    hypothetical: `crates/ambition_game_shell/src/plugin.rs:349,369` writes
    `ShellEvent` through `for event in …`.

    Four spellings are resolved: a `let` with an explicit type, a `let` whose
    right-hand side names a type, a `for` over a call whose `fn … -> Vec<T>`
    signature names one, and a function PARAMETER. Anything else stays
    unresolved and must be adjudicated — see [`UNRESOLVED_ADJUDICATED`].
    """
    for pattern in _BINDS:
        match = re.search(pattern.pattern.format(name=re.escape(name)), body)
        if not match:
            continue
        for group in match.groups():
            if not group:
                continue
            for word in re.findall(r"[A-Za-z_]\w*", group):
                if word in universe:
                    return word
            # `for x in router.advance_pending(..)` — read the callee's return.
            call = re.search(r"\.\s*([a-z_]\w*)\s*\(", group)
            if call:
                signature = re.compile(
                    _RETURNS.pattern.format(name=re.escape(call.group(1)))
                )
                for _src, other in sim._production_sources(repo):
                    found = signature.search(other)
                    if not found:
                        continue
                    for word in re.findall(r"[A-Za-z_]\w*", found.group(1)):
                        if word in universe:
                            return word
    return None


def _written_by_call(
    body: str, universe: frozenset[str], repo: Path | None = None
) -> set[str]:
    """Registered message types this body writes through `.write_message(..)`.

    ⚠ The argument is resolved by taking the first path segment that names a
    REGISTERED message, not the last: `ShellCommand::GoTo(..)` writes a
    `ShellCommand`, and reading the last segment invents a type called `GoTo`.
    An argument that resolves to nothing — a local, or `AppExit::from_code(..)` —
    is counted as unresolved and `unresolved_message_writes` reports it, because
    a silently dropped write is how this gap opened in the first place.
    """
    found: set[str] = set()
    for match in _WRITE_MESSAGE.finditer(body):
        parts = [p.strip() for p in match.group(1).split("::")]
        for part in parts:
            if part in universe:
                found.add(part)
                break
        else:
            if len(parts) == 1 and repo is not None:
                resolved = _resolve_binding(body, parts[0], universe, repo)
                if resolved:
                    found.add(resolved)
    return found


def unresolved_head(argument: str) -> str:
    """The type name an unattributable argument is spelled with.

    `bevy::app::AppExit::from_code` and `AppExit::Success` are one type and two
    spellings; `UNRESOLVED_ADJUDICATED` is keyed on the type so a row is an
    argument about a TYPE rather than about a punctuation style.
    """
    parts = [p.strip() for p in argument.split("::")]
    for part in parts:
        if part[:1].isupper():
            return part
    return parts[-1]


def unresolved_message_writes(repo: Path = REPO) -> list[tuple[str, str]]:
    """`(file, argument)` for every `.write_message(..)` naming no registered type."""
    universe = _registered_messages(repo)
    out: list[tuple[str, str]] = []
    for src, text in sim._production_sources(repo):
        for match in _WRITE_MESSAGE.finditer(text):
            parts = [p.strip() for p in match.group(1).split("::")]
            if any(p in universe for p in parts):
                continue
            if len(parts) == 1 and _resolve_binding(text, parts[0], universe, repo):
                continue
            out.append((src.relative_to(repo).as_posix(), match.group(1)))
    return out


#: The head of a `.write_message(..)` argument this script cannot attribute to a
#: registered message type → why that is correct rather than a hole.
#:
#: ⛔⛤ **AN ENTRY IS REQUIRED, AND THAT IS THE POINT.** Until 2026-09-18 an
#: unattributable write was merely COUNTED: a review poisoned the tree with
#: `let event = Heal; world.write_message(event);`, watched the crossing move
#: out of `message_crossings` and into `unresolved_message_writes`, and watched
#: this script print `ok:` anyway. A population that quietly drops what it
#: cannot read is not a population. `main()` now fails on any head not named
#: here, so the next unreadable spelling has to be resolved or argued for.
UNRESOLVED_ADJUDICATED: dict[str, str] = {
    "AppExit": (
        "⭐ OUT OF THE POPULATION BY DEFINITION, not tolerated. `AppExit` is "
        "Bevy's own message and this workspace never calls `add_message::"
        "<AppExit>()` — `_registered_messages` is built from that call, so the "
        "type cannot be in `universe` no matter how the argument is spelled. "
        "All 18 sites are `AppExit::Success` / `::from_code` / `::error` in "
        "capture tools, the render-recovery host and the menu's quit path: a "
        "process exiting is not an intent the simulation can consume, so there "
        "is no rewind for it to be lost to (read 2026-09-18)"
    ),
}


#: `(message type, system)` pairs whose only evidence is that the system takes a
#: `#[derive(SystemParam)]` bundle holding a `MessageWriter` for it. Filled by
#: [`_message_sides`], read only when a row is PRINTED: the detection above uses
#: the real name, because possession is a sound upper bound on who can write —
#: but a printed producer list is read as an assertion, and this one is not.
_HELD_WRITERS: set[tuple[str, str]] = set()


def held_writer(ty: str, system: str) -> bool:
    """True when `system`'s claim on `ty` is bundle POSSESSION, not a write."""
    return (ty, system) in _HELD_WRITERS


@functools.cache
def _message_sides(repo: Path) -> tuple[tuple[str, tuple[str, ...], tuple[str, ...]], ...]:
    universe = _registered_messages(repo)
    writers: dict[str, set[str]] = {}
    readers: dict[str, set[str]] = {}
    held: set[tuple[str, str]] = set()
    for _src, text in sim._production_sources(repo):
        for match in sim._PUB_FN.finditer(text):
            name = match.group(1)
            params = sim._params(text, match.end())
            for ty in _MSG_WRITER.findall(params):
                writers.setdefault(ty.split("::")[-1], set()).add(name)
            for ty in _MSG_READER.findall(params):
                readers.setdefault(ty.split("::")[-1], set()).add(name)
            for bundle, bundle_writes, bundle_reads in _bundle_message_fields(repo):
                if not re.search(_BUNDLE_PARAM.format(re.escape(bundle)), params):
                    continue
                # ⛔⛤ **POSSESSION IS NOT USE, AND THIS SCRIPT ALREADY PAID FOR
                # CONFLATING THEM ONCE.** Reducing a bundle to the set of types
                # it holds is what produced *"seven kaleidoscope systems raise
                # `NewGameResetRequested`"* — most of them merely TOOK
                # `SystemMenuParams`. The same shape is live here:
                # `MenuDispatchParams` carries a `MessageWriter
                # <PlayerHealRequested>` and `grid_menu_nav`
                # (`game/ambition_app/src/menu/grid_backend.rs:494`) takes the
                # bundle and writes only its own `MenuActionActivated`.
                #
                # ⇒ The TYPE still has to enter the universe, or a message
                # written ONLY through a bundle field has no writer at all and
                # drops out of the population — an undercount in the direction
                # that hides crossings. So the type is admitted and the NAME is
                # labelled, because "this system can write it" and "this system
                # writes it" are different claims and only the first is
                # readable from a parameter list.
                for ty in bundle_writes:
                    writers.setdefault(ty.split("::")[-1], set()).add(name)
                    held.add((ty.split("::")[-1], name))
                # ⚠ THE READ SIDE IS NOT LABELLED, AND THE ASYMMETRY IS THE
                # POINT. An over-named WRITER invents a producer; an unnamed
                # READER hides a crossing. A cursor in a bundle is consumed by
                # whoever holds it or by nobody, and either way the channel is
                # reachable from the rewinding schedule, which is the question.
                for ty in bundle_reads:
                    readers.setdefault(ty.split("::")[-1], set()).add(name)
            brace = text.find("{", match.end())
            if brace != -1:
                for ty in _written_by_call(
                    params + sim._braced(text, brace), universe, repo
                ):
                    writers.setdefault(ty, set()).add(name)
    _HELD_WRITERS.update(held)
    return tuple(
        (ty, tuple(sorted(ws)), tuple(sorted(readers.get(ty, ()))))
        for ty, ws in sorted(writers.items())
    )


def message_crossings(repo: Path = REPO) -> dict[str, tuple[list[str], list[str]]]:
    """`{message: (host writers, sim readers)}` — the OTHER ingress channel.

    ⛔⛤ **THIS SCRIPT REQUIRES THE `Resource` DERIVE AND THEREFORE MISSED AN
    ENTIRE CHANNEL.** The filter is deliberate — the first version reported 67
    rows because `App`, `Commands`, `NextState`, `Anchor` and `Sprite` are not
    resources — but it also excluded every intent raised as a `Message`. Four
    message types are written by a system registered only in a non-rewinding
    schedule and read by one inside the rewinding schedule, which is Q136's
    first mechanism on a second channel.

    ⚠ **AND ONE CANDIDATE LOSS MECHANISM IS A DECLARED ROLLBACK DECISION DOING
    WHAT IT SAYS.** `clear_message_on_rollback` adds `clear_message_channel::<T>`
    to `LoadWorld`, so every rewind EMPTIES the channel. For a message raised
    inside the simulation that is right: the resimulation re-raises it, and
    keeping the old copy would double it. For a host-raised message there is no
    resimulation to re-raise it, so the clear would be the loss.

    ⛔⛤ **THAT WAS WRITTEN AS THE MECHANISM AND IT IS NOT — IT IS THE READER'S
    CURSOR, AND THE CLEAR IS REDUNDANT.** Removing the registration for
    `PlayerHealRequested` on 2026-09-18 changed its witness's outcome not at all,
    and the poison was verified applied: it announced itself four times in the
    test binary. The arithmetic says why it could not have mattered.
    `Messages::clear` empties both buffers and sets each one's
    `start_message_count = self.message_count` (`bevy_ecs` 0.19.1,
    `message/messages.rs:228-232`), so `message_count` is MONOTONIC across a
    clear — never rewound. A cursor's unread count is
    `message_count.saturating_sub(last_message_count).min(len())`
    (`message_cursor.rs:120-129`). ⇒ A reader that consumed the message on the
    speculative frame reads `n - n = 0` on every resimulated frame, whether or
    not the channel still holds it. The cursor refuses first.

    ⭐ **AND THE SAME ARITHMETIC IS THE WHOLE RULE, WHICH IS WHY THIS PASS IS
    ALSO THE ADJUDICATION OF 99 `MessageReader` CURSORS** that
    `check_sim_schedule_memory_is_adjudicated.py` cannot see: a sim-raised
    message is re-raised by the resimulation, bumping the count PAST the cursor,
    so it is read. A host-raised one is not. ⇒

        A `MessageReader` inside the rewinding schedule loses its message
        exactly when nothing inside that schedule re-raises it.
    """
    by_system = schedules_by_system(repo)

    def side(fn: str) -> str | None:
        schedules = by_system.get(fn)
        if not schedules:
            return None
        return "host" if all(sim.is_non_rewinding(s) for s in schedules) else "sim"

    found: dict[str, tuple[list[str], list[str]]] = {}
    for ty, writers, readers in _message_sides(repo):
        host = sorted(fn for fn in writers if side(fn) == "host")
        in_sim = sorted(fn for fn in readers if side(fn) == "sim")
        if host and in_sim:
            found[ty] = (host, in_sim)
    return found


#: message → the reading, dated. Same contract as `ADJUDICATED`: an entry is a
#: reading, never a waiver.
#:
#: ⭐ TWO OF THE FOUR ARE BENIGN, AND THEY ARE THE USEFUL PART — each shows a
#: general escape this question can choose rather than an accident.
MESSAGE_ADJUDICATED: dict[str, str] = {
    "AmbientGravityRequest": (
        "⛔ LIVE, AND THE SAME MECHANISM AS THE CLONE. `cycle_dev_gravity` "
        "(`game/ambition_app/src/menu/kaleidoscope_app.rs:2054`) reads "
        "`keys.just_pressed(KeyCode::Backslash)` and writes once — an "
        "unregistered host EDGE spent in the sim, and the physical press is "
        "several host frames gone by the time a rewind ends. ⚠ THE MECHANISM "
        "SENTENCE THAT USED TO SIT HERE — *\"the rewind clears the channel\"* — "
        "is not supported by the witness below, and the same claim was already "
        "refuted for the heal by poisoning `clear_message_on_rollback`. A flat "
        "zero means the request was never applied even once, which a channel "
        "cleared AFTER a speculative apply cannot produce. A DEVELOPER hotkey, "
        "so the stakes are the clone's rather than a player's. ⭐ WITNESSED by "
        "`an_ambient_gravity_request_raised_outside_the_simulation_is_lost` "
        "(`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`), "
        "the last of the four to get an arm — and the arm CORRECTED the reading. "
        "The loss is the item grant's FLAT ZERO, not the heal's transient: "
        "`BaseGravity` never passes through the cycled direction on any of 200 "
        "frames, on the same ownership mode and rollback settings under which the "
        "heal is visible for two. ⚠ BOTH ITS ARMS RUN ONE COMPOSITION, because "
        "writing the request on every frame into the sandbox fixture the heal arm "
        "uses moves `BaseGravity` not once: `apply_ambient_gravity_requests` is "
        "not reached there, so a flat zero from it would have meant only \"no "
        "reader here\" (read 2026-09-18, witnessed 2026-09-18)"
    ),
    "PlayerHealRequested": (
        "⛔ LIVE, PLAYER-VISIBLE, AND WITNESSED. Raised by "
        "`kaleidoscope_menu_action_activated`, which is also one of the two real "
        "`NewGameResetRequested` producers — the menu has two lost-intent roads. "
        "Held by `a_player_heal_requested_outside_the_simulation_is_lost` "
        "(`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`) "
        "with an in-sim control that heals and holds. ⚠ AND THE LOSS IS NOT A "
        "FLAT ZERO LIKE THE ITEM GRANT'S: on a LocalMaintainer-owned timeline "
        "the heal LANDS, is visible for about two frames, and is then revoked "
        "by GGRS's first correction — the write reaches the live `Events` buffer "
        "before any `LoadWorld`, then the rollback restores `BodyHealth` from a "
        "pre-heal confirmed frame and resimulates with the channel already "
        "cleared. The arm asserts BOTH ends, because a composition that could "
        "not heal at all would print the same final number. "
        "⛤ AND THE MECHANISM IS THE READER'S CURSOR, NOT THE CHANNEL CLEAR. "
        "Removing `clear_message_on_rollback::<PlayerHealRequested>` changed the "
        "outcome NOT AT ALL (poison verified applied — it announced itself four "
        "times in the test binary). `Messages::clear` leaves `message_count` "
        "MONOTONIC (`bevy_ecs` 0.19.1 `message/messages.rs:228-232`) and a "
        "cursor's unread count is `message_count - last_message_count` "
        "(`message_cursor.rs:120-129`), so a reader that consumed the message on "
        "the speculative frame reads zero on every resimulated frame whether or "
        "not the channel was emptied. ⇒ The clear is redundant here, and the same "
        "arithmetic is why a SIM-raised message survives: the resimulation "
        "re-raises it and bumps the count past the cursor "
        "(read 2026-09-18, witnessed 2026-09-18, mechanism read 2026-09-18)"
    ),
    "ResetToCheckpoint": (
        "✅ BENIGN, OUTSIDE THE TIMELINE — BY AN ORDERING THE ROLLBACK LAYER "
        "ENFORCES ON PURPOSE. "
        "`maintain_local_session` returns without starting a session while "
        "`durable_hydration_is_pending` "
        "(`crates/ambition_platformer2d_rollback_ggrs/src/local_session.rs:334-340`), "
        "so `complete_durable_restore` has already run and its message has "
        "already been consumed before any timeline exists. ⚠ THE ARGUMENT IS THE "
        "ORDERING, NOT THE LATCH: `SaveRestored` is rollback-registered, and the "
        "same file records at `:321-326` that it *\"is not a latch that always "
        "rises\"* at a measured cost of 66 tests — so \"the rewind re-arms the "
        "latch and it re-raises\" is reasoning from the wrong fact "
        "(read 2026-09-18)"
    ),
    # ✅ `SetFlagRequested` IS REPAIRED AND ITS ROW IS GONE, 2026-09-18 — the
    # first use of the RE-DERIVED IN THE SIM escape this table named and had no
    # instance of. `emit_intro_flag_chains` moved from `Update` to
    # `app.sim_schedule()`, after `GameplayEffects`, so the derivation runs on
    # the resimulated tick and the message is re-raised past the reader's
    # cursor. The two things that could have blocked it were measured, not
    # assumed: `bevy_ggrs` restores with `ResMut::as_mut`, which re-arms the
    # `resource_exists_and_changed` gate on every rollback, and the derivation
    # skips targets already present, so the extra fire a restore can cause
    # writes nothing. ⇒ THIS SCRIPT IS WHAT NOTICED: the crossing count fell 4
    # -> 3 and the stale-adjudication check named the row on the next run.
}


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
        declared, _why = PRODUCER_BY_INSPECTION.get(ty, ((), ""))
        host = sorted(set(host) | set(declared))
        if host:
            found[ty] = (host, in_sim)
    return found


def population_sizes(repo: Path = REPO) -> dict[str, int]:
    spenders, _holders, _raisers = consumers_and_producers(repo)
    return {
        "spent types": len(spenders),
        "systems with a schedule": len(schedules_by_system(repo)),
        "message types written": len(_message_sides(repo)),
    }


#: The three verdicts a reading may open with. A reading is a reading, so the
#: vocabulary is small on purpose — this is not a taxonomy, it is a check that
#: the sentence answers the question it is filed under.
VERDICTS = ("⛔ LIVE", "⛔ FILED ELSEWHERE", "✅ BENIGN")

#: ⭐⭐ **A `✅ BENIGN` VERDICT MUST NAME WHICH ESCAPE IT TAKES**, because
#: 2026-09-18's measurement turned "why is this one safe?" from prose into a
#: closed list. The loss is the CONSUMPTION record, not the intent: a
#: `MessageReader`'s cursor is a `Local` no rewind restores, so a host→sim
#: handoff is safe only when *"this has already been consumed"* is rollback
#: state or re-derived every frame. There are exactly three ways out:
#:
#:   RE-DERIVED IN THE SIM the condition is recomputed INSIDE the rewinding
#:                         schedule, so the replay re-produces it itself
#:   OUTSIDE THE TIMELINE  the ordering never enters the rewinding schedule, so
#:                         no consumption is ever replayed (`ResetToCheckpoint`)
#:   REGISTERED CONSUMPTION the "already consumed" fact is itself rollback state,
#:                         so a rewind un-consumes it
#:
#: ⛔⛤ **THE FIRST ONE SAID "EVERY HOST FRAME" AND THAT IS NOT AN ESCAPE — A
#: 2026-09-18 REVIEW CAUGHT IT AND `SetFlagRequested` MOVED TO LIVE.** A host
#: system re-deriving the condition on a LATER frame does not re-run the
#: HISTORICAL tick being resimulated: the replay of that tick still has no
#: message, and `apply_flag_effects`' writes land in `AmbitionGameSave` and
#: `QuestRegistry`, both `rollback_resource_clone_checksum`-registered
#: (`ambition_persistence/src/rollback_registration.rs:31`, `:37`). ⇒ The replayed
#: frame diverges on a CHECKSUMMED value, and the later re-emission sets the flag
#: at a different tick. Re-derivation only escapes when it happens where the
#: replay can see it.
#:
#: ⚠ The third escape has no instance in the tree today, and now neither does
#: the first. They are listed because a check whose vocabulary only covers what
#: already exists cannot accept a correct new answer — and the third is
#: precisely the addition `Q136`'s option 2 needs.
BENIGN_ESCAPES = (
    "RE-DERIVED IN THE SIM",
    "OUTSIDE THE TIMELINE",
    "REGISTERED CONSUMPTION",
)


def misshapen_readings() -> list[str]:
    """Readings that do not open with a verdict, or claim benign without an escape."""
    problems: list[str] = []
    for table, label in ((ADJUDICATED, "resource"), (MESSAGE_ADJUDICATED, "message")):
        for name, reading in table.items():
            if not reading.startswith(VERDICTS):
                problems.append(
                    f"`{name}` [{label}] does not open with one of {VERDICTS}. "
                    "A reading has to say which of the three it is before it explains why."
                )
                continue
            if reading.startswith("✅ BENIGN") and not any(
                escape in reading.upper() for escape in BENIGN_ESCAPES
            ):
                problems.append(
                    f"`{name}` [{label}] is filed BENIGN without naming its escape. "
                    f"One of {BENIGN_ESCAPES} must appear — the loss is the CONSUMPTION "
                    "record, and 'the intent still arrives' is not an escape from it."
                )
    return problems



#: The ruling whose population this census defines. `Q136` restates these six
#: numbers in prose, and on 2026-09-19 five of them were stale — by the
#: instrument's OWN repair, which is the worst case: the script got better and
#: the page it justifies kept the number the older script printed.
RULING = REPO / "docs/planning/awaiting-maintainer-decision.md"
RULING_MARKER = re.compile(r"<!--\s*ingress-census:\s*([^>]*?)\s*-->")
MARKER_ENTRY = re.compile(r"(\w+)=(\d+)")


def ruling_marker_drift(**measured: int) -> list[str]:
    """Every number `Q136` restates from this census, compared to this run.

    ⚠ **THE MARKER IS NOT A SECOND MEASUREMENT AND MUST NOT BECOME ONE.** It is
    a transcription of what this script prints, checked here so that the prose
    beside it cannot quietly age. The classification stays owned by the code
    above; nothing here re-derives a verdict.
    """
    text = RULING.read_text(encoding="utf-8")
    hit = RULING_MARKER.search(text)
    if not hit:
        return [
            f"{RULING.relative_to(REPO)} carries no `ingress-census` marker, so the "
            "numbers it states in prose are held by nothing"
        ]
    stated = {name: int(value) for name, value in MARKER_ENTRY.findall(hit.group(1))}
    out = []
    for name, value in sorted(measured.items()):
        if name not in stated:
            out.append(f"the marker omits `{name}`, which this run measures as {value}")
        elif stated[name] != value:
            out.append(f"`{name}`: the ruling states {stated[name]}, this run measures {value}")
    for name in sorted(set(stated) - set(measured)):
        out.append(f"the marker states `{name}`, which this census does not measure")
    return out

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

    messages = message_crossings()
    print(
        f"{sizes['message types written']} message type(s) are written somewhere; "
        f"{len(messages)} are read inside the rewinding schedule and written only "
        "outside it.\n"
    )
    for ty in sorted(messages):
        host, in_sim = messages[ty]
        print(f"`{ty}`  [message]")
        print(
            "    host writers: "
            + ", ".join(f"{h} (holds a writer)" if held_writer(ty, h) else h for h in host)
        )
        print(f"    sim readers : {', '.join(in_sim)}")
        print(f"    {MESSAGE_ADJUDICATED.get(ty, '⛔ UNADJUDICATED')}\n")

    failures = list(short)
    unread_messages = sorted(set(messages) - set(MESSAGE_ADJUDICATED))
    if unread_messages:
        failures.append(
            f"{len(unread_messages)} host->sim MESSAGE crossing(s) nobody has read:\n    "
            + "\n    ".join(unread_messages)
            + "\n  ⇒ Same question as the resource rows, on the other channel: "
            "`clear_message_on_rollback` empties the channel in `LoadWorld`, and a "
            "host-raised message has no resimulation to re-raise it. Read both sides and "
            "say which it is. ⭐ Two of the four already here are BENIGN — a condition "
            "re-derived every frame, and an ordering that keeps the write outside the "
            "timeline — so look for those before assuming a defect."
        )
    stale_messages = sorted(set(MESSAGE_ADJUDICATED) - set(messages))
    if stale_messages:
        failures.append(
            f"{len(stale_messages)} adjudicated MESSAGE crossing(s) no longer exist:\n    "
            + "\n    ".join(stale_messages)
            + "\n  ⇒ Repaired, or this script can no longer see it."
        )
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

    failures.extend(misshapen_readings())

    if failures:
        print("⛔ FAILED\n")
        for row in failures:
            print(f"  {row}\n")
        return 1
    print("ok: every host-produced, sim-consumed intent is adjudicated")
    # ⛔⛤ SAID OUT LOUD, BECAUSE A SILENTLY DROPPED WRITE IS HOW THE
    # `.write_message(..)` GAP OPENED. These arguments name no registered
    # message type — a local variable, or a constructor like
    # `AppExit::from_code(..)` whose type this pass cannot resolve from the call
    # site. None is a sim-schedule consumer's producer today; the number is here
    # so a NEW one is visible rather than absent.
    # ⛔⛤ THE CROSSING SET IS A LOWER BOUND AND SAYS SO EVERY RUN. A system the
    # schedule map cannot place is dropped from both sides of the comparison.
    unlocated = unlocated_message_systems()
    if unlocated:
        exposed = {
            ty
            for ty, writers, readers in _message_sides(REPO)
            if any(fn in set(unlocated) for fn in list(writers) + list(readers))
        }
        print(
            f"⚠ LOWER BOUND: {len(unlocated)} message writer(s)/reader(s) are in no "
            f"registration this script can follow — neither an `add_systems` body nor a "
            f"wrapper that forwards into one — touching {len(exposed)} of the "
            f"{len(_message_sides(REPO))} written message types. Any of those could be an "
            "unseen crossing — see `unlocated_message_systems` for the reproducer."
        )

    unresolved = unresolved_message_writes()
    unadjudicated = sorted(
        {
            (file, arg)
            for file, arg in unresolved
            if unresolved_head(arg) not in UNRESOLVED_ADJUDICATED
        }
    )
    if unadjudicated:
        print(
            f"{len(unadjudicated)} `.write_message(..)` call(s) name no registered "
            "message type and nobody has said why:\n\n  "
            + "\n  ".join(f"{file}: {arg}" for file, arg in unadjudicated)
            + "\n\nThis script attributes a write by reading its argument, so an "
            "argument it cannot read is a write that silently leaves the population — "
            "a crossing can be hidden from every count above by binding it to a local "
            "first. Resolve it (`_resolve_binding` handles a `let`, a `for` over a "
            "call, and a parameter) or add the head to UNRESOLVED_ADJUDICATED with the "
            "argument for why it is out of scope.",
            file=sys.stderr,
        )
        return 1
    if unresolved:
        print(
            f"⚠ {len(unresolved)} `.write_message(..)` call(s) name no registered message "
            f"type and are adjudicated as out of scope: "
            + ", ".join(sorted({unresolved_head(a) for _f, a in unresolved}))
        )

    drift = ruling_marker_drift(
        spent_resources=len(spenders),
        resource_crossings=len(found),
        written_messages=sizes["message types written"],
        message_crossings=len(messages),
        unlocated=len(unlocated),
        unlocated_types=len(exposed) if unlocated else 0,
    )
    if drift:
        print("\n⛔ the ruling that cites this census no longer matches it:")
        for line in drift:
            print(f"  {line}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
