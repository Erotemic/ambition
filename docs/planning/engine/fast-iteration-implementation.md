# Fast iteration implementation packets

**Scope:** the packet catalog for fast content and procedural iteration. It is
not a second queue. [The queue](../queue.md) selects execution. The
[extension model](extension-model.md) owns decisions, the
[execution contract](extension-state-and-execution.md) owns state semantics.
[Generation and reload](content-generation-and-reload.md) and
[domain contracts](extension-domain-contracts.md) own their protocol details.
[Acceptance](fast-iteration-acceptance.md) defines FI1-FI10, and
[evidence](extension-iteration-evidence.md) owns M0-M3.

## Start here

Read the model and the protocol and fixture sections for the selected packet
before you change a runtime seam. Read the named source on the working head.
Record the active writer, installer, caller, lifetime, generation and the
cheapest existing behavioral test for that seam. Do not regenerate the whole
repository index or run the whole workspace suite to begin this work.

## Dependency graph and delivery cuts

```text
I0/M0 baseline and measurement helper ---------------------> report gains

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

| Packet | State |
| --- | --- |
| I0 | Recorder for the pack-data loop (`scripts/measure/iteration_record.py`, 2026-10-16); Rust-edit loops are `compile_cost.py`'s |
| I1 | Pure authoring moved; independent fixture `fixtures/content_builder` |
| I2 | Every data source of the Ambition pack and the demo packs is read off disk |
| I3 | Data reload is implemented for the participating families; open items below |
| I4 | First cut landed: every boss special, the wielded kit, the sentry, the vortex and the FSM conductor are modules |
| I5 | Body and session records, schema migration on reload; durable session records; body and checkpoint save roads open |
| I6 | One backend (wasmi) chosen; the native shared-library comparison is not done |
| I7 | Loaded road, hot reload, inspection tool and graph fixture landed; cancellation and source maps open |
| I8 | Not started |
| I9 | Not started |

| Existing program | Exact dependency | Not a dependency |
| --- | --- | --- |
| A6 definitions and materialization | Use its field census to place a moved field | Renaming or extracting all characters or the actor SCC |
| A9 minimal profiles | Reuse resolved-closure guards and the independent-consumer method | Completing every facade capability profile before pure authoring |
| A11/A12 techniques | Keep installed admission, reference validation and occurrence rules when moves load or techniques are supplied | Rebuilding the flow interpreter |
| A2 contact and A4 body | Their published request and observation contracts govern the matching ports | Completing every combat or movement feature first |
| A1 checkpoints | Use its restore and confirmed lifecycle road for required saved state | A new persistence engine inside the extension host |
| A8 multiple worlds | Two-instance FI9 is a planned requirement | Full concurrent-world execution before I1-I3 |
| A10 bounded candidate construction | I3b uses the hidden-candidate road | Arbitrary World cloning or undo |

## I0 - record real iteration costs without blocking clean boundaries

**Class:** MEASURE. Read `Cargo.toml`, `.cargo/config.toml`, `AGENTS.md`, the
B7 build plan and M0. Use existing dependency and absence instruments before
you add a helper.

1. Record the exact move-edit and procedural-edit commands, features, host
   process, assets, cache state and machine. Keep an input trace that makes
   each edit observable.
2. Get no-op and paired edit results with M0. Separate a host relink from
   unchanged dependencies that stay fresh. Include codegen, launch or
   reconstruction and observable readiness, not only `cargo check`.
3. Extend a measurement helper, or add a narrow iteration recorder with the M0
   output schema. It runs an explicitly selected command.
4. Save raw evidence under a generated target report directory. Summarize the
   revision, commands, sample count, result and limitations in the owning plan.
   Do not copy counts into queue, status or tracks.
5. Measure profile, link, feature and cache changes as separate experiments.

**Acceptance:** timing rows correlate edited input, admitted generation,
observed behavior and build or link events. A missing tool is recorded as
unavailable.

**Built 2026-10-16 for the pack-data loop:** a host that reloads a pack
prints one acknowledgment line per event with the generation's fingerprint
(`content::live_reload`: watching, staged with the compile time, published
after the restaters, refused). `scripts/measure/iteration_record.py` starts a named,
already built host, writes a real edit, and times it to the published
acknowledgment of a new generation; each revert must give back the starting
generation, and a save with no change must publish nothing. It writes M0 rows
to `target/iteration_reports/<stamp>/records.jsonl`. It refuses the run for a
host that loads nothing, answers with the running generation, refuses the
edit, or publishes a generation it did not stage (the poison: the host
acknowledged a publication and did not publish; refused). Witnesses:
`scripts/tests/test_iteration_record.py`, one fake-host defect for each
refusal. Not measured yet: the observed behaviour (`first_observed_result_seconds`
is null), and the loops that compile (`compile_cost.py` owns them). **Poison:** skip the load of the new artifact or feed an old host
acknowledgment; the helper refuses the run. Make its metadata command fail; the
closure report fails instead of reporting zero dependencies.

## I1 - pure Rust authoring without the facade

**Current shape:** the pure move-authoring helpers and the `smash_*`
vocabulary modules are in `crates/ambition_entity_catalog/src/authoring.rs` and
the entity-catalog crate. `SmashHoldState` (rollback-registered) is in
`ambition_characters::smash_hold_state`. The independent builder fixture is
`fixtures/content_builder`.

Rules for further moves:

- Move only functions that build pure `MoveSpec` values and the constants they
  need. Do not move live character or body preparation, repertoire policy or
  prefab selection.
- Keep one helper implementation. Delete the old road; do not keep a bridge.
- The fixture's resolved closure has no Bevy, runtime, render, audio, monolith
  or named game provider. Examine normal, build and dev dependencies
  separately; a proc-macro or build-script edge can break the claim.
- Keep emitted move values identical (golden or structural comparison). Do not
  bundle changes to game feel with an ownership move.

**Poison:** make the fixture import the facade; the closure witness fails and
names the path. Mutate one emitted move field; the parity test fails.
**Cheapest checks:** `cargo test -p ambition_entity_catalog --lib`, then the
affected character tests.

## I2 - a loadable move artifact through existing preparation

**Current shape:**

- The artifact codec is `crates/ambition_content_pack/src/artifact.rs`. The
  move section codec is next to its pure value owner; host hydration is next to
  character preparation.
- Every pack's `pack.ron` is the one list of its sources. A development build
  reads the manifest and each source it declares when the pack compiles
  (Ambition: `pack::pack_source_text`; the demos and the versus stage:
  `content_pack! { root: "assets" }` in `ambition_platformer2d`). Each crate's
  `build.rs` calls `ambition_content_pack::embed::embed_pack`, which writes the
  `include_str!` of the same list only under the crate's `static_content`
  feature (web, Android, a build without the source tree). Witnesses:
  `pack_selection_tests::a_source_declared_in_the_roots_manifest_is_played_without_a_build`,
  `content_sdk_tests::a_rooted_pack_reads_its_manifests_sources_and_names_every_one_it_cannot`.
- `platformer_defaults.ron` (starting abilities, `MovementTuning` and an
  optional `feel:` block with `deny_unknown_fields`) is read by
  `Platformer2dGameplayDefaults::load`. `ambition_app::app::movement_defaults_watch`
  writes a saved edit to `EditableMovementTuning` and `EditableFeelTuning`, so
  it takes the developer-edit road (proposed, admitted by the timeline owner,
  published). A change to the starting abilities needs a restart. Witness:
  `edit_to_play_through_the_shell::a_movement_tuning_saved_while_the_game_runs_is_played`.
- Two engine `include_str!` data files remain, and neither is tuning: the LDtk
  entity contract (a schema) and `test_boss_catalog` (behind `test-support`).
**Completed content extraction (October 2026):** Sanic and Mary-O worlds,
normal-development sprite tables/portraits/rigs/flipbooks, and other pack
content read from declared sources without a host rebuild. `static_content`
retains compiled assets for read-only targets. Relevant witnesses include
`each_world_is_read_off_disk_unless_the_build_embeds_it`,
`the_embedded_tables_are_the_tables_a_development_build_projects`, and
`pack_selection_tests` / `content_sdk_tests`. The LDtk entity contract and
`test_boss_catalog` remain intentional schema/test inclusions; the Mary-O
entity manifest is a test-only converter schema. The Oct 11 include census
found no other ordinary-development embedded non-test data in `crates/` or
`game/` beyond documented schemas/feature-gated content.

**Still open:** F11/F12 world reload covers the Ambition world, not running
demo worlds; a demo's authored world edit still requires a demo restart.
Runtime sprite publication is separate from reading tables without a rebuild;
see the visual-generation acceptance in I3 below.

- A world read off disk can be a bad save: a provider's source may refuse
  (`ambition_platformer2d_provider::world_source`, `WorldRefusal`), and the
  session's world preparation fails, retryable, with the reasons. Mary-O's
  source refuses. Witness
  `a_world_that_cannot_be_built_fails_its_preparation_and_the_game_keeps_running`.

Rules:

- Unknown keys, unavailable capabilities, invalid parameters and unresolved
  references refuse admission. Admission inspects installed technique support.
- A compiled table for a migrated input is at most a test-only parity oracle,
  never a runtime fallback.
- Do not serialize `Any` or function pointers, and do not add a second
  validator.

**Poison:** restore `include_str!` or a game-crate dependency for a migrated
input; M0 link tracing or the closure guard detects it. Remove installed
support; admission refuses even with a valid offline schema manifest.

## I3 - coordinated generation publication and local reload

**Class:** DO. **Requires:** I2. Read
[immutable construction](immutable-content-and-transactional-construction.md),
[netcode](netcode.md) and [generation and reload](content-generation-and-reload.md)
before you claim a transaction.

**Current shape:** a reload re-requests the active shell route
(`ShellCommand::ReplaceWith`, a fresh `LoadId`; no history push). The old
generation stays authoritative until the new one activates. A re-preparation
alone does not change the move tables, because the cast is registered once in
`Plugin::build`. So the transaction stages the cast revision with the request
and publishes both at the activation boundary
(`reload::commit_content_generation`).

| Part | Where | Witness |
| --- | --- | --- |
| File watch on every declared source | `ambition_content::content_watch` (off-disk builds) | `edit_to_play_through_the_shell::a_content_file_saved_while_the_game_runs_is_played` |
| Saved edit under a local timeline | `rollback::rebase_local_timeline_onto_published_content` (a restatement; the route's re-activation already ends the session) | `edit_to_play_through_the_shell::a_content_file_saved_under_a_local_timeline_rebases_it` |
| Boss profiles and encounters: candidate catalog admitted at request time against `BossCatalogRegistry::with_replaced`; frozen by the preparation (`PendingGenerationInputs::bosses`) | `reload::BOSS_DOMAINS`; `BossConfig::seed` (`BossSeed`) is resolved at construction from the carried catalog, and `update_boss_encounters` reads HP, phase triggers, outro and music from it | `edit_to_play_through_the_shell::a_boss_tuning_saved_while_the_game_runs_is_played` |
| Character catalog: the whole cast is staged again from the candidate catalog; a change to which characters are built is refused | `ambition_characters::prepared::admit_staged_revision_with_catalog`; `PendingGenerationInputs::catalog` (`candidate_catalog_for`) | `a_character_row_saved_while_the_game_runs_is_played`; `a_catalog_tag_saved_while_the_game_runs_is_built_from_the_candidate_catalog` |
| Frozen session cast: one owner, installed at adoption and removed at teardown; readers use `SessionCast` (the frozen cast while a session runs, the published cast when none does) | `ambition_characters::session_cast::{ActiveSessionCast, SessionCast}` | `a_reader_is_given_the_running_sessions_cast`; `the_frozen_cast_retires_with_its_own_session`; `developer_edits_under_rollback::publishing_a_cast_mid_timeline_leaves_history_resimulating_the_same` |
| Pack-derived families (fighter ladder, encounter waves, seeds, bands, item catalog) and audio registries | `reload::PACK_DERIVED_FAMILIES`, `reload::AUDIO_DOMAINS` | `reload_tests` |
| Dialogue: Yarn files are `yarn_dialogue` sources of the pack (the manifest is their one list; a reload selects the new pack and publishes nothing for them) and assets of the running `YarnProject`; a saved file is compiled with the whole project before it becomes the asset's text | `content_watch::YarnSourceWatch` (`replace_yarn_sources`) | `a_saved_dialogue_edit_is_played_and_a_broken_one_is_refused` |
| Module reload re-mints the session's content in place | `extension_composition::remint_session_content`; `publish_session_content` (also the LDtk world reload's road) | `a_module_reload_rebases_the_local_timeline_onto_the_new_identity` |

Steps that remain as rules:

1. Activation authority is App-scoped. Immutable data can be shared; activation
   authority is not.
2. Candidate hydration returns a candidate without publishing it. Do not call a
   mutating activation early and try to undo it. No domain publishes while
   another still validates.
3. Seal the base epoch and profile. A stale candidate refuses or is prepared
   again.
4. Developer tools use the same request path as the file watch.
5. Local scenario reconstruction, remote-session refusal and presentation-only
   reload follow the classification in the reload page. A supported refusal
   keeps the active scene. Do not promise undo after an arbitrary native plugin
   failure.
6. New state or caches carry their generation. Pending plans, handles and
   observers stay with their generation or retire before the new one steps.

**Published baseline:** App-scoped `SelectedContentPack` is sealed by its
first reader; subsequent competing selections refuse. All App-owned content
installs use the selected pack, and the two-composition tests disagree in ten
families. Candidates for bosses, cast, quest book, cutscenes, audio, music and
other pack-derived tables are staged and published at the activation boundary,
not piecemeal. Demo packs support live reload for participating schemas,
including cast and runtime restaters. This does not imply every schema can
be changed in an active session; a schema without a suitable restater is
refused and requires restart.

**Outstanding I3 witnesses and tooling:**

- Candidate brain-profile catalog arm: exercise a room placement and the
  catalog changed in one reload. No shipped placement currently names a
  `brain_profile`.
- Boss-volume updates: verify the frozen catalog's animation-key effect on
  frame-derived boxes when baked boss art is available.
- Restated demo schemas **inside an active session**, not only between
  sessions. Measure and manage the development-only retained source per
  published generation (historically about 0.3 MB for a Smash edit).
- I3c scenario pin/replay, changed-section explanation, activation status,
  and generation-aware cancellation. Keep refusal, unchanged-content and
  sealed-candidate behavior explicit.
- Running-demo world reload is a distinct I2/I3 customer: the source is read
  off disk, but the existing F11/F12 reload path does not adopt it in place.

**Reference contracts:** `content-generation-and-reload.md`,
`immutable-content-and-transactional-construction.md`, `netcode.md`.

**Acceptance:** a valid generation N+1 becomes visible at one boundary; no
system sees N's definitions with N+1's code or schema. An invalid N+1 leaves
N's digest, character generation, playback references and timeline unchanged.
Two Apps can select different packs. A same-session rebase cannot erase an
earlier unhealthy rollback diagnosis.
**Poison:** publish the character registry before another family validates;
the cross-family publication test fails. Increment the epoch on refusal; the
retention test fails. Reuse a sealed plan after an installer or profile change;
admission fails. FI2-FI4 give the independent assertions.

| Cut | Output | Acceptance |
| --- | --- | --- |
| I3a | Nonmutating candidate hydration, complete-bundle seals, no-op identity and stale-attempt refusal | FI2, FI3 |
| I3b | Typed candidate data split from active mutation; one bounded publication path with A10 | FI4: candidate refusal and valid reconstruction |
| I3c | Scenario pin and replay, changed-section explanation, activation status, generation-aware cancellation | FI1-FI4 plus M0 on the real edit loop |

### Sprite publish success is not live sprite reload

The October 11 sprint removed the sprite tree and authored world data from
the normal development binary's compilation closure. Preserve that win:
saving/publishing a sheet should not recompile or relink the consumer.
However, `ambition_sprite_sheet::baked_sheet_rons::sheet_rons` uses a
first-read `OnceLock` in the nonembedded build,
`baked_part_flipbooks::part_flipbooks` also caches, and the rigged-sprite
reader caches decoded rigs. A running process does not automatically see
edited sprite text merely because no Rust build is needed. That is a
**separate live-iteration capability gap**, not a failed compile-closure
migration. Refer to the [sprite reload owner](content-generation-and-reload.md#sprite-source-identity-and-live-presentation-refresh).
Since 2026-10-16 the sheet manifests and their page images are on a live
road (`content::sprite_reload`): a page image at once, a manifest from the
next session. The body rigs, part flipbooks, landmarks, portraits and pack
catalogs joined the same generation the same day, and each cast made from
the sheets is stated again at the publication.

Do not disable caching per frame as a shortcut. A typed, fingerprinted
candidate should validate/prepare changed sprite products once and publish
at a safe visual-generation boundary. Embedded/static-content targets keep
using their baked tables; headless simulation must not need a texture
asset server. Inspect one changed source, candidate rejection, and actual
on-screen rebind before saying live reload is supported.

## I4 - small procedural SDK and one native semantic reference

**Current shape (first cut landed).** Recipe:
`docs/recipes/writing-a-procedural-module.md`.

| Part | Where | Witness |
| --- | --- | --- |
| SDK: descriptors, schemas, records, canonical digest, typed ports, `Invocation`; no dependencies at all | `crates/ambition_extension_sdk` | unit tests; policy `engine.ambition_extension_sdk-portable` (kind `dependency-none`: a dependency of any name fails it) |
| Host: offers installed with their adapter, admission, serial order, staged writes, fault discard, body store | `crates/ambition_extension_host` | host tests |
| Request ports of one phase are lowered in install order (`ExtensionSet::LowerPort`) | `ambition_extension_host` | `request_ports_are_lowered_in_the_order_they_were_installed` |
| A request is lowered only by the adapter of the phase that submitted it (`ExtensionOutbox::drain` takes the phase; the phase markers `LowersIn`, `InWieldedUse`, ... are the host's). Nothing orders one phase's adapter against another phase's invocations | `ambition_extension_host::exec` | `each_phase_lowers_only_the_requests_its_own_invocations_submitted` |
| Boss ports: trigger `ambition.boss.special_cast`; `ambition.boss.summon`; conduct trigger `ambition.boss.conduct` (selector is the behavior id) with `ambition.boss.conducted_pose`, `ambition.presentation.drawn_row`, `ambition.feedback.burst` | `crates/ambition_boss_special_port`; adapters `ambition_boss_encounter::{extension, conduct}` | port cards and wire tests |
| Combat ports: `ambition.combat.damage_box` (on the owner's effective faction), `ambition.combat.held_damage_box` (combat owns the entity; record `combat.held_damage_boxes`), `ambition.combat.riding_hitbox` | `crates/ambition_combat_port`; adapter `ambition_combat::extension` | port cards |
| Projectile port `ambition.projectiles.spawn` | `ambition_projectile_spec::ProjectileSpawnPort`; adapter `ambition_projectiles::extension` | card on the port |
| Portal port: `ambition.portal.held_pair` (the portal domain owns the two ends; the pair's link is the owner's identity, entry and slot; record `portal.held_pairs`). Offered only with the runtime's `portal` feature | `crates/ambition_portal_port`; adapter `ambition_portal2d::extension` | `ambition_portal2d extension::tests` (owner-keyed pairs, release, orphan sweep, refusal); `app_it gnu_ton_fight::the_punchs_pair_is_one_pair_across_rewinds` (sync test) |
| Portal port: `ambition.portal.light_line` (a line of a portal's light that is no portal: a look, named by its slot). The adapter keeps this tick's lines of each body as a view on the body (`HeldLightLines`, a derived row), and the presentation draws each one (`dress_held_light_lines`); the simulation spawns nothing for a line. Offered only with the runtime's `portal` feature | `crates/ambition_portal_port`; adapter `ambition_portal2d::extension`; drawn by `ambition_portal2d_presentation` | `ambition_portal2d extension::tests::each_body_has_the_lines_its_entry_asks_for_this_tick`; `ambition_portal2d_presentation glow::tests::a_light_line_is_drawn_while_its_body_has_it_in_its_view`; `app_it gnu_ton_fight::his_look_lines_are_the_lines_his_conductor_asks_for` |
| Wielded items: phase `wielded_use` (`ItemPickupSet::WieldedAbilities`, after the native wielded users and before the movement cooldown ticks), trigger `ambition.items.wielded_use` v3, requests `ambition.resources.spend_mana`, `ambition.feedback.body_sound` v2 | `ambition_combat_port::wielded`; adapters `ambition_abilities::extension` | `the_ports_mana_rule_is_the_banks` |
| Body motion: request `ambition.motion.transit` v3 (any body; along a line, walls permitting, by the box of its last step, or to a point; an optional facing; lowered first in `wielded_use`, as a carry of the travelled path), `ambition.abilities.movement_cooldown`, `ambition.combat.strike` v2 (a circle or the span a body crossed; an optional knockback), `ambition.feedback.effect`. A module cannot see the walls, so a request after the transit takes a `Place`: `Place::Body` is where the body is when the request is lowered, which is the arrival | `ambition_combat_port::motion`; adapters `ambition_abilities::extension`, `ambition_combat::extension` | `wielded_ability_parity_tests::transits` (blink, dive and mark/recall, linked and WASM road, a crawler on a wall among the bodies, a mark of the other live room); `app_it::a_wielded_blink_runs_on_the_extension_host`, `a_wielded_dive_runs_on_the_extension_host`, `a_wielded_mark_and_recall_run_on_the_extension_host` (plain and sync-test); `a_blink_stops_at_a_wall_of_its_own_live_room`; `dive_drill_reachability` |
| The mark: trigger `ambition.items.wielded_alternate` (Blink; the same `Wielder`), observation `ambition.items.mark` (the body's mark, none of another live room), request `ambition.items.set_mark`. The mark is the world's fact (`PlayerMark`), which the beacon visual, the session reset and the simulation view read | `ambition_combat_port::wielded`; adapters `ambition_abilities::extension` | the same parity and app witnesses |
| The grapple: observation `ambition.items.aim_cast` (the first solid of the body's live room along its aim, within a reach of 600 px, with how far it is: the host casts, only for an entry that runs), requests `ambition.motion.velocity` (lowered after the body stepped, so the kernel moves the body by it on the next tick, as for the native held item), `ambition.feedback.burst` (also offered in `wielded_use`) and `ambition.feedback.hit_mark`. The module decides the range, the cooldown, the pull and the drawn line from the answer | `ambition_combat_port::{wielded, motion}`; adapters `ambition_abilities::extension`; the room rule `ambition_platformer2d_world::collision::with_room_in_world` | `wielded_ability_parity_tests::transits` (grapple: hits, misses, a wall past the range, a refused press, linked and WASM road); `app_it::a_wielded_grapple_runs_on_the_extension_host` (plain and sync-test) |
| Module-owned entities: request `ambition.world.spawn_module_entity`, phase `module_entity_tick`, `ambition.world.end_module_entity`, `ambition.world.pull_bodies`. A module entity is in its spawner's live room | `ambition_combat_port::module_entity`; adapters `ambition_abilities::module_entity` | `a_turret_is_in_its_deployers_live_room_and_shoots_only_there`; `a_well_pulls_only_the_bodies_in_its_own_live_room` |
| Phase `technique_execution` -> `CombatSet::ContentSpecials`; `extension_composition::install_ports` is the one list of offered ports | `ambition_platformer2d_runtime::extension_composition` | - |
| Modules: all eleven boss specials (shared strike rules in `strike`), the shockwave, beam, volley, meteor, sentry, vortex, blink, dive, mark/recall, grapple and the FSM conductor. The native systems are test-only references | `game/ambition_content_modules/src/` | `specials::module_parity_tests`, `wielded_ability_parity_tests`, `sentry_parity_tests`, `vortex_parity_tests`, `bosses::fsm::fsm_parity_tests` (each on the linked and the WASM road); `app_it::a_boss_special_runs_on_the_extension_host` (GGRS sync-test arms, driving the clockwork warden's `overfit_volley`; the arms drove the Mockingbird's `echo_fan` until `ea9bf8e71` removed it from that profile) |
| D6 module identity: the declared modules are the `extension.modules` section of the prepared content identity (code identity of a loaded module is the digest of its bytes) | `ambition_extension_host::ExtensionGeneration` | `the_prepared_content_identity_names_the_module_code_the_session_runs` |
| D6 local reload: a published module reload re-mints the session content (new section, fingerprint, epoch, identity and binding); same bytes keep the generation; a session whose timeline another owner holds refuses | `extension_composition::remint_session_content` at `MechanicalEditSet::Publish`; `publish_session_content` | `a_module_reload_rebases_the_local_timeline_onto_the_new_identity`; `a_module_file_replaced_while_the_game_runs_takes_over` |
| Fault policy: a fault discards the invocation's staged state and requests, is counted in `ExtensionFaults` on the first execution of its tick (a replayed tick does not count it again: `SimulationReplayState`), and does not stop the session | `ambition_extension_host::exec::run_phase` | `a_fault_discards_the_staged_state_and_the_staged_request`; `a_fault_on_a_replayed_tick_is_not_counted_again` |
| Inspection: composition and per-body records as text; dry-run `--try-replace` | `ambition_extension_host::inspect`; `ambition_app_tools --bin extension_inspect` | `a_call_that_leaves_its_record_initial_stores_nothing` |

Deliberate behavior changes from the native code: a module's box is on the
wielder's effective faction; a summon's id is `<label>:<boss id>:<serial>`; a
sentry deploy that is not paid uses no identity number.

**Class:** DO. Read the boss special producers, domain request types,
`combat_schedule`, `SimId` and session ownership and `RollbackRegistrar`
before you add a port.

1. The SDK is dependency-light: module, entry and schema descriptors, semantic
   handles, bounded observations and explicit own-state access. Domain request
   schemas stay with dependency-light domain owners, not in one engine-wide
   request enum. No prelude that re-exports the engine.
2. The host executor and registration adapter name no game algorithm. Keep it
   below the runtime composition root: it may use Bevy and the neutral
   registration vocabulary, but must not depend on
   ambition_platformer2d_runtime. The runtime composes it and the domain
   adapters. Reject any import-cycle workaround that moves game code into the
   host.
3. Add only the ports that a fixture needs. The owning domain supplies
   projections, validation and lowering. A port's advertised support follows its
   actual installation. Fill the domain-contract card for each port.
4. Map entry points to public phase and occurrence guarantees. Record input
   freshness, the output consume barrier, the `Commands` flush point and
   refusal behavior. Entry order is stable and serial.
5. Stage invocation outputs and apply deterministic limits. Stage only changed
   records. Never deep-clone the whole store per callback.
6. Keep a test-only reference trace for each ported algorithm.

**Delivered migrations:** GNU-ton's conductor and its boss/mount ports,
actor-think S1-S3 (the Void Lizard and Laden Swallow), body-contact C1/C2
(AI Slop and Laden Swallow squash), and the fighter-special F1-F14 route
(riposte, module entity/spring, sleep/song, dilation, resource refill,
parry/counter, homing, mark, bomb/mine, steered bolt, tether, portal recovery,
and shark ride). Their native duplicate systems were removed. The SDK/host
port catalog and parity witnesses are in the table above and the owner source
modules. This is completion history, **not** another list of work to redo.

**Remaining customer migrations and dependencies:**

1. **Mary-O snake shells:** their native `snake.rs` timers and velocities
   still need module-owned per-body state (box timer, slide speed, kick grace),
   wall-contact observations, and the same provider-owned stomp facts. Do not
   duplicate those facts as module constants. Preserve room isolation and
   body-contact ordering.
2. **Sanic badnik:** migrate contact consequences through `body_contact`
   using the existing owner of the toucher's rolling fact (currently the
   demo's `ball_dash::Rolling`) and contact-harm rule
   (`Empowerment::HARMS_ON_CONTACT`), without manufacturing a second
   source of truth. The provider-owned `sanic_rules.ron` bounce speed and
   stomp band must be exposed through a provider observation, following
   Mary-O's `mary_o.rules.stomp` precedent. Its current multi-participant,
   per-room contact behavior is already guarded; preserve it.
3. **Other native demo consumers:** the Snake Plant's path/pose behavior is
   an `actor_think` customer rather than a body-contact rule. Sanic ball dash
   and any remaining move-invoked technique need a concrete domain-port
   inventory before migration. Recheck the working tree first; the historical
   list predates F1-F14 and must not re-enqueue completed Smash specials.
4. **Clock semantics:** modules must use elapsed-time/simulation-rate
   semantics, not assume every tick is 1/60 second. Coordinate with
   [tick-rate compatibility](simulation-tick-rate-compatibility.md).

**Guards to retain:** actor control crosses the single control gate;
`ambition.body.contact` and `contact_bounce`/`defeat` work for each player
in the body's own live room; Mary-O's stomp and dust rows are read through
its published rules; a held hitbox, module entity, ride or portal pair is
retired or re-armed through its own owner. Do not restore the native GNU-ton
conductor, old special handlers or after-gate AI-fire writers. The next
migration must add its adapter witness, populated replay and refusal case
before deleting the native path.

**Acceptance:** an independent module builds against the SDK without Bevy or
engine code. A ported technique emits the same accepted requests and
occurrence credit as the native reference. Unsupported ports and phase cycles
fail admission. Engine-owned state is writable only through its public request
protocols. Direct health or body mutation through the SDK cannot be written:
the SDK has no dependencies, so no SDK signature can name engine state.
**Poison:** a metadata-only port fails admission; a foreign schema
write is refused at runtime; an entry misordered past the domain consume
barrier changes same-tick acceptance and the fixture fails. **Trust limit:**
these tests do not sandbox unsafe native code.

## I5 - generic extension state through the existing rollback host

**Current shape:**

| Part | Where | Witness |
| --- | --- | --- |
| Body-attached records (`BodyRecords`, `extension.body_records`): store, checksum, retirement with the body; typed accessors from `record!` | `ambition_extension_host::store`, `ambition_extension_sdk::typed` | host tests; every module's parity suite |
| Session-attached records (`Attachment::Session`): one record per session in `SessionRecords` (`extension.session_records`), a required component of `SessionRoot`. No session, or two, faults the call (`Fault::NoSession`) | `ambition_extension_host::{store, exec}` | `a_session_record_is_one_record_that_every_invocation_shares`; `app_it::a_loaded_module_keeps_session_state` (a module the game does not link, under a GGRS sync-test arm) |
| Schema evolution on reload: `StateSchema::migrate` carries each live record by field tag; a changed attachment or save policy is refused | `ambition_extension_sdk::schema`; `ambition_extension_host::reload` | `a_reload_that_reshapes_a_record_migrates_the_live_records_by_tag`; `a_record_migrates_by_tag_not_by_position_or_name` |
| Idle ticks: `IdlePolicy::ResetState` and `ResetStateExcept(keep)` reset records without a call | `ambition_extension_host::admission` | host tests; module parity suites |

**Durable session records (2026-10-11):** a session record declared `kept
durable` is kept in the durable world save (`module_records`, save v5) and
comes back when the save loads; a transient record beside it does not; a New
Game clears the session's records. The host admits the schema only where the
composition offers `SaveRoad::SessionDurable` with its adapter
(`a_policy_without_a_host_road_is_refused`). A row carries its schema, so a
load after a module edit migrates it by tag, and a row of a module that is not
loaded stays in the save (`a_saved_row_comes_back_and_a_row_of_an_older_shape_migrates`).
Witness: `a_durable_session_record_comes_back_with_its_save_and_a_transient_one_does_not`
(the loaded fixture, through the file; poisons: no restore, no mirror, no New
Game reducer, each red at its own assertion; no offer, refused at admission).
A death keeps the record. The save's flags and quest steps do not: since 2026-11-11 a death that spares nobody puts them back to the checkpoint with the bag (Q164, DIALOGUE-CHECKPOINT slice 1).

**Open:** save eligibility for body records and checkpoints (`Body` +
`Durable` and `Checkpoint` are
refused at admission); durable references to unloaded entities; measurement of
record visits and copied bytes.

**Rules:** schema descriptors live in immutable generation metadata. Register
the store through the current rollback registration method. `SnapshotState`
decoding has no schema context, so do not route it through a global registry.
Restore entity mapping before you validate references; test re-created
entities. Keep dormant durable state outside the active snapshot. A metadata
row or a presence-only checksum is not acceptance.

**Acceptance:** an actual `SyncTestSession` rewinds a populated stateful
mechanic, re-creates and remaps entities, and gives identical state and request
traces. A newly loaded schema adds state without a host recompile. An approved
world record survives a supported save and load; a transient cursor does not.
**Poison:** omit store registration, omit the only differing field from the
hash, keep one counter in a native static, or skip a dynamic population anchor;
each has a separate failing witness.

## I6 - compare executable backends behind the same contract

**Current shape:** one backend. ABI `ambition-ext-1` (three exports, no
imports, bytes in and out; `ambition_extension_sdk::{abi, wire}`,
`export_modules!`). The WebAssembly backend `crates/ambition_extension_wasm`
uses wasmi in deterministic mode with fuel and a new instance per call. It
refuses an importing module. Reason for wasmi: deterministic by construction
(NaN canonicalization, fuel instead of a clock), pure Rust, builds for every
shipped target. A new instance per call is what stops a guest static from
carrying state across a rewind.

**Open:** the trusted native shared-library prototype (versioned C entry table,
fixed-width wire values, caller-owned buffers, generation pinning before
unload) and the M1 comparison against it. The per-call glue cost (instance
reuse with a restored image; a lighter input encoding).

**Acceptance:** raw observations and a justified deployment choice. An
unsupported platform is reported, not simulated. **Poison:** mutate a guest
global across calls, allow an undeclared nondeterministic import, keep a stale
callback through reload or load a schema-mismatched module; the matching
conformance test fails. **Do not expand** into a mod marketplace, a stable ABI
for Bevy internals or several production language bindings.

## I7 - deliver the selected procedural path and retire its old road

**Current shape:**

| Part | Where | Witness |
| --- | --- | --- |
| Loaded runner; loaded output checked like native output; explicit replacement | `ambition_extension_host` (`DeclaredModule`, `ModuleBackend`, `Admitted::replaced`) | `a_loaded_module_replaces_a_native_one_only_when_it_says_so` |
| Hot reload in the shipped composition: a changed file is polled, proposed through the mechanical-edit protocol and published; last-good by construction | `ambition_platformer2d_runtime::extension_composition` (`load_developer_modules`, `propose_module_reload`, `publish_module_reload`) | `a_module_file_that_changes_while_the_game_runs_is_reloaded`; `a_module_file_replaced_while_the_game_runs_takes_over` |
| A file is the unit of replacement; one poll gives one candidate; a departed schema takes its records; a stored record of another shape faults (`Fault::StaleRecord`) | `ambition_extension_host::reload` (`stage_loaded_replacements`), `exec::read_state` | `a_module_a_rebuilt_file_no_longer_exports_leaves_with_it`; `two_files_changed_before_one_publication_both_take_over`; `a_departed_schemas_records_go_and_do_not_come_back`; `a_stored_record_of_another_shape_faults_the_invocation` |
| Developer road | `AMBITION_EXTENSION_MODULES` or `ExtensionModuleFiles`; runtime feature `wasm_modules`; `scripts/regen/build_extension_modules.sh` | `a_module_rebuilt_as_wasm_replaces_the_linked_one_in_the_same_game` |
| Graph fixture: `trail_loop` keeps a bounded graph in a body record and closes a cycle with existing ports | `fixtures/extension_fixture_modules` | `a_loaded_graph_module_closes_a_loop_the_same_way_through_rollback` |

**Open:**

- The shipped app's own local session (the `LocallyRebasable` arm) taking a
  module reload has no test; the harness cannot construct that ownership.
- Deterministic cancellation and occurrence cleanup: a despawn, interrupted
  move or room retirement must not leak references or repeat requests.
- Source-map diagnostics in the agent tool road.
- The graph fixture is not a shipped mechanic.

**Rules:** keep the native reference as a test oracle, not a second live
provider. Remove an old live system, its state attachment and its rollback
registration only after the new populated witness passes. Do not attach both
state roads. Add no game-specific opcode to the engine IR; where a fundamental
query is absent, add one reusable owner port and report it as engine work.

**Poison:** reintroduce the old live system and detect double emission; remove
a rewound graph edge or cursor and detect a changed cycle result; add a central
enum branch for the graph mechanic and fail FI8.

## I8 - improve runtime and snapshot costs where measured

**Class:** MEASURE-guided implementation. **Requires:** I5 and M2. Keep
canonical logical state unchanged.

1. Find the dominant cost among world projection, guest crossings, reset,
   copying, hashing, snapshot retention and restore or resimulation. Record
   workload sizes and write density.
2. Optimize one cause: batch a port, cache a generation-pinned pure projection,
   partition records by scope, add typed chunks, or use snapshot COW or deltas.
   State whether the change moves compile, runtime, memory or all three.
3. Keep a reference full logical snapshot and compare every optimized restore
   and hash against it. Delta history has a bounded base and chain and
   deterministic eviction.
4. For an unsafe dynamic ECS layout, review allocation, drop, reference
   lifetimes and code unloading. Keep physical layout out of the SDK and save
   identity.
5. Rerun paired M2 samples. Keep an optimization only with evidence.

**Poison:** bypass one COW or dirty barrier, expire a needed delta base, or
reuse a cache across generations; the full-reference comparison fails for each.
**Do not expand** into a new rollback scheduler or an engine-wide numeric
rewrite.

## I9 - independent consumers, flagship pressure and packaging

**Class:** DO with M3 and M0 evidence. **Requires:** I1-I3 for data acceptance;
I7 and acceptable M2 costs for procedural production acceptance.

1. Run an independent consumer with its own manifest and lockfile. Author data,
   build a procedural module and install a raw Bevy plugin through their
   supported surfaces.
2. Drive a two-body exchange, one stateful boss or encounter, a graph algorithm
   and a persistent record that crosses a room or save boundary. Use headless
   and presentation profiles.
3. Show independent N-participant identity, not hard-coded player zero or a
   singleton active boss. Add an active-region and dormant-world workload.
4. Package exact artifacts and modules for development and read-only installed
   targets. Verify that no undeclared source checkout file is needed at
   runtime.
5. Test two peers with mismatched content, code, state schema, host protocol and
   numeric profiles; refuse before speculative play.
6. Update iteration commands in authoring and build guidance; remove obsolete
   runtime tables at migrated seams.

**Poison:** substitute an old artifact, omit a required packaged dependency,
flatten all actors to player zero, bind a new code digest to old snapshots, or
change a view's asset quality to alter simulation; each relevant test fails.

## Prevent incomplete migrations from becoming the default

Before you edit, record a seam card: current writer, readers, installer,
preparation input, live scope, rollback registration, retirement owner and the
file to delete or simplify. A field declaration location or a crate name alone
is not an owner.

For each slice, implement preparation, installation, live behavior,
restoration and cleanup together for one customer. A loader with no consumer, a
schema with no populated rewind, or a port descriptor without a reducer stays
open. Keep the old implementation only as a test oracle that is not installed.

Stop and amend the owner plan when a port needs a new domain authority, when
construction can mutate outside its candidate, or when a state handoff has no
single writer. Do not add a fallback, global context, extra bridge registry or
second gameplay path to get a green test.

A missing measurement does not stop DO work; it leaves that measurement's
acceptance open. A benchmark win does not waive single-authority or rollback
requirements. [FI1-FI10](fast-iteration-acceptance.md) are fixture
specifications, not ten compulsory binaries.

## Validation routing and completion receipts

| Edited responsibility | Normal local check | Escalation trigger |
| --- | --- | --- |
| Planning only | Planning Markdown and pointer tests; link review | None |
| Pure move or schema helper | Owning unit tests; independent compiler fixture | Shared format or runtime hydration changes |
| Artifact input only | Compiler validation plus a prebuilt-host observation | Protocol or schema change |
| Procedural algorithm only | Module unit tests and trace replay | New state, port, phase or backend |
| Schema or state storage | Canonical codec, hash sensitivity, populated real rewind | Shared registry or backend change |
| Host or domain adapter | Selected domain and `app_it` module | Cross-domain lifecycle or profile change |
| Runtime assembly or package boundaries | Profile, workspace-policy and assembly checks | Deliberate integration checkpoint |

```bash
python3 -m pytest -q scripts/tests/test_planning_markdown_structure.py \
  scripts/tests/test_planning_pointers_are_live.py
cargo test -p ambition_entity_catalog --lib
cargo test -p ambition_content_pack --lib
cargo test -p ambition_workspace_policy
```

For app integration, add tests to the shared `app_it` target and run
`cargo test -p ambition_app --test app_it -- <selected_module>`. Do not add one
integration binary per case.

A completion receipt names the source revision, changed authority, removed old
road, positive behavior witness, independent poison and its observed failure,
commands run, omitted checks with reason, and measured versus unmeasured cost
claims. A source grep is not proof of execution; a codec roundtrip is not proof
of participation; a dependency count is not a compile-time speedup.
