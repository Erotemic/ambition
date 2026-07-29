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
| smash PERCENT (meter never kills; the world does) | `DeathPolicy::Unbounded` | ✔ lands, **uncalled** |
| blast zones / OOB death | queue F0 | ✔ |
| DI | `SandboxFeelTuning::di_max_angle`, `VERSUS_DI_MAX_ANGLE` | ✔ on for the fighting stage |
| 1–4 local players from device discovery | `LocalSeatTopology` | ✔ |
| a roster of participants, human or CPU | `MatchParticipantRoster` / `MatchParticipant` | ✔ |
| seating them as bodies | `seat_match_participants`, `MatchSeat` | ✔ |
| per-fighter teams | `MatchTeam` | ✔ |
| round/KO/countdown state machine | `VersusMatch`, `MatchPhase` | ✔ (ROUNDS, not stocks) |
| route-keyed declared HUD | the `with_hud` seam | ✔ |
| character portraits | `CharacterCatalog::portrait_ref` + registry | ◐ resolver being built (Y″6) |

**So the missing pieces are the RULE and the two screens**, not the fighting.

---

## The five parts, in dependency order

### 1. STOCKS — the rule that makes it Smash ▢ FIRST

A fighter has N stocks (3). A blast-zone death spends one. At zero it is
**eliminated** — out, not respawned. The match ends when one fighter remains.

⚠ **this is not `rounds_won` with a different name.** Rounds are symmetric and
reset both fighters; stocks are per-fighter, asymmetric, and never reset. A
2-stock fighter versus a 3-stock fighter is a legal, common mid-match state and
the round model cannot express it.

⚠ **it must compose with `DeathPolicy::Unbounded`**, or the percent meter kills
first and stocks never get consulted.

### 2. The 3-platform stage ▢

"battlefield like": one main platform plus three floating ones, blast zones on
all four sides. Authored as a room, not a special case — the engine already
stages rooms and the blast zone is engine-owned.

### 3. Character select ▢ — the biggest UI piece

1–4 seats, each **toggling between human and CPU**, and selection by **dragging
an orb onto a character**. `LocalSeatTopology` already answers "how many humans
are plugged in"; the screen has to let a seat be a CPU regardless, which the
roster's `MatchParticipant` already models.

⚠ the orb drag is a REAL interaction, not a menu list: an orb per seat, a grid of
characters, and a drop target. This is the part most likely to need its own
slice.

### 4. The loop ▢

Match over → back to character select. A shell ROUTE transition, and the
`ShellRouter` already owns those. The trap to avoid is the one `VersusMatch`
already hit: state that outlives its match, so the next visit resumes somebody
else's game.

### 5. The HUD ▢

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
