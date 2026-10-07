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
registries (music and SFX), and the pack-derived families in
`PACK_DERIVED_FAMILIES`: the fighter-brain ladder, encounter waves, the boss
seed library, the validator bands and the item catalog. A changed family that
is not in that list refuses the candidate. A family that is added later is
refused until it declares a publisher. The cutscene libraries, the quest book
and the music-cue catalog do not take part today.

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
publish. Today a second request while one is in flight is refused
(`ReloadRequest::AlreadyPending`), not superseded.

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

- The cutscene libraries, the quest book and the music-cue catalog do not take
  part in reload.
- Supersession of an in-flight generation through a real cancellation.
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
