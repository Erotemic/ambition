# Combat stances

**Status:** asked by Jon 2026-11-27; S1–S5 below, in order. Queue:
COMBAT-STANCES.

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
| Up | Overhead. | A rising corkscrew thrust (some lift in the air). | A tall crescent overhead, tip at the apex. |
| Down (ground) | Low. | A sliding low stab. | A low sweep, tip far out. |
| Air | Rise / overhead / behind. | A whirl (hits on both sides), a rising thrust, a back-kick thrust. | A wide forward crescent, an overhead crescent, a back crescent. |
| Pogo (down-air) | Straight down slash. | A diagonal plunge that dives forward and down, and bounces. | A sweeping crescent beneath, wide, and bounces. |
| Smashes | Forward, up, down. | Forward: a charged lunge across the floor. Up: a pillar thrust. Down: a stab to both sides. | Forward: a big tipper arc (the kill). Up: a crescent column. Down: a two-sided sweep. |

The tip: in Stance III a move's farthest volume deals more and launches
harder than its root (a hit takes the strongest volume it overlaps).

## Stages

- **S1 — rebinds and the items.** `EquipmentRow.rebinds`, applied in the
  repertoire fold; Stance I/II/III items in one exclusive slot; inventory room
  made by removing items that do nothing interactive if the grid is full.
  Core tests: a worn row rebinds, a missing target is inert, taking it off
  restores.
- **S2 — Stance II authored.** Its moves on existing rows (`slash`,
  `attack_side`, `dash_attack`, `air_*`, `meteor`); app witness that each verb
  plays the stance's move and the pogo bounces.
- **S3 — Stance III authored**, with the tip; the same witnesses.
- **S4 — the look.** Bespoke rows in the v3 generator for the lunge, whirl,
  plunge and crescents (as the dash's smear was), republished; captures of
  each stance.
- **S5 — feel pass.** Play each stance headless against the dense-melee room
  and tune reach, frames and the pogo's bounce.
