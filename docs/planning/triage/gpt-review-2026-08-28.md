# GPT review — the twelve-hour pass (relayed by Jon, 2026-08-28)

Reviewed `c154f3a86..a97c80feb`, 88 commits. Six findings carried over from the
previous review plus seven new ones.

✔ **All thirteen were addressed. TWO WERE THEN REOPENED BY THE NEXT PASS**
(below), and both reopenings are the same shape: a fix that was right at the
layer it touched and stopped one layer short. Every fix carries a poison; the
poisons were the layer the fix touched, which is exactly why they passed.

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
| 12 | menu row identity was `format!("{action:?}")` | `683b3b7e0`, then ↓ |
| 5 | the moveset exporter attributed the body's ranged kit to every move | `683b3b7e0` |
| 6 | package-filtered nextest runs lost doctest coverage | `8b85606ff` |
| 11 | nextest's unmeasured duration was persisted as a real `0.0` | `8b85606ff`, then ↓ |
| 10 | thrown-item impact was aggregate and unswept | `daa4e6b55` |
| 2 | the Trap's surfacing wrote a position with no Class-B remap | `daa4e6b55` |

## ⛔⛔ THE NEXT PASS REOPENED TWO, AND BOTH STOPPED ONE LAYER SHORT

Reviewed through `ec85703ea`. Findings 2, 4 and 5 of that pass are another
agent's (`moveset_takes`, `check_clip_handedness.py`); these two are these rows.

**11 — the per-job `null` never reached the ledger.** `JobResult.executed_seconds`
became `float | None` and `timings_payload` emitted `null`, and then three
aggregates summed `r.executed_seconds or 0.0` — the cost-ledger row, the status
payload, the human report — while `compile_report.py` derived
`build_seconds = seconds - executed_seconds`. So a 100s nextest run was still
PERSISTED as *0s executing, 100s building*. ⇒ the split is three numbers now
(`executed` / `build` / `unclassified`), build is derived only from jobs that
reported, and `build_share` is against what the report can account for.

**12 — `Action` is what a row DOES, not which row it is.** Replacing the `Debug`
string removed a real defect and left a coarser one: two destructive rows with
the same action share a confirm arm, so arming *Quit to Desktop* on one and
tapping the other fires it on the first tap. ⇒ keyed by `MenuFocusKey`, the
identity the menu already carries and which `PressArm`'s own doc asks a flat list
to use; `Action` rides beside it as the payload emitted on activation.

⭐ **THE LESSON THEY SHARE is about the poison, not the fix.** Both were
poison-verified — and both poisons targeted the layer the fix touched, so both
passed while the end-to-end answer stayed wrong. ⇒ **when a fix changes a
representation, poison the CONSUMER of that representation**, not the place it is
produced: the ledger row, not `timings_payload`; two rows with an equal action,
not one row tapped twice.

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

## ▢ THREE FINDINGS FROM THE SECOND PASS BELONG TO ANOTHER AGENT'S WORK

Recorded here so they are not lost between two agents' lanes. None of these
touch code I wrote; all three are on the `moveset_takes` / handedness-checker
road that landed in parallel.

▢ **`moveset_takes::settle` treats a MISSING seat zero as settled.**
`settle_facts` returns `(false, false, false)` both for a real idle airborne
fighter and for no seat-zero fighter at all, and `settle` accepts
`grounded || !ever_stood` with `ever_stood` starting false. ⇒ on the first
iteration a missing fighter, an ordinary airborne fighter and a flying Robot all
read as settled; only the third is intended. ⛔ `ever_stood` answers *"has this
body reported ground since this call began"*, not *"is this a body whose
locomotion may settle in the air"* — the question it is standing in for. The
reviewer also flags that `reseat` treats 240 app updates as proof of staging with
no postcondition that seat zero exists.

▢ **`check_clip_handedness.py` passes when there is nothing to check.**
`check_sheet` returns no findings when the generated `*_spritesheet.yaml` is
absent, and the default target population is a glob over those same generated
files — which are gitignored. Run on a clean tree it reports
`OK: 0 sheet(s), every clip reaches forward`, and naming `officer` explicitly
still succeeds while skipping it. It is also wired into no test or gate, so a
backwards clip can land without it ever running. ⛔ absence must not be success.

▢ **The take writer claims canonical identity and allows `id: null`.** Both
queries ask for `Option<&SimId>`, the sort maps a missing id to the empty string,
and the bundle checker only warns. `ensure_sim_id` documents that a dynamically
spawned body can remain unidentified — so an unidentified body can enter a take
and silently downgrade the ordering/join contract the comments call byte-stable.
⚠ the viewer's fallback is right for LEGACY takes; it should not define what the
current recorder may emit.

## Not closed by this pass

▢ **Production-comment cleanup.** The review is right that source still carries
dated incident chronology, old hypotheses and debugging narratives beside the
comments that state a contract. Lower priority than any of the above, and the
ledger should not read as if that campaign finished.
