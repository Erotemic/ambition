# Immutable content / transactional construction — remaining work

> **Verified against `7eea4f27` (2026-08-18).** Prepared content, structured
> diagnostics, explicit provenance, the construction registry/plan, migrated room
> construction families, removal of the legacy construction exemption,
> rollback-envelope coverage, and the first external consumer slice are
> implemented. The 2,400-line campaign record is archived at
> [`../../archive/planning-superseded/2026-08-13/engine/immutable-content-and-transactional-construction.md`](../../archive/planning-superseded/2026-08-13/engine/immutable-content-and-transactional-construction.md).

## Remaining construction work

- ▢ **Prove real snapshot reconstruction (the old operation 5).** There is still
  no production cross-room snapshot caller exercising source-snapshot selection,
  decode/compatibility rejection before mutation, rollback entity identity and
  remapping, restoration of non-room authoritative state, and atomic commit.
  Room-transition use of `RoomConstructionPlan::apply_to_world` does not by itself
  prove this operation.

- ▢ **Prove possession → transition → carried body end to end.** The controlled
  subject/carry plumbing is implemented; the full orchestration exercise is not.

- ▢ **Prove corrected-input cancellation and peer-coordinated lifecycle commit
  when external/P2P rollback becomes real.** Local sync testing cannot
  mispredict, so this belongs to the real external-netplay trigger rather than a
  synthetic local ritual.

## Remaining external-consumer proof

`fixtures/external_consumer` already proves an independent workspace can prepare
content, run headlessly, route a shell, stage a character/enemy, and traverse an
in-room transition through the umbrella. The remaining useful proof is:

- ▢ run the visible external consumer on a machine with a display;
- ▢ measure first-room workflow and deliberate-error diagnostics rather than only
  describing them qualitatively;
- ▢ add a queryable readiness/last-failure convenience API if a consumer actually
  benefits from it;
- ▢ exercise construction/content authoring from a second meaningfully different
  consumer before freezing a broad public prefab/content API.

## Exit

Prepared content remains immutable after activation; construction is planned
before mutation and commits atomically; snapshot reconstruction has a real
behavioral proof; public construction APIs are justified by multiple consumers
rather than by the historical migration campaign.
