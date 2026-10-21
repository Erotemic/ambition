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
| I0 | Recorder for the pack-data loop (`scripts/iteration_record.py`, 2026-10-16); Rust-edit loops are `compile_cost.py`'s |
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
after the restaters, refused). `scripts/iteration_record.py` starts a named,
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
- Not in that count, and still compiled in (seen 2026-10-11, in level and art
  work on Mary-O; an edit to one needs a build before a capture or a test
  shows it):
  - The world file of a demo. ✔ Sanic (2026-10-11): its three worlds are
    `content_text!` (`ambition_platformer2d`), one `PackText` each, read off
    disk unless the demo's `static_content` embeds them. A world edit cost
    10.3 s and compiled `ambition_demo_sanic` and its app; it now costs
    0.78 s and compiles nothing (`cargo build -p ambition_demo_sanic_app
    --bins`, warm). Witness
    `each_world_is_read_off_disk_unless_the_build_embeds_it` (poison: one
    world embedded, red at its arm); policy
    `game.demo-worlds-are-not-compiled-in`. ✔ Mary-O (2026-10-11):
    `MARY_O_WORLD` the same way, and `ldtk_migration_tests` reads it too; a
    world edit costs 0.69 s and compiles nothing (`cargo build -p
    ambition_demo_mary_o_app --bins`, warm). Witness
    `the_world_is_read_off_disk_unless_the_build_embeds_it` (poison: the
    world embedded, red at its arm and at the policy). A capture
    (`capture_mary_o`, `capture_scene --route mary_o_gameplay`) reads the
    world through the same function; not run here. Open: the F11/F12 world
    reload serves the Ambition world only, so a demo world edit costs a
    restart.
  - ✔ The published sprite tree (2026-10-11): each `*_spritesheet.ron`,
    portrait manifest, body rig, part flipbook, landmark table and
    ultrapack catalog. `build.rs` of `ambition_sprite_sheet` embeds them
    only under its `static_content` (forwarded by `ambition_platformer2d`'s,
    so `desktop_player`, the web build and `android` have them) or on a
    wasm or Android target, and sets `cfg(embedded_sprites)`. A development
    build watches no file of the tree: each `baked_*` table is a function
    that reads the tree on first use (`sprite_tree`, the one scan
    `build.rs` includes too). Measured with `cargo build -p
    ambition_demo_sanic_app --bins`, warm, after touching one
    `*_spritesheet.ron`: 185 s and 23 crates before, 0.67 s and none
    after. The price: a development build parses a part flipbook's RON
    when a sheet first realizes it (about 50 ms, against 3.5 ms to decode
    bincode), and parses the full-resolution flipbooks at startup for the
    landmark digest, on up to six threads. Witnesses
    `every_table_decodes_from_its_bincode_to_the_flipbook_its_ron_is` (the
    bincode road, in both builds) and, under `--features static_content`,
    `the_embedded_tables_are_the_tables_a_development_build_projects`
    (poison: one projected table dropped, red at the tables assertion).
    `run_tests.py` denies `static_*` from the union job, so the job
    `ambition_sprite_sheet [static_content] — the embedded sprite tree` runs
    that second witness, and `required_checks.py` requires it for a change
    to the crate (`test_a_change_to_the_sprite_tree_requires_its_embedded_road`;
    poison: the rule removed, red).
  - The entity manifest of a demo world (`mary_o.entities.json`): a schema,
    as the LDtk entity contract is. Decided 2026-10-11: it stays. It is a
    `#[cfg(test)]` include in no shipped binary; it pins the manifest the
    LDtk tools read against the converters the demo installs, and a real
    change to it needs a converter change in Rust.
  No packet owned these three. The first two cost a build for each level
  edit and each sprite publish of a demo.
- Census, 2026-10-11, after the three above: every `include_str!` and
  `include_bytes!` in non-test code of `crates/` and `game/` is behind
  `static_content`, `static_map` or `static_sfx_bank`, is a test fixture
  (`#[cfg(test)]`, `test-support`), or is one of the two schemas above. The
  only other build script that watches content (`game/ambition_app/build.rs`,
  the packed SFX bank) returns early unless `static_sfx_bank` is on. Measured:
  touching `character_catalog.ron` then `cargo build -p ambition_app --bin
  ambition_game_bin` compiles nothing (0.83 s). A content edit that still
  compiles is content written as Rust (the technique row of the M0 results).
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

**Open:**

- Boot-pack readers: DONE (step 1). Every App-owned content install (items,
  encounter waves, fighter ladder, character catalog and cast, boss fragment,
  audio registries, quests, cutscenes, music cues, and the startup content
  validation) derives from the App's `SelectedContentPack`; `plugin.rs` and
  each `register` call `pack::select` once. The boot read is
  `pack::shipped()`, named for its subject (offline validation, tools,
  source-content tests), and
  `pack_selection_tests::production_code_reaches_the_boot_pack_only_in_the_boot_scoped_roads`
  counts its production uses per file. `two_real_compositions_install_their_own_pack_in_every_family`
  composes the real plugin over two packs that disagree in ten families.
  Selection is still not publication: only the reload transaction revises an
  installed family. The first consumer (`pack::select`) SEALS the selection;
  `select_pack` of a different pack afterwards is refused and changes nothing
  (`a_different_pack_selected_after_composition_is_refused_and_changes_nothing`),
  so the selected identity, the installed families and the lazily filled quest
  book cannot split across generations. The census also counts the `shipped_*`
  helpers, `authored_movesets::` and `lineage()`, not only `pack::shipped()`.
- The presentation provider lookups, match preparation and readers below the
  monolith still read the App cast on purpose: shell menus have a session gate
  and no generation.
- The brain-profile arm of the candidate catalog has no witness. No shipped
  placement names a `brain_profile`, so a witness must save a room placement
  and the catalog in one reload.
- The boss-volume change (`refresh_boss_damageable_volumes` on the frozen
  catalog) has no witness on a machine without baked boss art:
  `BossVolumeContext` uses the catalog only for animation keys of
  sprite-frame-derived boxes. An arm needs the art and an edit to an animation
  key that the boss plays.
- A demo pack reloads in a running demo (2026-10-16):
  `ambition_platformer2d::content::live_reload` watches the pack's files,
  compiles an edit (Smash 7.7 ms, Mary-O 6.3 ms, Sanic 4.2 ms), and
  publishes it when no route prepares a session: between sessions, or inside
  one for a change to schemas read at use where the rollback timeline allows
  it (`mechanical_mutation_boundary`; a timeline this host owns is rebased in
  the same step). The schemas the demo reads where it uses them (Smash: `smash_rules`,
  `smash_stages`, `match_rules`; Mary-O: `mary_o_rules`, `mary_o_forms`; Sanic:
  `sanic_rules`), and the schemas a demo registered once at start, which a
  restater states again at the publication: each demo's `room_rules`, its
  `music_registry` and `sfx_registry`, and `prop_sheets`, `platform_looks`,
  `projectile_visuals`, `world_item_art`, and the cast (`character_catalog`,
  `moveset`, `smash_fighter`: one cast revision with its catalog). Every
  schema of the three demo packs takes part; a schema a future demo reads
  at start and gives no restater is refused and costs a restart.
  Each published generation is leaked (a reader holds `&'static` rows), about
  0.3 MB of source for Smash for each saved edit, in a build that reads
  off disk only. Open: a restated schema inside a live session.
- The quest book takes part as session-derived state (2026-10-07): witnesses
  `a_quest_edit_is_played_from_the_next_session_with_the_players_progress`,
  `a_refused_candidate_leaves_the_quest_book_at_the_live_generation` and
  `a_quest_book_with_no_place_for_a_recorded_step_is_refused`. The cutscene
  library does (2026-10-07):
  witnesses `a_cutscene_edit_is_visible_with_its_session_and_with_the_other_changed_families`,
  `a_refused_candidate_leaves_the_cutscene_library_at_the_live_generation`,
  `an_unchanged_or_stale_cutscene_candidate_publishes_nothing`, and the unit
  arms `a_cutscene_publication_replaces_only_the_rows_the_pack_owns` and
  `a_cutscene_file_the_candidate_drops_removes_its_rows_and_only_its_rows`. The
  music-cue catalog does (2026-10-07): witnesses `an_adaptive_cue_edit_is_visible_with_its_session_and_with_the_other_changed_families`,
  `a_refused_candidate_leaves_the_adaptive_cues_at_the_live_generation`,
  `a_candidate_that_drops_the_adaptive_cues_is_refused_and_the_live_cues_survive`
  and `an_unchanged_or_stale_cue_candidate_publishes_nothing`.
- I3c tooling: scenario pin and replay, changed-section explanation, activation
  status and generation-aware cancellation.

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

**Open:**

- ✅ 2026-10-11, GNU-ton's conductor as a module. Its back stopped being the
  conductor's first (2026-10-10): the gnu's character authors it (`CharacterMount::back`) and
  the mount domain makes a ridden mount's back one-way ground
  (`ambition_mount::mount_backs_are_ground`), so a module does not need to
  add collision. The rest moved with the module: the fists (pose, opening,
  the blow they pass on, their hit volumes), the stomp's travelling shocks,
  the buck, eureka.
  Ports land first, each with its adapter witness; the module, its parity
  harness and the deletion of the native conductor are one slice. Landed:
  observation `ambition.boss.mount` (the ridden body, its back, the hall under
  it, its limbs with rest, lost health and hold), with `boss_conduct` ordered
  after the rider sync; request `ambition.boss.limb_pose` (hold a limb of the
  ridden mount at a pose, or release it to fall straight down);
  `ambition.presentation.drawn_row` v2 (a row on the boss, its mount or a
  limb, with fallback names); trigger `ambition.boss.conduct` v6 (the
  boss's health and size, whether the encounter refuses damage now, and the
  tracked body now); requests `ambition.boss.limb_opening` and
  `ambition.boss.limb_blow` (a fist refuses damage until it is an opening;
  the blow it takes heals and passes on to the scholar once);
  `ambition.boss.mount_launch` (the buck's hop) and
  `ambition.feedback.speech_bubble` (eureka); `ambition.boss.limb_bond`
  (where a fist's trail starts, 2026-10-12). The fist hit volumes and the
  stomp's travelling shocks need no new port: `ambition.combat.held_damage_box`
  v2 keeps a box while it is re-submitted, moves it to the submitted centre,
  and a new generation re-arms it with a fresh hit memory. The module
  (`ambition_content_modules::gnu_ton`, its choreography beside it) and its
  parity harness (`bosses::gnu_ton::conductor_parity_tests`: identical to the
  native conductor for 1420 ticks of every move, the blows, the buck, the
  shocks, eureka and death) are landed. The harness found that the native
  stomp threw no shock (the wave was dropped on the tick it began); fixed.
  ✅ 2026-10-11: the module is composed and conducts the fight. The native
  conductor is the harness's test-only reference
  (`bosses::gnu_ton::conductor_reference_tests`), `content.gnu_ton_conductor`
  left the rollback registry (schema 341), and the app reads the module's
  record (`bosses::gnu_ton::conductor_of`). Poison: the module removed from
  `MODULES` reddens 12 `gnu_ton_fight` arms and the two-room hall arm.

- 2026-10-11 (S1-S3 ✅), **a custom brain as a module** (`actor_think`).
  A custom brain now decides through a loaded module after
  `ActorDecisionSet::Publish` and before the single
  `PlayerInputSet::ControlGate`. The Void Lizard and Laden Swallow have been
  migrated and their late native fire writers deleted; the production
  witnesses cover normal and held bodies. The old after-gate bypass from
  `ed0d1ccd` is **historical**, not remaining work. Moving their behavior
  from demo Rust to modules reduced one edit from 23.7 s for a host build
  to 0.34-0.57 s for a module (different measured edits; no claim of a
  controlled benchmark). The `Body-control contribution` route in
  `extension-domain-contracts.md` serves these brains. What remains in
  demo Rust includes plant/snake/AI Slop and Sanic contact consequences,
  plus move-invoked techniques: these need a body-contact phase and domain
  ports, not another custom-brain dispatcher. The SHOT-END-REPLAY
  rollback defect its sync-test arm found is fixed (a body's motion facts
  come back with a rewind; [queue](../queue.md#shot-end-replay--a-brain-that-saw-its-shot-end-replays-differently---done-2026-10-11)).
  - S1 ✅ (engine): the phase `actor_think`, after `ActorDecisionSet::Publish`
    and before `PlayerInputSet::ControlGate`. The trigger
    `ambition.actor.think` (selector: the `CharacterBrain::Custom` key) gives
    the body's position, velocity, facing, ground, health, size and the
    position of its target. The request `ambition.actor.fire` puts a fire
    direction into the body's control frame, so the gate restricts it like a
    brain's own fire. ✅ Witness
    `app_it::a_custom_brain_module_fires_through_the_gate`: the loaded
    fixture module `thinker` (`fixtures/extension_fixture_modules`) bound to
    the key `fixture_thinker` fires straight up in the gated frame and the
    body's moveset fires the shot; a body a script holds fires none; with no
    module, no frame or shot goes straight up. Poisons: the phase in
    `WorldPrepSet::AfterIntegrate` (after the gate) is red at the held body's
    shots (23 frames of shots); the fire dropped by the adapter is red at
    the free body's gated frame. The sync-test arm found SHOT-END-REPLAY
    (queue), a defect on the road from a shot's end to a brain.
  - S2 ✅ (Mary-O): the Void Lizard is the module
    `ambition_demo_mary_o_modules::void_lizard` (its throws, and its hop
    through `ambition.motion.velocity`, now offered in `actor_think`); the
    native `run_void_lizard` is deleted, and `sim_phase_pins` lost that row.
    The Mary-O app has `wasm_modules`, so a developer run loads the file:
    an edit is 0.57 s (`scripts/build_extension_modules.sh
    ambition_demo_mary_o_modules`) against 23.7 s. The hop is one tick
    earlier than the native rule's: the module asks before the body
    integrates, the native rule asked after. Witnesses
    `mary_o_it::the_void_lizard_throws_and_hops_on_its_modules_schedule_and_not_when_held`
    (each gated frame of a round against the module's schedule; a held
    lizard throws nothing) and `the_void_lizard_runs_from_its_module_file`
    (the same round from the `.wasm`, its runner `Loaded`). Poisons: the
    module not linked is red at tick 34's throw; the hop not asked is red
    at tick 165; the file not loaded is red at the runner (`Native`).
  - S3 ✅ (2026-10-11): the Laden Swallow is
    `ambition_demo_mary_o_modules::laden_swallow`; `ambition.actor.think`
    v2 gives the placement id, which offsets each bird's drop tick. The
    native `drop_coconuts` is deleted, and the control-gate census
    (`sim_phase_pins`) is green: no Mary-O rule writes control after the
    gate. Witness
    `mary_o_it::the_laden_swallow_drops_on_its_modules_rule_and_not_when_held`
    (each gated frame of two periods against `drops_on` and `over`; a held
    swallow drops nothing). Poison: the adapter sends no placement, red at
    tick 115 (`EnemySpawn-108048`).
  - Contact route C1-C2 ✅ (2026-10-12). C1 (engine): the phase
    `body_contact`, in `WorldPrepSet::AfterIntegrate` and before
    `WorldPrepSet::ContactDamage`. The trigger `ambition.body.contact`
    (selector: the `CharacterBrain::Custom` key) gives the touched body and
    each player body of its own live room that touches it (side to side
    overlap; top to bottom overlap or meet, so a body standing on a head
    touches it), in identity order. Requests: `ambition.body.contact_bounce`
    (a toucher by index jumps off; an optional pogo cue) and
    `ambition.body.defeat` (health zero before the contact damage, then
    removed); `ambition.feedback.burst` and `ambition.feedback.body_sound`
    are offered there too (one burst lowering, generic over the phase).
    Witnesses `ambition_combat extension::contact::tests` (a stomp bounces
    and defeats; a side touch, an unbound brain and an untouched body are
    left alone; a player of another live room does not touch). Poisons: no
    defeat, red at "the stomped walker is gone"; a strict top, red at
    "touches(body, standing)"; no room filter, red at "the walker is
    stomped only by a player of its own room". C2 (Mary-O): the AI Slop and
    Laden Swallow squash is the module `ambition_demo_mary_o_modules::squash`;
    the native `bounce_squash_ai_slop` is deleted, and the top/side rule
    moved into the modules crate (`stomp`), which the native snake shells
    read through `crate::stomp`, so the two can not disagree. Witness
    `mary_o_it::a_mob_is_squashed_by_each_player` (each player of the
    mob's room squashes it, a player of another room does not); poison:
    the module not linked, red at "the premise: her stomp squashes it".
    Next: the snake shells and the Sanic badnik. Measured 2026-10-12: the
    plant is not a contact rule (it moves on its path and stays down near a
    player: a brain and a pose, `actor_think`). The badnik needs two facts
    of the toucher that the trigger does not give: that it rolls (the
    demo's own `ball_dash::Rolling`) and that it harms on contact (the
    monolith's `Empowerment::HARMS_ON_CONTACT`). Each has one owner now, and
    a toucher fact copied from it for the port would be a second one; the
    road is to give the trigger the owner's fact where it is defined, which
    for the roll means moving `Rolling` out of the demo. The badnik rule
    read one global `PrimaryPlayer` and no room; since 2026-10-12 it reads
    each player of the badnik's live room
    (`badnik::tests::each_player_of_the_badniks_room_squashes_it_and_no_other`).
    Blocked 2026-10-16 by data: the badnik's bounce speed and the stomp band
    are now pack data (`badnik.bounce_speed` and `stomp.band` in
    `sanic_rules.ron`, read by the badnik and the monitors through
    `rules::feet_on_top`). A module's `schemas` are its own state; no port
    gives a module its pack's rules. So a port today would move these two
    facts from the pack into module constants, which the content-as-data
    rule forbids. Since 2026-10-16 a module can read its provider's rules:
    the provider owns an observation port whose value is the rows the
    module reads, and answers it from its published pack at each invocation
    (`mary_o.rules.stomp`, `ambition_demo_mary_o_modules::rules`, answered
    by the Mary-O plugin from `stomp` in `mary_o_rules.ron`; the squash
    module's `BOUNCE_SPEED` and the modules crate's `STOMP_BAND` are
    deleted, and the shell reads the same `stomp.bounce_speed`). Since
    2026-10-18 the observation (v2) also gives `stomp.dust`, the one dust
    row of a stomp, which the snake's shell plays too; the module's `DUST`
    constant, a second copy of the snake's row, is deleted (witness
    `mary_o_it::the_squash_plays_the_dust_its_rules_state`). The badnik
    can take the same road for its two facts; its toucher facts above still
    block it. The snake shells read Mary-O's
    `stomp` rule through the modules crate, but their own timers and
    speeds are Rust in `snake.rs`, and they need module-owned per-body
    state (box timer, slide speed, kick grace) and wall contact, which no
    port gives either.
  - Measured 2026-10-11 (the case before C1-C2): the demo behaviour still in Rust
    is contact rules and techniques, not brains. Mary-O's plant, snake
    shells and AI Slop, and Sanic's badnik, decide what a touch does (a
    stomp bounces the toucher and defeats the body, a roll defeats it) in
    `WorldPrepSet::AfterIntegrate` before the contact damage: they need a
    body-contact phase and its ports, which no route in
    `extension-domain-contracts.md` names yet. Sanic's ball dash and the
    Smash demo's specials (`bolt`, `bomb`, `counter`, `dilation`, `homing`,
    `limit`, `mark`, `mine`, `riposte`, `spring`, `tether`, `sing`,
    `capture`, `portal`) are move-invoked techniques (`technique_execution`
    exists); each needs its domain's ports (sleep and lock timers, steering,
    a mine as a module entity). Write the contact route first: it serves
    four rules in two demos. (Written since: `body_contact`, with
    `ambition.body.contact`, `contact_bounce` and `defeat`.) Measured
    2026-10-18 for the smallest special, Smash's `riposte` (105 lines: on
    its special action, one body strike in front of the fighter): two
    ports are missing. `technique_execution`'s only trigger is
    `ambition.boss.special_cast`, so a fighter's special reaches no module;
    and the cut is a box that follows its owner for `lifetime_s` with a hit
    sound, where `ambition.combat.strike` (v2) lands in its own tick only
    and `held_damage_box` (v2) must be submitted again each tick and has no
    sound. The first port to write is a fighter-special trigger; each
    special after it then needs only its domain's requests.
  - Special route F1 ✅ (2026-10-18). The trigger `ambition.fighter.special`
    (selector: the effect key a move authors) gives the fighter's position,
    the sign of its facing and the move's params, each leaf by the path an
    author reads (`ParamValue::leaves`). The request
    `ambition.combat.body_strike` is a damage box that follows its body for
    `lifetime_s`, in the body's own frame, with an optional hit sound
    (`strike::spawn_body_strike`). `install_module_technique` adds a module
    and declares its key in one statement, and panics at build time when
    the module does not answer that key. Smash's riposte is the module
    `ambition_demo_smash_modules::riposte` (workspace policy
    `game.ambition_demo_smash_modules-portable-closure`); the native
    `riposte.rs` is deleted. Its run-time check of a counter's answer is
    deleted too: a counter's answer is a nested effect
    (`NestedReferences::Effects`), checked at admission, so an answer in the
    wrong units is refused when the cast is prepared and not skipped in a
    match. Witnesses: `ambition_demo_smash riposte_tests` (the eight damage
    tests on the module road, and
    `the_cut_from_its_module_file_lands_as_the_linked_one_does`, its runner
    `Loaded`) and `ambition_characters nested_effects`. Poisons: the module
    not added, seven damage tests red; the adapter without the facing, red
    at "a left-facing riposte cut somebody standing to its right"; the cut
    one more in the wasm build only, red at "the loaded cut lands as the
    linked one does" (12 for 11); the nested loop skipped, both admission
    tests red. Cost: an edit to the cut builds the module file in 0.55 s
    (`scripts/build_extension_modules.sh ambition_demo_smash_modules`), and
    the Ambition app, whose polygons, Gnu-ton and Ramen answer with the
    riposte, loads it with no relink (`AMBITION_EXTENSION_MODULES`). The
    Smash demo app does not load module files: none of its fighters
    authors a special that is a module yet.
  - Special route F2 ✅ (2026-10-18): a special that leaves something on the
    stage. `ambition.world.spawn_module_entity` (v2) is offered in
    `technique_execution` too, and carries the box that bodies touch and the
    move's params, which the world keeps on the entity unchanged.
    `ambition.world.module_entity_tick` (v2) gives them back with the
    entity's `touchers` (the bodies of its live room whose box meets it, any
    side, in identity order), and `ambition.world.launch_toucher` throws one
    of them (the velocity set, not added). `ambition.feedback.effect` is
    offered in both phases. Smash's placed spring is the module
    `ambition_demo_smash_modules::spring`: its clocks and uses are the record
    `smash/spring.plate`, and a plate of this game is stamped with the
    running match (`stamp_smash_module_entities`), so the match's end sweeps
    it. The native `spring.rs` and its rollback component
    (`smash.placed_spring`) are deleted (schema 359). One rule changed: two
    fighters on one plate, the first in identity order is thrown (it was
    the lowest seat); both are rewind-stable. Found on the way: the host
    numbered a loaded file's modules by their place in the list the caller
    gave, so a caller that declared some of a file's modules ran another
    module's code for them (the spring's drop ran the riposte's and faulted
    "reach"). A backend now names the number of each module by key
    (`ModuleBackend::module_index`). Witnesses: `spring_tests` (the throws of
    one stand at the authored delays, 16, 29 and 42 ticks; the uses; set not
    add; the clock; one throw for two fighters; the cues and a silent plate;
    the match stamp; the loaded file, its runner `Loaded`). Poisons: the
    module number by list place, red at "the loaded plate throws as the
    linked one does"; a launch that adds, red at "the plate added to his
    speed"; no stamp, red at "the plate is not the match's".
  - Special route F3 ✅ (2026-10-20): a special that acts on the bodies
    around its fighter. The request `ambition.combat.sleep_area` puts every
    other body of the fighter's live room whose box meets an area (in the
    fighter's own frame) to sleep for at least the asked time; the sleep is a
    floor, so two pulses do not add up. Smash's song is the module
    `ambition_demo_smash_modules::sing`; the native `apply_authored_sleep` is
    deleted, and the game's mash rule (`mash_out_of_sleep`) stays in the demo
    and runs before the song is lowered. Admission now checks a song's params
    (`check_sleep_params`: a time more than 0 and an area that reaches
    somebody; it was "serde can build it"). One rule changed: a body of
    another live room is not caught (the native song read every body of the
    process). Witnesses: the seven sing tests on the module road
    (`ambition_demo_smash sing::tests`, among them
    `the_song_from_its_module_file_sleeps_as_the_linked_one_does`, its runner
    `Loaded`, and `the_song_does_not_reach_another_live_room`). Poisons, each
    red on its own line: no room check; the singer not excluded; a sleep that
    adds; a wasm-only drift of the time. Cost: an edit to the song builds the
    module file in 0.61 s, and the Ambition app, whose performer and Shadow
    Oni leader sing, loads it with no relink (`AMBITION_EXTENSION_MODULES`).
  - Special route F4 ✅ (2026-10-20, redone the same day on the status
    primitive below): a special that changes a body's clock. Smash's time
    dilation is the module `ambition_demo_smash_modules::dilation`, clock and
    all: it sets the slowed body's scale (`ambition.body.time_scale`), keeps
    the remainder in a record on that body, spends it each tick of its status
    `smash.dilated` on the world's step, and gives the body its own clock
    back at the end; a newer dilation replaces the scale and the remainder.
    Its spend entry runs before its dilation entry (`after`), so a dilation
    of N ticks slows N ticks, a replacing one included. The first version put
    the clock in the combat engine (`ambition.combat.dilate_time`,
    `TimeDilated`); both are deleted. Witnesses: the eight tests of
    `ambition_demo_smash dilation::tests` on the module road, among them
    `the_dilation_from_its_module_file_slows_as_the_linked_one_does` (runner
    `Loaded`), the Witch-Time join, the exact tick counts and
    `a_dilation_that_replaces_another_buys_exactly_its_own_ticks`. Poisons,
    each red at its own assertion: no restore at the end; the entries
    declared in the other order without the `after` edge (red at the
    replacement count; the same order with the edge is green); a wasm-only
    drift of the time.
  - Special route F5 ✅ (2026-10-20): a special that fills a meter. The
    request `ambition.resources.refill` adds an amount to the resource the
    request names (a resource id) on the body the invocation ran for, up to
    its cap; a body that holds no such resource gains nothing, and an amount
    below 0 or not finite is refused. Smash's move fill (`smash.fill_meter`)
    is the module `ambition_demo_smash_modules::limit`; the native
    `apply_authored_meter_fills` is deleted. The match's own sources (time,
    damage, blocks: `fill_limit_meters`) are the game's rule and stay.
    The Smash tests share one harness for a module special
    (`module_road::app_on`, linked or loaded). Witnesses: the limit tests
    on the module road, among them
    `a_fill_from_its_module_file_fills_as_the_linked_one_does` (runner
    `Loaded`), `a_negative_fill_from_a_module_is_refused` and
    `the_module_fills_the_resource_the_match_declares` (the module's spelling
    of `smash.limit` is the catalog's). Poisons, each red at its own
    assertion: no refusal of a negative fill; the first resource filled
    whatever its name (red at "filled a body's OTHER resource"); no fill; a
    wasm-only drift of the amount. Cost: an edit to the fill builds the module
    file in 0.55 s.
  - Special route F6 ✅ (2026-10-20): a counter stance. The request
    `ambition.combat.parry_window` holds the body's parry window open for at
    least a time (the longer of the guard's and the asked one, so a stance's
    re-arm never cuts it short), and with `absorbs_projectiles` its absorb
    too; it never raises the shield, and a window that is not finite or not
    more than 0 is refused. Smash's stance (`smash.counter`) is the module
    `ambition_demo_smash_modules::counter`, asked each tick the stance is
    live. What a caught parry answers with is the stance's authored data,
    which the same module answers since the hit-reaction primitive below
    (the engine answer this entry first moved into `ambition_combat` is
    deleted). `FighterSpecial::flag` reads a flag param. Witnesses:
    the counter tests of `ambition_demo_smash` (the riposte answer, the
    Witch-Time target, no answer outside the stance, a parry without a
    shield) and the module-road arms
    `the_stance_from_its_module_file_parries_as_the_linked_one_does` (runner
    `Loaded`), `a_short_re_arm_keeps_the_longer_window_and_absorbs_only_when_authored`
    and `a_window_with_no_end_is_refused`. Poisons, each red at its own
    assertion: a raised shield; the newest window in place of the longer; an
    absorb opened always; no refusal of an endless window; the answer always
    on the defender; a wasm-only drift of the window. Cost: an edit to the
    stance builds the module file in 0.54 s.
  - Module primitive: a body's statuses ✅ (2026-10-20). Jon, the same day:
    a game must be able to define its own moves without the engine carrying
    one mechanic per move, so a port offers a general verb or fact and the
    move's logic is its module's. A move that lives on a body for a while
    keeps its state in its module's records on the body and holds a status of
    its own kind there (`ambition.body.hold_status`; the body's
    `BodyStatuses`, rollback row `body.statuses`, schema 371). The world ticks
    each body that holds a bound kind (`ambition.body.status_tick`, in
    `technique_execution`, each tick with a gameplay step, in identity
    order), so the module spends its clock and decides the end; the requests
    of a tick apply in submission order, so a let-go and a later hold leave
    the status held. `ambition.body.time_scale` sets a body's clock scale.
    Witnesses: `ambition_combat extension::status::tests` (a fixture module
    counts its ticks in a record and lets go; a let-go and a hold on one
    step; a paused world ticks nothing; a scale of 0 is refused). Poisons,
    each red at its own assertion: ticks while paused; holds applied before
    let-gos; a scale of 0 accepted.
  - Module primitive: a body's foes ✅ (2026-10-20). `ambition.combat.foes`
    (an observation in `technique_execution`) gives the foes of the scope
    body: each other body of its live room that it may target (the
    targeting relation: a team outranks a faction, a neutral bystander is not
    a foe) and can hit (in play, alive, on the playable plane), in identity
    order. Who is a foe is the engine's answer, which no module restates; a
    cone, a nearest or a lock-on is the module's rule. Witnesses:
    `ambition_combat extension::targets::tests` (a teammate and a bystander
    are not foes; a body out of play is not; a body of another live room is
    not; the order is the identity order, with three foes so that it is
    neither the spawn order nor its reverse). Poisons, each red at its own
    assertion: no relation filter; no can-hit filter; no room filter; entity
    order in place of identity order. `ambition.motion.velocity` is now also
    offered in `technique_execution`.
  - Special route F7 ✅ (2026-10-20): the homing dash, on the primitives.
    Smash's `smash.homing_dash` is the module
    `ambition_demo_smash_modules::homing`: its clock, its committed direction
    and its speed are a record on the fighter under the status
    `smash.homing`; each tick it bends the heading toward the nearest foe
    inside the cone and the range (`ambition.combat.foes`, the first of
    equals by identity, as `assisted_fire_direction` breaks a tie) and sets
    the velocity. The tick that starts the dash carries it, and a dash
    pressed during a dash replaces it and lasts as long as a fresh one (the
    carry entry runs first, `after`). The native systems and the rollback
    row `smash.homing_dash` are deleted (schema 372). One rule changed: a
    body of another live room no longer attracts the dash (the native read
    every body of the process). Witnesses: the twelve tests of
    `ambition_demo_smash homing::tests` on the module road (the cone, the
    range, the clock, the committed direction, a KO'd fighter, a teammate, a
    bystander that does not outbid the foe, the loaded file carries as the
    linked module does, the first tick, the replacing dash) and the module's
    heading test. Poisons, each red at its own assertion: the last of equals
    in place of the first; no cone; the entries in the other order without
    `after` (with `after` that order is green); a wasm-only drift of the
    direction.
  - Module primitive: a reaction to a hit ✅ (2026-10-20). A phase,
    `hit_reaction` (`CombatSet::ContentFlavor`: after this tick's hits are
    resolved and while the messages that name them are still readable, which
    a reader one tick later is not on a rewind), where a module decides what
    a body does about how its hits came out. The first trigger is
    `ambition.combat.parried`: a parry the scope caught while a live window
    of its move sustains a bound effect (the first bound one, so an unbound
    effect in an earlier window hides nothing), with that effect's params.
    The request `ambition.combat.answer_parry` presses a technique on the
    defender or the attacker with the part of the stance's params at a path
    (`ParamValue::at`, in the syntax of the params' leaves: a params struct
    writes a nested `ParamValue` as a one-element sequence, so a shipped
    stance's are at `response_params[0]`); a parry the scope did not catch
    and a path the stance states nothing at are refused. The parries an
    answer may name are `TickParries` (derived, schema 373). Smash's counter
    answer is the entry `smash.counter_answer` of
    `ambition_demo_smash_modules::counter`, and
    `ambition_combat::counter` is deleted. Witnesses:
    `ambition_combat extension::reaction::tests` (the body and the params an
    answer names; startup, recovery, an unbound effect and a layered window;
    a poached and a stale parry; a misspelt path), the module's answer test,
    `the_answer_from_its_module_file_is_the_linked_one` (runner `Loaded`),
    and the joins `a_parry_answered_with_the_blade_cuts_the_attacker` and
    `a_witch_time_stance_slows_the_attacker_across_the_frame_boundary`, now
    on the module road with both modules. Poisons, each red at its own
    assertion: no defender check; no live-window filter; no bound filter;
    the answer always on the defender (in the adapter and in the module);
    the path ignored; an absent path pressed with no params; the module
    naming the unwrapped path; the loaded road linking the module.
  - Special route F8 ✅ (2026-10-20): the delayed mark, on four new
    primitives. `ambition.combat.struck` (a trigger in `hit_reaction`) says a
    strike whose volume authors an on-hit effect to the effect's module, on
    the body it struck, with the effect's params and the striker's credit (a
    `u64` that names a body durably: its simulation identity, hashed; `0` for
    a striker with none). `ambition.combat.blast` (a request in
    `technique_execution`) is a damage box centred on the scope body that
    takes no side (`HitSide::Environment`) and is credited to the body a
    credit names, or to a `CreditStandIn` that is no participant when that
    body has left (`ambition_match` stamps it with the running match);
    a credit of 0 and a box out of its ranges are refused.
    `ambition.body.status_clock` says how much of a status is left
    (`StatusClocks`, drawn by the body-clock view's
    `publish_status_clocks` while the body holds the status; letting go of
    the status removes it), and `ambition.body.in_play` says whether a body
    is out of play. `ambition.body.hold_status` is also offered in
    `hit_reaction`, and `install_module_technique` checks the road the
    offer's delivery names (`struck` for `OnHit`). Smash's `smash.mark_body`
    is the module `ambition_demo_smash_modules::mark`: the strike puts a
    record and the status `smash.marked` on the struck body, each tick
    spends the fuse and shows the clock, and the tick it runs out asks for
    the blast and lets go; a tick out of play retires it. The native systems
    and `BodyMark`, `SeatCreditStandIn` are deleted; the rows
    `smash.body_mark` and `smash.seat_credit_stand_in` leave and
    `body.status_clocks`, `combat.credit_stand_in` enter (schema 374). Two
    rules changed: the strike's own tick applies the mark (the native read
    the previous tick's on-hit messages, which a rewind clears, so a mark
    went off a tick later than authored and could diverge between peers),
    and a striker is credited by its identity, not its match seat, so a
    seated body with no identity leaves no mark and an unseated one with an
    identity does. The blast's name is `blast`. Witnesses: the thirteen
    tests of `ambition_demo_smash mark::tests` on the module road (the body
    it follows, one blast, the fuse, N ticks of fuse, an unbound effect, the
    refresh, the credit and its refresh, no identity, the retirement and its
    rewind, the clock, the stand-in, the loaded file);
    `ambition_combat extension::{blast, reaction, status}::tests`; the
    module's two tests; `ambition_match`'s stamp test; and app_it
    `the_director_leaves_a_note`. Poisons, each red at its own assertion,
    listed in the commit.
  - Special routes F9 and F10 ✅ (2026-10-20): the bomb and the remote
    mine, on module items. `ambition.world.spawn_module_item` (a request in
    `technique_execution`) is a module entity that is also a held item at
    rest: any body may pick it up and throw it, it stays its spawner's, and
    the entity is where the item is, whoever holds it (`follow_module_items`,
    before the entities are ticked); an unregistered item is refused.
    `ambition.world.item_state` (an observation in `module_entity_tick`) says
    whether the item is loose and the hardest it struck (the settle's speed
    or a body's). `ambition.world.owned_entities` (an observation in
    `technique_execution`) lists the module entities a body spawned, with
    their kind, place, age and a handle that credits them and that
    `ambition.world.end_owned_entity` ends them by (refused for an entity
    another body spawned). `ambition.combat.blast` is now v2 (the module
    states the centre) and is also offered in `module_entity_tick`, and
    `ambition.combat.credit` gives a scope its own credit. `ModuleEntity`
    gains its spawner and an age counted up as it ages (an hour's lifetime
    is too long for a step to register on as `lifetime - remaining`).
    Smash's `smash.drop_bomb` is `ambition_demo_smash_modules::bomb` (the
    fuse is a record on the bomb, burned each tick, and the bomb goes off on
    its fuse or a hard loose impact, owning its blast) and
    `smash.place_mine` is `ambition_demo_smash_modules::mine` (no mine out
    places one; an armed one, by age, goes off where it is and is ended; an
    arming one ignores the press; the mine owns its blast). `LiveBomb`,
    `PlacedMine` and their systems and rows are deleted (schema 375). Rules
    that changed: a bomb's first tick of fuse is the tick after its drop (the
    native burned on the drop's own tick), a mine is its placer's by
    identity rather than match seat, the numbers of a mine's blast are the
    press's that sets it off (the move that placed it), and a mine nobody
    sets off lives an hour (or to the end of its room). Witnesses: the nine
    bomb and nine mine tests of `ambition_demo_smash` on the module road
    (each with its loaded file), `ambition_abilities module_entity::tests`,
    the modules' tests, `ambition_combat extension::blast::tests`, and
    `smash_it::a_match_cleans_up_what_it_created`. Poisons listed in the
    commit.
  - Special route F11 ✅ (2026-10-20): the steered bolt, on a flown module
    entity. In `module_entity_tick`, `ambition.world.move_module_entity`
    puts the entity where its module says (refused for an item entity, which
    is where its item is, and for a point that is not finite);
    `ambition.world.spawner_body` gives the spawner's credit and its
    player's stick (the steering axis, live while a move roots the body);
    and `ambition.world.toucher_credits` names each toucher of the tick by
    its credit, so a module can tell its spawner from a rival. Smash's
    `smash.steered_bolt` is `ambition_demo_smash_modules::bolt`: the press
    puts a module entity of the kind `smash.bolt`, and the module keeps its
    heading, trail clock and clearance in a record, turns it toward the
    stick by at most the turn rate, hits the first rival it touches
    (credited to the caster, or to itself when the caster has left) and
    carries its caster home once clear of them. `SteeredBolt`, its systems
    and the row `smash.steered_bolt` are deleted (schema 376). Rules that
    changed: the bolt first moves on the tick after the press, and a tick's
    contact is read where the bolt was drawn the tick before (the native
    moved and then read contact in one system); rivals tie-break by
    identity rather than match seat; contact is the bodies' published boxes
    rather than their kinematic size. Witnesses: the ten bolt tests of
    `ambition_demo_smash` on the module road (with its loaded file), the
    module's tests, `ambition_abilities module_entity::tests`. Poisons
    listed in the commit.
  - Special route F12 ✅ (2026-10-20): the tether, on a ledge ahead. In
    `technique_execution`, `ambition.body.ledge_ahead` gives the first ledge
    along a body's facing in its own frame, by the ledge authority's own
    probe every 16 px within a host reach of 400 px (as
    `ambition.items.aim_cast` casts to a host reach and a module reads the
    distance), with distance `0` meaning the authority would catch the body
    where it is; `ambition.body.on_ground` says whether it stands; and
    `ambition.feedback.body_line` shows or clears the line it holds.
    Smash's `smash.tether_pull` is `ambition_demo_smash_modules::tether`: a
    throw from the air bites the first ledge within the move's reach and
    pulls on that tick; the status `smash.tethered` and a record reel the
    fighter each tick (`ambition.motion.velocity`), the last pull shortened
    onto the anchor, stopping her dead where the authority catches her and
    letting go untouched when the time runs out. `TetherReel`, its systems
    and the row `smash.tether_reel` are deleted (schema 377), and with them
    Smash's `motion.rs`, the commanded-velocity seam and its waiver in the
    velocity policy: no Smash move writes a velocity itself any more.
    Rule changes: the line is sampled every 16 px from the body rather than
    at `reach / ceil(reach / 16)`, so a ledge within a pixel or two of the
    reach can bite or miss differently. Witnesses: the ten tether tests of
    `ambition_demo_smash` on the module road (with its loaded file), the
    module's tests, `ambition_abilities extension::tests`, and
    `smash_it::the_tether_catches_a_ledge`. Poisons listed in the commit.
  - Special route F13 ✅ (2026-10-20): the portal recovery, on a held portal
    pair. `ambition.portal.held_pair` (v2) is offered in
    `technique_execution` as well as `boss_conduct`: a pair remembers the
    phase that holds it, and only that phase's adapter releases it; its
    slots are its module's, not its entry's, so the entry that opens a pair
    and the entry that keeps it name one pair; an end can be asked as deep
    as authored (`half_thickness`); and a pair asked with `closes_on_use`
    closes the first time a body comes out of it and stays closed while its
    slot is asked for. `ambition.fighter.aimed_stick` gives the stick a
    fighter's move latched. Smash's `smash.portal_pair` is
    `ambition_demo_smash_modules::portal`: a press opens a pair in the next
    slot (in place of any she has), and the status `smash.portals_open`
    keeps it for the move's lifetime. `MovePlacedPortal`, its two systems
    and the row `smash.move_placed_portal` are deleted (schema 378). Rules
    that changed: the pair's clock runs from the tick after it opens; a
    pair's channels are its link's (owner, module, slot) rather than the
    authored index offset by the match seat, and its colours are the
    authored index and its partner for every fighter; a pair goes with the
    fighter who holds it, as a held pair does. Witnesses: the seven portal
    tests of `ambition_demo_smash` on the module road (with its loaded
    file), the module's tests, `ambition_portal2d extension::tests`.
    Poisons listed in the commit.
  - Special route F14 ✅ (2026-10-20): the shark ride, on a summoned ride.
    In `technique_execution`, `ambition.world.summon_ride` summons a
    character where its asker is, on nobody's side, reserved for the asker
    to ride (the summon executor's one road, `Effect::Summon`), and the body
    holds a status from birth, so the module that summoned it ticks it (a
    summon may name statuses: `SummonSpec::holds`). The facts
    `ambition.body.ride` (awaiting a mount, or aboard and whether it lives),
    `ambition.body.saddle` (ridden, awaited), `ambition.body.presses` (the
    jump and burst edges), `ambition.body.tumbling` and `ambition.body.room`
    (its live room's size), and the requests `ambition.mount.dismount`,
    `ambition.body.hold_course` (the body steers a velocity in place of its
    brain, `HeldCourse`) and `ambition.world.dismiss` (a summoned body
    leaves) are `ambition_abilities::ride`. Smash's `smash.summon_ride` is
    `ambition_demo_smash_modules::shark`: the status `smash.riding` takes the
    rider off when the shark dies, a hit launches her, she leaves play, or
    she jumps or dodges; the status `smash.summoned_ride` on the shark sends
    it toward the nearer side of its room once nobody rides it and nobody is
    coming, and dismisses it at the end of the departure. Smash's six shark
    systems, `Departing` and the row `smash.departing_mount` are deleted
    (schema 379), and so is the last native fighter-special handler road in
    Smash (`install_special`). The shark's health, boarding radius and
    deadline are the move's params (`pirate_admiral.ron`), not Smash's rules
    file; its departure speed and time are the module's. Rules that changed:
    the genre rules hold for the ride the move made, not for any rider; a
    riderless shark sets off on the tick after its saddle empties and holds
    its course from the next, a tick later than before; "the nearer side" is
    of its room rather than of the stage's main platform (the same on every
    Smash stage: each platform is centred in a 640 px room); a rider who
    leaves play or whose shark dies may be put down a tick later (the rules
    read her in `technique_execution`, before this tick's hits). Witnesses:
    the thirteen `smash_ride` tests of `app_it`, the module's tests,
    `ambition_abilities ride::tests`. Poisons listed in the commit.
  - Snake Plant follow-up, verified 2026-10-18 against the tree:
    `run_snake_plants` asks each player of the plant's own live room
    (`geometry.room_of(player) == room`), witnessed by
    `mary_o_it::the_snake_plant_is_shy_of_each_player_of_its_room_and_of_no_other`.
    It lets go of its `PoseHold::Path` whenever it has no pot: a dead plant,
    a brain that is not the plant's and a pot that is gone take that one
    branch, and `a_dead_snake_plant_lets_go_of_its_own_hold_only` holds it
    (and that another holder's hold stays). Keep behavioral decisions
    distinct from contact consequences: the latter belong in the proposed
    body-contact phase.
    See [unified movement](unified-movement-kernel.md#pose-ownership-under-script-encounter-and-plant-control-new-customer).
  - Module clock contract: preserve elapsed-time and simulation-rate
    semantics rather than assuming that tick counts always represent
    60 Hz timing. Coordinate with
    [simulation tick-rate compatibility](simulation-tick-rate-compatibility.md).

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
A death keeps the record, as it keeps the durable facts (Q164; ruled 2026-10-19 that unbanked facts go back with their objects: DIALOGUE-CHECKPOINT in the queue).

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
| Developer road | `AMBITION_EXTENSION_MODULES` or `ExtensionModuleFiles`; runtime feature `wasm_modules`; `scripts/build_extension_modules.sh` | `a_module_rebuilt_as_wasm_replaces_the_linked_one_in_the_same_game` |
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
