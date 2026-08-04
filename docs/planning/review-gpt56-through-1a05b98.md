# GPT 5.6 review of work through `1a05b98` — recorded verbatim

Pasted by Jon 2026-08-04, mid-run, while the Mary-O LDtk migration was in
progress. **Recorded before triage**, per the standing rule that an answer or a
review not written down verbatim is one that gets half-remembered.

Triage lives in the 72h queue's `G1 PICK 10` row; this file is the source text.

---

# Review of work through `1a05b98`

This review covers the current head relative to the previously reviewed `7875f35` state. The Mary-O LDtk conversion is understood to be an active migration; this review does not treat incomplete migration stages as claimed completion.

## 1. The content-pack fingerprint omits lowered-only runtime content

The content compiler currently builds canonical source identity primarily from values emitted through:

```rust
out.define(...)
```

`PreparedContentPack::canonical_bytes()` fingerprints schemas, declared content identities, assets, capabilities, and references. It does not include the actual lowered artifacts or a canonical fingerprint of their runtime values.

This becomes incorrect for schemas that lower runtime content without defining corresponding canonical rows.

The encounter-wave schema is one example. It parses and lowers the `EncounterWaveBook`, but it does not emit each wave through `out.define(...)`. Consequently, edits to runtime-significant fields such as:

* enemy composition;
* delays;
* spawn timing;
* wave ordering;
* other authored parameters;

can change the lowered runtime artifact without changing the prepared pack fingerprint, provided the surrounding declared IDs remain unchanged.

This undermines the purposes assigned to the fingerprint:

* cache invalidation;
* package identity;
* session compatibility;
* content mismatch detection.

Two peers could carry behaviorally different encounter content while presenting the same content-pack identity.

### Required correction

Every lowered runtime artifact must contribute canonical identity to the prepared-pack fingerprint.

Possible designs include:

1. Require each schema to emit canonical runtime bytes or a stable runtime hash.
2. Have aggregation emit a canonical representation of its merged artifact.
3. Include canonical prepared-source fingerprints in `PreparedContentPack::canonical_bytes()`.

Do not serialize arbitrary `HashMap` iteration order into the fingerprint. Maps and sets must be canonicalized.

A narrow semantic check should establish that changing a wave delay or enemy row, while retaining the same content IDs, changes the final pack fingerprint.

This is the highest-priority issue because it compromises compatibility and cache identity rather than merely presentation.

## 2. Roster topology can still lose the race with local session construction

The ownership correction is real: caller-created sync sessions are no longer confused with sessions owned by the local maintainer.

The separate topology problem remains.

The local session maintainer can freeze input topology from connected devices before the roster-aware path publishes the topology of the decided match.

Those two inputs are not equivalent:

* a keyboard participant may not correspond to a gamepad entity;
* connected spare controllers need not be participants;
* CPU participants have no connected device;
* roster seat count can differ from connected-device count.

The source now acknowledges that restarting after discovering the mismatch is not a valid repair because seat-handle bindings may already have been constructed against the original topology.

### Required correction

Treat roster commitment, topology publication, and session construction as one ordered transition:

1. commit the decided participant roster;
2. derive and freeze its input topology;
3. calculate complete session settings from that topology;
4. install the rollback session.

The session owner should retain the complete settings used to start the session, not merely a high-level policy value.

Do not revive the restart workaround. The session must be constructed from the correct topology initially.

## 3. Power-up items remain marked as "emerging" forever

`WorldItem` now contains:

```rust
pub emerging: bool
```

Mary-O power-ups are spawned with:

```rust
emerging: true
```

The renderer uses this flag to place an emerging item behind world geometry.

The motion model independently knows when emergence has completed through `ItemMotion::emerging()`, but no current system transfers that result back to `WorldItem.emerging`. There is no assignment setting the field to `false`, despite comments referring to a cleanup path.

The result is that an item can finish rising and begin ordinary movement while its presentation remains permanently in the behind-world emergence layer.

### Required correction

Prefer removing the duplicate mutable fact.

Presentation can derive emergence directly from `ItemMotion`:

```text
item is emerging when its motion exists and motion.emerging() is true
```

If `WorldItem.emerging` must remain materialized, update it authoritatively at the transition and include it in the rollback checksum projection. At present the clone snapshot preserves the field, but its value probe does not detect disagreement.

## 4. Dialogue activation still confuses pointer departure with pointer release

The new one-tap dialogue interaction uses `Interaction::Pressed` to establish a `RowPress` and treats:

```rust
Interaction::None
```

as release.

In Bevy UI, `Interaction::None` does not uniquely mean that the finger or pointer button was released. It can also mean the pointer left the row while remaining held.

A press near a row boundary can therefore:

1. establish the row press;
2. move a small distance outside the row;
3. transition to `Interaction::None`;
4. remain under the drag threshold;
5. activate the row while the finger is still down.

The intended stable-identity and drag-cancellation model is sound, but the implementation needs a real pointer-up signal. `Interaction::None` alone is ambiguous.

### Required correction

Separate these events:

* pointer entered or left the row;
* pointer remains held;
* pointer or touch actually ended;
* movement exceeded cancellation slop.

Activation should occur only on an actual release of the same pointer/touch identity that began the press.

The comment above `effective_dialog_tap_mode()` also still describes automatic touch promotion to two-tap behavior even though that behavior was removed. Update it to match the current policy.

## 5. Sanic still inherits the full development sandbox ability set

Sanic's character rows define the peaceful action-set preset and character-specific momentum parameters, but they do not define an explicit generic ability set.

Actor construction therefore falls through to:

```rust
editable_abilities.as_engine()
```

whose default is the complete sandbox ability collection.

The peaceful action set suppresses some action presentation and combat behavior, but it does not make the underlying generic movement capability set character-authored.

Sanic's final capability set consequently depends on the current development fallback and can change when sandbox defaults change.

### Required correction

Give each Sanic form an explicit authored ability grant.

Character-specific momentum and ball-dash behavior can remain in Sanic-owned systems. Generic traversal abilities should still be deliberately selected in content rather than inherited from an editor fallback.

## 6. `--walk` does not guarantee the requested number of walking frames

Both Mary-O and Sanic capture flows maintain independent counters for:

* warmup frames;
* rightward walking frames.

The systems decrement both counters concurrently. Capture is gated by the warmup counter but not by the walking counter.

For:

```text
--walk 60 --warmup 10
```

capture can occur after approximately ten frames, while fifty requested walking frames remain.

That contradicts the option's documented meaning as the number of frames to hold right before capture.

### Required correction

Define one of these semantics explicitly:

**Sequential**

```text
walk for N frames
then warm up for M frames
then capture
```

**Concurrent**

```text
capture only when both counters have reached zero
```

Then gate capture on the corresponding completion condition. The current implementation does neither consistently.

Update usage-error text to include `--walk N` wherever the option is accepted.

## 7. Item, encounter, and LDtk extension installation remains process-global

The following extension seams still use process-global `OnceLock` storage:

* item catalog override;
* encounter-wave book;
* additional LDtk entity converters.

The latest work improves failure visibility: conflicting second installation now emits an error rather than silently retaining the first value.

That does not solve the ownership problem. Multiple Apps, game providers, or conversion contexts in one process still cannot use different content.

This matters for:

* test isolation;
* editor and preview applications;
* multiple game experiences;
* future hot reload;
* tools converting content for more than one provider.

### Required direction

Continue the App-local provider migration:

* item and encounter catalogs should be resources selected by provider/session context;
* pure LDtk conversion should accept an explicit converter registry or conversion context;
* ambient global state should not select game content.

The loud conflict is an acceptable temporary guard, but it should remain recorded as an interim limitation rather than a completed ownership migration.

## 8. Production comments continue to retain stale investigation history

Several comments still describe superseded implementations or retain extensive debugging chronology.

The dialogue tap-mode comment is now stale. Earlier `MovePlayback` comments had the same problem after its rollback implementation changed.

Keep production comments centered on:

* the current invariant;
* the authority that enforces it;
* the important non-obvious consequence.

Move dates, model names, eliminated hypotheses, and repair chronology into planning or incident notes. Long historical comments have already caused the source description to contradict current behavior.

## Mary-O LDtk migration

The migration is still in progress, so missing level families, incomplete presentation parity, or the lack of a final runtime switch are not review findings.

The present equivalence machinery should be described narrowly. It currently proves selected structural properties such as collision occupancy and identified special blocks. It does not yet prove complete level equivalence across:

* presentation;
* entities;
* triggers;
* authored metadata;
* runtime behavior.

That is appropriate for an intermediate migration stage. Avoid naming the current check as full level equivalence until those dimensions are intentionally included.

## Priority order

1. Include lowered runtime content in prepared-pack identity.
2. Make roster topology precede and define session creation.
3. Remove or derive the permanently stale `WorldItem.emerging` field.
4. Tie dialogue activation to actual pointer or touch release.
5. Give Sanic explicit abilities.
6. Correct capture `--walk` semantics.
7. Continue replacing process-global content registries as the relevant families are revisited.
8. Trim stale incident-history comments during nearby edits.

Use targeted validation around each changed seam. None of these findings requires repeatedly running the complete workspace suite.
