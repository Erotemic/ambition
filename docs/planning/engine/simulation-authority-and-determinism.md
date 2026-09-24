# Simulation authority and determinism

**State:** open. Rollback backend ownership and domain registration are largely
settled. Current work is peer-stable identity, owner-scoped lifetime, deterministic
composition and explicit phase/authority boundaries.

## Goal

A simulation result must be determined by explicit authoritative data and named
composition rules, not by:

- ECS/query iteration order choosing a winner;
- `Local<T>` or process-global memory carrying an authoritative future across a
  rewind;
- host-local lifecycle counters entering peer-stable mechanical identity;
- a rollback-registered component living on an entity that is not on the rollback
  timeline;
- mutable App/editor state being read directly during resimulation;
- two mutable representations independently answering the same mechanical
  question;
- scheduler topology acting as an undocumented conflict-resolution rule.

## Current authority model

An authoritative dynamic fact may need several independent guarantees. Do not
collapse them into one "rollback-safe" label.

### Rewind codec

The domain declares which component/resource state is snapshotted, restored,
entity-remapped and represented in checksum policy. The gameplay domain owns its
rollback declaration beside the types it understands.

The generic runtime collects backend-neutral declarations. The concrete GGRS
backend/session/schedule lives in `ambition_platformer2d_rollback_ggrs`. Do not
recreate a central concrete-type census in the runtime.

### Rollback participation

Registering a component type is not enough. The authoritative entity population
must also participate in the GGRS creation/destruction history. A value can have a
codec and still be absent from the actual rewound population.

### Stable semantic identity

`SimId` is the stable semantic identity used for deterministic simulation objects.
`bevy_ggrs::RollbackId` is frame-history machinery, not gameplay identity.

Use semantic identity when reconstruction, relationships, deterministic selection
or peer comparison need to refer to the same logical object. Do not mint canonical
identity from ECS entity order or an App-local activation count.

A type census cannot see a violation of this rule when the offending value is a
`String` field of a correctly registered component. For example, a perceived-actor
key built from `entity.index()` inside `PerceptionMemory` once put ECS allocation
order into the peer checksum and into target selection.
`the_peer_visible_surface_does_not_record_which_route_the_host_visited_first`
(`game/ambition_app/tests/shell_host_lifecycle.rs`) holds this rule; extend that
arm when the rule needs a new subject.

Local lifecycle identities such as `SessionScopeId`, `ContentEpoch` and shell/load
correlation ids remain useful for ownership/correlation. They are not substitutes
for peer-stable mechanical identity. The current cross-peer cleanup is tracked by
[ID-PEER in the queue](../queue.md#id-peer--remove-host-local-lineage-from-peer-stable-mechanical-identity).

### Deterministic selection and composition

If multiple valid entities can affect one result, define the rule at the owner
boundary. Use a stable semantic key where selection needs one. When effects do not
commute, define precedence/composition explicitly instead of relying on query
iteration order.

Prefer one shared ordering helper for a domain over several call sites inventing
their own stable order.

### Lifetime ownership

The important lifetime hierarchy is:

```text
process/App
  -> gameplay session
       -> rollback timeline generation
            -> entity / operation
```

`ActiveRollbackAuthority` intentionally owns rollback owner, timeline generation,
contract and health together because they are one authority. A different gameplay
session may not inherit the previous session's rollback health or live mechanical
mirrors. A same-session timeline rebase must not erase evidence of a real desync.

Session-owned values that still live as App resources are candidates for ownership
consolidation, not automatic defects. The architecture census records the current
population and the reset/retirement mechanisms that compensate for App storage.

### Mechanical policy admission

Mutable editor/preferences state is not automatically deterministic input. Live
mechanical editor domains use a proposal -> admission -> publication boundary so
a rollback host can rebase/refuse before simulation sees the new authority.

Direct `UserSettings` reads have been removed from the simulation schedule. Values
that affect mechanics are projected into admitted policy, such as per-seat control
frame modes and `PlayerDamagePolicy`. Difficulty is
[ruled](../maintainer-decisions.md) (`Q127`): there is no generic one-dimensional
engine difficulty architecture. Difficulty is game policy expressed as presets;
participant handicaps and CPU brain levels are separate concepts, and
participant-specific assist/handicap state stays distinct from game/match policy.
The topic is deprioritised: preserve enough architecture not to be boxed in later,
and get the default game playing well first.

## Established architecture

These decisions are current foundations; do not reopen them without new evidence.

### Domain-owned rollback declarations

Each gameplay domain owns the registration of its rollback types. The host/backend
composes offers; it does not enumerate leaf gameplay types itself.

`central-rollback-does-not-enumerate-domains` in
`scripts/check_absence_contracts.py` protects this boundary.

### Wire-format changes are explicit

Rollback schema names and encoded shapes are compatibility data. A source-file
move is not a reason to rename a stable schema entry. A real shape/name change must
update the declared wire-format baseline intentionally.

Use the repository's rollback baseline/checks; do not copy their counts into this
page.

**Open: the schema fingerprint hashes prose.** `schema_dump()` emits name, kind,
wire type and a human-readable `detail`, and `compute_schema_fingerprint` hashes
the whole dump. So correcting a description is indistinguishable from changing an
encoding, to the guard and to every peer. `Q122` is
[ruled](../maintainer-decisions.md): mechanical identity fingerprints mechanical
facts, not explanatory prose.

Excluding `detail` wholesale is refuted: for many kinds, `detail` carries facts
that `kind` does not encode (for example, entity handle versus entity set versus
keyed entity map remapping, or what a custom checksum function covers). The rule
that fits both halves keeps `detail` exactly where it distinguishes rows of the
same kind and drops it where one sentence covers the whole kind. The peer-checksum
tooling ratchet already applies that rule; `compute_schema_fingerprint` does not
yet, so the repository still answers this question two ways.

### Explicit simulation phases

The GGRS simulation schedule has named Ambition phases and currently uses a
single-threaded executor. This is a measured implementation choice, not a
requirement that deterministic simulation must always be single-threaded.

Ordering edges are meaningful only when the referenced set has real members in
the composition. `scripts/check_set_pins_have_engine_members.py` detects engine
pins to empty engine sets.

### Session-owned rollback authority

Rollback contract/health is scoped to the gameplay session. Session activation and
retirement establish/clear the session mirrors together rather than relying on
ad-hoc cleanup in each consumer.

Do not replace owner-scoped lifetime with a growing list of "clear on quit"
systems.

## Current work

### S5 — phase and ownership decomposition

Split high-authority systems only when the split creates a real semantic owner or
phase. Useful evidence includes:

- independent authoritative domains being read/written;
- correctness-sensitive ordering;
- broad queries that cross ownership boundaries;
- rollback participation differences;
- mutation mixed into proposal/decision logic;
- duplicate derivations of one authority.

A `SystemParam` or `QueryData` is useful when it names one concept. Hiding many
unrelated resources behind one parameter is not decomposition.

The current A4 control/body packet is one concrete customer; use the actual Bevy
schedule realization rather than retired abstract set names. See
[A4 in the queue](../queue.md#a4--separate-control-authority-from-body-execution-on-the-real-schedule).

### S6 — move session-owned state toward session ownership

The census identifies App-stored resources whose semantic lifetime is a gameplay
session or content generation. Existing reset systems and owner stamps often make
that correct today, but each reset-only mirror is a candidate for a stronger
storage owner.

A10 has established the live/candidate session shape, and `Q132` rules that there
is one canonical live `SessionRoot`; a prepared candidate has a distinct identity.
Consolidation C03 carries this migration. For every candidate value:

1. name its semantic lifetime;
2. name whether it must exist before `SessionRoot` construction;
3. preserve rollback registration/restore semantics;
4. choose `SessionRoot` component ownership, a session-keyed coordinator, or a
   genuine App resource based on those facts;
5. remove reset/synchronization machinery only after the new owner makes it
   redundant.

Do not move state merely to reduce a resource count.

### S7 — canonical checksum coverage

Rollback registration and session checksum coverage are different properties.
The rollback baseline contains rows intentionally cloned/restored for localization
that are not themselves encoded into the session checksum. Some are legitimate
because another canonical projection represents the same fact; some may be dead
or diagnostic-only state; a mechanically relevant value with no peer-comparison
projection needs review.

Do not preserve a copied count of these rows here. Recompute the population with
`python3 scripts/measure_unchecksummed_rollback_rows.py` (it reads
`game/ambition_app/tests/rollback_schema_baseline.txt`). This page owns the
argument; the script owns the cardinalities. Classify each row by:

1. production readers;
2. whether it can affect future mechanical behavior;
3. which canonical projection/checksum represents it, if any;
4. whether the state is reachable in production.

A green float-finiteness or codec test does not prove that every restored value is
peer-compared.

**How the census works.** It selects on `RollbackEntryKind` (`carries_state()`,
`feeds_peer_checksum()`), not on kind strings or on the `detail` text: a filter on
the text once hid most of the population, because the unhashed rows carried a
reassuring sentence. The census splits the population by whether a row has a
desync-localization probe, and triages readers: an unfiltered per-tick query or
`Res`/`ResMut` read; presence only, through a query filter; gated or point reads;
no production reader. Markers are read through `With`/`Without`/`Has`, and
resources through `Res<T>`, so a borrow-only scan reports both as unread. Known
answer controls pin both patterns. Types defined outside the repository
(`Name`, `Transform`) are listed in `EXTERNAL_TYPES` with their floats stated.
The population floor is a member diff against the baseline, not a count.

**Probe strength is a runtime fact.** `RollbackChecksumProbes` owns it:
`ProbeStrength` is `Value`, `Complete` (zero-sized types) or `Presence`, and
`presence_only_type_names()` lists the weak half.
`every_presence_only_probe_is_named_with_its_reason`
(`game/ambition_app/tests/rollback_exit_oracle.rs`) asserts each presence-only
probe has a recorded reason. A registration's justification that lives in that
allowlist, in a different crate, is the same one-owner question as `Q122`.
`RollbackChecksumProbes::strengthen_with::<T>(projection)` upgrades a probe to a
value probe without touching the registration, the schema or the GGRS aggregate.

**A component whose presence is read by a query filter is authoritative even
when its value is derived.** "Derived" says how a value is computed; rollback
cares whether anything reads it before its writer runs again. For example,
`Dormant` (`actor.dormant`) is recomputed every tick, but `Without<Dormant>`
filters production queries, so it must stay registered.
`scripts/check_presence_filtered_state_is_rollback_registered.py` measures the
inverse: a presence-filtered component with no rollback row at all.

`PlayerSlot` (`actor.player_slot`) is an open question: it is `component-clone`,
and no canonical projection of the per-body slot was found. Seating is
peer-agreed through `ActiveMatch::peer_stable_checksum`, which hashes the seat
count; whether that constrains per-body slot assignment is not measured.

#### The sharp rows

The rows that matter most satisfy three conditions at once: no value projection,
read by an unfiltered per-tick query, and a float-bearing field. A float drifts by
rounding rather than by a logic error, the drift compounds, the reader sees it on
the next frame, and no checksum two peers compare can see it. `sharp_rows` in the
census script computes this intersection and raises rather than returning an empty
list; `test_the_sharpest_list_is_an_intersection_and_excludes_each_operand_alone`
and `test_an_unresolved_type_is_not_promoted_into_the_sharpest_list` pin it.

The list is a ranking, not a defect list. The discriminator is whether anything
mutates the value after spawn: a value nobody writes cannot diverge between two
peers who authored it from the same content. Read the write set, not the row
name. The rows that are mutably borrowed in production are:

`actor.animation_facts`, `actor.render_size`, `boss.death_animation`,
`entity.transform` (republished every frame), `feature.hazard`,
`item.ground_item`, `player.blink_camera_state`, `portal.emission`,
`portal.gun_pickup`, `portal.placed`, `portal.shot`.

The remaining sharp rows (`actor.interaction`, `actor.spawn_baseline`,
`actor.sprite_offset`, `actor.sprite_posed_body`, `boss.capability`,
`boss.config`, `boss.overrides`, `combat.tuning`, `encounter.camera_zoom`,
`lifecycle.room_visual`, `mount.authored_size`, `mount.mass`, `mount.mountable`)
have no mutable borrow in production. That is "no mutable borrow", not "never
written": a `&mut` census cannot see replacement by re-insertion.
`RollbackRestoreAudit` answers that at runtime; no static scan can.

**Two-host arm.** `two_local_histories_agree_about_the_sharp_unchecksummed_rows`
(`game/ambition_app/tests/shell_host_lifecycle.rs`) asserts that each of these
eleven rows does not feed the peer checksum, then compares their probe census
between a host that reached the shipped Ambition route first and one that reached
it third, at every tick of the window, in five rooms. All eleven carry state and
agree. The set is pinned by equality, so a row that loses its carriers fails. The
arm prints the per-room split:

| room | sharp rows it carries |
|---|---|
| `<authored>` (what pressing launch reaches) | `actor.animation_facts`, `actor.render_size`, `entity.transform`, `item.ground_item`, `player.blink_camera_state`, `portal.gun_pickup` |
| `portal_lab` (driven into the authored aperture) | `actor.animation_facts`, `actor.render_size`, `entity.transform`, `player.blink_camera_state`, `portal.emission`, `portal.placed` |
| `basement_hazards` | `actor.animation_facts`, `actor.render_size`, `entity.transform`, `feature.hazard`, `player.blink_camera_state` |
| `portal_bridge` (driven, with the gun) | `actor.animation_facts`, `actor.render_size`, `entity.transform`, `player.blink_camera_state`, `portal.gun_pickup`, `portal.placed`, `portal.shot` |
| `basement_boss` | `actor.animation_facts`, `actor.render_size`, `boss.death_animation`, `entity.transform`, `player.blink_camera_state` |

`item.ground_item`, `portal.emission` and `boss.death_animation` have one carrier
room each. `boss.death_animation` is inserted at spawn and stays constant until a
boss dies, so the arm compares its presence, not a varying float; a real boss
death is still the stronger observation. To find where a row can be observed, read
the code that inserts the component, not the event its field is named for.

Two Apps with different local histories are still not two peers: no transport, no
input exchange, no interleaving, no rebase. The arm decides whether a row's value
depends on where the host has been, and nothing more.

**Local restore versus peer agreement.** `item.ground_item` and
`actor.animation_facts` also reproduce under local resimulation
(`game/ambition_app/tests/does_a_presence_probed_row_move_when_its_value_does.rs`,
using `strengthen_with` and
`RollbackRestoreAudit::distinct_censuses_across_compared_frames_of::<T>()`). That
clears a local restore defect, not the S7 question. `Session::SyncTest` is the only
session this workspace constructs; no P2P session is built. For the timeline half,
`Q128` and the missing P2P session are one blocker.

**Motion floor.** A window where the value never moves proves nothing: a census
that takes one value across the window agrees with itself for free. An attack
input left every `BodyAnimFacts` field at `0.000`; only landings moved it. Measure
which input moves a value; do not choose the window from the field name. Every arm
here asserts a floor on distinct censuses across the compared frames, not only on
`resimulations > 0`.

Spend the per-row instrument (`measure::<T>` plus the floors in
`the_reading_is_about_the_subject`) when a row is suspected, not to walk the list.

### S8 — hashed entries written from a schedule that never rewinds

**Closed for `AmbitionGameSave`.** The save mirrors (`persist_inventory_to_save`,
`persist_occurrence_horizon_to_save`, `persist_minted_item_horizon_to_save`) run in
the simulation schedule, and the dialogue visit count is
`count_the_dialogue_visit_when_a_conversation_opens`, also in the schedule. The
save was not unhashed: most systems that write it are simulation systems, and
removing it from the checksum would stop comparing them.

Three arms hold this:

| arm | asserts |
|---|---|
| `no_registered_type_is_written_outside_the_rewinding_schedule` | the set of registered types written outside the rewinding schedule is empty |
| `no_hashed_entry_disagrees_with_its_replay_when_the_bag_moves` | the diverging set is empty, with a floor on the projection varying |
| `the_saves_hashed_snapshot_tracks_the_frames_it_is_compared_at` | the save's projection takes at least 50 distinct censuses across the compared frames |

The floor is 50, not `> 1`, because the defect state was a pinned projection that
still took one or two values. An empty divergence set from a pinned projection is
a constant function: every equality arm passes when the projection stops
responding to the state it covers. So un-pinning is asserted beside emptiness.

A hashed entry written outside the rewinding schedule is a defect only if all
three hold: it is hashed (`feeds_peer_checksum()`), its writer is outside the
rewinding schedule, and its value actually differs at a frame compared twice.
`NewGameResetRequested` meets the first two and does not desync, because the flag
is put back before it is taken.

**Open: `CustodyBaseline` and `OccurrenceBaseline`.** Both desync when a
mid-session load seeds both halves of the durable horizon.
`probe_what_a_mid_session_load_writes_outside_the_rewinding_schedule`
(`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`,
`#[ignore]` while the defect is open) asserts the premise and the defect. This is a
lifecycle question owned by
[DURABLE-HORIZON-CHECKSUM](../queue.md#durable-horizon-checksum--the-save-mirrors-write-hashed-state-from-update)
and waits on
[Q135](../awaiting-maintainer-decision.md#q135--should-ggrs-start-before-the-durable-restore-has-finished).
Earlier "clean" readings of these two ran with empty baselines; a fixture can put
pressure on one field of a pair and none on the other. `Q129` owns whether the
save belongs in the peer contract at all.

The probe and the GGRS aggregate for a `rollback_resource_clone_checksum` entry
are installed from the same `checksum` argument in
`install_resource_clone_checksum`
(`ambition_platformer2d_rollback_ggrs/src/registration.rs`), so the probe's value
is the aggregate's value. The two systems are unordered within `SaveWorld`; that
is immaterial while nothing in `SaveWorld` writes the save.

The "outside the rewinding schedule" measurement sees only registered types whose
probe can see a value change. Unregistered state written from `Update` is
invisible to it. `scripts/check_rollback_mutators_run_in_sim.py` lists the
acknowledged offenders.

### Host-to-host determinism witness

An older two-host measurement found a duel that agreed for hundreds of ticks and
then diverged while each host remained repeatable on its own. That evidence was
stamped to an older commit and does **not** establish that current HEAD still
diverges.

Before drawing architecture conclusions, reproduce on current HEAD on both hosts
and capture the **first divergent authoritative state**, keyed by stable identity.
Do not reason backward from final bout statistics. If the current build no longer
reproduces, record closure rather than preserving the old investigation as a
current defect.

## Extension-state coverage

Dynamically declared extension state uses the same rules. Registry metadata alone
is not rollback participation. Module scratch heaps, statics, RNG cursors and
coroutine state may not remember authoritative futures outside the declared
state/execution contract.

Use [`extension-state-and-execution.md`](extension-state-and-execution.md) and
[`extension-domain-contracts.md`](extension-domain-contracts.md) for extension
read/write/request semantics. Extensions do not get a second rollback backend or a
weaker identity/lifetime model.

## State projection rule

Read models are allowed and encouraged. They must be one-way projections from
one authority.

If a projected component/resource contains fields that another system mutates as
independent authority, either split the representation or move that authority.
Do not rebuild a projection by saving and restoring selected fields around the
rebuild.

## Test and measurement topology

Use the smallest host that still contains the property being asserted.

| Property | Required proof shape |
|---|---|
| deterministic simulation | real headless `GgrsSchedule` / sync-test session |
| runtime-created rollback population | scenario that creates the population before rewind |
| cross-session isolation | shell/app host that creates, retires and creates real sessions |
| editor/mechanical admission | production ordering before rollback advance plus ownership arms |
| physical input/rebinding | real input/session host |
| rendered/raster behavior | rendered hardware measurement |
| durable persistence | fresh-process reconstruction |
| capability composition | explicit supported profile/feature matrix |
| peer-stable identity | two Apps/peers with intentionally different local history |

A helper that manually installs the resource/system under test cannot prove that
the production composition installs or orders it correctly.

## Static guards

These guards are discovery/structural checks, not substitutes for runtime proof:

- `scripts/check_rollback_mutators_run_in_sim.py` — catches known rollback-state
  mutation registered outside the rewinding schedule;
- `scripts/check_set_pins_have_engine_members.py` — catches engine ordering pins
  to empty engine sets;
- `scripts/check_absence_contracts.py` — includes the domain-ownership and wire
  compatibility absence contracts.

When a static census has an unattributed/unknown bucket, it can establish a floor,
not prove zero.

## Acceptance

- adding a rewindable domain type changes its domain declaration, not a central
  gameplay-type census;
- representative dynamic authoritative families survive rewind/recreation with
  correct semantic identity;
- no known future-affecting state depends on non-rewinding scratch/process memory;
- deterministic selection does not depend on raw ECS iteration order;
- host-local lifetime counters do not determine peer-stable mechanical identity;
- one gameplay session cannot consume another session's rollback health or live
  mirrors;
- same-session rebases preserve real desync evidence;
- simulation systems have named authority/phase contracts rather than parameter
  packing disguising unrelated ownership;
- restored mechanically relevant values have an explicit peer-comparison story.

## Non-goals

- another custom rollback/snapshot engine;
- pushing `bevy_ggrs` into every leaf domain;
- a universal raw-spawn wrapper;
- exhaustive pairwise `.before()`/`.after()` edges instead of semantic phases;
- scheduler parallelism as an architecture objective without new measurement;
- moving state merely to reduce crate/resource/system counts;
- source-text policy where a type/API/runtime witness can make the invariant
  structural.
