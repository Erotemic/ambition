# Fast iteration implementation packets

**State:** target packet catalog, not a completed feature or a second queue.
[The queue](../queue.md) selects execution. The [extension model](extension-model.md)
owns decisions, the [execution contract](extension-state-and-execution.md) owns
state semantics. [Generation/reload](content-generation-and-reload.md) and
[domain contracts](extension-domain-contracts.md) own their protocol details.
[Acceptance](fast-iteration-acceptance.md) defines FI1-FI10, and
[evidence](extension-iteration-evidence.md) owns M0-M3.
Baseline: `d81a7ae1d2db1fc5caa49efc807a39ea6b1ca266`.

## Start here

Read the model plus the protocol and fixture sections for the selected packet
before changing a runtime seam. Do not ingest every long plan by default. Re-read the
named source on the actual working head. Record the active writer, installer,
caller, lifetime, generation and cheapest existing behavioral test for that seam.
Do not treat an old paragraph about unfinished A11 validation as current source.
Do not regenerate the entire repository index or run the entire workspace suite
just to begin this work.

The paths explicitly marked **proposed** below do not exist at the baseline.
Create them only in the packet that owns them. Exact package/file names may
change after an import-cycle check; the dependency direction and tests may not.
Update these locators in the same commit when an implementation chooses a name.

## Dependency graph and delivery cuts

```text
I0/M0 baseline and measurement helper ---------------------> report gains
    (parallel; no blanket design gate)

I1 pure move authoring -> I2 loadable data artifact -> I3a candidate coordinator
                                                      -> I3b bounded construction (A10)
                                                      -> I3c repeatable reload
                                                        |
I4 procedural SDK + static semantic reference -> I5 state + real rewind
                       |                         |        |
                       +-------------------------+--------+
                                                 v
                                      I6 backend experiment (M1)
                                                 |
                                      I7 selected procedural path
                                                 |
                                      I9 external/flagship acceptance

I5 -> M2 -> I8 measured runtime/snapshot optimizations -> I9 cost evidence
```

**First useful delivery:** I1-I3 plus M0 evidence for a real move edit. Do not
wait for a scripting language, state-store optimization or whole-engine carve.
**Second useful delivery:** I4-I7 with one migrated boss mechanic and a graph-based
extension. I8 is not a prerequisite for a correct prototype, but measured budget
regressions must be resolved before claiming production readiness.

| Existing program | Exact dependency | Not a dependency |
| --- | --- | --- |
| A6 definitions/materialization | Use its field census to place any moved field | Renaming/extracting all characters or the actor SCC |
| A9 minimal profiles | Reuse resolved-closure guards and independent consumer method | Completing every facade capability profile before pure authoring |
| A11/A12 techniques | Preserve current installed admission, reference validation and occurrence rules when loading moves or providing techniques | Rebuilding the already implemented flow interpreter |
| A2/contact and A4/body | Their published request/observation contracts govern corresponding ports | Completing every combat or movement feature before the data artifact |
| A1/checkpoints | Use its durable restore/confirmed lifecycle road for required saved state | A new persistence engine inside the extension host |
| A8/multiple worlds | Two-instance FI9 is a planned requirement; scope portable records consistently with their actual owners | Full concurrent-world execution before I1-I3 |
| A10/bounded candidate construction | I3b needs one safe migrated reconstruction path; verify candidate materialization before retirement | Arbitrary World cloning/undo, all-plugin migration or a full streaming system |

## I0 - record real iteration costs without blocking clean boundaries

**Input:** current source and a configured developer machine. **Class:** MEASURE.
Read `Cargo.toml`, `.cargo/config.toml`, `AGENTS.md`, the B7 build plan and M0.
Use existing dependency/absence instruments before adding a new helper.

1. Record the exact normal move-edit and procedural-edit commands, features,
   host process, assets, cache state and machine. Preserve a representative input
   trace that makes each selected edit observable.
2. Obtain no-op and paired edit results using M0. Separate host relink from
   unchanged dependencies staying fresh. Include codegen, launch/reconstruction
   and observable readiness, not just cargo check.
3. Extend a current measurement helper if suitable. Otherwise add a narrowly
   scoped iteration recorder with the M0 output schema. It must run an explicitly
   selected command, not choose a broad build/test lane implicitly.
4. Save raw evidence under a generated target report directory. Summarize the
   source revision, commands, sample count, result and limitations in the owning
   plan. Do not copy numeric counts into queue/status/tracks.
5. Identify profile/link/feature/cache changes as separate measured experiments.
   Do not combine all flags into an untraceable speed patch.

**Acceptance:** timing rows correlate edited input, actual admitted generation,
observed behavior and build/link events. A missing tool is recorded as unavailable.
**Poison:** skip loading the new artifact or feed an old host acknowledgment;
the helper rejects the run instead of reporting a fast success. Make its metadata
command fail; the closure report must fail rather than report zero dependencies.
**Cheapest check:** helper unit tests using fake command/result fixtures, then the
selected real loops. No workspace test suite. **Stop:** lack of a representative
machine leaves M0 open; it does not hold I1's pure ownership extraction.

## I1 - pure Rust authoring without the facade

**Class:** DO. **Input:** no new runtime required.
Read `crates/ambition_entity_catalog/src/lib.rs`,
`crates/ambition_entity_catalog/src/authoring.rs` (it was
`crates/ambition_characters/src/moveset_authoring.rs` when this packet was written; I1 moved it 2026-09-11), <!-- cite-ok: the path it moved from -->
`crates/ambition_characters/src/moveset_prefabs.rs`, and imports in
`game/ambition_demo_smash/src/moveset.rs`. Follow all helper callers with rg and
inspect the corresponding import bodies. A6 supplies the field responsibility map.

1. Inventory each helper's input/output types, constants and dependencies. Move
   only functions which construct pure MoveSpec values and the pure constants
   they need into the existing entity-catalog authoring module. Do not move live
   character/body preparation, repertoire policy or prefab selection wholesale.
2. Update all helper callers, including other demo providers. Keep one helper
   implementation. Delete the old internal helper road/reexports rather than
   preserving a bridge for a pre-release API.
3. Create an out-of-workspace builder fixture that authors a real multi-window
   move with a technique reference. Its dependencies are the pure value/pipeline
   crates only. The fixture may share an authored input file, not import the
   broad game provider as a library.
4. Add a feature-resolved closure witness through the existing absence-contract
   machinery. Examine normal, build and dev dependencies for the actual build
   and test invocations separately. A proc-macro/build-script edge can defeat a
   claim even when normal imports look clean.
5. Move/grow pure helper tests alongside the owner. Preserve exact emitted move
   values through golden or direct structural comparison. Changes to game feel
   are not bundled with this ownership move.

✅ **STEP 1 COMPLETED 2026-09-11 in two passes, and the first pass was too
narrow.** `moveset_authoring.rs` moved first; the vocabulary shipped tables are
actually built from did not. The second pass moved the twenty `smash_*` modules
(captures, repertoires, counters, tethers, portals, 4,767 lines) after measuring
that every one of them names `bevy` only inside comments, and split
`SmashHoldState` — a rollback-registered `Component`, the single derive pinning
the family — into `ambition_characters::smash_hold_state`. Receipt, numbers and
the poison in [queue.md](../queue.md).

**Proposed output locations:**
`crates/ambition_entity_catalog/src/authoring.rs`;
<!-- cite-ok: proposed file to be created by I1, not source evidence. -->
`fixtures/content_builder/Cargo.toml` and its builder source.
<!-- cite-ok: proposed independent fixture, not a baseline file. -->

**Acceptance:** the external fixture builds and tests without Bevy, runtime,
render, audio, monolith or named game providers in its resolved closure. Move
semantics remain identical. This is an authoring improvement, not yet the claim
that the host never relinks for data edits.
**Poison:** make the fixture import the facade or restore the helper's runtime
edge; the resolved-closure witness fails and names the path. Mutate one emitted
move field; the parity test fails rather than normalizing it away.
**Cheapest existing checks:** `cargo test -p ambition_entity_catalog --lib`,
then the affected character/helper tests. Run the new independent fixture's
unit tests after it exists. Run the relevant demo acceptance once for the moved
consumer, not after every helper edit. **Do not expand:** into a character-domain
rename, global serde redesign or compile-profile tuning.

## I2 - a loadable move artifact through existing preparation

**2026-10-01: the boss, roster, item, encounter and fighter-ladder sources are
read off disk too.** Before, only the move tables, the fighter facets and the
music registry were; `boss_profiles.ron`, `boss_seeds.ron`,
`boss_validator_bands.ron`, the nine `boss_encounters/*.ron`,
`character_catalog.ron`, `items.ron`, `goblin_encounter.ron` and
`fighter_brain_ladder.ron` were `include_str!`ed, so a boss tuning edit rebuilt
`ambition_content` and relinked the game. Now they go through
`pack::source_text` (embedded only under `static_content`: web, Android, a build
without the source tree). MEASURED on the agent machine, warm, `cargo build -p
ambition_app` after touching the file: `boss_profiles.ron` 0.44 s (nothing
rebuilt), against 5.79 s for `sfx_registry.ron`, which then stayed embedded by
its stated policy. The sandbox Yarn dialogue (`dialogue/sandbox/*.yarn`,
`yarn::yarn_sources`) followed the same day: a dialogue edit 0.43 s, nothing
rebuilt. Later the same day boss tuning and dialogue became live reloads (see
I3), and the last embedded data files went off disk too: `sfx_registry.ron`
(synth-cue tuning is a sound designer's loop, not a code edit),
`boss_sheets.ron`, `boss_art_keys.ron` and the generated
`vanity_card_made_this_meme.ron`. MEASURED the same way, `cargo build -p
ambition_app --bin ambition_game_bin`: `sfx_registry.ron` 0.52 s,
`boss_sheets.ron` 0.48 s, the vanity card 0.52 s, each with no crate compiled;
the vanity card cost 7.18 s and two crates while it was still embedded. Only
`pack.ron` itself stays embedded: a new source needs a new declaration in
`pack.rs` anyway.

**2026-10-01: the demo packs are read off disk too.** Sanic, Mary-O, Smash,
Pocket, TwinTrack and the app's versus pack were `EmbeddedPack::new` over
`include_str!`, so a demo tuning edit compiled the demo crate and the app.
They are `content_pack!` now (`ambition_platformer2d`): a `PackText` per file,
read when the pack is first compiled, embedded only under the CALLING crate's
`static_content` (the macro's `cfg` is read in that crate). MEASURED: touching
a smash, sanic or versus move table costs 0.43 to 0.45 s with no crate
compiled; the smash pack embedded cost 6.62 s. Proven both ways: with
`sanic.ron` renamed, `cargo check -p ambition_demo_sanic --features
static_content` fails on the missing file and the default check passes. Witness:
`content_sdk_tests::a_sourced_pack_reads_its_files_and_names_every_one_it_cannot`
(poison "refuse at the first unreadable file" fails it). Not yet: a running
demo does not reload its pack; that road is `ambition_content::reload`'s and
Ambition's only.

**2026-10-01: the movement defaults are read off disk too.**
`platformer_defaults.ron` (the starting abilities and `MovementTuning`) was an
unconditional `include_str!` in the actor monolith: MEASURED, a touch compiled
15 crates in 14.10 s. `Platformer2dGameplayDefaults::load` reads it off disk and
embeds it only under the monolith's new `static_content`, which
`ambition_platformer2d` and `ambition_app`'s `static_content` forward. After:
0.49 s, no crate compiled. With the file renamed, the `static_content` check
fails on it and the default check passes. Census of the engine crates for the
same shape (`include_str!` of a data file outside a test): the LDtk entity
contract (`ldtk_entity_contract.json`, a schema, not tuning) and
`test_boss_catalog` (behind `test-support`) remain; neither is tuning data.
The same day a running game plays a saved defaults edit:
`ambition_app::app::movement_defaults_watch` (off-disk builds only) writes the
file's tuning to `EditableMovementTuning`, the F3 inspector's mirror, so the
edit takes the existing developer-edit road (proposed, admitted by the timeline
owner, published in `PreUpdate`); a file that does not parse is refused, and a
change to the starting abilities is reported and needs a restart. Witness:
`edit_to_play_through_the_shell::a_movement_tuning_saved_while_the_game_runs_is_played`
(`jump_speed` 630 → 700 in a copy: `ActiveMovementTuning` takes it **19 frames
after the save**, and the player's jump launch goes from 555 to 625; an
unparseable save first changes nothing). Poison "the watch does not write the
mirror" fails it at 630. The witness walks 10 frames before each jump: an idle
player in `proving_grounds` is hit at frame 122 (60 → 59 HP, measured), and a
press in the hitstun does not launch.

**Class:** DO. **Requires:** I1 for the lightweight Rust frontend; the data
format/host side can be developed in parallel.
Read `crates/ambition_content_pack/src/lib.rs`,
`crates/ambition_content_pack/src/prepared.rs`,
`game/ambition_content/src/pack.rs`,
`crates/ambition_characters/src/prepared.rs`, and
`crates/ambition_combat/src/technique.rs`.

1. Define the envelope and one domain-owned move section under the
   generation/reload contract. Record section dependencies, deletion semantics,
   cache inputs and diagnostic-versus-mechanical provenance. Version envelope and
   section separately. Specify canonical numeric/key encoding, bounded lengths,
   logical references and required versus optional section rules. The first
   implementation may favor clarity over compression.
2. Separate portable section data from Any-valued lowered objects and installed
   callbacks. Keep the current pipeline's diagnostics and validators. Add a
   domain hydration adapter that produces the same prepared move/character
   values used by existing consumers.
3. Have the independent Rust builder emit this artifact. Add a source-data
   frontend for the same section where it fits the existing RON path. Compare
   frontend outputs at the admitted semantic value, not at source formatting.
4. Add a selected-host load path through the current source/resolver policy.
   Publish immutable objects plus a complete manifest; partial writes or watcher
   order cannot select a mixed pack. Inspect actual installed technique support. Unknown keys, unavailable
   capabilities, invalid parameters and unresolved references refuse admission.
5. Remove the migrated move table as a compiled authoritative input of the host.
   A test-only old table may be a temporary parity oracle, not a runtime fallback.
   Other content families can remain compiled until their own migration packet.
6. Reuse the existing character candidate path. The first activation can happen
   at startup or a supported local reconstruction boundary. I3 adds coordinated
   repeated reload; do not build arbitrary live-world replacement here.

**Proposed file:** `crates/ambition_content_pack/src/artifact.rs`.
<!-- cite-ok: proposed I2 output, not a baseline file. -->
The move section codec belongs next to its pure value owner. Host hydration
belongs next to character/preparation integration, not in the pure artifact crate.

**Acceptance:** a prebuilt host plays the edited artifact without invoking Cargo
or its linker. The external compiler's dependency guard remains green. Changing
move timing changes a controlled exchange; alternate frontends admit the same
prepared result. Cross-section bad input is rejected before activation.
**Poison:** restore include_str or a game-crate dependency for the migrated input;
M0/link tracing or closure guard must detect it. Change the artifact while keeping
the old in-process table; the observed-move witness must fail. Remove installed
support; artifact admission must refuse even with a valid offline schema manifest.
**Cheapest checks:** artifact codec/roundtrip tests in content_pack and owning
value crate, affected character preparation tests, one selected host integration
module. No renderer build for codec-only tests. **Stop:** unresolved transport or
codec choice can use a simple versioned baseline; it cannot justify serializing
Any/function pointers or adding a second validator.

## I3 - coordinated generation publication and local reload

**Class:** DO. **Requires:** I2.
Read character stage/activate functions, PreparedContentBuilder and
PreparedContentIdentity in runtime content_identity, session_world, rollback
session authority, and the existing construction/reconstruction coordinator.
Read [immutable construction](immutable-content-and-transactional-construction.md)
and [netcode](netcode.md) before claiming a transaction.

1. Introduce one candidate-bundle coordinator in the existing lifecycle/content
   ownership road. Replace process-global mutable-generation assumptions from
   the OnceLock content route for migrated families with App-scoped selection.
   Keep immutable reusable data shareable, but never share activation authority.
2. Factor candidate construction/validation from the current character activation
   function if needed; return a prepared candidate without publishing it. Do not
   call today's mutating activation early and try to undo it. Preserve initial
   cast-withholding policy separately from all-or-nothing revision replacement.
   Extend the runtime's existing digest sections for the artifact and profile
   requirements. Capture domain revisions in the bundle. Candidate hydration
   must not publish one domain while another still validates.
3. Implement the generation/reload state machine and the execution contract's
   binding obligations. Seal the base epoch/profile. Stale candidates refuse or are re-prepared; no
   activation from whichever current registry happens to be readable.
4. Expose explicit validate, describe-diff, reload-request and activation-status
   operations to developer tools. File watching calls the same request path.
   Coalesce notifications without losing the identity of the chosen candidate.
5. Support local scenario reconstruction, remote-session refusal and
   presentation-only reload under the stated classification. Pin the scenario
   input/checkpoint. Implement one bounded safe candidate path with A10 before
   closing reliable reload. Retain the active scene on supported candidate
   refusals; classify explicit recovery separately from unchanged retention.
   Do not promise undo after arbitrary native plugin failure.
6. Add generation binding to any new state or cache. Ensure pending construction
   plans, handles and observers either stay with their generation or are retired
   before stepping the new one. Preserve same-session unhealthy diagnostics.

⭐⭐ **THE REQUEST PATH EXISTS AND IT IS A MESSAGE — MEASURED 2026-09-11, and it
is what step 4's *"file watching calls the same request path"* is about.** The
road to `prepare_platformer_content`, end to end:

```text
ShellEvent::PreparationRequested(ProviderLoadTransaction)   router.rs:490
        ↓
prepare_requested_sessions                                  provider/lifecycle.rs:219
        ↓
PlatformerPreparation::prepare                              provider/lifecycle.rs:280
        ↓
prepare_platformer_content                                  provider/lifecycle.rs:~819
        ↓  epoch allocated as "the final non-fallible step"
sessions.publish(transaction, …)                            provider/lifecycle.rs:~556
```

⇒ **A RELOAD DOES NOT NEED A NEW PUBLICATION ROAD; IT NEEDS TO ISSUE THAT
REQUEST.** The transaction is three fields (`route_id`, `experience_id`,
`barrier`), and the router emits the event as part of a ROUTE change: it opens a
load barrier, marks a pending route and expects an activation to follow. So
re-preparing a LIVE world is a route-level operation, which is exactly I3 step
2's *"a supported local reconstruction boundary"* — not a reload-specific
lifecycle, and the old generation stays authoritative until the activation
publishes the new one.

✅ **A ROUTE CAN BE REQUESTED TO ITSELF WHILE ACTIVE — MEASURED, `f470b18c3`.**
`start_route` has no same-route guard: the request mints a fresh `LoadId` and the
preparation lifecycle runs again, with the old generation authoritative until the
new one activates. ⇒ **The bounded A10 proof is "re-request the current route",
not a new route kind.** `ambition_content::reload::request_reload` issues it
(`e627a4399`), as `ReplaceWith` rather than `GoTo` so a reload does not push
history.

⛔⛔ **BUT RE-PREPARING IS NOT SUFFICIENT, AND THIS IS THE TRAP IN THE OBVIOUS
READING.** MEASURED 2026-09-11: `register_declared_cast` and
`character_catalog::register` run in `AmbitionContentPlugin::build` — ONCE, at App
construction. A session re-preparation reads registries that were built then, so
it moves the `ContentEpoch`, the content fingerprint and the rollback contract
**and does not change a single move table the live cast plays.**

⇒ **THE COMPLETE TRANSACTION IS BOTH ROADS, PUBLISHED AT ONE BOUNDARY:**

| what | which road | what it moves |
| --- | --- | --- |
| the engine's generation | `PreparationRequested` → `prepare_platformer_content` | `ContentEpoch`, content fingerprint, rollback contract |
| the live cast's moves | `stage_move_section` → `activate_staged_revision` | `CharacterCatalogGeneration`, what bodies play |

They are different mechanisms **by necessity** — you cannot re-run `Plugin::build`
— so "route the reload through the existing preparation" is necessary and NOT
sufficient. Anything that does only the first publishes a new generation of the
same moves; anything that does only the second (today's
`publish_candidate`) changes the moves under an unchanged engine generation.

**⇒ WHAT CLOSES I3:** make the activation boundary apply the staged cast revision,
so the two move together or not at all. That is the remaining P0, and it is where
A10's Prepare/Admit/Draft/Verify/Publish/Retire stages earn their keep.

**Acceptance:** valid generation N+1 becomes visible at one boundary; no system
observes N's definitions with N+1's code/schema. Invalid N+1 leaves N's digest,
character generation, playback references and active timeline unchanged. Two Apps
can select different packs without contamination. Same-session rebase cannot
erase a previously unhealthy rollback diagnosis.
**Poison:** publish the character registry before validating another family;
the cross-family atomic-publication test fails. Increment epoch on refusal; the
retention test fails. Reuse a sealed plan after an installer/profile change;
admission fails. Inject a materialization-draft failure after metadata admission;
the supported path retains the old scene. A legacy stopped-world result is not a
passing retained-scene witness. FI2-FI4 specify the independent assertions.
**Cheapest checks:** extend existing character revision tests, content_identity
unit tests and one lifecycle integration module in the shared app_it binary.
**Subcuts and done boundaries:**

| Cut | Ordered output | Acceptance |
| --- | --- | --- |
| I3a | Factor nonmutating candidate hydration; implement complete-bundle seals, no-op identity and stale-attempt rejection | FI2/FI3; no full scene-retention claim |
| I3b | Inventory the selected constructors and hooks; split typed candidate data from active mutation; validate relationships/resource deltas; connect one bounded publication path with A10 | FI4's candidate refusal and valid reconstruction, not only parser failure |
| I3c | Add scenario pin/replay, changed-section explanation, actual activation status and generation-aware cancellation to existing tools | FI1-FI4 plus M0 measurements on the real edit loop |

**Step 4's file watch, 2026-10-01: the running game plays a saved content
file.** Until then `request_reload` had no production caller: the road was
built and tested, and a developer had to restart the game to see an edit. A
build that reads its content off disk (no `static_content`) now installs
`ambition_content::content_watch` (from `reload::register`, under the same
shell condition). Every 20 frames it compares the modification time of each
source `pack.ron` declares; on a change it compiles the pack from
`pack::source_root()` and calls `request_reload`. The reload road answers:
a pack that does not compile, an unsupported domain and a foreign timeline are
refused and logged, and the running content stays. A menu (no active
preparing route) or a reload in flight keeps the change for the next look.
Witness: `app_it::edit_to_play_through_the_shell::a_content_file_saved_while_the_game_runs_is_played`
(a `jab` retimed in an exported copy: the three bodies play the new duration
**19 frames after the save**, measured, with one request; poison "the watch
returns early" fails it at 600 frames).

**A saved edit under the shipped ownership mode (2026-10-01).**
`edit_to_play_through_the_shell::a_content_file_saved_under_a_local_timeline_rebases_it`:
with a timeline the local maintainer owns, a saved move edit is played, the
timeline the maintainer starts again binds the reloaded content, and it is
healthy. ⚠ The poison "the commit does not rebase"
(`reload::rebase_local_timeline_onto_the_new_generation` not called) left it
GREEN, measured: a reload re-requests the route, the route's re-activation ends
the session (`session-end` / `session-start` in the world-event log), and
`retire_rollback_authority_with_its_scope` stands the timeline down with its
scope before the new one is installed. On the shipped road the commit's rebase
restates that; it is kept as a deliberate restatement (the content crate's
hand-built hosts reach it without a shell), not as the protection of this road.

**Dialogue reloads too (2026-10-01).** The Yarn files are not in the pack:
they are the running `YarnProject`'s assets. `content_watch::YarnSourceWatch`
(built with `ui`, not `static_content`) looks at each file the project was
built from; a saved one is compiled with every other file of the project
(`replace_yarn_sources`, the standalone Yarn compiler) and only then becomes the
asset's text, which bevy_yarnspinner recompiles, restarting a running dialogue
at its current node. Witness: `content_it::a_saved_dialogue_edit_is_played`
(a saved line plays; an unclosed block and a type error are each refused and
leave the file's text as well as the program; poisons "the watch never looks"
and "no validation" each fail it). ⚠ The text is asserted, not only the line:
bevy_yarnspinner's own recompile keeps the old program on a broken file, so the
line alone could not tell a refused edit from a half-applied one.

**Boss tuning reloads too (2026-10-01).** `boss_profiles` and
`boss_encounter` are participating domains (`reload::BOSS_DOMAINS`). Like the
moveset, they are admitted at request time against world state: the App's
`BossCatalogRegistry` with Ambition's fragment rebuilt from the candidate pack
(`bosses::boss_catalog_fragment_from`, `BossCatalogRegistry::with_replaced`),
so a roster that does not assemble is refused before anything is staged
(`MoveReload::BossCatalogRefused`). The transaction carries the candidate
catalog; `PendingGenerationInputs::bosses` hands it to its own preparation
(`candidate_bosses_for`), which fingerprints AND freezes that one value; the
commit publishes the registry and catalog to the App. Witness:
`edit_to_play_through_the_shell::a_boss_tuning_saved_while_the_game_runs_is_played`
(mockingbird's `strike_speed_scale` saved in an exported copy: the rebuilt
boss plays it **22 frames after the save**, measured; poison "the claim carries
no catalog" fails it, on the live boss and on `SessionMechanics.bosses`).

⛔ **FOUND BY THAT POISON: THE LIVE BOSS IS NOT BUILT FROM THE FROZEN
GENERATION.** With the claim emptied, the session froze N's catalog and
construction spawned the mockingbird from it (probed: `strike_speed_scale` 1.0
at freeze and at build), and the live boss still played N+1 on the activation
frame. `ambition_boss_encounter::systems::update_boss_encounters` seeds a new
boss's behaviour, HP and phase triggers on its first tick from
`BossEncounterRegistry` (populated from the App's `BossCatalog`, reset at
teardown) or from `Res<BossCatalog>` directly. The two agreed at that moment
only because the commit publishes the App catalog before the new session
ticks: an ordering that happened to hold, not a rule. **Fixed for the
behaviour:** the seed no longer writes it (`apply_behavior_profile` is gone);
construction resolved it from the frozen catalog and captured the brain's
pattern from the same value, so a second write could only repeat it or split
the boss from its own brain. With that, the poison fails on the live boss.
**Closed the same day for the rest:** `BossConfig::seed` (`BossSeed`: the
encounter spec and the reward) is resolved at construction, with the
behaviour, from the catalog the construction carries.
`update_boss_encounters` seeds HP and phase triggers, and reads the death outro
and the music, from it; the reward chest takes the boss's own reward
(`BossRewardAnchor`) instead of a registry lookup by archetype. Only a
hand-built config (a fixture) has no seed and is resolved from the App as
before. Measured with the claim poison: the App held the saved HP 41 and the
rebuilt mockingbird kept the frozen 28 (`a_boss_tuning_saved_while_the_game_runs_is_played`
now asserts the saved HP too). `BossEncounterRegistry` is left with that
fixture fallback as its only reader. Still refused: items, audio, the character catalog,
the boss seed library and validator bands, and every source outside the pack
(`boss_sheets.ron`, `boss_art_keys.ron`).

**The character catalog in the reload — LANDED 2026-10-01 (two increments).**
`character_catalog` is a participating domain. Increment 1
(`ambition_characters::prepared::admit_staged_revision_with_catalog`): a
revision against a candidate catalog folds the WHOLE cast (authored source with
the staged edits, plus a bare definition per catalog-only row, as the barrier
does), carries the catalog, and publication inserts every resource the
assembly publishes. Increment 2 (`ambition_content::reload`): the candidate
fragment is assembled with `CharacterCatalogRegistry::with_replaced`; a change
to WHICH characters are built is refused (restart); every buildable character
is staged again from the candidate catalog and pack
(`character_catalog::buildable_definitions`, the boot registration's one road,
moves included) through `stage_character_revision_in` with the engine's art
vocabulary; the admitted candidate cast rides `PendingGenerationInputs` as
before. Witness: `edit_to_play_through_the_shell::a_character_row_saved_while_the_game_runs_is_played`
(the goblin row's `max_health` 5 → 9 in an exported copy: the three rebuilt
goblins have 9, **22 frames after the save**; the App's catalog and the
session's frozen cast both say 9). Poisons: "admission ignores the candidate
catalog" fails it on the live goblins (5, 5, 5); "the claim carries no
candidate cast" failed it ONLY at the frozen-cast assertion: the session froze
5 while the App's registry held 9, and the rebuilt goblins had 9. The road
was `apply_worn_character_gameplay`: it re-derives a body whenever the cast
generation changes, and it read the App's `PreparedCharacterRegistry`. It now
reads the activated generation's cast (`session::mechanics::worn_cast_for`, the
same contract as `perception_extent_for`: generation first, refusal in a shell
session that lost it, the App only where no session gate exists). MEASURED
after the fix, 2026-10-01: the same poison stops the live loop at [5, 5, 5].
⛔ **OPEN: THE OTHER READERS.** A grep for `Res<…PreparedCharacterRegistry>`
outside tests finds about 30 systems that read the App cast in a live session
(combat moveset, sim_view pose and index, actor_spawn, damage, empowerment,
summon, aggression, match activation, presentation, the three demo crates).
REASONED, not measured per reader: they agree with the frozen cast in every
normal run, because only a reload commit changes the App cast and the commit
activates a new generation; they differ in the frames between the commit and
the activation, and under a broken claim. Most of them sit in crates below the
monolith and cannot name `SessionMechanics`, so the fix is a session cast in a
lower crate (`ambition_characters`), not one more `worn_cast_for` per reader.
The boss seed was the same class, closed today. ⚠ Observed once, not reproduced in two reruns:
`quality_change_keeps_each_character::a_quality_round_trip_converges_back_with_every_page_loaded_and_nothing_orphaned`
failed in a filtered batch ("a direct gameplay boot has a PrimaryPlayer wearing
a character") and passed alone and in the same batch twice.

What it took, measured before it was built:

1. The catalog is not only FOLDED, it is an INPUT to the definitions:
   `ambition_content::character_catalog::register_characters` builds each
   definition from its row (display name, sheet target, the scale asked of the
   baked sheet, the hurtbox inset). A candidate catalog therefore needs a
   candidate set of definitions, built by the same function from the candidate
   rows, not a re-fold of the retained `StagedCharacterOverrides` alone.
2. The fold reads `CastAuthorities` (the catalog, `BrainProfileRegistry`,
   `ProviderDeclarations`) from the App (`CastAuthorities::from_world`). An
   admission over a candidate catalog needs those passed in, so
   `admit_staged_revision` can fold the WHOLE cast against the candidate and
   admit it as one `AdmittedRevision` that also carries the catalog to publish.
3. 50 production sites read the App's `CharacterCatalog` directly (counted by a
   grep for `Res<`/`resource::<`/`get_resource::<` of it). The commit must
   publish it at the same boundary as the cast, and each reader is either fine
   reading the published value after activation or is a frozen-generation
   question like `SessionMechanics` — to be sorted before the first edit, not
   after.
4. Witness shape: `edit_to_play_through_the_shell`'s, a saved row value (a
   standing height or max health) on a live body after the reload, plus the
   frozen record, with the poison "the claim carries no candidate cast".
5. What exists to build on (read 2026-10-01): `prepared::stage_character_revision`
   stages a WHOLE definition as a revision, but takes `&mut App`; the reload
   holds a `World`, so it needs a `World` form (the body is
   `prepare_for_registration` plus an insert). The bindings need the engine's
   art vocabulary (`with_engine_vocabularies`, in the actor monolith), so the
   content side stages through a monolith wrapper. `admit_staged_revision`
   folds only the STAGED characters over the live registry; a catalog change
   re-folds EVERY character (the row feeds the fold), so the candidate catalog
   form must re-fold the whole cast. The catalog itself is assembled from
   provider fragments — the boss catalog's shape, so `with_replaced` on its
   registry is the model (`BossCatalogRegistry::with_replaced`).

I3a is independently useful. I3 is complete only after all three cuts. I1/I2 and
I4 contract work need not wait for I3b; procedural replacement does. Do not turn
I3b into arbitrary ECS undo or all-world concurrent simulation.

## I4 - small procedural SDK and one native semantic reference

**State (2026-10-01): the first cut is LANDED.** One real technique runs on the
host in the shipped game; I4 is not complete (see *Open* below).

| Part | Where | Witness |
| --- | --- | --- |
| SDK: descriptors, schemas, records, canonical digest, typed ports, `Invocation` | `crates/ambition_extension_sdk` (no dependencies) | its unit tests; policy `engine.ambition_extension_sdk-portable` |
| Host: offers installed WITH their adapter, admission, serial order, staged writes, fault discard, body store | `crates/ambition_extension_host` | `ambition_extension_host` tests (refusals, order cycle, fault discard, missing observation) |
| Trigger port `ambition.boss.special_cast` (v4: the boss's own live-room size) | values `crates/ambition_boss_special_port`; adapter `ambition_boss_encounter::extension` | card in the port crate's docs |
| Request port `ambition.projectiles.spawn` | value `ambition_projectile_spec::ProjectileSpawnPort`; adapter `ambition_projectiles::extension` | card on `ProjectileSpawnPort` |
| Request port `ambition.combat.damage_box` (the box's faction is its owner's EFFECTIVE faction, never the module's choice) | values `crates/ambition_combat_port`; adapter `ambition_combat::extension` | card in the port crate's docs |
| Request port `ambition.boss.summon` (only a boss summons; the minion joins its encounter on the enemy side; the id is `<label>:<boss id>:<serial>`, the module gives label and serial) | values `ambition_boss_special_port::BossSummonPort`; adapter `ambition_boss_encounter::extension::install_summons` | card on `BossSummonPort`; its id and wire tests |
| Request port `ambition.combat.held_damage_box` (a box that lives while the module re-submits its slot and generation each tick; the combat domain owns the entity, its record `combat.held_damage_boxes` is rollback state with mapped entities) — the I5 "module-owned entity" pattern without an entity in the module | values `ambition_combat_port::HeldDamageBoxPort`; adapter `ambition_combat::extension::lower_held_damage_boxes` | card on `HeldDamageBoxPort`; the saddle point's parity and sync-test arms |
| Phase `wielded_use` → `ItemPickupSet::WieldedAbilities` (after the native wielded chain); trigger `ambition.items.wielded_use` (selector = held item id; press, driven, body, gravity basis, aim, mana) and requests `ambition.resources.spend_mana`, `ambition.feedback.body_sound`, plus the damage box and projectile ports offered again in this phase. `extension_composition::install_ports` is the one list of offered ports; test harnesses call it | values `ambition_combat_port::wielded`; adapters `ambition_abilities::extension` | port cards; `the_ports_mana_rule_is_the_banks` (the port's `can_pay_mana` and the bank's `pay` are one rule) |
| Four wielded abilities as modules: the shockwave, the beam, the volley and the meteor (the wielded kit's damage/projectile verbs). Their native systems left `ambition_abilities`; they are test-only references in `ambition_content` | `game/ambition_content_modules/src/{shockwave,beam,volley,meteor,wielded}.rs` | `wielded_ability_parity_tests` (linked AND WASM, five bodies: driven and brain-driven, gravity down/sideways/up, aim stick/movement stick/none, too little mana, no pool; damage boxes, projectiles, mana paid, sounds; poisons "ignore the gravity frame", "a beam for an undriven body" and "never pay" each fail it). Deliberate change: a module's box is on the wielder's EFFECTIVE faction (the native hard-coded the player side) and has no inspector name |
| Module-owned entities (I5's pattern WITH an entity): request `ambition.world.spawn_module_entity` (`kind`, position, lifetime; the world mints the identity from the spawner's, freezes its effective side, team, session and presentation source; a spawner that cannot name it is refused, and `Wielder::names_spawns` (wielded_use v2) lets a module ask first) and phase `module_entity_tick` → `ItemPickupSet::WieldedAbilities`, after `wielded_use` (`extension_composition::order_phases`), with trigger `ambition.world.module_entity_tick` (selector = kind; position, lifetime left, nearest `Enemy`-side hittable body) and requests `ambition.projectiles.spawn` and `ambition.feedback.body_sound`. The world ages every module entity, bound or not, and removes it at the end of its lifetime; the module's per-entity state is a record on the entity (`extension.body_records`), so it goes with it. Rollback: `ModuleEntity` (`entity:module_entity`, `ability.module_entity`), schema 286. OW: a module entity is in its spawner's live room (as a shot is in its owner's), its `nearest_enemy` and a pull reach only bodies in that room — the native turret and well read no room | values `ambition_combat_port::module_entity`; adapters `ambition_abilities::module_entity` | port cards; their wire test |
| The sentry is a module: `deploy` on `wielded_use`, `turret` on `module_entity_tick` (a `Cadence` record on the turret). The native systems left `ambition_abilities` (WIELDED_MEMBERS 9 → 7) for the test-only references | `game/ambition_content_modules/src/sentry.rs` | `sentry_parity_tests` (linked AND WASM, tick for tick: turret identity, side, team, presentation source, position, lifetime; bolts by owner; sounds; mana; five deployers — driven, brain-driven, possessed, too little mana, no identity — and an equidistant pair, a dead, a far and an out-of-range enemy; poisons "arm delay 0.15" and "no identity tie-break" each fail it); `the_spawn_order_of_the_deployers_decides_nothing`; `a_turret_is_in_its_deployers_live_room_and_shoots_only_there` and `a_well_pulls_only_the_bodies_in_its_own_live_room` (poison: drop either room filter, it fails); `app_it::a_boss_special_runs_on_the_extension_host::a_wielded_sentry_deploys_a_module_entity_on_the_extension_host` (with a GGRS sync-test arm); `app_it::rollback_populated_timeline` (the module road's turret on the populated SyncTest timeline). Deliberate change: the native deploy minted the turret's identity before it asked for mana, so an unpaid press used a number of the body's mint stream; the module does not |
| The vortex is a module: `cast` on `wielded_use`, `well` on `module_entity_tick`. The well pulls, THEN ages, so the module keeps its clock (a `Well` record) and ends the well itself through `ambition.world.end_module_entity`; the world's lifetime is the backstop. Its pull is `ambition.world.pull_bodies` (centre, radius, rate; reaches `Enemy`-side hittable bodies; lowered in `BodyPathSet::Carry`, as the native well ran). Schema 288 (`ability.vortex_well`, `entity:vortex_well` gone). WIELDED_MEMBERS 7 → 5 | `game/ambition_content_modules/src/vortex.rs`; adapters `ambition_abilities::module_entity::{lower_body_pulls, lower_module_entity_ends}` | `vortex_parity_tests` (linked AND WASM, dt 1/60 so the well's last tick is compared: wells, every body's position, sounds, mana; six casters — gravity down and sideways with an aim stick, brain-driven, possessed, too little mana, no identity — and bodies in and out of reach, a corpse and a driven enemy body; poison "lifetime 0.92" fails at tick 54); `app_it::a_boss_special_runs_on_the_extension_host::a_wielded_vortex_opens_and_ends_its_well_on_the_extension_host` (with a GGRS sync-test arm); `app_it::rollback_populated_timeline` (a well on the populated SyncTest timeline) |
| A CONDUCTED boss: phase `boss_conduct` → `WorldPrepSet::AfterIntegrate`; trigger `ambition.boss.conduct` (selector = the boss's BEHAVIOUR id, the kind of boss, not a placement id; position, velocity, facing, the side it last chose, alive, the pattern's live `Special` tell and strike with their time left, target, the hall measured in its own live room, driven, enraged); requests `ambition.boss.conducted_pose` (hold the pose with a velocity, or release it to the driver; the side it faces, kept in `ConductedFacing`, `boss.conducted_facing`, applied by `face_conducted_bosses` in `BossSteerSlot`), `ambition.presentation.drawn_row` (`PinnedRow`), `ambition.feedback.burst`, `ambition.combat.riding_hitbox` (follows its owner; box or circle; feel-scaled or fixed launch with a direction; on the owner's effective side), `ambition.combat.held_damage_box` v2 (a held box follows its submitted centre), `ambition.boss.summon` v2 (`on_boss_side`), projectiles and sounds. `Hall`/`measure_hall` moved to `ambition_boss_encounter::hall` | values `ambition_boss_special_port::conduct`, `ambition_combat_port::riding`; adapters `ambition_boss_encounter::conduct`, `ambition_combat::extension` | port cards and wire tests |
| The Flying Spaghetti Monster's conductor (791 native lines) is a module: `ambition_content_modules::fsm`, one `Conductor` record (hall, live part, clocks, dive, stranded/rising, sting rearm, two rolling shocks with a held-box generation). The native conductor is a test-only reference; `ambition_content::bosses::fsm::conductor_of` reads the record for tests and inspectors. Schema 289 (`content.fsm_conductor` and its map gone, `boss.conducted_facing` new) | `game/ambition_content_modules/src/fsm.rs` | `bosses::fsm::fsm_parity_tests` (a scripted fight with every move, the dive's landing and stranding, a repeated move and death; three arms — alone, driven, enraged — on the linked road, and the WASM road; pose, held pose, facing, drawn row, every hitbox, throws, summons, bursts, sounds, tick for tick; poisons "fall 0.33 s" and "shock dust every 4 ticks" fail it). Building the fixture's boss with a placement id of its own found a real bug: the trigger had keyed on `BossConfig::id`, so in the shipped arena the module would never have run. `app_it::fsm_fight` (13: the dive lands on you, the shocks roll and end with the god, noodlings pass its volumes, possession) and `two_players_two_live_rooms::the_fsm_measures_its_hall_in_its_own_live_room` on the module road. Deliberate changes: a summon's id is the port's `<label>:<boss id>:<serial>`; a driven god's volumes are on its effective side; the shocks carry no inspector name |
| Request ports of one phase are lowered in INSTALL order (`ExtensionSet::LowerPort`), so two adapters that write one domain message have an order someone chose | `ambition_extension_host` | `request_ports_are_lowered_in_the_order_they_were_installed` (ambiguity detection at `Error`; without the rule the build fails) |
| Phase `technique_execution` → `CombatSet::ContentSpecials` | `ambition_platformer2d_runtime::extension_composition` | — |
| Every boss technique is a module (eleven: apple rain, the echo fan, the eye beam, the gradient cascade, the gradient nova, the minima trap, mode collapse, the overfit volley, the overflow flood, the saddle point and the seismic stomp; `strike::{once, once_numbered, locked, locked_when}` hold the shared strike rules) | `game/ambition_content_modules`; the native systems are test-only references; `ambition_content` registers no technique rollback state | `specials::module_parity_tests` (tick-for-tick on the linked AND the WASM road: requests, effects AND live hitbox entities, owner and move-use credit, telegraph locks, gameplay `dt`, the boss's OWN live room and a boss whose room cannot be told; poisons "no strike reset", "drop the occurrence", "no telegraph lock", "drop one loaded request", "apple rain forgets its lane sequence", "the flood floods without a room", "the volley skips its first sample", "the strike number does not advance", "the crawler on the wrong side", "the saddle arm never turns" and "a held box is never released" each fail it); `app_it::a_boss_special_runs_on_the_extension_host` (real brain press; GGRS sync-test arms for the fan's latch AND the saddle point's held arm entity — unregistering `combat.held_damage_boxes` fails it with a checksum mismatch); `app_it::two_players_two_live_rooms` (apple rain and the flood in each boss's own room) |

**Deliberate change:** the native fan aimed at its target's body only when the
target was the player, and otherwise at the stored point. The trigger adapter
aims at whatever body the boss tracks.

**D6, first cut (2026-10-01):** the declared modules are a section of the
prepared content identity (`extension.modules`, from
`ambition_extension_host::ExtensionGeneration`: each module's key, code
identity — a loaded module's is the digest of its exact bytes — and
descriptor digest, in declaration order), on both preparation roads. Witness:
`app_it::a_boss_special_runs_on_the_extension_host::the_prepared_content_identity_names_the_module_code_the_session_runs`
(the linked and the loaded build are different generations).

**D6, the local reload (2026-10-01):** a published module reload that changes
the declared modules re-mints the session's content
(`extension_composition::remint_session_content`, at
`MechanicalEditSet::Publish`): `PreparedContent::with_section` replaces
`EXTENSION_MODULES_SECTION` and computes the fingerprint again, a new epoch is
allocated, and `PreparedContentIdentity` and `ActiveContentBinding` move with
it through `ambition_platformer2d_runtime::publish_session_content`, which is
now the one road by which a running session changes generation in place (the
LDtk world reload publishes through it too, and only reads the active content). A reload of the
same bytes keeps the generation. The local rollback baseline, stopped at
`Admit`, starts again in `Update` against the new identity — MEASURED under
the shipped ownership mode (a timeline the local maintainer owns):
`a_module_reload_rebases_the_local_timeline_onto_the_new_identity`; poison "re-mint one
frame late" leaves the restarted timeline bound to the old identity and the
next frame reports it `Unhealthy` ("prepared content changed while the GGRS
session was active"). The live rooms are not
built again; a room built after the reload is stamped with the new content.
A session whose timeline another owner holds refuses the reload, so a remote
session keeps its generation. Witness:
`a_loaded_module_keeps_session_state::a_module_file_replaced_while_the_game_runs_takes_over`
(section, fingerprint, epoch, identity and binding; then a door, and the room
behind it names the new content; poisons "no re-mint" and "binding not moved"
each fail it) and `a_module_file_that_changes_while_the_game_runs_is_reloaded`
(same bytes, same fingerprint).

**M1 on the loaded road, measured 2026-10-01** (release, agent machine,
`extension_inspect --time` and `ambition_extension_wasm`'s ignored
`m1_cost_of_one_call`): one WASM call was ~30 us to instantiate plus ~32 us of
guest time spent rebuilding all fifteen module descriptors, plus the entry.
With one boss in the sandbox the loaded road cost **+445 us per tick**: three
modules kept a counter across strikes and so took `IdlePolicy::Invoke`, which
called them on every tick of every boss. Two fixes, both measured:
`IdlePolicy::ResetStateExcept(keep)` (an idle tick resets the records except
the kept ones, without a call; admission refuses a keep the entry does not
write) took the three idle calls away, and `export_modules!(list: ...)` builds
only the called module (guest time **31.8 -> 7.8 us** per call). The loaded
road's per-tick overhead with one boss is now within the run-to-run noise
(30-100 us). A fresh instance per call (~28 us) is kept: it is what makes a
guest static unable to carry state across a rewind.

**The SDK cannot name engine state (2026-10-01), held structurally instead of
by a compile-fail test.** `ambition_extension_sdk` has no dependencies at all,
so no SDK signature can carry a Bevy, host or engine type, and "direct
health/body mutation through the SDK" cannot be written: a compile-fail test
would only restate a missing method name. The workspace policy
`engine.ambition_extension_sdk-portable` is now kind `dependency-none` (new): a
dependency of ANY name fails it. It was a denylist of nine names, which passed
every engine crate it did not list. Poisons: `bevy_reflect` added to the SDK
(not on the old list) fails it; the rule's own fixture
(`poison_dependency_none_reacts`).

**I7 item 5, first cut (2026-10-01):** `ambition_extension_host::inspect`
(composition and per-body records as text) and the tool
`ambition_app_tools --bin extension_inspect` (ports, serial order, linked or
loaded code, replacements, generation, records by field, and a dry-run
`--try-replace` that reports an admission refusal). The host stores no record
that a call left at its initial value (an absent record is initial), so an
`IdlePolicy::Invoke` module adds no state to bodies it has nothing to remember
about (`a_call_that_leaves_its_record_initial_stores_nothing`).

**Open:** the deterministic fault
policy (today a fault discards the invocation's output and is counted in
`ExtensionFaults`; it does not stop the session — and, reasoned from the code,
not measured: `ExtensionFaults::record` has no replay gate, so under a rollback
session a fault is counted again on each resimulated tick); a second technique
with an observation port (no production observation port exists yet: every
module reads its trigger. Mark/recall's mark, if it becomes a module, would be
the first: its native system writes an instant `HitEvent` and a typed fx
effect, and its beacon visual reads `PlayerMark`, so it needs about five new
ports). (Session-attached records landed with I5's first cut; see I5.)

**The loaded road (an I6/I7 first cut), 2026-10-01: a module edit no longer
compiles the engine.** One backend, not the two-backend comparison I6 asks
for; the choice is recorded below so M1 can overturn it with numbers.

| Part | Where | Witness |
| --- | --- | --- |
| ABI `ambition-ext-1`: three exports, no imports, bytes in and out | `ambition_extension_sdk::{abi, wire}`; `export_modules!` | `ambition_extension_host` tests run a loaded module through the full wire in-process and compare it with its native build; an undeclared request from a loaded module faults |
| Port codecs | `Port::{encode, decode}` on each port | the two port crates |
| Host: loaded runner, output checked like native output, explicit replacement | `ambition_extension_host` (`DeclaredModule`, `ModuleBackend`, `Admitted::replaced`) | `a_loaded_module_replaces_a_native_one_only_when_it_says_so` |
| WebAssembly backend: wasmi, `deterministic`, fuel, a new instance per call | `crates/ambition_extension_wasm` | refuses an importing module; a runaway entry runs out of fuel |
| Hot reload in the SHIPPED composition: a changed file is polled, proposed through the mechanical-edit protocol, and published with no timeline (refused, staged and kept, under a sync-test session the harness owns) — `a_boss_special_runs_on_the_extension_host::a_module_file_that_changes_while_the_game_runs_is_reloaded`. And a reload that ADDS a module the running game did not have, with a schema the host never saw: it takes over and counts the next presses — `a_loaded_module_keeps_session_state::a_module_file_replaced_while_the_game_runs_takes_over` (2026-10-01; its commit message calls it the reload road's first app-level witness, which is wrong: the first is the one before it) | `ambition_platformer2d_runtime::extension_composition` (`load_developer_modules`, `propose_module_reload`, `publish_module_reload`) | the two tests named |
| Reload, a review's three findings (2026-10-01): **a file is the unit of replacement** — a loaded module carries its origin (`ModuleCode::Loaded::artifact`), and a reload of that file removes every module that came from it as one set, so a module the new build no longer exports leaves; **one poll, one candidate** — `stage_loaded_replacements` takes every changed file, builds on a candidate already staged, admits once; **a departed schema takes its records** — publication removes them from `BodyRecords` and `SessionRecords`, and `read_state` faults a stored record of another shape (`Fault::StaleRecord`) instead of reading it as the admitted one | `ambition_extension_host::reload`, `exec::read_state`, `store::RecordSet::{get_stored, retain_schemas}`; the runtime's `propose_module_reload` | `a_module_a_rebuilt_file_no_longer_exports_leaves_with_it`, `two_files_changed_before_one_publication_both_take_over` (one poll and two), `a_departed_schemas_records_go_and_do_not_come_back`, `a_stored_record_of_another_shape_faults_the_invocation`; poisons "replace by the new keys", "build on the published composition", "keep departed records" and "ignore the stored shape" each fail exactly its own test |
| The developer road | `AMBITION_EXTENSION_MODULES` or `ExtensionModuleFiles`, runtime feature `wasm_modules` (the app enables it); `scripts/build_extension_modules.sh` | `app_it::a_boss_special_runs_on_the_extension_host::a_module_rebuilt_as_wasm_replaces_the_linked_one_in_the_same_game`; content `wasm_parity` (floats to 1e-3: the guest's `sin`/`atan2` differ in the last bit, measured) |

**Measured (M0, this machine, 2026-10-01):** an edit to the echo fan to a
loadable `.wasm` is **1.36 s** wall (`scripts/build_extension_modules.sh`,
first edit after a cold module build); a one-constant edit warm is
**0.34 s**. The same constant edit to a technique still in
`game/ambition_content` is **7.05 s** to relink `ambition_app`, plus a
restart. Nothing in the engine compiles or links on the module road. Recipe:
`docs/recipes/writing-a-procedural-module.md`.

**Hot reload, the same day.** The runtime watches each loaded file (a stat
every 20 frames). A changed file is loaded, the WHOLE composition is
re-admitted with it (`ambition_extension_host::reload`), and the candidate
goes through the engine's mechanical-edit protocol: PROPOSED, the rollback
timeline's owner ADMITS (a local timeline is stopped and rebased; a timeline
the host did not start refuses), then PUBLISHED. Last-good by construction: a
file that does not load or admit is reported and the running code stays. A
reload that changed a state schema's SHAPE under live records was refused in
this first cut; since the schema-evolution row below, its records migrate by
field tag, and only a changed attachment or save policy is refused. Witnesses: host `a_reloaded_module_takes_over_at_publication_and_keeps_its_records`
and `a_reload_that_reshapes_live_state_or_is_refused_leaves_the_running_code`;
app `a_module_file_that_changes_while_the_game_runs_is_reloaded` (published
without a timeline; refused, still staged and healthy under the harness's own
sync-test session).

⚠ Not covered by a test: the SHIPPED app's own local session (the
`LocallyRebasable` arm) taking a reload. The harness cannot construct that
ownership today.

**Why wasmi first:** deterministic by construction (NaN canonicalization,
fuel instead of a clock), pure Rust, builds for every shipped target.

**M1, first reading (2026-10-01, this machine, while a test lane ran):** one
WASM call is **75–115 µs**, of which **~31 µs** is the new instance; a call
that does nothing burns **86k fuel** in the ABI glue. Every boss invoked every
bound key every tick only to reset strike latches, so five keys cost ~0.5 ms
a tick per boss, and more under rollback resimulation. ⇒ **`IdlePolicy`:** a
trigger marks the ticks with nothing to act on (the boss port: neither
pressed nor telegraphed) and an entry may declare `ResetState`: the host puts
its records back to their initial values WITHOUT the call. Every migrated
technique declares it; the parity suite still matches the native systems tick
for tick, so the shortcut is the same result. A WASM module now costs nothing
on idle ticks. Open: the per-call glue cost (instance reuse with a restored
image; a lighter input encoding).

**Class:** DO. **Requires:** the execution contract; does not wait for a VM.
Read actual boss special producers, domain request types, combat_schedule,
SimId/session ownership and RollbackRegistrar. Use one EchoFan-like technique as
a small real migration probe. Preserve its current request/provenance semantics.

1. Create the dependency-light SDK with module/entry/schema descriptors,
   semantic handles, bounded observations and explicit own-state access. Keep
   selected domain request schemas in dependency-light domain owners, not one
   engine-wide request enum. Reuse pure identity/schema primitives where they
   have the right owner; avoid a giant prelude reexporting the engine.
2. Create a host executor/registration adapter with no named game algorithm.
   Keep it below the runtime composition root: it may use Bevy and the existing
   neutral registration vocabulary, but must not depend on
   ambition_platformer2d_runtime. The runtime composes it and domain adapters.
   Reject any import-cycle workaround that moves game code into the host.
3. Add only the observation/request ports needed by the fixture. The owning
   domain supplies projections, validation and lowering. Couple each port's
   advertised support to actual installation. Fill the domain-contract card for
   each port, including submit/apply distinction, cancellation, grants and results.
4. Map fixture entry points to current public phase/occurrence guarantees.
   Record input freshness, output consume barrier, Commands flush point and
   rejection behavior. Implement stable serial entry ordering first.
5. Implement staged invocation outputs and deterministic limits. Make the
   native reference invoke the same semantic contract without copying all
   inputs through a serialized buffer unnecessarily. Stage only changed records;
   batch at the declared scope and preserve the read cut. Never deep-clone the
   complete store for each callback to simulate a transaction.
6. Port the selected algorithm while retaining a test-only reference trace.
   Its state initially uses a narrow explicit state interface which I5 backs
   with generic registration. Do not invent a VM-wide event bus.

**Proposed roots:** `crates/ambition_extension_sdk/Cargo.toml` and
<!-- cite-ok: proposed I4 package, not a baseline path. -->
`crates/ambition_extension_host/Cargo.toml`.
<!-- cite-ok: proposed I4 package, not a baseline path. -->

**Acceptance:** an independent module builds against the SDK without Bevy or
engine implementation. The selected real technique emits the same accepted
requests and occurrence credit as the native reference. Unsupported ports and
phase cycles fail admission. Engine-owned state is writable only through its
public request protocols in the supported API.
**Poison:** provide a fake metadata-only port; actual admission fails. Try direct
health/body mutation through the SDK in a compile-fail test; no such API exists.
Attempt foreign schema writes; runtime validation rejects them. Misorder entry
execution past the domain consume barrier; same-tick request acceptance changes
and the fixture fails. **Trust limit:** these API tests do not sandbox unsafe
native code or protect against a malicious shared library.
**Cheapest checks:** SDK unit/compile-fail tests, host adapter tests with a tiny
Bevy app, one domain integration trace. **Not complete:** statically linking this
reference into the host does not satisfy no-relink procedural iteration.

## I5 - generic extension state through the existing rollback host

**First cut, 2026-10-01 (state as it stands):**

| Part | Where | Witness |
| --- | --- | --- |
| Body-attached records (`BodyRecords`, `extension.body_records`): the store, its checksum, retirement with the body; typed accessors from `record!` | `ambition_extension_host::store`, `ambition_extension_sdk::typed` | host tests; every migrated module's parity suite |
| Session-attached records (`Attachment::Session`; `record! { .. = KEY, per session; .. }`): ONE record for the session, shared by every invocation of the module whatever body it runs for, in `SessionRecords` (`extension.session_records`, schema 286) on the session root — the runtime makes it a required component of `SessionRoot`, so it retires with the session and a new session starts from initial records. No session, or two, faults the call (`Fault::NoSession`); an idle body's `ResetState` does not reset a session record | `ambition_extension_host::{store, exec}`; `ambition_platformer2d_runtime::extension_composition` | host tests (`a_session_record_is_one_record_that_every_invocation_shares`, `a_session_record_with_no_one_session_faults`, `an_idle_body_does_not_reset_a_session_record`, `a_new_session_starts_from_the_initial_record`); `app_it::a_loaded_module_keeps_session_state` — a module the game does NOT link (`fixtures/extension_fixture_modules`, built to WASM and loaded) declares a session tally the host was never compiled with; it counts every press and every third press fires a bolt (a record field deciding a later spawn), under a GGRS sync-test arm. Poison: unregister `extension.session_records` ⇒ the rollback arm counts 3 of 7 |
| Schema evolution on a hot reload: a reload that changes a record's FIELDS is no longer refused — `StateSchema::migrate` carries each live record over by field tag at publication (a kept field keeps its value, also renamed or moved; a new or retyped field starts at its initial value; a removed one is dropped), in every `BodyRecords` and `SessionRecords`. A changed attachment or save policy is still refused. The publication is a mechanical edit the timeline's owner admitted, so no rewind crosses it | `ambition_extension_sdk::StateSchema::migrate`; `ambition_extension_host::reload::migrate_records` | `a_reload_that_reshapes_a_record_migrates_the_live_records_by_tag` (the new code counts on from the migrated record; a retyped field starts again); `a_record_migrates_by_tag_not_by_position_or_name`; `a_reload_that_moves_live_state_to_another_store_or_is_refused_leaves_the_running_code` |
| Module-owned entities: per-entity records go with the entity | see I4's table (`ambition.world.spawn_module_entity`) | `sentry_parity_tests`; the sentry's sync-test arm |

**Open:** save eligibility (`Checkpoint`, `Durable` are refused at admission);
durable references to unloaded entities; a graph/list record populated in real
play (I7.3's fixture); measurement of record visits and copied bytes.

**Class:** DO. **Requires:** I4; I3 before activation of changed state schemas.
Read core snapshot traits, rollback registry/registrar implementation, current
GGRS participation/identity probes, and
`game/ambition_content/src/bosses/specials/rollback.rs`.

1. Implement versioned bounded schemas, collision rejection and canonical
   encode/decode/hash over logical records. Put schema descriptors in immutable
   generation metadata. Generate typed accessors from that schema for Rust;
   avoid a handwritten codec per migrated mechanic.
2. Implement the safe host-owned state store and register its concrete type
   using the current rollback_resource_clone_checksum method. The reference may
   deep-clone active records; immutable versioned chunks are another safe option.
   Keep dormant durable state outside this active snapshot, with explicit pinned
   inputs/handoffs where it affects simulation. Do not clone metadata each tick. SnapshotState decoding has no schema
   context, so do not route it through a global registry. Explicitly cover value checksums, live
   population, dynamic entity creation, reference mapping and session retirement.
   A metadata row or presence-only checksum is not acceptance.
3. Connect attachment to existing semantic IDs and spawner counters. Implement
   required/optional live references separately from durable unloaded references.
   Restore entity mapping before validating references. Test re-created entities,
   not only existing entities whose numbers stay unchanged.
4. Add a module-defined resource, a component-like record, a graph/list and a
   cross-tick cursor. Populate them during real simulation. Include nondefault
   values on both sides of a rollback and a state field controlling a later spawn.
5. Wire save eligibility through the existing persistence/checkpoint owner for
   a required durable fixture. Refuse unsupported policies instead of treating
   everything as either permanent or discardable.
6. Add registry schema identity and update its current consumers in the same
   implementation change. At this baseline the Rust schema fixture and Python
   absence baseline both exist; follow their current owners and do not update
   only one. Prefer a single generator/source if that correction has landed.

**Acceptance:** an actual SyncTestSession rewinds a populated stateful mechanic,
recreates/remaps entities, and produces identical logical state and accepted
request traces. State may be added by a newly loaded schema without recompiling
host storage code. Independent Apps/sessions retire independently. An approved
world record survives a supported save/load while a transient cursor does not.
**Poison:** omit store registration, omit the only differing field from its hash,
leave one counter in a native static, or skip a dynamic population anchor; each
has a separate failing witness. Swap same-shaped records across semantic entities;
the checksum/reference test fails. A simple encode/decode roundtrip does not
replace FI6/FI7. Measure record visits and copied bytes as well as elapsed cost;
one active write must not force a traversal of unrelated dormant records. **Cheapest checks:** schema unit tests, then one populated
real-GGRS integration module. Schema changes also run the narrow schema baseline
guards. Do not run all game scenarios after every codec field change.

## I6 - compare executable backends behind the same contract

**Class:** MEASURE plus a bounded prototype. **Requires:** I3-I5 for honest reload
and rewind evidence. This packet chooses the deployment, not new semantics.
Read M1 and the primary runtime references. Pin versions and target profiles.

1. Use the I4/I5 fixture unchanged as a static native semantic reference.
   Build a WASM module against the small SDK. Supply only admitted deterministic
   imports. Apply the reset, numeric, allocation and work-budget policies from
   the execution contract. Do not expose WASI/files/clocks as convenient defaults.
2. Build a trusted native shared-library prototype with a versioned C entry
   table, fixed-width wire values, caller-owned buffers and no Rust container
   ABI. Implement generation pinning before trying unload/reload.
3. Use the same captured input traces and compare canonical state and domain
   request output across runs. Test failure, malformed output and budget paths.
   Do not extrapolate a float-equivalence result beyond the tested numeric profile.
4. Measure warm module rebuild/load, first-call and steady-state batch costs,
   state reset, allocation, debugger diagnostics and required target feasibility.
   Confirm no runtime host build/link is invoked during module iteration.
5. Record a short decision choosing one production path or naming the precise
   blocker. Remove throwaway prototype dependencies from normal builds. Retain
   useful conformance fixtures, not two production APIs by accident.

**Acceptance:** the report includes raw observations and a justified deployment
choice, not guessed timings or preference for a language brand. Both prototypes
must be honestly bounded; an unsupported platform is reported, not simulated by
running the Linux version. **Poison:** mutate a guest global across calls, allow
an undeclared nondeterministic import, retain a stale callback during reload or
load a schema-mismatched module; the respective conformance test must fail/refuse.
A host relink forced into the helper must fail the iteration witness.
**Cheapest checks:** module tests and selected host backend conformance fixture;
M1 loops. **Do not expand:** into a public mod marketplace, stable ABI for all
Bevy internals, or several production language bindings.

## I7 - deliver the selected procedural path and retire its old road

**I7.3 first cut, 2026-10-01: the graph fixture.** `fixtures/extension_fixture_modules`'s
`trail_loop` (a crate the game does not link; the test builds it to WASM and
loads it) keeps the cells a body passes through as a bounded graph in a
body record (nodes, edges, the last cell, a loop cursor). A move that joins
two connected nodes closes a cycle: breadth-first path, a damage box over the
cycle's cells, the cycle's edges out of the graph. No engine IR, no new port:
it uses `ambition.items.wielded_use` and `ambition.combat.damage_box`.
Witnesses: its unit tests (a straight walk and its way back close nothing; a
jump back over walked ground closes exactly that loop; the bound restarts the
graph); `app_it::a_loaded_module_keeps_session_state::a_loaded_graph_module_closes_a_loop_the_same_way_through_rollback`
(the player walks right, jumps back left and lands on walked floor; one loop
closes near frame 110, the same tick on the GGRS sync-test arm; poison:
unregister `extension.body_records` ⇒ a checksum mismatch at frame 108, the
landing). Open: the graph is a fixture, not a shipped mechanic.

**Class:** DO after M1 selection. **Requires:** I6.
Keep the I4 reference as a test oracle where useful, not a second live provider.
Read the selected boss file, its required-components installation and rollback
registration before removing old state.

1. Harden the chosen loader with exact executable/profile identity, inactive
   validation, source-linked diagnostics and the I3 activation coordinator.
   Guarantee local code edits do not rebuild the host.
2. Migrate the real mechanic completely. Remove its old live system, concrete
   state attachment and bespoke rollback registration only after the new
   populated conformance witness passes. Do not attach both state roads.
3. Add a graph-based procedural fixture: maintain an authored actor's trail or
   another custom graph, detect a cycle, update a bounded record graph and emit
   an existing domain request. Add no game-specific opcode to the engine IR.
   Where a required fundamental query is absent, implement one reusable owner
   port and report that engine work separately from subsequent module edits.
4. Add deterministic cancellation and occurrence cleanup. A despawn, interrupted
   move or room retirement must not leak references or repeated requests.
5. Expose module/schema/port inspection and source-map diagnostics in the current
   agent tool road. Add a scripting binding only when a concrete authoring need
   chooses it; distinguish offline artifact generation from live simulation.
   Simulation scripts use the same state/reset/port contract and conformance suite.

**Acceptance:** a new algorithm and new state schema reach playable behavior
through a separately built module. The graph fixture proves more than the finite
TechniqueFlow vocabulary. An agent can list required ports, inspect state, see
which generation is active and diagnose a failed replacement without private
World access. Current boss behavior passes its parity trace.
**Poison:** reintroduce the old live system and detect double emission; remove a
rewound graph edge/cursor and detect changed cycle results; add a central enum
branch specifically for this graph mechanic and fail FI8's independent consumer
requirement. Do not encode semantic expressiveness as a source-string ban. **Cheapest checks:** module/schema tests, selected mechanic
parity, populated rewind, then M0/M1 loop. Existing demos are regression customers,
not a reason to postpone this path until every demo is complete.

## I8 - improve runtime and snapshot costs where measured

**Class:** MEASURE-guided implementation. **Requires:** I5 and M2; selected backend
for meaningful end-to-end costs. Keep canonical logical state unchanged.

1. Identify the dominant cost among world projection, guest crossings, reset,
   copying, hashing, snapshot retention and restore/resimulation. Record workload
   sizes and write density. Do not start with unsafe dynamic components by taste.
2. Optimize one cause: batch a port, cache a generation-pinned pure projection,
   partition records by scope, add typed chunks, or use snapshot COW/deltas.
   State plainly whether the change moves compile, runtime, memory or all three.
3. Preserve a reference full logical snapshot and compare every optimized
   restore/hash against it. Delta history must have a bounded base/chain and
   deterministic eviction. Derived caches must survive being cleared each call.
4. For any unsafe dynamic ECS layout, review allocation/drop/reference lifetimes
   and code unloading. For incremental hashing, exercise every write entry point.
   Keep physical layout out of the SDK and save identity.
5. Rerun paired M2 samples and relevant M0/M1 cases. Keep an optimization only
   with evidence that its gain is worth its complexity and no hidden regression.

**Acceptance:** measured improvement in the named loop under representative
populations, with identical replay and request outcomes. A constant-factor gain
on two records does not demonstrate scalable open-world state handling.
**Poison:** bypass one COW/dirty barrier, expire a needed delta base, or reuse a
cache across generations; full-reference comparison fails for each defect.
**Cheapest checks:** affected store/port tests plus chosen scaling fixture.
**Do not expand:** into a new rollback scheduler or engine-wide numeric rewrite.

## I9 - independent consumers, flagship pressure and packaging

**Class:** DO with M3/M0 evidence. **Requires:** first delivery I1-I3 for data
acceptance; I7 and acceptable M2 costs for procedural production acceptance.
Partial delivery reports must name which half is complete.

1. Run a genuinely independent consumer with its own manifest/lockfile. Author
   data, build a procedural module and install a raw Bevy plugin through their
   respective supported surfaces. No private-path allowlist loopholes.
2. Drive a repeatable two-body exchange, one stateful boss/encounter, a graph
   algorithm and a persistent record crossing a supported room/save boundary.
   Use the same simulation with headless and available presentation profiles.
3. Demonstrate independent N-participant/actor identity, not hard-coded player
   zero or a singleton active boss. Add an active-region/dormant-world workload
   so persistent state is not conflated with every-tick snapshot population.
4. Package exact artifacts/modules for the supported development and read-only
   installed targets. Verify no undeclared source checkout files are needed at
   runtime. Report unsupported targets and actual fallback deployment semantics.
5. Test two peers with mismatched content, code, state schema, host protocol and
   numeric/runtime profiles; reject before speculative play. Local reload remains
   distinct from a future coordinated online migration feature.
6. Update normal iteration commands and their cheapest validation selection in
   existing authoring/build guidance. Remove obsolete runtime tables and deferral
   text at migrated seams. Update queue/status with actual witnesses, not counts
   copied from this document.

**Acceptance:** both no-host-relink loops are measured end to end, behavior is
visible, the module boundary supports non-IR algorithms, full Bevy extension
still works, real rollback is deterministic, and packaging uses the same logical
content model. No claim that these fixtures implement the whole future game.
**Poison:** substitute an old artifact, omit a required packaged dependency,
flatten all actors to player zero, bind a new code digest to old snapshots, or
change a view's asset quality to alter simulation; each relevant test fails.
**Cheapest check:** independent fixtures and selected integration/scaling cases.
Run a broader assembly/regression checkpoint only when the accumulated changes
warrant it under AGENTS.md, not once per scalar content edit.

## Prevent incomplete migrations from becoming the default

Before editing, record one small seam card with the current writer, readers,
installer, preparation input, live scope, rollback registration, retirement owner
and file to delete or simplify. Use the source map and the packet's actual paths.
A field declaration location or crate name alone is not an owner.

For each delivery slice, implement preparation, installation, live behavior,
restoration and cleanup together for one customer. A loader with no consumer, a
schema with no populated rewind, or a port descriptor without a reducer stays
open. Keep the old implementation only as a noninstalled test oracle during the
slice. Remove its production writers and compatibility exports before closure.

Stop and amend the owner plan when a requested port needs a new domain authority,
when construction can mutate outside its candidate, or when a state handoff has
no single writer. Do not add a fallback, global context, extra bridge registry or
second gameplay path to get a green test. Add the missing owned contract instead.

Do not stop merely because a measurement is unavailable: implement independent
DO work and leave that measurement's acceptance open. Conversely, a benchmark win
does not waive single-authority or rollback requirements. [FI1-FI10](fast-iteration-acceptance.md)
are concrete fixture specifications, not ten new compulsory binaries or a demand
to run all ten after every edit.

## Validation routing and completion receipts

Commands naming current packages below exist at the inspection baseline. New
fixture commands are added by their owning packet only after the files exist.
Do not paste a proposed crate command into a completion report as an executed test.

| Edited responsibility | Normal local check | Escalation trigger |
| --- | --- | --- |
| Planning only | Planning Markdown/pointer tests; diff and link review | None to Rust simply because a plan mentions code |
| Pure move/schema helper | Owning unit tests; independent compiler fixture | Shared format or runtime hydration changes |
| Artifact input only | Compiler validation plus selected prebuilt-host observation | Protocol/schema change, not a scalar move value |
| Procedural algorithm only | Module unit tests and trace replay | New state/port/phase or backend change |
| Schema / state storage | Canonical codec, hash sensitivity, populated real rewind | Shared registry/backend change |
| Host/domain adapter | Selected domain and app_it module | Cross-domain lifecycle/profile change |
| Runtime assembly / package boundaries | Relevant profile, workspace-policy and assembly checks | Deliberate integration checkpoint |

Current examples, run only for the responsibility they cover:

```bash
python3 -m pytest -q scripts/tests/test_planning_markdown_structure.py \
  scripts/tests/test_planning_pointers_are_live.py
cargo test -p ambition_entity_catalog --lib
cargo test -p ambition_content_pack --lib
cargo test -p ambition_characters --lib refused_revision
cargo test -p ambition_workspace_policy
```

For app integration, add tests to the existing shared app_it target and use
`cargo test -p ambition_app --test app_it -- <selected_module>` with the actual
module name. Do not add one integration binary per new case. Check target/disk
prerequisites before builds. The architecture merge gate is the applicable
assembly check; it is not every-edit policy.

Every completion receipt names the source revision, changed authority, removed
old road, positive behavior witness, independent poison and its observed failure,
commands actually run, omitted checks with reason, and measured versus unmeasured
cost claims. Restore the poisoned code before committing. A source grep is not
proof of execution; a successful codec roundtrip is not proof of participation;
a dependency count is not a compile-time speedup.
