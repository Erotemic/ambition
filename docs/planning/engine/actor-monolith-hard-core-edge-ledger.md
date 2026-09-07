# Actor-monolith hard-core edge ledger

**Purpose:** this is the required P5 deliverable after the four executable peel
packets in [`actor-monolith-work-frontier.md`](actor-monolith-work-frontier.md).

**Do not use this baseline seed as permission to edit the hard core.** P1-P4 will
change the graph. Refresh every row from the post-P4 HEAD first.

## Required procedure after P4

1. Run:

   ```bash
   python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 200
   ```

2. Write the measured SCC at the top of this file.
3. Enumerate every production source reference whose source and destination are
   both in that SCC. Tests and `#[cfg(test)]` references do not count.
4. Group references only when they have the same source symbol, destination
   authority and proposed disposition. Do not group merely because they share a
   module pair.
5. Fill every column below. `TBD` is a hard stop for implementation.
6. Write the resulting package map and one exact P6 packet.

Allowed dispositions:

```text
KEEP_DOWNWARD
MOVE_TYPE
MOVE_SYSTEM
PUBLISH_SET
SPLIT_RESOURCE
GROUP_PACKAGE
DELETE_DEAD
```

Edge classes:

```text
DATA
POLICY
SCHEDULING
CONSTRUCTION
LIFETIME
```

## Baseline seed at `625fa79af45e`

These rows are **investigation seeds** from the current source. Re-find exact
post-P4 references and replace this table; do not merely copy it forward.

| Edge | Current source family | Current dependency | Class to verify | Decision required |
| --- | --- | --- | --- | --- |
| `abilities -> control` | possession/traversal systems | `body_driving_seat`, `controlled_frame_down` | DATA/POLICY | Are these actor-control primitives part of one control/ability package, or should the accepted-control facts move lower? |
| `control -> abilities` | `control/authority.rs`, `control/input_systems.rs` | `PossessionState`, ascend/descend possession helpers | DATA/POLICY | Is possession a control mode owned with control, or an optional ability contributing claims/intent through a lower seam? |
| `features -> control` | chest/interact/dormancy roads | `ActingParticipant`, driving/control projection | DATA | Keep only if feature verbs consume a public actor-control fact rather than mutate control policy. |
| `control -> features` | input/animation road | `features::advance_body_anim_overlays` and any remaining feature mutation | POLICY | Likely wrong direction; decide whether presentation/animation mutation moves out of `features` or is published as an intent. |
| `features -> world` | feature overlay/construction verification | world overlay/set/verification vocabulary | CONSTRUCTION/SCHEDULING | Separate legitimate world contribution data from feature-owned installation or construction orchestration. |
| `world -> features` | overlay/reconstitution/stage roads | `WorldPrepSchedulePlugin`, `SpawnedThisAttempt`, feature construction plan/receipt | SCHEDULING/CONSTRUCTION | Replace private installation dependencies with public sets; decide construction receipt owner. |
| `items -> session` | item persistence | `SaveRestored` / durable horizon | LIFETIME | Decide whether item persistence consumes a generic persistence milestone or belongs in a session-owned persistence adapter. |
| `session -> items` | durable horizon / teardown | item persistence installers, `MintedItemBaseline` | LIFETIME/SCHEDULING | Session should order public persistence milestones, not item private systems; baseline lifetime must have one owner. |
| `world -> session` | room/reconstitution roads | lifecycle/session facts remaining after P3 | LIFETIME | Re-measure after moving lifecycle-commit vocabulary; this edge may shrink substantially. |
| `session -> world` | setup/reset | `ActiveContentBinding`, physics/world setup remaining after P4 | LIFETIME/CONSTRUCTION | Decide whether active content binding is session state, world state or provider binding. |
| `abilities -> features` | puppy-slug/runtime summon roads | `features::spawn_runtime_minion` | CONSTRUCTION | Ability should emit a generic spawn request or consume a lower spawn seam, not call feature orchestration. |
| `features -> items` | feature/item adapters | item state/installers | POLICY/SCHEDULING | Identify whether each is legitimate feature consumption or residual installer ownership. |
| `items -> abilities` | item-granted actions | ability installers/state | POLICY/SCHEDULING | Prefer capability contribution through a public ability/action seam; avoid item crate installing private ability systems. |
| `session -> abilities` | reset/teardown | possession/ally transient state | LIFETIME | Move retraction to capability-owned lifetime installer if the session merely signals the boundary. |
| `session -> features` | content staging/reset | feature staging/construction state | CONSTRUCTION/LIFETIME | Decide whether this is unavoidable session composition or residual feature catch-all ownership. |

## Post-P4 measured SCC

Replace after P4:

```text
TBD
```

## Post-P4 complete edge ledger

Replace after P4. Every row is mandatory.

| Edge | Source file + symbol | Destination symbol | Class | Semantic owner | Disposition | New path/API | Production poison |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TBD | TBD | TBD | TBD | TBD | TBD | TBD | TBD |

## Package map

Write after the ledger is complete:

```text
TBD
```

For each proposed package, state:

```text
owned authoritative state
owned mutation systems
installed systems/sets
rollback declarations
lifetime boundary
lower dependencies
optional capabilities it must NOT name
```

## P6 packet

Do not write generic prose such as “split features from world.” Write the same
shape as P1-P4:

```text
CUT:
OWNER DECISION:
FILES TO CREATE/MOVE/DELETE:
SYMBOLS TO MOVE:
CALLERS TO UPDATE:
ROLLBACK/LIFETIME CHANGES:
FORBIDDEN END STATE:
PRODUCTION TESTS:
SOURCE GUARDS:
EXPECTED SCC RECEIPT:
STOP IF:
```

Until every field is filled, P6 is not READY.
