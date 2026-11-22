# Content generations, incremental preparation and development reload

**Scope:** the protocol from an authored artifact to an active content
generation, and the development reload that uses it.
[Extension model](extension-model.md) owns the architecture.
[Construction](construction-and-reconstitution.md) owns materialization.
[Packets](fast-iteration-implementation.md) (I2, I3) select the implementation
steps; [the queue](../queue.md) selects work.

**Architecture revision, 2026-11-21:** the planned contribution and linking
contract is in [capability linking and prepared facets](capability-linking-and-prepared-facets.md).
This page owns how its results form one content generation. The changes below
are planned work, based on source at `6895e5168638`; they are not an
implementation or timing receipt. The SDK/demo publication follow-up below
uses source at `f41218979625` and completes the already planned App-owned
selection cut.

## The contract

An ordinary content edit changes the running result without a rebuild of the
host. A failed edit does not select built-in defaults, publish half a revision,
consume a gameplay reset or erase the developer's reproducible scene. The
system reports the exact artifact and behavior that it uses.

These are DO decisions. Compression, cache layout, the number of preparation
workers and snapshot layout are MEASURE choices. This page sets no latency
value.

## Current shape

The data reload road is implemented.

| Part | Where |
| --- | --- |
| File watch: polls every source that `pack.ron` declares, compiles the pack again from disk and asks for a reload | `ambition_content::content_watch` (`ContentSourceWatch`) |
| Request: one generation in flight; a refused, unchanged or blocked request is a stated `ReloadRequest` variant | `ambition_content::reload::request_reload` |
| Pending candidate: stored, not selected | `reload::PendingGeneration`; the preparation reads `PendingGenerationInputs` (`ambition_platformer2d_runtime::content_identity`), not the App-wide selection |
| Re-preparation: re-requests the active shell route with a minted `ShellRequestId` | `request_reload` |
| Publication at one boundary | `reload::commit_content_generation`; `content_identity::publish_session_content` |
| Module reload (procedural code) | `ambition_platformer2d_runtime::extension_composition` (`load_developer_modules`, `propose_module_reload`, `publish_module_reload`); `ambition_extension_host::reload` |

**Families that take part** (`reload::participates`): movesets, the character
catalog, the smash-fighter facet, boss profiles and encounters, the audio
registries (music and SFX), the adaptive music-cue catalog (2026-10-07), and
the pack-derived families in
`PACK_DERIVED_FAMILIES`: the fighter-brain ladder, encounter waves, the boss
seed library, the validator bands and the item catalog. A changed family that
is not in that list refuses the candidate. A family that is added later is
refused until it declares a publisher.

The quest book takes part too (2026-10-07), and it is the one family that
publishes nothing at the commit: `QuestRegistry` is session state (a teardown
resets it and the next session's first tick fills it from the App's selected
pack and the save), so the new session's registry is the candidate's book with
the player's recorded progress, and editing the live registry would write a
second copy of a fact the next session derives. What the transaction owns is
admission: `candidate_quest_book` refuses a candidate that is not compatible
with the progress the save records (the rule is below).
Measured, not assumed: the registry is empty for the first tick of the new
session, never N's. A step naming a boss, encounter, flag or room that does not
exist is refused by the content-graph judge (below). A removed quest is
allowed, and the save keeps its row. A pack with no quest file compiles and
starts the next session with an empty book (it used to panic there).

The quest check is asked twice. At request time it reads the save as it was,
and the activation gate (`answer_the_publication_gate`) asks the same function
again, with the save as it is then, in the operation that activates the route.
A player whose recorded progress the candidate no longer fits cancels the generation
through the gate's existing `Refuse` (the shell cancels the transaction and
`TransactionEnded` discards the staged generation), instead of the next
session's rebuild clamping their step. The commit stays infallible. Witness:
`a_quest_book_that_loses_its_place_while_the_generation_waits_is_cancelled`,
red before the gate asked (the generation activated over the moved save). The
content-graph judgment is not repeated: it reads the App's `ActiveLdtkProject`,
and the worlds do not reload, so nothing it reads moves while a generation
waits.

### What recorded quest progress is compatible with (2026-10-07)

A save records `(InProgress, k)`: steps `0..k` are done and step `k` is the
objective now. The number means something only against the step list it was
recorded under, so the old check (`k < candidate.steps.len()`) admitted a
same-length candidate that swapped a completed step's condition (the save then
says the player did something they never did) or the current one (the objective
changed silently). `candidate_quest_book` now states the rule, comparing the
candidate with the book the App is playing (generation N):

1. step `k` must exist;
2. steps `0..=k` keep their **conditions** (`QuestStepCondition`);
3. free to edit: step descriptions, every step after `k`, title, summary,
   `auto_start`.

Refused, never clamped or rewritten; the message names the step, whether it was
completed or current, and both conditions. A quest the active pack does not
carry has no baseline, so a save row for it is held to rule 1 only. This is a
policy, not a derivation: a looser one (let the author re-point the current
step) is a decision to make deliberately, and the rule's comment is where to
change it. Witnesses: `an_edited_book_must_keep_the_conditions_of_the_steps_a_player_has_done_or_is_doing`,
`an_edited_book_may_change_text_and_the_steps_after_the_current_one` (control),
`the_pinned_prefix_ends_at_the_current_step`; poisoned by shrinking the range to
`0..current` and to `0..0`. The gate asks the same rule again, so a step the player reaches while a
generation waits is pinned at activation:
`a_step_the_player_reaches_while_the_generation_waits_is_pinned_at_the_gate`
(a future-step edit is admitted, the player reaches the step, the gate cancels;
poisoned by dropping the quest question from the gate, which turns it and
`a_quest_book_that_loses_its_place_while_the_generation_waits_is_cancelled` red).

### Cutscene rows have no recorded owner (2026-10-07)

`CutsceneLibrary` is one `id -> script` map. The publication removes the rows
Ambition owns only while the library still holds exactly generation N's script,
which protects a foreign row from the removal, but its insert half overwrote a
foreign row that reused an id (a review finding; the first arm used a different
id and could not collide). The publication cannot refuse, so the answer is a
collision refusal at admission: `candidate_cutscene_ownership` refuses a
candidate row whose id the library holds as anything other than generation N's
own row, naming the ids (`MoveReload::CutsceneOwnershipRefused`). It is asked
again at the activation gate, because a provider can take an id between the
request and the activation. Dropping an id a foreign row now holds is allowed
and leaves the foreign row. Witnesses:
`a_candidate_cutscene_whose_id_a_foreign_row_holds_is_refused_before_it_can_overwrite_it`,
its control `a_candidate_cutscene_that_replaces_ambitions_own_row_is_admitted`,
`a_candidate_dropping_a_cutscene_a_foreign_row_now_holds_leaves_the_foreign_row`,
and the gate arm `a_provider_that_takes_a_cutscene_id_while_the_generation_waits_cancels_it`
(poisoned by removing the question from the gate only). The gate's refusal is a value, `ActivationRefusal` (`QuestProgressIncompatible` or
`CutsceneOwnershipChanged`), produced by `activation_refusal` and logged as such;
it used to chain both questions through one error and log every refusal as "the
save moved". Witnesses: `a_provider_that_takes_a_cutscene_id_while_the_generation_waits_cancels_it`
(classifies the in-flight race as the cutscene reason, not the save) and
`a_quest_refusal_at_activation_names_the_quest_and_the_save_not_a_cutscene`;
poisoned by reporting the cutscene error as the quest variant. Provider provenance in
the registry itself (so replacement is provider-local rather than refused) is
the larger design; this closes the loss without it.

A request that is refused AFTER admission (an unknown character, a catalog,
the quest book, the content graph) still cancels the generation in flight, while
one refused AT admission (stale base, speculating timeline) does not. The
candidate is the whole on-disk state, so what was in flight describes a disk
that no longer exists, and the late refusals discard the staged reload that
generation shares. Witness:
`a_candidate_refused_by_the_content_graph_cancels_the_generation_in_flight`.

### The content graph is judged at request time

A candidate is judged against the world that is running by the SAME function
startup aborts on, `content_validation::validate_content_graph`, with the
candidate's pack, music registry and character catalog and the App's
`ActiveLdtkProject` (the worlds do not reload). A refusal is
`MoveReload::ContentGraphRefused(errors)`, before anything is staged for the
rest of the transaction. It covers every cross-reference that function covers:
room links and zones, room and encounter music, NPC dialogue, character and
brain ids, quest steps, cutscene bindings and boss music. It costs about 15 ms
(measured). An App with no LDtk project is not asked. Not judged: references the
validator itself does not cover (a Yarn node naming a cutscene, an item id in a
dialogue).

The cutscene library does (2026-10-07), as a pack-derived family with a
shared-registry publisher: the library also holds rows other providers add, so
the publication removes the rows the App's selected pack (generation N) owns,
while the library still holds exactly that script, and inserts the candidate's.
A cutscene that is playing holds its own copy of its script and is not touched.
A candidate that stops declaring a cutscene file removes that file's rows. A
room's `entry_cutscene` that names a cutscene the candidate removed is refused
at request time by the content-graph judge (below).

The music-cue catalog took the same road as the audio registries: it is
admitted at request time (`AdaptiveMusicCatalogRegistry::with_replaced`,
which keeps every other provider's catalog and removes Ambition's when a
candidate declares no cue file), carried by the pending generation, published
at the commit, and the transaction's preparation reads which providers have
cues through `PendingGenerationInputs::adaptive_providers`. A pack without its
cue file compiles; that was measured, not assumed. The director's source cache
is keyed by the asset path, so a cue whose file changed loads the new file.

A candidate that is mechanically identical to the selected pack is
`Unchanged` and requests nothing. A file watcher fires on a save, not on a
change, so this is the usual result.

## Representations and owners

| Value | Owner and contents | Excluded responsibility |
| --- | --- | --- |
| Authored source | Existing Rust, data or media authoring tool; editable values and diagnostic spans | Installed runtime support or live ECS handles |
| Portable section | Pure domain schema; canonical values, references, codec version | Domain callbacks, `Any`, Bevy entities, source-loader policy |
| Artifact manifest | Content pack; immutable section digests, module inputs and requirements | A duplicate character catalog or a service locator |
| Checked domain fragment | Owning parser and validator; typed local data, declarations and unresolved imports | A claim that other selected domains or host support exist |
| Linked domain value | Owning binder; immutable definition with resolved imports and explicit selected inputs | Current actor health, inventory or mutable playback |
| Candidate generation | Lifecycle and content coordinator; complete selected domain view and executable profile | A partially published set of domain revisions |
| Active selection | Session owner; admitted generation and local activation epoch | Process-global mutable selection |
| Live state | Existing body, combat, item, world and extension owners | A second copy inside a content cache |

`PreparedContentPack::lowered` can remain an in-process storage boundary during
migration. Its erased value is not a portable format or proof of complete
linkage. Reuse the owning validator; do not write its rules again in a loader.
A compiler cannot certify installed support. The host checks actual offers.

## Link one selected generation

**Observed:** [`ContentSchemaHandler`](../../../crates/ambition_content_pack/src/schema.rs)
emits declarations, references and a lowered value from `check`. Its aggregate
step sees fragments of one schema. [`compile`](../../../crates/ambition_content_pack/src/lib.rs)
then resolves references after aggregation. The list of resolved references is
separate from the lowered values; the handler has no later binding step that
uses those resolutions. This reuses parsed source, but does not make every
lowered value a fully linked definition.

The character path shows the remaining composition. The schema lowers
`CharacterCatalogData`; the provider builds definitions from the catalog,
facets and selected sheet metadata. `fold_character_facets` names the current
folds. `finalize_character` then resolves catalog and policy values against
`CastAuthorities`. These are different stages of one preparation operation.
Their order and selected inputs should be explicit, rather than reconstructed
by each boot or reload caller. See
[`character_catalog.rs`](../../../game/ambition_content/src/character_catalog.rs),
[`pack_facets.rs`](../../../crates/ambition_characters/src/pack_facets.rs), and
[`prepared.rs`](../../../crates/ambition_characters/src/prepared.rs).

**Planned contract:** declare local symbols and checked fragments, aggregate
under each domain's rule, resolve typed imports in the selected composition,
then let the owning binder produce immutable domain values. Host admission and
live-state compatibility checks follow. The linking plan owns the contribution
API; this page does not add a second registry or a universal intermediate
format.

Each binding pass receives an explicit immutable view of the definitions,
metadata and profile facts it uses. It must not read whichever generation is
currently selected in the `World` halfway through preparation. Its result
retains the identities of those inputs. References that affect simulation are
resolved in that result; source strings remain for diagnostics. The owner
derives the semantic identity and the inspectable reference projection from
the same linked value where possible.

Capture the selected-file snapshot once. Bind mechanically relevant metadata
from that snapshot and retain its actual immutable product versions for the
candidate and active generation. A recorded digest with a path that later
resolves through a changed `published.json` is insufficient. A moving library
head can request new preparation; it cannot retarget an in-flight or active
mechanical binding. The store's existing immutable bundles and generation
lifetimes supply this retention. Pure presentation refresh keeps its own
declared policy.

Content linkage is distinct from body specialization. Equipment, form and
ruleset transitions can specialize already admitted definitions through the
same pure owner functions. They do not create a content generation or
reconstruct a session. Custody, hand occupancy, resource amounts and transient
eligibility remain live state; see the
[binding-time distinction](capability-linking-and-prepared-facets.md#inputs-and-identity).

Symbol references can be cyclic without requiring cyclic computation. Declare
the complete symbol set before binding. If preparation needs another value to
be computed first, record that dependency. Reject an unsupported computation
cycle with its owner and source path; do not add a global fixed-point solver.

A source-only compiler may report missing generated inputs or unavailable host
support. It must distinguish an incomplete diagnostic result from a runnable
generation. A successful parse, a resolved symbol name, and an installed
executor remain separate facts. Local presentation fallback remains valid
where the owner permits it; missing mechanical input cannot become a default
chosen by a different preparation road.

### First migration and deletion targets

1. Apply the linking pilot to a character, one selected facet, and the sheet
   metadata used for its body. Keep the authored source format. Use the current
   validators and owner preparation functions.
2. Produce owner-created checked fragments and linked definitions. Resolve
   provider-relative preset names before construction. Remove the matching
   spawn-time string resolution when all callers consume the prepared ID.
   Do not remove checks while public callers can still supply unchecked data.
3. Make the selected facet contribute its own binding step. Remove its central
   entry in `fold_character_facets` after boot and reload use that step. Keep
   domain behavior out of the coordinator.
4. Pass the selected sheet metadata to the same binder at boot and reload.
   Replace duplicated provider fold orchestration with this operation. Keep
   actual live-state and host-support checks at admission.
5. Seal the complete set of linked values and executable requirements into
   the existing candidate generation. Publish through the current lifecycle
   and hidden-candidate construction road.

Acceptance uses one edit that changes sheet metadata used for body geometry,
a move table, and the manifest that selects them. An invalid cancel target in
the move table must keep the active mechanical definition, identity and
metadata binding unchanged. The newly generated media may remain accepted in
the product store. After the move is corrected, one publication selects the
complete new generation. A host without the required technique must refuse
admission even when source linkage succeeds. This is planned acceptance, not
a reproduced failure of the current reload code.

Keep the implemented sheet-generation and cast-restatement paths. In
[`reload.rs`](../../../game/ambition_content/src/reload.rs),
`stage_cast_from_pack` and `restate_the_cast_from_the_sheets` already share
substantial preparation. Replace their repeated composition as the linked
result becomes available; do not build another sheet watcher or publisher.

## Dependency graph, not one rebuild switch

A generation is a set of independently identified immutable sections. Each
domain adapter declares the inputs used by its preparation. Keep three edge
meanings distinct: a named definition uses another definition; a computation
observes an input; and a producer derives a product from source. The
[asset-library plan](asset-library-editor-and-delivery.md#dependency-evidence-has-three-meanings)
owns the producer side. Shared inspection can join these relations, but a list
of resolved content names is not a complete invalidation graph.

A cache key includes the source or section digest, the codec and validator
versions, the dependencies that preparation used and the selected profile facts
that affect the result. Runtime hydration also includes the host contract and
build identity. Caches can reuse immutable results across Apps; activation
authority is App-local.

For a move edit, follow the real dependency closure:

```text
move values -> referenced techniques/moves/mechanical assets
            -> affected prepared character/action definitions
            -> selected generation -> dependent live bindings
```

Do not prepare unrelated rooms, audio or characters again because one scalar
changed. Do not ignore a dependency to get that result. A domain with no
precise invalidation description prepares its own section and its reported
dependents again; it does not claim an incremental hit. Record why each section
was rebuilt or reused.

The graph includes deletions, reverse references, collection membership and
failed optional lookups that affected preparation. A newly available default
can change a result even when none of its earlier positive references changed.
Record the selected default and relevant environment or profile facts, or
prepare that domain again conservatively. If you remove a move that a character
uses, that character fails or changes in the same candidate. A missing required
section cannot fall back to a compiled table. A field with uncertain mechanical
influence is mechanical until its owner shows otherwise.

Reuse applies to preparation, not publication. Reusing ten domain values and
rebuilding one still yields one complete candidate with one admission decision
and one visibility boundary. It does not authorize eleven independently active
domain generations. The disk-cache and certificate proposal remains under the
explicit hold in [prevalidated content artifacts](prevalidated-content-artifacts.md);
this plan does not open that hold.

**Example:** a change to diagnostic source spans changes diagnostic
provenance, not mechanical identity. A change to attack duration changes the
mechanical digest, invalidates the dependent preparation and changes the
observed trace. A change to a sprite that authored collision uses does the
same. A change to a purely visual texture need not restart simulation if its
owner proves that classification.

## Identity and canonical form

Extend `PreparedContentBuilder`; do not add a parallel hash authority. Keep
these meanings separate:

| Identity | Purpose |
| --- | --- |
| Source revision and provenance | Tell which author input made a value |
| Portable section digest | Reuse and verify exact canonical domain data |
| Mechanical generation digest | Bind selected definitions, code, schemas, ports and execution policy |
| Host compatibility identity | State the engine, build and target policy that execution needs |
| ContentEpoch | Refuse stale App-local activation plans |
| PeerContentIdentity | State which content, in the vocabulary two peers share |
| Gameplay session and rollback timeline | Own live state and the history that is replayed |
| Construction attempt | Own candidate work and cleanup, not durable object identity |

**The two content rows are one pair. Mint them together.** `ContentEpoch` is an
App-local activation count and does the staleness job. `PeerContentIdentity`
names which content; two Apps that hold the same prepared definition agree on
it, however many times each one reloaded. `ContentBinding::Content` carries
both. The peer projection of `TransactionId` keeps the identity and drops the
epoch.

Rules that the code enforces:

- `ContentBinding::Content` is `#[non_exhaustive]`
  (`crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs`). Outside
  that crate a caller uses `ContentBinding::content(..)` or
  `ContentBinding::content_unstated(..)`. The half-stated form does not compile.
- "Content-derived, but no road stated which content" and "not content-derived"
  are different answers. `ContentBinding::peer_content` returns `Option` to keep
  them apart.
- A construction plan has two bindings. `expected_live` is the generation that
  the plan is committed into (the staleness comparison at the commit boundary).
  `incoming` is the generation that the plan's content came from (the stamp on
  each root's `TransactionId`).
- Only a content replacement can state two generations.
  `ActorConstructionContext::for_live_room_construction` takes one binding;
  activation, door, death, reset, prefetch and fixtures use it.
  `ActorConstructionContext::for_content_replacement` takes `expected_live` and
  `incoming` by name. Both delegate to one private body.
- A room behind a door is built from the generation that runs, so that
  generation is its incoming identity. Ordinary roads pass
  `ActiveContentBinding::live_or(active, content_unstated(..))`: a session that
  publishes a binding always stamps it, and a headless fixture that publishes
  none keeps an honest gap.

Witnesses: `an_ordinary_room_transition_stamps_its_roots_with_the_session_content`
(asserts on `TransactionId::peer_content_term`, the one term that the rule
decides; the full projection also folds in the room, so two rooms always
differ) and the disagreement arm of
`an_edited_pack_reaches_the_cast_the_shipped_composition_plays` (one process
at two prepared fingerprints).

**An agreement guard needs a disagreement arm.** An assertion `f(a) == f(b)`
passes for every `f` that discards information, including a constant. For each
agreement guard, ask what a constant would do. If the constant passes, add an
arm that asserts disagreement. A guard of this shape
(`two_hosts_at_different_content_epochs_share_one_construction_provenance` in
`game/ambition_app/tests/id_peer_audit.rs`) stayed green while every root was
stamped with a constant content term.

Canonical bytes specify tag order, integer width, sequence order, string
encoding, map order, numeric restrictions and unknown-field behavior. Do not
hash Rust memory or default `Debug`/`Hash` output. Keep `-0`, NaN and infinity
handling explicit at the owning numeric contract. Required unknown fields or
sections refuse admission. An optional field can be ignored only under a
versioned rule that excludes simulation influence.

Source timestamps, local paths, temporary IDs and file enumeration order are
not mechanical identity. Diagnostic metadata can have its own digest. Identical
input with the same selected preparation contract is a no-op, not a new epoch
or timeline.

Persistent saves do not use ContentEpoch or executable addresses. Same-build
network compatibility, save-schema compatibility and authoring-format
compatibility are separate contracts. A code digest gives identity, not trust
or safety.

## Producer, source transport and complete candidates

Use the existing source and asset resolver for local, packaged and web
transports. An authoring tool publishes immutable section and module objects
and then a complete manifest. A watcher only requests inspection; it is not
ordered simulation input.

The reader verifies lengths, versions, dependency identities and complete
bytes before it builds a candidate. Missing or partly written objects report
pending or invalid; they never activate a mixture.

A candidate carries its requested source revision, base ContentEpoch, selected
profile identity and an attempt key. Superseded work can finish but cannot
publish.

A second request while one is in flight SUPERSEDES it (2026-10-07; it used to be
refused as `AlreadyPending`). The newest candidate that passes admission wins:
the in-flight generation is cancelled by the road the publication-lease breaker
uses (its content half dropped: staged cast, claim, activation hold and gate
registration; then `ShellCommand::CancelPending`), and only then is the new one
staged, with its own request identity. Adoption matches on that identity, so the
cancelled transaction's late events name a request that is no longer pending
and are ignored (`a_cancelled_generations_late_end_does_not_discard_its_successor`).
A candidate refused at admission (stale, a speculating timeline) cancels
nothing. A candidate equal to the live content while one is in flight is a
revert: the in-flight generation is cancelled and nothing replaces it
(`ReloadRequest::CancelledInFlight`). The answer to a superseding request
carries the identity it superseded. Measured while writing it: without the
explicit cancel, a superseding request already supersedes the shell's
transaction (`ReplaceWith` does), so what the cancel adds is the content half
(a leaked activation-gate registration for the old request, which nothing
released) and the revert case, which has no `ReplaceWith` at all. Not measured to
matter: a staged cast revision of a superseded generation did not leak into its
successors even without the discard.

## Activation state machine

The names below are roles, not required Rust type names.

```text
Requested -> ReadComplete -> Prepared -> Admitted -> Sealed
          -> BoundaryGranted -> CandidateReady -> Published -> Retired
```

| Transition | Required input and invariant |
| --- | --- |
| ReadComplete | Immutable complete manifest and verified dependencies |
| Prepared | Domain validators, aggregation and typed binding make the linked values from one selected input view; active registries and live state do not change |
| Admitted | Installed ports, capabilities, schemas and resource limits satisfy the requirements |
| Sealed | Complete replacement view plus base epoch, profile and reconstruction policy |
| BoundaryGranted | The local lifecycle owner authorizes the operation; a stale candidate or a timeline that another owner holds refuses |
| CandidateReady | Materialization and state mapping are verified without publishing partial state |
| Published | One active selection changes; all participating owners and the new timeline name it |
| Retired | Old calls, readers, snapshots, effects, tasks and code-owned destructors are not reachable |

Publication is a simulation visibility barrier. No simulation, event reader,
observer or presentation extractor sees half of the committed generation. A
deferred-command flush does not prove this; the owner names the systems and
hooks that its barrier covers.

Snapshots replay only with their bound mechanical generation. Remote sessions
keep their admitted generation; next-session packaging is not online hot
migration.

Room construction for a reload uses the hidden-candidate road
(`construction-and-reconstitution.md`): every root is built as an inactive
candidate, verified and published or dropped. There is no destructive phase
before the decision.

## Reload policies

Each family declares one supported action. Tooling reports it before
activation; it does not choose between reset and retain by a heuristic.

| Class | Allowed action and proof |
| --- | --- |
| Diagnostic-only | Refresh diagnostics; mechanical generation and timeline do not change |
| Presentation-only | Use existing asset reload; prove that no simulation reader depends on the changed bytes |
| Immutable mechanical definitions | Replace at a local barrier after every affected live binding has a declared retain, rebind or retire policy |
| State, schema or body-construction change | Reconstruct the selected development scenario through the lifecycle and domain restore paths |
| Fundamental host or port change | Rebuild the engine or profile; report this as engine iteration |
| Unsupported migration | Refuse, or require an explicit scenario restart; never silently discard durable progress |

The current mechanical path is **scenario reconstruction**: the active route is
prepared again against the candidate. It is not an arbitrary save migration or
a swap in the middle of an attack. A checkpoint that is usable with generation N
is not automatically usable with N+1.

A faster path for immutable definitions must state what happens to active
`MovePlayback` references, cooldowns, attached capabilities, prepared collision
facts and derived caches. Either finish or retire the affected occurrences
before publication, or include the retained definitions in the admitted
generation.

A supported refusal keeps the old selection and scene. If the implementation
can only recover by building the pinned old scenario again, it reports
**recovered**, not **unchanged**. If it can do neither, it reports
**failed/stopped** and keeps normal simulation disabled.

## Tool operations

Use the existing CLI and workbench routes. The operations are: validate a
candidate; explain affected sections; request activation; inspect status;
inspect the active generation; replay the selected scenario. A status result
names the attempt, source revision, base, active and candidate identity,
policy, stage, refusal owner and reason, rebuilt and reused sections, and
whether the prior scene is unchanged, recovered or stopped.

## Open work

- **A generation that drops a worn id** (WEAR-REFUSES remainder, Q103). When
  the cast changes and no longer holds the id a live body wears, the body keeps
  its kit from the old generation and the refusal is reported once. That mixes
  generation N and N+1. The repair is probably at admission: a generation that
  drops a live body's character is not admitted.
- **The home body of an unprepared starting id.** `session::setup` still builds
  the home body (an empty kit, named after the id, reported). Refusing it is a
  session-admission question: the session then has no body to drive.
- The SDK/demo reload path needs coherent publication of prepared domain
  products as well as App-owned selection. The bounded cut below completes
  that contract. Support for restated schemas inside a live session remains
  separate work; it is not added by fixing publication between sessions.
- Measure source read, changed-section preparation, candidate construction,
  activation, scenario reset and first observed behavior separately (M0).
  Fixtures FI1-FI4 in [acceptance](fast-iteration-acceptance.md).

### SDK/demo publication — admit owner products before selection

**Source follow-up, 2026-11-21 (`f41218979625`).** The current SDK watcher in
[`content::live_reload`](../../../crates/ambition_platformer2d/src/content/live_reload.rs)
selects the new pack in `publish_the_staged_pack_between_sessions`, increments
its publication count, and then invokes schema restaters. A restater returns
`()`. Audio, owned-row and cast callbacks can catch a domain refusal and keep
the prior product. The watcher still reports `published`. Main Ambition's
prepared generation path already separates these stages; this gap is in the
reusable SDK/demo path. The failure trace below is deduced from source, not
a fresh runtime reproduction.

For a small witness, compose two valid audio providers. Edit one pack so that
it claims the other provider's track id with a different asset path, and
change a live rules value in the same revision. Pack compilation can succeed:
the foreign App catalog is not part of that local schema check. The watcher
selects the pack; audio's
[`AudioCatalogRegistry::with_replaced`](../../../crates/ambition_audio/src/catalog.rs)
then refuses the composition. New rules and old audio remain selected, yet
the receipt says published. Moving `EmbeddedPack.current` into an App resource
alone does not fix this sequence.

**Decision:** a schema is an input; the publication unit is an owner's
prepared product. Select affected products from the changed input-schema set,
prepare each product once, and admit the complete pack replacement before any
visible publication. Keep the existing owners and retained generation handles.

| Product | Existing inputs and owner operation |
| --- | --- |
| Provider audio | Music and SFX schemas jointly produce one composed audio catalog. `with_replaced` builds the candidate; `publish_revised_audio` installs it. |
| Provider cast | Character catalog, movesets and fighter data jointly produce one admitted cast with its candidate catalog. Keep character definition/admission in `ambition_characters`. |
| Provider-owned presentation or rule rows | Each existing catalog admits its owned replacement rows. Its prepared result includes removals and the ownership bookkeeping that will become current at commit. |

Current audio registration attaches a whole-audio callback to each schema;
an edit to music and SFX runs it twice. Cast registration likewise stages the
whole cast for each changed input schema. The product boundary removes those
duplicate operations. This is a source-visible work reduction, not a measured
reload-latency result. Product registration states its supported inputs and
absence/removal rules; an unsupported schema still refuses or requires the
existing restart policy.

#### Prepare privately; publish under the existing authority

For the first implementation, prepare and commit one watched pack at a time
inside the existing exclusive between-session publication operation. The
next pack prepares against the composed catalogs that are current after the
prior successful commit. Do not prepare all watches from the frame's initial
catalog clones and later install them: a whole-registry candidate could erase
another pack's accepted change. A refusal of pack B does not undo a committed
pack A. A revision that only validates with changes to several packs needs an
explicit grouped candidate through the existing generation path; sharing a
frame does not create such a group.

Preparation reads one frozen pack and the current App context. A product
that depends on another changed product reads its candidate value. In the
cast, build the candidate character catalog before folding/admitting its
definitions. These known owner calls and their order are sufficient; do not
add a configurable dependency graph or another generation coordinator.

Do not call today's `CastReload::restage` as a preflight. Its
[`staging road`](../../../crates/ambition_characters/src/prepared.rs)
writes and drains the shared `StagedCastRevision`, and its failure path can
remove that resource. Factor the existing admission core to take an explicit,
privately owned revision buffer, read-only current context and candidate
catalog, and return the existing admitted revision. Keep private character
staging types in their owner. Existing staging adapters can call this same
core; preparation must not consume another caller's edits or temporarily
replace live resources. Prepare shared cast vocabulary once per candidate,
as the main content path already does, rather than once per character.

Apply the same narrow split to other rows. For example, `PropSheets` can
admit its rows on a private clone; asset realization and its existing
placeholder behavior follow authoritative publication. Pure audio candidate
construction already exists. Keep each owner's admission rules there; the
SDK coordinator must not duplicate catalog collision or cast validity rules.

After every affected product admits, install the pack selection and all
authoritative prepared values at one visibility boundary. No selected product
can return an ordinary content refusal after the first visible mutation.
Refresh presentation projections, apply the existing timeline policy, update
owned-row/built-row bookkeeping, and only then advance the publication count
and success receipt. The boundary must cover observers and hooks that read
related catalogs; an exclusive function alone is not proof that synchronous
hooks cannot see a partial selection. Restoring the old pack pointer after a
callback failure cannot undo catalogs an earlier callback already changed.

If preparation later spans frames, retain the domain facts it used and
recheck them before commit. The first synchronous cut does not need a new
epoch system for those facts. It does need one consistent base during
preparation and publication. Read-only source fingerprints remain a no-op
path; gameplay never walks the product dependency set.

#### Complete App-owned selection without ambient leaf lookups

The remaining
[P5 step 4 cut 2](runtime-authoring-audit-2026-11-14/execution-plan.md#p5-complete-revision-persistence-and-target-loading)
already replaces the process-wide mutable pack selection. Keep immutable
compiled source reuse process-wide where valid; select and retain the active
generation in the App/session that admitted it. App A cannot publish rules or
catalogs into App B merely because they use the same embedded source.

Bind a typed rules view once at an operation or system boundary. Pass normal
borrowed rows into pure helpers; retain a generation handle when a row must
outlive that call. Do not require every leaf accessor to query the World.
Current [`EmbeddedPack::prepared`](../../../crates/ambition_platformer2d/src/lib.rs)
takes a shared lock and clones an `Arc` for each call. Operation-bound views
can remove that repeated selection work while making the App owner explicit.
No runtime saving is claimed until the relevant caller path is measured.

#### Delivery, deletion and acceptance

1. Add the valid-pack/foreign-audio-collision witness above through the
   actual watcher publication path, including the simultaneous rules edit.
   A refusal keeps the old pack, rules, audio and publication count, and
   reports the refusing owner/id.
2. Prepare one product per affected owner and retain its admitted result.
   Extract private cast preparation as described above. Delete duplicate
   per-schema whole-product callbacks and swallowed post-publication
   refusals. An edit to both audio schemas prepares/commits audio once; a
   character-plus-moveset edit prepares/commits the cast once. Check removal
   of an owned row, and preserve foreign rows.
3. Publish selection and products together. A later product refusal leaves
   all affected products and their ownership trackers unchanged. Two watched
   packs that publish in one frame retain both accepted changes; a held
   candidate, if introduced, cannot overwrite a foreign change made after
   its admission.
4. Finish App-owned operation views through the existing demo consumers.
   Use two Apps with the same embedded pack: reloading A must not change B's
   selected rules, catalogs, session identity or rollback baseline. Retained
   old-generation rows remain valid until their last reader retires.

Preserve `only_live`, local timeline rebase, remote generation and shell
preparation policy. This cut does not permit cast/catalog restatement inside
a live session or lift the prepared-artifact serialization hold. Acceptance
uses a few production-path refusal/publication cases, not a new reload
framework or a matrix over every schema.

### Sprite source identity and live presentation refresh

The first-read development `OnceLock` tables in
`ambition_sprite_sheet/src/baked_sheet_rons.rs`,
`baked_part_flipbooks.rs`, and the parsed rig cache prevent changes from
reaching an already running process. This is not a claim that the October 11
*no Rust recompilation* sprint failed; that path is landed and useful.
It means a programmatic 2D authoring experience still needs a separate
candidate/live-presentation activation path.

**Collision half landed (2026-10-12):** `companions` and `sheet_manifests`
refuse a key that two files give, and the refusal names both files
(`one_file_for_each_key`); the build script and the development read use
the same scan, so both refuse. The shipped tree has none (measured: 644
tiered parts keys, 161 base, 9 rigs, 884 sheets, no collision). Witness:
`ambition_sprite_sheet sprite_tree a_key_that_two_files_give_is_refused`
(two folders, a quality tier, the sheet scan; one file a key is the
control); the old keep-the-first poison fails it. The text below is the
case as found.

The source scanner has a more immediate deterministic-validation issue:
`ambition_sprite_sheet/src/sprite_tree.rs:85-104` collects companion names,
sorts only by logical key and silently `dedup_by` key. Two directories
that publish the same target can silently select one file. The adjacent
sheet-manifest scanner returns duplicate keys without an explicit
source-aware conflict policy. Reject collisions with both source paths,
or require explicit, inspectable pack overlay precedence; do not let file
traversal order or sort ties decide the winner. Add a two-source collision
fixture for both manifests and companions (including quality tiers).

**Measured 2026-10-16 (the read chain in a development build).** A
sprite edit reaches a freshly started development build with no compile:
the tree is read off disk (`published_tree::owner()`, the monolith's
`assets/`). A PNG edit reaches a running game only under the opt-in
`dev_hot_reload` feature (`bevy/file_watcher`, off in `desktop_dev`;
nothing calls `AssetServer::reload`). A sheet text edit does not reach a
running game at all. The caches that hold it for the life of the process:

- Text: `baked_sheet_rons::sheet_rons` (sheets), `baked_body_rigs`,
  `baked_part_flipbooks` (the file list), `baked_portrait_rons`,
  `sprite_packs`.
- Parsed: the record index (`character::sheets::record_index`) and its
  mechanics digest, `attack_hitbox`'s `SheetRegistry`,
  `shared_baked_sheet_registry`, the landmark tables and their digest
  (`baked_landmarks`), the decoded rig cache (`character::rigged`), the fx
  and portrait indexes, the pack catalogs; and the `SheetRegistry` and
  `PortraitSheetRegistry` resources, filled once at `Startup`.
- Binding: a bound body rebinds only when its quality tier changes
  (`upgrade_actor_sprites`, `bind_worn_character_presentation` and the two
  tier refresh systems), and `materialize_declared_character_sprite` skips a
  sheet that is ready at the active tier. Part-drawn bodies rebind on a new
  flipbook `Arc` (`bind_rigged_presentations`), but its `posed_by_target`
  local is never cleared. (Both fixed 2026-10-16: see below.)

The same cached records feed simulation: posed body geometry and inset
hurtboxes, body seeds and collision sizes (`catalog_join`), attack
volumes (`attack_hitbox`), a boss's size
(`boss_sprite_metrics_from_registry`), landmarks, and the content
fingerprint (`landmarks_digest`). So a live sheet edit is not a
presentation-only change. The road it needs: one sprite generation that
publishes every cache above together (each `OnceLock` becomes a published
`&'static` generation, as `EmbeddedPack::prepared` is), published only when
no session is live (as the demo packs' restated schemas are), plus a
generation key beside `BoundSpriteQuality` so that a bound body rebinds at
the same tier, and `AssetServer::reload` of the changed pages.

**Landed 2026-10-16, the sheet manifests.** The sheet text, the record index,
the file-root registry and the mechanics digest are one published generation
(`baked_sheet_rons::{current, publish_sheet_generation}`; witness
`ambition_sprite_sheet --test sheet_generation`, red at "the record index is
not the published table's" with the index kept in its own `OnceLock`).
`content::sprite_reload::SpriteSheetReloadPlugin` (installed by
`ambition_app` where content is read off disk) watches each manifest and each
page image from a background thread. One look at the published tree (a stat
of each sheet, rig, flipbook and page file) is 0.2 to 0.4 s in a debug build,
so the thread looks only when the operating system reports a change in a
sprite folder (`notify`; a burst of reports is one look), and once a second
only where no report is possible (witness `sprite_watch_reports`: one look in
three quiet seconds, and one look for one save; red with the reports off, at
"the watch looked while no file changed"). A changed page image is
loaded again at once. A changed manifest makes the same thread read the whole
table again (`prepare_sheet_table`). The preparation parses only the sheet
files whose text changed and checks only the changed rigs; each other file's
records and texts are the current generation's (a text read is kept for the
life of the process, so a read keeps only what changed). Witness: the
`parsed_now` and text arms of `sheet_generation` (0 files parsed for an
unchanged table and 1 for one edit, red at 1220 with no carry-over; red at
"a read of the tree kept a new text for 1220 files" with no reuse). Measured
2026-10-16 in a debug build, a loaded box (`sprite_sheet_reload --ignored`):
one read and preparation with the flipbooks decoded, median 1136 ms (2.1 s
before the carry-over); from an undecoded generation, 2229 ms. The parsed table waits for the end of the session, then it is
published whole (`publish_prepared_sheet_table` plus the resource clone,
median 18 ms on the frame), the `SheetRegistry` resource is stated
again, each resident character realization is retired so that the next
session builds it from the new records, and the changed sheets' pages are
loaded again. A table with a file that does not parse is refused with the
file named. Witness `ambition_platformer2d --features content_pack --test
sprite_sheet_reload` (red at "an edit was published inside a session" with
the session gate removed, and at the `SheetRegistry` assertion with the
resource not stated again). A cast reads its body scale, inset hurtboxes and
rig from its sheets when its definitions are made, so the publication also
states again each cast registered in `sprite_reload::SheetDerivedRestaters`
(each `PackCast`, and Ambition's cast from its content watch), before a frame
can build a body. Witness `ambition_platformer2d --features content_pack
--test sheet_derived_cast` (red at "the cast keeps a body scale made from
sheet generation 1", 2.0 kept against 4.0, with the pack cast's restater not
registered); the shipped composition's casts are each named by
`each_cast_of_the_shipped_composition_is_stated_again_with_the_sheets`
(app_it; red with Ambition's restater removed). Ambition's restater has no
behaviour witness of its own: it stages the live catalog's cast as a facet
edit does (`stage_cast_from_catalog`). The body rigs published beside the
sheets (`*_body_rig.ron`) are a part of the same generation since
2026-10-16: the watch looks at them, a rig that does not parse or prepare
refuses the table with the rig named (a reader of a rig panics on one it
cannot read), and the cast restaters state each cast's rig again (witnesses
`sheet_generation` and `sheet_derived_cast`, red with the rig table kept in
its own `OnceLock`, with no rig check, and with the rigs not watched). The
part flipbooks (`*_parts.ron`, each quality tier too) and the landmark tables
projected from them are a part of it as well
(`baked_part_flipbooks::PartTables`): the preparation decodes each
full-resolution table off the frame (a table whose file kept its
modification time is taken from the current generation), refuses one that
does not decode with the file named, and projects the landmarks again when a
flipbook changed; the watch reloads an edited flipbook's atlas pages
(`<target>_parts*.png`) at once. The old generation drops its decoded tables
at the publication. Witness: the flipbook arm of `sprite_sheet_reload`. A
tier table is decoded where a sheet realizes at that tier, and that reader
panics on one that does not decode; so the preparation decodes each tier
table whose file changed since the current generation read it, and refuses
one that does not decode with its path under `assets/` named (2026-10-17;
the tier arm, red with the tier tables not checked). An unchanged tier table
costs a stat: the preparation median did not move (2412 and 2046 ms
before, 2140 and 2008 ms after, two runs of seven samples each). The portrait manifests (`*_portraits.ron`) and
the sprite pack catalogs (`sprite_packs/<tier>/ultrapack.json`) are parts of
the generation too: a manifest that does not parse or a catalog that does
not parse or validate refuses the table with the file named, the
`PortraitSheetRegistry` resource is stated again at the publication, and the
portrait vocabulary a cast is prepared against is the generation's (the
portrait and catalog arms of `sprite_sheet_reload`; red with the resource
not stated again, with no portrait or catalog check, and with the catalogs
not watched). The effect sheets and the authored effect index
(`fx::fx_sheets`, `fx::authored_effects`) are built from each generation's
own records (they were filled once from the first records, so a published
effect row's timing was not the index's; witness: the effect arm of
`sheet_generation`, red with the index in its own `OnceLock`). A part-drawn
body's posed parts (its tracks bound to its body rig) are bound again at each
change of the assets, so a body realized from a new generation is not posed
with the old rig (`a_new_flipbook_binds_its_own_posed_parts`, red with the
cache kept). A table that changes only where frames are drawn from (the
pages, the rects, the label band: a repack) is published at once, also
inside a session (2026-10-16): the watch loads a changed page again at once,
so the old rects cut the new pixels until the session ended. The look decides
it off the frame (`PreparedSheetTable::changes_mechanics_of`: the mechanical
projection of the records, the landmarks, the body rigs and the portrait
manifests, each compared with the live generation); such a table states no
cast again. Each resident realization records the sheet generation it was
made from, and one of an older generation is stale as one of another tier
is (`CharacterSpriteAssets::is_current_realization`), so the residency
convergence makes a worn one again in place and retires the others (the
publication's retire of every realization is deleted). Each binder stamps
the realization it bound (`BoundSpriteQuality::of`: the tier and the atlas
layout, which each realization adds anew), so a body binds again to a new
realization at the same tier. Witnesses: the in-session arm of
`sprite_sheet_reload` (red with every table judged mechanical, and with
none), the mechanics arms of `sheet_generation` (each comparison red when
removed, except the landmarks), its staleness arm (red with the generation
not compared), and `a_body_binds_again_to_a_new_realization_at_the_same_tier`
(red with the tier alone stamped). The prop sheets made from a changed
sheet (`PropSheetSource`) are made again at the publication
(`reload_prop_sheets_of_targets`), and the prop binder binds each prop again
to its new realization (the prop arm of `sprite_sheet_reload`, red with the
reload removed). Each resident effect sheet of a changed sheet is made again
too, at its tier (`reload_fx_sheets_of_targets`; an effect is drawn from the
set when it spawns): the effect arm, red with the reload removed. The
intro's props were the last prop rows with their own spec (a `Catalog` source
and a Rust table); they are pack rows since 2026-10-16. The page reload has no witness of its
own. Selected libraries refuse duplicate sheet, rig, and flipbook identities with
both source paths named. A failed candidate keeps the accepted generation, and
the watch remains active. Native reload tests cover generation admission and
retention of the accepted files and metadata.

**Future live path:** the source scanner reports the changed logical key
and a canonical source fingerprint. Preparation decodes the changed sheet,
landmarks, part frames/flipbooks and dependent rig/atlas descriptions into
a candidate presentation generation. Validate name/key uniqueness,
references, atlas page dimensions and material dependencies. Publish the
whole dependent visual slice coherently or leave the old visuals alone
and report refusal. A successfully activated visual revision rebinds
all currently visible consumers, including rigged bodies, multiple view
instances and quality variants. Keep a reasoned policy for switching
simulation-authored physical geometry if it depends on the sheet; **do not**
secretly update hitboxes mid-timeline under the label 'visual reload'.
Static-content and packaged builds still consume baked immutable tables.

**Witnesses:** duplicate logical sheet key, duplicate companion key, bad
RON edit leaving live visuals unchanged, valid edit changing one visible
character without relinking, two instances rebinding together, replay after
rewind, and quality-tier switch. Record cold source-read versus live
activation times. Do not call it 'hot reload' if only the next process boot
sees the update.

## Forbidden regressions

- A reload that publishes one family at generation N+1 while another family
  that changed still serves N.
- A struct literal of `ContentBinding::Content`, or a default
  `PeerContentIdentity` that stands in for an unstated one.
- An ordinary construction road that can state two different generations.
- An agreement guard with no disagreement arm.
- A watcher event treated as ordered simulation input.
- A failed candidate that falls back to a compiled table.
