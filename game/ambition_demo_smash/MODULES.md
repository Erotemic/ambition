# ambition_demo_smash

The stocks demo: a platform fighter where the damage meter never kills and the
WORLD does. Jon, 2026-07-31: *"stocks are more important. maybe start the smash
demo. versus can be a generic fighter demo and test things smash doesn't."*

| module | what it owns |
|---|---|
| `lib.rs` | the mode tag, the stocks roster, the respawn placement, the victory banner |

## Why it exists, in order

1. **First real consumer of the S4 stocks loop.** `ambition_combat::stocks` owns
   the COUNT and deliberately refuses to place a body or decide what a match
   ending means — those need a stage and a scoreboard. This crate is the other
   side of that split, and the split is only real once something stands there.
2. **The E9 oracle for a stocks game.** `ambition` + `bevy`, nothing else. It has
   already paid for itself: writing it required
   `ambition::actors::character_runtime::MatchParticipantRoster` — reaching
   through the crate re-export into an implementation module — which is exactly
   the leak class the SDK-purity contract exists to catch. The match vocabulary
   is curated at `ambition::actor` now because this crate could not be written
   without it.

## What it deliberately does NOT do

Kill anybody. A stocks fighter is `DeathPolicy::Unbounded`, so its meter climbs
past 100% and never kills it; the engine's blast-zone gate does the killing and
writes `BodyKnockedOut` from the same `RulesetOwnsDeath` arm that already decided
a match rather than the world owns the death. This crate answers only the two
questions the engine refuses to guess: WHERE a respawn lands, and WHAT a match
ending says.
