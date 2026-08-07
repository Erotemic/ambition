# Smash Siblings — the mode Jon specified

Jon, 2026-07-29, answering "does versus end on HP or on stocks?":

> For a generic "versus" fighting proof of concept I don't care. Probably use
> health, to make it a generic fighter. For Smash Siblings **3 stock, no items,
> final destination, fox only** (that's fox only part is a joke). What I want for
> smash siblings is actually **character select screen, ability to have 1-4
> players, have them toggle between real player or cpu, use a smash like, drag an
> orb onto your character to select**, and then the fight boots into a **single
> battlefield like 3 platform level**. Its **3 stocks**, and then **when the game
> ends it goes back to the character select screen**. We don't need items in a
> first pass.

And on the HUD (answering the reserved-surround question):

> A smash game would have a **character portrait on the bottom for each character
> with an icon for each stock and their current percentage**. There is **no
> score**, when you lose your stock you are dead.

⚠ **this is a SEPARATE mode from `versus_gameplay`, not a reskin of it.** Generic
versus stays a health fighter — that is the other half of the same decision, and
it is why `DeathPolicy::Unbounded` stays uncalled there.

---

## What already exists (checked, not assumed — 2026-07-29)

| Piece | Where | State |
|---|---|---|
| smash PERCENT (meter never kills; the world does) | `DeathPolicy::Unbounded` | ⛔ **NOT lands** — see part 0 below |
| blast zones / OOB death | queue F0 | ✔ |
| DI | `Platformer2dFeelTuningMonolith::di_max_angle`, `VERSUS_DI_MAX_ANGLE` | ✔ on for the fighting stage |
| 1–4 local players from device discovery | `LocalSeatTopology` | ✔ |
| a roster of participants, human or CPU | `MatchParticipantRoster` / `MatchParticipant` | ✔ |
| seating them as bodies | `seat_match_participants`, `MatchSeat` | ✔ |
| per-fighter teams | `MatchTeam` | ✔ |
| round/KO/countdown state machine | `VersusMatch`, `MatchPhase` | ✔ (ROUNDS, not stocks) |
| route-keyed declared HUD | the `with_hud` seam | ✔ |
| character portraits | `CharacterCatalog::portrait_ref` + registry | ◐ resolver being built (Y″6) |

**So the missing pieces are the DAMAGE MODEL, the RULE, and the two screens.**

⚠ this line read *"the missing pieces are the RULE and the two screens, not the
fighting"* until 2026-07-30, on the strength of the ✔ above it. Both were wrong,
and the row was checked the way a row gets wrongly checked: `DeathPolicy` exists,
compiles, has a test, and is named "smash percent" in its own doc comment. What
nobody did was ask the meter to count past 100 (GPT 5.6, 2026-07-30 — the finding
holds).

---

## The parts, in dependency order

### 0. THE DAMAGE MODEL — percent has to be able to EXCEED the pool ✔ LANDED

> ✔ **VERIFIED AGAINST THE TREE 2026-08-07.** Every claim below is now false of
> the code, which is the good outcome: `BodyHealth::damage_taken()` is
> `self.accumulated` — an unbounded counter, not the saturating `(max -
> current).max(0)` this row is about. `DeathPolicy` MOVED DOWN into
> `ambition_characters::actor::body` exactly as the row said it had to
> ("the enum moves down, it cannot be a field where it is"). Production DOES
> select `Unbounded` (`prepared_match.rs:200` → `with_policy` at :498), so the
> row's "nothing in production selects `Unbounded`" no longer holds either. The
> HUD consequence landed with its own test, named for the number this row
> predicted: `damage_percent_is_unclamped_so_a_hud_can_print_188`.
> **The analysis is kept because it is why the shape is what it is.**

`DeathPolicy::Unbounded` suppresses one consequence — a meter-kill no longer
kills. It does not make the meter unbounded, and the meter is the thing smash
percent *is*:

* `Health::damage` clamps: `self.current = (self.current - amount).max(0)`;
* `BodyHealth::damage_taken()` is `(max - current).max(0)`, so it **saturates at
  `max`** — 100% is the ceiling, not a display normalizer;
* `Health::alive()` is `current > 0`, and `resolve_body_hit` returns `Ignored`
  for a body that is not alive. So an `Unbounded` fighter at zero HP stops taking
  hits entirely: it cannot be damaged, cannot be launched further, and cannot be
  knocked off the stage by the one mechanism that is allowed to kill it.

Knockback growth scales off `damage_taken()`
(`knockback + kb_growth * victim.damage_taken() / victim.weight`), so the
saturation is not cosmetic — at the exact point a smash match becomes decidable,
every fighter's launch distance stops growing and the body becomes an inert,
unkillable punching bag. That is the opposite of the mode.

⚠ **the reason none of this showed up is that nothing in production selects
`Unbounded`.** The enum's own tests assert `kills_at_max()` for both variants,
which is true and proves nothing about the meter. `Unbounded` has exactly one
consumer (`actor_hit.rs`) and it reads only the death flag.

**What the mode needs**, and it is one authority, not three patches:

```text
accumulated damage        counts UP, no ceiling — the smash-percent axis
death by threshold        HpDepleted: the meter reaching the pool kills
death by the world        blast zone / OOB — already engine-owned, already unstoppable
```

Today the first two are one clamped `i32` and the policy that separates them
lives in `actor_tuning`, where the resolver cannot see it. Consequences to settle
when this is built:

* `alive()` is currently a fact about the meter. Under `Unbounded` it must be a
  fact about the BODY, so the policy has to travel with the health component
  (`DeathPolicy` lives in `ambition_combat`, `Health` in `ambition_characters`,
  and `ambition_combat` depends on `ambition_characters` — so the enum moves
  down, it cannot be a field where it is);
* the ADOPTED controlled-body path takes the same route and is not separately
  handled;
* the HUD reads a percent that can print `188%`, which `Health::ratio()`
  (clamped to 1.0) cannot express.

### 1. STOCKS — the rule that makes it Smash ✔ LANDED

> ✔ **VERIFIED 2026-08-07, and all five of the things this row says are missing
> now exist.** It reads *"`FighterStocks` is vocabulary only right now — no
> runtime consumer, no stock-loss rule, no respawn lifecycle, no rollback
> registration, no elimination flow."* `crates/ambition_combat/src/stocks.rs`
> carries the rule and `FighterEliminated`; the component is rollback-registered
> (`domains/combat.rs:89` — the row's own warning that "a stock count that is not
> registered rollback state is a stock count that un-spends itself on a rewind");
> it is inserted in production at `prepared_match.rs:863`; and it has its own
> ordered set, `FighterStocksSpent`.

A fighter has N stocks (3). A blast-zone death spends one. At zero it is
**eliminated** — out, not respawned. The match ends when one fighter remains.

⚠ **this is not `rounds_won` with a different name.** Rounds are symmetric and
reset both fighters; stocks are per-fighter, asymmetric, and never reset. A
2-stock fighter versus a 3-stock fighter is a legal, common mid-match state and
the round model cannot express it.

⚠ **it must compose with part 0**, or the percent meter kills first and stocks
never get consulted — and with part 0 as it stands today, a fighter at 100%
simply stops being hittable and no stock is ever spent either. Stocks after the
damage model, not before: a stock rule built on a saturating meter would look
correct in every test that never reaches 100%.

`FighterStocks` is vocabulary only right now — no runtime consumer, no
stock-loss rule, no respawn lifecycle, no rollback registration, no elimination
flow. All five land together with the rule; a stock count that is not registered
rollback state is a stock count that un-spends itself on a rewind.

### 2. The 3-platform stage ✔ LANDED

> ✔ `smash_stage()` / `SMASH_STAGE_ROOM_ID` in `ambition_demo_smash`, authored as
> a room exactly as this row asked.

"battlefield like": one main platform plus three floating ones, blast zones on
all four sides. Authored as a room, not a special case — the engine already
stages rooms and the blast zone is engine-owned.

### 3. Character select ✔ LANDED — the biggest UI piece

> ✔ **and it became Jon's own headline feature**, spec kept verbatim in
> `JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`. Both parts this row flagged as risky are
> in: the CPU toggle (`SlotOccupant::Cpu`) and the ORB DRAG, which is a real
> drag with its own click-vs-drag slop threshold (`select_screen/cursor.rs`,
> mirroring `ROW_TAP_SLOP_PX`) rather than a menu list.

1–4 seats, each **toggling between human and CPU**, and selection by **dragging
an orb onto a character**. `LocalSeatTopology` already answers "how many humans
are plugged in"; the screen has to let a seat be a CPU regardless, which the
roster's `MatchParticipant` already models.

⚠ the orb drag is a REAL interaction, not a menu list: an orb per seat, a grid of
characters, and a drop target. This is the part most likely to need its own
slice.

### 4. The loop ✔ LANDED

> ✔ `RETURN_TO_SELECT_AFTER` counts down on match end and issues
> `ShellCommand::GoTo(SMASH_SELECT_ROUTE)`. ⭐ and the trap this row named — state
> outliving its match — was live until 2026-08-07 in a form the row did not
> predict: smash and versus BOTH removed `ActiveMatch`/`PreparedMatch` by type on
> the way out, so whichever left first deleted the other's match. Now
> owner-scoped, with `app_it::experience_scope_ownership` over the composed scope
> registry.

Match over → back to character select. A shell ROUTE transition, and the
`ShellRouter` already owns those. The trap to avoid is the one `VersusMatch`
already hit: state that outlives its match, so the next visit resumes somebody
else's game.

### 5. The HUD ◐ LANDED, with two presentation deviations

> ✔ per fighter along the bottom, sorted by SEAT (query order is not an order),
> with unwritten slots CLEARED so a 1v1 does not inherit the last match's fourth
> fighter. Percent is unclamped.
> ⚠ **two deviations from this row, stated rather than quietly accepted**: it
> asks for a PORTRAIT and ONE ICON PER REMAINING STOCK; the HUD prints the
> fighter's NAME and the stocks as text (`2/3`). Both are presentation choices
> a photograph would settle, and neither is a defect — but a row that asked for
> icons and got text should say so.

Per fighter, along the bottom: **portrait, one icon per remaining stock, current
percent**. **No score** — the reserved-surround branch (`wip/versus-reserved-surround`)
existed to place a scoreboard, so Jon's answer makes it moot rather than wanted.

---

## Deliberately NOT in a first pass

* **items** — Jon: *"We don't need items in a first pass."*
* **"final destination, fox only"** — a joke, per his own parenthetical.
* **stages beyond the one** — one battlefield-like level.
* **`interact` on the fighting stage** — a SEPARATE open question he has
  explicitly reserved for a design discussion (queue Z′4): whether `interact`
  belongs to the engine at all or only to Ambition-the-game.

---

## Named debt after the match-preparation campaign (2026-08-07)

The campaign that made CPU-vs-CPU and person-vs-CPU work is done and the
freeze that followed it is fixed. Two things are deliberately NOT done, named
here so neither reads as an oversight.

### The "one match lifecycle" invariant is not fully reached — engine backlog

`PreparedMatch` is the authority for SMASH. Versus still consults
`RosterSeating::{Proposed, Activated}`, `activate_if_seatable` and
`SessionSeatingSource` as independent readiness/topology gates *before*
preparation, rather than as inputs to it. They still carry useful data — the
frozen topology in particular — so this is consolidation, not deletion, and it
belongs with the participant-action work rather than bolted onto a bug fix.

⚠ **do not describe the invariant as achieved.** It is achieved for one
provider.

### The Perfect Cellular Automaton is off the grid, on purpose

`perfect_cellular_automaton` is in `SMASH_ROSTER` and deliberately absent from
the registered playable cast, so the shipped grid is one portrait shorter than
the authored roster. The cause is a real and separate engine defect —
**a fight that starts before its sheets land never recovers** — measured on
2026-08-06: sixty seconds with neither fighter pressing melee, where settling
180 frames first gives an ordinary fight. Combat geometry resolved against a
missing sheet appears to stick for the life of the body.

Jon, 2026-08-07: *"I do want to fix the PCA issue, but we can do that in a
separate pass, because I need to understand why it is an issue first."* So the
workaround stays until the sheet-timing defect is understood, and this
paragraph is the waiver — a test asserting the exact grid contents would pin
the workaround rather than the gap it stands in for.

⚠ `every_smash_roster_id_resolves_in_the_shipped_host` checks the
`CharacterCatalog` while `SmashRoster::assemble` filters the
`PreparedCharacterRegistry`, so nothing fails today on this omission. That is
the correct outcome for a decision somebody made, and the wrong one for a
decision somebody forgot — which is why it is written down here.
