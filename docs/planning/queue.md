# The queue — live execution order

This file is the current executable engineering queue. It is not a work log, a
review transcript or an archive. Git history keeps completed investigations.
Durable design and measurements belong in the linked owner document. Product
decisions belong in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

Each row states the current failure, the owner, the next action and the
acceptance. A row stays here only while an engineer can act on it. When a row
closes, delete it in the same change. Keep a receipt of 1 to 5 lines (what was
wrong, what fixed it, the guard, any standing prohibition) only when an open
row, a script or an inbound link depends on it. Receipts are at the end.

Re-measure a row before you implement it. Check a row's owner against who is
actually running before you wait on them. When a row converges, rewrite it
around its result. Do not append to it.

Scripts read this file:

- `scripts/check_blocking_set_names_every_gate.py` reads each `**Blocked by:**`
  field to the next blank line, and each `**Acceptance:**` clause.
- A heading that contains `✅`, `DONE` or `CLOSED` marks its row closed for
  that script, for `scripts/check_discharged_holds_are_rewritten.py` and for the
  two consolidation-ledger checks. The ledger holds work on `ID-PEER` by its
  row id, and a ledger test needs `A10` to stay a closed row.
- Other documents and source comments link to row headings. Do not rename a
  heading without fixing its inbound links (`rg -n "queue.md#" .`).

Measure the per-row mass with:

```sh
awk '/^### /{if(n)printf "%s %s\n", c, n; n=$2; c=0} {c++} END{printf "%s %s\n", c, n}' \
  docs/planning/queue.md | sort -rn | head
```

## Priority order (Jon, 2026-10-14, 48 h; reordered 2026-10-17)

Content packs, and changes to the game, made elegant and efficient and
possible without a recompile. **A restart is acceptable; a recompile is not**
(Jon, 2026-10-17): an edit is ranked by whether it needs a Cargo build, and
a live reload is a second priority. Work in this order:

1. [CONTENT-AS-DATA](#content-as-data--content-still-written-as-rust):
   move each content fact that is still Rust onto the pack's data, and
   delete the Rust copy. A value read once at start is enough. In order:
   (a) the shown text, which has no data file at all (each demo's banners,
   HUD labels, cards, launcher rows, select screen, dialogue hints): one
   strings schema for each pack. The engine side is
   `shown_text_schema::<K>` (`ambition_content_pack::shown_text`): the
   game declares its keys and their holes, and admission refuses a missing
   line, an unknown line and an unknown hole, each by key and file. Done
   2026-10-18 for each game: Smash, Mary-O, Sanic and Versus
   (`data/<game>_text.ron`) and Ambition's launcher row
   (`data/ambition_text.ron`); the host's start cover and the pause menu's own
   rows are the host's own pack (`ambition_app/assets/host/pack.ron`,
   `host_text`), so no host module reads a content crate for its words. A wording edit is 0.5-0.8 s to a test; it was
   a crate build (Smash 11.7 s, Mary-O 18.7 s, Sanic 6.8 s) and a relink.
   Left: the shell's own words (menus, settings, the engine's "Talk" and
   "HP"), one shell text file by Q68
   ([UI](engine/ui-localization-and-accessibility.md#localization-trigger)); (b) the census tail (item 2 there);
   (c) the procedural techniques still in `ambition_content` Rust, onto the
   module road (a relink and a restart now; 0.34 s on that road).
   Second priority, and only where a recompile does not depend on it: the
   live-reload remainder (a restated schema inside a live session, sprite
   publication inside a session). A live reload for the demo packs' tuning
   is done (2026-10-16).
2. Efficient content preparation: [TEST-LANES](#test-lanes--keep-required-test-lanes-executable)
   item 10, converged 2026-10-15: a publish with nothing to render is 54 s
   (was about 1 hour); a frame that changes size costs its render and 74 s
   of packing.
3. Pack elegance: admission diagnostics that name the file and the field
   (done 2026-10-15: `FacetSource::parse` gives the field path, ca3bd5d8f);
   [CPU-LADDER](#cpu-ladder--the-brain-owns-the-knobs-smash-owns-the-ladder)
   steps 2 and 3 (done 2026-10-16: a reaction spread broke the George L5
   resonance; Smash declares its ladder rows from its pack; the engine's
   level curve is one level-free default).
4. Each slice states its measured runtime and iteration cost.
5. Then the 2026-10-09 order below.

## Priority order before 2026-10-14 (Jon, 2026-10-09)

The risk now is improving components one at a time without exercising the
whole engine. Work in this order, and fix an authoritative-state defect found on
the way in its own slice:

1. **P0:** A4, BAG-RECORD-HORIZON and AUTHORITY-POLISH are receipts
   (BAG-RECORD-HORIZON's open line is Q161). C03 stops as a
   campaign after its family 7: a remaining session resource moves only when a
   two-session witness shows it leaks
   ([consolidation plan §3](consolidation/consolidation-plan.md#3-c03--consolidate-session-owned-state-and-reduce-reset-only-app-globals)).
2. [WORLD-ACCEPTANCE](#world-acceptance--one-headless-playthrough-of-the-persistent-world).
3. [SDK-GAME](#sdk-game--a-small-independent-game-on-the-supported-api): acceptance met 2026-10-09.
4. [TEST-LANES](#test-lanes--keep-required-test-lanes-executable).
5. [NAVIGATION](#navigation--a-character-reaches-an-item-in-another-room).

**Done (review of `1d668f4a..9c6318bc7`, 2026-10-11):** the five
[REVIEW-1011](#review-1011--five-boundaries-where-one-subsystem-reconstructs-anothers-fact)
rows.

Correctness interrupts from the 2026-10-14 review:
[REVIEW-1014](#review-1014---sprite-cache-dependencies-and-rest-row-precedence-from-the-2026-10-14-review).

Correctness interrupts from the 2026-10-13 review:
[REVIEW-1013](#review-1013---frame-and-acceptance-gaps-from-the-2026-10-13-review)
(all four items done; item 3 seen 2026-10-15).

Correctness interrupts from the 2026-10-12 review:
[REVIEW-1012](#review-1012---feedback-and-adapter-contracts-from-the-2026-10-12-review)
(all five items done 2026-10-13; the hands-on feel of GNU-ton waits for a
person with a pad).

New correctness interrupts from the 2026-10-11 integration review:
[REVIEW-1011-INTEGRATION](#review-1011-integration---real-engine-contracts-for-new-demo-behavior)
and [SHOT-END-REPLAY](#shot-end-replay--a-brain-that-saw-its-shot-end-replays-differently---done-2026-10-11) (done 2026-10-11).
Re-check against the live tree before acting: the I4 S1-S3 control-gate migration
landed after the reviewed `ed0d1ccd` snapshot. Do not reopen its solved work or
the five closed REVIEW-1011 rows.

Done 2026-10-09: [PUBLICATION-FAULT](#publication-fault--a-refused-later-room-must-not-leave-a-mixed-world---done-2026-10-09).
The two owner pages the 2026-10-08 review found stale (the fingerprint and the
second-seat join) were corrected against the tree on 2026-10-09.

Deferred, not queued: a 3D migration, the cube menu's second camera, crate
renames, a full monolith decomposition, online netplay, a plugin marketplace, a
universal planner and the Bevy 0.20 upgrade. Q157 is Jon's decision; build
neither side of it.

## P0 — architecture and correctness

### CONTENT-AS-DATA — content still written as Rust

**Owner:** [`engine/fast-iteration-implementation.md`](engine/fast-iteration-implementation.md)
and the pack of each game.

**Current state (2026-10-14):** a development build reads every pack source
off disk, and `pack.ron` is the one list of a pack's sources (an edit to it
compiles nothing: 0.71 s). Content written as Rust costs a host relink
(7.05 s in M0) and a restart; a data edit reached its test in 0.7 to 2.5 s
with no compile in each slice below. Each moved fact was held equal to the
deleted Rust by a temporary test, and its schema refuses a bad value with
the file and the field named.

Done (the commit messages hold the detail):

- Room rules of every game (`room_rules`: death replay, heavy line, landing
  lag share, combat block, hostile wake radius). Reload restates them.
- Smash's and the versus stage's match rules (`match_rules`).
- A game's own schemas: `content_pack! { schemas: [..] }` and
  `content::rules_schema`. Versus rounds, Sanic's and Mary-O's tuning
  (`sanic_rules`, `mary_o_rules`, with Mary-O's castle end and flag).
- Ambition's own schemas: the crate `ambition_content_schemas` and the
  binary `ambition_content_check`, which the package guard runs.
  `ambition_rules`: boss idle bark cadence, GNU-ton duel, African swallow.
  Reload restates it.
- Tables: dialogue voiceprints (`dialogue_voiceprints`, reload), projectile
  looks of Ambition and Mary-O (`projectile_visuals`, reload), held and
  wielded item art (`held_item_art`, `wielded_item_art`, reload).
- The item catalog: the pack's `items.ron` is the one item table
  (`ITEM_META` and `builtin_item_catalog` are deleted). A slot that the
  installed catalog does not name is no item: nothing is granted, saved or
  drawn. Mary-O, Sanic and Smash state no `items.ron`, so they have no
  items. The owned-items checksum keys a slot by its grid index.
- The effect sheets: each renderer target states its FX facts (cue family,
  residency, fade), the publish writes them into the sheet record (`fx`),
  and `fx::fx_sheets()` is the records that have them. `FX_SHEETS` is
  deleted: a new effect sheet is a publish, with no Rust edit.
- Smash's three stages (`smash_stages`, owned by the demo): the shared
  room, landing point and blast envelope, and each stage's blocks. The
  geometry constants are deleted, and the select screen's ids must each
  have a row (refused at admission).

Open:

1. ✅ 2026-10-16 **A live reload for the demo packs' tuning.** The deferral
   waited on priority 2, which converged on 2026-10-15. The Ambition road
   was not lifted: a demo reads its rules through `PACK.prepared()`, so the
   engine's `EmbeddedPack` now holds a published generation, and
   `content::live_reload::EmbeddedPackReloadPlugin` watches the pack's files,
   compiles an edit, and publishes it when no session is live and no route
   prepares one. A session reads one generation from its preparation to its
   end, so a rollback cannot read two. Each demo names the schemas it reads
   where it uses them (Smash `smash_rules`, `smash_stages`, `match_rules`;
   Mary-O `mary_o_rules`; Sanic `sanic_rules`); an edit to another schema is
   refused with "restart", and a name the pack does not state panics when the
   plugin is added. Witnesses in `live_reload.rs`, one for each arm (the
   session, the preparing route, the live list, the publication, the
   dedupe), each red under its poison at its own assertion; each demo's lane
   reddens with a typo in its list. Cost: a save is seen within 20 frames,
   and the compile is 7.7 ms (Smash), 6.3 ms (Mary-O), 4.2 ms (Sanic).
   The same day: a schema the game read once at start takes part when the
   game gives a `Restater` beside that read; each demo's `room_rules` does
   (`EmbeddedPackReloadPlugin::room_rules`, through `restate_room_rules`),
   witnessed by `the_room_rules_restater_states_the_scopes_rules_again` and
   `a_restated_schema_is_stated_again_from_the_published_generation`.
   Then the presentation tables a pack shares with other providers:
   `prop_sheets` (Mary-O, Sanic; their sheets are loaded again),
   `platform_looks` and `projectile_visuals` (Mary-O). A restater remembers
   the ids its pack owns and replaces only those
   (`EmbeddedPackReloadPlugin::owning_rows`); Ambition's reload and the
   demos share `restate_prop_sheet_rows`. Witness
   `a_pack_restates_only_its_own_rows_of_a_shared_catalog` (changed, added,
   dropped, and another provider's id refused; red with the owned ids not
   updated and with the previous ids ignored). The platform-look and
   projectile-look restaters have no witness of their own: each is one
   `restate` call on a catalog that Ambition's reload witnesses.
   Then each demo's audio (`EmbeddedPackReloadPlugin::audio`, through
   `ambition_audio::catalog::publish_provider_audio`): the provider's
   fragment in the catalog, the active selection, and the provider's own
   tracks in the library (`AudioLibrary::revise_provider_tracks`: another
   provider's track, and an unchanged track with the path the host resolved,
   are kept). Witnesses
   `an_edit_to_the_music_is_the_providers_music_from_the_next_session` (red
   with the catalog not published) and
   `a_provider_reload_revises_its_own_tracks_only` (red with a dropped track
   kept, and with an unchanged track resolved again). Ambition's own audio
   reload revised the whole track table through the asset catalog, which was
   built at start, so a moved Ambition track kept playing its old file
   (`a_reload_that_moves_a_tracks_file_plays_the_new_file`, red before the
   fix). It now publishes through the same `publish_revised_audio`; its only
   own step is the host's SFX registry copy. `revise_music_tracks` is
   deleted.
   Mary-O's ladder (`mary_o_forms`) is read where it is used, so it joined
   her live list (read, by a search for every reader; no reader at start).
   Her pickup art (`world_item_art`) has a restater
   (`EmbeddedPackReloadPlugin::world_item_art`, one lowering with her
   registration: `EmbeddedPack::world_item_art`), and the render layer now
   resolves the pickup art when its manifest changes, as it does for held
   art (`resolve_world_item_art_when_the_manifest_changes`; it was a
   `Startup` system). Witnesses
   `the_pickup_art_restater_restates_the_packs_entries` (red with the
   previous ids not withdrawn) and `a_changed_world_art_manifest_is_resolved_again`
   (red with the art resolved only when the manifest is added).
   Each demo's cast (`character_catalog`, `moveset`, `smash_fighter`):
   `PackCast::register` adds `EmbeddedPackReloadPlugin::cast`, which builds
   the catalog fragment again (`cast_fragment`) and replaces it in the
   registry, defines every character again through the registration's own
   road (`cast_definitions`), stages each, and admits and publishes one
   revision with its catalog (`take_admitted_revision_with_catalog`,
   `publish_admitted_revision`); a row the pack drops is retired, and a
   refused revision keeps the last good cast and catalog. Witness
   `an_edit_to_the_cast_is_played_by_the_next_sessions_cast` (a move edit
   compared with the saved file; a row added, then dropped); red with no
   activation, with nothing retired, and with the catalog not restated.
   With no restater it is red too, but at the watch lookup.
   With this, every schema of the three demo packs takes part, and each
   demo guards it: `every_schema_of_the_pack_reloads_between_sessions`
   builds the demo's experience and names each schema its watch does not
   publish (`EmbeddedPackWatches::unwatched`; red, naming the schema, with
   Smash's audio restater removed and with Mary-O's ladder off her list).
   Then publication into a live demo session, for a change to schemas the
   demo reads where it uses them: the one authority on mechanical mutation
   around rollback (`mechanical_mutation_boundary`) decides it. With no
   timeline it is published at once; with a timeline this host owns it is
   published and the timeline is rebased in the same step
   (`rollback::rebase_local_timeline_onto_published_content`, moved out of
   Ambition's reload, which calls it too); with a foreign or unhealthy one
   it waits for the session's end. A restated schema always waits for the
   session's end. Without the facade's `rollback` feature the crate cannot
   ask the timeline, so a live session keeps its pack. Witnesses
   `an_edit_to_a_live_schema_reaches_a_live_session_only_where_the_timeline_allows`
   (red with no timeline made to wait) and the restated arms (red with a
   restated schema published in a session). The foreign and unhealthy arms
   have no witness of their own here: they are the boundary's own answers,
   which its owner tests.
   Remainder: a restated schema inside a live session (the session's
   bodies were built from the cast; that is Ambition's I2/I3 road with a
   session handoff). Each
   needs a restater (the cast is a registry of prepared characters, so its
   restater is I2/I3 scale) or its reader moved to read at use.

2. **A census of content still written as Rust in the game crates
   (2026-10-16).** About 485 declarations in `game/ambition_demo_{smash,mary_o,sanic}/src`
   and `game/ambition_content/src` (not the WASM modules, not test modules);
   about 450 are structural (UI layout, render depth, ids, derived
   geometry). Done from it: the shown names of Mary-O's nine enemies and
   Sanic's badnik (her boss gauge read a Rust copy; `EmbeddedPack::display_name`),
   Sanic's scattered ring size (`ring_loss.ring_size`), his FINISH signs
   (derived from each course; the highway's stood at the speedway's floor
   height), `GOAL_X`/`LEVEL_WIDTH` (deleted; the course's own width), Smash's
   `SMASH_STAGE_TRACK` (deleted; the registry's `default_track`), and
   Sanic's stomp band (two Rust copies; now `stomp.band`, read by one
   predicate for the badnik's head and the monitor's lid).
   Smash's offered stages, their order and their button labels are
   `offered` and `label` in `smash_stages.ron`; `SmashStageChoice` is a
   choice from that list, so a new stage is a row with no Rust edit. A
   bomb's and the mine's blast knockback and the spring's arm and re-arm
   delays are fields of the move's params (`knockback`, `arm_s`, `rearm_s`),
   refused at admission when out of range. Smash's Limit fill is `limit`
   in `smash_rules.ron` (it was the engine preset `JONS_BASELINE`), and the sleep mash is
   `sleep_mash_seconds` there; the stocks button's counts are
   `stock_choices` there (`SmashStockChoice` is a count from that list).
   A thrown item's rebound is its `thrown_impacts` row's (`rebound`,
   `rebound_cap`). GNU-ton's duel book box and overhead scales are
   `gnu_ton_duel` rows (`book`, `overhead_rise`, `overhead_grow`). Sanic's
   super-form beat is `super_form` in `sanic_rules.ron`; Mary-O's step-up
   clock scale and cosmic arrive beat are in `mary_o_rules.ron`. Ambition's
   day (length, phase hours, start hour, daylight veil) is `day` in
   `ambition_rules.ron`, and a content reload restates it. Sanic's act
   titles and numbers are `acts` in `sanic_rules.ron`; his goal line's
   distance from each course's end, the outro brake and the goal post's
   height are `act.goal_from_end`, `act.outro_brake_per_second` and
   `act.goal_post_height` there (the goal line is witnessed through
   `goal_x_with`; the brake and the post are each one read of the row). Mary-O's castle
   rescue is a `cutscene_library` file in her pack (`data/cutscenes/castle.ron`),
   restated at a reload (`EmbeddedPackReloadPlugin::cutscenes`). The
   character Ambition starts the player as is `starting_character` in
   `ambition_rules.ron`, and the Noether attunement's signals and banner
   are `symmetry_attunement` there (the switches' test checks the level
   against them). A
   respawn and a departing shark read the centre of the body's live room
   (`StageCentre`), not the flat stage's. `smash_tool ladder-rig
   --scenarios` hangs a ledge fixture on the live stage's main platform (it
   refused every stage but Flat); one seed on Narrow ran to the end in 207 s.
   The speedway's floor and pit numbers are now `speedway_oracle`, read by
   tests only. The Smirking Behemoth's victory NPC (name, box, dialogue
   node) is `cut_rope_victory` in `ambition_rules.ron`; the Yarn file still
   writes the speaker's name in its lines. The spectator duel (room, centre,
   fighters and their grudges) is `duel` there: the arena's stager holds a
   copy of the row, and a content reload restates it
   (`RoomContentStagingRegistry::restate`), so the next staging of the room
   reads the edit. The versus stage's room (size, seating point, blast
   lines, blocks) is `versus_arena`, a schema the app owns, in the versus
   pack; its blocks and Smash's stage blocks are one engine shape,
   `AuthoredBlock` (Smash's own `StageBlock` is deleted); an edit reached its
   test in 0.82 s with no compile. The versus pack now reloads like a demo's:
   the stage reads its room, rounds and match rules where it uses them, and
   its room rules have a restater (`register_versus_cast`), so an edit is
   played from the next match; `every_schema_of_the_versus_pack_reloads_between_matches`
   names a schema that would cost a restart (red, naming `versus_arena`, with
   it off the list). Who the stage seats (the fighters in seat order, the
   CPU seats' brain profile, the team names) is `versus_seats` there
   (`FIGHTERS` and `VERSUS_CPU_BRAIN` deleted); witness
   `the_versus_seats_are_its_pack_file` (red with the teams written in Rust).
   The first fighter is also the stage's catalog default, read when the stage
   is composed. Left: Ambition's seat count (waits on Q153).
   Sanic's spin dash (the rev, the launch, the roll and the rev sound's
   tiers) is `spin_dash` in `sanic_rules.ron` (2026-10-17); its systems read
   the row where they use it, so a saved edit is played as a live schema,
   and `BallDashTuning::default()` and its resource are deleted. Admission
   refuses a ball with no size, a charge or a stick share outside 0 to 1,
   and tiers that step down. An edit reached its test in 0.95-1.59 s with no
   compile. The readers have a check by an edited file, not a standing
   test: with `launch_speed: 1650` saved, a system that kept a compiled 1600
   fails `releasing_one_rev_writes_its_launch_speed_into_v_t` (640 against
   660); with the shipped file it passes, so the check needs the edit.

   **The census of 2026-10-17** (610 declarations and about 160 literal
   sites in the five game crates, tests and the WASM modules left out;
   about 45 sites already read a pack and about 490 are structural or test
   oracles) left 25 findings. Done from it: Smash's seating jab, the swing a
   kitless fighter is seated with, is `seating_melee` in `smash_rules.ron`
   (it copied the duelists' catalog swipe, and its comment named another
   source); `smash_seating_melee` reads the row when a match seats, and
   admission refuses a swing with no time, negative damage or no reach. An
   edit reached its test in 0.82-1.28 s with no compile; the one seating
   call has no behaviour witness of its own. Mary-O's power-up rise is
   `power_ups.rise_seconds` in `mary_o_rules.ron` (each reward's rise reads
   it; a negative rise is refused; with 0.6 saved, a reader that kept a
   compiled 0.4 is red). How long a Smash blast is live (the bomb's, the
   mine's and the mark's) and the bolt's hit are `blast_s` and `hit_s` in
   each move's params (four Rust literals deleted); admission refuses a
   time that is never live, and the mark's seat-credit stand-in lasts the
   blast's own time and a margin (it was a 0.25 that had to stay above the
   0.08 by hand). Each reader's arm builds a time that is not the shipped
   one and reads it off the damage box (each red with its reader on a
   compiled time). Mary-O's sky zone rise is `level.sky_zone_rise_tiles` in
   her rules file; Ambition's quest payout banner time is
   `quest_payout.banner_seconds`, and the cut-rope trap (what hangs, in its
   order, the flattened banner and its time, the rope's spark rate) is
   `cut_rope_trap` in `ambition_rules.ron` (the anvil and piano enum is
   deleted; the dialogue asks for a name from that list). Done 2026-10-18:
   the dropped portal gun is `portal_gun_drop` in `ambition_rules.ron`
   (where it lies, its pickup box, its arm time; host-side policy, as the
   `portal-gun-seam` FIXME asks, and it goes with that seam); each demo's
   shown text is its `data/<game>_text.ron` (the shell's words are left);
   Smash's select cursor (speed, the ramp, the Back hold) is
   `select_cursor` and `back_hold_to_leave_seconds` in `smash_rules.ron`.
   The title screen's look (the drifting scene, the fade, the window, the
   cursor's breath) is `title_look` in the host's own pack
   (`ambition_app/assets/host`); Smash's select screen colours (each
   slot's, the extra hand, the inks and panels) are `select_look` in
   `smash_rules.ron`. Each edit reached its test in 0.7-0.9 s with no
   compile. Left: Twintrack's two seat colours, and the effect looks last
   (sparkle, dust, bursts, Mary-O's sky shimmer, the room palette).

Not here: Twintrack's plaza. It has no blocks; its size and origin
(`ROOM_WIDTH`, `LAB_POS`, 46 uses) are the frame its exhibit code is
written in, so they move with that code, not alone (decided 2026-10-14).
The boss and Mary-O modules' tuning constants. They are on the
module road (a 0.34 s WASM rebuild and a live reload).

**Done 2026-10-15:** Smash's respawn rise and protection (`smash_stages`),
its results-card wait and the admiral's shark (`smash_rules`, a new demo
schema), and Mary-O's level clock, lives and power-up motion
(`mary_o_rules`). Each data edit reached its test in 0.7 to 0.8 s with no
compile. Each demo that owns a schema (Smash, Mary-O, Sanic) has a check
binary, the content CLI with the demo's schemas
(`cargo run -p ambition_demo_<game> --features content_check --bin
<game>_content_check -- <pack>`): the generic CLI refused all three packs
(no installed capability owns the demo's schema), and Smash's `pack.ron`
told the author to run it. A quest's payout is its `reward` in
`quests.ron`, checked against the item catalog at admission (it was the
pirate treasure's Rust constant and a match on that quest's name). The
intro's flag chains are Ambition's `flag_chains` schema, and the thrown
items that hurt are `thrown_impacts` in `ambition_rules.ron`. The Smash
roster and stand-ins are `smash_rules.ron`. The player's motion-input
gestures (`qcf`, `qcf_grace`, `hcf`) are the pack's `motion_techniques.ron`
(a projectile-capability schema), restated by a content reload. The Sanic
start line's sign is a `DebugLabel` in `sanic_speedway.ldtk`, read at run
time. The prop sheets a room's props name (the Noodly Temple's keys and
doors, the Math town's fronts, the palm, the shark; Mary-O's construction;
Sanic's ring, monitors and goal post) are each game's `prop_sheets.ron` (a
`prop_sheets` schema owned by the sprite capability); Ambition's reload
restates its rows and loads their sheets again. The intro's props (the
cart, the creator lab's props, the cut-rope arena's props, the gate ring and
portal) are rows there too since 2026-10-16: a row names its record (`sheet`)
where the record is not its kind, and `packed: true` draws it from the shared
sprite pack first; `intro_prop_sprite_rows`, its asset ids and catalog
entries, and the `Catalog` prop source are deleted, and the pack reach
scripts read `packed` from the pack files. How a moving platform drawn as
a prop sheet is laid on its platform (the shark, the intro's cart and its
ramp, Mary-O's lift) is each game's `platform_looks.ron`, and a reload
restates Ambition's looks. Mary-O's pickup art (the wand, the beacon, the
quasar) is her `world_item_art.ron`. Her form ladder is her `mary_o_forms.ron` (schema `mary_o_forms`, owned by the
demo): the rungs, the star wand's and the cinder beacon's rows (the spark is
the beacon's ranged grant), and the cue of each change of form. The ladder is
the downgrade chain, so lowering builds each row's armor and slot; admission
refuses a bottom rung with a row, a rung over it with no row or arrival, a
spark the projectile road cannot fly, and a cue between characters off the
ladder. Mary-O's pickup boxes are `power_ups.*_half` in her rules file, and
Smash's respawn seats and platform are `respawn` in `smash_stages.ron` (a
seat spacing not wider than a standing body is refused). The gate portals
of Ambition's rooms (the intro's gate, the Math gate pair) are its
`gate_portals.ron`: a `gate_portals` schema of the world capability, whose
rows live in the world crate and which the engine's schema registry
registers (the world crate does not link the content pack); a reload
restates Ambition's rows, and `math_town.rs` is deleted. Each Yarn dialogue file is a `yarn_dialogue` source of Ambition's pack
(the `yarn_files!` list in Rust is deleted): a new file is a manifest line
(measured 1.0 s to its test, no compile), a build without a source tree
embeds it as every source, and admission refuses a node with no `===` and
a title two nodes use. Each node's text is its identity's canonical form,
so an edit to a spoken line is a new pack fingerprint. Ambition's world declaration (the room play starts in, and each LDtk
world file) is its `worlds.ron` (schema `world_manifest`, world
capability; rows in the world crate as for the gate portals). A new
dimension is a line there and no Rust edit; `build.rs` embeds each listed
file under `static_map`, and admission refuses an optional primary world, a
file that is not a `.ldtk` name, and a world stated twice. Not content:
the Mary-O test course. It is a fixture that no level renders, and its
module says why it stays Rust (an editor drag would make it a different
test).

**Next action:** the remainder of open item 1, one schema at a time: give a
reader that reads once at start a `Restater`, or move it to read at use.

**Blocked by:** nothing.

**Acceptance:** for each item, an edit to the data reaches the game with no
Cargo build (measured), a malformed value is refused at admission with the
file and field named, and the Rust copy is deleted.

### REVIEW-1014 - sprite cache dependencies and rest-row precedence from the 2026-10-14 review

**Owner:** the sprite publish (`scripts/regen/sprites.sh`) and
`ambition_platformer2d_actor_monolith::features::ecs::rest_row`. Review of
`80116a77b..d450b0220`.

1. ✅ 2026-10-14 **P1, the cache key missed a target's authored files.**
   The key guessed data by name (a rig at `rigged/<name>.rig.json`,
   `data/characters/<name>/`), so a target that reads another target's art
   (`gnu_ton_scholar_fighter` reads `gnu_ton_scholar_villain.svg`) kept its
   key when that file changed. Now `publish-many` renders each target in a
   new interpreter that records the files it opens and the directories it
   lists (the renderer's `read_record.py`), and the key hashes the record
   (`leaf_hash`; a target with no record is never fresh). Witness:
   `test_an_edit_to_shared_art_makes_each_target_that_read_it_stale_and_no_other`.
   Two more defects found by measuring it on the real publish: the key of
   the whole publish (`compute_fingerprint`) hashed only `.py`, `.yaml` and
   `.json`, so a comment in the villain SVG was a whole-run cache hit that
   rendered nothing; now it hashes every file of the renderer and every
   recorded read (three witnesses, one for each arm, each red under its
   poison). And the output-digest cache keyed diagnostics that the publish
   boundary then moves out, so `gnu_ton_boss` and `mockingbird_boss`
   re-rendered on every publish (`test_the_cache_keys_only_what_stays_after_the_publish_boundary`).
   Measured: one comment in `gnu_ton_scholar_villain.svg` re-renders the
   targets whose records list it (`gnu_ton_scholar_villain`, `_fighter`,
   `_boss`; the name guess had only the villain), render cost 394 s, wall
   1026 s; the uncached phases are the rest. A publish with no edit is a
   whole-run hit, 82 s.
2. ✅ **P2, the rest row wrote over a later pin.** `pin_rest_rows` re-pinned
   its row whenever its marker was present, so a conductor that pinned after
   it lost its row on the next tick. The rest row is now a fallback: the pin
   is the system's only while it holds the row the system wrote
   (`a_pin_made_after_the_rest_row_holds_until_it_is_released`, both writer
   orders and the release; red with the old condition put back).
   Follow-up review the same day: ownership was still read from the row
   name, so a conductor that pins the same row (GNU-ton's choreography pins
   `armed_rest`) lost its clock to the rest row. The pin now names its writer
   (`PinnedRow::pin_as`, `written_by`), and the marker on the body is deleted
   (`a_conductor_that_pins_the_same_row_keeps_its_own_clock`; red, on its
   clock and its loop, with ownership by row name put back). The same review
   found that a successful publish saved a fingerprint from before its
   renders rewrote their records; fixed (`fingerprint_after_publish`).
3. **Measure first:** the background warm-up of every rig table
   (`RiggedSpriteAsset::warm_baked_in_background`): peak and retained memory
   with and without it, cold and warm frame stalls, desktop and Android.
   Desktop, measured 2026-10-14 (`measure_the_memory_the_warm_up_keeps`,
   ignored; run by name with `--ignored --nocapture`; two runs agree): the
   166 published tables decode in 0.19 s on one thread and keep 22 MiB
   (resident 23 to 46 MiB). The slowest table is 12 ms
   (`imperfect_cellular_automaton`): without the warm-up, that is the stall
   of a first draw, most of a 16.7 ms frame. So the warm-up buys about 12 ms
   off the worst first draw for 22 MiB. Open: Android, and frame stalls
   measured in the running game.
4. **Acceptance by a person:** the robot's crouch, turn, double jump and
   back view in play; the new Hall and sandbox characters; the MSAA quality
   change on a real GPU.

**Blocked by:** nothing for 3. Item 4 needs a person at the game.

**Acceptance:** item 1 and 2 each have a witness that fails without the fix;
item 3 has a measurement in the owner page before any design change.

### REVIEW-1013 - frame and acceptance gaps from the 2026-10-13 review

Review of `4af5c66f..80116a77b` (2026-10-13). It retires the five
REVIEW-1012 items. Each item is its own commit with a witness, a poison
and a control.

1. **Done 2026-10-13:** the brain's stop and coast distances, and the
   ledge and overshoot facts that read them, are on the body's own side
   axis, not world x (`perception_body_for` → `ground_distances`;
   `WorldView::floor_ahead` and `side_sign_toward`; `walks_off`,
   `overshoots`, the corner width in `situation.rs`). Witnesses:
   `the_ground_distances_are_along_the_bodys_own_side_axis` and
   `the_ledge_and_overshoot_verdicts_are_the_same_in_every_gravity_frame`
   (the two ledge witnesses turned to sideways and reversed gravity).
   Five poisons (each world-x read put back) are red. Under screen-down
   gravity each changed expression is the same float operation. Open:
   `StageView::room_toward` asks the stage's world box along world x, so a
   cornered check under turned gravity is still a world-x question.
2. **Done 2026-10-13 (ToothbrushArtAgent):** the real Hall conversation
   puts the armor on Jeff Hinter by key presses
   (`the_armor_choice_of_his_hall_conversation_puts_the_armor_on_him`).
3. **Done 2026-10-15 (seen, ultra):** front idle ↔ walk: Mallory as the
   player in `central_hub_complex` faces the viewer at rest, turns to
   profile while she walks, and faces the viewer again when she stops
   (`AMBITION_QUALITY_PROFILE=ultra capture_scene central_hub_complex
   player OUT.png 960x540 --character npc_mallory --frames 6 --stride 30
   --player-holds right@150 --player-holds none@210`). A held item through
   an attack, both facings: the player robot's blade is behind it in the
   wind-up and raised in front in the swing, in the posed hand, and the
   left-facing swing is its mirror
   (`--player-holds attack@100 ... left@140 ... attack@160`, stride 3).
   Not seen: front idle ↔ combat.
4. **Done 2026-10-13:** the posed hand's root is never the feature's
   stand-in (`UnclaimedBodyPlaceholder`, which carries the feature's id in
   a transition and draws no parts): the root query excludes it.
   `the_root_of_a_feature_is_its_rigged_visual_and_not_its_stand_in`
   spawns the stand-in first; the filter removed, it is red at "the
   stand-in ... was taken for the root".

Found on the way, open for its author (the session of `047f37e92`,
`27a90dba3`, `d7e82dfaf`, `Claude-Session: …017PmHh7VwJMuFrWZkxix2gB`):
three repo tooling guards are red on main after the bubble shield and the
behemoth's beam: `test_foreign_system_ordering` (78 edges, ceiling 76;
81 at `4b8863b27`, so later commits add to it),
`test_multi_writer_resources_are_adjudicated` and
`test_resources_crossing_the_rewind_boundary`. Each judges the new
systems, so the author decides. The two list entries (the behemoth's
module arm, the `bubble-shield` road) are done.

### REVIEW-1012 - feedback and adapter contracts from the 2026-10-12 review

Review of `857ec24c..4af5c66f` (2026-10-12). Each item is its own commit
with a witness, a poison and a control.

1. **Done 2026-10-13 (`c1abe758d`):** a fighter learns whether its own attack hit from
   its body's record of that use of the move (`BrainSnapshot::own_move`,
   from `MovePlayback`), not from the foe seen in hitstun.
2. **Done 2026-10-13 (`ba4a799b1`):** a held portal pair takes the whole
   request each tick (place, normal, opening, look), and the pair of an
   entry that is no longer admitted is released. Open: a holder that crosses
   rooms keeps its pair in the room it was made in (the portal core's
   channel pair is per room; no customer crosses rooms with one).
3. **Done 2026-10-13 (`e901e04ec`):** a counter reads a swing in the foe's
   own frame (`PerceivedActor::gravity_down`).
4. **Done 2026-10-13 (`cde89753f`):** a contact bounce is against the
   toucher's own gravity; a toucher with no frame is refused.
5. **Done 2026-10-13:** `a_player_walks_in_takes_the_four_keys_and_beats_the_monster`
   plays the whole Noodly Temple by ordinary controls: from the central hub
   by the room graph, the scullery's fight of eight cooks won by Attack
   presses (`common::walk::fight`) for the Marinara Key, the cistern's
   wheels turned by Interact (a, b, a, c: the least turns `check_temple.py`
   prints) for the Pesto Key, the belfry's three bells for the Parmesan Key,
   the reliquary's three-wave vigil for the Colander Key, and the Flying
   Spaghetti Monster (48 HP, flying) fought until the save has it beaten
   (about 6100 frames). No teleport and no health written; a control before
   each room asserts its key or defeat is not yet set. The fight reaches
   from a foe's edge (from its centre it stood inside the monster and took
   its touch), jumps under a foe only within its
   measured apex and never during a leg, follows a Fly leg's toggle, and
   turns off a flight that no leg steers (`walk` too). Poison: Attack never
   pressed, red at "the fight ... was not over". GNU-ton's fighter timing
   is measured in the Smash match against his template (the patent clerk)
   and Alice (`gnu-ton-fighter.md`, "In the match, against his template"):
   faster than the clerk by 0 to 4 ticks of startup, a jab string that is +2
   on hit. The measure found his up tilt could not hit a foe of the robot's
   height beside him; its box now reaches that head
   (`each_ground_strike_of_gnu_ton_hits_a_short_foe_beside_him`, red before,
   the clerk's up tilt as the control). Open: feel by a hand on a pad in a
   window. The WORLD-ACCEPTANCE opener now states the Q164 remainder.

### REVIEW-1011-INTEGRATION - real engine contracts for new demo behavior

**Source:** the static review of `abf79de..ed0d1ccd` (2026-10-11). Each
finding was re-measured against the live tree and repaired at its owning
domain on 2026-10-12, with a failing witness first and a poison for each
arm; the evidence and the poisons are on the owner pages, and the
[source ledger](triage/2026-10-11-integration-source-audit.md) marks each
row. I4 S1-S3 (the pre-gate `actor_think` road) had landed before and is
not part of this row.

**Done 2026-10-12** (one line each; the owner page has the case):

- IN1 portal core: stable pairs and ends, a cap per room, `c128`+ refused,
  derived aperture ([portal identity](engine/portal-pair-identity-and-aperture.md)).
  GNU-ton: one link per scholar, owner-keyed join, `PortalLook`
  ([debt 12](game/gnu-ton-fight-debts.md#12-the-portal-and-trail-names-omit-the-encounter), interim).
- IN2 Mary-O players: the Snake Plant, snake shells, AI Slop squashes, and
  her walk/run, spark, form ladder, weaker-form refusal and quasar serve
  each player of the live room, not one global `PrimaryPlayer`; the plant
  holds its own pose and has its own out fraction (it never moved before)
  ([movement](engine/unified-movement-kernel.md#pose-ownership-under-script-encounter-and-plant-control-new-customer)).
- IN3: the pipe ride holds `ControlHold::Transit` and `PoseHold::Transit`
  ([control policy](engine/control-authority-and-ai-policy.md#scripted-movement-cannot-borrow-combat-recoil-as-a-control-lease));
  a camera has only the void field of what it draws
  ([R7](engine/render-animation-and-vfx.md#r7---full-screen-field-visibility-must-match-the-cameras-render-layers));
  a void visual has the aura of its own body, now
  ([R6](engine/render-animation-and-vfx.md#r6---reconcile-semantic-screen-field-sources-by-visual-ownership)).
- IN4: part asks by reader and the barrier refuses what it can not draw
  ([part reader](engine/semantic-part-rendering-and-ragdolls.md#a-part-set-demand-cannot-be-owned-by-one-effect-component));
  the cosmic form serves each body in it and shortens her own stop
  (contact hits never stop the world, measured;
  [combat](engine/combat-model.md#a-forms-shorter-global-hitstop-belongs-at-hit-resolution)).
- IN5: the GNU-ton arena opens on the recorded defeat
  ([debt 13](game/gnu-ton-fight-debts.md#13-the-exit-gate-reads-a-corpse-not-an-encounter-outcome)).
- IN6: far-side piece churn measured: remade each frame, no growth
  ([R8](engine/render-animation-and-vfx.md#r8---portal-fragment-lifetime-and-measured-render-churn)).
- IN7: a provider names the look of its launcher row (`with_launcher_look`),
  and the route-name prefixes are gone
  ([UI](engine/ui-localization-and-accessibility.md#launcher-theme-quality-and-provider-composition));
  the launcher backdrop follows the quality
  ([UI](engine/ui-localization-and-accessibility.md#launcher-theme-quality-and-provider-composition));
  a sprite key that two files give is refused
  ([reload](engine/content-generation-and-reload.md#sprite-source-identity-and-live-presentation-refresh)).
- Saved module rows are the schema written in them
  ([extension state](engine/extension-state-and-execution.md#saved-row-identity-validation-new-durable-session-customer-2026-10-11)).
- IN1, GNU-ton's pair is a port: the conductor asks for a held pair through
  `ambition.portal.held_pair`, the portal domain keeps it (rollback row
  `portal.held_pairs`), and `open_gnu_ton_portals` is gone
  ([debt 12](game/gnu-ton-fight-debts.md#12-the-portal-and-trail-names-omit-the-encounter)).
  Each fist carries its trail start (`LimbBondStart`, in its view); the
  renderer's `LimbTrailStarts` is gone
  ([debt 6](game/gnu-ton-fight-debts.md#6-the-trail-of-a-fist-joins-two-lines-by-a-resource)).
- IN1, GNU-ton's look lines are a port: `ambition.portal.light_line`; the
  portal domain keeps each body's lines as a view and the presentation
  draws them, and `light_gnu_ton_lines` is gone
  ([debt 2](game/gnu-ton-fight-debts.md#2-a-module-can-not-open-a-portal)).
  Each fist's trail start is `ambition.boss.limb_bond`, lowered by the boss
  domain, and `portals.rs` is gone
  ([debt 6](game/gnu-ton-fight-debts.md#6-the-trail-of-a-fist-joins-two-lines-by-a-resource)).

**Open:**

1. **IN1, the view index (P1, waits for its road).** The pair, the look
   lines and each fist's trail start are ports (done). The view index is
   keyed by `FeatureId` alone, so two live instances of one room would
   share a limb's view (debt 6). Measured 2026-10-12: `FeatureViewIndex`
   has 107 references outside tests, most in `ambition_render`, and no road
   of the game makes a definition live twice (REVIEW-1011 item 4: the
   witnesses make the second instance by hand). The key changes with the
   first road that does, and that road's two-instance witness is the
   failing fixture this item asks for. See [debt 12](game/gnu-ton-fight-debts.md#12-the-portal-and-trail-names-omit-the-encounter).
2. **IN6, frame time (P2).** Measure the portal-piece frame cost in a drawn
   host before any retained-piece strategy. Needs a machine with a GPU:
   the agent VM has none (checked 2026-10-12: no `/dev/dri`, and Vulkan
   lists no device), so its offscreen device is a software rasterizer and a
   frame time there measures the rasterizer. The drawn host exists
   (`OffscreenGpu`, as WA7 uses it), and `capture_scene --portal-shot`
   opens a pair in a real room.
3. **IN7, live sprite refresh (P2).** The launcher theme is the provider's
   (done). Since 2026-10-16 a running development build plays an edited sheet
   manifest, body rig, part flipbook (with its landmarks), portrait manifest
   or sprite pack catalog from the next session (with each cast made from them
   stated again, and the effect index rebuilt from the new records) and an
   edited page image at once; the watch looks when the operating system
   reports a change
   (`content::sprite_reload`; the road and what is not on it yet are in
   [the sprite reload owner](engine/content-generation-and-reload.md#sprite-source-identity-and-live-presentation-refresh)).

**Acceptance:** each open item is exposed by a failing engine-level
fixture before repair, and the fix changes the owning domain.

**Blocked by:** none.

### REVIEW-1011 — five boundaries where one subsystem reconstructs another's fact

**Source:** an external review of `1d668f4a..9c6318bc7` (Jon, 2026-10-11). Each
row starts with a failing production-path witness.

1. ✅ 2026-10-11, THROWN-CONTACT. `ItemStruckBody` carried only the speed, and
   the coconut rule found its victim where the item stood at the end of the
   tick. A coconut that crossed a body in one tick hurt nobody and still
   bounced. The item step's sweep names the bodies now
   (`ItemStruckBody::bodies`), and the rule hits those; `ThrownImpact::reach`
   is deleted. Witness:
   `a_coconut_that_crosses_the_player_in_one_tick_is_one_blow` (red with the
   old rule: health 60 -> 60), with the held-items sweep test naming the one
   body it crossed and `the_blow_lands_on_the_bodies_the_item_struck_and_no_other`.
   ✅ Same road, Jon 2026-10-11: a throw takes the stick's direction and
   strength, as in Smash (forward, back, up, down; a flick is a smash throw;
   the item keeps half the thrower's velocity). Witness:
   `the_player_throws_a_coconut_the_way_the_stick_points` (app) and
   `a_throw_goes_the_way_the_stick_points_and_a_flick_throws_harder`.
2. ✅ 2026-10-11, ABILITY-COST. Two admitted modules on one trigger saw one
   mana and cooldown snapshot, and movement was lowered before the cost, so
   the second action ran after its spend was refused. The host gives each
   invocation an id on every request it submits
   (`ambition_extension_host::InvocationId`) and a `Settle` step before
   `Lower`. `ambition.resources.spend_mana` and
   `ambition.abilities.movement_cooldown` are costs
   (`install_extension_cost`). `settle_wielded_costs` pays each invocation's
   costs together, in invocation order, or refuses the whole invocation
   (`ExtensionOutbox::refuse_invocation`). A cooldown armed this tick by an
   earlier invocation runs for a later one. Witness:
   `wielded_cost_admission_tests` (two `dive` entries with 30 mana: one lunge;
   two `blink` entries: one blink). Poisons: no refusal, red at
   `(280, 4, 2)`; no same-tick cooldown memory, red at 300 px. Not
   witnessed: a rewind across the settle (the settle keeps no state of its
   own; the bank and the cooldown are rollback state).
3. ✅ 2026-10-11, PORTAL-VISIBILITY. `PortalSourceHidden` recorded the
   portal's own hide, and the publisher read it as "visible but for the
   portal": a body that died behind a pane kept its candidate and its pieces.
   The marker keeps the change tick of the resolver's own `Hidden` write; a
   later write is another owner's answer
   (`PortalSourceHidden::portal_alone_hides`, also read by hit-flash). The
   publisher removes the candidate of a drawable that is not one this frame,
   and the compositor gives back the far-side reason of a body that is no
   longer a candidate, so the portal lets go when the pane goes. Witness:
   `a_body_that_dies_behind_a_pane_draws_no_pieces` (poisons: the old marker
   rule, and no candidate removal, both red at "pieces draw the dead body";
   without the give-back, red at the marker after the pane is gone). Not
   witnessed: the hit-flash overlay of a body that dies behind a pane.
4. ✅ 2026-10-11, ERRAND-INSTANCE. A cross-room errand routed by room
   definition and entered the least live instance, so a dog sent for an item
   in the second live instance of a room went into the first, and an item in
   another instance of the dog's own room read as `Gone`. `errand_crossing`
   keeps the item's live room: the last hop enters it, a route through a
   definition live twice is refused (no rule says which instance its door
   leads into), and a resident item no route reaches is `NoRoute`, not
   `Gone`. Witness: `a_dog_goes_into_the_live_room_its_item_lies_in` and
   `an_item_in_another_instance_of_the_dogs_room_has_no_route_and_is_not_gone`
   (red with the old crossing: instance 1 for 2, and `Gone`). Open: which live
   instance a door leads into when its definition is live twice (OW1); the
   witnesses make the second instance by hand, as no road of the game makes
   one yet.
5. ✅ 2026-10-11, PLATE-LIFETIME. `RoomPlates` kept its strong image handles
   when the quality budget turned off the system that evicts them (high ->
   Potato): 3 pages of up to 2048 x 2048 stayed loaded. Out of budget,
   `forget_plates_out_of_budget` drops them; back in budget they are drawn
   again off the main thread. Witness: the high -> Potato -> high arm of
   `a_room_look_undresses_when_its_budget_goes` counts the pages held
   (`room_look::plate_pages_held`; poison: the system unscheduled, red at 3).
   Not done, acceptance work per the review: profile terrain-skin setup in a
   large room before changing it.

### WORLD-ACCEPTANCE — one headless playthrough of the persistent world

**Owner:** [open-world roadmap](game/open-world-roadmap.md) (P4), with the
[residency plan](engine/open-world-runtime-and-residency.md) and
[item custody](engine/item-custody-and-accounting.md).

**Current state (2026-10-13):** the playthrough exists and is walked
(WA1-WA8 below): one run of the shipped game explores, takes the Blink,
carries Alice's note to Bob, opens the wall, dies, saves, reloads in a new
process and walks back, with a second participant, and in the composition
that draws it. What is left is the fact horizon of a death, which waits on
[Q164](awaiting-maintainer-decision.md). (Until WA2 each mechanism had only
its own arm, and no single run proved they compose.)

**Measured 2026-10-09, the content it can run on:** the intro world authors the
route as `intro_cartography_route` (`data/quests.ron`): Alice's sealed note in
`alice_relay`, Bob's field survey in `bob_relay`, then the first system boss and
the P5 route-memory pickup. The note has two roads: Alice's dialogue
(`dialogue/sandbox/intro.yarn`) puts a `sealednote` quantity in the bag and
Bob's takes it (`sell_item "sealednote" 0`, then `give_item "fieldsurvey"`), but
the quest steps read flags that separate `PickupSpawn`s (`flag:<name>`) set. So
the quest can advance with no note handed over, and no persistent object or
custody is exercised. The first slice gives the fact one road (the hand-over
the quest reads) and makes Bob's reward a world consequence rather than a
flag.

**WA1 done 2026-10-09:** the note has one road. The two flag `PickupSpawn`s
are deleted (`intro.ldtk` and the two room specs); Alice's choice writes
`alice_route_note_carried` as it gives the note, and Bob's hand-over is the one
writer of `bob_field_survey_received`. The content validator now counts a Yarn
`world.set_flag` as an authored flag (`dialogue::dialogue_set_flags`). Witness:
`the_note_travels_from_alice_to_bob` (real key presses in the shell session;
the old map makes it red at "the note's flag was set with no note handed
over"). Bob's reward is already a world consequence: the flag opens the two
`LockWall`s gated by it.

**WA2 done 2026-10-09:** `a_playthrough_of_the_persistent_world` runs the
shipped shell session from the hub to Alice through six real exits (doors by
Interact, edge exits by standing in them), the note to Bob, back to a wall
that is now open, out to another room, a save, a NEW process loaded from the
file (bag, flags, quest step), and the walk back to the open wall. Poison
(the survey flag cleared from the file): red at the after-load assertion.
Found: a death before any checkpoint takes the survey back and keeps the
facts it made, two horizons in one death; filed as
[Q164](awaiting-maintainer-decision.md).

**WA3 done 2026-10-09:** the playthrough first takes the Blink from the hub
basement. Attack moves the player 0 px before the take and 170 px after it,
and the one `ground_blink` is `Held` by the player at Alice and after the
load. Poisons: no take, red at the capability assertion (0 px and 0 px); a
stow before the route, red at "did not come along to Alice".

**WA4 done 2026-10-09:** a second participant.
`the_hand_over_opens_the_wall_in_the_other_participants_room` starts the shell
session with two pads, so it holds two seats (`declare_ambition_seating`).
Seat 1 joins at Alice and stays there while the player hands Bob the note in
`bob_relay`. With the two rooms live at once, Alice's return lock is open in
the second participant's room. Poison (gated walls re-evaluated only in the
primary's live room): red at the after-hand-over assertion, with the lock
still standing in `alice_relay`. Control: with no pads, the same Jump presses
give seat 1 no body in 120 frames, because the session opened one handle.

**WA5 done 2026-10-09:** the death step, the part Q164 does not decide.
`a_death_sends_the_blink_back_to_where_it_lay`: the player dies at Alice with
the Blink in hand and no checkpoint. After the death no Blink is in the live
world (not in the hand, not dropped there), back in the hub the one Blink lies
where it was found and can be taken again, and Alice's return lock agrees
with the survey flag. Poison (the restore keeps the dying player's held
objects as a spared room's): red at "a Blink is in the live world", the Blink
still `Held`. Next: the fact horizon of a death, when Q164 is ruled.

**WA6 done 2026-10-09 (review of f0409fcc):** the reload goes through the
file. It used to clone `AmbitionGameSaveData` into a new App in the same
process while the docs said "a new process loads the file". Now the first App's
autosave writes the save under its `PersistenceRoot`, the test reads the file
back and requires it to equal the live save, and a new App on that root loads
it at Startup (`load_save_at_startup`). Poison (the new App without the root):
red at "the new App's save is not the file the first App wrote". The words now
say what runs: one OS process. The route is walked since 2026-10-10 (WA8).

**WA7 done 2026-10-09:** the playthrough runs in the composition that draws
it. `the_playthrough_runs_in_the_composition_that_draws_it` builds the shipped
game with `OffscreenGpu` (a wgpu device, no window), waits for its plugins,
and plays the Blink, the route to Alice, the note, Bob and the way back. In
the end Alice's return lock is open. Poison (`NoWindow`): red at "this
composition has no wgpu device" in 5 s. Found on the way: Bob's published
sheet had no `air_dodge` row, which panicked neighbour-room preparation in
the drawn game until the sheets were published again; `sprites.sh --check`
now says so (`0d5c17f20`). The pixels of the open lock are `capture_scene`'s
question and are not asserted here.

**Next action:** the fact horizon of a death, when Q164 is ruled. Everything
else in the scenario has a witness (WA1-WA5), and the workspace lane runs it.
What the scenario does not exercise, measured 2026-10-09: the note and the
survey are `KeyItem` bag quantities (`items.ron`, no `held_item_id`), so they
are accounted by count and have no instance identity. The object with an
identity and a custody is the Blink. A note as an object is a content
decision, not a defect.

**WA8 done 2026-10-10: the playthrough is walked.** Each arm of
`a_playthrough_of_the_persistent_world` walks by keys in the shipped App
(`common::walk`): the in-room `NavGraph` for the player's own body, the input
`follow_leg` gives pressed as the arrows-and-ZXC keys, Interact at a door,
confirm on a cutscene's beats. The player walks to the Blink and through each
exit, back from Alice after a death too, and no step puts the body anywhere.
`a_walked_route_of_the_persistent_world` walks the hub to Bob through the sim
harness (an analog stick, the agent's input). Found and fixed on the way:
a leg through a hazard or another exit was kept (the rollout now fails on the
kernel's reset, and `build_avoiding` takes the exits); `drain_alley` had no
road down to the under-town door (the grate in the street is open); the graph
did not see a standing lock wall; the hub basement has no road up but flight
(a 512 px void; its pogo orb and moving platform are not in the graph), so
flight is a leg now, with starts beside block ends so that it goes up
through a grate. Measured, not changed: a body with one jump has no road from
Alice to Bob before the survey, and none out of the hub basement. Poisons: no
fly leg, red at the hub basement ("no route"); the graph over the authored
blocks only, red at `alice_relay`. `walked_route_census` (ignored) routes 9
of 9 crossings.

**Acceptance:** the scenario runs headless in a standing lane and is playable in
the rendered game; each step asserts its fact against the authority that owns
it; a poison at each step reddens that step's assertion.

### SDK-GAME — a small independent game on the supported API

**Owner:** [Public SDK 1.0](engine/public-sdk-1.0.md) and
[capability and runtime composition](engine/capability-and-runtime-composition.md)
(P3); release artifact per
[build and distribution](engine/project-build-and-distribution.md).

**State 2026-10-09: acceptance met (SG1–SG3 below).** What is left is the
deferred monolith carve: through the monolith, Outlander's build still links
menu, items, encounter, boss_encounter, cutscene, conversation, held_items and
audio, which its profile does not install. That is not a next action for this
priority.

**SG1 done 2026-10-09:** Outlander (`fixtures/external_consumer`) is the SDK
game: its own workspace and lockfile, the facade only, no `shared_tangle`,
monolith or Ambition content named in its source. Measured: its windowed build
linked `ambition_portal2d` and `ambition_platformer2d_ldtk`, which it never
selects, because the monolith's `visible` turned on `portal`, `portal_render`,
`portal_ldtk` and `ldtk_runtime`. They are out of `visible` now; a game that
uses them selects them (the Ambition app and sanic already did). Outlander's
windowed closure went from 61 to 59 ambition crates. New contract
`the-sdk-game-links-no-capability-it-omits` (`check_absence_contracts.py`)
resolves both builds and forbids the eight facade capabilities Outlander
omits, with a floor. Poison (the old `visible`): red, naming exactly
`visible: ambition_portal2d` and `visible: ambition_platformer2d_ldtk`. The
lane now compiles `outlander_visible` (`--features visible --bins`). Measured
on the way and not from this change (red on the uncut manifest too):
mary_o_app `her_spark_leaves_her_hand` (3 tests, the fire sheet's published
rows) and smash_app `nothing_a_match_created_survives_into_the_next_one`
under `--features visible`. Still linked through the monolith and not used by
Outlander: menu, items, encounter, boss_encounter, cutscene, conversation,
held_items, audio (the deferred monolith carve).

**SG2 done 2026-10-09:** the release artifact. `python3
scripts/package_outlander.py` builds `outlander_visible` (release,
`--features visible`) and writes `fixtures/external_consumer/dist/outlander/`
(binary, `run.sh`, `assets/`, `MANIFEST.json` with the engine revision and a
sha256 per file) and `outlander.tar.gz` (72.5 MB; 37 engine assets, 1.5 MB).
It then runs `run.sh --smoke 300` from the artifact under strace and refuses
it when an asset load failed or a file was read outside the artifact. Exit 2
is a missing prerequisite, 3 a failed build, 1 a failed artifact. The full lane
runs it (unrunnable without strace). Measured on the way:
`outlander_asset_root` baked `CARGO_MANIFEST_DIR` in, so it now follows the
engine's rule (`BEVY_ASSET_ROOT` → `assets`). Outlander's three windowed tests
were red and no lane compiled them (`#![cfg(feature = "visible")]`): they
stepped an app whose plugins were never finished, so the trail gizmo groups
were never registered. `stepped_windowed_app` finishes it, and the full lane
runs `cargo test --features visible`. The shrine sheet path was wrong in the
engine (180f89923). Poisons: an asset dropped from `package_assets.txt`, red
naming that file inside the artifact; a launcher without `BEVY_ASSET_ROOT`,
red naming the checkout files read. `--measure` writes the asset list again.

**SG3 done 2026-10-09:** reduced profiles through the shipped composition.
`PlatformerApp::profile(EngineProfile)` (profiles at `app::profile`) installs
`PlatformerEnginePlugins::for_profile`, or `RollbackProfilePlugin` under
`.rollback`. A module's `ModuleDraft::content_pack` is admitted at declaration
against `engine_schemas_without(profile.omitted_content_capabilities())`, so a
pack that requires an omitted capability is refused there, naming the profile.
Outlander composes `COMBAT_WITHOUT_INVENTORY_BOSS_DIALOGUE` in all three
builders. Witnesses: `a_reduced_profile_omits_what_it_names` (Outlander,
control: cutscenes stay installed; poison, the headless builder without
`.profile`: red at the omits assertion, naming inventory, held-use,
boss-encounters, dialogue) and the facade's
`a_profile_refuses_a_pack_that_needs_a_capability_it_omits` (poison, admission
against the full registry: red at the stage assertion). Measured on the way: a
windowed game without dialogue panicked on `ResMut<DialogState>`, because the
host's `dialog_pointer_input` ran unconditionally; it now runs only when the
dialogue state exists. Outlander's full suite under `--features visible` is
green (29 tests). The absence contract now prints cargo's error and the
`cargo fetch` remedy when the fixture's dependencies were never fetched.
The content CLI takes `--without <capability>` (repeatable) and checks a pack
against that reduced registry; witness `a_pack_is_refused_for_a_composition_without_its_capability`
(control: no flag admits; poison, the flag ignored: red at the exit code).

**Acceptance:** a dependency check shows the omitted capability crates absent
from the game's graph; its headless test and its windowed build both run in a
lane; a release artifact is produced by a documented command.

### TEST-LANES — keep required test lanes executable

**Owner:** test runner / app integration lane. Operational rules and what a
green lane does not clear:
[`running-the-heavy-app-it-lane.md`](../recipes/running-the-heavy-app-it-lane.md).
How a check can fail to run: [`checks-that-did-not-run.md`](../recipes/checks-that-did-not-run.md).

**Current state:** the `app_it` lane runs (re-run 2026-10-08; read the count
from a fresh run, not from here). Cargo diagnostics are read through
`scripts/lib/cargo_output.py`, which disables colour and strips ANSI codes, and
`scripts/tests/test_cargo_diagnostics_are_read_plain.py` holds every script that
reads cargo output to it.

**Sanic's visible job, 2026-10-17 (RED, inherited):** `cargo test -p
ambition_demo_sanic_app --features capture,input,perf_probe,visible`: 59
passed, 1 failed, `ov1_draws_the_world::the_start_line_sign_names_the_keys_his_actions_are_on`
("left: None": no `signage:` label with an id ending `start` exists after
600 frames of `build_windowed_demo_app(RenderMode::Headless)`). It fails the
same way at 98bd44559 (before P18) and with `serialize_main_schedules`
removed (before D12), so neither slice caused it. Not bisected. First
question: does the windowed demo still load the speedway, or does it now
stay at the launcher route, where no room is loaded?

**The repo tooling job (pytest), 2026-10-15:** 1984 passed, 9 failed. The
content-as-data slices had left 18 of its reds (two checks parsed Rust that
is now `held_item_art.ron`, the fx census parsed the deleted `FX_SHEETS`,
the gated-test count, a ledger row, a lockfile, a patch, unclassified pack-row
resources); those are repaired. The day-cycle reds were fixed at
c3eeb32cb (its session). Left, from the `capture_scene` and Hunny commits
(8c35d4b4b, 1b6dda77c): foreign ordering 81 edges (ceiling 76; capture_scene's
bubble shield, slot controls and shield rings, and the portal plugin), the
level-box read at `ambition_combat/src/extension.rs:90`, the multi-writer
verdicts for `ShieldRingsView` and `SlotControls`, and the rewind-boundary
census (`ShieldRingsView`).

**Re-run 2026-10-16:** 1865 passed, 4 failed. The demo live-reload slices
had added three reds (a `none-arm:` row for the reload's optional
`ActiveSessionScope` read, the feature-gated test count, and the audio
reload's writer that moved to `ambition_audio/src/catalog.rs`); those are
repaired, and so are four others' (the hall's cast prose, 158 placements;
the two sub-workspace lockfiles, after `ambition_portal2d` took
`ambition_time`; a severed sentence; four ambiguous citations). Left, none
from these slices: foreign ordering 84 edges (ceiling 76), the multi-writer
baseline (`capture_scene`'s `ShieldRingsView`, `PortalEffectSelection`,
`SlotControls`), the level-box reads at `ambition_combat/src/extension.rs:90`
and `ambition_platformer2d_actor_monolith/src/projectile/systems.rs:902`, and
the rewind-boundary census.

**Re-run 2026-10-16, later:** 1844 passed, 8 failed. Two were from the
CPU-LADDER and live-reload slices: the rollback codec shape (the brain's
checksum projection gained the reaction age, and the schema version had not
moved; now v352) and a multi-writer verdict that named the renamed rebase.
Both are repaired (3a0daf768). Two are new and not from these slices: an
unadjudicated test-side static (`crates/ambition_input/tests/leafwing_allocations.rs`
`ALLOCS`, b20c5fa86) and the presence-filter corpus, which no longer yields
`MovePlayback` (the boss encounter's `tick.rs` changed); its session was
told. The four above remain.

**Open items:**

1. **The compile-cost ratchet reports and does not fail the gate** (re-measured 2026-10-09: 8 findings, 0 gating, exit 0). Since `a614327fe` (2026-09-25, Jon: "line-count should not be a gate") only PATH and GONE gate; the size and seconds rows are reports. The critical path of 15 crates is banked (2026-10-09, `--adopt-wins`). The carve was `d56c46de9`, which removed `ambition_items`'s dead dependency on `ambition_combat`; the old 16-crate chain went through that edge. Two `--adopt-wins` defects found on the way are fixed: a held row kept its old `depth` (path 15 beside a row of depth 16), and `carried_from` dropped the older commit of a chain (`scripts/tests/test_compile_ratchet.py`, each arm red under its poison). Still open: the baseline disagrees with itself in three places (`ambition_geometry`, the monolith, `ambition_platformer2d_core`). The held numbers are from `11ef33c5b5a5`, and their table rows were already different, so no adopt can repair this; only a deliberate re-freeze can. Reported: the monolith's largest unit (100,742 → 132,359 lines). ⛔ Do not re-freeze with `--update` as bookkeeping: it banks the regressions, and that is a judgement to state in its own commit.
2. **An arm fails only in company** (see [the triage page](triage/a-composition-acceptance-that-only-fails-in-company.md)): `composes_through_the_sdk::a_host_that_omits_boss_encounters_still_builds_and_steps` failed once on 2026-09-10, and its assertion was never captured. Three other instances of the signature were per-arm measurements reading process-global state (`app_it` runs arms as threads of one process); they are fixed, and `scripts/a_test_static_is_a_channel_between_arms.py` guards the class. Next: capture this arm's assertion. `hall_redecode_census.rs` asserts over a delta of a process-wide counter, and it is not a candidate: it is `#[ignore]`d and run alone by `scripts/measure_hall_redecodes.sh` (read 2026-10-08). Counted 2026-10-09 from this machine's cost ledger (`run_tests_cost.jsonl`) since 2026-09-10: 56 job runs that run `app_it` in company passed and 35 failed, and no failed row says which arm failed: the evidence was only in a status file that the next run writes over. Measured 2026-10-13: that cost-ledger row was not written for a narrowed `--only-job` run, which is how a change's required checks run, so the lane ledger (`target/lane_ledger.jsonl`) held 25 failed `ambition_app` runs since 2026-10-09 and none said which arm failed. Now the lane ledger row, written for each job that runs, keeps a failed job's `failure_evidence`, and the cost ledger copy is deleted (`test_a_failed_job_keeps_its_evidence_in_the_lane_ledger_row`, through `run()`). A recurrence can be counted from that ledger. The A9 probe found and repaired two couplings that fail a composition without `BossEncounters` (`simulation_world` required `BossCatalog`; the progression plugin registered `populate_boss_encounter_registry`); whether either was this failure is not known. ⛔ Do not add a retry.
3. **The reload family failed under load, and its waits for the first activation now wait on the condition (2026-10-09).** The family is `an_edit_reaches_the_shipped_game`. Measured with the family alone at 30 test threads, five runs for each state: on main `0e2c22f9e`, five of five runs were red (two or three arms for each run, ten different arms, 13 failures), and each failure was a premise that the first session is live. With the waits changed, five of five runs were green. The cause: the gameplay route becomes active after work on other threads, so a fixed 240 frames is not a wait. Alone, the route was active on frame 3; in company it was seen on frame 83, and the red runs are the cases later than frame 240. The waits are in `game/ambition_app/tests/common/mod.rs`: `step_until_route_is_active`, `step_until_route_is_active_and_settled` and `ActivationWindow`. Each waits on the route under a ceiling of 120 seconds of wall time and then steps the frames the old loop stepped after frame 3. Two arms in that file hold the ceiling (`a_wait_for_a_route_that_never_activates_ends_at_its_ceiling` and the one for the window); with a ceiling check removed, its arm does not end. Converted 2026-10-09 (CalculexAmbition, `4cfe4e476` and the next commit): 16 candidate arms wait until `PendingGeneration` is gone and first assert that one was pending; two arms refused at request time assert that none is pending; two handoff waits wait on the activation id and on the handoff, under the same ceiling. Measured: a generation settled in 2-3 frames alone and at 30 test threads, so these arms were not short of frames; the change is that a negative arm now fails if the refusal was never decided. Left: the census probe, one handoff whose premise check fails loudly when its transaction has not run, three loops that assert on each frame, and each other family (63 loops of 240 frames outside this file). Done 2026-10-12, Smash: `wait_for_the_round_to_go_live` was vacuous with no seats ("none held" before the cast was built), so six fixtures waited a blind 240 frames for the 180-tick count, which a late activation can outlast. It now waits for seats with no opening hold, under the wall-clock ceiling, and the six waits use it (75 of 75 pass; the old condition reddens 13 at their "the cast exists" premises). Counted after: 63 `0..240` loops in the app tests, of which 5 are in the reload file and 5 are Smash loops that break on their own condition. No failure of those was seen in the ten runs after the change; that is not a proof that they hold. One older session-root handoff failure did not reproduce in four full runs, and its assertion was never captured. One of its two candidate arms (`the_shipped_app_never_holds_two_session_roots_across_a_handoff`) had this window; whether that was the failure is not known. ⛔ Do not add a retry, and do not make the 240 larger: wait on the condition.

4. **`NOT RUN` is a first-class receipt state** (Q59 ruling, 2026-10-03). A
   ledger or receipt must tell PASS, FAIL, "not run, not required now" and "not
   run, required at this boundary" apart; a gate blocks only where its policy
   requires it at the current boundary. Two collapses repaired (2026-10-08):
   `last_test_run.py` read a `--only-job` status as the lane's PASS (it now
   says `NOT RUN: n of the lane's m job(s)` and exits 2; a FAIL in the
   selection still exits 1), and a full gate on a machine without
   `wasm32-unknown-unknown` dropped the web check from the plan and wrote
   `done` (it is now planned as unrunnable, `Job.missing`, so the run is
   `incomplete`). The receipt for a push exists (2026-10-09):
   `scripts/required_checks.py` prints, for each check the change requires,
   `passed on this change`, `FAILED on this change`, `NOT RUN`, `NOT RUN:
   <remedy>` or `ran only on a tree before this change`. A push does not wait
   for it (Q166, ruled no 2026-10-09): it reports. Each boundary's lane is
   listed in
   [testing and validation](../concepts/testing-and-validation.md#validation-states-and-cadence)
   (2026-10-09), pointing at the owner of each rule. Next: the commit
   messages and queue rows that quote a lane by hand.

5. **A sync test did not see an effect that only the first run of a frame
   has; the rollback host now does (2026-10-05).** GGRS never saves the state
   that the first run of a frame leaves, so each of its compares is between
   two resimulations. `first_run_witness` (in the GGRS host crate) takes the
   checksum of the first-run state and compares it with the first save of
   that frame; `the_sync_test_sees_a_first_run_only_effect` holds it, with
   the measurement before the repair in its doc. The full app lane found no
   first-run-only effect in the tree. Named limits, not built: only a sync
   test with a check distance above zero; a peer session is not covered (GGRS
   saves its first run when no rollback is owed). The witness covers the last
   advance of a host tick only, and that limit is closed for the harnesses by
   measurement (2026-10-05, one mark for each host tick of a sync test that
   rewinds, over the full `app_it` lane and the four demo host binaries):
   23,372 ticks advanced one frame, 236 advanced none, none advanced two or
   more. A pinned host adds one frame for each update. The proof pulse of
   the rollback observatory (a developer affordance) runs a sync test under
   the real clock, and it is the one host where a long render frame advances
   twice; the first of those two advances has no witness.
6. **The demo host apps' own integration tests were in no standing lane;
   the recipe now names their lane and when it is required (2026-10-05).**
   `mary_o_it`, `sanic_it`, `smash_it` and `twintrack_it` run only under the
   whole-workspace lane, so a change that both agents' standing lanes
   (`app_it`, pytest) pass can leave them red. Found that way: four death and
   room-replay arms were red on main from `f7ecfc019` (P1) until `69d29caa0`:
   instruments counted the message a restore no longer writes. The behaviour
   held; only the counters were blind. The command, the changes that require
   it, and the measurement that it needs no waiver are in
   [the heavy lane recipe](../recipes/running-the-heavy-app-it-lane.md#the-demo-host-apps-have-their-own-lane);
   [the check matrix](../recipes/cheapest-sufficient-check.md#the-matrix) has
   the row. **Reported 2026-10-09:** `run_tests.py` records each finished job
   against the tree it tested (`target/lane_ledger.jsonl`), and
   `scripts/required_checks.py` names the checks the change since
   `origin/main` requires (a changed crate's own tests, the demo host lane for
   its paths, the repo tooling job for `scripts/`) and says for each one
   whether it passed on a tree with the change in it. It reports and does not
   refuse a push (Q166). Measured: the SG3 commit
   `1485939ec` changed the rollback host and was pushed without the demo
   lane; the gate names that lane for it. Witness
   `scripts/tests/test_required_checks.py` (poisons: freshness ignored, red at
   `test_a_pass_before_a_later_edit_is_old`; untracked files left out of the
   tested tree, red at `test_a_new_untracked_source_file_is_part_of_the_tested_tree`).
   A row holds the tree at both ends of its job, and a check counts when
   both hold the change's paths (`36eb555ee`): a file the host of a shared
   folder edited during a 14-job run had voided two passing jobs. A
   checked-out submodule is recorded at the commit it has checked out
   (`8ac789621`): the index entry had made a change that moved a pointer
   impossible to certify. Only the
   run that writes the default status records evidence, so a test's fake
   jobs do not. Not held: the demo rule's fourth case (an instrument a demo
   test reads), the external-consumer fixtures, the matrix rows without a
   path, and a peer's change merged after the run. Two more rows are held
   (2026-10-09): a change to `game/ambition_content/assets` or the map
   assets requires the content arms (`declared_art_resolves`,
   `registered_character_art`), and a rollback registration or
   `sim_phase_pins.rs` requires the `rollback_` arms and the repo tooling
   job. A check named by test name counts for a run whose filter is part of
   that name. Found on the way: a filtered nextest run certified the whole
   crate, because `run_tests.py` gives nextest its filter as a bare word and
   the rule looked for it after `--` (witness: the nextest rows of
   `test_only_a_default_feature_run_of_every_target_covers_a_package`).
   A doc (`.md`) requires the repo tooling job, whose guards read docs, and
   counts for no other check: a doc committed while a crate's lane ran
   voided that crate's pass (`738d7c3da`, witness
   `test_a_doc_edited_after_a_crate_ran_does_not_void_the_crate`).
   A change under `crates/` or `game/`, or to the root manifest, requires
   `cargo test -p ambition_workspace_policy` (2026-10-10): the policies read
   every crate, and three of them named a file the dive module deleted, red on
   main for a day while each lane the receipt asked for passed (witness
   `test_a_change_in_any_crate_requires_the_workspace_policies`; replayed on
   `7174d8015`, the receipt now names the policy check). A script under
   `scripts/` no longer voids a crate's pass: no Rust source reads one
   (`test_no_rust_source_reads_or_runs_a_script`).
   A member is named by its manifest's `[package] name` (2026-10-11,
   `scripts/lib/cargo_package.py`, in both `required_checks.py` and
   `run_tests.py -p`): the receipt asked for `cargo test -p
   extension_fixture_modules`, a package that does not exist (the package of
   `fixtures/extension_fixture_modules` is
   `ambition_extension_fixture_modules`), and the job failed at cargo's
   "did not match any packages" while every real check passed (witness
   `test_a_member_is_named_by_its_manifest_not_its_directory`, red with the
   directory rule back).

7. **The repo tooling job runs on several cores (2026-10-10).** It ran
   `scripts/tests` serially, and each push waits on it. Measured on 14 cores:
   1160 s serial; 232 s and 236 s with 8 workers, the same 1776 passed and 19
   skipped; 224-231 s through `run_tests.sh` with 14. `run_tests.py` adds
   `-n auto` when pytest-xdist is importable (`pytest_worker_args`), and
   `scripts/setup/python_tools.sh` installs it. The count is the `-j` cap, or
   at most 6 with none (`DEFAULT_JOB_CAP`: the shared machine's rule
   is at most 6 parallel jobs and no pool that takes every CPU by default).
   Since 2026-10-10 the same default caps cargo's build jobs and test threads
   when no `-j` and no exported value is given (cargo took every core before,
   and `required_checks.py --run` runs the lanes with no `-j`). The sprite
   publish pool, the audio-level pool and the worktree budget of main take at
   most 6 too (`test_no_pool_takes_more_than_six_jobs_by_default.py`).
   Without xdist the job runs serially and prints why: the same tests run.
   Witness `scripts/tests/test_the_repo_tooling_job_runs_on_every_core.py`
   (poisons: no workers, red at "the job runs serially"; no default cap, red
   at "with no -j the pool is not capped"). The pre-push measurement is in
   [extension evidence M0](engine/extension-iteration-evidence.md#m0-results):
   `app_it`'s 890-913 s is now the largest part of a push.

8. **The change selects its checks, and one command runs them (2026-10-10).**
   `python3 scripts/required_checks.py --run` runs the `run_tests.sh` command
   of each required check that is not certified, then judges again from the
   ledger the runs wrote. Witness
   `test_run_runs_only_the_checks_that_are_not_certified_and_judges_again`
   (control: all certified, nothing runs; poison: run every required check,
   red at the commands assertion).

9. **The union feature job is required by the two kinds of change that
   reddened it (2026-10-12).** `./run_tests.sh
   --run-everything-you-probably-dont-need-this --only-job 'every headless-safe
   feature'` runs the 528 tests behind an opt-in feature (the footer of
   `scripts/feature_gated_tests.py`). Measured 2026-10-12 from an empty target:
   3283 s, of which the tests are 1764 s (`app_it` 1248 s). Of its reds on
   2026-10-11 and 2026-10-12, three were rollback arms (the exit oracle twice,
   the schema parity once; one of them a probe that only the `portal`
   composition registers) and two were gated test files. `required_checks.py` now requires it for
   a path of `ROLLBACK_REGISTRATION_PATHS` and for a file that holds a gated
   test (89 of 2225 tracked `.rs` files), and its remedy asks for the
   exhaustive mode. Witnesses `test_a_registration_requires_the_union_feature_job`
   and `test_a_file_with_a_test_behind_an_opt_in_feature_requires_the_union_feature_job`
   (poisons: each rule, the gated `mod` arm, a default-on gate counted as
   hidden, the missing exhaustive flag; each red at its own assertion).

   A demo's own change is held too: a change to a demo's crates or its map
   assets requires its host app's tests behind the app's own features
   (`run_tests.py` plans `<app> — its tests behind its own features` when
   `-p` names the app; for most apps that is `visible`). The demos' HUD
   declarations and a 1-2 map edit were red only there on 2026-10-12.
   Witnesses `test_a_demo_change_requires_its_apps_tests_behind_its_own_features`
   and `test_each_demo_apps_own_feature_job_is_planned_for_its_package`
   (poisons: no rule, no map-assets arm, no `-p` in the remedy, not
   planned).

   A change to the feature wiring of the root or a member manifest (a
   feature, or a dependency's `features`, `optional` or `default-features`)
   requires it too (2026-10-12, `feature_wiring_changed`): the boundary table
   of [testing and validation](../concepts/testing-and-validation.md#validation-states-and-cadence)
   gives "a change to features" to the exhaustive run. Witness
   `test_a_change_to_feature_wiring_requires_the_union_feature_job` (four
   wiring arms, four controls: a version, a plain dependency, package
   metadata, a workspace version; poisons: no wiring check, red at the four
   arms; the version counted as wiring, red at the workspace control).

   A change in one crate that can redden another crate's gated test is
   named, not required (2026-10-12): the receipt prints the union job as
   `defer ... NOT RUN, not required at this boundary` with the crates the
   change reaches that hide tests (`deferred_union_job`, through the members'
   manifests, dev-dependencies included). It refuses the change only when
   the job failed on it, and `--run` does not run it. Measured on
   `68f527cc4` (the drop-through leg): the change reaches 28 of the 31 crates
   that hide tests, so requiring the 55-minute job here would require it for
   nearly each Rust push. Witnesses
   `test_a_change_whose_dependents_hide_tests_names_the_union_job_as_not_required`
   (control `test_a_change_with_no_dependent_that_hides_a_test_names_no_union_job`)
   and `test_run_does_not_run_a_check_this_boundary_does_not_require`
   (poisons: no deferral, dependents not followed, the deferral required,
   `--run` runs it, a failure that does not refuse; each red at its own
   assertion).

   **Run 2026-10-13 at `c7ae3da0c`:** the union job ran each gated test
   (2624 s, the tests 1872 s): all green but one Mary-O arm,
   `a_used_up_brick_in_a_skinned_room_wears_the_spent_plate_and_no_skin`,
   red at its premise (the brick of 1-3 has no skin). The same arm is the
   one red of `ambition_demo_mary_o_app`'s own-feature job (117 of 118); the
   Sanic, Smash and Twintrack own-feature jobs are green. The cause was
   this machine: the terrain skins are published art (git ignores
   `assets/room_dressing/`), and this clone had none. After
   `./scripts/regen/backgrounds.sh` (20 `starlight_*` skins) the arm is
   green. Its premise now names the missing file and the command
   (Mary-O's owner, `436b564f0`; seen to fire here before the publish).

The published-sheet floor in `ambition_sprite_sheet` (780 below a floor of 800
on one checkout) is machine state. ⛔ Do not lower the floor.

A test that needs a sheet row a commit just added is red on each machine until
that machine publishes the sheet again: the sheets are untracked. Measured
2026-10-09, the default gate at `19e44c13`: 14 of 16 jobs green. The workspace
job ran 9730 tests, 9728 passed, and the 2 that failed
(`her_spark_leaves_her_hand::*` in the Mary-O app) said "her fire sheet has no
`shoot` row" on a machine that had not regenerated `mary_o_v2_fire`. The other
red job was the wasm check (a native-only call, fixed by its owner). The same
day Bob's sheet had no `air_dodge` row, and the rendered game panicked in
neighbour-room preparation. `scripts/regen/sprites.sh --check` now says
whether the published sheets are older than the renderer (the fingerprint
saved by the last full publish) and prints the command to run (`0d5c17f20`).
It cannot tell which row a test needs; a red test still reads as a defect
until someone runs the check.

A full regen took more than 2.5 h on one process, and now takes 74 min on 7
(2026-10-09, `38865ba48`). `AMBITION_SPRITE_JOBS` defaults to half the cores,
at most 6 (8 when this was measured). A batch with one failure keeps the cache key of each target that
published. The run's own "render cost" line: 3392 s of the 4436 s in 7
renderer processes (tack-ons 1374 s, draw-review 893 s, draw-all 700 s, late
targets 287 s, factions 136 s). The other ~1044 s are stages outside the
renderer (ultrapack, quality variants, LDtk). The tack-on batch was at 143 of
144 after 1145 s, so its last target held about 230 s alone. Since renderer
`890c457` a batch starts the slowest known targets first (each target's last
publish seconds, kept in `.cache/publish_seconds.json`); its effect on a full
regen is not measured yet. Still serial: the faction-leader
lineup, the review canonical gallery, the ultrapack and the quality
variants. Also found in that regen:
- a run through the `/home/agent/code/ambition` alias wrote `relPath`s that
  climb to `/` into five worlds. Fixed in `rel_to_ldtk`, which now keeps both
  paths in one spelling (`test_a_repo_reached_through_a_symlinked_alias_keeps_the_virtual_mount`).
- the regen rewrites `mary_o.ldtk`'s editor-art tile ids: 22 rule tiles
  moved by +256 and 4 by +3, with their auto-layer tiles. These are editor
  visuals; why the atlas moved is not measured, and the change was not
  committed (Q62: a delta needs a domain-aware reading first).
10. **The sprite publish costs in proportion to what changed (converged
    2026-10-15).** It was a 1-hour publish that wrote the runtime roots in
    place, so a stopped run left a mixed set that app_it reported as a code
    defect (2026-10-13). Built, each with its witness in the commit:
    - A staging root swapped in only when every sprite phase passed
      (`scripts/lib/sprite_staging.sh`), made from the set the last swap
      replaced (`.cache/publish_mirror`, `rsync --delete`: 0.7 s, was 41 s).
    - A per-sheet key: the recorder, the files the registry says a
      target is made of, and the files its last render opened and the
      renderer modules it imported (`read_record.py`,
      `scripts/lib/sprite_cache_key.sh`). One process
      judges a batch (`scripts/lib/sprite_freshness.py`, 6 s for 240
      targets, the bytes of the bash key).
    - `draw-review`, `draw-all` and `draw-factions` deleted: their targets
      publish in the cached batch. The vanity card is keyed as a target.
    - Ultrapack: a tier is skipped when its inputs hold, patched when only
      pixels changed (6.7 s), and otherwise packed on threads within the
      job cap (the four tiers 82 s, was 362 s one after another).
    - The variants read only page fields (6 s, was 72 s) and build on the
      job cap's processes (a full rebuild 102 s in a publish, was 554 s on
      one; the same files); the LDtk
      manifest reads only the sheets it registers (1.7 s, was 25 s), for
      each world in `worlds.ron`.
    - A run with a cargo job names a stale sprite set and its remedy in a
      failed job's lane-ledger row (`environment`).
    **Measured 2026-10-15:** no change, 37 s (the whole-run hit); nothing
    to render, 54 s; a recolour of one sheet, its render plus about 7 s of
    packing; a frame that changes size, its render plus 74 s (every tier
    re-packs). A change to the shared core re-rendered every sheet: 56 min
    (tack-ons 2255 s of render, the variants 575 s before they built on
    six processes). Since 2026-10-15 a sheet's key holds the modules its
    render imported, not the whole core (68 files): ten sampled renders
    imported 15 to 69 of them, and 35 were imported by none. Each record
    gets its modules at its next render, so the first full publish after
    the change rendered every sheet once (44 min 20 s, 2026-10-16). Then,
    of the 68 core files, 21 make no sheet stale, 15 make at most 10, 5 at
    most 100, 14 more than 100 and 13 all 294 (`core/draw.py`,
    `registry/discovery.py`, `authoring/actor_contract.py` and the package
    markers among them); the mean is 107 sheets, was 294. Measured: a
    comment added to `authoring/faction_lineup.py` re-rendered 4 sheets, a
    publish of 3 min 37 s (was a full publish).
    The tiers pack only the runtime groups since 2026-10-15 (the full tier
    1.1 s, was 126 s; the same bytes), so a frame size change outside them
    re-packs nothing (the tier runs, finds no runtime sheet changed, and
    keeps its pages). A focused publish (`sprites.sh --target NAME`, 22 s
    for `mallory`: its render and its tiers) packs the runtime groups too,
    so the one prop that draws from a pack (`intro_cart`, `packed: true` in
    Ambition's `prop_sheets.ron`) shows its new art at once
    (`test_a_focused_publish_packs_the_runtime_groups.py`), and it writes
    through the staging root as a full publish does (8.3 s for
    `intro_cart`, 2026-10-16).

**Acceptance:** the failing population is reproducible or explicitly classified,
and the production cause is fixed or the harness proves why the failure is not a
production invariant.

### NAVIGATION — a character reaches an item in another room

**Owner:** [navigation and reachability](engine/platformer-navigation-and-reachability.md),
with the [agentic character runtime](engine/agentic-character-runtime.md) (P6).
Not built: enemy navigation and a baked graph (both Jon's open decisions).

**Acceptance met 2026-10-10.** A character sent for an item (`Errand`, a goal
given from outside) decides with its real movement whether it can reach it
(the in-room `NavGraph`: walk, hop, drop, double hop, wall climb, flight, each
kept only when its kernel rollout arrives; `RoomSet::route` over rooms), goes
there and takes it, in its room, in another live room (through a zone, by the
second-seat road) and in a room that is not live (through the ledger). Arms in
`a_dog_sent_for_an_item`: `a_dog_sent_for_the_blink_takes_it`,
`a_dog_sent_for_an_item_out_of_its_reach_says_there_is_no_route`,
`without_its_jump_the_dog_refuses_what_needs_one`,
`a_dog_sent_for_an_item_in_another_live_room_goes_through_the_door`,
`a_dog_sent_for_an_item_in_a_room_nobody_holds_leaves_for_it`, and a sync-test
arm for each road. Deterministic; no model call inside the simulation. The
slices, measurements and poisons are on the owner page.

Open, not in this row: permanent flight and a pogo bounce. The dash hop,
the wall jump and the drop through a one-way surface legs landed 2026-10-12, and a body on an errand waits at
a gate portal that is shut for the player (owner page). The errand already
went through an Interact door (`npc_door`).

### PUBLICATION-FAULT — a refused later room must not leave a mixed world — ✅ DONE 2026-10-09

**Owner:** [residency plan](engine/open-world-runtime-and-residency.md), the
multi-room hot reload row.

**Done 2026-10-09: a multi-room reload publishes all its rooms or none.** It
used to publish one room at a time, so a later room refused after the first
had published left a mixed world (the status said `THE WORLD IS MIXED`). Now
each room is held after its check (`replace_live_world_held`,
`HeldForOwner`): nothing is promoted and its effects wait on the
publication. Each later room is checked against the projected set and
counter. Then every room is committed (`commit_held_publication`) or every
room is refused by the refusal road (`refuse_held_publication`), so no undo
is needed. The design is in
[the residency plan](engine/open-world-runtime-and-residency.md#design-a-multi-room-publication-checks-every-room-then-commits-every-room).
Deleted: `republish_live_room`, the `THE WORLD IS MIXED` status and
`kept_the_old_generation`.

- Witness `a_later_room_refused_after_the_first_room_passed_rebuilds_none`.
  The fault is injected while the first room is built; each live room keeps
  its instance and generation, the epoch does not move, no receipt or
  candidate is left, and with the fault gone the reload applies.
- Poison (commit the first room before the later rooms are checked): red at
  "the first room was published although a later room was refused".
- Control: the rest of the reload family passed (48 passed), including
  `a_world_reload_rebuilds_every_live_room`.
- The guard before staging stays (`DescribableRooms`,
  `a_world_reload_with_a_live_room_that_cannot_be_described_rebuilds_none`):
  it refuses before anything is built.

Not done: an apply-time refusal after every room passed is logged
(`apply_world_replacement`), not failed loudly. With a correct projection it
cannot happen.

### SHOT-END-REPLAY — a brain that saw its shot end replays differently — ✅ DONE 2026-10-11

**Owner:** [simulation authority](engine/simulation-authority-and-determinism.md).

The shot's end was not the cause. `BodyMotionFacts` was declared derived
("republished every movement step"), but the brains read it in the decision
chain, before this tick's movement step. After a rewind a smash brain read
`running` from the abandoned timeline, so `melee_reach_of` gave the dash
attack's reach (54) on one run and the jab's (40) on the other; the mode
choice split (Engage against Approach at sim tick 38) with identical
kinematics, target, memory and combat state. Now `actor.body_motion_facts` is
a value-probed clone and the false derived declaration is deleted (schema
343 -> 344). Witness `app_it::a_custom_brain_module_replays_the_same_under_rollback`:
red before (GGRS mismatch at frame 38), green after. Commit d215fcbdf.

### AP14 — semantic actor-monolith SCC decomposition (continuous; deferred 2026-10-09)

**Owner:** [`actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md)
and [`actor-monolith-work-frontier.md`](engine/actor-monolith-work-frontier.md).
Jon's 2026-10-09 order defers a full decomposition: take a cut only when a slice
above already moves the state it names.

| # | Item | Next step |
|---|---|---|
| AP14 | Semantic actor-monolith SCC decomposition (continuous, Jon 2026-09-24) | Measured 2026-10-08: the SCC is 9 modules (`abilities avatar character_runtime construction features items projectile session world`); `avatar`, `character_runtime` and `construction` joined it after 2026-09-27. One leg was a spelling: 17 readers named `SessionCast` through a `session::mechanics` re-export of `ambition_characters::prepared::SessionCast`; they name the owner now and the re-export is deleted. The single-edge cuts left are genuine reads: `avatar→session` (1 ref, `RulesOf`, the per-room rules authority; removing it would leave 7), `construction→session` (1 ref, `CommitFactsSource::AfterTheRestore` names a checkpoint operation), `features→projectile` (1 ref, perception reads `ProjectileAllegiance`, which cannot sink below `ambition_combat`'s `MatchTeam`), `character_runtime→avatar` (1), `session→world` (3). The 2026-09-27 kernel was 6 modules; its legs were walked and judged genuine; the one misplaced leg, `abilities→features` (the puppy-slug gun asks the actor domain to spawn a minion), is not inverted only for the graph. Open question: is the runtime-mint description in `session→items` (`MintedItemBaseline`, `OwnedItemsBaseline`, `ItemCheckpointRestoreInputs`) item knowledge or occurrence-lifecycle knowledge? When an owner is clear, move state, behavior and installation together and delete the old edge (no callbacks, no compatibility re-exports). After each migration run `python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80`. See [`actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md) and [`actor-monolith-work-frontier.md`](engine/actor-monolith-work-frontier.md). |

**Acceptance:** each cut moves state, behaviour and installation together and
deletes the old edge; `python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80`
is run after it.

### I2/I3 — finish independent content authoring and safe reload

**Owner:** [`engine/extension-model.md`](engine/extension-model.md) and content
reload/preparation owners. Implementation state:
[I3](engine/fast-iteration-implementation.md#i3---coordinated-generation-publication-and-local-reload)
and [I4](engine/fast-iteration-implementation.md#i4---small-procedural-sdk-and-one-native-semantic-reference).

**Current state:** a prebuilt host loads edited content without Cargo. Reload
has explicit outcomes, a no-op revision does not advance the generation, and a
stale attempt carries the generation it was prepared against. A build that reads
content off disk watches the pack's sources (`ambition_content::content_watch`)
and reloads on save. Participating families include moves, boss profiles and
encounters, the character catalog, fighter facets, the boss seed library,
validator bands, items and audio registries. Content ownership is App-scoped
(I3 step 1): every install reads the App's `SelectedContentPack`, and the
process boot pack (`pack::shipped()`) is for inspection of the shipped product
only. The adaptive music-cue catalog (admitted with
`AdaptiveMusicCatalogRegistry::with_replaced`) and the cutscene library
(`publish_cutscene_library`, which replaces only the rows the selected pack
owns) do, since 2026-10-07, each with witnesses in
`an_edit_reaches_the_shipped_game.rs`; so does the quest book, as session-derived
state (it publishes nothing at the commit; `candidate_quest_book` admits or
refuses against the saved progress). The procedural tier (I4) runs
technique, boss-special and wielded-item modules on the linked and WASM roads,
with hot reload through the mechanical-edit protocol.

**Rulings (2026-09-19):** `Q110`: mechanical registry changes use explicit
lifecycle/replacement semantics; do not invent a universal silent overwrite.
`Q104`: content-authored movesets are the long-term authority, and duplicate
Rust move tables are migration scaffolding.

**Open work:**

- The three families that did not reload (music cues, cutscenes, quests) all take part since 2026-10-07. The references a candidate can break (a room naming a removed cutscene; a quest step naming a boss or room that does not exist) are judged at request time by the startup validator itself (`ContentGraphRefused`, 2026-10-07). Supersession of an in-flight generation is done (2026-10-07), and the quest book is asked again at the activation gate with the save as it is then, so a save that moves while a generation waits cancels it instead of being clamped.
- I4: save eligibility for body records and checkpoints. A session record `kept durable` is kept by the durable save since 2026-10-11 (`module_save_road`; see the I5 section). GNU-ton's conductor is a module (2026-10-11). Every held traversal item is a module (blink, dive, mark/recall and the grapple, 2026-10-10; the grapple reads the first wall along its aim through `ambition.items.aim_cast`).
- I4 `actor_think` S1-S3 are complete (pre-gate module brain, Void Lizard, Laden Swallow; tests in `app_it` and `mary_o_it`). The body-contact route exists (2026-10-12: phase `body_contact`, `ambition.body.contact`, `ambition.body.contact_bounce`, `ambition.body.defeat`), and the AI Slop and Laden Swallow squash is its first module (`ambition_demo_mary_o_modules::squash`). Next on it: the snake shells and the Sanic badnik (which needs the toucher's roll and its harm on contact, each owned elsewhere now; see I4). The plant is not a contact rule (a brain and a pose). Do not replace the completed brain road.

**Blocked by:** nothing.

**Acceptance:** changed content publishes exactly once under a new admitted
generation; identical content is a no-op; stale work refuses rather than folding
against a generation it did not read; no runtime road silently falls back to a
second authoring source.

### CONTENT-AUTHORITY-DUPLICATES — one authoritative reader per authored fact

**Owner:** [`engine/extension-model.md`](engine/extension-model.md) and
[`engine/participant-action-system.md`](engine/participant-action-system.md).

**Current state:** some authored facts have two readers:

- ✅ 2026-10-03: `boss_ron_target` no longer strips `_body`/`_hands` to map
  two files to one record; the key is the file stem. A probe on the strip
  fired 0 times over app_it (949 tests) and the content and boss lanes, with a
  positive control that fired. The `tools` generator still writes
  `gnu_ton_boss_{body,hands}` and `giant_gnu_{body,hands}` sheets into the
  published (gitignored) `gnu_ton_boss/` folder, and nothing loads them.
  The rows, the animation row and the hurtbox sample row of each boss attack
  are authored in `boss_art_keys.ron`; no other boss animation or sprite map
  was searched for after those moved.
- Yarn dialogue has its reader, and
  `game/ambition_content/src/content_validation.rs` checks dialogue references
  again. ✅ 2026-10-03, the `__` root fold: `known_dialogue_ids` also accepted
  the root of every `root__x` title. Four roots exist only as `__N` jump
  targets, so the validator accepted NpcSpawn ids the runtime cannot start
  (`a_spawn_naming_a_root_that_exists_only_as_variants_is_refused`). The ids
  are exact titles now. ✅ 2026-10-11, the catalog fold: `known_dialogue_ids`
  also accepted each catalog `hall_dialogue_id`, so a catalog that named a
  Hall dialogue with no node made the id valid, and the spawn started nothing.
  The Yarn titles are the one source now; the catalog's ids are checked
  against them (`a_hall_dialogue_the_catalog_names_with_no_node_is_refused`,
  red with the fold put back; the control is `hall_player`). ✅ 2026-10-15,
  the title scan: the ids came from `yarn_title_ids`, a `title:` line scan
  beside the pack's admission, which reads each node since the Yarn files
  became `yarn_dialogue` sources. Admission lowers each file's titles
  (`YarnFile::titles`), the validator and the Yarn tests read them, and the
  scan is deleted (the same 287 ids on the shipped files; poison: no titles
  lowered, red at `hub_guide` in `central_hub_main`). The Yarn compiler
  still compiles the files when the game loads them.
- ✅ 2026-10-15, the boss encounter files in tests. Four test fixtures kept
  their own `include_str!` list of Ambition's encounter files. A new boss
  (`wolfenstein_boss`) was in `pack.ron` and in none of them, so the boss
  fixture refused its profile (`MissingEncounter`) and 112 monolith tests
  failed. `ambition_boss_encounter::ambition_encounter_sources` (test
  support) reads the files that `pack.ron` states, and the four lists are
  deleted (poison: one manifest row left out, the same `MissingEncounter`).
- LDtk/world cross-reference rules in `content_validation.rs` repeat rules that
  a world owner already checks. ✅ 2026-10-03, the LoadingZone target rule:
  `validate_ldtk_room_links` refused every zone without both targets, which <!-- cite-ok: the function's name on 2026-10-03; it is `validate_room_links` now -->
  refused a landing pad that `LdtkProject::validate` allows and reported half a
  target twice. It now checks only that a complete target names a room and a
  zone that exist (`a_landing_pad_is_allowed_and_half_a_target_is_refused_once`).
  ✅ 2026-10-03, the rest of the LoadingZone rules, compared with their owners:
  - Measured first: the validator had its own scan of the `LoadingZone`
    fields, and it trimmed a target that the converter did not trim. With
    `target_room = "scroll_lab "` the validator reported nothing and the room
    set dropped the link: a dead door that validation accepted.
  - The LDtk owner has the one text rule now (`field_text`: trimmed, blank is
    absent). `LdtkProject::validate` and `collect_room_links` both use it.
  - A blank zone id and a second zone with one id in an area are the LDtk
    owner's errors now. `validate` gates each room set, so each LDtk game gets
    them, not only this validator's game.
  - `rooms::unresolved_links` is the one judge of "this link names a room and
    a zone that exist". `RoomSet::try_from_parts` warns of an unknown room and
    `layout_warnings` of an unknown zone, both from it. A room set cannot
    refuse such a link: measured, a partial set keeps the exits of its rooms
    (one room alone, and the engine's 59-room world, whose 4 links name rooms
    that only the full game adds).
  - The validator holds the complete game, so there each unresolved link is
    an error. It reads the rooms and links that the runtime builds
    (`to_room_parts`), and its own scan is deleted.
  ✅ 2026-10-03, the NpcSpawn ids, same class. The validator judged
  `character_id`, `brain_override` and `dialogue_id` trimmed, and
  `convert_npc_spawn` gave the runtime the raw values (measured:
  `"npc_ai_slop "`, `" guard "`). An unknown character is a body with no
  identity, and an unknown preset is a panic when the room loads. The
  converter reads the three with `field_text` now
  (`an_npc_spawn_carries_its_trimmed_ids`).
  ✅ 2026-10-04, the quest checks' boss and encounter ids. Measured on the
  shipped world: the validator slugged the display name of each `BossSpawn`,
  and for 9 of the 11 bosses that is not the id that the boss reports when it
  is defeated (`system_boss` against `clockwork_warden`, `t_rex` against
  `trex_boss`). A quest that named the reported id was refused, and a quest
  that named the slug was accepted and could not complete. The two shipped
  boss quests name the only two bosses for which the ids agree.
  - `ambition_boss_encounter::behavior::authored_boss_behavior` is the one
    resolution from a placement to its behaviour. The boss constructor, the
    room boss-art keys and the validator use it.
  - The boss looted flag is keyed by the placement id, as
    `ambition_boss_encounter::rewards` keys the chest. The validator derived
    it from the slug.
  - The encounter ids and their looted flags are those of the loader
    (`load_encounter_specs_from_rooms`), which builds only the first trigger
    of a room. The validator's scan named each trigger.
  - Witnesses: `a_quest_names_a_boss_by_the_id_its_defeat_reports`,
    `a_boss_looted_flag_is_keyed_by_its_placement`. `check_quest_steps` takes
    the quests as an argument, so a test can plant a step.
  ✅ 2026-10-04, the rest of the quest targets. The flags of NPC talk,
  switches and `flag:` pickups, and the ids of NPCs, pickups and rooms, are
  read from the composed rooms (`QuestTargets::of`), each made by the function
  the runtime makes it with (`npc_talk_dialogue_id`, `SwitchActivation::
  parse_custom`, `PickupKind::StoryFlag`). Measured on the shipped world before
  the change: the sets were equal (198 flags, 172 NPCs, 38 pickups, 72 rooms)
  but for one flag that the scan did not know, `npc_generic_npc_talked`, which
  a talk to an NPC with no dialogue sets. The validator has no scan of the
  LDtk entities for a quest target now; `authored_flag_ids`, <!-- cite-ok: records deleted functions -->
  `authored_npc_ids`, `authored_pickup_ids` and `authored_entity_iids` are
  deleted. <!-- cite-ok: records deleted functions -->
  ✅ 2026-10-08, `QuestStepCondition::ItemCollected` had no producer of its
  event, so the validator accepted a step on a pickup that could not advance.
  `collect_ecs_pickups` writes `ItemCollected(pickup id)`, the placement id
  the validator reads (`a_collected_pickup_reports_its_id_to_the_quests`).
  No shipped quest uses it.
  ✅ 2026-10-11, an area's metadata has one owner. Two levels of one active
  area that set one field to two values were refused only for
  `entry_cutscene`, by a per-room count in the content validator; the other 14
  fields merged first-wins with no message. `LdtkProject::validate` folds each
  area's levels in the composer's order and refuses each field that
  `RoomMetadata::conflicts` names
  (`two_levels_of_one_area_cannot_give_it_two_entry_cutscenes`, red with the
  `entry_cutscene` arm removed; the control is one value on two levels). The
  validator's per-room count and its `active_area_ids` room check, which held
  by construction, are deleted; it keeps the missing-script check. Shipped
  worlds: 0 conflicts.
  ✅ 2026-10-07, `scripts/check_world_graph_is_navigable.py` read the zone
  targets untrimmed, so it was stricter than the engine (a `"vault "` target
  resolves in the game and was reported as dangling). It reads them with the
  engine's `field_text` rule now (`test_a_door_target_is_read_with_the_engines_text_rule`,
  red before). It is still a second reader of the target fields, and it owns the
  trap analysis the Rust side does not have; the id of a zone is read as the
  engine reads it, untrimmed.

**Open work:**

- Remove each duplicate. Do not wrap Yarn in a schema unless that removes an
  authority. Do not move worlds into a content pack only for uniformity.
- External-capability witness: one capability outside the actor monolith uses a
  provider schema, a provider semantic action with a real device binding
  (`ProviderBindings`), and a causal fact, through public APIs only. It needs no
  new central enum variant and no private reader.
  State 2026-10-07: `examples/capability_demo` does all three (a registered
  schema, a `ProviderBindings` action that returns as `SemanticActionPressed`,
  causal facts), and its rollback test compiles and fails when the cooldown's
  registration is removed.
  **Gate C5 is read (review ruling, 2026-10-07):** an extension may name the
  narrow public crates it extends (`ambition_content_pack`, `ambition_causal`,
  `ambition_input`, `ambition_platformer2d_core`); it is NOT forced through the
  umbrella facade, because [`public-sdk-1.0.md`](engine/public-sdk-1.0.md) says the
  facade is not the dependency boundary for independent builders. What it may
  not name is engine-internal topology. The demo named one such crate,
  `ambition_platformer2d_shared_tangle`, for two items (`SimScheduleExt` and a
  phase set); those moved to a narrow crate, `ambition_sim_schedule` (depends only
  on `bevy`; `shared_tangle::schedule` re-exports them, so no engine path moved),
  and `scripts/tests/test_capability_demo_names_no_engine_topology_crate.py`
  keeps the demo off the tangle crate. **The demo is now the external-capability
  witness**: its normal closure is eight narrow crates. The phase set's topology name was dropped in the same sitting
  (`Platformer2dSimulationPhaseMonolith` is now `Platformer2dSimulationPhase`; 378 uses, <!-- cite-ok: records the old name -->
  114 files, compile-verified; the 0019 ADR keeps the old name as a record).

**Blocked by:** nothing. The external-capability witness is
`examples/capability_demo` (Gate C5 read above).

**Acceptance:** each fact has one authoritative read; diagnostics name the
authored source; the old reader is deleted.

### A9 — establish truthful minimal engine profiles

**Owner:** public SDK/composition architecture.

**Current state:** capability-footprint and absence-contract tooling measure
what a profile links and installs. The three composition probes in
`composes_through_the_sdk` pin the fixed step and assert that `FixedUpdate` ran.
A profile witness must assert that it stepped, not only that it built.

**Ruling (2026-09-19, `Q100`, `Q106`, `Q108`, `Q97`, with `Q146` and `Q144`):**
capabilities are optional and composable. A capability that authored production
content requires and the composition lacks must refuse that content or its
admission. Reduced tools and tests may omit capabilities explicitly. The ruling
says: implement this architecture rather than continuing to census hypothetical
composition variants.

**Done 2026-10-08 (first set):** five named profiles are a registry
(`ambition_platformer2d_runtime::profile`): `headless-body-world`,
`windowed-body-world`, `combat-without-inventory-boss-dialogue`,
`collection-without-held-use`, `encounters-without-named-bosses`. Each constructs
and steps a real body, installs none of what it omits, and has its session-edge
parameters validated; a control arm proves the probe can say yes
(`ambition_platformer2d_host/tests/supported_profiles.rs`,
`scripts/check_engine_profiles.py`). The probe found eight couplings that made a
promised omission fail on its first tick; all are repaired in source, and the table
is in [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md#supported-profiles).

**Done 2026-10-08 (admission):** a profile admits content with
`ambition_engine_schemas::engine_schemas_without(profile.omitted_content_capabilities())`;
`Capability::content_capability` names the pack capability each runtime
capability interprets (`BossEncounters` → `boss_pattern`, `Cutscenes` →
`cutscene`). A pack whose manifest requires an omitted one is refused with
`MissingCapability`; the full schemas admit the same pack. Witness:
`supported_profiles.rs` `a_profile_refuses_content_that_needs_a_capability_it_omits`
(poison: the omission filter off gives no refusals).

**Open:** the claim is *not installed*, not *not linked* (`Q106`: the crates behind
these capabilities are unconditional dependencies); `Dialogue` has no pack
capability (Yarn is not a pack schema), and a module has no road to declare a
Yarn script (`ModuleDraft` declares experiences, capabilities, rollback,
actions and content packs), so there is nothing to refuse until one exists.
The shipped facade admits under a profile since SG3: `PlatformerApp::profile`
admits each module's pack against the profile's schemas
(`a_profile_refuses_a_pack_that_needs_a_capability_it_omits`), and Outlander
composes `COMBAT_WITHOUT_INVENTORY_BOSS_DIALOGUE`, with no pack of its own;
`world-without-cutscenes` (2026-10-08) omits `Cutscenes` and steps; a pack
that requires `cutscene` is refused under it. Re-entry is witnessed for
the headless profiles (`a_second_session_in_one_process_steps_as_the_first`:
a second session in one process puts the body at the same position bit for
bit; poison: a process-global spawn offset, red).

**Blocked by:** nothing.

**Acceptance:** each supported profile constructs and steps a real subject; its
promised absent capabilities are absent from installation and resolved dependency
closure; the full Ambition composition remains intact.

### A7 — make item occurrence authority explicit where it carries a real invariant

**Owner:** [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md).

**Current state:** `GroundItem` construction is sealed, but that seal does not own
occurrence creation. Death drops, match spawns and other roads still mint
identity/provenance/custody facts at their occurrence sites. Two mint sites
degrade on purpose and belong to the `ItemCustody` inventory leg:
`ambition_held_items`'s thrown-item mint and `puppy_slug_gun`'s minion mint.

**Next implementation:** centralize only the occurrence decisions that share an
actual invariant (identity, custody, provenance, rollback ownership). Do not add a
generic request bus merely to reduce writer count.

**Acceptance:** reward policy consumes accepted occurrence outcomes and cannot
become an alternative minting authority; every remaining occurrence creator has
an explicit ownership reason.

### BRAIN — finish truthful fighter attack selection

**Owner:** [`engine/fighter-brain.md`](engine/fighter-brain.md), which holds the
modeling rules and the evaluation-rig contract.

**Current state:** the option layer prices what a move can do from one combat
model:

- A move's hittable region (`MoveFrameData::coverage`) and its push region (`push_coverage`) are separate. A pure shove is priced by `Features::displacement_value` (push coverage × the foe's proximity to a blast line × whether the push sends them that way).
- Admission is absolute: the opponent must lie inside the move's own region, with `ADMISSION_SLACK_PX` on each side. Admission leads the foe over `Perceived::staleness_s()` plus startup. Scoring stays on the observed position, so `reaction_ms` stays the difficulty axis.
- The brain and the hit resolver spend one launch law, `ambition_entity_catalog::launch::launch_speed`. A hazard is a value (`MoveHazard`, `ThreatTravel` in `ambition_entity_catalog/src/hazard.rs`), and an unresolvable ranged request is no offer.
- The shipped ladder's weights still order the rungs at least as well as any single-weight change tried (2026-09-21 refit sweep), so do not refit them.

Instrument: the `#[ignore]`d sweep `every_fighter_on_the_grid_can_fight_its_mirror`
(21 mirror matches of 3600 ticks; `AMBITION_GRID_TRACE` for one fighter). Run it
before and after a change, on one binary, and report both columns.

**Open roads, in order:**

1. **Done 2026-10-12: a brain declines a move it whiffed.** It remembers its
   own last press (`FighterState::own_attack`, by binding) and watches for the
   foe in hitstun for the move's length plus its perception delay; a whiff
   refuses that binding for as long as the move plays (`refused`), and the
   next attack of the ranking answers. Both are rollback state (projected in
   the brain snapshot). Witness
   `a_brain_that_whiffed_declines_that_move_for_as_long_as_it_plays`
   (control: a foe seen reeling; poisons: no refusal, no hit seen). The grid,
   before and after on one binary (23 mirrors, 3600 ticks): damage dealt
   5363 → 6440, distinct moves 224 → 253; Bob 3 → 19 moves (12/31% → 172/149%),
   Carl Stargan 6 → 17, the pirate admiral's `air_up`×101 lock is gone; the
   robot (17 → 12), the director (10 → 7) and the officer (15 → 11) use fewer.
   Unchanged: the medic (its one move lands), the goblin, the patent clerk and
   GNU-ton (`third_law`×160 at point blank, 0%).
   **Done 2026-10-12: a move's startup is its first live hit.** GNU-ton's
   lock was not road 2: `third_law`'s counter stance is an Active window from
   0.0 s, and `frame_data` read its sweep (0.18 s) as a frame-0 strike. A
   window is live when it has a volume, a capture or a sustained hazard
   (`frame_data_startup_is_the_first_live_hit_not_a_counters_stance`;
   poison: every Active window live). Grid, one binary each side, 23
   mirrors: dealt 5577 → 6296, moves 260 → 290; only the four kits with a
   counter before a strike moved (GNU-ton 0% → 74%/74%, 2 → 12 moves; the
   patent clerk 11/11% → 198/173%, 2 → 19).
2. **A buff cannot be chosen between.** Pricing it needs its own feature, not a wider admission rule. **Done 2026-10-12, the counter:** `counter_value` is 1 when the foe is committed to a swing that faces me, reaches me and goes live in the stance, and only a pure counter is priced so (a stance before a strike is priced as the strike: priced as both, the patent clerk's mirror pressed `synchronize_clocks` 160 times for 0%). Weight 0.8, the middle of the band on the Officer's real kit (0.49 against his gust, 1.17 against an uppercut that lands first). Witnesses `a_counter_is_chosen_against_a_swing_that_meets_its_stance` (five poisons, each red at its own arm) and `only_a_pure_counter_has_its_stance_priced`. The grid is identical in all 23 rows: a mirror never holds a swing that outranges the fighter, so the stance is chosen only between two fighters; no cross-fighter measure exists yet.
3. **`sanic` @5 leaves the stage early** (about 640–780 ticks, both seats airborne). The guard for it is a bout that ends early, not move variety. **Measured 2026-10-13: not a stage defect, the L2 edge margin.** In the mirror (`AMBITION_GRID_ONLY=sanic AMBITION_DUEL_RUNG=5`, positions every 10 ticks) the two seats stand 36 px apart at tick 366, each presses `spin_charge` (no impulse; its windows keep `motion_scale` 1.0, so the stick steers) and each runs outward at up to 571 px/s, off its own edge of the 480 px stage; after the respawn they do it again (ticks 576–650). The brake measured on the way out is about 850 px/s², so from 571 px/s he needs about 190 px to stop. `options.rs::walks_off` penalizes a retreat only when less than one body width (about 32 px) of floor is left, which is right for a walker and six times short for him. No rung rolls the future (`rollout_depth: 0` on all, road 7), so nothing vetoes it. Fix shape: the margin is one body width plus the body's stopping distance at its current speed along that side, a present fact perception gives (`SelfView`), from the body's own law. That needs the braking of each law where it lives (the responsive axis law stops at `ground_friction` hands-off and `run_accel` against; the axis law's momentum variant; `MomentumParams::brake`/`friction` of the surface-momentum model), and none of those defaults is 850, so first find which law Sanic runs in the match. Witness: a fixture with a fast body near the edge (retreat penalized; control: the same body at rest), and this row's mirror no longer leaving early. **Speed term landed 2026-10-13, and it is not what decides this bout.** `MotionModel::ground_stop_distance` (each law's strongest ground braking; the responsive law brakes at `run_accel + ground_friction` hands off, measured, since it applies both) reaches the brain as `SelfView::stop_distance`, and `walks_off` adds it when the body moves toward that edge (kernel tests per law in `movement/tests/stopping.rs` and `surface_momentum/tests.rs`; witness `a_body_coasting_toward_the_edge_does_not_choose_to_retreat_off_it`, two poisons each caught by their own assertion). The mirror's `[grid-sweep]` line is unchanged to the digit, and a probe on `ActorControl` shows why: the trace's `x`/`vx` are the brain's DELAYED view (rung 5, 300 ms, about 20 ticks). The brain saw x=342 vx=347 and pressed against the motion while the body was at x=488 vx=600 with 72 px of floor (it needs about 100). Before that, the two seats were 36 px apart at the `spin_charge` press. The stick stayed live through the move, Approach held full stick toward the foe, and each ran THROUGH the other to 600 px/s, unseen for 20 ticks. **(a) landed 2026-10-13:** `options.rs::overshoots` penalizes Approach and Dash when the body, on the ground and moving toward the foe, cannot stop before it passes the foe's far side (its `stop_distance`); Retreat brakes there. Witness `an_approach_that_runs_through_the_foe_loses_to_the_brake` (controls at rest and moving away, and a walker on a stale view; each poison red at its own arm). ⛔ The first version (26366474f) also added the body's own travel over `staleness_s`: 81 px for a walker at rung 5, so two George mirrors in the Smash demo backed away within about 84 px and threw no move (`smash_it`: four `the_repertoire_gets_used` arms and `the_fighter_brain_engages_rather_than_standing_still` red on main until the lead was removed the same day; a fixture with a live CPU, `the_trap_holds_her_under`, was also red and now seats a passive opponent). The grid numbers taken with the lead are void. Grid at 88d5569da (speed term, arrive rule on the stop distance, rooted slide), one binary, 23 mirrors at the default rung, against the baseline before road 3 (deterministic): 21 rows moved, by cascades of small decision changes; damage taken 6703 → 6820, distinct moves 334 → 341; the medic's one-move lock is broken (1 → 18 moves); the largest falls are Carl Stargan 143/157 → 46/86, the ninja 289/327 → 122/137 and Alice 206/175 → 120/132, not attributed. **At rung 5 Sanic still deals 0%, and the next link is measured:** the rule fires (the brain retreats at vx −458), then it starts `sonic_boom` at about 530 px/s, a grounded move with `roots_steering: true`; a rooted momentum body has no stick, so it coasts at `friction` 400 px/s² (a 355 px slide) to the edge. (c) priced 2026-10-13: a move that roots its owner (`MoveFrameData::roots_steering`) adds the body's hands-off slide (`SelfView::coast_distance`, from `MotionModel::ground_coast_distance`, held to each law's integrator) to its `stage_risk` travel; witness `a_move_that_roots_the_body_is_costed_for_the_slide_it_brings` (controls at rest and running away; three poisons). Grid against (a): 20 rows identical, three moved (the oiler 119/130 → 227/210, the robot 243/140 → 210/156, the cellular automaton 247/255 → 237/256), tie flips on axis bodies whose slide is a few px. It does not save rung-5 Sanic: any admitted attack is pressed, a risk only ranks attacks, and each of his grounded attacks roots. Whether a rooted momentum body slides or plants its feet is [Q166](awaiting-maintainer-decision.md) (recommended: plant, at the law's strongest rate); (b) the ledge margin may add the reaction travel `|vx| * staleness_s`. Each changes every fighter, so each needs the grid before and after.
4. **A ranged move scores `launch: 0`.** A projectile hit writes a dimensionless `HitKnockbackMagnitude::FeelScale(0.85)`, not `LaunchSpeed`. Settle whether the feel reference belongs on `LaunchConditions`, whether `0.85` leaves the projectile stepper, and what `max_knockback` means for a launcher. A launcher also needs a hazard-coverage feature (its `reach_fit` is zero at every range).
5. **A placed trap has a position, and `reach` is a radius.** Carry the dangerous region relative to the body at the resolved-offer seam, not another reach scalar.
6. **A teleport's destination does not reach the brain** (`TeleportParams`: `behind_nearest_foe`, `behind_gap`, aim). It belongs in the same resolved offer.
7. **The ladder's step per rung.** All shipped rungs author `rollout_depth: 0`, so the habit read is off at every rung (`read_weight` is deleted, Q90) and the L3 step the engine ladder takes at level 6 is missing. Measure `--rungs 3,4,6,7,8` to separate step size from one pair.

**Standing prohibitions:**

- Do not fix the lock with a second, softer admission rule or a `reach_fit` floor.
- Do not feed push coverage back into `reach_fit`.
- Do not patch the evaluator with a fighter-specific exception. Keep press generation separate from move utility.
- A two-move fixture cannot price a move against a kit. Pin a feature against the kit `attack_kit_of` builds.
- A fixture that shares an app with a live CPU measures the CPU too.

**Acceptance:** representative CPUs select movement-compatible and
movement-transition attacks from their authored menu across the intended
difficulty ladder, with no regression to the press/move identity contract.

### D72 — continue Smash parity from the inventory

**Owner:** [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md).

**Current state:** the inventory is the source of feature/parity truth. Do not
turn this queue row back into a chronological parity diary.

**Next implementation:** take the next inventory row whose policy is settled,
implement it on the production path, update that inventory row and add the
production acceptance witness.

**Blocked where applicable by:** product rows named by the inventory. (Q62
and Q89 were ruled on 2026-10-04: Q62 is the LDTK-SEMANTIC-DIFF task, and a
stand-in may keep an incomplete kit.)

### D166 — make character authoring boundaries load-bearing

**Owner:** [`engine/character-authoring-package.md`](engine/character-authoring-package.md).

**Current state (2026-10-08):** no known residual. The last one, the Smash
stand-in's hand-written verb list, is content in the form every fighter
authors (owner document, closed slices).

✅ 2026-10-15, the sheet image path. Each catalog row stated
`spritesheet: "sprites/<stem>_spritesheet.png"` beside
`manifest: "sprites/<stem>_spritesheet.ron"`: 187 rows in 8 authored RONs,
and every one agreed. The renderer writes the two files as one product, so
the image path is now `CharacterCatalogEntry::spritesheet()`, the manifest
path with `.png`. The field is deleted. Admission refuses a row that still
states it (`UnknownField`, `a_row_that_states_its_sheet_image_too_is_refused`)
and a manifest path that does not end in `.ron`. A missing image is refused
at `manifest`, the field the author wrote. The pairing guard
`test_sheet_and_manifest_are_the_same_character.py` is deleted: two stems in
one row cannot be written now. The Python readers (Hall inspectors, catalog
publish, renderer actor sidecar) and the two row writers
(`codegen_character_catalog.py`, `character_notes.py`) were moved too. The
SDK consumer fixture was the one row whose two paths did not agree
(`outlander.png` beside `outlander_spritesheet.ron`); its image is now
`outlander_spritesheet.png`.

**Next implementation:** search for a new duplicated authored/runtime value
with the owner document's two questions (A1). For each one found, choose one
authoring owner and make every runtime representation a projection or admitted
prepared value. Prefer deleting the second truth to synchronizing it. Exit
criterion 5 (another experience consumes a character without irrelevant
facets) is not yet measured.

**Acceptance:** the owner document can name one authoritative authored value for
each migrated fact, and production consumers cannot bypass its preparation or
projection boundary.

### WOLFENSTEIN-FAIRNESS — the Evil dimension's boss breaks a §3 rule

**Owner:** the boss author ([`boss-design.md`](engine/boss-design.md) §3).

**Finding (2026-10-15, `content_it` `boss_fight_validator`):** Wolfenstein's
`side_sweep` gave an Enrage punish window of 0 ticks against a floor of 12,
and `floor_slam`, `full_body_pulse` and `side_sweep` authored no telegraph.
The roster pin moved from 3/5 to 4/6 to record it.

**Closed 2026-10-17:** his fight is conducted now
(`ambition_content_modules::wolfenstein`, "the leash" in
[`bosses.md`](game/bosses.md)). Every move has its own pose and cue and a Rest
after its Strike. The pin is back at 3/5. Receipt:
`game/ambition_app/tests/wolfenstein_fight.rs`.

### BOSS-DRAMATIC-PATTERNS — a boss's `intro` and `transition` steps never run

**Owner:** `ambition_boss_encounter` (the pattern tick) and the boss authors.

**Finding (2026-10-17, read):** `tick_boss_pattern` runs a pattern only in
Phase1, Phase2 and Enrage (`is_attacking`), and `between_phases` says a boss
in its transition "runs no move". So the `intro` and `transition` patterns of
every `Scripted` boss are data that decides nothing. Seven bosses author
strikes in their intro (the T-rex's bite, the Hunny Horror's swipe, the FSM's
pulse and more), and the validator still counts those beats. A conducted boss
plays its own intro and transition (the Hunny Horror's roar, Wolfenstein's
wake-up and lightning).

**Decision owed:** delete `intro` and `transition` from `Scripted` (the
encounter and the conductor own the dramatic beats), or run them as authored.
Do not leave a field that decides nothing.

### PORTAL-STRUCTURAL-FX — sprite and particle effects ignore the panes

Jon, 2026-10-17: "it should not be structurally possible to add something
into the game that doesn't work with portals." Meshes built on the CPU now
have one road that folds them through pairs and out of far-side panes
(`ambition_render::folded_mesh`, used by every line look). The `fx.rs`
effects have none: particles, beams, impacts and slash sprites are drawn
whole at z 30 and up, over a pane on its far side, and are not cut at a
crossing. Measured the same day by reading `portal_compositing.rs` (they
carry no candidate marker). Next: give them that one road (a sprite effect
through the far-side and transit pieces by construction, or the folded
writer for quads), not a per-effect opt-in.

### DOG-THROUGH-THE-DOOR — a roaming dog's door test measured a distance

**Closed 2026-10-16:** `a_dog_sent_for_an_item_in_another_live_room_goes_through_the_door`
failed at its control because the control measured a distance (more than
200 px from the door's middle) and the roaming dog now climbs the ledge over
the door to 192 px. It never went into the door's zone. The control now asks
what the door acts on: the dog's box never overlaps the door's zone, and the
dog stays in the hub.

### BAKED-SHEET-IDENTITY — a body's collision box is outside the content identity

**Owner:** `ambition_sprite_sheet` (the baked sheet index) and
`ambition_platformer2d_provider` (`MechanicalRegistries`).

**Current failure (read 2026-10-05, not poisoned):** a sprite-authored body
takes its collision box, its per-pose boxes and its attack polygons from the
baked sheet index (`record_for_sheet_key`: `sprite_body_collision_for_sheet`,
`posed_body_geometry`). The index is `baked_sheet_rons::sheet_rons()` (compiled in under
`static_content`, read from the published tree at startup in a development
build since 2026-10-11), held behind a process `OnceLock`. No content-identity section reads it:
`characters.authored-sheets` is `AuthoredSheets::deterministic_dump`, which
holds only the sheets a provider registers (one production caller,
`room_transition_assets.rs`). Published sprites are not in version control,
so two machines at one revision can hold different body metrics under one
content fingerprint. A rollback timeline contract that compares the
fingerprint then accepts a peer whose bodies are a different size.

**Blocked by:** [Q157](awaiting-maintainer-decision.md#q157--when-two-machines-hold-different-published-sprite-metrics-may-they-play-together)
for what a difference does (refuse, warn, or nothing). The projection itself
is not blocked.

**Not the fix:** a digest of each sheet's text. It also changes when only the
atlas packing changes.

**The projection is built (2026-10-08):**
`ambition_sprite_sheet::sheet_mechanics` encodes each record's mechanics (key,
target, frame size, body metrics with every box, part, polygon and feet,
per-animation hurtboxes, hitboxes and frame durations, tuning, drawn facing,
and each row's animation, frame count, durations and mirror) and not its
packing (images, pages, `label_width`, `y_offset`, `row_index`, `rects`).
`character::sheets::baked_sheet_mechanics_digest` is that digest over the baked
index in key order, computed once at runtime. Witnesses:
`a_sheet_whose_packing_alone_differs_has_the_same_digest` (poison: hash the
rects; red), `a_sheet_whose_mechanics_differ_has_another_digest`,
`the_baked_digest_is_this_builds_records_in_key_order`.

**What is left is the ruling.** The digest is not in the content fingerprint,
because Q157's default in force is (c), leave it out. Ruling (a) is one
section, `characters.baked-sheets`, beside `characters.baked-landmarks` in
`prepare_platformer_content` (a field on `MechanicalRegistries` filled by both
provider roads, as `baked_landmarks` is). Ruling (b) needs a weaker class of
identity section.

**Acceptance:** a baked sheet whose body box differs gives a different content
fingerprint; a sheet whose packing alone differs gives the same one.

### MOUNT-RIDER-CUSTOMER — a shipped rider controls a shipped mount

**Owner:** `features/brain_command.rs` (`MountedBrainCache`, the source-only
arm) and the dismounted-rider road (`features/ecs/dismounted_rider.rs`).

**Ruling:** Q76 (2026-10-03): keep mount/rider support and give it an early
customer (a character riding the dog, or the robot commandeering a shark).

**Current state:** `MountedBrainCache` has no production constructor, so
`ControlClaimant::Mount` has no production writer. Nothing resumes the mount's
recorded brain when a ride ends; a mount death rebuilds a solo brain from
config.

**Next action:** author the customer, then specify and repair, each with a
witness: control transfer, the mount's previous brain, dismount and its
restoration, body and room lifetime, damage and death, participant ownership,
animation composition.

**Acceptance:** the customer mounts, the mount obeys the rider, and on
dismount the mount runs the brain it had before; the same after a rewind.

### REACH-VIEW — AI reads reach from the move's geometry

**Owner:** the moveset; `ambition_entity_catalog::MoveFrameData::reach` for
fighters and `ambition_characters::brain::action_set` (`reach_px`) for action
sets.

**Ruling:** Q35 (2026-10-03): the moveset/attack definition owns prospective
mechanical reach, also during startup. AI reads a derived view of the move
geometry, never sprite bounds. This extends Q107.

**Next action:** find each AI read of reach, and make sure that each one comes
from the move geometry by phase. `MoveFrameData::reach` is already derived from
the Active volumes.

**Acceptance:** a test changes a move's hitbox and sees the AI's reach change,
with no sprite change.

**Measured first (2026-10-03, the shipped prepared cast, 29 rows with an
autonomous profile):** 15 Smash rows have an attack move, and for 14 of them the
authored `smash_hit_band` was more than 1 px from the reach of the move. Examples:
goblin 32 against 38, the automata 36 against 44, Carl Stargan 36 against 22,
the goblin brute and the pirate heavies 36 against 48.4. Also measured: the
moveset derived from an action set reaches `1.1 × reach_px` (28 gives 30.8), so
`reach_px` is not the hit extent. The moveset geometry is the one answer.

**Slice 1, DONE 2026-10-03: the Smash brain.**

- `BrainSnapshot::melee_reach` is the reach of the move that the body's forward
  attack press starts, in the real posture of the body. The snapshot builder
  fills it with `melee_reach_of`, which asks `move_for_attack`, the resolver of
  the press road. A running body thus reads its dash attack.
- `tick_smash` sets its three distance bands from that reach each tick
  (`SmashCfg::with_hit_band`). The bands in a `SmashCfg` are only those of a
  body with no attack move (`NO_ATTACK_MOVE_HIT_BAND`, 36 px).
- `BrainProfile::smash_hit_band` is deleted, with its four authored rows. <!-- cite-ok: records a deleted field -->
- Witnesses: `the_hit_band_is_the_reach_the_snapshot_states` (combat),
  `a_smash_brain_swings_where_the_hitbox_of_its_move_reaches` (the acceptance
  test: two bodies that differ only in one hitbox) and
  `the_reach_is_that_of_the_move_the_press_starts` (monolith), and
  `a_smash_enemy_swings_from_the_reach_of_its_own_move` (app_it: the goblin
  brute decides its first press at 48.3 px; with the snapshot line removed it
  closes to 35.4 px).

This changes how the Smash enemies space themselves: each now stops and swings
where its own move reaches.

**Slice 2, DONE 2026-10-03: MeleeBrute, the hostile Aerial bird, the aggressive
Patrol.** Each read `cfg.attack_range`, an authored distance. Each now reads
`BrainSnapshot::melee_reach`, and its cfg distance is only for a body with no
attack move. The snapshot builder derives the reach for every body with an
attack move (review 2026-10-04: the reach is a fact of the moveset, so no
brain gate decides if it is derived). Each brain decides if it reads it: a
peaceful patroller does not, because its `attack_range` is where it stops to
talk (`a_peaceful_patroller_keeps_its_own_distance_when_told_a_reach`; poison:
it reads the reach, and it stops turning to the foe at 40 px). The old gate,
`StateMachineCfg::closes_to_its_melee_reach`, is deleted. <!-- cite-ok: a deleted name --> The shipped MeleeBrute users are
the provoked pirate heavies (reach 48.4, authored 53 to 59 with the 56 px
floor). The parrot read 60 and fsm_noodling 50; both pecks reach 52.8.
Witnesses: `melee_reach_tests` in `brain/state_machine/tests.rs`. Each read
was poisoned alone, and only its own test failed.

**Remaining readers (not changed):**

- `ChargeCrashCfg::bite_range` (the shark 200 against 46.2). It is not a
  reach. The shark presses its bite and then charges, and the charge carries
  the hitbox to the foe. A view from geometry must add the travel of the
  charge to the reach of the bite. This needs a measurement of how far the
  charge moves the hitbox while the Active window is open.
- The fighter's `assumed_foe_reach` (60 px): since 2026-10-08 it is only
  the reach of an attack the shadow predicts. A foe the view sees swinging
  carries the reach of the move it plays (`PerceivedActor::attack_reach`,
  from `MoveFrameData::reach`), and the shadow lands that swing from there.
  Witnesses: `a_watcher_reads_a_swings_reach_from_its_hitbox` (monolith;
  two hitboxes 20 px apart read 36 and 56) and
  `a_foes_swing_reaches_as_far_as_the_move_it_plays` (combat; a 100 px move
  lands from 90 px, a 40 px move and the 60 px assumption do not). Each
  poisoned.
- A body with no attack move keeps the authored distance of its brain. One
  case looks incorrect and is not measured in play: a dismounted rider with no
  ranged item gets a MeleeBrute brain whose distance is the profile's
  `attack_range` (1100 px for the pirate raider), so it can stop and press
  nothing from far away.
- `MoveFrameData::reach` is the reach of the volumes in the body frame. It
  does not include the motion of the move (a dash attack moves the body), so a
  running Smash enemy reads 40 px for a dash attack that travels farther.

### CPU-LADDER — the brain owns the knobs, Smash owns the ladder

**Owner:** `ambition_combat::brain::fighter` (mechanism) and the Smash
rules/content (ladder tuning).

**Ruling:** Q88 (2026-10-03): the generic fighter brain exposes reusable
controls (reaction, tactics/aggression, prediction/evaluation, execution/error);
Smash owns the mapping from "CPU level N" to them. Q90 (2026-10-03): remove the
inert `read_weight`; a correctly named parameter returns with an implementation
that gives it meaning.

**Current state:** `fighter_brain_ladder.ron` is in
`game/ambition_content/assets/data/`, and its schema is registered by
`ambition_combat`. ✅ Q90 is built (2026-10-08): `read_weight` is deleted from
the profile, the ladder and every fixture; the habit decay is the constant
`HABIT_DECAY` (0.9); the rollout reads the habit whenever the read is genuine.
No ladder rung changes (none rolls out). `for_level` brains at levels 6 to 9
roll out, so their habit counts now decay at 0.9, not `t × 0.6`.

**Acceptance:** the ladder data and its level vocabulary live with the Smash
rules/content; another game can build the brain with no ladder; no authored
ladder field is inert.

**Measured 2026-10-08 (NamekAmbition):** the Smash demo app installs no
fighter ladder (`the_ladder_the_demo_runs.rs` pins the floor).
`ambition_content` `plugin.rs` and `reload.rs` (`publish_fighter_ladder`)
are the only installers. A Smash pack source may name a file outside its
root, as George's facet does.

**Next (2026-10-08):** the engine still carries a level curve,
`FighterBrainProfile::for_level` (13 production fallback sites), and the demo
plays it. The order of the move is in
[`fighter-brain.md` F1](engine/fighter-brain.md#f1--the-authored-ladder-is-the-authority-q88-2026-10-03):
the ladder becomes a rule scoped to the rooms it governs (`ambition_app`
composes Smash, so a second global installer would make two authorities),
Smash declares its pack's rows for its mode, and then the floor becomes one
level-free default.

**Built 2026-10-13 (step 1):** the global resource is deleted; the ladder is
`DeclaredRules<FighterBrainLadder>`, declared by `ambition_content` for every
room and restated by its reload; each brain reads its own room's ladder.
Witness `each_fighter_takes_the_rung_of_its_own_rooms_ladder`; grid unchanged.
**Blocked (step 2):** with Smash's rows declared, the George mirror in the demo
throws no move at rung 5 (18-tick reaction resonance; 325 ms fights). See F1.
**Measured 2026-10-15:** the own-motion lead (the live self through
`Perceived`) was built and does not unblock it: still 0 moves in the mirror.
The other error is the foe's turn, unseen for the whole approach (turn to
crossing about 0.37 s, reaction 0.30 s). Next: a brain policy for a foe it
cannot yet see turning, measured on the rig; it changes every CPU. See F1.
**Built 2026-10-16 (step 2):** the block is fixed by a reaction spread (each
decision reacts in the delay ± one sixth of it, drawn from the seat's own
stream), measured on the seeded rig: the George rung-5 mirror went from 0%
and 480 s stalemates on all 16 seeds to 54 s bouts, and every rung pair of
George's ladder is ordered (16:0, 16:0, 14:2, 16:0). Smash's pack names the
ladder file and the demo declares and restates its rows. **Next (step 3):**
the floor becomes one level-free default. See F1.
**Built 2026-10-16 (step 3):** `for_level` is deleted; a room with no ladder
plays `FighterBrainProfile::engine_default`, one profile at every level. The
acceptance holds: Smash's content owns the ladder, another game builds the
brain with no ladder and inherits no curve, and the one inert field
(`read_weight`) was removed under Q90 on 2026-10-08.
CPU-LADDER is done.

### LANDMARK-CLIP-TIME — a published landmark clip loops or holds as the row it describes

**Owner:** `ambition_sprite_sheet` (`baked_landmarks`) and
`ambition_characters::actor::landmarks`. Part of RIG-LANDMARKS. Review of
2026-10-05, finding 3.

**Current failure (read 2026-10-05):** three sources answer one landmark
question and they keep time in three ways. A rig clip states `looping` and
holds the last frame of a one-shot (`RigClip::frame_at_time`). The visual
animator holds a row whose pose is in a code table
(`ambition_sprite_sheet::character::non_looping`), and holds every row a move
asks for by name. The package landmark clip (`LandmarkClip::frame_at_time`)
always wraps, and its published schema has no loop statement. So after a
one-shot row ends, the sprite and the rig hold the last frame and the package
landmark goes back to frame 0: a hand or a head jumps while the art does not.
The pet reads this query, and so do the shots and the held items.

**Built 2026-10-05:** `LandmarkClip` carries `looping`, and
`LandmarkClip::frame_at_time` holds a one-shot on its last frame, as
`RigClip::frame_at_time` does. The table's source states the bit from the one
rule the animator uses for a row name
(`ambition_sprite_sheet::character::row_loops`); the query (`BodyLandmarks`)
infers nothing. Witnesses: the real `CharacterAnimator` against the table of
`player_robot_v3`, 2.3 clip lengths in, for `idle` (loops) and `shoot`
(holds): before the change the table was on frame 2 while the sprite held
frame 5 (`a_landmark_clip_loops_or_holds_as_the_animator_shows_its_row`); and
a rig against a table of the same clips, for a looping and a one-shot clip
(`a_rig_and_a_package_keep_one_time_for_a_looping_and_a_one_shot_clip`).

**Open, measured 2026-10-05:** the rigs and the animator do not agree with
each other. No published sheet states which rows loop. Each rig target states
it by hand (Mary-O: `idle`, `walk`, `climb`, `swim`, `crouch_walk` loop; every
other row holds), and the animator's code table loops five rows that the
eight published rigs hold: `crouch`, `crouch_jump`, `jump`, `skid`, `taunt`
(`the_rows_a_rig_and_the_animator_time_differently_are_these` holds the set).
A rig is not admitted in a shipping game, so no player sees this today. The
repair is ONE loop statement for each row, published in the sheet, which the
animator, the rig and the landmark table all read; then the code table and
the rig's own bit go. The renderer owns the publish (Toothbrush), and every
sheet is published again.

**Named limits:** the bit is of the row and not of the pose. The animator
times a pose by the pose it was asked for, so a one-shot pose that falls back
to a looping row (a `hurt` pose on a sheet with no `hurt` row shows `idle`)
is held by the animator and wrapped by the table and by a rig. The bit is set
when the table is decoded, from code, so the landmark digest does not hold
it: two builds of one revision agree.

**Acceptance:** ✅ for a looping row and for a one-shot row, more than one
duration in, the package frame is the frame the animator shows; ✅ the rows on
which the rig and the animator disagree are a named, counted set that a test
holds. Open: one published loop statement.

### MOUNT-SEAT-LANDMARK — a mount's seat is the saddle its art states

**Owner:** `ambition_mount` and the landmark query; the renderer for the
publish (Toothbrush). Part of RIG-LANDMARKS. Review of 2026-10-05, finding 6.

**Current failure (read 2026-10-05):** the shark's catalog row holds
`saddle: Some((28.4, -10.2))`, a number measured from the art's `saddle`
socket and typed into the gameplay catalog (`d1589237c`). The art moved once
and the number did not, which seated the rider off the saddle. A second
redraw repeats it. The socket itself is placed by hand in the renderer target
(`burning_flying_shark.py`, `_px(396.0, 152.0)`), is a rest-pose point in
`<target>_actor.ron`, and is not a track of the part flipbook: it does not
move with the fly or bob frames (Toothbrush, measured 2026-10-05).

**The fix:** gameplay names the seat as a landmark and the art package
supplies its place for each pose: a `saddle` track in the part flipbook,
projected into the landmark table as `head` is. An authored seat stays for a
mount whose art publishes none, as the named fallback. Do not write an
importer for the shark alone.

**Blocked on:** the renderer publishing the `saddle` track (Toothbrush).

**2026-10-06 (review P2, the T-rex jaw):** the seam for a named point that a
body rides exists for a body with a BODY RIG: an attachment the art states on
a joint (`RigAttachment`), placed by `resolve_body_rig_poses`, read by the
capture relation (`CapturedBy::hold_attachment`) and offered to a module
(`ambition.body.attachments`). The T-rex's jaw uses it. The shark does not:
it publishes no body rig and no per-pose saddle, so this row stays blocked on
the publish. When the shark has a rig, its saddle is one `ATTACHMENTS` row in
its renderer target and the mount reads the attachment as the capture relation
does; do not add a second road. One known difference to close then:
`body_landmarks::feet_of` places a rig from the bottom of the body's box and
does not read `RigFeetOffset`, which `rig_feet_from_centre` (hurt parts, hold
points) does.

**Acceptance:** the shark's row holds no saddle number; a redraw that moves
the saddle moves the rider with no gameplay edit; a mount with no published
saddle still seats its rider at its authored seat.

### RIG-IMPOSTOR-CONTAINMENT — a part-drawn body is drawn whole or refused

**The invariant:** every pixel a composited body draws lies in its cell. One
camera draws a whole page with no per-cell scissor, so a part past its cell
is cut from its own body and drawn into its neighbour's.

**Owner:** `ambition_render::rendering::actors::rigged` (`impostor_margin`,
`posed_reach`), `ambition_sprite_sheet::character::rigged` (`art_overhang`)
and `scripts/measure_rigged_parity.py`. Plan:
[`engine/mary-o-part-realization.md`](engine/mary-o-part-realization.md).

**Scope since 37497a444 (2026-10-05):** a part-drawn body draws its parts in
the world, and uses an impostor cell only while something composes it
(`ComposedBodyDemand`: hit flash, portal pieces, deep dream, quasar) or its
frame fades as one picture. So the invariant applies to that composed road
only.

**Solved, by kind of draw:**

- A published frame (2026-10-05). The cell is the frame plus
  `impostor_margin`, which covers the flipbook's stated `art_overhang`. The
  oni leader's banner is drawn whole (`a_cell_holds_every_draw_of_its_body`).
- A tween's in-between (2026-10-06, review P2). `art_overhang` is a
  conservative envelope of every draw `tween_into` can make, taken when the
  flipbook is read: each moving draw is sampled finely enough that no corner
  moves more than half a pixel between samples, and that slack is added
  (`tween_overhang`). A part that turns between two frames reaches where
  neither frame does (`the_overhang_of_a_tweened_clip_covers_its_in_betweens`;
  the published corpus: `no_in_between_of_a_published_flipbook_reaches_past_its_overhang`).
  Measured on the 123 published flipbooks with a tweened clip: the in-betweens
  move the stated overhang of 12 of them, by 0.21 px at most; no cell changed.
- Parts placed by a `PartPose` (2026-10-06, review P2). A pose is in no file,
  so its reach is measured each frame (`posed_reach`). While it is more than
  the margin, the body is drawn directly, as a body whose frame fits no cell
  is: whole, and its readers see no image, with one warning. The hold that
  smooths composition demand smooths this too
  (`a_part_pose_that_reaches_past_its_cell_is_drawn_directly`).

**Open:**

1. `scripts/measure_rigged_parity.py` still clips its oracle to the cell, so
   a cut part cannot read as a parity failure there. The clip was added when
   the cell was the frame plus 16 px (the oni leader's banner read as 1153
   wrong pixels of a correct draw). With the margin from `art_overhang` the
   clip should remove nothing. Remove it and run the harness on a GPU
   (`rigged_sprite_parity`); acceptance: the oni leader passes unclipped, and
   with `impostor_margin` poisoned back to 16 px it fails.
2. A posed body that reaches past its cell loses its composed readers (its
   hit flash overlay, its portal pieces) while it does. If a ragdoll must keep
   them, give the presentation a cell chosen from the pose's reach, or let a
   pose declare an envelope. No shipped body carries a `PartPose` today.
3. A two-body shared-page test that reads the pixels of both cells needs a
   GPU; the three proofs above are on the draw geometry.

### DENSE-MELEE-ROOM — author the dense-melee development room

**Owner:** content; first measured customer
[`engine/bounded-perception-and-attention.md`](engine/bounded-perception-and-attention.md#acceptance-for-the-open-increment).

**Ruling:** Q93 (2026-10-04): keep or create a deliberate dense-melee
development/stress room. It is a real scenario, not polished content.

**Current state:** no such room; the density script fakes density by widening
the viewport over the Hall's `stand_still` cast.

**Next action:** after the P1 correctness rows above, author one room (LDtk or
a generated spec) with many tactical-brain fighters in close melee, reachable
by an ordinary route and by `measure_perception_density.sh`.

**Acceptance:** in the room `kept` tracks population until the budget caps it,
and the census, frame time and camera framing are readable from one run.

### LDTK-SEMANTIC-DIFF — review content diffs by meaning, and find the rewrite — ✅ DONE 2026-10-15

**Owner:** `tools/ambition_ldtk_tools` (`edit/semantic_diff.py`). Plan:
[`engine/authoring-and-tools.md`](engine/authoring-and-tools.md#content-diffs-need-domain-aware-comparison-q62-q78).
Ruling: Q62 and Q78 (2026-10-04) are engineering tasks.

**Receipt:** the comparison is `diff semantic` (file or `REV:PATH`), `diff
range` and `diff normalize` (`tests/test_semantic_diff.py`). The rewriter was
`generate hall-of-characters`, which renumbered 145 iids and 5 uids on each
regen; a replaced level now carries its identities
(`tests/test_area_regen_keeps_identities.py`). A second run of each writer on
its own output changes no byte: `compact`, `policy fix`, `asset editor-art`,
`repair`, `camera auto-cover --create`, the Hall regen (2026-10-08), and
`entity move` and `entity set-field` (2026-10-15, on a copy of
`sandbox.ldtk`: the first run changed only the lines it was asked to, 3 and
2). `level clone` mints new ids by design.

### ASSET-PRODUCT-LAYOUT — runtime roots by tier; editor products apart

**Owner:** `ambition_asset_manager` path builders, `ambition_sprite_sheet`,
the publish scripts. Plan:
[`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md#products-are-laid-out-by-what-they-are-q82-q83).

**Ruling:** Q82, Q83 (2026-10-04).

**Current state:** full quality is the bare `sprites/` root and the reduced
tiers are suffixed siblings. ✅ 2026-10-15, the ultrapack's runtime product
is cut by runtime unit: only the pack-plan groups a runtime reader loads ship
(`runtime_groups`), 1.8 MB, was 695 MB; the whole pack is a source in the
renderer's cache (owner page).

**Next action:** audit the listed path builders, then move to
`<root>/{full,half,quarter,potato}/` in one change.

**Acceptance:** every tier is a named directory; the packager selects roots
and has no new exclusion; the packaged ultrapack holds only reachable pages;
the game draws the same at each tier.

## Receipts

Closed rows that an open row, a script or an inbound link still names.

### A4 — separate control authority from body execution on the real schedule — ✅ DONE 2026-10-09

`sim_phase_pins::every_control_writer_is_ordered_against_the_gate_and_the_gate_before_integration`
reads the declared access of every system in the shipped `GgrsSchedule`
(2026-10-08: 22 `ActorControl` writers, none unordered against
`PlayerInputSet::ControlGate`). The four that produced intent after the gate
moved before it: `shark_ride::tick_departures` and the boss brain
(`tick_boss_brains_system`) run after `ActorDecisionSet::Publish`; `BossSteerSlot`
runs in the gate after `drive_commanded_moves`. `WRITES_CONTROL_AFTER_THE_GATE`
lists only systems that spend a press, integrate, or derive from a gated frame.
Witness: `boss_motion_parity::the_frame_a_held_boss_integrates_under_is_the_gated_one`.
One commanded-move road walks every body, a boss too
(`a_lured_boss_walks_to_its_mark_and_starts_no_attack`, `a_dead_body_is_not_walked`).
The Mockingbird's path is its conductor's
(`its_path_is_its_conductors_and_a_hold_on_its_brain_changes_nothing`). Its
brain keeps its movement write: the body road declines locomotion for a held
body in one place (`integrate_actor_body`), for a rider and a conducted boss
alike.

### AUTHORITY-POLISH — one owner per mechanical fact, and no mirror in the rollback kernel — ✅ DONE 2026-10-09

Closed on a fresh census, as its acceptance asked (2026-10-09): every
multi-writer App resource and session-world component carries a verdict
(`check_multi_writer_resources_are_adjudicated.py`: 8 of 23 session-world
components are multi-writer, all adjudicated; the rollback-registered queue is
spent), every optional read of a session authority says what its `None` means
(`check_session_authority_none_arms.py`: 141 sites, 99 rows), and no resource
crosses the rewind boundary unclassified (`resources_crossing_the_rewind_boundary.py`).
Its continuous items: AP9 (stale docs) is ordinary hygiene; AP14 is its own row
under P1. C11 in [`consolidation-plan.md`](consolidation/consolidation-plan.md)
owns the history.

### BAG-RECORD-HORIZON — a bag record is owned by what is left of its consequence — ✅ DONE 2026-10-09

A restore keeps the grant and spend records it keeps beside the bag
(`ItemCheckpointRestoreInputs.grants`/`.spends`), an occurrence that ends in a
live room becomes `Consumed` with its room's participants as owners, an ended
mint's row is compacted at the next checkpoint, and an object in a spared room
stays where it is. Witnesses are in `death_restores_the_checkpoint.rs`. The one
open line, whether a dormant row is owned by participants, is Q161 (default (a),
today's behaviour, in force); nothing is buildable before its answer.

### SETTINGS-ROLLBACK — a settings change reaches simulation only at an admitted rebase — ✅ DONE 2026-10-08

The frame-mode half was closed earlier (`ControlFrame` carries
`control_frame_modes`). The damage half: `PlayerDamagePolicy` was written from
`UserSettings` in `Update` and read by three simulation systems, so a
resimulated frame read the current difficulty. It is now a `Q120` mechanical
domain. `propose_player_damage_policy` writes `ProposedPlayerDamagePolicy` and
proposes; `publish_player_damage_policy`, the only writer of the policy, copies
it in `MechanicalEditSet::Publish` when the timeline admits it. A local
timeline is stopped and rebased; a foreign or unhealthy one refuses, and the
proposal waits. No simulation system reads `UserSettings` or the proposal.
Witness: `a_settings_change_reaches_the_simulated_policy_only_when_the_timeline_admits_it`
(poisons: a publisher that ignores the admission; a proposer that never
proposes). `Q127` (match-wide or per participant) changes the shape of the
policy, not this road.

### CANDIDATE-GENERATION-ORDER — a candidate session is prepared from the generation before its own activation — ✅ DONE 2026-10-08

Owner: [`engine/extension-model.md`](engine/extension-model.md) and
`crates/ambition_platformer2d_provider/src/lifecycle.rs`. Candidate B is built
from transaction-local N+1 (`PendingGenerationInputs`, keyed by `load_id`):
the cast, bosses, character catalog, audio and adaptive cue providers. The
fighter ladder and the encounter waves are stated standing projections.
`AuthoredSheets` cannot change across the window (a reload refuses it), and
`forced_brains`, `population_cap` and `perception_extent` are inserted once at
plugin build (re-read 2026-10-08). Witnesses:
`the_candidate_is_built_before_the_router_advances_and_providers_only_adopts`,
`a_boss_tuning_saved_while_the_game_runs_is_played`,
`a_candidate_that_drops_a_providers_audio_is_refused_and_the_live_audio_survives`
(control `a_candidate_that_edits_a_providers_audio_activates_and_publishes_it`).

### MUSIC-CANDIDATES — music is chosen from scoped, prioritized candidates — ✅ DONE 2026-10-08

Ruling Q72, Q150. Owner: `ambition_encounter::music` (`EncounterMusicRequest`,
`MusicSource`) and `ambition_platformer2d_actor_monolith/src/music/intent.rs`.
Each source owns one candidate per room; a source is a kind and an instance
(each encounter script is its own source, keyed by its `SimId`); the latest
claim plays, ties by source order; the request is in the peer checksum by
value (schema 331). Witnesses:
`a_participants_boss_outranks_the_primary_seats_room_music`,
`the_heard_room_is_the_highest_priority_then_the_primary_then_the_lowest`,
`a_release_leaves_the_claim_of_another_source_in_the_room`,
`the_claim_that_plays_does_not_depend_on_the_order_of_the_claims`,
`two_scripts_in_one_room_are_two_music_candidates`,
`the_checksum_sees_each_part_of_a_claim_and_not_its_order` and
`a_boss_music_claim_is_the_same_after_a_rewind` (poison: a resimulated tick
that writes another track; a checksum mismatch with the value probe, none
with the old presence probe). Deferred to its customer: a priority authored
per candidate, when content needs a value between the tiers.
`EncounterEffect::SetMusic` is kept with no shipped customer (Q74: usage is
not worth).

### MIRROR-SYMMETRY — mirrored CPUs stay mirrored per tick — ✅ DONE 2026-10-08

Q49: symmetry is a correctness property. Plan:
[`engine/fighter-brain.md`](engine/fighter-brain.md#mirror-symmetry-is-a-correctness-property-q49).
Two Emmys are one fighter reflected, compared per tick over the full state
(`two_emmys_are_one_fighter_reflected_through_a_mutual_grab`). They parted on
frame 883 while a mutual grab went to the lower `SimId`; since 2026-10-11 a
mutual grab parries (S0.2) and they do not part. The decision layer has its reflection test.
Each poison of plan item 5 is red: placement and an unmirrored stream in the
match; the zero-lateral `signum`, the floor list order and the left-first
`nearest_support` in their unit witnesses, which the match does not reach.

### LEDGE-OCCUPANCY — two fighters can hold one ledge — ✅ DONE 2026-10-08

Q43 follow-up: Ultimate-like occupancy and trump, deterministic and
rollback-compatible. Plan:
[`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md#ledge-occupancy).
One corner is one edge whatever the bodies' sizes
(`two_fighters_of_different_sizes_on_one_corner_are_one_edge`); occupancy is
derived each tick from the hang, so release and knockoff free the edge, and a
body out of play holds none (`a_body_out_of_play_holds_no_edge`). A rewind
across a trump gives the same holder
(`a_rewind_across_a_ledge_trump_gives_the_same_holder`). The regrab limit
(six per airtime, decaying intangibility, reset by landing or a hit) and the
trumped body's lockout (`CombatRules::ledge_trump_lockout`, Smash 0.5 s) are
built. Not built: Ultimate's "the trumper cannot let go for about 20 frames"
(unverified source).

### AUTHORED-INTERACTABLE-STATE — facing gates, per-chest and per-pickup persistence — ✅ DONE 2026-10-08

Q63, Q105. A one-time pickup stays taken through its `Consumed` occurrence
row, and an opened chest through its `Spent` row; both survive a rebuild of
the room, and the save carries them across a load
(`a_one_time_heart_stays_gone_when_its_room_is_built_again`,
`a_load_does_not_build_a_one_time_heart_the_file_remembers_taken`,
`an_opened_chest_is_built_opened_when_its_room_is_built_again`,
`a_load_builds_opened_a_chest_the_file_remembers_spent`, and end to end
through a written save file,
`a_chest_opened_in_play_is_opened_after_its_save_file_is_loaded`). Two
bodies that open one chest on one tick are paid once
(`two_bodies_on_one_chest_in_one_tick_are_paid_once`). A chest authored
open (`ChestSpec::opened`) lowers into the `Opened` marker that play sets.
`InteractableSpec::requires_facing` lowers into `RequiresFacing`, and the
interact road refuses a person or switch the body does not face
(`a_facing_gated_switch_refuses_a_body_that_faces_away`). LDtk authors both
(`opened` on `ChestSpawn`, `requires_facing` on `NpcSpawn` and `Switch`:
contract rows, converter reads, editor definitions in `sandbox.ldtk`;
`an_author_writes_an_opened_chest_and_a_facing_gate`). The inert
`InteractableSpec::enabled` is deleted. Deferred by Q63 itself: the
per-breakable debris cue.

### DURABLE-HORIZON-CHECKSUM — the save mirrors write hashed state from `Update` — ✅ DONE 2026-10-08

Q129: shared durable world state is peer state, compared by its canonical
form. Every writer of the hashed save runs in the simulation schedule (the
three `persist_*_to_save` mirrors, the dialogue-visit count and the restore
chain, BODY-BORN-ON-THE-TIMELINE). The bag (`OwnedItems`) is in the peer
checksum by its own value (schema 327,
`the_bag_is_compared_by_its_own_value`). Deferred to P6, not before:
`dialog_visits` moves out of the shared save to the participant with the
reactive-character memory model (Q134).

### CAST-FRAMING-TARGET — framing asks for a composition, not only a floor — ✅ DONE 2026-10-08

Q86. A framed cast asks for a view (`camera_snapshot::cast_view_target`):
the authored view scaled so the eased cast box and its margin fill it in the
tighter axis, in both directions. The hard bound
(`CAST_FRAMING_MIN_VIEW_SCALE`, 0.75 of the authored view) clamps that
target; the eased box is the smoothing and the room clamp is downstream.
Witness: `a_cast_that_closes_in_zooms_the_view_in_down_to_the_hard_bound`
(the control is a wide cast, which grows the view as before).

### WEAPON-READINESS — a refused trigger is visible as "not ready" — ✅ DONE 2026-10-08

Q33. `WeaponReadiness` (`Ready`, `Recharging { progress }`, `NoRoom`,
`NoAmmunition`) is the read model of the weapon a body's ranged press
reaches: the fire-rate floor, which keeps the length it was armed with (a
held item arms it with its own spec), or a charge body's fireball spawner,
which keeps the length of its cooldown. Each refusal on either road writes
`RangedFireRefused` with that readiness, and the prompt reads the model.
Plan and witnesses:
[`engine/participant-action-system.md`](engine/participant-action-system.md#p5--weapon-readiness-is-a-semantic-state-q33).

### GATE-PER-ACTOR — a body/capability gate is solid or open for each actor — ✅ DONE 2026-10-03

Q54. A gated wall with a subject form writes a `GatePass` for each body that
satisfies it; `RoomCollision::gates_open_for` is the one rule, and body steps
and the brain's movement and floor queries read the per-body walls. The design
is in [world gating](engine/capability-progression-and-world-gating.md#gate-subject-per-actor-q54).
Witnesses: `a_body_gate_is_open_only_for_the_bodies_that_satisfy_it`,
`a_gate_open_for_a_badnik_does_not_change_the_ground_it_plans_on`,
`a_fighter_over_a_floor_open_for_it_plays_as_over_the_void`. Not ruled:
awareness through an open gate stays on the shared walls.

### BOSS-REPLAY-RETRACTION — a replay that un-defeats a boss un-defeats it for every family — ✅ DONE 2026-10-03

Q51/Q56. One generic retraction for every boss family takes back the
placement, the chest, the mints, the bounty, the quest steps and their payout,
and on the death road every defeat since the checkpoint. The consequence list
is in [boss system](engine/boss-system.md). Witnesses:
`game/ambition_app/tests/boss_replay_retraction.rs`.

### MENU-OVER-DIALOGUE — an overlay opened during a conversation must not end it — ✅ DONE 2026-10-02

Q75. Start no longer ends a conversation (`back` alone closes it); pause, map
and inventory open over it, capture its input (`INVENTORY_CONTEXT` at 160,
above `DIALOGUE` at 150) and close back to `Dialogue`. Map and inventory are two
faces of one overlay (`menu::model::open_overlay_key`). Witnesses:
`start_never_ends_a_conversation_and_back_alone_does`,
`an_overlay_above_the_conversation_captures_its_input`, the per-backend
`start_during_a_conversation_opens_the_{menu,cube}_and_closes_back_to_it`, and
`update_schedule_census::the_conversation_reads_its_input_after_the_inventory_claims_it`.
Residual, read and not measured: under the Grid backend (the web build),
`menu.map` also toggles the standalone map panel
(`handle_map_menu_hotkeys`), which declares no input context.

### ROLLBACK-DEAD-SESSION — an invalidated GGRS session stops the clock in silence — ✅ DONE 2026-10-02

Q138. The harness refuses to step an unhealthy session
(`the_harness_refuses_to_step_an_unhealthy_session`); the rule is in
[headless verification](engine/headless-verification.md#drive-the-real-sim).

### SAVE-DIVERGES-AFTER-RELEASE — a pickup and release in `blink_run` desyncs the sync test on the save — ✅ DONE 2026-10-02

The save mirror read `InCustodyOf` before the projection that derives it.
`DurableHorizonSet` now runs after `ResidencyStep::Project`. Guard:
`every_reader_of_in_custody_of_runs_after_both_derivers`.

### BREAKABLE-SOLIDITY — a solid breakable is a barrier, not a blink wall — ✅ DONE 2026-10-03

Q102. `BlockKind::Barrier` and one solidity mechanism
(`is_full_collision_surface`, 21 readers); a moving platform stays a soft blink
wall. The rule is in [collision](engine/collision-and-ccd.md). Witnesses:
`the_hard_blink_upgrade_does_not_pass_an_unbroken_solid_breakable`,
`the_soft_blink_upgrade_passes_a_moving_platform`.

### BARK-CARDINALITY — a singular bark role has one owner — ✅ DONE 2026-10-03

Q52. A character's barks are the `barks` field of its one catalog row; catalog
assembly refuses a second provider's row for the same id with
`DuplicateCharacter`. Witness:
`a_second_provider_cannot_author_the_barks_of_one_character`. No plural bark
collection exists, because no content asks for one.

### SESSION-EDGE-STATE — a session starts from nothing the last one left — ✅ DONE 2026-10-05

A session that follows another equals a fresh host's first session with the
same save, on every tick from 0: run-time spawns carry the session scope,
channels and per-session state reset at the activation, a session is built
from its own experience's save (and a stale candidate is discarded), and it
begins with its experience's bag. The rules, the witnesses and five named
limits are in [the session edge](engine/construction-and-reconstitution.md#the-session-edge).

### LEVEL-BOX-READERS — a reader of a body's box outside the kernel asks the level box — ✅ DONE 2026-10-06

Every class of level-box read that was a footprint question asks the turned
box now, each with an arm red in turned gravity first; every remaining read
has a class and a reason in `scripts/baselines/level-box-readers.json`, held
by `scripts/check_level_box_readers.py`. The rule, two rejected approaches and
nine open items are in
[one box for a turned body](engine/controlled-character-actor-kernel.md#one-box-for-a-turned-body).

### CRAWLER-HAZARD-FOOTPRINT — a body tests hazards with the footprint it collides with — ✅ DONE 2026-10-06

A body that is not square tested hazards, water, ladders, a ledge carry and a
rebound pad with its level box while its step moved the turned box. Each arm
now states the box it moved and every world read in the step uses it. Witnesses:
`movement/tests/hazard_footprint.rs`, `step_box_world_reads.rs`,
`integration/body_box_tests.rs`. Q160 (water in turned gravity) and the view's
inverse footprint are open in
[one box for a turned body](engine/controlled-character-actor-kernel.md#one-box-for-a-turned-body).

### RIG-LANDMARKS — interactions read authored landmarks — ✅ DONE 2026-10-06

Q41. One landmark query (`BodyLandmarks`), answered by the rig or by the
tables built from each part flipbook (in the content fingerprint); the pet's
hand meets the petted body's authored contact point; the player's shot, a hand
weapon, a held item and its prop read one hand
(`ambition_held_items::holding_hand_world`; `gesture_hand` on the row names it);
a match seat and a re-worn body state the quad their art is drawn at.
Witnesses: `a_pet_hand_meets_the_contact_point.rs`,
`a_fireball_leaves_the_hand.rs`, `a_hand_muzzle_fires_from_the_drawn_hand.rs`,
`a_seat_with_no_sheet_states_no_drawn_quad`,
`a_reworn_body_states_the_quad_it_is_drawn_at`. Open questions: Q158 (the
fireball's flight from the hand) and Q159 (fitting a worn character's art). The
design and its open populations are in
[semantic landmarks](engine/runtime-rigged-sprite-animation.md#semantic-landmarks-q41).

### CALIBRATION-LAB-SHOT — a shot born at chest height in the calibration lab is gone on its first tick — ✅ CLOSED 2026-10-08

The cause is the room. The raider stands 4 px past the right end of a solid
in the lab's collision layer (144..272 by 688..704, the base of the rebound
pad). A shot born 26.6 ahead at chest height has a 24 by 18 box that starts
3.4 px inside that solid's lower corner; the sweep reports a start overlap
(time of impact 0) and the world-hit branch ends the shot on its first step.
The hip-height shot passes under the solid, and `mockingbird_arena` has no
such block. Not a defect of the spawn or of the instrument: a muzzle inside a
wall fires into the wall. Measured with a probe of the world-hit branch (the
hit block's name, box and kind); the comment in
`a_hand_muzzle_fires_from_the_drawn_hand.rs` now says why that room is not
used.

### CHARGE-SPEC-NAME — `SmashChargeSpec` is a generic mechanism — ✅ DONE 2026-10-08 <!-- cite-ok: the retired name this row records -->

Q44: no leaf-game name on a generic API. `SmashChargeSpec` is <!-- cite-ok: the retired name this row records -->
`MoveChargeSpec`, the field `move_charge`, and the payoff multiplier
`charge_mult`, in every source, table (the renderer submodule's George table
too), the exporter's JSON and the moveset inspector (`web/app.js`,
`check_bundle_contract.mjs`). The smash GESTURE (`ChargeGesture::Smash`), the
`smash_charge` animation clip and its SFX ids name the gesture and stay. The
content fingerprint moved; no rollback row did.

### ID-PEER — remove host-local lineage from peer-stable mechanical identity — ✅ DONE 2026-10-03

Two Apps that burned different numbers of local session activations reach the
same canonical identities and checksums. Closed roads: the smash roster seed,
`SessionScopedEntity` in the checksum, the match ordinal, `random_context`,
checkpoint operation keys, the session root's `SimId`, `TransactionId`
provenance, `GameplayElapsed`, the startup-resume checksum, perception's
`Entity` fallback, the GGRS carrier order, the schema fingerprint over
mechanical facts (Q122, `rewording_a_row_leaves_the_fingerprint_alone`), the
session-relative `SimTick` (Q128), the unchecksummed float rows
(`two_peers.rs`, two GGRS peers in one process) and the save handed to its
experience (Q129, `SaveOwner`). Witnesses:
`two_local_histories_compute_the_same_ggrs_component_checksums`,
`the_peer_visible_surface_does_not_record_which_route_the_host_visited_first`
and the arms in `id_peer_audit.rs`. The standing rules are in
[netcode identity rules](engine/netcode.md#identity-rules).

### ROLLBACK-MUTATOR-POPULATION — the mutator guard sees a quarter of rollback state — ✅ DONE 2026-10-03

`scripts/check_rollback_mutators_run_in_sim.py` reads every rollback
registration in every supported parameter spelling, including exclusive-world
bodies and one helper hop (`inherited_mutations`). 494 systems, 0 acknowledged
offenders. The save restore chain left the scan with BODY-BORN-ON-THE-TIMELINE;
`reconcile_roster_with_frozen_topology` left when the unread
`ActiveMatch` copy was deleted (schema 302). The guard's limits (`Transform`
excluded by name, Q139; one hop; a run condition is not reachability) are
stated in the script (`PRESENTATION_SHARED`, `BLIND_SPOTS`). Tests:
`scripts/tests/test_rollback_mutators_run_in_sim.py`.

### BODY-BORN-ON-THE-TIMELINE — the simulation applies the save — ✅ DONE 2026-10-03

The restore chain (`adopt_occurrence_checkpoint_from_save`,
`restore_inventory_from_save`, `complete_durable_restore`) runs at the head of
the gameplay root in the simulation schedule, so a rewind past it applies it
again on the same tick. Deleted with the `Update` window: the Q135
session-start gate, `refuse_a_restore_over_a_live_timeline`, the fixture <!-- cite-ok: records a deleted check -->
declaration `TheBodyIsBornOnTheTimeline` and three mutator-guard waivers. Q135
is reopened in the awaiting file. Witnesses:
`the_save_is_applied_by_the_simulation`,
`a_startup_load_is_applied_on_the_timeline_and_resimulates_identically`,
`a_mid_session_load_does_not_reach_back_across_the_rewind`.

### WEAR-REFUSES-UNPREPARED — a character outside the prepared generation is never worn — ✅ DONE 2026-10-03

Q103: refuse. The kit compiler takes a prepared definition (`WornKit::of`);
`WornKit::resolve` and the peaceful-kit fallback are deleted. A re-wear to an
unprepared id writes nothing and puts `WornCharacter` back to
`PersonaBaseline::id`. Witnesses
(`avatar::starting_character::tests`):
`a_rewear_to_an_unprepared_id_keeps_the_previous_character`,
`an_unprepared_id_writes_nothing_on_the_body`. The two remainders (a generation
that drops a worn id; the home body of an unprepared starting id) are open work
in [content generations](engine/content-generation-and-reload.md#open-work).

### THROW-MODIFIERS — route throws through rage and staleness policy — ✅ DONE 2026-10-03

Q133: follow Smash. A set launch declines rage
(`ambition_entity_catalog::launch::launch_speed`). A throw stales the throw
move: `CaptureThrowRequested` carries the use's `move_instance`, damage stales
through `hitbox::staled_damage` and the percent term through
`knockback_stale_scale`; a set throw stales its damage, not its launch.
Witnesses: `a_hurt_captor_throws_farther_and_a_set_throw_is_immune`,
`capture::a_repeated_throw_stales_and_a_neutral_ruleset_leaves_it_whole`,
`a_set_throw_stales_its_damage_and_not_its_launch`,
`a_throw_that_no_playing_use_claims_is_not_staled_or_recorded`.

### CHECKPOINT-ADMISSION-IS-NOT-COMMIT — an accepted restore changes nothing until it commits — ✅ BUILT 2026-10-05

An accepted restore pins its replay and changes no live state; its room is
built from the prospective facts, and its consequences run at the commit
(`RestoreConsequences`). One terminal rule gives every uncommitted operation
`Cancelled { NotCommitted }`; two primary bodies give `AmbiguousSubject`.
Schema 313. Witness: `a_cancelled_restore_changes_nothing` (three refusal
arms, and a committed control). The design, the witnesses and the one open
witness are in
[checkpoint restoration](engine/checkpoint-restoration-protocol.md).

### DEATH-IS-ROOM-LOCAL — a participant's death rewinds its own horizon, not the session's — ✅ DONE 2026-10-07

Q151. Each consequence since the checkpoint names the participants whose
horizons own it (a boss defeat's `present`, `GrantSource::Authored { owners }`,
`WorldTimeSchedule` owners, `ConsumedSinceCheckpoint`; a bag spend is owned by
its object), and a restore takes the dying participant out of each. A New Game
retires every other live room (`retires_beside`). Witnesses:
`boss_replay_retraction.rs`, `death_restores_the_checkpoint.rs`,
`pickup_regrowth_across_rooms.rs`, `a_second_seat_joins_the_session.rs`. The
design summary is in [open-world runtime](engine/open-world-runtime-and-residency.md)
("Death horizon (Q151)"); BAG-RECORD-HORIZON continues it. ⛔ No global
durable rewind followed by reconciliation of surviving rooms; no new uses of
`RoomReplayAdmitted::spared` / `spared_participants` as the model; a
`CheckpointDomainApply` reducer does not write back a row the acceptance pins.

### NPC-UNREGISTERED-CHARACTER — a person who names an unregistered character stopped the game — ✅ DONE 2026-10-05

**Owner:** `construction::preflight_planned_bodies`
(`ambition_platformer2d_actor_monolith`).

**Found** 2026-10-04 on the world reload road: an `NpcSpawn` whose
`character_id` is in no cast and no catalog passed LDtk validation and plan
preparation, and the recipe panicked when the room was built
(`report_unprepared_character`, `actor_spawn/character_spawn_plan.rs`). A
person is a placement row, and the preflight read no placement row. An
`EnemySpawn` with the same fault was already refused when its room was planned.

**Done:** the preflight takes the character catalog and refuses a person in
the one case the NPC road cannot build: the cast does not have the character,
a cast is published, and no catalog row can give it a body. It is the enemy's
refusal (`BodyCharacterNotRegistered`), and the room error names the room and
the character. The three cases the NPC road can build stay plans: a character
that only the catalog has (a borrowed kit), a person who names nobody, and a
composition that published no cast (a warning).

**Witnesses:**
`construction::tests::a_person_who_names_an_unregistered_character_is_refused_when_the_room_is_planned`
(the refusal and the four controls), and on the shipped session
`a_world_reload_that_cannot_rebuild_one_live_room_rebuilds_none`
(`an_edit_reaches_the_shipped_game.rs`): an enemy in the other live room (the
control, refused before and after), a person in the other live room, and a
person in the one live room. Each reload is refused with the room and the
character named, and no live room and no generation changes. Poison: the
refusal removed fails the unit arm, and the two person arms panic as before.

**Not done:** the assertion in `report_unprepared_character` stays. A road
that builds a person with no plan (none is known) would still reach it.

### NEW-GAME-RESYNC — a New Game after durable hydration fails the sync test — ✅ DONE 2026-09-29

A New Game is a checkpoint resume to the fresh baseline at the start room, on
the confirmed-frame road the death restore takes (`resume_at_checkpoint_on_reset`
reads `NewGameRequested`; the fresh-run reducers run inside
`CheckpointDomainApply`). ⛔ Do not register the four `derived` body types for
rollback to hide a rebuild on a speculative frame. Witness:
`a_new_game_asked_for_by_the_host_commits_once_under_a_rewind`.

### A10 — candidate world / last-good-world publication — ✅ DONE, DEMOLITION CLOSED 2026-09-16

A failed candidate world leaves world N playable and unchanged, and N+1 is
verified before N is retired. Every road that changes the authoritative world
crosses its own publication verdict. Owner and witnesses:
[construction and reconstitution](engine/construction-and-reconstitution.md).
⛔ Not part of A10: peer-stable identity (ID-PEER), the defensive
`DepartureAuthority::Custodian` fallback, and the refused-door player signal.

### CUTSCENE-ROLLBACK-DECISION — two session-scoped cutscene values cross into simulation with no rollback decision — ✅ DONE 2026-09-28

The cutscene dismiss and skip are seat intents on the `ControlFrame`, which
`tick_active_cutscene` reads from `SlotControls`, and `CutsceneSkipHold` is
rollback state. The host-side `Update` request is deleted. Guards:
`a_cutscene_dismiss_on_the_seats_input_survives_the_rewind_once` and
`a_cutscene_skip_is_a_hold_of_the_seats_cancel`.

### ROLLBACK-KIND-SPELLING — one registration, one kind, spelled once — ✅ DONE 2026-09-16

`ambition_platformer2d_core::rollback_kind::spelling` holds every (kind,
sentence) pair once, and both registration roads reference it. Guard:
`scripts/check_rollback_kind_spelled_once.py`.

### A12 — finish move-contact attribution and reflection identity

✅ DONE 2026-09-19. A late projectile or melee outcome, a reflected shot and an
ability contact cannot credit the wrong move occurrence. An outcome that names
another occurrence, or none, credits no move (ruled: an ability contact is
independent by default). Witnesses: `an_outcome_naming_another_occurrence_credits_no_move`,
`an_unclaimed_outcome_credits_no_move`,
`a_move_occurrence_reaches_the_same_number_with_and_without_a_rewind`. ⛔ The
frontier's `A12` (flow validation) is a different subject.

### ROLLBACK-BAG-DESYNC — `AmbitionGameSave` disagrees with its own rollback replay — ✅ REPAIRED 2026-09-16, acceptance MET; the authority/representation split is DEFERRED

The three live→save mirrors ran in `Update` and wrote a hashed resource that a
replay did not re-derive. They register through `app.sim_schedule()` now. Guards:
`no_hashed_entry_disagrees_with_its_replay_when_the_bag_moves` and
`the_saves_hashed_snapshot_tracks_the_frames_it_is_compared_at`. ⛔ Do not remove
`AmbitionGameSave` from the checksum: most of its writers run inside rewinding
schedules. Deferred: the authority/representation split. Q129 (decided
2026-10-03) keeps shared durable state in what peers compare, by its canonical
semantic form rather than the save's serialization. The bag itself is compared
by its own value (schema 327).

### MENU-RESET-MIDSESSION — the menu writes rollback state from `Update` — CLOSED 2026-09-19

Menu presses reach the simulation through `HostIntentLedger<M>`
(`crates/ambition_platformer2d_actor_monolith/src/session/host_intents.rs`),
released at the head of the stamped tick in every pass. Ruled (`Q140`): one frame of stale UI is acceptable. ⛔ Do not add
duplicate authoritative inventory state or optimistic reconciliation to hide it.
Witness: `a_health_cell_used_from_the_menu_heals_once_and_spends_one_cell`.
