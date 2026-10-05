# Open-world runtime and residency

**State:** OPEN. A session can hold more than one live room, and the shipped
app simulates and draws each one. Two live rooms exist when two seats' driven
bodies are in different rooms. Ambition has no production join road for a
second seat yet (Q153). The customers are the maintainer's persistent systemic platformer and
separated multiplayer (Alice and Bob in different rooms). [The
queue](../queue.md) selects execution. This page owns residency and activity
semantics. Construction, custody, spatial facts and rollback keep their owners.

## Goal

Support a persistent world larger than its live simulation. An actor, item,
encounter or quest must not stop existing because a camera moved. Human, AI,
possessed and uncontrolled actors use the same body and domain rules. A
one-room profile and a multi-room profile use one implementation at different
populations.

Keep these axes separate:

```text
persistent existence != prepared data != live ECS population
                     != simulation activity != local view visibility
```

Rooms are the residency unit. Sub-room chunks or region hierarchies need a
measured consumer. Do not build a universal partition tree.

## Current shape

A session holds one or more **live rooms**. Each live room is one entity.

| Fact | Where it lives |
| --- | --- |
| The prepared room graph and the minting counter | `RoomSet` on the session root (`activation`, `next_live_room`) |
| One live room | A `RoomInstanceRoot` entity with `LiveRoomInstance` (an ordinal), `LiveRoomDefinition`, `RoomGeometry`, `MovingPlatformSet` and `FeatureEcsWorldOverlay` |
| Which live room an entity is in | `InRoomInstance(LiveRoomInstance)` on the entity, stamped by `SessionSpawnScope::apply_to` |
| One live occurrence of a body | `LiveBodyId` (`SimId` plus live room), resolved by `LiveBodies` |
| The rule "which room is this entity in" | `LiveRooms::of` (its stamp, else the sole live room); `live_room_of` for identity (its stamp only) |
| Per-room reads | `LiveRoomOf<T>`, `LiveRoomSpecs` (`definition_in`, `left_by`), `CollisionWorld::room(..)`, `RulesOf<T>` |
| Ambient gravity | `BaseGravity`: the turned live rooms only, keyed by `Option<LiveRoomInstance>`. A switch turns its own room; a body reads its own room's (else the sole room's); a replay forgets its room; a crossing that leaves a room standing forgets the room left when it retires. Gravity and force zones carry their room too (`zone_acts_in`) |
| Music | `EncounterMusicRequest` keeps its two tiers per live room: a boss, a script, a wave, the cut-rope intro and Mary-O's beats claim the tier of their own room. `compute_music_intent` plays for one room (`the_room_the_music_plays_for`): of the participants' rooms, the one with the highest `priority_of` (boss over encounter over the room's own music), the primary seat's on a tie (Q150, Q72). That room's music and its fight are heard, and a conversation's track is released when that room changes to another authored room. The developer's gravity cycle turns the primary seat's room (`PrimaryLiveRoom`). An authored per-candidate priority is [open work](#open-work) |
| A recall mark | `PlayerMark` keeps the live room it was dropped in: a recall from any other room does nothing, and its beacon is drawn in its own room. Decided 2026-10-02 (a position names a place only with its room); before, a recall after a crossing moved the body to the old coordinates in the new room |
| Encounter camera zoom | `EncounterView` keeps the zoom per live room (`set_camera_zooms`, `camera_zoom_in`); each view reads the zoom of the room it frames |
| The one-live-room read (named debt) | `SoleLiveRoom<T>`, `SoleLiveRoomSpec`, `RoomOverlays::sole()` (`SoleLiveRoomMut` is deleted) |

`InRoomInstance` is a value, not an `Entity`, so it snapshots without entity
mapping. The ordinal is never durable. The save names places by room id.

### Crossings

A crossing publication carries a `LiveRoomSuccession`, chosen by
`for_crossing`:

- `Replace { replaces, mints }`: nobody else holds the room it leaves.
- `Open { leaves, mints }`: another seat's driven body stays in the room it
  leaves (`another_player_stays`). A new live room root is built.
- `Join { leaves, joins, retires }`: a live room of the target room holds
  another seat's body. Nothing is built or minted. The body and its custody
  closure (`InCustodyOf`) move into the joined room.

`RoomTransitionIntent` records the subject (`LiveBodyId`) and the driving
participant when the crossing is accepted, not at commit. The verifier refuses
a stale publication before anything is torn down (`StaleRoomInstance`,
`StaleMint`, `StaleJoinedRoom`). A room transaction's world is the pair of live
rooms it touches (`TransactionRooms`), so another live room's occupants are
never superseded. The GGRS carrier order and its census key by
(`SimId`, live room).

A crossing resets only what it leaves behind (`CrossingScope`). A crossing,
a dialogue, a replay or a hazard respawn stops or resets session-wide state
(the clock, gravity, `GameMode`) only when no other live room is in play.
`RoomTransitionCooldown` is one countdown per seat.

### Residency claims (OW4)

A live room is held by each driven body stamped into it.
`rooms::residency::claims_on` is the one rule, and `live_room_claims` gives
the answer for every room. `[census] rooms` prints it (`holders=[#0:slot1]`).
A claim is derived from bodies and stamps, not stored, so a departed body's
claim is gone with it. Views and pending transitions hold no claim yet. No
budget is stored, because a budget with no consumer is not a policy.

### Dormant records (OW3)

- A runtime mint (`SpawnOrigin::Dynamic`) enters the occurrence ledger where it
  lies, in the tick it appears (`AuthoredOccurrences::admit_mints`). A mint
  left in a retired room is rebuilt when the room is live again, and across a
  save.
- A persistent (`DeadStaysDead`) authored body released in another room gets a
  `Placed { room, at }` row (`record_placed_bodies`). Its home room does not
  build it. The room it lies in builds it there.
- A respawning population body gets no row. While it lives in a room another
  player holds, its home room does not build it a second time
  (`keeps_durable_whereabouts`). Its replacement comes from its authored room
  (Q38 ruling).
- An enemy death and an encounter outcome persist. A living enemy's HP and an
  encounter's wave index do not, so a returned room is fresh. Q149 (2026-10-03):
  keeping wounds is an opt-in actor policy for actors whose continuity
  matters; an ordinary respawning enemy is rebuilt fresh. No actor opts in yet.
- Dormant records add no all-world walk to an idle tick: the custody projection
  reads a custody index, and the save mirrors walk dormant rows only when an
  input changed (FI9).
- A room has at most one live room, so the durable rows can name a place by
  its room id (`outlook_for(room)`, `WorldTimeSchedule`). The
  publication verifier refuses an open or a replace into a room another live
  room already is (`DefinitionAlreadyLive`). Measured 2026-10-02 before the
  refusal: no shipped road reached it. A crossing into a held room joins it,
  a replay or a checkpoint reset beside another player rebuilds the one live
  room with both players in it, and none of the 72 rooms has a door into
  itself. Q109 (2026-10-03): the room occurrence is a scope beside `SimId`,
  never part of it, so an instanced copy of a room names its bodies by
  `(LiveRoomInstance, SimId)`; lifting `DefinitionAlreadyLive` also needs the
  durable rows above to stop naming a place by room id alone.
- A rollback frame does not copy or hash unchanged dormant rows. The save's
  rows, the ledger and `WorldTimeSchedule` are `Arc`-shared with
  checksums kept per allocation (M2; the schedule 2026-10-02, measured 2.0 ms
  per snapshot at 10,000 records before). Census of the readers that walk
  dormant rows: the custody projection reads an index, the save mirror walks
  only when its key changed, and the rest run at a commit or a restore.

### Logical time while a room is not live (OW5)

Two mechanisms keep time while their room is not live, through one schedule:
a broken breakable's respawn and a collected pickup's regrowth (Q152). Each is
due on `GameplayElapsed`, the session's sum of scaled simulation dt.
`WorldTimeSchedule` keeps the due time by (room definition id, authored id).
While the room is live, the occurrence's live `RespawnTimer` is the authority,
and `mirror_breakable_respawns` and `regrow_pickups` keep the record. When the
room retires, the record stays and nothing ticks it. Construction builds an
occurrence that is not yet due as gone (a breakable broken, a pickup
collected), with the time that remains. A replay, a checkpoint restore and
session teardown forget the records. A pickup authors its regrowth as a
breakable authors its respawn (`respawn: AfterSeconds` with
`respawn_seconds`, one parser). Q152 (2026-10-03) sets the order: breakable
respawn, regrowth/restocking (regrowth done 2026-10-04; shop restocking has
no stock to refill, because a shop sells without a count), then scheduled
persistent characters.

### The view half

Each player sees their own live room:

- The camera resolve frames each view in the live room of the body it frames
  (V1). `camera_follow` places a view by that room's geometry (V2a).
- While two or more rooms are live, `isolate_live_rooms` puts each stamped
  entity and its descendants on its room's render band
  (`LIVE_ROOM_RENDER_LAYER_BASE` + the room's place in instance order). Each
  main camera adds the band of its view's room (V3).
- Static room visuals, LDtk levels and parallax are presented per live room
  and retire with it (`present_live_room_visuals`, V4).
- While the seats' driven bodies are in two or more live rooms, each seat gets
  a view (`split_views_by_live_room`, `present_split_view_rigs`, V5). The
  views close when the seats meet.
- Draw roads place each entity by its own room's geometry (V2). Sprites,
  items, projectiles, lock walls, nameplates, effects, body-riding visuals,
  zones, shrines, the blink ring, broken blocks, health bars, world labels,
  portals and the through-portal capture are per room.
- Every effect producer names a room. `VfxWriter::write` is deleted; a
  producer writes `write_in(room, …)`, and helpers take `&mut VfxForRoom`.
  `FxRequest::new(room, …)` and `FireworksRequest::around(room, …)` take the
  room. Each producer names the room of its subject by `LiveRooms::of`.
- `DebrisBurstMessage` carries the room of its producer's effects. The debris
  reader (`physics_spawn_debris_messages`, built only with `physics_debris`)
  places each burst by that room's geometry and stamps each piece, so the
  room retires its own debris.
- Live rooms share one coordinate space, so interaction compares rooms: a body
  opens a chest, talks to an NPC, and collapses a breakable only in its own
  live room.
- Gravity and force zones act in their own live room. Each `GravityZones` and
  `ForceZones` row carries its room; `FrameEnv::resolve` and
  `GravityCtx::dir_for/dir_at` take the body's room. `gravity::zone_acts_in`
  separates a zone and a body only when both rooms are known and differ, so a
  one-room game is unchanged.
- The demo readers (Sanic rings, monitors, milestone and act clear; Mary-O
  bricks, power blocks, flag, pipes, title card and hidden blocks; Smash
  respawn platforms; the cut-rope props and music release) read their own
  subject's live room or every live room.

The banner is still one per session and follows the primary seat. The Q150
ruling (2026-10-03) makes the HUD per participant, also on one merged screen,
and the music an authored-priority choice across local participants with the
primary participant as the tie-break. The music is built, and so is the
built-in vitals HUD per view: see the open work below for the rest.

## Ownership table

| Responsibility | Owner | Required output |
| --- | --- | --- |
| Authored room/region definition | World provider and preparation | Immutable definition and stable references |
| Live spatial instance | Spatial/world authority | Scoped geometry, membership and semantic entity resolution |
| Need for a room to be active | Gameplay/lifecycle policy using declared interests | Deterministic required set and reasons |
| Read/prepare/prefetch work | Existing source/load coordinator | Candidate data with identity and readiness |
| Construction and publication | Existing lifecycle plus typed domain constructors | One admitted live population and timeline boundary |
| Actor/item/quest durable facts | Existing corresponding domain owners | Pinned revision or accepted state handoff |
| Visual residency and quality | Asset/presentation owners | Device readiness independent of simulation truth |
| Rollback population/history | Existing registrar and GGRS host | The admitted active state under one generation |

Do not centralize these facts in a new `WorldContext` resource. A coordinator
combines owner results. It does not become a second item ledger, physics
engine, checkpoint router or query system.

## Identity and coordinates

A room definition can be instantiated more than once. A live query identifies
the instance whose geometry and occurrences it interprets. A construction
attempt and a `ContentEpoch` are not durable occurrence identities. A view ID is
never the identity of the simulated room it shows.

Where an occurrence must survive save/load, the world owner supplies its
stable location identity (room id, never the ordinal). Placement, runtime
spawn, custody and definition IDs keep their distinct meaning.

Spatial requests name units, coordinate frame and instance. A cross-instance
operation requires an installed transform or transfer service. Copying
coordinates or querying "the" live room is not that service. A portal may map
spaces. A camera looking through it does not transfer simulation ownership.

## Activity policy and deterministic inputs

The simulation-required set follows gameplay: controlled bodies, unresolved
contacts and projectiles, active encounters and declared cross-room mechanisms.
It does not follow local cameras, rendering quality, memory pressure or IO
completion order. Optional prefetch and visual detail can be best-effort.

The background policy is explicit suspension. A mechanism that needs time
evolution declares a supported rule: a scheduled logical event, deterministic
elapsed-time reconstruction, or full active simulation. Do not invent
approximate combat or resource production when a room leaves view.

Event-driven processing uses an admitted logical clock, input facts and wake
conditions. Persist and rewind its counters and next-event state at the owning
lifetime. Do not ship a second physics solver for dormant rooms.

## Promotion and demotion

Use the existing prepare/admit/commit road. At a confirmed boundary:

1. Pin content generation, source instance, target instance and the required
   durable revisions.
2. Prepare the destination population through owning constructors. Resolve
   retained bodies, custody and relationship endpoints without duplicates.
3. Obtain lifecycle authorization. Partial asset loading may affect reveal,
   never collision truth.
4. Transfer write authority once. Demotion writes the settled disposition
   through its domain's accepted policy.
5. Establish the rollback baseline before simulation resumes. Retire only the
   departed instance, not every room with the same definition.

Failure before publication keeps previous ownership. Records that influence
future ticks join the rollback model or a pinned deterministic input revision.

The room transition rebases the timeline at session scope. Independent
per-room rollback clocks are not part of this plan.

## Open work

| Item | Work | Acceptance |
| --- | --- | --- |
| Sole-room readers | Key the remaining `SoleLiveRoom*` readers by subject. Classified 2026-10-04 (production uses only). Done: a restore's verification (`verify_restored_domains`) asks for the live room standing in the room it names (`live_room_standing_in`), so Alice's restore beside Bob's live room is verified, where before the room and subject checks read the sole live room and checked nothing (`with_two_live_rooms_a_restore_is_verified_against_the_room_it_names`). Open, simulation side: `restore_checkpoint_on_session_start` (session start has one room), `rebuild_ldtk_runtime_spine_index` (one `active_area`; its readers are debug and headless reports. Read 2026-10-04: it is not keyed by the primary seat as the developer overlay is, because it is written in the simulation schedule and the primary seat differs between peers, so that would be state two peers disagree on. If a reader needs two rooms, move the rebuild to `Update` beside the overlay first), and the gravity switch's second `room: None` fallback in `drive_wave_encounters` (production writes the switch's room through `LiveRooms::of`, so only fixtures reach it). Done 2026-10-04: the trail draw places each trail by its own body's live room (`trail_strips`, `each_trail_is_drawn_in_its_own_live_room`; it drew nothing while two rooms were live). Done 2026-10-04: the parallax retirement keeps each live room's theme and its neighbours' (`parallax_keep_set`, `every_live_room_keeps_its_theme_and_its_neighbours`; it did not run while two rooms were live, so no theme retired). Done 2026-10-04: each `FeatureView` names the live room of its feature (`room`, by `LiveRooms::of`; `each_row_names_the_live_room_of_its_feature`), the unclaimed stand-in is placed by that room's geometry and stamped with it (`each_unclaimed_view_stands_in_its_own_live_room`; none was drawn while two rooms were live), and the engine debug viz draws the primary seat's room with only that room's features and combat rows (`the_debug_viz_draws_the_primary_bodys_room_while_two_rooms_are_live`; it drew nothing while two rooms were live). Still open there: `FeatureViewIndex` is keyed by the bare `FeatureId`, so two live instances of one definition would fold into one view (a crossing joins a room and does not make a second instance). Done 2026-10-04: the neighbour prefetch prepares the neighbours of each live room (`each_live_room_keeps_the_plans_of_its_own_neighbours`, on the presentation host: Alice and Bob cross from two live rooms in the two orders, each crossing is a prefetch hit, and the plans of a room that is no longer live are retired). Before, it read the sole live room, so with two live rooms it did not run; Alice's second crossing missed in the two orders, and Bob's crossing hit only when it came first, from plans left from the time of one room. The plans are keyed source room, then target room, under one world (content epoch and session): a new world clears all, a new source room clears nothing, and the producer retires the entries of a room that is not live. The rule is one type, `PrefetchedByRoom`, held by `RoomConstructionPlanPrefetch` and by `RoomPreparationPrefetchState`. The budget of 4 rooms is the total, dealt in turn to the live rooms (`neighbour_prefetch_set`, `the_prefetch_budget_is_dealt_to_the_live_rooms_in_turn`); one live room gets the 4 it got before. A neighbour that is live gets no plan and uses no budget, because a crossing into a live room joins it and builds no room (`a_live_neighbour_gets_no_prefetched_plan`: with two live rooms that are neighbours of each other, two of the four rooms of the budget were the live rooms). Not changed: the transaction of a join still prepares a plan of its own when it begins, and the commit does not use it. Done 2026-10-04: the map, the developer overlay with its label renderer, and the developer text HUD read the room of the primary seat (`PrimaryLiveRoomSpec`, which is `PrimaryLiveRoom` with its definition: the primary body's room, and with no primary body the sole one). Each did not run while two rooms were live: the map's active room stood still, the overlay drew nothing, and the HUD showed the room it showed last (`the_map_shows_the_primary_bodys_room_while_two_rooms_are_live`, `the_developer_overlay_and_hud_show_the_primary_bodys_room_while_two_rooms_are_live`; the player's vitals HUD is per participant, see the Q150 row). Sole by design, with the reason in place: `settle_versus_round` and `prepare_the_match` (a match is one stage: all its seats are in one room, so with two live rooms there is no match to prepare or settle), the demo setups at activation, tool binaries, the test harness | Each runs per room while two rooms are live; one-room control unchanged. `check_alias_census_agrees_with_source.py` holds the count |
| Remaining one-room state | Done for per-attempt state: `AttemptScoped` holds a `PerLiveRoom` map, so `BrokenBricks`, `SpentPowerBlocks` and `SpentMonitors` are keyed by `LiveRoomInstance`, each overlay subtracts its own room's names, and `rearm_attempt_scoped` drops the state of a room that is no longer live (a replay seats a new instance, so it starts whole; another participant's room keeps its state). A session activation forgets all of it (`forget_attempts_on_activation`, registered by the same `install_attempt_scoped`), because each session's first room has the key `LiveRoomInstance(0)` again (`a_block_broken_in_one_session_is_whole_in_the_next`). `mary_o_setup` and `sanic_setup` run at activation, which has one room. Done 2026-10-03 for the breakers: the brick, ?-block and monitor breakers act for every body of the player population (`PlayerEntity`), each in its own live room, in stable `SimId` order, so when two seats strike one block in one tick the first seat by `SimId` is paid (`a_second_seats_strike_breaks_in_its_own_room`, `a_second_seats_bonk_spends_and_pays_in_its_own_room`, `a_second_seat_breaks_the_monitor_of_its_own_room`, `two_seats_bonking_one_block_pay_the_first_seat_by_sim_id` and `two_seats_on_one_monitor_pay_the_first_seat_by_sim_id`) | A second seat's strike breaks in its own room (met) |
| Debris physics | Decided and served: two colliders meet only when they are in one live room. Avian has one space and the live rooms overlap in it, so `OneRoomContacts` (`world/physics.rs`, Avian `CollisionHooks::filter_pairs`) refuses a contact between two different room stamps; a collider with no stamp meets all. Debris carries `ActiveCollisionHooks::FILTER_PAIRS`. Witnesses: `debris_of_two_live_rooms_does_not_meet` (two rooms stay apart, one room pushes apart) and `each_debris_burst_is_thrown_in_the_live_room_its_message_names` (production debris carries the hook). The `physics_debris` feature is in the monolith default (`desktop_dev` → `visible`), so both run in the default lane. Decided and done 2026-10-04: debris lands on the floor of its own room. The adapter's `spawn_static_collider_for_block` had had no caller since the git epoch and the render twin was a stub (deleted, with the settings parameter it threaded through the room visuals), so every piece fell out of the room. The first debris burst in a live room now builds that room's floor (`PhysicsRoomFloor`, one static collider per support block, stamped with the room), so the room retires its floor and `OneRoomContacts` keeps another room's debris off it. It is built on the first burst, not when the room goes live, and avian pauses on debris presence (it paused on any rigid body), so a room with no debris does not pay for the physics engine (`debris_lands_on_the_floor_of_its_own_live_room`: the pieces rest on the floor a second after the burst, one floor for two bursts, and avian pauses when the debris is gone; control: a room with no floor, whose pieces fall) | Debris lands on its room's floor (met) |
| Replay reset effect | Measured 2026-10-02 (a one-off probe of `VfxInRoom`, Alice's manual replay in the hub, with one live room and with Bob's room live beside it): `ResetEffects` is written twice, on the admission tick for the room being replaced (#1) and on the next tick for the new instance (#2), where she stands. They are two effects from two writers: #1 is the replay's from→to trail (`sandbox_reset::reset_sandbox`, `for_room(subject.room)`, in the old room's coordinates), and #2 is the crossing commit's arrival effect (`room_transition/commit.rs`, `for_room(arrival_room)`, `from == to`). So the player sees the arrival effect in her room, and the trail goes to the room being replaced. The #1 write goes to a room that retires a tick later. Its particles are stamped into that room and are not room-scoped, so they live on for their lifetime. Read 2026-10-04: the room-band pass put an entity of a room that is no longer live back on the world layer, so every camera drew it, and Bob saw Alice's trail in his own room (the isolation test under the poison: Bob's camera drew the trail and its child). Done 2026-10-04: while two or more rooms are live, such an entity draws on `RETIRED_ROOM_RENDER_LAYER`, which no camera draws (`an_entity_of_a_room_that_is_no_longer_live_is_drawn_by_no_camera`; with one live room it is on the world layer as before). Not measured: whether a rendered composition spawns the #1 particles at all (the renderer skips a message whose room has no geometry when it reads it); either way no camera draws them | Met for the drawing. Decided 2026-10-04: both writes stay. Only the replay knows where the body was (#1 is the trail from there to the spawn), and #2 is the arrival effect every door writes, so neither can stand for the other; with two live rooms no camera draws #1, and with one room it is drawn over the rebuilt room, as before |
| Join road | Ambition has no production road that seats a second player (Q153). A join road must stamp its body into the room it joins. A rebuild of a room replaces a seat's body that is a placement of that room, as the fixtures' Bob is, so the join road must also give the body a home that a rebuild does not replace (possession uses custody) | A second seat's body keeps its room live in ordinary play |
| Death horizon (Q151) | A participant's ordinary death rewinds that participant and the affected room, and only the durable records that horizon covers; another participant's live room and its consequences (a boss defeat, its reward) stay. An explicit whole-session reload may rewind the whole session; no reconciliation of surviving rooms. Done for a death: the boss defeat, its reward, a breakable's respawn and Alice's custody across Bob's room each have a witness, and a defeat Bob won in a room he has since left stays (a defeat record names the participants in its room when it fell; `a_death_keeps_the_defeat_another_player_won_in_a_room_he_left`). Since 2026-10-04 each consequence names the participants whose horizons own it, and a restore takes out the dying one: a breakable's respawn and a one-time pickup Bob took in a room he left stay (`a_death_keeps_the_respawn_of_a_platform_another_player_broke_in_a_room_he_left`, `a_death_keeps_gone_a_one_time_heart_another_player_took_in_a_room_he_left`), and an item a kept reward gave stays in the bag (`a_death_keeps_the_item_taken_from_the_other_players_boss`), and so does what a placed pickup or an ordinary chest gave in a room another participant was in (`a_death_keeps_the_coin_taken_in_another_players_live_room`); queue row DEATH-IS-ROOM-LOCAL has the rest. A New Game retires every other live room in its commit (2026-10-04, `a_new_game_leaves_no_live_room_holding_what_it_took_back`) | Alice's death beside Bob's boss defeat leaves Bob's room, the boss row and its reward as they were, and Alice's room agrees with the durable records it reads |
| Per-participant HUD and music priority (Q150, Q72) | One HUD per participant/view, also on a merged screen. Music is built: of the participants' rooms the one with the highest priority (ambient < encounter < boss) is heard, the primary participant's on a tie (queue row MUSIC-CANDIDATES has what is left). The built-in vitals HUD is per view: each `LocalView` carries `ViewHudFacts` (`rebuild_view_hud_facts`, after `resolve_view_subjects`), the meters of the body it follows (a view that names nothing follows the controlled body; a view whose seat has no body holds and never shows another participant's meters), and `spawn_player_hud` gives each view its own HUD in its own column, stacked when views start at one point (`each_view_shows_the_meters_of_the_seat_it_follows`, `each_view_has_its_own_hud_in_its_own_column`, `huds_of_two_views_over_one_area_stack`, `each_view_of_the_split_shows_its_own_participants_purse`). On a merged screen each other seat in the controlled body's live room that no view follows has its own HUD, stacked in the shared view (`SharedViewHudFacts`; `a_shared_view_shows_each_seat_on_it`, `each_participant_on_a_shared_view_has_a_hud`, `bob_beside_alice_has_his_own_hud_on_the_shared_view`). Done 2026-10-04: stacked HUDs say whose each is ("P1", "P2", from `ViewHudSeat`, the seat that drives the body a view shows; shown only while two or more HUDs are up; `stacked_huds_say_whose_each_is`). Open: the declared readouts (`HudReadouts`: Mary-O's coins, Sanic's rings) are one per session; every seat counts as local (an online peer needs the client-local layout, multiplayer A4) | Bob's boss music outranks Alice's town music; two equal tracks play Alice's |
| Regrowth/restock (Q152) | Done 2026-10-04 for regrowth: `basement_breakables` authors "regrowing heart" (`AfterSeconds`, 4 s), and the breakable respawn's schedule, renamed `WorldTimeSchedule`, serves both customers (`pickup_regrowth_across_rooms.rs`: a quick return finds it still gone for the time that remains, a late return finds it regrown, it regrows in its live room while a plain heart beside it does not, a replay rebuilds it whole). Open: restocking. A shop (`ambition_items::shop`) sells from no stock, so nothing runs out to refill. Done 2026-10-04 for a pickup authored `Never`: once taken it is remembered `Consumed` in the occurrence ledger (`record_consumed_pickups`, republished from live state), so its room is built again without it and the save keeps it gone; a death brings back only one taken after the checkpoint (`basement_breakables` "one-time heart"; `a_one_time_heart_stays_gone_when_its_room_is_built_again`, `a_death_brings_back_a_one_time_heart_only_if_taken_after_the_checkpoint`). A pickup that authors no policy reads `OnRoomReload` and comes back on every rebuild, as before; whether that default should be `Never` is Q154 | Harvest, leave, wait, return: it has regrown (met) |
| Root identity | Two live room roots wear one `session:room_instance` `SimId`. Under Q109 that is right: the live occurrence is (`LiveRoomInstance`, `SimId`), and a construction baseline is scoped to its transaction's rooms. Measured 2026-10-03: two live rooms hold under a sync test, and a nudge of the non-primary room from outside the timeline is a mismatch (`two_live_rooms_hold_under_a_sync_test`). Census 2026-10-04 (production code; a root carries `RoomInstanceRoot`, the shared `SimId`, `LiveRoomInstance` and no `InRoomInstance`): the root lookups key by `LiveRoomInstance` (`live_room_root_for`, `LiveRoomOf`, the `retires_beside` and join despawns), and the rollback carrier order sorts by (`SimId`, `live_room_of`, previous order, `Entity`), so two roots never reach the host-local tie-break. Two maps keyed by `SimId` alone see both roots and keep one, harmlessly: the observatory's `pre_entities` (its reader iterates bodies) and `heal_projectile_owners` (looked up only by a spawn parent; a root mints nothing). One reader sees both and refuses: the publication baseline falls back to `TransactionRooms::EVERY` when a crossing has no succession (no departing room), and two roots are then `DuplicateIdentity`, so the publication is refused, not corrupted. No production road reaches it with two rooms: a replay with no controlled subject stops at admission (`definition_named(None)` names only a sole room), and a crossing's subject is stamped. Open: the same two maps collapse twin authored bodies (two instances of one room), which is not a root question | Every root reader keys by the pair; the cross-peer fold order is the remote-peer row |
| OW4 budgets | Views and pending transitions as claim holders; admission/eviction with a consumer | Cancellation and re-entry release only their own claims |
| Multi-room hot reload | Hot reload was gated to one live room, and is not now. `LiveRoomDefinition` is an index into the current `RoomSet`, not a generation-stable identity. Done 2026-10-04, the core: a publication that brings its own set (`next_rooms`) carries each live room it keeps into that set by the room's id (`apply_world_replacement`), refuses a set that has no room for one of them (`LiveRoomNotInNextSet`), and resolves its target by id for `DefinitionAlreadyLive` (that check had resolved an index of the new set in the old one, so it refused a good reload and passed a second live room of one room) (`a_reload_that_brings_a_room_set_keeps_the_other_live_rooms_rooms`). Done, the gate is explicit: it was the `Single` parameters, so the system did not run and a press was lost with no status; then a reload asked for while two rooms were live waited and said so, and `pending` stayed (the arm `a_world_reload_asked_for_while_two_rooms_are_live_waits`, which is now the arm for the one wait that is left, below). Done 2026-10-04, the gate is lifted: a reload rebuilds every live room (`a_world_reload_rebuilds_every_live_room`, on the shipped session with a changed copy of the world file: with two live rooms each room gets a new live instance and carries only the content term of the new generation; before, the press waited). It is a sequence of publications in one command flush, so no tick runs between two of them: the room of the primary seat first, which brings the set, then each other live room from that set (`republish_live_room`). The generation of the session moves one time, after the last room; moved before the other rooms, each of them was a `ContentBindingMismatch` (measured). All the plans are prepared before the first room is staged, so a live room that is not in the new set, or whose plan does not prepare, refuses the whole reload with the room named and nothing changes (`a_world_reload_that_cannot_rebuild_one_live_room_rebuilds_none`). The one wait that is left: a live room whose root has no geometry (`a_world_reload_waits_for_a_live_room_with_no_geometry`; without that guard the result is a mixed world). Residual, named and not solved: a later room whose transaction is refused when it is verified, after the first room published, leaves a mixed world; the reload is marked failed and the status says `THE WORLD IS MIXED` and names the room. No road to it is known with the guards above. Residual: a body that stays in a room other than the primary seat's is not moved to a safe position in the new geometry (the primary body is). Found, not caused by this: an `NpcSpawn` that names an unregistered character passes validation and plan preparation and panics at spawn (`character_spawn_plan.rs`), so a reload of such a file stops the game; an `EnemySpawn` with the same fault is refused when the plan is prepared | Removing the gate re-prepares every resident root with the set (met), or makes the root name its generation. An old index never takes a new generation's meaning (met for the index) |
| Remote peers | Two peers run in one process (`two_peers.rs`, netcode N2) and agree at every confirmed frame in one room. A CROSSING UNDER A PEER SESSION COMMITS BEHIND A PEER BARRIER (2026-10-04, shape A). The barrier has three parts. (1) The freeze: a session started by `start_peer_session` has the ownership `Peer`, and under it the gameplay simulation does not run from C = R + `PEER_COMMIT_FREEZE_DELAY` (1) while an operation waits (`lifecycle_commit::a_peer_commit_holds_the_simulation`, one run condition on `GameplaySimulationRoot`). When the recording frame was confirmed the two peers were at different frames (36 and 31), so the sync-test rule would run on two worlds; with the freeze those frames hold one state. (2) The commit: each peer commits alone, with the same executor as a sync test, when ITS confirmed frame reaches C, its world is at a frozen frame and its plan is authorized. (3) The rebase: each peer starts the next generation of its peer session at frame zero (`PeerLineage`), on a socket of that generation (`PeerTransport`: a socket receives only its own generation, see `netcode.md`); a peer that committed first runs no frame of the next session until the other is there (measured: it stays at frame 0). Witness `two_peers::a_door_under_a_peer_session_commits_on_each_peer_and_so_does_the_next`: Alice's crossing leaves two live rooms on each peer, the slot is free, Bob's crossing in the second session commits the same way and retires `switch_lab`, and no probed row differs at any frame of the three sessions. A same-room replay commits the same way (the `portal_bridge` walk of `two_peers_agree_in_the_rooms_that_carry_the_float_rows` runs two sessions). Poisons, as arms: one frozen frame that simulates moves the census; the commit with no freeze starts the next sessions from two worlds (the tick differs at frame zero, GGRS reports a desync). A sync test does not freeze (6 ticks from recording to commit). The readiness gate counts passes, not ticks, so a held simulation still authorizes its plan (`readiness_gate.rs`). MEASURED COST at a link latency of 3 updates each way: a peer's simulation is held 23 to 43 updates for one crossing (0.38 s to 0.72 s at 60 Hz): 2 to 7 until the commit, 21 to 36 for the GGRS handshake of the next session | Q155 (is that hold acceptable, or does the crossing keep its session). Named remainders: a link that loses parcels needs a linger before the old session ends (the in-memory link loses none); the freeze stops each live room, also the room nobody leaves; A PREPARATION THAT FAILS HAS NO RULE (measured 2026-10-04, Q156; a probe that is not in the tree): (1) on one peer, for a door: the other peer commits and waits at frame 0 of a session that never starts, the peer that failed stays held, the two worlds differ, and NOTHING is reported (`rollback_health` is `Ok` on both; my earlier text said the next session reports a desync, and it does not start); (2) on one peer, for a checkpoint restore: the same, and the note of the failure is never spent (the sync-test road ends it, and that is a host decision a peer cannot make alone); (3) on each peer: both stay held with no end, where a sync test goes on. THE VERDICT OF EACH PEER TRAVELS IN THE PEER INPUT (built 2026-10-05, `PeerInput`, `netcode.md`): a peer commits only when each handle said `Prepared` in a confirmed input, so a machine that cannot prepare holds both peers on one frozen world and is reported by handle (`two_peers::a_peer_does_not_commit_a_crossing_the_other_peer_could_not_prepare`); what a `Failed` then does is Q156; a door's plan is lowered from this machine's save (`minted_baseline_from_save`), so peers with different saves build different plans: plans must lower from a durable horizon that the peers agree on (read, not measured); a real transport owes the generation rule |
| Product policy | Q153 (join road). Still open: whether a world clock survives a save | Rulings in `maintainer-decisions.md` |

## Forbidden regressions

- No `Single` fast path and no fallback to "the live room" in a reader that has
  a subject. One live room is the same implementation as two.
- Do not key a live identity by `SimId` alone or by authored room id. Two
  instances of one room share both.
- Do not store the live room ordinal in a save.
- Do not let a camera, a view or device pressure decide which room simulates.
- Do not despawn mechanical actors, shorten rollback history or reduce physics
  accuracy in response to local device pressure.
- Do not gate a rewinding-schedule system on `is_changed()`, `Changed<>` or
  `Added<>` unless its output is identical on a load frame. A rollback load
  marks every snapshotted value changed.
- One authority defines room aliases. A caption or art key is not occurrence
  identity. One owner retracts a fact; two fallback retractors hide each
  other's defects. The producer publishes identity with a prefetch plan; the
  consumer does not fabricate it.
- Settled architecture, not open questions: two-instance identity, one writer
  during handoff, generation-aware preparation, and no camera-owned simulation.

## Measurement

- `[census] rooms` prints every live room, its definition, its instance and its
  holders.
- `scripts/check_alias_census_agrees_with_source.py` counts the sole-room
  readers.
- The two-player witnesses are in
  `game/ambition_app/tests/two_players_two_live_rooms.rs`.
