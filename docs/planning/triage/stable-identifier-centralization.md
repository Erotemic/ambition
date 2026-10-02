# Stable identifier centralization - semantic scope before shared syntax

**Status:** triage. `tracks.md` gates it on concrete identity families that
share actual operations.

## Finding

Identifier newtypes repeat the same operations (construction, `as_str`,
`Display`, conversions, serde, validation, ordering). Shared machinery could
prevent drift, but two identifiers with the same representation need not share
equality, validation, persistence or retirement rules. The problem to solve is
consistency, not boilerplate.

Current shape:

- The syntax consolidation is done. There is one `string_id!` definition
  (`ambition_load/src/id.rs`); other crates `use ambition_load::string_id`.
- A second type generator, `digest_type!`
  (`ambition_platformer2d_runtime/src/content_identity.rs`), mints
  `[u8; 32]` digests (`ContentFingerprint`, `SnapshotSchemaFingerprint`). It
  shares no policy with `string_id!`. This supports the rule above.
- `sfx_ids!` (`ambition_sfx`) and `fx_ids!` (`ambition_vfx`) mint values, not
  types, and generate a `NAMED` census from their own declarations. That is the
  idiomatic shape for a derived census when one module owns the whole
  population.
- The authority axis (may a value enter what two peers compare?) has a
  mechanical owner: `RollbackEntryKind::feeds_peer_checksum`,
  `in_peer_schema_identity` and `HOST_LOCAL_IDENTITIES` (`id_peer_audit.rs`).
  The mint site decides it, not the type: `LoadId` (host-local) and
  `LoadWorkId` (content-derived) share one `string_id!` policy and have opposite
  answers. So a policy inventory keyed on the type cannot answer this axis.
  `LoadPresentationOwnerId` and `ShellHoldId` are host-local carriers that
  `HOST_LOCAL_IDENTITIES` does not list. That is not a defect today, because
  neither is rollback-registered.

## Rules

- Explicit, locally readable newtypes stay valid. Do not mass-change
  constructor behavior because a macro exists.
- For untrusted authored data, validation produces source-local errors. A
  constructor panic is acceptable only for an internal trusted invariant. Record
  which boundary a constructor serves.
- The policy of an id is visible at its declaration, and `rg` finds where its
  construction and validation come from.
- Use typed or generated ids to make bad references unrepresentable only when
  the owning pipeline already exposes stable generated symbols. Do not open a
  standalone symbol-generation campaign to replace strings, and do not add an
  always-on boot census of declared ids.
- Non-goals: a general utilities crate, unifying ids because they wrap strings,
  a universal delimiter convention, a procedural derive before the policy
  inventory exists, one crate for all identifier types.

## Evidence command

```bash
rg -n 'macro_rules! (string_id|digest_type|sfx_ids|fx_ids)' crates game
rg -n 'string_id!\(' crates game --type rust
rg -n 'HOST_LOCAL_IDENTITIES' game crates
```

## Owner

None.

## Trigger to promote

Concrete identity families that share actual operations. Then classify the
explicit newtypes on the remaining axes (representation, validation, stability,
construction, serialization, interchange, error policy), group only exact
policy matches, and pilot one option: conventions plus explicit code, a small
declarative macro whose invocation shows every policy choice, or shared
validation primitives with explicit types. Revert the pilot if it mainly hides
straightforward code.
