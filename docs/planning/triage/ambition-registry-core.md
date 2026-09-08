# Registry protocol, identity and deliberate duplicate policy

`ambition_registry_core` already exists. This is a bounded owner/triage contract,
not a campaign to make every map adopt it. The architecture review corrects the
old argument that a function-valued registry cannot honestly reject duplicates.
See [finding F7](../engine/architecture-review-findings.md) and
[packet A11](../engine/actor-monolith-work-frontier.md).

## What the shared core owns

`crates/ambition_registry_core/src/lib.rs` provides stable registration metadata,
required-field checks, equality-based new/idempotent/conflict classification and
canonical row/section framing. Domains retain keys, storage, semantic validation,
provider policy, executable behavior and lifecycle. The core remains
low-dependency; it is not a generic Bevy registry or service locator.

Canonical row framing currently rejects unsupported separators rather than
escaping them. Changing that grammar changes downstream fingerprint bytes and
requires a deliberate compatibility change. Canonical sections preserve the
caller-defined row grammar/order; deterministic order is still the domain's
responsibility. The core does not own the complete prepared-content fingerprint.

## Four independent questions per registry

A review must identify the key's scope, the value's meaning, duplicate admission
policy, and the stable metadata included in diagnostics/fingerprints. Then trace
actual production registration and lookup customers. A healthy unit test does not
establish that any game installs or reads the registry.

| Kind | Typical policy | What must remain explicit |
| --- | --- | --- |
| Immutable declaration table | Reject duplicate keys, or support narrowly defined idempotence | Owner/source/schema, preparation freeze, conflict leaves old entry unchanged |
| Layered content override | Deliberate replacement with deterministic precedence | Which layer can replace which, diagnostics/provenance, active revision boundary |
| Live identity-to-entity index | Replacement on an authorized new live entity | Lifetime/generation, stale entry removal, scope and live-target validity |
| Derived cache | Replace or invalidate under a key/revision contract | Rebuild inputs, cache-key completeness, no independent semantic authority |
| Closed implementation dispatch | Prefer ordinary typed code | No new registry merely to invert a dependency or lower an SCC |
| Genuinely open provider extension | Explicit registration during composition | One installed owner, collision policy, validation, ordering and testable absence |

Do not infer a registry's kind from its suffix or function return type.

## Function equality is not required to reject duplicate keys

`ParamSchemaRegistry` stores validators as function pointers. Its current comment
concludes that unavailable stable behavior equality rules out an honest Conflict
case and therefore replacement is necessary. That inference is incorrect.

The default can simply be: **a key may be declared once per installed catalog**.
An occupied key returns a duplicate-registration error without comparing either
function. This is the same useful policy already available for closure-valued
room content staging. Lack of a meaningful Idempotent case does not remove the
Duplicate case.

Where idempotent installation is necessary, specify the registration token,
owner/source/revision and installer lifecycle that justify it. Equal metadata is
not proof of equal executable behavior. A separate explicitly named replacement
operation may be appropriate for a hot-reload boundary; it should not be the
accidental semantics of ordinary registration.

A11 also closes the more immediate problem: the validator registry currently has
no production register/validate callers, and an absent validator accepts an
unknown technique key. Couple installed support declarations to real handler
installation, distinguish unknown/uninstalled/parameterless cases and validate
all effect-reference locations before content activation. Do not add another
unconnected table of supported names.

## Local pointer checks and portable identity are different

`PlacementLoweringRegistry` currently includes `fn_addr_eq` in local equality;
`RoomContentStagingRegistry` rejects repeated sources without closure equality.
Neither pattern is a universal registry policy. Rust documents that function
address comparisons can have false negatives and can merge distinct source
functions. They do not provide a portable source-function identity or a proof of
cross-build compatibility. Even where equal compatible function pointers imply
equivalent calls, equal metadata alone does not.

Reference: https://doc.rust-lang.org/std/ptr/fn.fn_addr_eq.html

Do not hash function addresses. Do not remove a local pointer check without
covering its duplicate-registration behavior. Prefer explicit registration
ownership and freeze/revision policy when durable identity is required.

## Current examples to preserve or revisit

| Surface | Existing role / evidence | Consequence |
| --- | --- | --- |
| `ConstructionRegistry` | Typed recipe/relation metadata in shared construction | Keep validation/fingerprint grammar; do not turn metadata into an executable recipe service |
| `RollbackRegistry` | Backend-neutral domain state registration plus wire identity | Preserve domain semantics and wire collision checks beyond common metadata |
| `PlacementLoweringRegistry` | Open, typed placement lowering in world | A3 relocates actor-specific adapters, not all providers into a universal registry |
| `RoomContentStagingRegistry` | Explicit source registration, sealed stagers, duplicate refusal | Retain duplicate refusal without inventing closure equality |
| `EncounterRegistry` | Index from semantic encounter ID to live entity | Authorized replacement can be correct; immutable-declaration refusal would be the wrong rule |
| `PreparedCharacterRegistry` | Declaration admission versus intentional prepared/hot-reload replacement | Keep these operations distinct; a blanket reject/replace policy is insufficient |
| `MovePrefabRegistry` | Expansion API must establish a production customer before wider investment | Recheck current install/expand callers; do not mistake tests for adoption |
| `FrontendAudioRegistry` / banter tables | Explicit override semantics in current source | Precedence/product layering requires its own ruling; naming registry_core is not acceptance |
| `ParamSchemaRegistry` | Unwired validator surface, permissive unknown lookup, replacement rationale defect | A11; do not classify the rationale as settled solely because source comments explain it |

Find definitions/callers rather than copying return signatures into a permanent
inventory. Source paths and the named snapshot are the receipt; old counts and
pilot history remain in Git.

⛔ **AND THE DERIVATION SCRIPT WENT WITH THE COLUMN, 2026-09-08.**
`scripts/registry_register_returns.py` and its test existed because an earlier <!-- cite-ok: names a DELETED file, which is this row's whole point -->
version of the table above carried a hand-typed *verdict* column that had gone
stale in three of three rows re-read; the script derived that column from
`register`'s actual signature so the copy could not rot. Deleting the column is
the stronger form of the same fix — there is no copy left to police — so the
script was retired rather than taught the new table shape, which would have
rebuilt the inventory this section declines. Read it back with
`git show 542481fae:scripts/registry_register_returns.py` if a future ruling
wants the derivation; do not resurrect it as a standing check without a consumer
for its output.

## Measurement and expansion rule

```bash
python3 scripts/measure_registry_core_adoption.py
```

The script's ADOPTED/JUSTIFIED/UNREFERENCED buckets measure manifest/code/prose
references. In particular, JUSTIFIED does not prove that the prose argument is
valid: F7 is a counterexample. UNREFERENCED does not prove absence of a documented
policy. Inline/comment filtering is textual, not semantic Rust analysis.

Promote only a named registry whose production role and repeated invariant are
known. A per-change review obligation can be justified by correctness risk; it
does not require reconstructing a historical registry growth rate. Do not impose
a workspace adoption count merely because the census can count names.

A migration must preserve or intentionally change duplicate handling,
transactional rejection, ordering, diagnostic provenance, canonical bytes and
lookup behavior. Use a real caller, a conflict fixture, reversed registration
order, and a freeze/reload case where applicable. Stop when sharing the helper
requires weaker diagnostics or broader dependencies than the invariant warrants.

## Non-goals

No universal `Registry<K,V>`, global discovery, `Any`/TypeId service lookup,
executable provider callbacks for closed domains, shared complete engine context,
forced policy uniformity, automatic schema inference from functions, or new
registry for a single consumer just to remove a direct import.
