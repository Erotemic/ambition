# `ambition_platformer2d_session` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_session** — Session construction, admitted lifecycle operations, restoration, and retirement.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`checkpoint`](src/checkpoint.rs) | Restoring a session to its checkpoint: the startup resume and the death/retry reset. |
| [`death`](src/death.rs) | Participant death interlude and level-reset policy. |
| [`durable_horizon`](src/durable_horizon.rs) | Durable persistence for occurrence, custody, and minted-item checkpoint state. |
| [`initial_body`](src/initial_body.rs) | Session startup body selection and its prepared resource bank. |
| [`join`](src/join.rs) | Start admits an unjoined participant and constructs its home body. |
| [`lifecycle_commit`](src/lifecycle_commit.rs) | Confirmed-frame lifecycle commitment. |
| [`match_activation`](src/match_activation.rs) | Build, bind, and release the cast of an admitted match. |
| [`match_lifecycle`](src/match_lifecycle.rs) | Installation of match construction and lifecycle systems. |
| [`participants`](src/participants.rs) | Admitted session participants, independent of devices and controlled bodies. |
| [`reset`](src/reset/mod.rs) | New Game and the room-replay vocabulary. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback state owned by the lifecycle coordinator. |
| [`rooms`](src/rooms/mod.rs) | Data-driven sandbox room-set and loading-zone graph. |
| [`setup`](src/setup.rs) | Initial session bodies from prepared or live session inputs. |
| [`snapshot_impls`](src/snapshot_impls.rs) | The lifecycle snapshot encoding. |
| [`teardown`](src/teardown.rs) | Re-establish the process-global mirrors of live-session state at BOTH edges of a gameplay session. |

_15 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
