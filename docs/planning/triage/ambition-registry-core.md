# Registry protocol, identity and deliberate duplicate policy

**Status:** bounded owner contract for `ambition_registry_core`. It is not a
campaign to make every registry adopt the core. Workspace policies in
`tests/ambition_workspace_policy/policies/engine.toml` cite this page.

## Finding

Each `*Registry` type decides on its own what counts as identity, whether a
second registration of a key is a no-op, a refusal or a replacement, what enters
a fingerprint, and whether a conflict leaves the old entry unchanged. The shared
core gives that decision one vocabulary; it does not choose the policy for a
domain.

## What the core owns

`crates/ambition_registry_core/src/lib.rs` provides stable registration
metadata, required-field checks, equality-based new/idempotent/conflict
classification, and canonical row/section framing. `ConstructionRegistry`,
`RollbackRegistry` and `PlacementLoweringRegistry` use it.

- The core stays dependency-free: no `ambition_*` crate and no Bevy. It is not
  a generic Bevy registry or service locator.
- Domains keep keys, storage, semantic validation, provider policy, executable
  behavior and lifecycle.
- Canonical row framing rejects unsupported separators instead of escaping
  them. Changing that grammar changes fingerprint bytes and needs a deliberate
  compatibility change.
- The core does not own the complete prepared-content fingerprint. Deterministic
  row order is the domain's responsibility.

## Rules for a registry review

Identify four things: the key's scope, the value's meaning, the duplicate
admission policy, and the stable metadata in diagnostics and fingerprints. Then
trace real production registration and lookup callers. A unit test does not
prove that any game installs or reads the registry. Do not infer a registry's
kind from its name or return type.

| Kind | Typical policy | What must stay explicit |
| --- | --- | --- |
| Immutable declaration table | Reject duplicate keys, or a narrowly defined idempotence | Owner/source/schema, preparation freeze, conflict leaves the old entry unchanged |
| Layered content override | Deliberate replacement with deterministic precedence | Which layer replaces which, provenance, revision boundary |
| Live identity-to-entity index | Replacement on an authorized new live entity | Lifetime/generation, stale-entry removal, scope |
| Derived cache | Replace or invalidate under a key/revision contract | Rebuild inputs, cache-key completeness, no semantic authority |
| Closed implementation dispatch | Ordinary typed code | No registry only to invert a dependency |
| Open provider extension | Explicit registration during composition | One owner, collision policy, validation, ordering, testable absence |

- A function-valued registry can still refuse duplicate keys. "A key may be
  declared once per installed catalog" needs no function comparison.
  `TechniqueSupport::declare` (`crates/ambition_entity_catalog/src/lib.rs`)
  does this and replaces `ParamSchemaRegistry`, which accepted unknown keys
  and overwrote duplicates.
- Idempotent installation needs a stated registration token, owner/source/
  revision and installer lifecycle. Equal metadata does not prove equal
  behavior. Hot-reload replacement is a separately named operation, not the
  accidental meaning of ordinary registration.
- Do not hash function addresses. `fn_addr_eq` (used for local equality in
  `PlacementLoweringRegistry`, `crates/ambition_platformer2d_world/src/placements.rs`)
  can give false negatives and can merge distinct
  functions; it is not a portable identity. Do not remove a local pointer check
  without covering its duplicate-registration behavior.
- `PreparedCharacterRegistry` keeps declaration admission and prepared/
  hot-reload replacement as distinct operations.
- `FrontendAudioRegistry` and the banter tables state override semantics in
  source. Their precedence is product layering and needs its own ruling.

## Evidence command

```bash
python3 scripts/measure_registry_core_adoption.py
```

ADOPTED/JUSTIFIED/UNREFERENCED measure references, not policy. JUSTIFIED does
not prove that the prose argument is valid, and UNREFERENCED is an upper bound
on work, not a list of it. Read each `register` function before you treat a row
as a task.

## Owner

None. The core crate is maintained by whoever changes a registry that uses it.

## Trigger to promote

Promote one named registry when its production role and a repeated invariant
are known. A migration must keep (or deliberately change) duplicate handling,
transactional rejection, ordering, diagnostic provenance, canonical bytes and
lookup behavior. Test it with a real caller, a conflict fixture, reversed
registration order, and a freeze/reload case where that applies. Stop when
sharing the helper needs weaker diagnostics or broader dependencies than the
invariant warrants.

Non-goals: a universal `Registry<K,V>`, global discovery, `Any`/`TypeId`
service lookup, executable provider callbacks for closed domains, forced policy
uniformity, schema inference from functions, a workspace adoption count, or a
new registry for one consumer.
