# Content generations, incremental preparation and development reload

**Scope:** the protocol from an authored artifact to an active content
generation, and the development reload that uses it.
[Extension model](extension-model.md) owns the architecture.
[Construction](construction-and-reconstitution.md) owns materialization.
[Packets](fast-iteration-implementation.md) (I2, I3) select the implementation
steps; [the queue](../queue.md) selects work.

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
(poisoned by removing the question from the gate only). Provider provenance in
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
| Prepared domain value | Owning validator or hydrator; resolved immutable definition | Current actor health, inventory or mutable playback |
| Candidate generation | Lifecycle and content coordinator; complete selected domain view and executable profile | A partially published set of domain revisions |
| Active selection | Session owner; admitted generation and local activation epoch | Process-global mutable selection |
| Live state | Existing body, combat, item, world and extension owners | A second copy inside a content cache |

`PreparedContentPack::lowered` can stay an in-process hydration container. Do
not make it the wire format. Extract the pure validator that both frontends use
when necessary; do not write its rules again in the loader. A compiler cannot
certify installed support. The host checks actual offers.

## Dependency graph, not one rebuild switch

A generation is a set of independently identified immutable sections. Each
domain adapter declares its inputs and references. The pack supplies one graph
of these requirements, not another evaluator for the domain's own rules.

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

The graph includes deletions and reverse references. If you remove a move that
a character uses, that character fails or changes in the same candidate. A
missing required section cannot fall back to a compiled table. A field with
uncertain mechanical influence is mechanical until its owner shows otherwise.

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
| Prepared | Domain validators make values; active registries and live state do not change |
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

- Demo packs do not reload in a running demo.
- Measure source read, changed-section preparation, candidate construction,
  activation, scenario reset and first observed behavior separately (M0).
  Fixtures FI1-FI4 in [acceptance](fast-iteration-acceptance.md).

## Forbidden regressions

- A reload that publishes one family at generation N+1 while another family
  that changed still serves N.
- A struct literal of `ContentBinding::Content`, or a default
  `PeerContentIdentity` that stands in for an unstated one.
- An ordinary construction road that can state two different generations.
- An agreement guard with no disagreement arm.
- A watcher event treated as ordered simulation input.
- A failed candidate that falls back to a compiled table.
