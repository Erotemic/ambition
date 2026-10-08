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

## P0 — architecture and correctness

### BAG-RECORD-HORIZON — a bag record is owned by what is left of its consequence

**Owner:** `items::pickup::minted_horizon` (the grant and spend records),
`session::checkpoint` (the restore's bag fold), the occurrence ledger
(`AuthoredOccurrences::end`, the `Placed` index) and
`features::ecs::pickups` (`ConsumedSinceCheckpoint`). Part of
DEATH-IS-ROOM-LOCAL (Q151). Review of 2026-10-05, findings 1 and 2.

**Finding 1, built 2026-10-05:** a restore kept in the bag what a spared
participant's grant gave and what a kept object's throw spent, and then
forgot both records. A second death of the same checkpoint had nothing to
keep: it took back the coin Alice took in Bob's live room while the coin
stayed gone (money 25 → 0), and put back the javelin quantity while the
javelin stayed in his room (javelins 1, 1, 2). The acceptance now pins the
records it keeps beside the bag (`ItemCheckpointRestoreInputs.grants` and
`.spends`), and the reducer writes them back. An authored grant stays owned
only by its spared owners, as a kept boss defeat shrinks its participants.
Witnesses: `a_second_death_keeps_the_coin_taken_in_another_players_live_room`,
`a_second_death_does_not_put_back_in_the_bag_what_was_thrown_into_another_players_room`.
Poisons, each red on its own arm: forget the grants, forget the spends, no
owner shrink (owners `[0, 1]` for `[1]`).

**Finding 2, built 2026-10-05:** the spend of a throw stood only while its
object was an entity. Measured before the change:

- An object that ENDS (a bomb explodes) kept its `Placed` row, so its room
  built it again when the room was live again, with the bag already spent:
  a duplicate with no death at all. And Alice's death put a bomb that
  exploded in Bob's live room back in the bag.
- An object lying in a room that is NOT LIVE (Bob left) was conserved: the
  restore's pinned ledger has no row for it, so the object goes and the
  quantity comes back. The spend got that answer from the absence of an
  entity, not from the row.

Now an occurrence that a row places in a live room, and that no entity is
any longer, has ended there: its row becomes `Consumed`
(`AuthoredOccurrences::end`, read by `record_ended_occurrences` through a
per-room `Placed` index), and the participants in the room own the ending
(`ConsumedSinceCheckpoint`, as a consumed pickup). Nothing names the item
or the system that ended it. A spend stands when its object is in a room the
restore spares, or when the ledger the restore pins still holds the object's
end. Witnesses:
`a_death_does_not_put_back_in_the_bag_a_bomb_that_exploded_in_another_players_room`,
`an_object_that_ended_is_not_built_again_when_its_room_is_live_again`,
`a_death_takes_back_a_javelin_whose_room_is_not_live_with_its_row`.
Poisons, each red: no `end` (the exploded bomb is lying in its room again,
and the bag gets it back); no owners recorded for the ending (bombs 0, 1, 1
after Bob's room kept the explosion); the spend reads the live object only
(the same).

**Open:**

- A dormant row is not owned by participants. Another participant's death
  takes back a javelin Bob's room held after Bob left it, while a one-time
  pickup he consumed in that room stays consumed (Q151 keeps the second by
  owners). The spend reads the row, so it follows if the ledger comes to
  keep a spared participant's dormant rows. Filed as Q161 (2026-10-06),
  default in force: (a), the current behaviour.

**Built 2026-10-08 (mint-row compaction):** a `Consumed` row of an ended
runtime mint no longer stays in the ledger and the save for the run. The
ledger records mint-ness as provenance BESIDE the row (`AuthoredOccurrences`
`mints`, not a new variant of `OccurrenceWhereabouts`), marked from
`SpawnOrigin::Dynamic` by `record_placed_ground_items` and by `admit_mints`;
`compact_ended_mints_at_checkpoint` drops the `Consumed` row of a marked id
when a checkpoint commits, before `capture_occurrence_baseline`, so only a
row ended before the committed checkpoint goes. An authored `Consumed` (a
taken pickup) and a `Spent` (an opened chest) are never marked and stay. The
mark is rollback state (hashed in `encode_rows`, schema 328) and a save field
(`PersistedOccurrence.mint`, default false, absent from old files). Witness:
`a_bomb_that_exploded_leaves_no_ledger_row_once_a_checkpoint_commits`
(live ledger, pinned baseline and save carry no row; controls: authored
`Consumed` and `Spent` survive; a death afterwards leaves the bag alone), and
the unit tests beside `compact_ended_mints` in `continuity.rs`. The three
bomb/javelin tests stay green. Poisons, each red: compaction never runs; it
drops every `Consumed` row (the authored control goes).

**Built 2026-10-06:**

- An object in a room the restore spares stays where it
  is. Bob takes the hub's gun-sword after Alice's checkpoint. Measured
  before: when he carried it out after her death, the restore took it out of
  his hand and authored it on its pedestal again; when he put it down in
  `duel_arena` before her death, it lay twice (next door and on the pedestal).
  The acceptance now pins, for each object lying in a spared room or held by
  a body there, its live ledger row, and for a held one its holder in the
  pinned custody. An object the checkpoint had in a hand still goes back to
  that hand. When Bob dies in Alice's room with it in his hand, nobody is
  spared: it goes back to the pedestal and his hand is empty. Witness:
  `a_death_with_bob_holding_an_object_in_the_room_keeps_one_copy` (three
  arms); the precedence: `a_death_takes_back_from_bobs_hand_what_alice_banked_in_hers`.
  Poisons, each red: keep nothing (the two copies and the empty hand come
  back); keep the row but not the hand (no copy at all); no precedence (Bob
  keeps what Alice banked). The precedence poison is green on
  `a_death_takes_back_what_was_put_down_in_another_players_room`: when the
  object lies, the custody restore moves it into the banked hand whatever
  the row says, so only a held object needs the precedence.

### SETTINGS-ROLLBACK — finish the settings/mechanics admission boundary

**Owner:** rollback/mechanical-policy owners.

**Current state:** no simulation system reads `UserSettings`
(`scripts/measure_user_settings_in_simulation.py`). The frame-mode half is
closed: `ControlFrame` carries `control_frame_modes`, stamped at capture in
`populate_seat_control_frames`, and GGRS replays it per frame. The damage half is
open: `project_player_damage_policy` writes `PlayerDamagePolicy` from
`UserSettings.gameplay` in `Update`, the policy has no rollback registration, and
three simulation systems read it (`apply_player_hit_events`,
`apply_feature_hit_events`, `charge_projectile_input`). The in-game System
overlay can change `Difficulty`, `Assist` and `PlayerDamage` during a live
timeline, so a resimulation of frame N reads the policy that holds now.

**Ruling (`Q127`, 2026-09-19): deprioritised.** There is no generic
one-dimensional difficulty architecture. Difficulty is game policy expressed as
presets. Participant handicaps and CPU brain levels are separate concepts from
match policy. Spend no substantial effort here until the default game plays
exceptionally well, and do not box the design in.

**Ruling (`Q68`, 2026-10-03):** difficulty, gameplay modifiers and combat
behaviour are game-owned settings; the shell owns audio, display, bindings,
reusable accessibility and localization. `UserSettings.gameplay` is in the shell
crate today.

**Next action (when picked up):** use the `PortalTuning` precedent (the `Q120`
admission protocol). The settings road writes a mirror and proposes in
`MechanicalEditSet::Propose`, and the publisher is the only writer of the
authority. A match-wide policy is admitted at match activation; a
per-participant policy travels with deterministic per-seat input. Do not
reintroduce simulation reads of mutable `UserSettings`. Peers must not have to
share accessibility settings, so input interpretation travels with the input.

**Blocked by:** nothing.

**Acceptance:** rewinding/resimulating frame N observes the policy admitted for
that timeline, not whatever the settings UI contains now; the settings-to-policy
projection remains witnessed end to end; and no `sim`-schedule system takes
either policy resource as a parameter.

### A4 — separate control authority from body execution on the real schedule

**Owner:** accepted control writer map and actor-monolith frontier.

**Current state:** the prerequisite writer census is complete and did not find a
competing control authority. The old `PlatformerRuntimeSet` vocabulary is gone; <!-- cite-ok: the DELETED `PlatformerRuntimeSet` vocabulary, named on purpose: a resolvable citation here would mean the deletion did not happen -->
the real realization places body integration inside
`WorldPrepSet::Integrate`. Prior prose that mapped old and new set names by name
is not an implementation guide.

**Next implementation:** size the packet against the actual schedule seam:
control production, accepted control authority, then body execution/integration.
Keep the measured invariant that a body is advanced once per tick.

**Acceptance:** one accepted control fact feeds one body execution road; no
second body tick or hidden writer is introduced; schedule witnesses are placed
between actual neighboring phases rather than only `.after(...)` an abstract set.

### AUTHORITY-POLISH — one owner per mechanical fact, and no mirror in the rollback kernel

**Owner:** the architecture-completion campaign (C11 in
[`consolidation-plan.md`](consolidation/consolidation-plan.md)). Evidence for
`W0xx` rows stays in [`architecture-warts/README.md`](architecture-warts/README.md)
until the row closes. This row is the execution order. It does not displace
ID-PEER, A4 or the rollback rows.

**The question every item answers:** what owns this semantic fact? Target shape:
authoring/ruleset → preparation → one construction or transition → one canonical
live authority → disposable read models → presentation. Prefer deleting the
field, the writer and the reconciler over a stronger sync.

**Current state (2026-09-27):** steps 1–5 of the review order are done. Step 6
was reassessed: `measure_kernel_module_graph.py --scc` gives the 6-module kernel
(`abilities features items projectile session world`) plus
`assets ↔ character_sprites`, and each kernel leg has a recorded verdict in
AP14. The discovery passes then pointed at game expression. Closed items
(receipts in Git history): AP3, AP8, AP10–AP13, AP15, AP17–AP19, AP28–AP34,
AP36, AP37, AP45, AP48, AP54, AP56–AP58, AP62, AP66–AP71, AP76–AP85, AP93,
AP101, AP102, AP104, AP108, AP109, AP128, AP133, AP137, AP140, AP146–AP148.

**Open items:**

| # | Item | Next step |
|---|---|---|
| AP9 | Stale architecture docs (continuous) | Remove closed wart rows as items land. Keep a one-line receipt only where another row depends on it. |
| AP14 | Semantic actor-monolith SCC decomposition (continuous, Jon 2026-09-24) | The residual kernel is the 6-module SCC. Its legs were walked and judged genuine; the one misplaced leg, `abilities→features` (the puppy-slug gun asks the actor domain to spawn a minion), is not inverted only for the graph. Open question: is the runtime-mint description in `session→items` (`MintedItemBaseline`, `OwnedItemsBaseline`, `ItemCheckpointRestoreInputs`) item knowledge or occurrence-lifecycle knowledge? When an owner is clear, move state, behavior and installation together and delete the old edge (no callbacks, no compatibility re-exports). After each migration run `python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80`. See [`actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md) and [`actor-monolith-work-frontier.md`](engine/actor-monolith-work-frontier.md). |

`WorldTime` is recomputed from `Time.delta` × `ClockState.time_scale` at the
head of each step. That is the canonical clock, not a defect.

**Acceptance for the lane:** no mechanical fact has two mutable canonical
owners; rollback rows are authorities, not projections; construction publishes
no plausible-but-incomplete object; required mechanical policy does not fail
open. Close with a fresh census rather than a checked list.

## P1 — ownership, composition and iteration

### CANDIDATE-GENERATION-ORDER — a candidate session is prepared from the generation before its own activation

**Owner:** [`engine/extension-model.md`](engine/extension-model.md) (content
reload) jointly with the session-lifecycle owner,
`crates/ambition_platformer2d_provider/src/lifecycle.rs`.

**Current shape:** live session A stays on published generation N. Candidate B
is built and verified from transaction-local N+1, which
`PendingGenerationInputs` (`ambition_platformer2d_runtime/src/content_identity.rs`)
hands to the transaction's own preparation, keyed by its `load_id` claim.
Adoption makes B/N+1 authoritative atomically. Construction runs
`.before(AmbitionGameShellSet::Pending)` and the content commit runs after it, so
the channel, not an ordering edge, is how a candidate sees N+1. The channel
carries the admitted cast (`characters`), the boss catalog (`bosses`), the
character catalog (`catalog`) and the audio catalog (`audio`). The fighter ladder and the encounter waves are
standing projections that converge after the commit, so they are not frozen.
Guard: `the_candidate_is_built_before_the_router_advances_and_providers_only_adopts`.

**Next action:** none open for the channel. A boss's numbers already reach the candidate. Construction
seeds each boss from the frozen N+1 catalog (`SessionMechanics::bosses`), and
since 2026-10-03 `BossConfig::seed` is required, so no boss reads the App
catalog at construction or on its first tick. Witness:
`a_boss_tuning_saved_while_the_game_runs_is_played`. Read from source
2026-10-03, not measured by a test: the other inputs preparation reads from the
App cannot differ between N and N+1.
- `ReloadRequest` refuses a change to `sheets` (`AuthoredSheets`), and its
  only writer is `register_character_sheet_ron` at plugin build.
- `forced_brains`, `population_cap` and `perception_extent` are immutable
  developer knobs, read once at build.
- Audio was the exception, and is now closed (2026-10-07). Its domains take
  part in a reload, and the transaction holds the N+1 `AudioCatalogRegistry`
  until the commit, while preparation used to read the App's (N), only for
  provider presence: `validate`'s `has_provider`, and `music_ready` /
  `procedural_sfx_ready`. The channel now carries `audio`
  (`PendingGenerationInputs::audio_for(load_id)`), and the lifecycle resolves
  it through `candidate_audio_for`, falling back to the App's registry only
  when no claim exists for that `load_id`. Witness:
  `a_candidate_that_drops_a_providers_audio_is_refused_and_the_live_audio_survives`
  (control: `a_candidate_that_edits_a_providers_audio_activates_and_publishes_it`)
  in `an_edit_reaches_the_shipped_game.rs`, which drops the
  SFX rows from N+1 and expects the refusal; reading `self.audio_catalogs`
  makes it fail. A music-fragment drop cannot be built: the pack compiler
  refuses it first, because bosses reference music tracks. Adaptive-cue
  presence was the next consumer and is on the channel too
  (`adaptive_providers`; measured: a pack without its cue file compiles).

⛔ Not by an ordering edge and not by re-fingerprinting. Do not reopen A10.5's
guarantee that a candidate that cannot be built never retires the live session.
This is engineering, not a maintainer question.

**Blocked by:** nothing.

**Acceptance:** every generation input a candidate must see at N+1 reaches it
through the transaction-local channel rather than through global publication,
and a value that is a standing projection is stated as one rather than frozen.

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
- I4: save eligibility; ports for body motion so the remaining wielded items (dive, blink, grapple, mark/recall) can become modules; GNU-ton's conductor as a module.

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
  are exact titles now. Open: the ids still come from `yarn_title_ids`, a
  `title:` line scan beside the Yarn compiler. The compiler is an optional
  dependency (`ui`), so replacing the scan is a dependency decision.
- LDtk/world cross-reference rules in `content_validation.rs` repeat rules that
  a world owner already checks. ✅ 2026-10-03, the LoadingZone target rule:
  `validate_ldtk_room_links` refused every zone without both targets, which
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
  LDtk entities for a quest target now; `authored_flag_ids`,
  `authored_npc_ids`, `authored_pickup_ids` and `authored_entity_iids` are
  deleted. <!-- cite-ok: records deleted functions -->
  Open: `QuestStepCondition::ItemCollected` has no producer of its event in
  the tree (a search for `ItemCollected` over `.rs`, `.ron` and `.yarn`), and
  no shipped quest uses it. The cutscene bindings still read `active_area_ids`
  from the project; that is the level-by-level read the two-per-room rule
  needs.
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
  (`Platformer2dSimulationPhaseMonolith` is now `Platformer2dSimulationPhase`; 378 uses,
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

**Open:** the claim is *not installed*, not *not linked* (`Q106`: the crates behind
these capabilities are unconditional dependencies); content that requires an omitted
capability refusing at admission is not witnessed; re-entry is not exercised;
`Cutscenes` is a capability but in no profile yet.

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

1. **A brain cannot decline to attack.** The decision takes `options.attacks.first()` whenever the body is free, so the move that survives at range is thrown until the world changes. The brain does not remember its own last move. A decline gate on decision ticks was built and was a no-op, because presses are bounded by move duration (24 ticks), not the decision cadence (5). Next: a refusal that lasts as long as the move would have (`frames.total_s`), which needs one more piece of brain state and its snapshot projection.
2. **A counter and a buff cannot be chosen between.** Pricing them needs a defensive feature ("is the opponent committed to a swing"), not a wider admission rule.
3. **`sanic` @5 leaves the stage early** (about 640–780 ticks, both seats airborne). That is a stage defect, not the one-move lock. The guard for it is a bout that ends early, not move variety.
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

**Next implementation:** search for a new duplicated authored/runtime value
with the owner document's two questions (A1). For each one found, choose one
authoring owner and make every runtime representation a projection or admitted
prepared value. Prefer deleting the second truth to synchronizing it. Exit
criterion 5 (another experience consumes a character without irrelevant
facets) is not yet measured.

**Acceptance:** the owner document can name one authoritative authored value for
each migrated fact, and production consumers cannot bypass its preparation or
projection boundary.

### BAKED-SHEET-IDENTITY — a body's collision box is outside the content identity

**Owner:** `ambition_sprite_sheet` (the baked sheet index) and
`ambition_platformer2d_provider` (`MechanicalRegistries`).

**Current failure (read 2026-10-05, not poisoned):** a sprite-authored body
takes its collision box, its per-pose boxes and its attack polygons from the
baked sheet index (`record_for_sheet_key`: `sprite_body_collision_for_sheet`,
`posed_body_geometry`). The index is `BAKED_SHEET_RONS`, compiled in and held
behind a process `OnceLock`. No content-identity section reads it:
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
`AuthoredFighterLadder` (`the_ladder_the_demo_runs.rs` pins the floor).
`ambition_content` `plugin.rs` and `reload.rs` (`publish_fighter_ladder`)
are the only installers. A Smash pack source may name a file outside its
root, as George's facet does.

### MIRROR-SYMMETRY — mirrored CPUs stay mirrored per tick

**Owner:** `ambition_combat::brain` and the systems it reads. Plan:
[`engine/fighter-brain.md`](engine/fighter-brain.md#mirror-symmetry-is-a-correctness-property-q49).

**Ruling:** Q49 (2026-10-04): symmetry is a correctness property; variation
comes only from modelled asymmetric facts.

**Current failure:** the Emmy test compares positions only, accepts a break at
the first grab, and asks only 1.5x the ordinary rate; known asymmetry sources
(the 69% seat-0 term) is untriaged; the left-first recovery search is
triaged (no decision reads the order). The
zero-lateral `signum` sites in `rollout.rs` were a defect, fixed 2026-10-08
(`the_shadow_of_a_reflected_scene_is_the_reflected_shadow`). The two `SimId` tie-breaks are triaged
(plan item 4, 2026-10-08): the grab tie is an authored rule with a fixture,
and the target tie is not reachable with one foe. The decision
layer has its reflection test (plan item 3, 2026-10-08), and it found no
defect.

**Acceptance:** a per-tick reflection test of position, velocity, facing,
move, decision and stream position; a reflected-observation unit test of the
decision layer; each poison listed in the plan turns one of them red.

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

### LDTK-SEMANTIC-DIFF — review content diffs by meaning, and find the rewrite

**Owner:** `tools/ambition_ldtk_tools` (`edit/semantic_diff.py`). Plan:
[`engine/authoring-and-tools.md`](engine/authoring-and-tools.md#content-diffs-need-domain-aware-comparison-q62-q78).

**Ruling:** Q62 and Q78 (2026-10-04) are engineering and evidence tasks, not
maintainer choices.

**Current state:** the comparison is done: `diff semantic` (file or
`REV:PATH`), `diff range` and `diff normalize`, with verdict, per-level
summary, ambiguities and a noise section (tests:
`tests/test_semantic_diff.py`). It prints the per-level summary for the
post-fix rewrite commits `576a8fd`, `c6df2b7` and `056079f`. `48f8e26` is not
in this clone, so `48f8e26 → cb7062a` is not re-measured. LDtk files still
rewrite far beyond their edits after `54d99e7fb` and `2e69e81b9`; the writer
is not identified.

**Next action:** run each LDtk writer twice on an unchanged input and compare
the two outputs with `diff semantic`; the writer whose second run is not
`identical` is a rewriter.

**Acceptance:** a second run of each writer on its own output changes no
byte.

### ASSET-PRODUCT-LAYOUT — runtime roots by tier; editor products apart

**Owner:** `ambition_asset_manager` path builders, `ambition_sprite_sheet`,
the publish scripts. Plan:
[`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md#products-are-laid-out-by-what-they-are-q82-q83).

**Ruling:** Q82, Q83 (2026-10-04).

**Current state:** full quality is the bare `sprites/` root and the reduced
tiers are suffixed siblings; the packager ships every file, including the
98.8% of the 449 MB ultrapack that no runtime road reaches.

**Next action:** audit the listed path builders, then move to
`<root>/{full,half,quarter,potato}/` in one change; then cut the ultrapack's
runtime product by runtime unit.

**Acceptance:** every tier is a named directory; the packager selects roots
and has no new exclusion; the packaged ultrapack holds only reachable pages;
the game draws the same at each tier.

### PORTRAIT-TIERS — portraits scale with quality and stay usable

**Owner:** `ambition_sprite_sheet` (`bake_portrait_manifests`) and the dialog
portrait consumer. Plan:
[`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md#quality-is-a-presentation-policy-q84).

**Ruling:** Q84 (2026-10-04).

**Current state:** reduced portrait tiers are generated and shipped, and
nothing loads them.

**Acceptance:** each quality tier loads its own portrait product, sized from
the dialog draw size; potato portraits are smaller than full and still fill
their box.

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

**Open items:**

1. **The compile-cost ratchet fails the full gate** (`scripts/compile_ratchet.py`, measured 2026-09-18). Its baseline records commit `b3bd00a4a` (2026-09-05), which no ref reaches, and it disagrees with itself in three places. Over budget: `ambition_platformer2d_actor_monolith`'s largest unit (100,742 → 115,105 lines) and edit cost, and `ambition_geometry`'s worst edit cost (94.9% of the workspace). ⛔ Do not re-freeze to go green. Next: find which part of the monolith's largest unit belongs in its own crate, and repair the baseline's self-disagreement before any deliberate re-freeze. <!-- cite-ok: `b3bd00a4a` is quoted BECAUSE it resolves nowhere; it is `dev/compile_ratchet_baseline.json`'s own recorded `commit` field -->
2. **An arm fails only in company** (see [the triage page](triage/a-composition-acceptance-that-only-fails-in-company.md)): `composes_through_the_sdk::a_host_that_omits_boss_encounters_still_builds_and_steps` failed once on 2026-09-10, and its assertion was never captured. Three other instances of the signature were per-arm measurements reading process-global state (`app_it` runs arms as threads of one process); they are fixed, and `scripts/a_test_static_is_a_channel_between_arms.py` guards the class. Next: capture this arm's assertion. The next candidate of the class is `hall_redecode_census.rs`, which asserts over a delta of a process-wide counter. ⛔ Do not add a retry.
3. **One older session-root handoff failure** did not reproduce in four full runs, and its assertion was never captured. Both candidate arms (`the_shipped_app_never_holds_two_session_roots_across_a_handoff`, `a_candidate_session_replaced_while_pending_is_discarded`) assert their own premises, so a new failure carries its cause. The next step is not more runs.

4. **`NOT RUN` is a first-class receipt state** (Q59 ruling, 2026-10-03). A
   ledger or receipt must tell PASS, FAIL, "not run, not required now" and "not
   run, required at this boundary" apart; a gate blocks only where its policy
   requires it at the current boundary. Next: find the receipts that collapse a
   lane that did not run into PASS or FAIL, and give each lane its cadence in
   [testing and validation](../concepts/testing-and-validation.md#validation-states-and-cadence).

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
   the row. Open: nothing runs the lane for you. It is a rule in prose, and a
   push that skips it is not refused.

The published-sheet floor in `ambition_sprite_sheet` (780 below a floor of 800
on one checkout) is machine state. ⛔ Do not lower the floor.

**Acceptance:** the failing population is reproducible or explicitly classified,
and the production cause is fixed or the harness proves why the failure is not a
production invariant.

## Receipts

Closed rows that an open row, a script or an inbound link still names.

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

### CHARGE-SPEC-NAME — `SmashChargeSpec` is a generic mechanism — ✅ DONE 2026-10-08

Q44: no leaf-game name on a generic API. `SmashChargeSpec` is
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
