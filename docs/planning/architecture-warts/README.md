# Local architecture wart index

This page lists small and medium ownership defects that the named architecture
campaigns do not cover. It is not a second queue. It is a re-measurement index.

Before you implement a row, re-check it against the current tree. When the work
is selected, put the executable task in [`queue.md`](../queue.md) and the durable
design in its owner page. Remove the row here when another live page owns it or
when the defect is fixed. The
[AUTHORITY-POLISH](../queue.md#authority-polish--one-owner-per-mechanical-fact-and-no-mirror-in-the-rollback-kernel---done-2026-10-09)
receipt closed the campaign on a fresh census (2026-10-09); an open row here is
fixed when a queue slice reaches it.

The index looks for four failure shapes:

1. authored controls that the runtime does not consume;
2. derived, diagnostic or presentation state promoted to rollback authority;
3. fields or components owned by the wrong subsystem;
4. fallback policy that silently invents capabilities or behaviour.

`STRUCTURAL` means source shows the ownership problem, but the repair can depend
on intended product behaviour.

## Index

| ID | Status | Area | Current defect | Smallest sound direction |
| --- | --- | --- | --- | --- |
| W017 | STRUCTURAL | actor tuning | `ActorTuning` (`crates/ambition_combat/src/actor_tuning.rs`) is one construction record with several owners: reusable body facts, placement/session facts and presentation facts. Its runtime half is already split out: the live brain policy is `ActorPolicy`, and `ActorConfig` is construction input with no runtime writer. | Split by owner when a real consumer boundary exists. Do not add unrelated fields to this record, and do not put a runtime-written field back in it. |

## Owned elsewhere

Do not duplicate these here:

- Actor resources (Mana, the Smash Limit) are owned by
  [`engine/composable-actor-resources.md`](../engine/composable-actor-resources.md).
- Rollback-presence and deterministic-ingress defects stay in their owner
  documents and decision rows.

## Re-measurement rule

Search the current source for the named field and its readers before you act.
For an "unused" claim, check direct field reads and adapters that translate the
containing type. For an authoring claim, check shipped content as well as Rust.
For rollback-state cleanup, verify the real restore and checksum contract before
you delete a registration.
