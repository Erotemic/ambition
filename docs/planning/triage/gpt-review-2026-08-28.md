# GPT review — the twelve-hour pass (relayed by Jon, 2026-08-28)

Reviewed `c154f3a86..a97c80feb`, 88 commits. Six findings carried over from the
previous review plus seven new ones.

✔✔ **ALL THIRTEEN ARE CLOSED.** Every one was confirmed at HEAD before being
touched, and every fix carries a poison. Shas below.

⭐⭐ **THE REVIEW'S SHARPEST FINDING WAS ABOUT WORK I HAD JUST DONE AND CALLED
FINISHED.** `HeldItemView` had been made plural the day before — the right
change, one step short. It still asked `With<DrivingParticipant>`, and the Pirate
Admiral's side-B draws `admiral_gun_sword`, an id registered in the IN-HAND art
manifest and in no other, so in a CPU-versus-CPU match neither Admiral's
gun-sword was drawn by any road at all. **My own new test asserted that as
intended behaviour.** ⇒ a fix that is 90% right ships a test that defends the
last 10% of the bug.

| # | finding | landed |
|---|---|---|
| 7 | held-item presentation followed DRIVING authority, not custody | `e14fe5d69` |
| 1 | the Trap's submerged beat could not steer — `hitless_special` roots the whole duration and `motion_scale_at` folds with `min` | `c600a0d4e` |
| 3 | Monologue's doc names `hitless_special`'s rooting; it is built from `strike` | `c600a0d4e` |
| 4 | the wire asked for `player.blink` beside the teleport executor's | `c600a0d4e` |
| 8 | provider semantic actions never consulted the resolved seat context | `a6b54ffd3` |
| 13 | the D166 ratchet excluded a whole file for one known edge | `a6b54ffd3` |
| 9 | TwinTrack repaired a one-tick wrong-authority episode instead of preventing it | `4c32152ac` |
| 12 | menu row identity was `format!("{action:?}")` | `683b3b7e0` |
| 5 | the moveset exporter attributed the body's ranged kit to every move | `683b3b7e0` |
| 6 | package-filtered nextest runs lost doctest coverage | `8b85606ff` |
| 11 | nextest's unmeasured duration was persisted as a real `0.0` | `8b85606ff` |
| 10 | thrown-item impact was aggregate and unswept | `daa4e6b55` |
| 2 | the Trap's surfacing wrote a position with no Class-B remap | `daa4e6b55` |

## What the review got wrong, and it is worth knowing which half

⚠ **Finding 1's mechanism was half wrong and the defect was real anyway.** It
said the Trap *"adds another overlapping window with `motion_scale = 1`"* that
the `min` fold defeats. `author_trapdoor` adds an EVENT and no window at all —
there was never a competing 1.0. The zero came from `hitless_special` alone, and
the fix is the Recovery window starting at `SURFACE_AT_S` instead of `SINK_AT_S`,
which leaves the submerged span uncovered so the fold returns its identity.
⇒ **confirm the mechanism even when the conclusion is right**; the proposed fix
(author three windows) would have worked and been larger.

⚠ **Finding 4's exemption was the interesting part.** `author_teleport_blink.rs`
carried a note listing *"the Actor's trap and wire and Alice's side-B"* as moves
that never run the teleport executor. True of the trap (`author_trapdoor`) and of
Alice (an `impulse`); **false of the wire**, which is authored through
`author_teleport`. A note written to explain why a cue was not duplicated was
covering one that was.

## Two things a later reader should not "fix" back

⛔ **The held-item partition is not a carve-out.** `HeldItemView` covers every
holder MINUS the ones the over-hand road claims, and `drawn_over_the_hand` is
that road's own admission test hoisted out of it so both systems read one
sentence. It is a partition because BOTH registries carry `gun_sword`: a holder
on both roads draws twice and a holder on neither draws not at all. The corpse
arm in the tests is the one that matters — it is where the two conditions
disagree, and how a body ends up on neither road.

⛔ **`ClassBRemap::ScriptedTeleport` covers the Trap's surfacing on purpose.**
The review suggested a peer variant, since a trapdoor is not a teleport in the
fiction. The enum sorts WHO MOVED THE BODY, and a move that picks a destination
and writes it is the same authority however it is dressed. The variant's doc now
says so.

## The withdrawal

⭐ The reviewer retracted its own earlier finding that locally driven match
bodies might lack a stable `SimId` and order nondeterministically. Prepared match
bodies carry seat-specific `FeatureId`s and `ensure_sim_id` runs before core
simulation. ⇒ **that row is dead; do not carry it forward.**

## Not closed by this pass

▢ **Production-comment cleanup.** The review is right that source still carries
dated incident chronology, old hypotheses and debugging narratives beside the
comments that state a contract. Lower priority than any of the above, and the
ledger should not read as if that campaign finished.
