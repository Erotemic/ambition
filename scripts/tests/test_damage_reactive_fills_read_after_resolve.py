"""A meter that ACCUMULATES from `ResolvedBodyHit` must read AFTER `Resolve`.

⛔⛔ THIS GUARD EXISTS BECAUSE THE OPPOSITE SHIPPED AND BROKE ROLLBACK OUTRIGHT.
`fill_limit_meters` was registered in `CombatSet::ContentSpecials`, which sits
INSIDE `Materialize` and therefore runs BEFORE `Resolve`. Bevy's message buffers
are double-buffered, so a reader there consumes the PREVIOUS frame's hits.

⭐ AND `clear_message_on_rollback` DOES NOT RESCUE IT — `ResolvedBodyHit` is
registered `message-clear` in the schema baseline and the divergence happened
anyway. The clear wipes the buffer at load, so a leftover the original run
consumed at frame N+1 is absent from the resimulation. `rollback_exit_oracle`'s
per-component diagnostic named `BodyMana`; a two-arm experiment (fill-from-hits
disabled ⇒ 3/3 green, enabled ⇒ 3/3 red) pinned it to this one read.

⚠ THE RULE IS ABOUT THE READER'S ARITHMETIC, NOT THE MESSAGE. A LATCHING reader
survives the early phase — `mark_move_playback_resolved_hits` reads a frame late
by design and only sets booleans, and setting a bool twice is setting it once. An
ACCUMULATOR cannot, because `+=` is exactly what a doubled or dropped read
corrupts. So this guard names the accumulators, not the channel.

The end-to-end guard is `rollback_exit_oracle`, which is where the bug was
actually caught. This one is the FAST one: it fails in a second, at the line a
future author would change, instead of 50 seconds later inside a checksum.
"""

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]

# (file, system, the set it must be registered in, why that phase and no other)
#
# BOTH halves of the Limit are listed, because each is an accumulator and each is
# wrong in the OTHER's phase. The damage half read a frame late and broke
# rollback; the authored half would read a press a frame late and pay a move that
# had already ended. One rule, two directions.
ACCUMULATORS = [
    (
        "game/ambition_demo_smash/src/lib.rs",
        "fill_limit_meters",
        "ContentFlavor",
        "`ContentSpecials` sits inside `Materialize` and runs BEFORE `Resolve`, so a reader "
        "there consumes the PREVIOUS frame's `ResolvedBodyHit` out of Bevy's second buffer. "
        "Under GGRS that leftover is cleared at load and never re-emitted, so the meter fills "
        "on the original pass and not on the resimulation — `rollback_exit_oracle` goes red "
        "with `BodyMana` diverging. An ACCUMULATING damage reader belongs in `ContentFlavor`, "
        "between `Resolve` and `Settle`, where every hit is read on the frame it was emitted.",
    ),
    (
        "game/ambition_demo_smash/src/lib.rs",
        "apply_authored_meter_fills",
        "ContentSpecials",
        "the authored half reads `ActorActionMessage`, and `ContentSpecials` is the phase whose "
        "own doc promises that \"a special dispatched this frame reaches its content technique "
        "THIS frame\". Moved later it would read the press out of the second buffer on the "
        "following frame — the same cross-frame read, in the same direction, on a different "
        "channel.",
    ),
]


def test_damage_reactive_fills_are_registered_after_resolve():
    checked = 0
    for rel, system, required_set, why in ACCUMULATORS:
        path = REPO / rel
        assert path.exists(), f"guard points at a file that is gone: {rel}"
        source = path.read_text()

        assert (
            system in source
        ), f"{rel} no longer registers `{system}` — the guard is stale, not passing"

        # The registration block: the system name through the `.in_set(...)` that
        # follows it. Anchored on the system so a second registration elsewhere
        # cannot satisfy the guard on the first one's behalf.
        match = re.search(
            re.escape(system) + r"\s*\.?\s*\n?\s*\.in_set\([^)]*::(\w+)\)",
            source,
        )
        assert match, (
            f"{rel}: could not find an `.in_set(...)` attached to `{system}`. "
            "A damage-reactive accumulator must state its phase explicitly — an "
            "unset system runs wherever the schedule happens to put it."
        )
        found = match.group(1)
        assert found == required_set, (
            f"{rel}: `{system}` is registered in `{found}`, not `{required_set}`.\n⇒ {why}"
        )
        checked += 1

    assert checked == len(ACCUMULATORS) and checked > 0, "the guard checked nothing"
