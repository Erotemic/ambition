# `ambition_match` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_match** — The versus match, prepared: a roster of participants, the stage rules, and the immutable plan a kernel activates without a lookup.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`lifetime`](src/lifetime.rs) | What a match created ends when the match does. |
| [`live_match_clock`](src/live_match_clock.rs) | HOW LONG THIS MATCH HAS ACTUALLY BEEN FOUGHT. |
| [`match_rules`](src/match_rules.rs) | The `match_rules` authored-content schema: the rules a game's matches play under, as data in its pack. |
| [`prepared`](src/prepared.rs) | Match preparation resolves all fallible character, brain, and control-authority questions before construction. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declarations for match identity, outcomes, and elapsed time. |
| [`seating`](src/seating.rs) | Match-seat binding and the rollback-safe receipt for a live prepared match. |
| [`settlement`](src/settlement.rs) | Rollback-safe match settlement state. |
| [`snapshot_impls`](src/snapshot_impls.rs) | Rollback wire format for the match receipt and the per-body seat. |
| [`staging`](src/staging.rs) | Three ways to stage a cast, one projection. |

_9 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
