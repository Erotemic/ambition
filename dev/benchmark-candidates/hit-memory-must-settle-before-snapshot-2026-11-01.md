# Hit memory must settle before the snapshot

`PerceptionMemory` can restore correctly and still differ on replay if its
input crosses a tick boundary outside rollback state.

`ResolvedBodyHit` is cleared on load. A consumer in the next tick's Observe
phase loses the hit when replay starts after its producer. Consume the hit in
`CombatSet::Settle`, after damage and before the snapshot. The next brain
decision then reads memory that the snapshot can restore.

The component localizer in
`game/ambition_app/tests/rollback_exit_oracle.rs` can distinguish a bad restore
from a different replay. It found the changed memory at frame 30. After the
phase repair, it compared 595 restored frames without a divergence. The real
contact, possession, boss-technique and checkpoint replay tests cover the fix.

Keep the bounded perception builders and their allocation reuse. They were
not the cause. Do not add a second message history to repair a same-tick
consequence that can settle at its existing owner.
