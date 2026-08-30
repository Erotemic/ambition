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

## ✔ THREE FINDINGS I FIRST ROUTED AWAY, THEN FIXED

⛔⛔ **AND THE ROUTING WAS THE MISTAKE.** These landed on the `moveset_takes` /
handedness-checker road that another agent built, so I recorded them here as
"another agent's lane" instead of fixing them. Jon pushed back and was right:
I had edited `moveset_export.rs` in that same directory the same day and
committed to the renderer submodule twice, and finding 2 is a correctness bug in
what this project treats as CANONICAL EVIDENCE. ⇒ **a lane is not a reason**;
if the defect is real and the code is reachable, fix it.

✔ **`moveset_takes::settle` treated a MISSING seat zero as settled** — fixed `7ef7ebe93`'s parent `813bc16f1`. Presence is explicit (`Option<SettleFacts>`), the airborne question is answered by the body's LOCOMOTION rather than by `ever_stood`'s observation history, and `reseat` returns whether seat zero actually arrived instead of spending a fixed 240 updates. Four arms, poisoned both ways.
`settle_facts` returns `(false, false, false)` both for a real idle airborne
fighter and for no seat-zero fighter at all, and `settle` accepts
`grounded || !ever_stood` with `ever_stood` starting false. ⇒ on the first
iteration a missing fighter, an ordinary airborne fighter and a flying Robot all
read as settled; only the third is intended. ⛔ `ever_stood` answers *"has this
body reported ground since this call began"*, not *"is this a body whose
locomotion may settle in the air"* — the question it is standing in for. The
reviewer also flags that `reseat` treats 240 app updates as proof of staging with
no postcondition that seat zero exists.

✔ **`check_clip_handedness.py` passed when there was nothing to check** — fixed in the renderer at `53dbceb`. Absence is a finding for a NAMED target and an empty glob fails the run; ⚠ but not for a discovered one, because the default population is every published sheet and most are not rigged fighters — failing those would make it permanently red. Seven fixture-based tests, so the guard no longer depends on gitignored art, and it now runs at all.
`check_sheet` returns no findings when the generated `*_spritesheet.yaml` is
absent, and the default target population is a glob over those same generated
files — which are gitignored. Run on a clean tree it reports
`OK: 0 sheet(s), every clip reaches forward`, and naming `officer` explicitly
still succeeds while skipping it. It is also wired into no test or gate, so a
backwards clip can land without it ever running. ⛔ absence must not be success.

✔ **The take writer claimed canonical identity and allowed `id: null`** — fixed `7ef7ebe93`. A take that recorded a body with no `SimId` is skipped and the body named, because the fix belongs at its spawn site. ⭐ and `ensure_sim_id`'s doc claimed an `unidentified_bodies` counter that was never written; the recorder is that number now. Both
queries ask for `Option<&SimId>`, the sort maps a missing id to the empty string,
and the bundle checker only warns. `ensure_sim_id` documents that a dynamically
spawned body can remain unidentified — so an unidentified body can enter a take
and silently downgrade the ordering/join contract the comments call byte-stable.
⚠ the viewer's fallback is right for LEGACY takes; it should not define what the
current recorder may emit.

## Not closed by this pass

◐ **Production-comment cleanup.** The review is right that source carries dated
incident chronology and debugging narrative beside the comments that state a
contract — and it raised this TWICE, the second time noting that the batch fixing
its own findings added more of it.

✔ **The six I wrote this session are trimmed** (`9fbb92256`): the Trap, the
Monologue, both copies of the blink exemption, `JobResult`, the brain dispatcher's
line-count correction, and the re-export note — which got STRONGER, from a record
of nine deleted names to an instruction not to add more.

▣ **CLOSED 2026-08-29. The sweep ran, and its value was in the RULE it had to
state first.** ⚠ do NOT turn it into a de-dating campaign: `Moved 2026-08-28
(D33)` on a carved module is PROVENANCE and the repo uses it everywhere. The
target is the incident TRANSCRIPT around a rule — what the code used to do, when
it was corrected, and a quotation of the sentence that was wrong.

⭐⭐ **THE SWEEP IS ONLY SAFE ONCE THE THREE CLASSES ARE SEPARATED, because two of
them look identical to a grep and only one is surplus:**

| class | census | verdict |
|---|---:|---|
| model attribution — *which reviewer found it* | 10 | ⛔ REMOVED. Incident history; the review documents own it |
| first-person debugging narrative — *"I first wrote…", "I had guessed…"* | 9 | ⛔ REMOVED. The reasoning survives in the third person |
| a QUOTATION of the superseded sentence, with the chronology around it | 28 | ⛔ REMOVED, keeping the hazard the quotation was illustrating |
| dated comments generally (`2026-08-2x` on a rule) | **241** | ✔ KEPT. A date on a rule that CHANGED is part of the rule |
| "X is not Y" where Y is the wrong reading | ~200 | ✔ KEPT. Naming the wrong reading is what makes the rule checkable |
| Jon's quoted words | — | ✔ KEPT. The maintainer's sentence is the authority, not a transcript |

⭐ **THE TEST FOR EACH ONE: delete the sentence and ask whether the RULE is still
checkable.** *"`fly_toggle` must stay false, because a boss steers only by
commanded velocity"* survives; *"this said `fly_toggle: true` — the toggled kind
a boss has always had"* does not add to it. The first is a contract, the second
is a diff.

⛔⛔ **AND ONE SUB-CLASS WAS QUOTING A SENTENCE THAT NO LONGER EXISTS.** Three
comments (`stocks_match.rs`, `rollback_coverage.rs`, `stocks.rs`) quoted
`clear_message_on_rollback`'s claim that the clear *"restores the channel with its
cursor"* — a comment that had already been corrected, so the quotation was the
only remaining copy of a sentence the repo had disowned. ⇒ **a quoted mistake
outlives the mistake**, and a reader greps the quotation and finds it.

⭐ Also fixed in passing: seven copies of one mangled fixture comment across six
app tests (blank `//` line between every sentence), replaced by the rule it was
narrating — spawn by CHARACTER, because a `Custom(..)` archetype row that no
longer exists falls back to a generic `combatant` and the fixture keeps passing
while asserting on the wrong body.

⚠ **WHAT WAS NOT TOUCHED, deliberately:** the two test docs that quote this
review as their SPEC (`smash_in_the_host.rs`), for the same reason R19 kept a
waiver's reason string — there the quoted text IS the contract.
