# Item recipes and live continuity

**State:** IR1 and IR2 done (2026-11-24); IR3 and IR4 PLANNED. From the 2026-11-21 review of production source
`f412189796254f280e9f5018b685314b6994060c`. The review checkout also contains
planning changes through `57c1db339f25`. This page reports source findings,
not a new runtime measurement or an implemented change.

**Owners:** [A7 item custody](item-custody-and-accounting.md),
[construction](construction-and-reconstitution.md), and
[OW3 residency](open-world-runtime-and-residency.md). The
[queue](../queue.md#a7--make-item-occurrence-authority-explicit-where-it-carries-a-real-invariant)
selects IR1–IR4. This page owns the shared recipe and continuity work; it does
not replace those domains' identity, retention or publication contracts.

## Decision

Retain the item recipe selected when an occurrence is created. Use that
recipe for both room construction and custody reconstruction. Keep a live
item's current pose in its body owner. Capture dormant and durable state at
the accepted lifecycle boundary. Save rows are an output of those native
facts, not the only native store from which a dormant recipe can be read.

These facts have different meanings and change rates:

| Fact | Meaning | Owner and change boundary |
| --- | --- | --- |
| Content definition | Held/use behavior and authored physical form | Admitted content generation |
| Occurrence recipe | The selected definition and required instance choices, including canonical geometry | Item owner; selected at mint or explicit migration |
| Identity and disposition | Which occurrence exists, its custody and its room | Existing occurrence/custody owners; accepted transitions |
| Current pose | Where a live body is now | `BodyKinematics` in that room's simulation |
| Held exit context | Where and how the occurrence can re-enter the world if its holder ends | Item custody; binding, accepted transfer and selected retention cuts |
| Dormant state | The fields retained when a room gives up live ownership | Existing residency/retention boundary |
| Durable export | The retained facts at one accepted save or checkpoint cut | Session coordinates; domain owners supply their records |

With the same admitted content and retention policy, create → retain →
reconstruct must preserve each retained field. This does not require saving contact
solver state, velocity or every ECS component. The policy must name what it
keeps. Provenance says who created an item; it cannot supply physical inputs
that the creator did not retain.

## Source findings

### Reconstruction discards the original geometry

`GroundItem::released` sets `BodyKinematics::canonical_size` to twice its
`half_extent` argument. Real callers choose different extents:

| Creation road | Canonical size | Source |
| --- | --- | --- |
| Ordinary actor weapon drop | 32 × 32 | `ambition_platformer2d_actor_monolith/src/body/damage/actor_hit.rs`, `drop_held_weapon` |
| Boss signature weapon drop | 36 × 36 | `body/damage/boss_hit.rs`, the same item-domain drop road |
| Shipped trailhead regrown coconut | 14 × 11 | `ambition_platformer2d_items/src/regrowth.rs`, LDtk conversion, and `game/ambition_map_assets/ambition_content/worlds/dinosaur.ldtk` |
| Shipped orchard regrown apple | 16 × 16 | The same regrowth road and `game/ambition_map_assets/ambition_content/worlds/science.ldtk` |

`MintedItemDescription` in `pickup/minted_horizon.rs` retains only provenance
and a held-item ID. Its live capture query does not read canonical geometry.
`PersistedMintedItem` in `ambition_persistence/src/save_data.rs` has the same
loss. Both `ambition_platformer2d_items/src/pickup/mod.rs` and
`ambition_platformer2d_publication/src/construction/mod.rs` rebuild with
`MINTED_ITEM_HALF_EXTENT`, which gives a 36 × 36 body.

Thus a regrown coconut can keep its occurrence ID and held behavior but
change its collision geometry after room reconstruction or save/load. The
ordinary weapon drop supplies the same counterexample without map data.
These are source deductions. The review inspected the shipped LDtk values;
it did not run these gameplay transitions.

### Live motion invalidates dormant recipe work

The current production path is:

1. `ambition_held_items::record_placed_ground_items` records exact body
   positions each simulation tick. Its population includes moving items
   and items carried by moving support geometry.
2. `AuthoredOccurrences::republish_placements` changes a row in one shared
   `Arc<BTreeMap<SimId, OccurrenceWhereabouts>>`. Previous-tick
   `OccurrenceMirrorKey` and `MintedMirrorKey` values retain that allocation.
   The first position write therefore copies the whole map in the composed
   path, even without a rollback host.
3. `MintedItemRows::rows` uses the whole ledger in its change key. A position
   change calls `minted_baseline_from_save`, merges all dormant recipes and
   builds the persisted recipe vector again. The recipes did not change.
4. `persist_occurrence_horizon_to_save` projects all occurrence rows and
   filters all recipes. When the rounded save position changes,
   `set_durable_horizon` replaces both row allocations, including the
   unchanged recipe allocation. Their cached checksums no longer apply.
5. Ordinary `session/src/room_preparation.rs` reads the saved recipes back
   into typed data. The mirror is therefore also a simulation input.
6. `autosave_sandbox_save` runs on each confirmed `Update`. If the saved
   value changed, it serializes the full RON file. A changed rounded item
   position can therefore cause full-file output on every confirmed moving
   frame, in addition to the simulation work above.

With `N` occurrence rows and `M` recipes, one changed position causes an
`N`-row map copy, `M`-row recipe decode/merge, `M`-row recipe encoding and an
`N`-row occurrence projection. A subpixel change can avoid the final save
setter while still doing these earlier walks. This is a count of source
operations, not a timing result.

The quiet-world optimizations remain valid. The missing case is a small live
population moving beside a large dormant population. Replacing the map with
a faster map alone leaves the incorrect invalidation and save feedback loop.

## IR1 — retain the recipe chosen at mint

Make this correctness packet independently complete before changing storage.

1. Give the item owner one typed construction description. Start with the
   proved missing fact: canonical extent. Retain the stable content
   reference and any actual instance override. A regrown placement can
   name its source recipe when the selected content inputs preserve that
   meaning; a runtime choice needs its explicit value.
2. Supply the description at original creation. Do not infer it later from
   the ID spelling, current sprite, transformed world extent or parent
   category. Canonical geometry and retained material use the existing
   [scale contract](scale-relative-gameplay.md); applying material twice
   during reconstruction is invalid.
3. Resolve the description against the construction's pinned content
   generation. A stable reference is not permission to read a newer global
   registry during candidate construction. An admitted content change uses
   the existing reload/migration rule.
4. Make room and custody reconstruction consume the same prepared item
   description through the existing construction seam. Keep occurrence and
   custody decisions with their existing owners. Original creation uses the
   same physical inputs.
5. Name which other item fields are retained. An initial hanging policy and
   a current remaining timer are different facts. This packet must not
   silently make all initial state durable.

Use the existing save version/admission mechanism for older rows that lack a
required recipe field. A conversion may recover a value only when retained
content/provenance proves it. Otherwise use an explicit unsupported-version
or reset policy. Exact recovery of an extent that was never stored cannot be
promised. Provisional compatibility must not keep two semantic constructors.

**Remove:** the universal-size assumption at reconstruction, the two private
reconstruction parameter assemblies, and live-query inference of an immutable
recipe. Keep the default extent only at mint sites that choose that value.

**Acceptance:** create a non-default-size item, then reconstruct it through
both the room and missing-custody roads. Preserve occurrence, recipe and
canonical geometry. Include a regrown shipped coconut as the authored-input
case. No portal is needed to expose this defect.

### IR1 first slice done (2026-11-22)

The proved missing fact, canonical extent, is now part of the occurrence's
recipe. `GroundItem::half_extent` is the extent its mint chose (every mint
goes through `GroundItem::released`, which already took it), the capture
copies it into `MintedItemDescription`, the checksum includes it, and
`PersistedMintedItem` carries it as exact `f32` bit words (save v12), and
writes no words for the default extent, which is what an absent field reads
back as. That keeps the round trip stable: a row from an older save is
rebuilt at the default and written back unchanged. (Writing the default as
words changed every loaded save once, and the refused-candidate fixture in
`an_edit_reaches_the_shipped_game` caught the extra candidate cycle.) Both
reconstruction roads, the custody restore in `pickup` and the room
construction in `publication`, build at `description.half_extent`; the
universal `MINTED_ITEM_HALF_EXTENT` survives only at the mint sites that
choose it and as the stated value of a pre-v12 row, which never stored its
extent and was rebuilt at that default. Witness:
`a_mint_keeps_the_extent_it_was_made_at` (a 7 × 5.5 coconut, its persisted
words, and the pre-v12 row). The rollback codec shape is unchanged (the item
and baseline roll back by clone).

### IR1 done (2026-11-24)

**End-to-end witness:** `a_regrown_coconut_keeps_its_extent_through_both_reconstructions`
(app_it, `the_primeval_trail`). The trailhead palm regrows
`primeval_trailhead_coconut_1` at its authored extent (not the universal
one). The room road: the trailhead unloads (the entity is gone in the hub)
and its build places the occurrence again at that extent. The custody road:
the coconut is banked in hand at a checkpoint, left in the hub so its
entity goes with that room, and a death rebuilds it in the hand from the
checkpoint's description at the same extent.

**What a reconstruction retains, by owner:**

| Fact | Retained by | Note |
| --- | --- | --- |
| Identity (`SimId`) and provenance (`SpawnOrigin`) | the minted description | the rebuilt object is the same occurrence, visible to the next capture |
| Item spec (`held_item`, a catalog reference) | the minted description | resolved at restore; a spec edited out of the catalog is reported, not guessed |
| Canonical extent (`half_extent`) | the minted description | save v12 words; a pre-v12 row keeps the default it was rebuilt at |
| Custody (held by whom, or in the world) | the custody baseline | unchanged by IR1 |
| Where an in-world occurrence lies | the occurrence ledger | unchanged by IR1 |

**Not retained, deliberately:** a rebuild is at rest.

- **Hanging state** (`HangingItem`): both the remaining timer and the
  placement's initial hang policy. The policy belongs to the authored
  placement and applies to a fresh growth. A rebuilt grown apple rests
  where its row places it and does not hang again.
- **Flight state:** velocity, `SettledItem` and the release kind
  (`ReleasedAs`, which arms a thrown bomb's fuse).
- **Holder-derived material** (`HeldMaterialBasis`): re-derived from the
  holder at the rebuild.

Making any of these durable is a separate decision with its own owner. Do
not add it to the description by default.

## IR2 — read native item continuity

Make typed item recipes available while their entities are absent. Do not
move the only retained recipe from SaveData into a live-only component.

- The item owner retains recipes across live, dormant and carried states.
  Existing retention policy decides when a terminal recipe can be removed.
  Current state, checkpoint baselines and replay baselines keep their
  distinct horizons, with shared immutable records where their values agree.
- Select recipes from the retained occurrence population at that cut,
  including dormant mints. Every retained minted occurrence that the cut can
  reconstruct needs its selected recipe. The existing checkpoint recipe
  capture queries only live entities; it cannot be reused unchanged. When
  restore spares another room's newer occurrences, merge those recipes at
  the same acceptance point. Neither the old checkpoint recipe map nor an
  indiscriminate copy of every latest recipe describes that selected horizon.
- Decode a saved recipe family once at durable adoption. Room preparation,
  checkpoint capture and custody reconstruction read the native owner.
  Preparation pins the selected recipe values with its other inputs.
  Boss-defeat retraction also reads saved mint parentage today; move that
  reader to the same native recipe owner.
- Export occurrence, custody and recipe families from the same accepted
  cut. Preserve the atomic association established by `set_durable_horizon`:
  no recipe may survive in the file without the occurrence it describes.
- Include the native exit context selected for a held occurrence. A recipe
  can rebuild its extent without recovering its orphan fallback placement.
  [REL4](relationship-endpoints-and-transitions.md#rel4--retain-exit-context-and-authorize-endpoint-succession)
  proves that missing-mint reconstruction at zero loses a later legal exit.
  Resolve the retained custodian within the admitted restore population,
  with explicit succession where needed; a global bare-ID body map is not
  that population.
- Move every simulation reader off the serialized recipe family before
  removing its per-tick mirror. Scheduling serialization in `Update` while
  leaving room preparation dependent on it would supply stale authority.

`MintedItemBaseline` currently uses a plain `BTreeMap`. Full snapshot clone
and checksum paths still visit that map. Include these baselines when making
unchanged recipe records shareable; changing only the SaveData allocation
does not remove that cost. Baseline selection remains authoritative. Sharing
storage cannot make a checkpoint read the latest recipes instead of its own.

**Remove:** `minted_baseline_from_save` from ordinary simulation and room
preparation, the saved-row feedback key, and repeated typed → save → typed
conversion between item and session owners. The import/export codec remains
at the durable boundary.

**Acceptance:** one accepted checkpoint and one room reconstruction read the
same native recipe selection, including a recipe whose entity is dormant.
Leave a mint in room A, checkpoint in room B, restore, then reconstruct A.
Also retain a post-checkpoint mint in a room the restore spares. In both
cases its retained occurrence has the corresponding selected recipe.
Durable adoption and export preserve the selected horizon and supported save
meaning. A recipe change participates in the existing prepared-input
dependency evidence.

### IR2 done (2026-11-24)

**The native owner.** `RetainedMintRecipes`
(`ambition_platformer2d_items/src/pickup/minted_horizon.rs`) holds the
recipe of every live mint and every dormant mint the ledger still has:
placed in a room that is not live, or carried by a body that is not. It
is rollback state on the session root (`resource.retained_mint_recipes`,
schema 539) and has one writer, `retain_mint_recipes`. That writer applies
the existing ledger rule to the live mints and to the earlier and
checkpoint recipes. A recipe goes when the ledger ends the occurrence
(consumed, spent) or forgets it (a restore retracts it).

**Decoded once.** A load decodes the saved family into the checkpoint
baseline (`adopt_checkpoint_baselines_from_save`). A candidate decodes it
into its own baseline (`CandidateDurableHorizon::from_save`). The retained
recipes take those rows from the baseline, so no second writer adopts them.

**Readers moved.** Room preparation (`RoomContinuity`) pins the native
recipes. Boss-defeat retraction reads them for dormant mints. The save's
minted rows are an export of them (`MintedItemRows`), encoded only when the
retained allocation changed. No simulation reader calls
`minted_baseline_from_save`.

**Checkpoint selection.** The capture selects the retained population at
the cut, including dormant mints. A restore that places a mint again after
it was taken up and used finds its recipe in the checkpoint, and the
retention writer returns it to the owner.

**Shared records.** `MintedItemBaseline` holds an `Arc` map. Checkpoint
pins, prepared inputs and snapshot clones share it instead of copying rows.
Its checksum is computed once per allocation and kept with it. The save's
minted rows had that memo (`RowsChecksum`); without it, the retained
recipes and the widened checkpoint baseline would hash every dormant
recipe on each checksummed frame.
The prepared-input digest (`ContinuityDependencies`) now covers the
recipe's extent as well as its provenance and spec.

**Witnesses:**
`a_dormant_recipe_is_retained_by_its_owner_and_returned_by_its_checkpoint`
(items lib) and the durable-horizon FI9 fixtures (session lib). The FI9
fixtures now seed their dormant recipes by the load road. Their idle-tick
and rollback-load timing bounds still hold with 10,000 dormant mints.

**Not done here:** the held exit context is
[REL4](relationship-endpoints-and-transitions.md#rel4--retain-exit-context-and-authorize-endpoint-succession)'s.
A ledger position change still rebuilds the retention map (an O(M) merge,
without the decode or the encode), because the ledger moves on every
moving tick. Removing that walk is IR3's work.

## IR3 — stop active motion from copying dormant state

Keep the live pose in `BodyKinematics`. Existing room rollback already saves
and restores it. A mutable room snapshot that also receives every live pose
would introduce another authority and another synchronization step.

For a held item, distinguish the hand-derived view from the stored body that
current orphan release uses. That stored placement is required exit context,
not disposable stale pose. Retain it at the selected cuts and prepare its
destination meaning on accepted transfer, as REL4 states. Preserve the
same-room orphan receipt; do not keep a duplicate hand pose current each tick
to repair a boundary-only loss.

Keep the small identity/disposition and room lookup needed for routing.
Change it for creation, custody, transfer and retirement, not for ordinary
motion. A native continuity read joins this disposition with either the live
body or the explicitly retained dormant record. A missing entity during
candidate publication or replay does not, by itself, make an item dormant.

Capture the retained pose before an accepted room retirement removes its
live owner. Capture checkpoint/save inputs at their existing accepted cuts.
The checkpoint restore's spared-room merge currently copies live `Placed`
rows; change that reader to capture the same joined current pose. A room
exit or checkpoint on the tick an item moves must use the selected tick's
state. Do not sample an arbitrary presentation frame.

Capture remains domain-specific. Items retain their center; persistent
authored bodies currently retain their feet. Exporting `BodyKinematics.pos`
for both would change reconstruction. Preserve the sparse population rule:
this is the existing eligible continuity population, not every authored body.

Move each boundary explicitly:

- **Checkpoint:** pin the accepted restore inputs once. The spared-room
  join happens at acceptance. Migrate `verify_restored_domains` from whole
  live-ledger equality to the selected discrete continuity and publication
  obligations. Transit can reconcile a body's pose, so that check must not
  demand equality with its pre-construction coordinates.
- **Room transition:** retain target-room dependency revalidation before
  destructive work. Capture an ordinary departure immediately before
  accepted teardown, not when asynchronous preparation started. A restore
  continues to use its pinned historical inputs.
  `CandidateSave::horizon_of` and `PreparedFromSave::is_current` must use
  that selected native cut. They must not obtain older geometry or
  whereabouts merely because disk output is not yet due.
- **Save:** freeze the export input after that tick's physics, custody,
  ending decisions and deferred-command flush. Preserve the confirmed-state
  gate for durable writes. An output task consumes the frozen values; it
  does not read a later `World` after serialization becomes ready.

Use existing room ownership to partition retained records and snapshots.
Untouched dormant partitions and immutable recipes keep their allocations.
An accepted transfer updates its participating owners through the current
handoff contract. Local motion does not enter the global world-operation
queue. Preserve the scope of room rewind and the existing
[causal-world contract](causal-world-commit.md) for cross-history effects.
The new recipe owner must follow the current local mint/end retraction.
Restoring one room's records must not overwrite another room's current
recipes or dispositions.

The representation can retain per-partition encodings or encode a pinned
native snapshot when an actual durable output is needed. A flat save file
can still require `O(N + M)` work at export. The goal is to remove that work
from native simulation and snapshot maintenance. Today's autosave cadence
can still request full output on every changed confirmed frame. IR4 supplies
an explicit output policy; moving work to `Update` or a background task alone
does not remove that cost.

A top-level `Arc<BTreeMap<Room, ...>>` can still copy or hash all room entries
when one child changes. Count that root work. Do not claim independence from
dormant-room count merely because item rows are partitioned. Select a more
incremental root representation only if its measured cost warrants it.

**Remove:** active position updates to the whole-world copy-on-write map,
per-tick flattening of these derived save families, and duplicate derived
checksums after all native readers have migrated. Keep intentional durable
rounding and each retention policy explicit at export.

**Acceptance and cost evidence:**

- Hold the live population constant. Compare one moving item with no
  dormant items and with many dormant items. Include a supported moving
  item so the witness is not limited to short throws.
- Count copied rows, recipe decodes, encoded rows and hash visits. Vary
  dormant item count and dormant room count separately. Include root
  containers, baseline clones and checksum work.
- Compare retained values, reconstruction and rewind. Exercise a checkpoint
  and a room exit on the same tick as motion. Keep an unchanged-world
  control to preserve the current cheap path.
- Keep full-save work outside this packet visible. Other SaveData families
  still have clone/serialization checksum work; these changes do not prove
  that all snapshot cost is independent of world history.

## IR4 — select durable output without changing gameplay state

**Proposed output policy:** preserve prompt writes for accepted semantic
changes and coalesce changes that affect only retained position. Start the
prototype with a one-second maximum pose-only interval. This is a disclosed
crash-resume freshness tradeoff, not a measured optimum or a new gameplay
clock. Keep the interval with persistence and record the selected value with
the cost result.

| Cause | Output behavior |
| --- | --- |
| Accepted progress, quantity/custody/disposition change, checkpoint, room retirement or explicit save | Request output at the first eligible confirmed opportunity |
| Only retained positions changed | At the finite deadline, export the latest eligible coherent cut |
| No retained value changed | No write |
| New or migrated save | Keep the existing initial/rewrite obligation |
| A source file this build must not replace | Keep `SaveFileWritable`; a deadline cannot override it |

Use a maximum-interval throttle, not a trailing debounce. Continuous motion
must not postpone its deadline forever. A prompt write contains the whole
selected cut, including current eligible positions, and can satisfy the
pending pose write. New custody cannot be combined with an unrelated older
position snapshot.

Keep scheduling in persistence, using monotonic host time outside rollback
and checksums. The timer decides when to request output; it does not decide
which simulation facts exist. Candidate construction, native continuity and
checkpoint capture remain independent of the disk timer. Preserve the
existing immediate progress-flag write in a non-speculating composition.

A due request waits for an eligible confirmed cut. Do not mark it complete
when confirmation or the writer refuses. Preserve its save owner and
session/timeline identity if a captured value waits for output. A changed
experience must not send the old capture to the new owner's file. The
persisted shadow advances only on successful installation, as it does now.
Keep the current refused-state behavior and unsupported-file protection.

The extra pose delay is bounded only while confirmed captures and successful
writes are available. Existing prediction policy can postpone saving
indefinitely; storage failure and process suspension also prevent a wall-time
guarantee. A deadline never permits a speculative write.

Close the graceful lifecycle gap before enabling the interval.
`hand_the_save_to` currently parks the outgoing save but does not write it;
shell/menu quit directly emits `AppExit`. On experience retirement/switch
and graceful quit, freeze and attempt the outgoing owner's latest eligible
confirmed cut before teardown completes. If output is asynchronous, keep
ownership and ordering until it finishes; an older capture must not overwrite
a newer one. Do not promise a flush after a crash or abrupt browser close.

This changes disk freshness only. It does not establish a new checkpoint or
bank an ordinary dialogue exchange. Preserve Q38's durable whereabouts,
Q129's semantic agreement and Q164/Q151's retention rules in
[maintainer decisions](../maintainer-decisions.md). Those rulings do not
require every pose to reach disk on every frame.

**Acceptance and measurement:** run IR3's moving-item case with persistence
enabled. Report native simulation maintenance separately from capture,
encoding and actual output. Continuous motion reaches the deadline; an
accepted progress change does not wait for it; a checkpoint/room exit captures
the exact selected cut; switch/quit uses the correct save owner; no pending
request bypasses confirmation. Keep unsupported-file and failed-write
behavior. Use the existing persistence fixture for these boundary cases.

Every full-file write still traverses the exported population. Other save
families retain their current output demands. If frame-by-frame pose
durability later becomes a requirement, revise this policy explicitly and
scope an incremental output format separately. Background serialization alone
cannot provide both that guarantee and a size-independent write cost.

## Delivery and limits

IR1 can land without IR2/IR3. IR2 changes native authority before IR3 removes
the derived mirror. IR4 changes only output selection after native readers
and capture no longer depend on the per-tick save mirror. A narrow interim
optimization may key recipe retention
on membership rather than position and preserve an unchanged recipe
allocation. It must not be reported as completion of IR3: the global map
copy and occurrence projection would remain.

Reuse the existing item, construction, residency and persistence owners.
This work needs no universal prefab type, ECS serializer or new collection
library. If a shared data-structure operation emerges, give it the measured
contract required by [K1–K3](data-structure-kernels.md). The domain must still
decide which state exists, who owns it and when it can change.

Do not use Bevy change flags as the sole evidence of restored state, let
cache presence decide existence, or widen persistence to every spawned
object. Preserve session-only match items, count-based bag semantics and
the distinct policies for actors, items and transient effects.
