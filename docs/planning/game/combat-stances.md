# Combat stances

**Status:** asked by Jon 2026-11-27; S1–S5 landed the same day.
Queue: COMBAT-STANCES.

Jon (2026-11-27): *"I was wondering if you could implement variants of the
players kit (if you don't have space in inventory you can remove any item that
doesn't actually do something interactive in game). But I want a combat stance
1, 2, and 3. with 1 being the current, and 2 and 3 being your proposed
versions. I still want something that feels like the knight or hornet, but
also its own thing, maybe a bit of marth / lucina in there. The combat stances
should replace the entire hit kit (a pogo is always available, potentially in
different forms), but the tilts, smashes, aerials might change."*

## The mechanism

A stance is an item, as the dashes are: three rows in one exclusive slot
(`stance`), so wearing one takes the slot from the others. Stance I is the
body's own kit (an empty row, as `dash` is the body's own dash).

A row **rebinds** verbs: `rebinds: {"attack": "lunge_thrust", ...}`. The moves
are authored in Scout's own table (`data/movesets/player_robot.ron`) under
their stance's prefix, so every move has one owner and the repertoire fold
(`ambition_characters::repertoire::effective_repertoire`) stays the one place
a body's kit is decided. A rebinding to a move the body's table does not have
changes nothing (a stance row on another character is inert), and taking the
row off gives the verbs back, as `lends` and `withholds` do.

A stance rebinds the whole home hit kit: `attack` and its directions, the air
attacks and their directions, and the three smashes. Specials, grabs and the
taunt are the character's and stay.

## The stances

Each has a pogo: its down-air bounces on what it strikes (`pogo_bounce`).

| | I — Edge (the current kit) | II — Lunge | III — Crescent |
| --- | --- | --- | --- |
| Feel | The knight: a 1-2 that roots no one, short and safe. | Hornet's momentum, Scout's own: thrusts that carry the body, reach over width. | Marth/Lucina spacing: wide arcs with a sweet spot at the tip. |
| Attack | Rise, fall, thrust (or rise-fall-rise-fall held forward). | A thrust that carries the body forward; a second press is a longer lunge. | A three-step string: a level arc, then up or down by the stick, then a finishing sweep. |
| Up | Overhead. | A corkscrew thrust straight up, tall and narrow (no lift: a lift on a move that repeats in the air is a climb). | A tall crescent overhead, tip at the apex. |
| Down (ground) | Low. | A sliding low stab. | A low sweep, tip far out. |
| Air | Rise / overhead / behind. | A whirl (hits on both sides), a rising thrust, a back-kick thrust. | A wide forward crescent, an overhead crescent, a back crescent. |
| Pogo (down-air) | Straight down slash. | A diagonal plunge that dives forward and down, and bounces. | A sweeping crescent beneath, wide, and bounces. |
| Smashes | Forward, up, down. | Forward: a charged lunge across the floor. Up: a pillar thrust. Down: a stab to both sides. | Forward: a big tipper arc (the kill). Up: a crescent column. Down: a two-sided sweep. |

The tip: in Stance III a move's farthest volume deals more and launches
harder than its root. A victim takes the first-authored volume that reaches it
(`StrikeRank`, as `authoring::tipper` builds one), so each Crescent strike
lists its tip first.

## Stages

- **S1 — rebinds and the items.** `EquipmentRow.rebinds`, applied in the
  repertoire fold; Stance I/II/III items in one exclusive slot; inventory room
  made by removing items that do nothing interactive if the grid is full.
  Core tests: a worn row rebinds, a missing target is inert, taking it off
  restores.
  *Landed:* `EquipmentRow.rebinds` (verb → move id), folded first in
  `effective_repertoire` (`stance_kit`) so a held weapon still releases the
  melee verbs over it; a rebinding to a move the table lacks is skipped.
  The placeholder Axe and Whip Stance cells became Stance II and III (dialog
  ids `lungestance`, `crescentstance`), so no cell was removed; all three are
  in the starting bag. Core test `a_worn_stance_rebinds_the_kit_and_gives_it_back`.
- **S2 — Stance II authored.** Its moves on existing rows (`slash`,
  `attack_side`, `dash_attack`, `air_*`, `meteor`); app witness that each verb
  plays the stance's move and the pogo bounces.
- **S3 — Stance III authored**, with the tip; the same witnesses.
  *Landed (S2, S3):* `lunge_*` and `crescent_*` in `player_robot.ron`, on the
  published rows (`dash_attack`, `slide`, `slash`, `jab`, `attack_*`, `air_*`,
  `smash_*`). A Lunge carry is `SetSide` with `HoldVelocity`, a step ground
  friction does not eat; the strings are named successors (`lunge_thrust` →
  `lunge_long`, `crescent_arc` → `crescent_return` → `crescent_sweep`).
  Witnesses `ambition_app/tests/combat_stances.rs`: in each stance, attack, up,
  down, forward smash and the pogo play the stance's own move, every pogo
  bounces off an orb, the Lunge thrust carries the body where the Edge's swing
  does not, and the Crescent string is three steps.
- **S4 — the look.** Bespoke rows in the v3 generator for the lunge, whirl,
  plunge and crescents (as the dash's smear was), republished; captures of
  each stance. *Landed 2026-11-27.* Four keyed strike rows in
  `player_robot_v3_strikes.py`: `lunge_thrust` and `lunge_plunge` draw the
  dash smear while they carry (`STRIKE_SMEAR` on the `dash_fx` channel), and
  `crescent_arc` and `crescent_sweep` are wide straight-arm arcs. The whirl
  spins the sheet on the move clock (`sprite_spin_hz: 8`) instead of a row.
  The Lunge and Crescent moves play these clips; the rest of each stance
  borrows the Edge's published rows.
- **S5 — feel pass.** Play each stance headless against the dense-melee room
  and tune reach, frames and the pogo's bounce. *Done 2026-11-27, no retune.*
  Each stance fought `dense_melee_pit` for 1800 ticks under one scripted
  routine (a jump with pogo presses, tilts every 16 ticks, a forward smash
  every 120): Edge dealt 2169 (93 kills, pogo 16/60 presses bounced, 71 px
  travelled), Lunge 11617 (757, 22/60, 2073 px), Crescent 2962 (120, 15/60,
  73 px). None lost more than 1 hp. Per-hit numbers match (damage 1–2,
  knockback 55–110, actives 0.07–0.12 s); the Lunge's lead is ground: each
  thrust carries it past the foes it reached into fresh ones, which is the
  stance's role. The pogo bounces at the same rate in every stance. A
  retune waits for hands-on play.
