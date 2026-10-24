# Performance and composition: findings and proposals

**Scope:** what makes the game cost more CPU than it must on a weak device, and
what keeps Mary-O and Sanic from paying for Ambition. **Asked by:** Jon,
2026-10-15/16 ("optimize by finding more elegant patterns, instead of chasing
milliseconds"; "run [approximations] by me first"; "Mary-O and Sanic [must] not
get bogged down by everything else that is in Ambition"). **Status:** an 8-hour
run started 2026-10-16T04:40Z; from 13:30Z the target is the hub with Mary-O
("Hub first" below). This page is the live record.

**Related pages (they own their plans; this page ranks across them):**
[system-composition-audit](system-composition-audit.md) (S1-S6),
[capability-and-runtime-composition](capability-and-runtime-composition.md),
[performance-and-iteration](performance-and-iteration.md),
[simulation-tick-rate-compatibility](simulation-tick-rate-compatibility.md),
[consolidation plan](../consolidation/consolidation-plan.md).

Each claim is **MEASURED** (a run, a census, a profile, or a read of source)
or **REASONED** (inferred). A number here has the run that made it; run it
again before you quote it.

## Run progress (live, 2026-10-22 24h)

**Asked by:** Jon, 2026-10-22: "a 24 hour goal to work on monolith
decomposition and system consolidation (esp ones that might make maryo and
ambition faster if it is still bottlenecked on cpu scheduling)". Started
2026-10-22T21:49Z, ends 2026-10-23T21:49Z. Order: the answer (F40), then
the systems that run for nothing on the Mary-O route, then the larger
consolidations and the monolith packets.

**Status, 2026-10-24 04:29Z (MEASURED, desktop census and A/B):** hub Mary-O
route 1,320 → 919 systems a frame; hub Ambition route 1,323 → 993 since
the ledger slice; no surveyed room spawns an entity a frame. Desktop A/B of the
run's start against HEAD (F44, 19:55Z): main-world wall 15.27 → 7.89 ms a frame (Mary-O), 15.93 → 8.47 ms (Ambition), frame
time unchanged there because llvmpipe bounds it. Monolith packets this run:
audio, the trace recorder, the quest producer, the cutscene player, the time
module, the debris physics adapter, actor construction (K6, its own crate) the gravity-zone mechanic and the body causal recorders (to the runtime; ~8,650 production lines out, 59,217 left, by `wc -l` and the module graph
at each move). Last green commit: `8aae83270` (full `app_it` 1,304 of 1,304 run, app 253, Mary-O 204; F45: `7c6745589`..`27f7fee6d` were red in `canonical_reconstitution`). The `ambition_content` `worlds::tests` red since `0d4b5f656` are green again (`bb99b17fc`).
Waiting on: a phone profile of a build with these slices, the measurement that
says what they bought in frame time.

| slice | status | measured effect |
| --- | --- | --- |
| F40: is the phone still bound by scheduling? | done | see F40 |
| Smash's select screen sleeps off its routes (`shell_scope_is_on_screen_or_routing`, `5c6846a02`) | done | Smash 14 → 0 systems a frame on the hub's Mary-O route |
| Sanic's dressers and readout, same condition (`c26e34cb4`) | done | Sanic 4 → 1 |
| Boss overlays (sauce, flames, space sky, Inugami aura) run only with a boss visual or an overlay (`41c22de78`) | done | Ambition content 54 → 46; all three slices: 1,320 → 1,295 systems a frame |
| Portal presentation in `CapabilityPresentationSet(Portals)`: asleep where no live room uses portals, one clean-up frame after (`f40a9ba7a`) | done | not yet re-measured |
| The touch joystick's UI stage runs only with work (`93238b9e3`): its two `Node`s were written each frame, so the UI was laid out each frame | done | not yet re-measured; `ui_layout_system` was ~2.4% of the phone's main thread |
| Conversation effects (challenge, pet, wear, talker spacing) in `CapabilitySet(Dialogue)`; boss volumes and limbs in `BossEncounters`; new `Capability::MatchStocks` (`70657f415`) | done | not yet re-measured; 15 systems a tick in Mary-O's rooms by the census list |
| Boss presentation (sprites, part binding, stage light, name card, finish, health bar) in `CapabilityPresentationSet(BossEncounters)` (`aa1cb7565`) | done | not yet re-measured; 11 systems a frame |
| Every main-world schedule serial: `SerialScheduleSweep` in `Plugin::cleanup` (`42e1843d4`) | done | see F41: `ShelterFontSources` 334 → 8 µs a frame, `ProcessLdtkApi` 469 → 24, `ReadInputs` 96 → 28 |
| Portal observation in `CapabilityPresentationSet(Portals)` (`147e09d9f`) | done | not yet re-measured |
| The animators write a sprite or anchor only when the frame changes it (`FrameWrites`, `abcc6ec7e`) | done | not yet re-measured; every animated sprite was marked changed each frame, so its extraction re-ran |
| The flat menu's readers and restylers (14 systems) sleep while no menu control or tab exists; one frame after, so a hover edge resets (`AnyMenuControl`, required by every `AmbitionMenuControl<A>`, `f1cdc7ea8`) | done | census with portal observation and boss presentation: 1,253 → 1,229 systems a frame; `ambition_menu` 11 → 5, host 20 → 11 |
| The load presentation pass (12 systems) and its shell adapter (4) sleep between loads; the idle pass rewrote the empty model each frame (`447fada93`) | done | census: `ambition_load_presentation` 15 → 0 |
| The shell's launcher and startup-card presentation (7 systems) sleeps in play: it runs while the launcher or a sequence is up, a root remains, or the launcher, sequence or router changed; its render built the catalog and two strings each frame (`c5f6a8f91`) | done | census: `ambition_game_shell` 29 → 22 |
| The extension host's request adapters that act only on what was submitted (73 of 81; the 8 held ports, which release on an empty tick, stay as they were) run in `ExtensionSet::LowerSubmitted(phase)`, asleep on a tick whose invocations submitted nothing in the phase | done | census: the simulation 432 → 398 systems a tick, 1,195 → 1,160 a frame (with the row below) |
| Mary-O's presentation (16 systems: power blocks, fire, goal, pipe signs, shimmer, quasar, cosmic aura and barrier, void looks, readouts) in `MaryOPresentationSet`, asleep while another game is on screen | done | census, hub Ambition route: 1,400 → 1,382 systems a frame (Mary-O's 29 → 13; the rest are her materials' Bevy per-type systems); Mary-O route unchanged |
| S7c: the shield rings and the guard breaks are one pass over the shielded bodies (`rebuild_shield_views`) | done | `ambition_sim_view` 40 → 39 |
| Ambition's room looks (9) run while a live room asks for a look and one frame after; the vanity card (3) while a startup sequence runs or a card remains; the grid menu's republish while a menu is open or its tree is up | done | census: `ambition_content` 46 → 34; with the two rows above, 1,229 → 1,195 systems a frame (one new system, the desktop cursor, included) |
| The player trail is one retained gizmo drawing per room band, rebuilt only when its strips change (`present_player_trails`), instead of four `RoomTrailGizmos` groups of 6 Bevy systems each in every composition | done | census: Mary-O route 1,195 → 1,184 systems a frame, hub Ambition route 1,382 → 1,365; gizmo groups 9 → 5 |
| The effect quarantine is one pass per phase over its families (`ExternalEffectFamilies`: open, journal, release, discard), not one system per message type per phase; each family's step is the old system as a `&mut World` function | done | census: Mary-O route 1,184 → 1,163 systems a frame (25 quarantine systems → 3, plus the discard on a load) |
| The host intents' release and prune are one pass each over the registered types (`HostIntentTypes`), reading the live session once | done | 6 → 2 systems (REASONED from the three registered types; app host-intent tests green) |
| Held and wielded item presentation (in-hand sprite, over-hand sprite, hand requests) in `HeldItemPresentationSet`: runs while an item is held or wielded and one frame after | done | 3 systems a frame where nothing is held (REASONED by count) |
| Monolith packet: `audio` and `music` leave the actor monolith as `ambition_platformer2d_audio` (`9c23bfefc`); the facade's `audio` installs it, the monolith's `visible` no longer implies audio | done | ~1,200 production and ~1,400 test lines out of the monolith; 0 systems added or removed; Outlander and the minimal game stop linking the audio runtime (REASONED from the feature graph) |
| New `[census] changed_transforms` (entities whose `Transform` was written this frame, by name). Ground items are kept visuals (they were despawned and spawned each frame); world labels, the shrine and the boss stage write their transform only when it moves | done | census, hub Ambition route: 83 → 53 changed transforms a frame (14 ground-item respawns a frame → 0); Mary-O route was 73-145, mostly moving rig parts and motes |
| World item visuals (Mary-O's grow caps, wands and beacons) are kept per row of a room and look, not despawned and spawned each frame | done | REASONED: one spawn and one despawn per pickup a frame fewer, with their archetype moves and extraction; witness `a_world_item_visual_is_kept_while_its_row_stays` |
| New `[census] changed_sprites` (Sprite and Visibility written per frame, by name). The touch controls' roots and the world labels wrote their visibility each frame with no compare | done | census: visibility writes a frame, Mary-O route 14 → 4, hub 28 → 8 (what is left: the dev HUDs, the cutscene overlay, the portal-gun pickup); sprite writes were already near the moving set (motes) |
| The cutscene overlay is rebuilt only when what it shows changes (banner text, dialogue, fade and skip hold at drawn precision); it was despawned and rebuilt each frame of a cutscene, so its UI was laid out and its text measured each frame. The census row also counts UI `Node` and text writes | done | census, hub route during the intro cutscene: 6 node and 3 text writes a frame from the overlay; Mary-O route 0 and 0 |
| The dialog view is rewritten only while the dialogue changes: an open dialogue waiting on the player wrote it each tick, and Ambition's dialogue box rebuilds on that change | done | REASONED: no box rebuild (layout, text measure, portrait) on the frames a line is fully shown; witness `a_waiting_dialogue_does_not_rewrite_the_view` |
| `perf_probe` pass (Mary-O route): the effect and host-intent passes used `resource_scope`, which in Bevy 0.19 takes a resource's entity out and puts it back; with the per-family journals that was 13 archetype moves a frame. Steps are taken out of the list and put back now, and a journal's boundary is copied | done | churn: 26 resource inserts and removes a frame of ours → 0. Our systems allocate ~1,300 times a frame (top: input update 262, rollback lifecycle commit 147, body integration 135, room-transition prefetch trio ~220); none is a slice yet |
| The music intent rebuilds its playback authority (a copy of each of the provider's track and cue ids) only when the audio selection, the catalogs or the provider change | done | `perf_probe`, `basement_enemies`: `compute_music_intent` 139 → 6 allocations a frame |
| Bevy's `AnimationPlugin` is left out of every host (`ambition_platformer2d::app::default_plugins`, used by the facade's windowed foundation and the app's desktop and web builders): nothing plays a Bevy clip | done | census, hub Mary-O route: 1,157 → 1,149 systems a frame, 2,121 → 2,109 registered |
| An open dialogue is no longer rebuilt each frame. The bridge's dispatch ran each frame and took `DialogState` mutably, the menu input handed it on as `&mut`, and each rebuilt choice row arrived as `Interaction::None` and was "left": each marked the state changed, so the view was rewritten each tick and the box was despawned and rebuilt each frame. The dispatch runs only with a request; input and hover write only what they change. The dialogue-view witness from this run (and a parallax one) read `is_changed` after `update`, which is always false; they compare the write's tick now | done | census, the chicken's Hall dialogue waiting on a choice: 16 UI nodes and 5 texts written a frame → 0 and 0, the box still up (same live entity count); witness `a_waiting_dialogue_is_not_written` fails without either fix |
| Held item visuals are kept per holder, room and item, placed and flipped in place; they were despawned and spawned each frame. The ground item witness read `is_changed` after `update` for its "writes nothing" half; it compares the write's tick now | done | census, Hall: 3 held-item spawns a frame → 0 (`added=0`); witness `a_held_item_visual_is_kept_while_its_holder_holds_it` |
| Dialogue systems run only with work: the dialogue presentation set while a dialogue is shown or its view changed, the pointer input while one is open, the game-mode sync in the dialogue mode, the markup cue's clear when a cue is set, the runner spawn until there is a runner (with the dispatch from the row above) | done | census, hub Mary-O route: 1,149 → 1,142 systems a frame |
| Portal labels, gun pickups and shots are reconciled by source and look (`PortalVisualDrawn`), and the held portal gun is one sprite updated in place; all were despawned and spawned each frame, and each label is a `Text2d` laid out again when new. A crossing's two clipped gun copies are still made each frame | done | census: hub spawns a frame 1 → 0 (the pickup); `portal_lab` 0 with its portals' labels kept; witnesses `a_portals_label_is_kept_while_the_portal_stays` and the held-gun half of `transiting_carrier_gun_decomposes_into_two_clipped_charts` fail with the reconcile taken out |
| The portal captures' copies of the backdrop write their sprite, render layers and transform only when they change (the layers, one per capture and numbered past 64, were built on the heap each frame) | done | census, `portal_lab` standing still: sprite writes 76 → 36 a frame, transform writes 82 → 42; screenshot unchanged but for drifting motes |
| The boss health bar is rebuilt only when the boss, its health or the fonts change, or it is gone; it was despawned and rebuilt each frame of a boss fight, so its UI was laid out and its two texts measured each frame | done | census, `you_have_to_cut_the_rope`: 6 UI nodes and 2 texts written a frame → 0 and 0, visibility writes 10 → 4; live entities the same series; witness `the_boss_bar_is_kept_while_the_fight_stands_still` |
| The boss name card is kept through a boss's entrance, its colours, rise and rule written in place (its 72 px name was spawned and shaped again each frame); mark beacons are spawned only when the marks change (each frame, with a path lookup of their art) | done | REASONED from the code; witnesses `the_name_card_is_kept_as_it_comes_in` and `a_beacon_is_kept_while_its_mark_stays` fail with the gate taken out. A code search for systems that open by despawning what they drew finds the rest one-shot or brief (the finish bars, the portal disorientation glyph) or dev-only (the debug health bars) |
| Monolith packet: the gameplay trace recorder (`dev::trace`) leaves the actor monolith as `ambition_platformer2d_trace`; the runtime installs it, the facade names it `trace_recorder`. The room-entered quest producer moves to the runtime that installs it | done | ~1,340 production and ~910 test lines out of the monolith (36 tests); 0 systems added or removed |
| Each narrative ledger's release and prune run only while the ledger holds a record (twelve ledgers, two systems each, ran every frame in Ambition's rooms); the match-decided camera zoom sleeps with the stocks rules (`CapabilitySet(MatchStocks)`), as the only writer of its message does; the quest panel builds its text only when the quests, the setting or the panel change | done | census: hub Ambition route 1,323 → 1,298 systems a frame, Mary-O route 1,142 → 1,141 |
| Monolith packet: the cutscene player (`cutscene`, `CutsceneSchedulePlugin`) moves to the runtime that installs it and that the profiles disable it from | done | ~400 lines out of the monolith; 0 systems added or removed |
| Monolith packet: the time module (time control and the `SimDt` mirror) moves to the runtime as `sim_clock`; the monolith keeps its two clock-discipline scans | done | ~400 production lines out of the monolith; 0 systems added or removed; monolith 1,240 and runtime 100 lib tests pass |
| `perf_probe`, hub Ambition route: the parallax theme retirement builds its keep-set only when the live rooms or the room set change (it looked up every live room's neighbours and their themes each frame, then compared); nameplate candidates borrow their ids and words from the nameplate index and the door snapshot instead of copying them, and a still plate's label is not rewritten | done | allocations a frame: `retire_departed_parallax_themes` 193 → 0, `sync_actor_nameplates` 248 → 96 |
| The touch buttons' glyphs are built again only when the active device, the settings or the seat bindings change, or a button is new; each glyph string was built every frame to be compared | done | `perf_probe`, hub: `update_button_glyph_from_active_input` 22 → 0 allocations a frame. Left as they are, with reasons: the occurrence horizon's mirror key (18) compares cloned values on purpose so a rewind that changes an input is seen; the input action update (262) is leafwing's map, already cut from ~19,000 |
| Mary-O's castle ending (5 rules: the ending, the charges, the lava, the gate, the thanks) is one set that runs only where the controlled body's room has the switch; each rule searched the room's blocks for the switch by name every tick, in every level | done | REASONED: 5 systems a tick fewer in a level without the switch, and one block search instead of five where there is one; Mary-O 204 lib and 11 switch/lap integration tests pass |
| The effect pipeline's nine stages run only with work (a request stage while its message is unread, an ageing stage while a visual of its kind exists), as do the slash's three; the puppy slug's dream sync, clean-up and demand while a dreaming slug or an overlay exists; the cut-rope boss's music release while one of its claims stands (it took the music request mutably each tick, so the request was marked changed each tick in every game) | done | census: Mary-O route 1,137 → 1,119 systems a frame, hub Ambition route 1,299 → 1,283; witnesses `an_expired_visual_of_each_kind_is_still_removed` and `the_cut_rope_music_claim_is_released_when_no_live_room_is_its_room` fail with a gate poisoned; the scoping guard lost its one exemption |
| S7c: the simulation's row views (held, ground and world items, mark beacons, body clocks, shrines, hostile wielded items, projectile views) are rebuilt while a source of their rows exists or the view still holds rows, the tick after the last source goes included; each cleared and refilled its row (and marked it changed) every tick with nothing to show | done | census: Mary-O route 1,119 → 1,110 systems a frame, hub Ambition route 1,283 → 1,275; witness `a_row_view_is_cleared_after_its_last_source_goes` fails with the rows half taken out; 80 item, projectile, shrine, mark and clock app tests pass |
| The shell's command processors (shell, launcher, sequence, developer quit) run while their message is unread, its event readers (scoped clean-up, routing failures) likewise, its initializer until it is initialized, the sequence tick while a sequence runs and the segment clean-up while a segment-scoped entity exists; the camera's shake and zoom appliers run on their request, and the zoom's hold-and-release tick while a zoom is held or releasing (at rest it wrote the zoom each frame) | done | census: Mary-O route 1,110 → 1,100 systems a frame, hub Ambition route 1,275 → 1,263 (12 each, by the list diff); witness `a_finishing_zoom_releases_and_then_rests_unwritten` fails without the gate; the 63 shell tests and 89 shell, route, shake and zoom app tests pass. A witness that the idle sequence tick wrote its resource passed without the gate (the tick returns before taking it mutably), so it was dropped and the claim with it |
| Physics debris: the burst subscriber runs on a burst message, the lifetimes while a piece exists, the retirement while a piece waits on its despawn (the avian pause still runs every frame and writes only on a change) | done | census: 3 systems a frame fewer on both routes (Mary-O 1,100 → 1,095 with two noise rows, Ambition 1,263 → 1,260); the plugin-level avian test now also despawns an expired piece through the gated passes and fails with the retirement gate poisoned |
| Monolith packet: the debris physics adapter (`world::physics`: the avian install, room floors, bursts, lifetimes, retirement) leaves the actor monolith as `ambition_platformer2d_physics_debris`, a leaf the monolith re-exports as `world::physics` | done | ~580 production and ~460 test lines out of the monolith (11 tests with avian, 2 without, in the new crate); 0 systems added or removed; footprint 61 → 62 (declared); the footprint doc's quote, stale since the trace packet, re-quoted |
| The FPS counter (on by default, also on the phone's `android_dev` build) syncs its setting and fonts on a change, toggles on the hotkey and is placed when the setting, the menu row or its node changes; Mary-O's quasar overlay syncs and tidies while a quasar source or overlay exists | done | census: Mary-O route 1,095 → 1,091 systems a frame (5 by the list diff, one noise row), hub Ambition route 1,260 → 1,257; witness `turning_the_counter_off_hides_it` fails with the placement condition poisoned |
| The rollback observatory (dev, also in `android_dev`): its proof passes run while a proof runs, its observer while anything of it shows, its HUD while it shows or is still up, the proof request on its hotkey; the HUD wrote its visibility each frame. The gamepad probe samples while shown (a toggle resets its peaks, so hidden samples were thrown away), its text follows the probes and its visibility is written on a change | done | census: Mary-O route 1,091 → 1,082 systems a frame, hub Ambition route 1,257 → 1,248; visibility writes a frame 2 → 0 on both routes (none is left); witness `an_idle_rollback_hud_is_not_written` |
| Portal systems outside `CapabilitySet(Portals)` that ran in rooms with no portal: the carve bridge runs while a carve stands or a room still holds a hole or glue, the host depths while a portal stands or depths remain, the orphan sweep while a portal or shot exists, the reset clear on `ClearPortals`, the pose-warp declaration on a transit, the dev toggle on its hotkey. The capability's contract holds: a room with no portal still has an empty answer | done | census: 6 systems a frame fewer on both routes (Mary-O 1,082 → 1,075 with one noise row, Ambition 1,248 → 1,244 with two); witness `a_hole_left_after_the_last_carve_is_sealed`; 129 portal, 61 content portal and 74 app portal, carve, transit and reset tests pass |
| Message and request consumers run only with a message: the nine `SessionScopeSet::Activate` resets on the activation message (the save hand-over on its own session event), and in the simulation the clock reset, ambient gravity, heal, actor spawn and banner requests. Three took their resource mutably each tick and so marked it changed each tick: the quest registry (now gated on a pending event), the switch drain (an activation queued or one resolved last tick) and the cutscene trigger queue (a trigger queued) | done | census: 17 systems a frame fewer on both routes (Mary-O 1,075 → 1,058, Ambition 1,244 → 1,225); witness `a_resolved_activation_is_cleared_and_then_the_drain_rests`; 124 app switch, quest, heal, cutscene, banner, clock, gravity and spawn tests, Mary-O's 204 + 11 + 102 and the scoping guard pass |
| The combat trigger adapters (fighter specials, status ticks, actor thinks, body contacts, strikes, parries) queue only while an admitted entry binds their port, and once on the tick the admission changes (`ambition_extension_host::trigger_is_bound`); each built its list of bound selectors every tick. Not in the installer: `ModuleEntityTickPort` is bound by placed module entities, which no entry declares (a blanket gate silenced Smash's mines in its 4 tests) | done | census: 0 on both hub routes, which admit Ambition's and Smash's modules for all six ports. REASONED: a composition that admits none for a port (a Mary-O-only build) runs none of its adapters. Witness `a_trigger_adapter_runs_while_its_port_is_bound`; Smash 293, combat 790 and 53 app extension tests pass |
| Versus-match activation runs only with a match to make or run: preparation while a roster is published, activation while a plan is prepared, the opening hold's release and the live match clock while a match is active, the camera's match framing while a match is active or a framed cast remains | done | census: 5 systems a frame fewer on both hub routes (Mary-O 1,060 → 1,053, Ambition 1,230 → 1,221 with noise rows); Mary-O-only hub (`AMBITION_HUB_GAMES=mary_o`) 981 → 976. Smash 293 and 136 app match, stock, knockout and seating tests pass; gating activation on the active match instead fails all 5 knockout tests |
| The bubble shield's shader layer writes its scale only on a change, and compares its material before taking it mutably: `Assets::get_mut` marks the material modified, so every shown player's bubble material was prepared for the GPU again each frame (the hit flash already followed this rule and says so) | done | census: `Bubble_Shield_Fx` leaves the changed-transform rows on both routes (1 a frame → 0). REASONED from the code: one material preparation a frame fewer per shown bubble; the census does not count asset modifications |
| Four world render passes run only with work: the block flinch on a strike or while a flinch plays, the lock-wall slide while a wall slides, the removed-block sweep while a live room's overlay names a removed block, the sprite-handle refresh when `GameAssets` changes (its own early return) | done | census: 4 systems a frame fewer on both routes (Mary-O 1,053 → 1,049, Ambition 1,221 → 1,218 with one noise row); witness `a_struck_block_flinches_and_settles_home`; render 443, 39 app lock, brick, block and wall tests and Mary-O's 102 integration tests pass |
| The versus stage outside versus: its roster tracking runs while the versus route is open or its roster stands (the teardown arm), its reconciliation while a roster exists, and its scoreboard while a match runs or a slot still holds a readout; the scoreboard cleared its slots every frame in every game, marking the HUD readouts changed every frame | done | census: 3 systems a frame fewer on both routes (Mary-O 1,049 → 1,046, Ambition 1,218 → 1,213 with noise rows); witness `a_finished_scoreboard_is_cleared_once_and_then_rests`; 31 versus lib and 67 app versus, roster and HUD tests pass |
| The debug and navigation overlays (dev, in `android_dev` too) run while switched on: each system's first test is its run condition, and the label pass runs while a label is queued | done | census: 3 systems a frame fewer on both routes (Mary-O 1,046 → 1,044 with one noise row, Ambition 1,213 → 1,210). REASONED: the conditions are the systems' own early returns, so what draws is unchanged; app lib 253 tests pass |
| The passes that draw a simulation row view (ground and world items, shrines, mark beacons, body clocks) run while the view holds rows or a drawn entity remains to retire (`ambition_sim_view::rows_or_drawn`), the shrine animation while a shrine is drawn; a shrine's sprite size was written every frame | done | census: Mary-O route 1,044 → 1,038 systems a frame, hub Ambition route 1,210 → 1,208 (its hall has ground items and a shrine); witness `a_drawing_pass_runs_until_its_last_drawn_entity_is_retired`; render 444, sim_view 135 and 74 app item, shrine, mark and clock tests pass |
| Five effect passes run only with work: the knockout beat on its message, the dizzy stars while a guard is shattered, ray strikes while a strike stands, the unauthored attack volumes while a strike stands or a drawn volume remains to retire, the gravity zone visuals while a zone or its visual stands | done | census: 5 systems a frame fewer on both routes (Mary-O 1,038 → 1,032 with one noise row, Ambition 1,208 → 1,203); witness `an_ended_swings_volume_is_taken_away`; render 445 and 59 app strike, ray, knockout, guard and gravity tests pass |
| The simulation's request appliers run on their request: item grants, shop transactions, flag, quest, switch and gameplay-SFX effects, NPC provocations and their release, conversation-cut barks, and the projectiles of brain actions. Found on the way: `PauseRequested` (the Android pause, 10-22) was unnamed in the rollback message oracle; it is a shell channel no simulation crate can read, named now | done | census: 10 systems a tick fewer on the Mary-O route, 9 on the hub's Ambition route (1,194 a frame); monolith 1,229, 125 app flag, quest, switch, item, shop, provocation, bark and projectile tests and 129 rollback tests pass |
| Three hit consumers run on their message: landed-hit effects (`LandedBodyHit`), the player-victim staging (`HitEvent`) and the impact hitstop request (`ResolvedBodyHit`). Not `apply_feature_hit_events`, which does more after its message loop than was vetted | done | census: 3 systems a tick fewer on both routes (Mary-O 1,020, Ambition 1,190 a frame); combat 790, Smash 293, 42 app hit and 229 app Smash, knockback, stock, rollback and combo tests pass |
| The projectile owners' identity heal (twice a tick, head and tail) runs while a projectile is live; the falling chests while a chest falls | done | census: 3 systems a tick fewer on both routes (Mary-O 1,017, Ambition 1,188 a frame); runtime 114 and 37 app chest, projectile and identity tests pass |
| Zone snapshots (gravity, force, oscillating zones) run while a zone or a snapshot row stands; item motion while an item moves; projectile perception while a projectile is live or its rows stand. Gated systems' docs no longer promise "every frame" | done | census: 5 systems a tick fewer on both routes (Mary-O 1,017 → 1,012, Ambition 1,188 → 1,184 a frame); poisoned witness `the_snapshot_is_cleared_after_the_last_zone_and_then_rests`; shared_tangle 334, world_items 19, monolith 1,229 tests pass |
| Twenty-six more passes run only with their work. Sim: hazards (any hazard), death interludes (open on a death, tick and close while one stands), death beats, transform beats, errands, the banner while it is up, clank rebounds, mount-death brain rebuilds, summons, item uses, the cutscene tick while one plays, boss render and frame indices while a boss or a row stands, the shrine pulse while it runs. Render: spirit and void fields while an aura or a field stands, launch trails while a body is launched, chain earth and turf while one is undressed, the bubble-shield pool spawn and its layer attach once, the cutscene overlay while one plays or is drawn. Ambition's seating restatement runs on a change of its inputs, so Mary-O no longer pays for it | done | census: Mary-O 1,012 → 984, Ambition 1,184 → 1,157 systems a frame (26 and 25 gated rows, the rest noise); poisoned witnesses `a_banner_runs_down_and_then_rests_unwritten`, `an_ended_cutscene_leaves_its_resources_unwritten`; combat 791, monolith 1,229, runtime 101, encounter_features 45, sim_view 135, render 445 tests pass |
| Checkpoint, replay and retire passes run on their message: the commit-time baselines and forgets (four, `CheckpointCommitted`), the replay answers (four, the new `a_replay_is_admitted`: an admitted replay or a pinned committed restore, the two roads `AdmittedReplays` reads), retracted boss mints, the session-retire reset; possession release while a body is possessed | done | census: Mary-O 984 → 975, Ambition 1,157 → 1,147 systems a frame (11 gated rows each); poisoned witness `a_gated_replay_answer_sees_both_roads`; combat 792, monolith 1,229, runtime 101 tests pass |
| ⛔ REGRESSION FIXED (F45): `7c6745589` gated `apply_clock_reset_requests` with `on_message` inside `GameplayGated`; Bevy evaluates a system's conditions while its set is off, so the condition took a replay's `ClockResetRequest` during the transition hold and the reset never ran. All 56 `on_message` gates of this run now use `ambition_platformer2d_core::a_message_waits` (a peek), as do `a_replay_is_admitted` and the block-flinch condition | done | `git bisect run` over `0d4b5f656..27f7fee6d`: first bad `7c6745589`; removing that gate alone turns 4 red `canonical_reconstitution` tests green; with the peek, 19/19 pass |
| Presentation passes that ran for nothing on the Mary-O route: ground shadows (no game asks for them since 10-10) while a game asks or one is drawn, the developer health bars while on or drawn, daylight while a game has a `DaylightStyle`, mote drift while a mote stands, world-label fonts on a font or label change, the quest panel on a quest, setting or new-panel change; Ambition's portal-facing setting sync on a change, its reset bridge while a replay waits | done | census: Mary-O 975 → 968, Ambition 1,147 → 1,140 systems a frame (6 gated rows each; daylight and motes have work on both routes); the peek gates add no run; 12 lib suites and 238 `app_it` checkpoint, restore, replay, death, session, activation and clock tests pass |
| ⛔ REGRESSION FIXED: `1e7616973` (10-22) put Mary-O's presentation to sleep while another game is on screen; her pipe-sign pass had stamped the hub's room at boot (the router was moving), and after a world reload it slept, so the room drew one stamp fewer (`a_world_reload_draws_its_room_once`). A room with no pipe to a level gets no stamp now | done | full `app_it`: 1,303 pass, this 1 failed and now passes; app 253, Mary-O 204 lib tests pass. Still red and not this run's: `ambition_content` `worlds::tests` (3), from `0d4b5f656` (the seventh world) |
| The themed door and fixture dressers run while one of their rows is undressed | done | census: Mary-O 968 → 965 (one noise row), Ambition 1,140 → 1,138 systems a frame; render 445 tests pass |
| The room-transition cooldown counts down only while a seat's runs (it wrote the session root's component every tick) | done | census: Mary-O 965 → 964, Ambition 1,138 → 1,137; shared_tangle 334, monolith 1,229 and 76 app reconstitution, door and transition tests pass |
| Avatar passes: moving platforms advance while a room has one, scripted control frames blank while a body is held (`ControlHolds` is present exactly then), commanded moves drive while one is commanded, mana regenerates while a composition declares `PlayerManaRegen` | done | census: Mary-O 3 gated rows (964 a frame, 3 noise rows), Ambition 1,137 → 1,135; monolith 1,229, runtime 101 and 87 app reconstitution, platform, cutscene, hold, capture and conversation tests pass |
| Hit passes: hitbox damage and lifetime while a hitbox stands, breakables while one stands, landed and resolved move-hit marks and the pogo bounce while their message waits (peek) | done | census: Mary-O 964 → 955, Ambition 1,135 → 1,129 (6 gated rows each); combat 792, monolith 1,229, runtime 101, Smash 293, Mary-O 204 and 186 app hit, strike, Smash, guard and reconstitution tests pass |
| F46 cut 1: the hub's two-state look draws its front one texel a 16 px cell into a field each frame, and its skins and overlay read it in place of three to five per-pixel claims | done | hub capture identical (1 pixel by 1 level of 921,600); llvmpipe hub A/B frame p50 40.91 → 36.13 ms; content 519 (3 red, `0d4b5f656`'s) and 17 app look tests pass; phone A/B pending (`AMBITION_ROOM_FRONT_FIELD=off`) |
| The visual-quality sync and the portal capture budget run on a change of their input or output; the placeholder-sprite override while placeholders are on or a sprite keeps its original. The front field's material adds Bevy's two per-material systems on every route | done | census: 3 gated rows each; net Mary-O 955 → 954, Ambition 1,129 → 1,129; render 445 and 9 app quality, tier and hall tests pass |
| The room-transition machine's idle passes: readiness authorization while a transition awaits it, the two failure passes while one failed (they took the load state mutably each frame, so it read as changed each frame), and the two exclusive finalizers while a crossing or its restore is pending | done | census: 5 systems a frame fewer on both routes (Mary-O 949, Ambition 1,124), two of them exclusive and per tick; runtime 101 and 200 app transition, door, reconstitution, checkpoint, restore, load and cover tests pass |
| Two exclusive per-tick passes that returned at once: extension admission runs until the world is admitted, authored commands while one waits (peek) | done | census: 2 systems a tick fewer on both routes (Mary-O 948, Ambition 1,123); extension_host 43, shared_tangle 334 and 94 app extension, module, authored, lever, switch, special and reconstitution tests pass |
| The two exclusive live-reload publishers run while a pack or a sheet table waits to be published | done | census: 2 systems a frame fewer on both routes (Mary-O 945, Ambition 1,121); 76 app reload, pack and sheet tests pass |
| Three more exclusive passes that idled: the publication-lease breaker while a reload generation holds a lease, the local GGRS restart on the frame an LDtk reload asks for it, the character capability audit while a sheet demand is outstanding | done | census: 3 systems a frame fewer on both routes (Mary-O 942, Ambition 1,117); monolith 1,229 and 43 app reload, publication, hot-reload and character tests pass |
| Mary-O's life spend and death voice on a death (peek), her return from the cosmic form while a transform beat stands | done | census: Mary-O 942 → 940 (3 gated rows, one noise); Mary-O 204, Mary-O app 102 and 20 hub Mary-O tests pass |
| ⛔ F46 cut 1 fix: the state shader's new binding `field` redefined `fn field` and the shader did not compile (veils and overlay undrawn since `4a5ff08cc`); renamed `front_field`. And an exact early-out: the overlay works out no field where no tear can be (plane distance below -230 or above 800) | done | hub capture against the true pre-field baseline: 2 pixels differ, none by more than 8 levels (three runs); the phone run of 21:13 must be repeated; llvmpipe hub A/B with the overlay drawn: frame p50 59.45 → 42.99 ms |
| Portal carves while a portal stands (or last tick's holes or glues are still to clear); Ambition's ground-item carves while a portal stands, so Mary-O stops paying for them. Not the passages: they rebuild from tuning and body budgets with no portal, and would go stale | done | census: 2 systems a tick fewer on both routes (Mary-O 937, Ambition 1,117 with two noise rows); portal2d 129 and 127 app portal, transit, seam, aperture, gun and window tests pass |
| The durable-restore slots run only while the save waits to be applied (every member returned at once on the latch); the boss defeat retraction pair on a waiting checkpoint or admitted replay (peeks); Mary-O's four priority-music claims (star, death, victory, cosmic) while a source stands or its claim does (`holds_claims_of_kind`). Tried and reverted: the gate-portal visuals, whose registry is not empty on either route | done | census: Mary-O 937 → 928, Ambition 1,117 → 1,110 systems a tick (both with noise rows); `canonical_reconstitution`, boss replay, save-road and custody tests 84 pass, Mary-O music 37, Sanic restore 3 |
| Press-only technique handlers (teleport, trapdoor, flyline, vitality) and the fighter-special queue run while an action message waits (a peek). Ambition does not gain: the robot's charge-projectile tick fills that channel every tick | done | census: Mary-O 928 → 922 systems a tick, Ambition 1,110 → 1,109 (noise); app technique and reconstitution tests 48 pass, `smash_it` 52 |
| The boss special-cast queue runs while a boss stands (`any_with_component::<BossEncounter>`): it casts for each boss and none means none. Declined after reading: the trapdoor, flyline, tether and limb-trail visuals, which visit every body and actor each frame; a condition would repeat that walk, and fusing the four is the remedy if they show in a profile | done | census: Mary-O 922 → 921 (one noise row); boss possession, specials and GNU-ton tests 56 pass |
| K6: actor construction leaves the monolith as `ambition_platformer2d_construction` (a leaf since K3; K4 was its precondition). No system moved; the tests that drive it through the kernel's lowering stay in the monolith | done | monolith production lines 63,879 → 61,303; the cycle is the four of K3; construction 61 tests (55 + 6), publication 82, room-building app tests 105, absence contracts 55/55 |
| The gravity-zone mechanic moves up into the runtime, its only installer. Found while proving it: `ambition_workspace_policy` was red from this run's time, music and trace moves (and K4's and S2b's), a lane those moves never ran; its policies now name the new owners. The level-box readers ratchet was red from five reads across 10-20..23; four classified, the slow-zone overlap converted to the footprint | done | monolith 61,303 → 60,612 production lines; workspace policy 38/38, level-box guard green, runtime gravity 9, app gravity 26, Mary-O clock, Hunny slow 13 |
| Ambition's hub stops running absent bosses' content: the cut-rope flavor chain while a cut-rope arena is live or held, GNU-ton's duel and Mockingbird's moon strike while a boss stands (F47's candidate row, the exact ones) | done | census: Ambition route 1,109 → 1,103 systems a tick; cut-rope, GNU-ton, Mockingbird and reconstitution app tests 95, content lib 29 |
| The capture passes run with their work: the request appliers (attempt, pummel, throw, carry, parried rebound) while their message waits (peeks), and the seven that visit holds while a body is held (`a_capture_stands`). The anim-facts mirror keeps running: it clears a released body's flags the tick after | done | census: Ambition route 1,103 → 1,093 systems a tick (Mary-O does not use captures); combat capture 42, `smash_it` 52, Smash lib 293, app capture/grab/throw/reconstitution 44 |
| The mount crate's sets (link, boarding, leases, dismounts, backs as ground, steering, rider sync) run while a mount, rider, reservation or lease is in the world (`mounts_are_in_play`), beside the capability's own room condition | done | census: Ambition route 1,093 → 1,086 systems a tick; mount lib 13, app mount/ride/GNU-ton/FSM/reconstitution 107, `smash_it` 52 |
| The boss bodies' passes run while a boss body stands, each on the component its query visits: integration and brains (`BossConfig`), phase (`BossPhase`), animators (`BossAnimFrame`), attack trigger and projection, damageable volumes (`BossAttackIntent` / `BossAttackState`), conducted facing, conduct queue (`BossEncounter`). Not the progression chain: it builds a boss room's encounters on entry | done | census: Ambition route 1,086 → 1,074 systems a tick; boss, GNU-ton, Inugami, Wolfenstein, hall, cut-rope, Mockingbird, FSM and reconstitution app tests 233 (the one red, the Hall's Steel and Straw Man with nothing to say, fixed in the next commit), boss lib 226 |
| Lit fuses (bomb, gravity grenade), temporary gravity zones and respawn grace tick while one exists; stock spending peeks its knockout; the match's pace runs with an active match. The arming passes stay: they visit every ground item | done | census: Ambition route 1,074 → 1,070 systems a tick (two noise rows); Mary-O 922 unchanged; abilities 32, `smash_it` 52, app stock/bomb/grenade/versus/respawn/reconstitution 111 |
| The wave encounter's effect and cleanup adapters run on a lifecycle event (a peek); the lock walls contribute while a lock wall exists. Checked first: the overlay rebuild clears every contribution each tick (`clear_engine_contributions`), so a sleeping contributor leaves nothing stale. That also clears the mount backs gated above | done | census: Ambition route 1,070 → 1,067 systems a tick (noise rows aside); encounter features 45, app encounter/wave/lock/arena/reconstitution 60 |
| The boss animator passes (animate, demand part sheets, bind, pose) run while a `BossAnimator` exists: each loops over it alone. The stage veil, name card, finish bars and health bar stay: they fade or retire their UI after a boss leaves | done | census: Ambition route 1,067 → 1,063 systems a tick; render boss tests 5; a capture of GNU-ton's arena draws the giant and GNU-ton |
| F46 cut 3: the hub look's overlay draws only the front's unsettled band (a mesh cut to the band, hidden when the room is settled). Exact: outside the band the shader returned zero alpha | done | hub capture against the pre-cut binary: 4 px differ by ≤1 level (run-to-run noise ~2,600 px); room_look witnesses 2 new, 5 pass; GPU fragment saving REASONED, needs the phone |
| Portal simulation runs with a portal: transit, snags, links and apertures while a portal stands or a body carries a transit latch (which transit clears); evictions while a portal stands or the frame history still holds one (the tick a portal closes still evicts); fire, toggles and transit reconcile peek their messages; cooldowns while one runs. Conversation input and reveal while a conversation is open or its hold mark is up. The hub has no placed portal (its LDtk level, MEASURED) | done | census: Ambition route −15 systems a tick (13 portal, 2 dialogue); portal2d 129, app portal/transit/snag/evict/transition/reconstitution/dialog/pause 84, dialog 26 + conversation 28 |
| The held-item passes run with an item: release, pickup, settle and physics while a `GroundItem` exists (carried or lying); firing and throwing while a body holds a `HeldItem`. Residency stays ungated, as its comment requires | done | census: Ambition route −2 (the hub has ground items, no holder); held items 43 pass. The `a_dog_sent_for_an_item` reds in the shared tree fail with this change reverted too (another session's crossing work) |
| The portal observation publishers (viewer, camera-continuity focus, F1 sync, scene/affordance tags, body views, transiting bodies, threaded pictures) run while a portal stands or a body carries a transit latch, and one frame after, which publishes the empty answers; the view-cone debug passes run on their trigger (`2f41700b3`) | done | census: Ambition route 1,045 → 1,035 systems a frame (11 here, 1 noise row); app portal/view/continuity/transiting/far-side 43, host 37 |
| The conversation passes run while a conversation is live: the dialog projection and end stamp while live or shown and the frame after, the end intake peeks `ConversationEnded`, the separation check on a live one, the hold while live or a body is still held (`2835c098a`) | done | census: Ambition route 1,035 → 1,029; `canonical_reconstitution` green |
| The boss presentation passes (stage light, name card, finish drain and bars, boss bar) run while the boss read model holds a row, and the frame after: `bosses_are_shown_or_were` (`3f5a9992d`) | done | census: Ambition route 1,029 → 1,024, all five idle in the hub; all five run in GNU-ton's arena |
| The boss encounter sim passes run on their subject: finish slow motion and the encounter wrap on a `BossConfig`, hazards on a `FallingHazard`, payloads on `ReleaseOnDeath`, peeks on `BossPhaseChanged` and `MountDied`, the boss-life switch on a boss-life press, the registry fill until it is filled (`ad07994fb`) | done | census: Ambition route 1,024 → 1,015; app_it `boss` 90, `gnu_ton` 44, `canonical_reconstitution` 19 green |
| The content portal adapters and the cut-rope room passes run on their subject: fire, pickup, drop and the sfx peek their messages, host attachment on a placed portal, the shot step on a shot; the cut-rope resets on an admitted replay, its "try again" while chosen or pending, its victory NPC while a release waits or an arena is live (`a274c338e`) | done | census: Ambition route 1,015 → 1,005; app_it `portal` 24, content `portal` 62 and `cut_rope` 7 green |
| Social, match and item passes run on their subject: challenge, pet and wear requests and actor stimuli peek their messages, pet beats and pending challenges on their components; the stocks verdict zoom peeks `StocksMatchDecided`, respawn on `PendingRespawn`, match items with an `ActiveMatch`, held courses on `HeldCourse`, thrown impacts on `ItemStruckBody` (`00de8d98c`) | done | census: Ambition route 1,005 → 993 (11 gated, 1 noise row); Mary-O route 919; app_it pet, wear, conversation, stocks, smash 115, match 43, ride, thrown and `canonical_reconstitution` green; the `persisted_fate_is_built` challenge red is not this slice's (red with it reverted; sent to the gate lane) |
| The causal recorders of the bodies (movement intent, control frame, movement operations, hit resolutions and reactions) leave the monolith for the runtime's causal adapter, registered beside its frame stamp; the monolith keeps `BodyMovementOps`, the message its kernel writes (`e9d1161a5`) | done | monolith `causal.rs` 750 → 14 lines; no system added; runtime `causal` lib 8 + `causal_it` 6, app_it `causal` 3 (features `causal,rl_sim`) green |
| Android schedules no hit flash: its four passes were empty stubs there and still ran each frame, and its `Material2dPlugin` paid per-type systems in both worlds for a material no Android frame draws (`0c8aaec1f`) | done | REASONED: 4 main-world systems a frame plus the material's per-type main and render-world systems, on both phone routes; desktop unchanged; render `hit_flash`/`portal_compositing` 18 green; the Android build is NOT checked here (no NDK on this VM) |
| The dialogue-visit counter runs while a conversation is live: it counts the tick one opens (`7b009182d`) | done | census: 1 system a tick on both routes (REASONED from its body); app_it `visit`/`canonical_reconstitution` 26 green; F48 records what is left |
| Body whereabouts (where an authored body is away from home: the placed and away-from-home records and the durable-whereabouts predicate) leave the monolith for the construction crate, beside authored placements and persisted fate (`2b70a6203`) | done | monolith -208 lines, no system added; construction 6, runtime 110, workspace policy 38, multi-writer and level-box guards, app_it `persistent` 7, `errand` 4, `canonical_reconstitution` 19 green; `carried` 26/28, the 2 reds are the GNU-ton kick leg's (sent back) |
| The actor's live cluster view (`ActorMut`, `SeedActorMut`, `ActorClusterQueryData`, the damage i-frame window) leaves the monolith for `ambition_body_seed::clusters`, beside the seed it binds to the simulation (`41972de96`) | done | monolith -255 lines (59,566 left), no system added; body_seed 19, monolith lib 1,214, workspace policy 38 (sweep and archetype policies follow the file), app_it `canonical_reconstitution` 19, `enemy` 8, `boss_lifecycle` 15 green |
| The body-capability conditions (`body.can`, `body.fits`) leave the monolith for `ambition_abilities::body_conditions`, the crate that owns what a body can do (`BODYCOND`) | done | monolith -349 lines (59,217 left), no system added; abilities `body_conditions` 12, monolith `gated_lock` 19 green; the session none-arm row for the visit counter's condition added |

### F48 — After the content gates: what is left is Bevy's and Jon's (2026-10-24)

MEASURED (desktop census, hub routes, `--warmup 600`, census on, HEAD
`0c8aaec1f`): 919 systems ran in a Mary-O frame (F47: 921) and 993 in an
Ambition frame (F47: about 1,110). The Ambition drop is F47's third bullet,
done: the boss, cut-rope, portal-adapter, conversation, social, match and item
passes now run on their subject (run table, 10-23 and 10-24 rows).

What the Mary-O frame still pays, by owner:

- **Bevy's gizmo groups: 24 systems a frame, MEASURED.** Five groups
  (default, AABB, frustum, skinned-mesh bounds, light) cost four fixed-loop
  passes each, plus one draw pass each for the four 3D groups. The game draws
  gizmos only in developer overlays and uses only the default group.
  `GizmoPlugin` and `bevy_light` add the four 3D groups unconditionally
  (bevy_gizmos 0.19.1 `lib.rs`), so no condition reaches them. **Waits on:** a
  patched `bevy_gizmos`, or dropping `GizmoPlugin` from a play build with the
  debug overlays behind `dev_tools`. Jon's call.
- **Ambition's materials on the Mary-O route: 9 material types, MEASURED.**
  These are the room look's five (blueprint, state, skin, sky, front field),
  deep dream, sauce, flame and space. Each costs an `Assets<T>` pass and a
  specialization check in the main world, plus per-type passes in the render
  world, in every room of every game. **Waits on:** a single-game build (P1),
  or folding the room look's five materials into one. That is a shader change
  to the hub's look, so it goes to Jon first.
- **Developer tooling: 38 systems a frame** (F47, unchanged). **Waits on:**
  Jon's `android` versus `android_dev` answer.
- **Android-only:** the hit flash's four empty stubs and its material no
  longer install there (`0c8aaec1f`). REASONED: not measurable on this VM,
  which has no NDK.

REASONED: excluding the census markers, the gating campaign is at its floor
on both routes. Every further cut is one of the three decisions above or K5
decomposition.

### F47 — What the per-frame system count waits on now (2026-10-23)

MEASURED (desktop census, hub routes, `--warmup 600`, census on, HEAD
`bd1d11352`): 921 systems ran in a Mary-O frame, about 1,110 in an Ambition
frame. Of these, 37 are the census's own phase markers. They are installed only
when `AMBITION_PROFILE_CENSUS` is set, so they are not phone cost.

What is left, by what it waits on:

- **Developer tooling in the phone build: 38 systems a frame on either route.**
  These are egui input (16), the developer editors' propose and publish pairs
  (10), the inspector hooks, and the save, hot-reload and flash passes. MEASURED
  from the census. `build_for_android.sh` defaults to `--features android_dev`,
  and `android_dev = ["android", "dev_tools"]`, which brings
  `bevy-inspector-egui`. REASONED: a play build with `--features android`
  drops all 38 without touching the game. **Jon's call:** whether the play
  build keeps developer tooling.
- **Bevy's per-type systems (79 in the main world).** These are 55
  `Assets<T>` event passes, one per asset type, and 24 Material2d
  specialization checks, one per material. F42 and F43 found that they cannot
  be gated from outside. The render world's per-material systems are not in
  this count. **Waits on:** a fork item, or materials registered only by a
  composition whose games use them (a single-game build, P1).
- **Ambition's own content in the hub (Ambition route).** Ambition declares
  every capability, so per-boss content systems run in every Ambition room:
  the cut-rope arena (4), GNU-ton's duel, fist lines and ladder gate,
  Mockingbird's moon strike, the swallows, and the attunement spawn. Each needs
  its own exact condition (its arena live, its boss present, its encounter
  missing), and each saves microseconds. **Candidate**, not done.
- **Gates tried this evening and reverted or declined.** The gate-portal
  visuals: the registry is not empty on either route. The trapdoor, flyline,
  tether and limb-trail visuals: a condition would repeat their walk over
  every body. The animator's every-frame writes, an F43 row, already compare
  before writing (`FrameWrites`), so that row is stale.

REASONED: the gating campaign is at its floor on the Mary-O route. The next
phone-frame wins are the hub look's approximations (F46, Jon's call), the
`android_dev` question above, and K5-scale decomposition.

### F46 — Why the central hub is the expensive room: the two-state look (2026-10-23)

Jon, 10-23: on the phone and the Steam Deck the central hub is far more
expensive than any other level. REASONED from the shaders, not yet measured
on a device. The hub (`central_hub_main`) is the one room with
`palette = clean_corrupted`. With no saved state it is balanced, so the
front crosses the middle of the room, and every pixel within the
not-settled band (plane distance -560..740 px, most of a 1,900 px room)
evaluates `look_claim` (`room_look_common.wgsl`): four block levels, each
`look_field` (two `value_noise`) plus up to four `rand_cell`, about fifty
hashes. A block pixel pays it in BOTH skin quads (`room_skin.wgsl`: each
quad tests the claim to discard itself; the corrupted one tests again at an
offset for its burning edge), and the room-wide overlay quad
(`room_state.wgsl`) pays it again, so three to five claims a pixel, plus an
alpha-blended full-room layer, which a tiled mobile GPU pays in bandwidth
even where it writes nothing.

The cuts, none of which changes the picture:

1. The claim is piecewise constant on the 16 px grid (the finest level's
   cell; each coarser level is a union of them). Render it once a frame
   into a small target (about 120 x 64 for the hub) and let the three
   shaders read it: about 250x fewer claim evaluations.
2. One skin material that samples both skins and picks one: half the
   claims on block pixels.
3. The overlay's mesh covers only the not-settled band.
4. Per-level settle bounds, so more pixels leave `look_claim` early.

MEASURED by Jon on the phone, 10-23: with the lever turned so the hub is all
corrupted (the front out of the room, every pixel settled), the hub goes
from about 20 FPS to 30-40 FPS. That is the cost of the band.

**Cut 1 done (2026-10-23):** `RoomFrontFieldMaterial` draws each look
room's claims into a field, one texel a 16 px cell (`room_front_field.wgsl`,
a camera of its own on `ROOM_FRONT_FIELD_LAYER`, linear light into an sRGB
target, as the impostor atlas); the skins and the overlay read it
(`look_claim_in`, `look_is_corrupt_in`). MEASURED: `capture_scene` of the hub
at `high`, field on against off, 1 pixel of 921,600 differs, by 1 level; a
probe that wrote every texel clean changed 132,647, so the field is what is
read. ⛔ CORRECTED 22:20Z: those two captures compared a broken shader with
itself. In `room_state.wgsl` the new binding `field` redefined the shader's
`fn field`, so the state shader did not compile from `4a5ff08cc` until the
fix (`front_field`): no blink-wall veil and no overlay (loose blocks, tears)
was drawn, field on or off. With the fix, against the TRUE baseline (the
pre-field shaders on disk under the same binary): 2 pixels of 921,600 differ,
none by more than 8 levels, field on and field off alike (MEASURED). Jon's
phone run of 21:13 (`4a5ff08cc`) drew no overlay, so part of what it showed
was the missing overlay, not the field. Desktop llvmpipe A/B (the CPU rasterizes, so fragment work shows; both arms without the overlay),
hub route, three interleaved rounds: frame p50 40.91 -> 36.13 ms,
rasterizer CPU 345 -> 284 ms a frame (both arms without the overlay, see the
correction). AGAIN with the shader fixed (`445592083`), both arms drawing the
overlay: frame p50 59.45 -> 42.99 ms (-28%), rasterizer CPU 460 -> 323 ms a
frame (MEASURED, 274 and 202 windows). The phone A/B:
`--launch-env AMBITION_ROOM_FRONT_FIELD=off` makes no field. What the band
still pays per pixel is noise (`look_field` in the skins' heat and wash, the
sky's air and fog, the overlay's tears), exact only per pixel; cutting it
further is an approximation and Jon's call.

**Cut 3 done (2026-10-24):** the overlay of a look with a front draws only
its unsettled band: its mesh is the room and its pad cut to
`LOOK_SURELY_CLEAN..=LOOK_SURELY_CORRUPT` (plus 2 px), remade when the front
moves, and hidden when the band misses the quad (a room the lever settled).
Outside the band the shader returned zero alpha, so the picture is the same.
MEASURED: hub captures at `high`, the pre-cut binary against the cut: 4 pixels of
921,600 differ, by at most 1 level, while two runs of either binary differ by
about 2,600 (the tears and bobbing blocks animate). Witnesses: the band holds
every unsettled point of the quad at seven advances and no far-settled one, and the
band bounds equal the shader's. REASONED, not measured on a device: the
fragments saved are the settled share of the quad; in the all-corrupted hub, the case Jon
measured at 30-40 FPS, the overlay's room-sized alpha-blended layer is gone.

The test that confirms the cause on a device: the hub with screen shaders
off (`budget.shaders`) against on; the look is built to read the same off
(F-room-look rule, Jon 10-18). A CPU profile shows GPU time only as the
render thread waiting on the driver.

Monolith cycles, the same day (MEASURED, `measure_kernel_module_graph.py
--scc --cuts`): one cyclic component of four modules, `abilities`,
`features`, `items`, `session`; no single edge splits it (`items <->
session`, 20 and 4 references, is A7's). `features` is 26,328 production
lines.

### F45 — A run condition can eat a message its system never reads (2026-10-23)

MEASURED (source and bisect). Bevy 0.19.1 evaluates a system's own run
conditions even when a set the system is in has a false condition
(`schedule/executor/multi_threaded.rs` and `single_threaded.rs`, `should_run`).
`on_message::<T>` keeps its own cursor and advances it when it is evaluated.
So on a system in a conditioned set it takes the message while the set is off,
and when the set comes back on the condition is false and the system never
reads it. `apply_clock_reset_requests` sits in `GameplayGated`; a room replay
writes its `ClockResetRequest` during the transition hold, when the set is
off. `app_it canonical_reconstitution` caught it: four arms rebuilt the room
one tick younger than a fresh entry (enemy x 420.07 against 420.67). Found by
`git bisect run` (first bad `7c6745589`), not by the per-slice lib tests or
the census.

The pattern is `ambition_platformer2d_core::a_message_waits::<T>`: a peek at
`Messages<T>` that takes nothing. It stays true while the channel holds a
message read or not, so a system may run one extra frame. REASONED, not
audited: a condition built on change ticks (`resource_changed`, `is_changed`,
a `Changed<T>` query) has the same exposure in a conditioned set; this run's
are in sets with no condition or are covered by an `Added`/`Changed` row on
the drawn entity.

A sim slice that gates on a message or a change runs
`cargo test -p ambition_app --test app_it -- canonical_reconstitution` before
it is pushed.

### F44 — Desktop A/B of the run: the main thread's work halved (2026-10-23)

MEASURED (desktop VM, `profiling` build of `capture_scene`, xvfb with
llvmpipe, hub Mary-O route, `--warmup 1500`, census on; three interleaved
rounds of each binary, about 100 one-second windows each; script
`scratchpad/ab/ab_interleave.py`): the run's start (`315d36bf0`) against
HEAD (`2902cf64e`).

| per frame (median of windows) | run start | HEAD |
| --- | ---: | ---: |
| systems run | 1,362 | 1,099 |
| main-world wall, summed over the main schedules | 13.45 ms | 7.08 ms |
| `Update` wall | 4.34 ms | 1.60 ms |
| `PostUpdate` wall | 5.07 ms | 2.85 ms |
| `PreUpdate` wall | 2.10 ms | 1.67 ms |
| `SpawnScene` wall | 0.80 ms | 0.07 ms |
| `Update` process CPU (every thread) | 35.3 ms | 6.8 ms |
| frame p50 | 23.2 ms | 23.0 ms |

The hub's Ambition route, the same way (about 220 windows a side): systems run
1,540 → 1,261; main-world wall 14.72 → 7.17 ms; `Update` 3.86 → 1.90 ms;
`PostUpdate` 6.60 → 2.53 ms; frame p50 50.9 → 48.0 ms.

REASONED from the tables: the frame did not get much shorter here because
llvmpipe draws it on the CPU and bounds it; the main thread's freed time
became waiting (the `outside` bucket, 10.4 → 16.3 ms). The census warns that
render blocking lands in whichever phase brackets it, and both sides draw the
same scene, so the deltas are the work removed, not a frame-time claim. The
`Update` CPU drop is mostly F41 (main-world schedules serial: no worker
threads woken for systems that do nothing). On the phone, which F40 found
CPU-bound, this is the number that becomes frame time; that is the next
measurement, and it needs the phone.

Found on the way: the A/B worktree must carry the untracked generated assets
(FX sheets, parallax tiers, audio) or the base never leaves the launcher; the
first attempt compared HEAD's gameplay with the base's "load failed" screen
(751 systems). `cp -rs` of the main tree's asset files into the worktree.

**The run's last A/B (19:55Z, the same method, HEAD `8aae83270`).** The
machine was slower than at 10:20 (the base's own wall rose 13.45 → 15.27 ms),
so read each row against its own base:

| per frame (median of windows) | Mary-O start | Mary-O HEAD | Ambition start | Ambition HEAD |
| --- | ---: | ---: | ---: | ---: |
| systems run | 1,362 | 972 | 1,540 | 1,141 |
| main-world wall, summed | 15.27 ms | 7.89 ms | 15.93 ms | 8.47 ms |
| `Update` wall | 4.99 ms | 2.03 ms | 4.52 ms | 2.25 ms |
| `PostUpdate` wall | 5.39 ms | 2.72 ms | 6.69 ms | 2.84 ms |
| `SpawnScene` wall | 0.73 ms | 0.08 ms | 0.64 ms | 0.07 ms |
| `Update` process CPU | 36.1 ms | 9.1 ms | 28.2 ms | 10.3 ms |
| frame p50 (llvmpipe-bound) | 30.3 ms | 28.1 ms | 56.5 ms | 55.6 ms |

The afternoon's slices took the systems run from 1,099 to 972 on Mary-O; the
main-world wall's share of its base stayed about half (46% → 52% removed).
REASONED: the late slices gate passes that were cheap per run, so on this
host they count in systems rather than milliseconds; on the phone's executor
each run's overhead is larger (F3). The phone measurement is still the one
that says what this buys there.

### F43 — What still changes, spawns and allocates each frame (2026-10-23)

MEASURED (desktop, dev build; the census rows `changed_transforms` and
`changed_sprites` added in this run, and a `perf_probe` build for
allocations and churn; `capture_scene --warmup 600-900`, counts not times):

| per frame | Mary-O route | hub (Ambition) | `basement_enemies` |
| --- | ---: | ---: | ---: |
| `Transform` written | 68-145 (rig parts, motes, actors) | 53 (was 83) | 98 |
| entities spawned (`Added<Transform>`) | 0 | 1 (portal-gun pickup, by design) | 0 |
| `Visibility` written | 4 (dev HUDs; was 14) | 8 (was 28) | 2 |
| UI `Node` / text written | 0 / 0 | 6 / 3 during the intro cutscene, before the overlay fix | 0 / 0 |
| allocations in our systems | ~1,300 | | ~1,000 |
| resource archetype moves of ours | 0 (was 26) | | |

What is left is mostly real per-frame work: animated rig parts and ambient
motes move, actors move their nameplates and hit-flash mirrors. The largest
allocators left: the input action update (262, already cut from ~19,000 by
`press_all_action_update`), the rollback lifecycle commit (147), body
integration (135), the room-transition prefetch trio (~220, which ranks
neighbours by where the players stand), the character animator in a room
of enemies (135) and the nameplates (62-82). None is a slice: the phone
profile decides whether allocation is a lever at all.

REASONED: on the systems-run axis the hub's Mary-O frame went 1,320 → 1,163
in this run, and the per-frame churn of what is on screen is now near the
set of things that move. The next lever needs the phone: a profile after
the serial schedules (F41) and these slices.

### F42 — Where the hub's Mary-O frame's systems are now (2026-10-23)

MEASURED (dev build, `capture_scene --route mary_o_gameplay --warmup 600`,
`AMBITION_SYSTEM_LIST`, head `87da084a0`): 1,195 systems ran in one frame
(1,320 at the start of this run). By schedule: the simulation 432 a tick
(28 of them the census's own markers), `Update` 387, `PostUpdate` 137,
`PreUpdate` 164. Not on the phone: the census and probe markers (40), the
developer tools and the app's `dev` module (76 with the markers), egui
(16).

What is left, by kind (MEASURED counts, REASONED reading):

- **Bevy's per-type systems, 339 in all.** `Assets<T>::track_assets` 53
  (one per registered asset type: Ambition's room-look, sauce, flame, space
  and dream materials run in Mary-O's frame too); the Material2d
  specialization checks 25 (one per material); gizmo groups 6 systems each
  (37: start, end, clear, collect, propagate, meshes), of which 16 were the
  four `RoomTrailGizmos` bands (now one retained drawing per band) of `Capability::Trails`, which Mary-O's rooms
  do not use. None of these can be gated from outside: Bevy refuses to
  configure a system-type set, and a condition per system costs what it
  saves. They are composition items: a material or gizmo group registered
  only by a composition whose games use it (P1), the trail drawn as one mesh
  per band instead of four gizmo groups, or a bevy fork that gates a group's
  systems on its storage being empty. Run by Jon first.
- **The simulation, ~404 a tick.** Mary-O's own rules and bodies, combat,
  the extension host's lowerings (~30 a tick, each draining an outbox; a
  phase-wide gate is not exact because the held ports, such as a held
  damage box, a held status or a limb bond, release on an empty tick, and
  a remembered "last tick" is not rollback-safe), the effect quarantine (16
  plus 5 in `PreUpdate`, one per effect type) and the sim-view
  publishers (31, S7c).
- **Rendering reconciliation**, `ambition_render` 144: mostly real per-frame
  work (animation, rigged parts, camera, parallax, HUD). The rigged parts
  already write only on change (`rigged.rs` `write_slots`); its 34 part
  transforms a frame are real animation.

Next by leverage: S7c (fuse the sim-view publishers sharing one body
traversal), the held-port classification that would let the drain-only
extension lowerings sleep per phase, and the composition items above.
(Since done: the lowerings, `670a8d4f4`; the shield pair, `e97a14eec`.)

**The Ambition route** (MEASURED, same method, after `1e7616973`): 1,342
systems a frame, against the Mary-O route's 1,160. Other games' systems left
on it: Mary-O's 13 (her five materials' Bevy per-type systems and two reset
ledgers) and two of Smash's fighter-special rules, which Ambition's cast
uses too. The boss domain runs 24 systems a tick and Ambition's per-boss
rules 12 in the hub room, where no boss stands: they are scoped to
Ambition's rules, and a finer scope ("a live room holds this boss") must
keep the save-ledger systems (defeat retraction, a boss's music release)
running everywhere and the arena trackers' one tidy-up frame. REASONED: a
per-room capability use derived from a room's authored placements is the
general form; it is a design step (P18), not a gate.

### F41 — The hub's Mary-O route by span, and the schedules left parallel (2026-10-22)

MEASURED (profiling build with `perf_probe`, `capture_scene --route
mary_o_gameplay --warmup 900`, frames after 300, the VM; read ranks and
ratios, the probe adds cost to each span):

- **Schedules outside the serial policy.** `serialize_main_schedules`
  named twelve schedules and ran before the plugins that make the rest, so
  ~30 schedule runs a frame were on the multi-threaded executor. Each paid a
  hand-off to the pool for a few systems: `ShelterFontSources` (one system)
  334 µs a frame, `ProcessLdtkApi` (four) 469, `ReadInputs` (three) 96.
  `SerialScheduleSweep` serializes every main-world schedule in
  `Plugin::cleanup` (`42e1843d4`): 8, 24 and 28 µs, and no multi-threaded
  executor span left. On the phone each hand-off woke pool threads
  (the compute pool's four threads were ~13% busy each in F40's profile);
  the next phone profile measures it there.
- **Avian** stepped on ~60% of frames while debris lived: ~1 ms a stepped
  frame here, of it ~200 µs in the substep systems themselves. Its
  schedules are already serial (avian sets them); the rest is the probe's
  cost per span over ~120 spans a step. Not a lever found here.
- **Run conditions that read the live rooms**: `room_uses` 13 calls a frame,
  `in_rules_scope` 14, `session_world_exists` 35, ~1.5-1.8 µs a call. One
  reading of the live rooms per tick would serve them all, but a room can
  change inside a tick (a transition commits in the simulation), so a
  cached reading must be refreshed after the commit too, under rollback.
  Not done; a measured candidate.
- **Material2d specialization checks** (upstream `check_entities_needing_specialization`,
  a `par_iter` with asset-change filters each frame for each material
  type): hit flash 57, Mary-O's shimmer 47, the portal clip 47, the bubble
  shield 26, the rigged parts 22 µs a frame. A fork item for Jon.
- **Changed every frame for nothing** (`changes_by_site.csv`): the animator
  writes each animated sprite's `Anchor` (34 a frame) and `Sprite` colour
  each frame with no compare (`draw_animator_frame`,
  `apply_character_frame`), so each of them is extracted and re-bounded
  each frame. Bevy UI re-inserts the target camera of each of ~30 UI roots
  each frame (`propagate_ui_target_cameras`, upstream; its propagation to
  children compares, so layout does not follow from it).
- **Extension lowerings** (~30 a tick in Mary-O) each drain an empty
  outbox. A condition on the whole `ExtensionSet::Lower(phase)` is not
  exact: `bond_limbs` and others do per-tick work beside the drain.

### F40 — Is the phone still bound by the scheduler? (2026-10-22)

MEASURED (phone, `android-all-mary_o-auto-20261022T213742Z`, build
`315d36bf0`, the main thread tid 28744, `symbolize_android_profile.py
--ours --by-system --thread`, rows grouped):

| run | main thread, % of process | in our systems (framework under them included) | framework systems | the executor's own loop | run conditions |
| --- | ---: | ---: | ---: | ---: | ---: |
| `maryo-1-1` (10-16) | 53.5 | 55.2% | 26.1% | 13.0% | 3.5% |
| `maryo-1-1-morning3` (10-16) | 49.6 | 56.4% | 24.0% | 14.2% | 2.9% |
| `maryo-terrain` (10-16) | 38.0 | 58.9% | 20.0% | 14.8% | 3.9% |
| `mary_o-auto-…213742Z` (10-22) | 34.6 | 52.3% | 22.9% | 13.0% | 2.7% |

The main thread is 83% busy (one core), so it is still the limit; the
other threads are not (the async pool's 19% in this run is the autopilot's
navigation build, autoplay only). The executor's own loop and the run
conditions are ~16% of the main thread and have not changed share since
10-16: the per-frame work fell with the system count, but each system still
pays the executor. F0's "a third" counted every sample with no system body
on the stack, a wider set, and is not comparable with this column.

MEASURED (desktop census, `capture_scene --route mary_o_gameplay --warmup
600`, dev build, `AMBITION_SYSTEM_LIST`): 1,320 of 2,121 systems ran in one
frame (1,394 on 10-17, F37). Still running for nothing while Mary-O plays:
Smash's select screen 15, portal presentation 19 and portal publishers 6,
Ambition's room looks, overlays, vanity card and Inugami field ~30, Sanic's
dressers 3, the versus roster 3.

Answer (REASONED from the two): yes, in part. The scheduler's share is
steady at ~16% of the bound thread, so each system that runs for nothing
costs the frame on the phone; removing them is the lever with no behaviour
change. The rest of the bound thread is real work in our systems and in
Bevy's transform, UI and visibility systems (`mark_dirty_trees` ~2.8% of
the main thread, `ui_layout_system` ~2.4%, `check_visibility_cpu_culling`
~1.5%, material specialization ~1.4%), which scale with entities and
changes, not with the system count.

## Hub first (2026-10-16, from 13:30Z)

**Target, from Jon:** the four-game app with Mary-O active, on the phone
("It needs to run fast when I open up the main 4 game app and play maryo").
Standalone Mary-O is a baseline for diagnosis only. D12 is approved at low
priority. Priority order: P12/P13, then P1/S2/S2b/P3, P5, F19, the hub's frame
executor, D12, the phone render scale. No tick-rate cut, no physics rewrite,
no quality cut in place of a fix.

**The phone after last night's fixes** (MEASURED, `android-all-maryo-1-1-morning`
and `-morning3`, build `f66b3a8aa`, all fixes in it, no throttling in the
cpufreq logs; p50 per 5 s window):

| scene | last night (`android-all-maryo-1-1`) | this morning |
| --- | ---: | ---: |
| hub room (`hub_clean`) | 35-45 ms | 13-17 ms |
| Mary-O 1-1 in the hub | 24-33 ms | 17 ms at entry, 23-33 ms during play, 36-46 ms in 1-2 |

The first morning run recorded the hub room only (Mary-O started after the
recording window). ⚠ Every phone profile up to 2026-10-16 is of an
INCREMENTAL `android-profile` build: `build_for_android.sh` did not set
`CARGO_INCREMENTAL=0` (fixed 10-16, after a link failure). Incremental makes
smaller codegen units and less inlining than the shipped build, so the
framework shares below may be high; the next run is the first like-shipped one. `AMBITION_BOOT_ROUTE=mary_o_gameplay` now boots the hub in
Mary-O, so a phone profile cannot miss it (`profile_android.sh` records it).

**Reconciliation:**

| item | state | hub effect |
| --- | --- | --- |
| leafwing clash pass (F13) | done | the hub's input allocations 31,942 → 258 a frame |
| star re-fold, parallax respawn, room isolation, SFX, HUD, seat bindings (F14-F21) | done | measured on the hub probe; not alone on the phone |
| rigged part slots and label colours written only on a change (F26) | done 10-16 | ~136 slot `Transform`/`Sprite` writes a frame, which woke propagation and extraction |
| LDtk respawn clean with a kept query state (F25) | done 10-16, replaces D5 | 1.04% of phone samples |
| ~9.4k allocations a frame with no system owner (old item 7) | resolved: tracing spans of the `trace` feature (F27) | none in the APK |
| UI, HUD, touch and sprite writes only on a change (F29) | done 10-16 | text re-layout 3.2 → 0.2 a frame; all changes −18% |
| P1/S2b: foreign games and capabilities while Mary-O plays | in part, 10-16/17 (F31): game scope sets; boss encounters, dialogue, held use and fighter specials sleep where unused | GGRS tick 615 → 470 systems, frame 1,602 → 1,444 |
| P5 read-models per frame | open | the `ambition_sim_view` rebuilds are ~0.9% of phone samples |
| F19 live-room lookups | open | `room_instance.rs:172` and `room_graph.rs` ~1.9% |
| repertoire re-fold in the hub | open, small | 1.3% of phone samples; 0.04 re-folds a frame on the desktop, so room loads and power-ups, not steady state |
| D12 | done 2026-10-17: `serialize_main_schedules` in every foundation, the switch deleted | none in the hub (its simulation is GGRS, serial already) |

**Objection to the order, from the code and the phone:** the remaining P12
items are small on the phone; the cost is in how many systems run and in
third-party systems (F25, F28). So P1/S2b goes before the rest of P12.
Repeat allocations are no longer the main cost: the hub with Mary-O makes
~4,900 a frame of its own (desktop probe, without the probe and the spans),
near standalone Mary-O's 4,128.

### F35 — Exclusive systems that do their work each frame (2026-10-17)

MEASURED (probe, hub `basement_enemies`, ~600 frames, `time_by_span.csv`
joined with each `fn x(world: &mut World)`): the exclusive systems of ours
that ran each frame cost ~130 µs a frame, and most of it was no work. Two
causes. A query made new with `world.query()` or `try_query()` matches each
archetype of the world (~2,000 in the hub), and `session_world_entity` made
one at each call. A `resource_scope` that passes `&mut` through the `Mut`
marks the resource changed even when nothing changed.

Done: `advance_pending_route` runs only while a route is pending
(`a_route_is_pending`), 33.3 → 0 µs (its condition 2.0 µs).
`SessionRootQuery` keeps the session roots' query state, so
`session_world_entity` and `session_root_owned_by` update it only for new
archetypes: `enforce_session_contract` 31.8 → 8.9 µs, `maintain_local_session`
15.5 → 6.9, `commit_confirmed_lifecycle` 17.7 → 8.0.
`release_departed_experience_state` runs only on a frame that wrote the
router. Witnesses `an_idle_frame_does_not_run_the_pending_step`,
`the_live_root_is_read_through_the_kept_query_and_a_later_root_is_found`,
`the_shell_releases_on_a_departure_and_an_idle_frame_writes_no_scope`.

The same pattern outside exclusive systems: `publish_shell_ui_cues` called
`ActiveUiCues::sync` through `ResMut` each frame, so the cues were marked
changed each frame (`changes_by_site.csv`: 1.000 a frame) and
`rebuild_control_prompt`, which derives on change, derived each frame
(44 µs). `ActiveUiCues::sync_marked` marks them only when they changed; the
shell, Smash's select screen and the kaleidoscope call it (witness
`a_cue_published_again_unchanged_does_not_mark_the_cues_changed`).
MEASURED (the same probe after): `rebuild_control_prompt` 44.2 → 12.5 µs.
`prepare_room_plates` found each live and neighbouring room's architecture
pieces each frame to compare them (34 µs); it now returns at once while the
live rooms, the room set and the plate count are as it left them and no
plate draws (witness `a_quiet_frame_prepares_no_plates`): 34.5 → 3.7 µs.
`release_departed_experience_state` 18.5 → 0 µs (its condition 1.9 µs).
The debug HUD and quest panel text, the map's status text and visibility,
the deep-dream overlay and the shield bubbles wrote each frame with no
compare (a changed `Text` is laid out again); they use `set_if_neq`, and
their rows of 1.000 a frame are gone from `changes_by_site.csv`. The
content watchers (`watch_content_sources` 81 µs) are not in the phone build
(`static_content`).

### F36 — The hub's Mary-O route, after the dormancy slices (2026-10-17)

MEASURED (probe, profiling `capture_scene --route mary_o_gameplay`, ~580
frames): systems run 1,452 a frame, 425 in the GGRS tick. Avian stepped on
~63% of frames: combat debris (an impact, a ragdoll, a broken block) wakes
it for its lifetime, and while awake it cost ~2.6 ms a frame. Of that,
616 µs was `block_on_optimize_trees`: avian clones the debris tree into an
async task and waits on it. It now optimizes in place on the main thread
(no spatial query runs during the step): `run_physics_schedule` 1,537 →
805 µs a frame (witness `avian_optimizes_its_trees_in_place_on_this_thread`).
Yarn's `fetch_resources` (146 µs) and `continue_runtime` (38 µs) ran with no
dialogue; its public `YarnSpinnerSystemSet` now sleeps where the room's game
uses none, once its project is compiled (witness
`yarn_sleeps_in_rooms_whose_game_uses_no_dialogue`). Not a fork edit after
all. Left on this route: the solver's six substeps (383 µs while awake; a
visual change for cosmetic debris), the rigged presentation (145 µs), the
body step (144 µs), and the render world, which is most of the frame.
Found on the way: `ParticleBudget::max_particles` and `spawn_rate_scale`
(16 on Potato, 1,024 on Ultra) were read by nothing but their own tests, so
the quality tiers did not reduce the debris that wakes avian. Now each burst
throws `count × spawn_rate_scale` pieces (at least one) and no burst passes
`max_particles` live pieces; a burst at the cap is dropped before it builds
its room's floor colliders. On Potato a ragdoll throws 1 piece, not 7
(witness `a_burst_throws_what_the_particle_budget_allows`). With no
`ResolvedVisualQuality` (tests, tools), every piece.

### F37 — What still runs for nothing on the Mary-O route (2026-10-17)

MEASURED (same probe as F36; `[census] systems_ran` by crate; µs a frame
from `time_by_span.csv`). After the two slices below, systems run 1,446 →
1,394 a frame. Shipped on the phone (`android` features):
- `bevy_lunex` 35/35 systems, 82 µs: it lays out only the kaleidoscope
  cube, and ran with the cube closed. Its public `UiSystems` sets now sleep
  with the cube's own render sets (`kaleidoscope_render_needed`): 28
  systems idle, 82 → 48 µs (witness `lunex_sleeps_while_the_cube_rests`).
  The 15 left (its software cursor, 2D picking, text animators) are in no
  public set.
- `bevy_transform_interpolation` 19/19, 41 µs: avian's
  `PhysicsInterpolationPlugin` interpolates no body by default and none of
  ours asks; it is disabled, 41 → 0 µs (witness
  `avian_runs_no_interpolation_for_bodies_it_does_not_interpolate`).
- `bevy_pbr` 169 µs and the OIT resolve bind group 130 µs with no 3D camera
  active: the cube is the only 3D, and OIT is added inside
  `CorePipelinePlugin`, so neither can be left out from a plugin group. A
  fork or upstream item; the probe's software GPU inflates both. Bevy's GPU
  mesh preprocessing is ~185 µs of this (`write_batched_instance_buffers`
  109 µs, counted under `bevy_render`; `collect_meshes_for_gpu_building` 54;
  `extract_meshes_for_gpu_building` 22), a fixed task-pool fan-out each frame. The phone does not pay it: Bevy turns
  GPU preprocessing off for Adreno 730 and older, and the profiled Pixel 5
  is an Adreno 620. A desktop can turn it off with `PbrPlugin {
  use_gpu_instance_buffer_builder: false }`, but only in builds whose
  feature set includes `bevy_pbr` (`set` panics on a plugin the group does
  not hold), so it is not done.
- The FPS frame-time graph, ~180 µs (its material is re-uploaded each
  frame): on by Jon's choice (2026-08-31, `dev/fps_overlay.rs`).
- `virtual_joystick` writes the stick's `Node`s each frame (the `JoystickFixed`
  behaviour zeroes `base_offset`, ours sets it back), so the touch UI lays
  out each frame. A fork edit (compare before write).
- `ambition_load_presentation` 15/15, 40 µs between loads; `CurrentRoom`
  conditions (`in_rules_scope`) ~3 µs a call, 12 calls a frame (F19).
Not on the phone (`static_content`, no `dev_tools`): the pack and content
watchers (`look_at_the_pack_sources` 197 µs, `watch_content_sources` 133 µs,
`poll_world_source_changes` 70 µs, `watch_movement_defaults` 33 µs), egui
(129 µs) and the trace recorders (81 µs, already off on android). A desktop
probe of the dev build over-counts by about 0.6 ms a frame for these.

### F25 — Where the phone's main thread goes, hub with Mary-O

MEASURED (`android-all-maryo-1-1-morning3`, main thread tid 28006, 83% busy):
50% of all samples are the main thread; 30% are main-thread samples with no
frame of ours (framework only). Of these, by the system they are in
(`scripts` helpers in the run log): the executor's own loop
(`is_apply_deferred`, `try_run_schedule`, `run_schedule`) ~6.6%, `avian2d`
2.0%, `bevy_ui` layout and text 1.8%, LDtk `clean_respawn_entities` 1.04%,
Yarn `fetch_resources` 0.77%, `bevy_light` directional-light visibility
0.49%, transforms and visibility ~1.2%. Rollback: the shipped one-player
session is a sync test with `check_distance: 0`, so GGRS saves nothing per
tick; `SaveWorld`'s 1,146 systems do not run each tick. The archetype count
does not grow during idle play (2,049 after 15 s, desktop, new
`[frame-census]` columns); each query that is made new each run matches all of
them, which is the cost of the LDtk, Yarn and `bevy_light` systems. LDtk is
fixed (`ambition_platformer2d_ldtk`'s `bevy_runtime::clean`, the hub's `app::ldtk_clean`
until 2026-10-21). Yarn's system and set are private to the crate: an
upstream fix (keep the `SystemState`) or a fork, for Jon.

### F26 — Rigged part slots were written every frame

MEASURED (desktop probe, hub Mary-O route, 893 frames): `write_slots` wrote
each slot's `rect`, `custom_size`, anchor and `Transform` each frame, ~136
entities, with no change; the impostor root did the same. That gave 232
`TransformTreeChanged` and 151 `GlobalTransform` changes a frame and sprite
re-extraction. Now each field is written only when it differs.

### F27 — The schedule-level allocations are tracing spans

MEASURED (`AMBITION_PERF_PROBE_TRACE`, hub Mary-O route): of ~9,550
allocations a frame inside schedule spans with no system span, 3,660 are the
`name` field format and 3,400 are `Span::new` (one span per system run), 1,500
the capture tool's loop. Only a build with Bevy's `trace` feature (probe,
Tracy) makes them; tracing is 0.2% of the phone profile.

### F28 — Foreign systems that run while Mary-O plays in the hub

MEASURED (`[census] systems_ran`, desktop, hub Mary-O route; 1,599 of 2,110
registered run): `ambition_content` 81, `ambition_demo_smash` 49, portals 39,
`ambition_boss_encounter` 32, `bevy_pbr`+`bevy_light` 33, `avian2d` 20,
`bevy_egui` 16, conversation and Yarn ~23, `ambition_demo_sanic` 4: ~300.
REASONED cost on the phone: ~20 ms of framework time over ~2,200 system runs a
frame is ~9 µs a run, so ~3-5 ms a frame. S2b step 1 (`AMBITION_HUB_GAMES`)
removes `ambition_content`'s at build time; the shipped app needs them dormant
at run time instead (P1/S2: a game's systems in its own schedules, run only
while its route is live), so the hub can still switch games.

### F29 — UI and sprites marked changed each frame, hub with Mary-O

MEASURED (desktop probe, hub Mary-O route, before → after, a frame): touch
control `Node`s 20 → 0, HUD `Text` 3 → 0 and `Node` 2 → 0, Bevy text
re-layout (`ComputedTextBlock`, `TextLayoutInfo`) 3.2 → 0.2, rigged part
slots 407 → 0, label colours 35 → 3.7, all value changes 2,449 → 2,003. Two
causes: writes without a compare, and helpers that take `&mut T`. A `&mut T`
made from a `Mut<T>` marks the value changed before the helper compares; the
HUD's `set_text_if_changed` did that. A helper that may not write takes the
`Mut`. On the phone `bevy_ui` layout and text were 1.9% of the main thread;
not yet measured there after the fix. Left: `virtual_joystick`'s `update_ui`
(2 `Node` a frame), private in the fork (`Erotemic/virtual_joystick`).

The ~18-a-frame changes of body components (`ActorRoll`, `PogoTargetVolumes`,
`RangedRefire` and 10 more) have no `Changed<>` reader: they cost nothing
downstream and are left.

### F30 — The GGRS tick runs other games' systems, twice a frame on the phone

MEASURED (`AMBITION_SYSTEM_LIST`, desktop, hub Mary-O route): 612 systems ran
in `GgrsSchedule`; ~150 belong to other games or unused capabilities (Smash
35, boss encounters 34, `ambition_content` 30, portals 17, conversation 15,
mounts 7, encounters 9). At 30 fps the hub runs ~2 ticks a frame. REASONED
cost on the phone: GGRS framework-only time (10.2% of samples) over ~1.1 M
system runs in 30 s is ~4.6 µs a run, so the ~150 cost ~1.4 ms a frame, ~2 ms
with the frame schedules' foreign systems. A faster frame also runs fewer
ticks per frame, which is the larger effect.

### F31 — Dormancy: a game's rules and the engine's capabilities sleep where unused (2026-10-16/17)

Two sets, both keyed by the active room, so a session that visits another
game's room is served there, and a rewind restores what the condition reads:

- `RulesScopeSet(scope)` (`ambition_platformer2d_runtime::mode_scope`): the
  systems of one game's rules, with `in_rules_scope(scope)` as the condition
  in each schedule (`add_scoped_systems`). Ambition's gameplay systems are in
  `AMBITION_RULES` (the untagged rooms).
- `CapabilitySet(capability)` (`shared_tangle::capability`): a capability's
  per-tick systems. `ModeScopePlugin` gives each set `room_uses(capability)`:
  a live room's game declared `UsedCapabilities` naming it, or declared none.
  Mary-O and Sanic declare no boss encounters, dialogue, held use or fighter
  specials; Smash declares no boss encounters or dialogue.

MEASURED (desktop capture, hub Mary-O, `AMBITION_SYSTEM_LIST`): systems run a
frame 1,602 → 1,444; in the GGRS tick 615 → 470. Smash 49 → 15, held items 9
→ 0, abilities 24 → 3, boss encounters 34 → 9, conversation 42 → 13,
`ambition_content` 62 → 47. MEASURED (phone, `android-all-maryo-dormant`
against `-auto`): 1-1 median 22-30 → 20-25 ms; 1-2 unchanged (33-36 ms).
Left (portals, mounts and encounters done below): cutscenes and inventory are
used by Mary-O;
Ambition's hub room and the presentation systems; standalone builds that
compose only what they declare (P1).

### F32 — The terrain skin was an entity for each piece (2026-10-17)

MEASURED (desktop census, Mary-O 1-2): 1,993 of 3,423 sprite entities were
pieces of the terrain skin. They are now `SkinQuad`s drawn as a few `Mesh2d`
chunks for each room (`terrain_skin::lay_room_terrain`): sprites 3,423 →
1,250, transforms 4,025 → 2,025, the same 1-2 frame pixel for pixel. A block
that goes now gives its neighbours the trims of their open edges. MEASURED
(phone, `android-all-maryo-terrain`): 1-2 entities ~5,800 → ~3,800, but its
median frame did not change (31-37 ms); the render thread went 36% → 33% busy,
and the visibility and transform systems lost ~0.6 points of samples. The
entity count followed the cost of 1-2; it did not cause it.

### F33 — What makes 1-2 dear: the body step and the collision world (2026-10-17)

MEASURED (phone, `android-all-maryo-terrain`, `symbolize_android_profile.py
--by-system --thread busiest --window 44,60 --window-b 2,30`, 1-2 play
against 1-3 play in one run): as a share of the main thread's samples, 1-2 is
heavier in `integrate_sim_bodies` by 5.9 points, `tick_actor_brains` 1.7,
`update_body_mode` 0.6, `apply_player_hit_events` 0.5. Under them, the
collision-world composition (`world_with_sandbox_solids`,
`world_with_moving_platforms`) is ~1.6% of all samples in 1-2 and ~0.8% in
1-3: ~4% of the main thread in 1-2. These are the four composers of F24. Each
copies the room's blocks, so a large room makes each copy dearer, and the
phone runs ~2 ticks a frame. REASONED: a slower frame runs more ticks a
frame, so every cost for each tick is paid more often. Per-tick cost is the
multiplier. Lever: P14.

### F34 — A run condition on each system costs on each system (2026-10-17)

MEASURED (phone, `android-all-maryo-dormant`, by system): the conditions that
read `CurrentRoom` are 0.57% of samples. Mary-O and Sanic gate each of their
systems with its own copy of `in_rules_scope`, so the condition runs once a
system. MEASURED: `bevy_light`'s `check_dir_light_mesh_visibility` and its
cluster assignment run for the 2D games (~0.45% and ~0.11%). Levers: move each
game's systems into its `RulesScopeSet` (P18); S2b for 3D lighting.

**P18 done for Mary-O and Sanic (2026-10-17).** Their rules are added with
`add_scoped_systems`: one `in_rules_scope` condition on the set, asked once a
schedule run. `run_if` on a tuple had put a copy on each system. MEASURED
(the schedule graph of the shipped standalone Mary-O app): members of her
`RulesScopeSet` in the sim schedule 0 → 38, so 38 condition calls a tick
become one. Witness `her_rules_are_members_of_her_scope_set` (`mary_o_it`;
red at 0 with the old registration). Sanic's eleven groups moved the same
way, with no witness of their own. `install_attempt_scoped` keeps its
condition on its two systems in each game.

**S2b: the light checks sleep without a light (2026-10-17).** Only the hub
installs Bevy's `LightPlugin`: the cube menu's `bevy_pbr` turns it on
(MEASURED: no `bevy_light` system in the standalone Mary-O census below). No
crate of ours spawns a light, and the cube is unlit. Source read
(`bevy_light` 0.19.1, `check_dir_light_mesh_visibility`): with no light the
system still queues a command that builds `world.query::<&mut
ViewVisibility>()`, a new query state over every archetype, each frame.
`sleep_light_visibility_without_lights` (`ambition_menu_kaleidoscope`) puts
`SimulationLightSystems::CheckLightVisibility` under one condition: a
directional, point or spot light exists. Witness
`the_light_checks_run_only_while_a_light_exists`. The cluster assignment
(~0.11%) is left: the cube's 3D camera needs its clusters.

**Standalone Mary-O census (2026-10-17, debug `capture_mary_o --warmup 600`,
`bevy/debug`, 1-1).** MEASURED: 939 of 1,264 of our systems ran in one
frame. Still running for what she does not use: Kira channel systems 30
(D6), Bevy gizmo groups 32 (D9), capture (grabs) 14 (D4), the narrative
ledgers' `forget_narrative_inputs_on_activation` 9, developer-tool
`editable` 8, mounts 7, boss ticks 5, and in `features::ecs` the pets,
errands, challenges, wear, summons, fighter ladder, barks and boss bodies
(~20). No `bevy_light` system.

**The hub with Mary-O playing (2026-10-17, profiling `capture_scene --route
mary_o_gameplay --warmup 600`, `perf_probe` build, which keeps system
names).** MEASURED: 1,403 of 2,081 of our systems ran in one frame. Foreign
to her: portals 38 (19 in `GgrsSchedule`, 19 presentation in `Update`),
Ambition's content 64 (portal adapters, the cut-rope boss, room looks,
deep-dream and sauce overlays, the vanity card), Smash 15, developer tools
16, boss encounters 8, mounts 7.

**Portals sleep where the room's game uses none (2026-10-17).**
`Capability::Portals`: the portal sets that act (`Pose`, `InputAdapter`,
`InputWarp`, `WeaponAndProjectiles`, `TransitGuards`, `Frame`, `Transit`)
are in its `CapabilitySet`. The publishers (`Carves`, `Passages`), the
orphan cleanup and the reset stay outside it, so a room with no portal
publishes an empty answer, not the last answer of a room with portals.
Mary-O declares no portals; Sanic and Smash keep them (`portal.rs`; Sanic's
Act 3 rifts are portals: declared without them, the rift slept and
`three_acts` went red, 2026-10-17). `a_game_declares_each_capability_its_maps_author`
now compares each game's declaration with the entities its LDtk worlds hold
(red, naming Sanic's `Portal`, with the old declaration). With
two live rooms the set runs while ANY live room uses the capability
(`GoverningRules::any_live_room_uses`, the reading `CurrentRoom::in_scope`
gives game scopes); it ran in each two-room frame before, because THE room
read as none (review P3, 2026-10-17; witness
`a_capability_sleeps_while_no_live_room_uses_it`). Witness
`portals_sleep_in_mary_os_rooms_and_run_in_ambitions` (`app_it`, build-time
over the composed shipped host): red at "Mary-O's rooms use portals"
without her declaration, and at "the portal transit is not in the portals'
capability set" without the set membership. The 19 portal presentation
systems in `Update` are not gated yet: they despawn visuals, so each needs
the same publish-or-clean reading first. MEASURED (the same census after it): systems run in
the GGRS tick 472 → 446 (portal-named 36 → 10), in the frame 1,403 → 1,374.

**The boss steps and the mounts sleep too (2026-10-17).** Six per-tick boss
steps were outside the boss capability's set (`integrate_boss_bodies`,
`update_ecs_bosses`, `tick_boss_brains_system`, `drive_boss_animators`, the
boss attack trigger and projection); no Mary-O, Sanic or Smash content makes
a boss body, and they now join it (witness
`the_boss_steps_are_in_the_boss_capability`). `Capability::Mounts`: the
mount crate's seven systems, through its sets; Mary-O and Sanic declare no
mounts, Ambition and Smash keep them (witness
`mounts_sleep_in_mary_os_rooms_and_run_in_ambitions`, red at each half).

**Wave encounters sleep too (2026-10-17).** `Capability::Encounters`: the
lifecycle reducer and the cleared list, the occurrence projection, the wave
driver, its effects and cleanup, the lock walls and the reward chests (nine
systems). Mary-O and Sanic declare no encounters (their maps hold no
encounter entity); Smash keeps them undeclared. The switch drain, the switch
index and the banners stay awake. `UsedCapabilities` is a `u16`. Witness
`encounters_sleep_in_mary_os_and_sanics_rooms_and_run_in_ambitions`, red at
each half.

**Hub frame by system (2026-10-17, `basement_enemies`, the same build,
`time_by_span.csv`).** MEASURED: Avian's transform propagation before each
fixed step ran with avian paused (`mark_dirty_trees` 245 µs,
`propagate_parent_transforms` 76 µs, `sync_simple_transforms` 62 µs a
step); it now sleeps while avian is paused (witness
`the_propagation_before_physics_sleeps_while_avian_is_paused`). The
desktop-only costs (pack and content watchers ~300 µs a frame as a spike
every 20 frames, the FPS graph, the traces) are not on the phone. Left for
the phone: `ProcessLdtkApi` ~0.55 ms a frame (P8), Yarn's
`fetch_resources` 120 µs (fork item), and `check_entities_needing_specialization`
35-44 µs for each of four materials.

### F38 — What 1-2's simulation costs, measured on the desktop (2026-10-17)

The review asked for 1-2's simulation cost on the phone (P2). No device is on
this machine, so the same question was asked on the desktop:
`capture_mary_o --room mary_o_1_X --warmup 300 --walk 900`, profiling build
with `perf_probe`, `AMBITION_PROFILE_CENSUS=1`, 1-1, 1-2 and 1-3 in one
session (the box loaded by a parallel `app_it` in each run, so read the
ratios, not the absolute times).

MEASURED, per tick, 1-2 against 1-3: our simulation systems grow by ~390 µs.
`integrate_sim_bodies` is 243 of it (420 against 145 µs a call); the
developer trace recorders are 79 (already off on Android); then
`rebuild_feature_view_index` 18, and no other system more than 5. In
`Update`, `dress_power_blocks` was 143 µs a frame in 1-2 (23 in 1-1): it
decoded the name of each of the 332 blocks each frame.

MEASURED (temporary spans inside `integrate_sim_bodies`, removed): 1-2
steps 26 actor bodies a tick at 13.2 µs each (342 µs); 1-3 steps 20 at 8.6
µs (173 µs). The walls of each body (`solids_for`) are ~1 µs; the player's
step is 8 µs. 1-2's actors are 20 walkers (15 AI Slop, 5 snakes); 1-3's are
mostly plants and flyers. So 1-2 is dear because it has many walking bodies,
and each one runs a full kernel step on each tick.

A RECORDED NEGATIVE: six block queries of the kernel (one-way support, the
brink probe, the two hazard tests, rebound, the supporting block) scan each
block of the room. Moved onto the room's index (`Layered::near`) with a
witness that each answer equals the scan's, `integrate_sim_bodies` in 1-2
went 420 → 412 µs a call: noise. The change was reverted. The terrain is
already merged into rectangles (`merge_intgrid_rects`), so a scan is short.

The lever (not built): Mary-O's `hostile_wake_radius` (720 px) sleeps a far
hostile's BRAIN, and its body still steps (`dormancy.rs`: "body physics
continues"). In a level 7,680 px wide most of the 26 bodies are dormant and
at rest. A body at rest whose step is a fixed point need not step. It must
be exact: `ActorMut::update` also advances health, timers, the hit flash and
what a body carries, so the rule needs its own design and a witness that a
skipped tick and a stepped tick give the same components, under rollback.
REASONED, the exact form: the step is a pure function of its inputs, so
keep for each body the fingerprint of the last step that changed nothing
(the body's clusters, combat, motion model and control frame, the blocks
near it, its contact field, its frame), and skip a step whose inputs have
that fingerprint. No dormancy rule is needed for that; dormancy is what
makes the case common. The risk is an input the fingerprint leaves out,
so its witness must step and skip the same bodies side by side over many
ticks and compare every component.

Done: `dress_power_blocks` does its work only when a block, its art or the
spent set changed (witness `the_dresser_follows_each_of_its_inputs_and_nothing_else`,
red at its own assertion with each arm removed). MEASURED (the same capture,
after): in 1-2, 142.7 → 7.1 µs a frame and 354 → 3 allocations a frame; in
1-1, 22.9 → 9.3 µs (the "before" on a loaded box).

Done: a rigged body drawn directly that no view shows is hidden and not
posed (`drive_rigged_presentations`, `seen_by_a_view`: the active
orthographic cameras on the body's layers, with a 256-unit margin for the
part reach and the camera's last-frame place). A body read as one image, or
whose parts a reader asked for, is posed wherever it is; with no camera every
body is posed. This is the "culling/bounds" rule that
`semantic-part-rendering-and-ragdolls.md` asked for, for the direct road.
Witness `a_body_no_view_shows_is_not_posed_until_a_view_shows_it` (red at its
own assertion with no cull, with the body hidden and still posed, with the
layers ignored, with a cull when no view exists, and with a view that never
shows it). MEASURED (same capture, idle box, frames 24-26 ms in both):
`drive_rigged_presentations` in 1-2 172.7 → 97.7 µs a frame, in 1-1 129.8
→ 104.3 µs.

### F39 — Ambition's hall, and what Tracy does to a profile (2026-10-18)

Asked by Jon ("my game is running slow, even on my 3090"), on the autoplay
route (hub, hall of characters, the primeval trail).

MEASURED (VM, same commit, same route, `scripts/profile_desktop.sh
--autoplay`): the Tracy build made the tick ~5× dearer. With `--features
profile` the simulation sets cost 21 ms a tick; with `--no-tracy` 3.85 ms
(the GGRS driver 5.68 ms). A span for each of ~560 systems a tick is the
cost, and with no client connected Tracy also buffers. ⇒ Rank systems with
Tracy; take milliseconds from a `--no-tracy` run. Jon's host profile (Tracy
on): the driver 12-21 ms a frame, the simulation sets ~7 ms a tick.

MEASURED (headless hall, `scripts/headless_system_shares.sh`, cycles): the
per-system cost of running systems at all (executor, parameter fetch,
`update_archetypes`, deferred application) is ~35% of the main thread at
~800 systems a tick; then `integrate_sim_bodies` 8%, `tick_actor_brains`
6%. Local play is a GGRS sync test with `check_distance 0`, which saves
nothing (ggrs skips `SaveWorld` there), so this is not snapshot cost.

Done (one change each, no behaviour change; the hall, `headless --bench`,
`scripts/sim_tick_instructions.sh`, interleaved A/B of only these changes):
**10.0 → 8.5-8.8 M instructions a tick (−13%).**

- The prepared cast is hashed by id, with an ordered id list for walks:
  each body looked its character up in a `BTreeMap` of ~215 ids each tick
  (`record_bodies_away_from_home`, the render and pose indexes, wallet
  shields, rest rows, presentation sources).
- `record_bodies_away_from_home` asks "at home?" before the durable-row scan.
- Directional and dash verbs are composed on the stack, not as `String`s
  (`move_for_attack` ×15 a body a tick in the attack kit).
- `melee_swing_of` asks for a playing move before walking the verbs.
- Early outs: `carry_homing_dashes` with nobody dashing; the ranged-route
  lookup only for a ranged request; a realized sheet before the load ledger.
- Compare before allocating: presentation source ids, attack VFX sheets.
- Targeting borrows candidate ids and asks the relation only of a nearer body.
- The view indexes use the fixed-seed hasher; the OOB recorder's kinds are
  `&'static str` and its rollback slot scan stops at older rows.

Open, by size (agents' reads, REASONED until built): `MoveSpec::frame_data`
is derived for each body's kit each tick (hydrating params), ~1% — derive
once per moveset; `CharacterCatalog` lookups are a `BTreeMap` too
(`attack_vfx`, `rest_row`), ~1%; `Layered::near` allocates two `Vec`s a
query; per-room dormancy for Ambition's own rooms (its rooms declare every
capability, so 45 boss-encounter and 35 fighter-special systems run in the
hall) is the structural lever, and a design question for Jon.

## Summary (2026-10-16)

- **One thread is the limit.** On the phone the main thread is 82% busy, the
  render thread 25%, each of 4 pool workers 8-9% (F11). 62% of the main
  thread is framework work outside any system body: per-system scheduling,
  access checks, archetype updates, allocation (F0, F11). The cost follows how
  many systems run, not what they do; no system body of ours is over 1.4%.
  Levers: fewer systems per game (P1), a serial executor for the
  standalone games (P2: MEASURED on the desktop, a standalone tick is ~6×
  cheaper and the main thread does 23% less work per second while drawing
  32% more frames, F22). In the hub (what the phone runs) the simulation is
  already serial and only ~15% of the main thread; the frame phases are the
  rest (F23).
- **Input was 82% of a standalone frame's allocations** (19.2k a frame,
  F13): leafwing rebuilt every action pair on each call though we resolve no
  clashes. Fixed in this repo: allocations a frame Mary-O 23,407 → 4,128,
  hub 46,949 → 15,257 (pixel-identical capture). On the phone the pass was
  4.5% of all CPU samples (~8% of the main thread).
- **Change detection is defeated.** About 30 components of every body are
  marked changed every tick whether they change or not (F20), and several
  resources too; one no-op `&mut` re-folded Mary-O's whole repertoire every
  tick (F15). The common style re-derives every tick (F16). The new change
  counter names each such write by source line. Levers: P12, then P5.
- **Presentation entities in immediate mode** (F21): despawn all and spawn
  all, every frame. Three loops found, all fixed.
- **Composition:** standalone Mary-O and Sanic run ~170 systems a frame for
  capabilities they do not use (P1; D3, D4 for Jon). Both hosts now build the
  engine without named boss encounters (−13 and −11 systems a frame); most
  of the rest is in the core extension phases (C1).
- **Fixed tonight, no behaviour change, each verified by the probe, a
  pixel-identical hub capture, or a test:** Mary-O's star re-fold (F15),
  Sanic's gravity visuals (F18), the hub's per-frame parallax respawn and its
  cause in `first_room_art.rs` (F17), the hub's wielded-item overlays (F21),
  room isolation walking only when a mask can be stale (F14: 443 → 7.6
  allocations a frame in Mary-O, 1,118 → 2.0 in Sanic), read-before-write
  for the SFX authorizer, the ledgers and the trail gizmo bands (F20), HUD,
  feature-visual and hit-flash writes only of a change (F20), seat bindings
  projected only on a change, the two whole-world queries filtered (F7), and
  leafwing's clash pass skipped under `PressAll` (F13).
- **Before → after tonight** (MEASURED, `perf_probe`, idle, per frame,
  without the probe's and the census's own work; `probe/*_v2` against
  `probe/*_final`):

  | | Mary-O | hub `basement_enemies` |
  | --- | ---: | ---: |
  | allocations in our systems | 2,218 → 1,156 (−48%) | 2,328 → 1,743 (−25%) |
  | all allocations | 24,772 → 23,407 | 47,478 → 46,949 |
  | of which leafwing input (fixed after this table, F13) | 19,222 | 31,942 |
  | archetype churn from our code | 10.3 → 7.2 | 22.2 → 7.2 (−68%) |
  | value changes from our code | 1,575 → 1,379 | 994 → 956 |

  After the table, the leafwing fix (F13): all allocations a frame Mary-O
  23,407 → 4,128, hub 46,949 → 15,257. Sanic act 1 over the whole night
  (`probe/sanic_act1_v2` → `probe/sanic_act1_final`): all allocations 24,034
  → 3,492 (−85%), in our systems 1,942 → 692, value changes from our code
  682 → 503. The large remaining items: the serial executor for the
  standalone games (D12, needs a yes), then P1 and P12.
- **Time, desktop** (MEASURED, `scripts/executor_ab.sh`, 3 interleaved
  repetitions, the same script early in the night and at the end; the box's
  load differed, so read the direction, not the digit). Main-schedule time
  per drawn frame: Mary-O today's policy 14.1 → 12.6 ms (−11%), with
  `serial` (D12) 8.3 → 7.0 ms; hub today's policy 13.4 → 12.4 ms (−7%), with
  `serial` 12.5 → 10.9 ms. Standalone Sanic at the end only (3 repetitions):
  simulation 7.21 ms per tick today, 2.98 ms with `serial`; main-schedule
  time per drawn frame 9.5 → 7.6 ms.
- **What tonight's fixes held on the phone** (MEASURED, the all-games phone
  profile of 10-16, `symbolize_android_profile.py --under NAME`, inclusive
  share of all process CPU samples): `handle_clashes` 3.14%,
  `reconcile_effective_repertoire` (the star's false change) 1.43%,
  `isolate_live_rooms` 0.47%, the SFX authorizer 0.40%,
  `publish_seat_bindings` 0.22%, the HUD 0.13%, the rest under 0.1% each:
  about 6.2% of all samples, ~11% of the main thread. The first two are gone;
  the others now run only on a change. REASONED until a new phone run
  measures it.
- **Instrumentation:** `perf_probe` (allocations per span, churn and value
  changes per source line; `profile_desktop.sh --perf-probe`); the phone
  symbolizer's thread table, `--thread`, `--ours`, `--inclusive` and
  `--under`; the executor A/B switch (`AMBITION_SCHEDULE_EXECUTOR`,
  `scripts/executor_ab.sh`, to remove with D12). Each is one switch, one
  directory or one script to remove.
- **For Jon:** D1-D12 below (D13 withdrawn: fixed without a fork). None is
  done. D12 (serial executor default for the standalone games) changes no
  behaviour and is ready.

## Next, in order

Updated 2026-10-17. Each one helps every game, not only Mary-O.

1. P14: one composed collision world for each room each tick (F24, F33).
   Step 1 done 2026-10-17 (six compositions a frame in 1-2 → one); step 2,
   the layered view, done by the portal work the same day (F24).
2. P12 and P5: derive on change, and build the read-models once for each
   presented frame, not each tick (F16, F20). MEASURED 2026-10-17 (F5): the
   views are ~3% of a tick, so P5 is smaller than this list first said.
3. P1/S2 and P18: the rest of dormancy (F31) and the conditions of F34.
4. P15: render primitives apart from simulation objects (F32 is the first).
5. P16: a repeat-allocation pass from `alloc_traces.csv`.
6. P4: interned identities. P17: baked navigation graphs.
7. Fork edits, for Jon: Yarn `fetch_resources` (keep its `SystemState`;
   it now sleeps outside dialogue rooms, F36),
   `virtual_joystick::update_ui` (compare before write); `bevy_light` open.
8. D12: done 2026-10-17 (F22).

## How to answer a cost question

| Question | Tool |
| --- | --- |
| Which systems run in a frame, by crate? | `AMBITION_PROFILE_CENSUS=1` + `AMBITION_SYSTEM_LIST=PATH`; `scripts/system_census_table.py` |
| Which Bevy system takes the time (desktop)? | `scripts/profile_desktop.sh` → `tracy_summary.md` |
| Where does the phone spend its CPU? | `scripts/profile_android.sh all ...`, then `scripts/symbolize_android_profile.py DIR` |
| Which line of OURS caused framework time (phone)? | `symbolize_android_profile.py DIR --ours` (needs `simpleperf-device-samples.txt`, which a profile made after 2026-10-16 has) |
| Which functions of ours cost the most, framework time under them included (phone)? | `symbolize_android_profile.py DIR --inclusive [--thread TID]` |
| How much phone CPU passes through one function (inclusive)? | `symbolize_android_profile.py DIR --under NAME` (repeatable; with `--thread TID`) |
| Which thread is saturated, and what does it do (phone)? | the `threads` table of `symbolize_android_profile.py DIR` (busy % of one core, by thread id), then `--ours --thread TID` |
| Which system allocates, and which code makes archetype churn? | `profile_desktop.sh --perf-probe` → `alloc_by_span.csv`, `churn_by_site.csv` (recipe `profiling.md` §1d) |
| Which code marks a component or resource changed every frame (wakes every `Changed<T>` reader)? | the same run → `changes_by_site.csv` |

## Measured baseline

**Phone (Pixel 5, hub with every game, Mary-O 1-1, commit 97ff2541e,
2026-10-16; `profiles/android-all-maryo-1-1`).** MEASURED.

- Frame time, 5 s windows: p50 24-46 ms, p95 37-131 ms.
- Threads: main 67%, the Bevy compute pool 22% (shares of the process; see F11
  for how busy each one is).
- Crates by own time: `core` 21%, `bevy_ecs` 13.2%, `mimalloc` 5.2%, `alloc`
  3.1%, `concurrent_queue` 2.2%, `fixedbitset` 1.2%, `async_executor` 1.0%;
  each crate of ours under 1%.
- Reading (REASONED): the time is in the framework and the allocator, and the
  framework does what our composition asks: about 1000 systems a frame, their
  access checks (`fixedbitset`), the task executor, and allocations. Which
  line of ours causes each part is the next measurement (`--ours` with the
  sample dump, and the `perf_probe`).

**System census (desktop debug, VM, 2026-10-16).** MEASURED.

| run | registered | ran in one frame |
| --- | --- | --- |
| standalone Mary-O, 1-1 | 1277 | 1036 |
| standalone Sanic, act 1 | 1264 | 1043 |
| hub Ambition, `basement_enemies` | 1964 | 1529 |

Systems that ran in each standalone game for capabilities it does not use
(Mary-O / Sanic): boss encounters 41/41, conversation 36/36, Smash grabs and
throws 14/14, Smash moveset 13/13, mounts 7/7, held items 9/9, match stocks
7/7, gizmos 36/36, Kira audio channels 30/30, developer tools and traces 18/18,
extension lowering 32/34. About 170 systems a frame (17%).

## Findings

### F0 — A third of the phone's CPU is the executor's work for each system, outside any system

MEASURED (phone, `android-all-maryo-1-1`, every sample's full call chain,
`symbolize_android_profile.py --ours`). 32.2% of all app samples are in the
schedule executor with no system body on the stack: per system, before or
after it runs. By innermost frame: access checks over component bitsets
(`bevy_ecs/query/access.rs` with `fixedbitset`, about 2.9%), the <!-- cite-ok: a source file of a dependency (bevy_ecs, bevy_camera, bevy_ggrs, ggrs), not of this repo -->
`TypeId` test that finds apply-deferred points (`system/system.rs` and <!-- cite-ok: a source file of a dependency (bevy_ecs, bevy_camera, bevy_ggrs, ggrs), not of this repo -->
`executor/mod.rs`, about 2.5%), archetype-cache updates <!-- cite-ok: a source file of a dependency (bevy_ecs, bevy_camera, bevy_ggrs, ggrs), not of this repo -->
(`update_archetypes_unsafe_world_cell`, `query/state.rs`), parameter <!-- cite-ok: a source file of a dependency (bevy_ecs, bevy_camera, bevy_ggrs, ggrs), not of this repo -->
validation (`function_system.rs`), and allocation in the executor. A further
44.8% has no frame of ours at all: the render world, the GPU driver
(`vulkan.adreno.so`), the kernel, and the task pool.

What it means (REASONED): this cost grows with the number of systems that run
each frame (about 1000) and, for the access checks, with the number of
component and resource types the world has registered (about 1900 and 1400).
It does not depend on what the systems do. So the levers are: fewer systems
installed (P1, P3), fewer and coarser systems where many small ones do one
job (the ten narrative ledgers), a set run condition that skips a whole
feature group when it has no work (a skipped system pays none of the per-system
work above), and fewer registered types (291 message types, 330 GGRS snapshot
resources).

Lines of ours that do take time (each under 1.5%): `room_instance.rs:172`
(1.4%), `room_graph.rs:432` and `:471`, `clip_material.rs:145`,
`ambition_entity_catalog/src/lib.rs:1563`, `crates/ambition_touch_input/src/virtual_device.rs:50`,
`wallet_shield.rs:45`, `held_items.rs:108`, `hand.rs:63`.

**Mary-O only, against every game (phone, one run each).** MEASURED: with
`AMBITION_HUB_GAMES=mary_o` the app used 41.7 s of CPU in 30 s, against
49.9 s with every game (-17%), and 38% fewer context switches. Frame time was
not better in this pair (p50 41-69 ms against 24-46 ms), but the pair is
confounded: other commits (97ff2541e, 11130fb49), one run each, and the place
in the level. Clocks were at their maximum and thermal status 0 in both.

### F1 — The hosts install everything; the capability profiles can omit five things

MEASURED. Both demo hosts build `PlatformerEnginePlugins::fixed_tick()`
(`game/ambition_demo_mary_o_app/src/lib.rs:45,143`,
`game/ambition_demo_sanic_app/src/lib.rs:28,121`); no host calls `for_profile`.
`Capability` has five variants (`crates/ambition_platformer2d_runtime/src/profile.rs:32`). Grabs and throws, the Smash
moveset, mounts and match stocks are installed inside `CombatSchedulePlugin`
(`ambition_platformer2d_runtime/src/combat_schedule.rs`, mounts at `:776`),
where a plugin group cannot disable them. Boss encounters CAN be omitted
already: `BossCatalog` is core state (`runtime/src/sim_core_resources.rs:194-203`)
and the `encounters-without-named-bosses` profile is witnessed; the demo hosts
simply do not ask. `BossEncounterSimulationPlugin` initializes the catalog a
second time (`boss_encounter/src/lib.rs:132`), and the run condition at
`actor_monolith/src/features/mod.rs:1094-1102` tests a resource that is now
always there. The extension phases `technique_execution` and `boss_conduct`
(about 29 systems) and `wielded_use`/`module_entity_tick` (about 20) are core,
whatever the profile. Since 2026-10-16 the Mary-O and Sanic hosts build the
engine without `Capability::BossEncounters` (their integration tests pass:
95 and 42). MEASURED: systems run a frame Mary-O 1,079 → 1,066, Sanic 1,086
→ 1,075; the rest of the boss systems are in the core extension phases (C1).
Mary-O's and Sanic's own movesets author
`smash.capture_attempt/pummel/throw`, so Grabs cannot leave a standalone game
until its fighter kit does (D4).

### F2 — Twelve narrative ledgers run in every game

MEASURED. `NarrativeInputPlugin<M>` (`ambition_conversation/src/ledger.rs:247`)
adds three systems per payload; twelve payloads (two in `ambition_conversation`,
seven in `actor_monolith/src/features/mod.rs:1491-1497`, three in
`ambition_content`) make 36 systems; a standalone game runs nine (27). The seven
monolith ones and the ten outside `ConversationPlugin` are
not removed by `Capability::Dialogue`. Every writer is in `ambition_conversation`
or `ambition_content`.

### F3 — The fixed schedules run on the multi-threaded executor

MEASURED. `serialize_frame_schedules`
(`ambition_platformer2d_runtime/src/lib.rs:741`) covers `First`..`Last`, not
`Fixed*`. Standalone games simulate in `FixedUpdate`. `dev/bevy_idle_systems`:
an idle system costs about 0.016 us on the single-threaded executor and 1.7-12 us
on the multi-threaded one. The executor policy has a second owner
(`game/ambition_app/src/app/plugins.rs:356`, an env switch the demos never reach).
Worse: the windowed demo hosts did not call the policy at all; only the
headless foundation and `game/ambition_app/src/app/visible_composition.rs`
did. So windowed Mary-O, Sanic and Smash run `First`..`Last` and `Fixed*` on
the multi-threaded executor. Since 2026-10-16 all three foundations call
`apply_executor_policy`, with unchanged defaults (F22, D12). Bevy
itself runs `Main`, `FixedMain` and `RunFixedMainLoop` serially; the rollback
crate serializes `GgrsSchedule` only, not `SaveWorld`/`LoadWorld`/`ReadInputs`/
`AdvanceWorld`.

### F4 — In the hub, each game runs the other games' gameplay

MEASURED (10-14 census). While Ambition plays, Smash runs 49 systems, Mary-O 29,
Sanic 4. Scope gates: `ambition_demo_smash` 1 of 51 `add_systems`,
`ambition_demo_sanic` 2 of 33, `ambition_demo_mary_o` 11 of 52,
`ambition_content` 0 of 150. Physics debris (avian2d 20, interpolation 19) and
3D lighting (`bevy_light` 19, `bevy_pbr` 14) run in the hub; no code spawns a
light. Ambition's cast uses techniques that `ambition_demo_smash` registers
(`smash.counter`, `smash.place_spring`), so the hub cannot leave Smash out.

### F5 — Read-models rebuild each tick, inside the simulation, for the renderer

MEASURED. `SimViewPlugin` (`ambition_sim_view/src/facts.rs:1141-1180`) rebuilds
its views in the sim schedule's `FeatureViewSync` (about 36 systems); only
`ambition_render` (and the demo HUDs) read them. A headless host builds them for
no reader; on a slow frame with several ticks they are built several times.

MEASURED 2026-10-17 (debug, headless standalone Mary-O 1-1, one tick an
update, `run_system_cached` 200 times for each system): the set holds 30
systems; the 28 rebuilds of `SimViewPlugin` and `FeatureViewSyncSchedulePlugin`
cost 0.30 ms a run together (`rebuild_body_pose_views` 0.045,
`rebuild_feature_view_index` 0.045, `rebuild_actor_anim_index` 0.034,
`rebuild_actor_render_index` 0.028, `rebuild_nameplate_index` 0.025, the rest
under 0.02 each), about 3% of the 11 ms tick. Gating the whole set off cut
the frame by 0.9-1.3 ms, but that arm also starves the readers that wait for
a view to change, so it overstates the views. REASONED: P5 saves at most one
build for each extra tick in a frame (~0.3 ms and 30 system runs on a two-tick
frame), and it moves ~50 registrations across schedules, with two tick-rate
writers in the set (`advance_body_anim_overlay_clocks`,
`tick_shrine_activation_pulse`). It is a small lever for its size; keep it
behind the others.

### F6 — String identities are cloned, compared and sorted every tick

MEASURED (source). `select_actor_targets`, `collect_perception_peers`,
`rebuild_actor_anim_index`, `rebuild_body_pose_views`,
`rebuild_actor_render_index`, `rebuild_feature_view_index` and
`rebuild_nameplate_index` clone `SimId`/`FeatureId` strings, sort by string, or
key `HashMap`s by `String`, per body per tick.

### F7 — Two systems visit every entity every tick

MEASURED (source). `project_wallet_shields`
(`actor_monolith/src/features/wallet_shield.rs:32`) and
`brandish_the_playing_move_s_weapon` (`ambition_combat/src/held_items.rs:91`)
iterate a query with no required component, so they visit every entity in the
world each tick (the hub census counts about 8000 entities, 1300 live). A body
they can act on has `WornCharacter` or `BodyWalletShield`, and `MovePlayback` or
`MoveBrandishedItem`; the filters `Or<(With<WornCharacter>,
With<BodyWalletShield>)>` and `Or<(With<MovePlayback>, With<MoveBrandishedItem>)>`
keep what they do and drop the walk (both applied 2026-10-16; the
wallet-shield, held-item, Mary-O and Sanic tests pass). Eight other queries in the scan have no
required component but are only used with `get(entity)`, which costs nothing.
`project_prepared_character_definitions` (`character_runtime/presentation.rs`)
walks every body only on the tick the registry changes.

### F8 — Plumbing that scales with what is registered, not with what is used

MEASURED. Gizmos: 8 groups x 4 systems (16 for `RoomTrailGizmos0..3`); Bevy's
debug gizmo groups are on. Kira: 15 channel types x 2 systems. Message types:
291 `Messages<T>` resources, each with its own update. Developer tools: about
18 systems in shipping hosts (`DevToolsSimPlugin`, including a file-watch poll).
Input: `publish_seat_bindings` builds a `BTreeMap` and formats strings every
frame to find a rebind.

### F9 — Upstream query-state costs

MEASURED (10-13 phone profile). `virtual_joystick` is fixed in our fork.
Open: `bevy_ecs_ldtk` `clean_respawn_entities` (~0.8%), `bevy_yarnspinner`
`fetch_resources` (~0.6%), `bevy_light` `check_dir_light_mesh_visibility`
(~0.5%), `bevy_ggrs` `handle_requests` (0.2-0.4%).

### F10 — Content in engine crates (composition hygiene, not CPU)

MEASURED. `HELD_ITEMS` (`ambition_characters/src/brain/action_set/mod.rs`, 25
Ambition items), the closed `ambition_items::Item` enum and `icon_path`,
`fire_puppy_slug_gun_system` and its rollback key in every game, portal-gun
`gun_*` modules in the generic `ambition_portal2d`, `BUILTIN_STRIKE_KEYS` with a
second list of the same 11 keys, the invented `gradient_sentinel` boss.

### F11 — The main thread is the limit; the pool and the render thread are not

MEASURED (both phone runs, the `threads` table and `--thread TID`, 30 s each).
Busy, as a share of one core over the recording (Mary-O-only / all games):

| thread | busy |
| --- | --- |
| main (`android_main`) | 82% / 83% |
| Bevy render thread (also named `android_main`: an unnamed `std::thread` has its parent's name) | 25% / 26% |
| compute task pool, 4 workers | 8-9% each |

On the main thread (56.5% of the process samples in the Mary-O-only run):

- 34.9 points (62% of the thread) are under `app.run()`
  (`ambition_app/src/app/cli.rs:273`) with no system of ours below them:
  access bitset tests, `update_archetypes_unsafe_world_cell`,
  `is_apply_deferred`, `run_without_applying_deferred`, mimalloc, and Bevy's
  own systems.
- The rest is in system bodies of ours, spread thin: the largest is 1.4 points
  (`lifecycle/room_instance.rs:172`), all others under 0.6.

The render thread (17.1%) has no frame of ours: kernel, the Adreno Vulkan
driver, wgpu.

Reading (REASONED): the frame waits on the main thread. It is on-CPU 82% of
the time; the off-CPU rest is most likely the multi-threaded executor waiting
for its tasks, because the render thread is busy only a quarter of the time.
The pool does little: the parallel executor buys almost no parallel work and
costs the scheduling. This is the strongest support for P2 (serial schedules)
and P1 (fewer systems). The GPU is not the limit of these runs: the render
thread, which submits and presents, is idle three quarters of the time.

### F12 — No catch-up limit of ours on the fixed step

MEASURED (code read). The hub's GGRS loop (`bevy_ggrs` `schedule_systems.rs:55`) <!-- cite-ok: a source file of a dependency (bevy_ecs, bevy_camera, bevy_ggrs, ggrs), not of this repo -->
runs `while accumulator >= 1/60 s` on `Time::delta()`; the standalone games use
Bevy's `FixedUpdate`. Neither has a limit of ours; only Bevy's `Time<Virtual>`
`max_delta` (250 ms) applies, so one slow frame can run up to 15 ticks. The
offline GGRS session uses `check_distance: 0`, which skips every save
(`ggrs` `sync_test_session.rs:155`). <!-- cite-ok: a source file of a dependency (bevy_ecs, bevy_camera, bevy_ggrs, ggrs), not of this repo -->

Reading (REASONED): with p50 frames of 24-46 ms on the phone, a frame runs
1.5-2.8 ticks on average, and the read-models of `FeatureViewSync` are rebuilt
on each tick (P5). A lower catch-up limit trades real time for frame time (the
game slows down under load): D11.

### F13 — Three quarters of a standalone frame's allocations are input lookups

MEASURED (`perf_probe`, desktop debug, 578 frames, allocations per tracing
span; scratchpad `probe/`). Allocations per frame, standalone Mary-O 1-1 /
Sanic act 1:

| span | Mary-O | Sanic |
| --- | --- | --- |
| `leafwing_input_manager::systems::update_action_state` | 19,222 | 19,222 |
| multi-threaded executor | 2,293 | 2,116 |
| `view_isolation::isolate_live_rooms` | 444 | 2,773 |
| `bevy_render::render_asset::prepare_assets` | — | 1,064 |
| `hand::reconcile_effective_repertoire` | 596 | — |
| `starting_character::gate_body_control` | 295 | 123 |
| `authorize_staged_character_presentation_sources` | 203 | 210 |
| `dev::trace::actor_oob::record_actor_oob_frame_system` (a developer trace) | 196 | — |

The input number is the same in both games: it does not depend on the scene.
The hub makes 31,942 a frame (26.6k a call). Two causes in
`leafwing-input-manager` 0.21, both run twice a frame (frame and fixed loop):

- MEASURED (source read), the larger by the count per call: `InputMap::handle_clashes`
  calls `get_clashes`, which calls `possible_clashes()` on every call
  (`src/clashing_inputs.rs:190-232`): every pair of button actions, each
  binding decomposed into new `Vec`s. The comment there says "cached"; it is
  not. Our host chooses `ClashStrategy::PressAll` for chord-free maps
  (`ambition_platformer2d_host/src/lib.rs:379`), and under `PressAll` a clash
  removes nothing, so all of this work has no effect for us.
- MEASURED (source read): `CentralInputStore::pressed` and `button_value`
  (`src/user_input/updating.rs:111-140`) box a clone of the binding for each
  lookup, to use it as a hash key (`// PERF: surely there's a way to avoid
  cloning here`).

Split (MEASURED, `crates/ambition_input/tests/leafwing_allocations.rs`, the
game's own seat map): one `process_actions` call makes 9,611 allocations (the
probe's per-call figure exactly), and 9,522 of them (99.1%) are
`handle_clashes`. The boxed lookups are the other ~90.

Fixed in this repo, with no fork (2026-10-16):
`ambition_input::press_all_action_update` removes leafwing's two
`update_action_state` systems (`App::remove_systems_in_set`) and adds one that
skips the clash pass under `PressAll` and calls `process_actions` under any
other strategy; the first half of `process_actions` is copied, and a test
compares the two on held keys. Verified (MEASURED, `probe/mary_o_1_1_pressall`):
all allocations of a standalone Mary-O frame 23,407 → 4,128; the input update
19,222 → 178. The hub (`probe/basement_enemies_pressall`, what the phone
runs): 46,949 → 15,257, the input update 31,942 → 258, the capture
pixel-identical. Tests: `ambition_input` 152, Mary-O 95, Sanic 42. Left for
an upstream PR: skip the pass in `process_actions` itself, and look up
without boxing (`hashbrown::Equivalent` on `&dyn Buttonlike`).

On the phone (MEASURED, the existing profiles, `symbolize_android_profile.py
--under handle_clashes`): 4.5% of all process samples in the Mary-O-only run
(3.1% with every game) were inside `handle_clashes`, about 8% of the
saturated main thread. REASONED: that is the CPU this fix removes on the
phone; a new phone run will measure it. The probe counts allocations; their
time on the phone is inside the 5.2% of `mimalloc` and the 3.1% of `alloc`.

Idle re-runs (MEASURED, `probe/mary_o_1_1_v2`, `probe/basement_enemies_v2`):
standalone Mary-O makes 26.0k allocations a frame in all spans, 19.2k of them
in `update_action_state` (74%); the hub makes 61.4k, 31.9k in it (67% without
the probe's own 13.4k). `gate_body_control` makes 189 a call,
`authorize_staged_character_presentation_sources` 202.

Caveat on the per-frame numbers (MEASURED): the first Mary-O probe run was taken
while the VM was loaded, at about 14 fixed ticks a frame against about 1.6 at
idle, so the per-frame numbers of fixed-step systems are high; the per-call
numbers (`allocs_per_call` in the CSV) are the ones to compare. Other per-call
numbers of ours: `reconcile_effective_repertoire` ~40, `gate_body_control`
~20, `record_actor_oob_frame_system` 13, `integrate_sim_bodies` 9.6. Sanic:
`prepare_assets<HitFlashMaterial>` ~1,070 a frame (133 KB), so a hit-flash
material is changed every frame. The write is already guarded
(`hit_flash.rs:416-437`, write only a change); the material changes because
each overlay follows its body's animation frame (`uv_rect`) also at intensity
0, and overlays live as long as their bodies and stay `Visible`. REASONED: an
overlay at intensity 0 draws nothing; tracking the frame only while a flash
is on would remove the re-uploads and the draws of idle overlays. The comment
there explains why the overlay asserts `Visible` (portal compositing), so this
needs a check of the portal path first.

Open (MEASURED, `probe/basement_enemies_pressall`): after the fix, ~9.4k of
the hub's 15.3k allocations a frame are on schedule spans, not on a system:
`GgrsSchedule` 3,877, `Update` 3,067, `PostUpdate` 1,558, `PreUpdate` 857. In
standalone Mary-O (parallel executor) the same work is on system spans.
Two explanations are ruled out (MEASURED, source read): command
application has its own `(commands)` rows (Bevy enters `system_commands` in
`CommandQueue::apply`), and only Bevy's `DefaultPlugins` are built before
`capture_scene` installs the subscriber, so no system of ours has a dead span.
REASONED, left: observers and component hooks, which run with no system span
(the hub spawns and inserts more than Mary-O), and the executor. Next probe
step: a span in the probe's own observer path, or a Tracy look at
`GgrsSchedule`.

### F14 — Room isolation re-derives every room mask every frame

MEASURED (source read; allocations from F13). `isolate_live_rooms`
(`ambition_render/src/rendering/view_isolation.rs:216`) walks the subtree of
every entity stamped `InRoomInstance` each frame, with a new `Vec` per root,
and rebuilds each `RenderLayers` with several clones (`in_room_band`,
`without_room_layers`). The result changes only when the live-room set or its
order changes, or when an entity or child is new. The room bands are layers
256-511 and the local-view band starts at 1024; Bevy's `RenderLayers` keeps one
`u64` inline (`bevy_camera` `render_layers.rs:23`), so each mask with a room <!-- cite-ok: a source file of a dependency (bevy_ecs, bevy_camera, bevy_ggrs, ggrs), not of this repo -->
layer is on the heap, and each clone of it allocates. Sanic: 2,773
allocations a frame; Mary-O: 444.

Fixed (2026-10-16): the walk runs only when a mask can be stale (the
live-room set or its order changed; a stamp, a child or a `PresentedForView`
is new or changed; a mask was removed; or, while rooms are banded, another
system wrote a mask). Verified (MEASURED): allocations a frame Mary-O 443 →
7.6, Sanic 1,118 → 2.0, hub 274 → 19; the hub capture is pixel-identical; the
seven view-isolation tests pass. A second, smaller lever is left: put the room
bands in layers below 64, so a mask fits inline.

### F15 — A `&mut` that writes nothing wakes every change filter downstream

MEASURED (probe: `reconcile_effective_repertoire` ran 901 times in 901 ticks
of Mary-O, 596 allocations a frame; it is not in Sanic's list; source read).
Mary-O's `begin_star_power` (`game/ambition_demo_mary_o/src/star.rs:62`) calls
`worn.unequip(POCKET_QUASAR_ID)` on `Mut<WornEquipment>` every tick. The
`&mut` call marks `WornEquipment` changed whether or not a row is removed, so
the change-filtered `reconcile_effective_repertoire` re-folds the whole
repertoire every tick and writes `ActionSet` and `ActorMoveset`, which marks
those changed for each of their readers. Fix (no behaviour change): test
`wears` first, through `Deref`.

Reading (REASONED): this is a class, not one bug. Bevy marks a component
changed on `DerefMut`, not on a different value. The general answer is a
detector, not a code search: the probe reports, per component type, the
entities marked changed per frame; a component changed on every frame for
every entity is either a per-frame value (expected) or this bug.

### F16 — The common style: re-derive every tick, idempotently

MEASURED (probe allocations, source read). Four of the top allocators of ours
re-derive, every tick, a value whose inputs change rarely, and say so as a
virtue ("idempotent by construction", "running every tick can only add"):

| system | re-derives each tick | inputs change when |
| --- | --- | --- |
| `isolate_live_rooms` (F14) | every room entity's `RenderLayers` | the live-room set changes, an entity is new |
| `reconcile_effective_repertoire` (F15) | the repertoire, after a false change | equipment, identity, held item change |
| `gate_body_control` (`avatar/starting_character.rs:631`) | each body's action scheme (`derive_action_scheme`); the control prompt derives it again | abilities, moveset, action set, techniques change |
| `authorize_staged_character_presentation_sources` (`character_runtime/presentation.rs:54`) | each staged provider's SFX authorization, with catalog clones; its `ResMut` write marks the selection changed every tick | the cast, a bank or a catalog arrives |

Reading (REASONED): the style is robust (no missed update, no order bug,
rollback replays it) and its cost is per tick per body. The elegant pattern is
derive on change: a run condition (`resource_changed`, `any_with_component`),
a `Changed<T>` filter, or one cached component (`DerivedActionScheme`) that
both the gate and the prompt read. Under rollback a cached value must be
rollback state or derived from inputs that a restore marks changed; check
each cache against that before it lands. Together with F15 this is P12.

**`gate_body_control` done (2026-10-17).** `DerivedActionScheme` (required by
`ActionSet`) keeps the scheme and what change detection cannot see: the
driven techniques and which optional authorities were present. The gate
derives again only when an authority is changed, added or removed. It is
derived rollback state: a restore writes back a differing authority, which
marks it changed. MEASURED (probe, hub `basement_enemies`, ~600 frames):
54.8 → 16.4 µs a frame, 209 → 29 allocations a frame. The control prompt
already derives on change, from the abilities masked while the pose is held,
so it does not share the gate's cache. Witness
`the_gate_derives_the_scheme_again_only_when_an_authority_changed`.

### F17 — The hub respawns a room's parallax every frame

MEASURED (probe churn, hub `basement_enemies`, 595 frames): exactly 1.000
`PresentedRoomParallax` root spawn, 2.000 parallax layer spawns, 2.000 layer
despawns and 2.000 `PresentedForView` inserts (`parallax.rs:623`) per frame;
also 1.000 `WieldedItemVisual` spawn per frame. A steady rate of exactly one
per frame is a loop, not a load.

Cause (MEASURED with the change counter, `probe/basement_enemies_v2`):
`GameAssets` was marked changed on 593 of 593 frames, by a caller inside
`Option::as_deref_mut`. `refresh_parallax_layers_on_quality_change`
(`parallax.rs:325`) despawns every parallax layer when `GameAssets` is changed,
and `present_live_room_parallax` spawns them again. The writer:
`prepare_first_room_art_system` (`ambition_app/src/app/world_flow/first_room_art.rs`)
took `GameAssets`, the atlas layouts and the character load states through
`as_deref_mut()` every frame, also with no session to prepare. Fixed (it
returns before the borrows when every published session is answered).
Verified (`probe/basement_enemies_v3`): `GameAssets` changed on 0 frames, no
parallax spawn or despawn.

### F18 — Other steady churn of ours (probe)

MEASURED (probe churn, per frame at idle):

- Sanic: `gravity_visuals.rs:29` despawns and respawns every gravity-zone
  visual every frame (1.43 each way in the first probe run); it runs in
  `Update` with no condition. Fixed; verified by a unit test (an unchanged
  zone keeps its entities). The later Sanic runs, before and after the fix,
  had no live gravity zone at the captured point, so the probe cannot show
  the difference.
- Mary-O: `crates/ambition_render/src/rendering/actors/rigged.rs:923` spawns rig part slots and despawns as many, ~1.4 a
  frame each.
- Every game: `lifecycle/session.rs:316` and `:326` insert
  `SessionScopedEntity` and then `InRoomInstance` as two inserts after each
  session spawn (4-8 a frame each): three archetype moves per spawn instead of
  one. Fix: one bundle in the spawn.
- Outside our code: `FixedMainScheduleOrder` removed and inserted per fixed
  tick (Bevy), `bevy_ggrs` reinserts `Session`, `PlayerInputs` and
  `FixedTimestepData` every frame.

### F19 — A widely shared system parameter is a per-system cost

MEASURED (phone, `--ours --thread`): the largest line of ours on the main
thread, 1.36% of the process, is `lifecycle/room_instance.rs:172`, the
`#[derive(SystemParam)]` of `LiveRooms`: the fetch and validation of its two
queries (`roots`, and `stamps` over every entity with `InRoomInstance`), with
the bodies inlined. About 208 parameter declarations in `crates/` and `game/`
take `LiveRooms` or `LiveRoomOf` (git grep), so the cost is paid once per run
of each of those systems.

The same holds for the two room-spec parameters of
`ambition_platformer2d_world/src/rooms/room_graph.rs`: `LiveRoomSpecs` (its
derive line, 0.51%; it embeds `LiveRooms`) and `SoleLiveRoomSpec` (0.32%).
Together the room-lookup parameters are ~2.2% of phone samples. All
parameter fetches together (`get_param`, inclusive, main thread of the
all-games run) are 3.59%. Users by a
`git grep` of parameter declarations: `LiveRooms` ~120, `LiveRoomSpecs` ~90,
`LiveRoomOf` ~72, `SoleLiveRoom` ~17, `SoleLiveRoomSpec` 5. (The next phone
line of ours, `crates/ambition_touch_input/src/virtual_device.rs:50`, 0.46%, is the touch button's
`decompose`, called by leafwing's clash pass; it went with F13.)

Reading (REASONED): the live-room set is one or two values that change on a
room transition. A resource kept by one system (the live rooms in order, and
each band) is a cheaper parameter than a query: no archetype matching and no
per-run query validation. A system that asks for the room of its own entities
can carry `Option<&InRoomInstance>` in its own query. Measure one system first.

### F20 — Every body's whole state is marked changed every tick

MEASURED (probe `changes_by_site.csv`, standalone Mary-O 1-1, 561 frames
after warm-up, `scratchpad/probe/mary_o_1_1_v2`). About 2,300 component values
are marked changed a frame. Of ours:

- ~30 simulation components of each of the ~17 bodies, every frame
  (`BodyKinematics`, `BodyHealth`, `BodyLedgeState`, `BodyDodgeState`,
  `BodyRestartLatch`, ... at `actor_clusters.rs:170-198`, and
  `update.rs:1508-1543`). Cause (source read): `as_actor_mut`
  (`actor_clusters.rs:163`) takes `&mut` of each cluster through `Mut<T>`,
  which marks it changed whether or not the step writes it.
- More per-body writes each frame: `BodyLifeStats` (`features/mod.rs:280`),
  `ResolvedTechniqueEdges` (`starting_character.rs:705`), `BodyMelee`,
  `RangedRefire`, `AttackGestureState`, `ResolvedAttackGesture`,
  `PogoTargetVolumes`, `BodyAnimFacts`, `PresentedPose`, `BodyPoseView`.
- Resources changed every frame by our code: `ActiveAudioSelection`
  (`authorize_staged_character_presentation_sources`, F16, fixed since), `GizmoConfigStore`
  (`avatar/trail.rs:677`), `CameraShakeState`, `FinishZoomState`
  (`camera_ease.rs`), `SeatControlFrameModes`, the narrative ledgers
  (`prune_narrative_inputs` and `release_narrative_inputs`, both fixed since)
  in a game with no dialogue, and two developer-tool
  resources (`EditablePlayerStats`, `PlayerStatsSyncSnapshot`).
- Rendering: rig part slots (`rigged.rs:1426-1434`, ~148 a frame:
  animation), the HUD's `Visibility` (`hud/declared.rs:784`, 30 a frame),
  ambient motes, hit-flash overlays (`hit_flash.rs:416`, fixed since).

Readers today (MEASURED, `git grep` for `Changed<T>`, `Ref<T>` and
`is_changed` per component): none for `BodyLifeStats`,
`ResolvedTechniqueEdges`, `BodyMelee`, `RangedRefire`, `AttackGestureState`,
`ResolvedAttackGesture`, `PogoTargetVolumes`, `BodyAnimFacts`,
`PresentedPose`, `BodyPoseView`, `DamageableVolumes`, `ActorRoll`,
`BodyHealth`; three for `BodyKinematics`, one for `ActionSet`. So the per-body
false changes cost nothing now; the resource false changes did (F15, F17).

Reading (REASONED): a false change costs little by itself. It matters
because it makes change detection useless: no `Changed<Body…>` filter, and no
read-model "built by change" (P5, P12), can skip a body while all of its state
is marked changed every tick. So the order is: first make writes mark only
real changes (pass `Mut<T>` into the step, or compare before writing), then
the change-driven read-models pay. The detector verifies each step.

Sanic act 1 (MEASURED, `probe/sanic_act1_v2`, 561 frames idle): ~1,200
values changed a frame; 79 components and resources of ours are changed on
about every frame. Among them the portal presentation takes `Assets<Mesh>`,
`Assets<Image>` and `Assets<ColorMaterial>` through `&mut` every frame
(`ambition_portal2d_presentation/src/view_cones.rs:1035-1037`) in a game with
no portals: a composition cost (P1) and a false change. Also every frame: the
input-system resources (`SeatControlFrameModes`, `SeatRawFrames`,
`MenuInputState`, ...), the camera read-models, the music and audio state,
the lifecycle resources of the provider (`ShellRouteHolds`,
`PreparedPlatformerSessions`, ...), the two trace buffers, and all the
narrative and host-intent ledgers.

Fixed with read-before-write, verified with the counter (MEASURED,
`probe/mary_o_1_1_v3`): `ActiveAudioSelection` (the SFX authorizer now runs
only when one of its inputs changed; its ~200 allocations a frame are gone),
`GizmoConfigStore` (trail bands), the host-intent ledgers (prune). The
narrative ledgers had a second every-frame writer, the release
(`ambition_conversation/src/ledger.rs:194`), now fixed the same way (covered
by the conversation tests).

Write-only-a-change in the presentation (MEASURED, `probe/mary_o_1_1_v5`
against `_v4`): the HUD helpers `set_shown`/`set_hidden` took
`&mut Visibility`, and a `Mut` coerced to it at the call is marked changed
before the helper's test runs, so every HUD node changed every frame; now
generic over the `Mut`. The feature visuals (`actors/mod.rs`) and the
hit-flash overlays now use `set_if_neq`. Per frame: `Visibility` changes 74 →
1, transforms propagated (`GlobalTransform`) 224 → 182, all value changes
2,216 → 2,017. The hub capture is pixel-identical.

Verified with it (MEASURED): the `star.rs` fix (F15) works. After it,
`ActionSet` and `ActorMoveset` were changed 17 times in 561 frames (loading),
and `reconcile_effective_repertoire` has no allocations left.

### F21 — Presentation entities drawn in immediate mode

MEASURED (probe churn): three presentation systems despawned all their
entities and spawned them again every frame: gravity-zone visuals (Sanic,
fixed), the parallax refresh (hub, F17, fixed at its cause), and wielded-item
overlays (`wielded_item_visuals.rs:153`, hub, 1.000 a frame; rewritten to
update in place; verified: no churn, pixel-identical capture). A code search for the same shape
(despawn every entity of a marker, then spawn) also finds
`item_visuals.rs`, `projectile_visuals.rs`, `mark_beacon.rs`, `hud/declared.rs`
and the portal presentation (`visuals.rs`, `gun_visuals.rs`, `far_side.rs`);
not all of them run every frame.

Reading (REASONED): this is immediate-mode drawing written with retained
entities. Each frame pays two archetype moves per entity, new render
entities and extraction state, and change marks for everything downstream.
The elegant pattern is one shared helper: keyed reconcile (key → entity:
update in place, spawn the new, despawn the gone), as the gravity and wielded
systems now do by hand.

### F22 — The serial executor makes a standalone tick about 6× cheaper

MEASURED (desktop VM, standalone Mary-O 1-1 offscreen, `capture_mary_o
--warmup 1200`, profiling build, 3 interleaved repetitions per arm, medians;
`scripts/executor_ab.sh`, `scripts/executor_ab_report.py`; the switch is
`AMBITION_SCHEDULE_EXECUTOR`, `apply_executor_policy` in
`ambition_platformer2d_runtime/src/lib.rs`):

| arm | sim wall per tick | main schedules, ms per second | frames per second | process CPU, s |
| --- | ---: | ---: | ---: | ---: |
| `parallel` (the windowed demos today) | 10.70 | 968 | 68.5 | 103.0 |
| `frame-serial` (`First`..`Last` serial; the hub's frame policy) | 10.78 | 918 | 73.1 | 99.4 |
| `serial` (also `Fixed*`, `StateTransition`, `SpawnScene`) | 1.68 | 745 | 90.1 | 96.6 |

The spread between repetitions is small (sim per tick 1.67-1.76 serial,
10.52-10.72 parallel). Process CPU is mostly software rendering on this VM
(llvmpipe), so the schedule times are the measure. Serializing only the frame
schedules does almost nothing for a standalone game: its simulation is in
`FixedUpdate` (501 systems), which stays multi-threaded.

Reading (REASONED): with ~500 small systems in `FixedUpdate`, the
multi-threaded executor's per-system dispatch costs more than the systems.
The phone shows the same shape (F11: main thread saturated, pool idle). The
hub's simulation is in `GgrsSchedule`, which the rollback crate already runs
serially, so this is a standalone-game lever (Mary-O, Sanic, Smash). The
single-threaded executor runs systems in a valid order of the declared
constraints; the multi-threaded one already may run unordered systems in any
order, so no new order is possible. Check of that (MEASURED, the hub A/B
captures, `capture_scene` steps deterministically, 1,200 frames): the player
ends at the same point in every run of every arm; `serial` and `frame-serial`
captures are pixel-identical; `parallel` differs from both in 218 of 921,600
pixels, the same 218 in each of its three runs. All 218 are brighter in
`parallel`: the room's ember motes, drawn under `parallel` and off the frame
under `frame-serial` and `serial`. Cause (MEASURED with temporary logging in
`drift_room_motes`, not kept): the motes read the camera at y = +290 under
`parallel` and at y = −290 under `frame-serial`, every frame. The other
writer of the camera is the capture tool itself: `apply_capture_snapshot`
(`game/ambition_app_tools/src/bin/capture_scene.rs`) pins the camera with no
order against `drift_room_motes`, so under `parallel` it runs first and under
the serial orders after. So this is an artifact of `capture_scene`, not of
the game: in the game only `camera_follow` writes the camera, and the motes
run after it. The rest of the frame and the player are the same in every
arm. Fixed for captures (2026-10-16): `apply_capture_snapshot` now runs before
`drift_room_motes` and `light_the_world`; the `parallel`, `frame-serial`
and `serial` hub captures (600 frames) are then pixel-identical (MEASURED). Recommendation: make `Serial` the default
of every foundation, then remove the switch; check `room_replay` under it
first.

**Done 2026-10-17 (D12).** `serialize_main_schedules`
(`ambition_platformer2d_runtime`) is the one executor policy, called by the
headless, windowed and hub foundations. Deleted with it: `ExecutorPolicy`,
`apply_executor_policy`, the switch `AMBITION_SCHEDULE_EXECUTOR`,
`scripts/executor_ab.sh` and its report, and `ambition_app`'s second switch
`AMBITION_MAIN_SCHEDULES_SINGLE_THREADED`. The tables above are the record of
the A/B. MEASURED after it (profiling build, `capture_mary_o --warmup 600`
with `perf_probe`, 1-1, before and after interleaved, two runs each;
`time_by_span.csv`): `FixedUpdate` 12.66 and 12.51 → 4.31 and 4.13 ms a
tick, the whole frame 41.6 and 37.9 → 22.8 and 23.1 ms. The probe's spans
add to every system run, so the probe-free ratio of the table above (6×) is
the better number. Witness `every_main_schedule_runs_its_systems_on_the_calling_thread`
(`ambition_platformer2d_runtime`): it can tell the executors apart only with
`bevy/multi_threaded`, which only the visible builds have; with it, the
witness is red at "320 of 320 probe runs of First were on another thread"
without the call, and a headless build prints NOT RUN. So the earlier
"Mary-O's and Sanic's tests pass with the switch at `serial`" was vacuous:
their headless builds ran one thread in every arm.

### F23 — In the hub the executor is not the lever; the frame phases are

MEASURED (hub `basement_enemies`, `capture_scene`, same method as F22,
`CAPTURE="capture_scene basement_enemies 0,0 {png} 1280x720" scripts/executor_ab.sh`):

| arm | sim wall per tick | main schedules, ms per second | frames per second |
| --- | ---: | ---: | ---: |
| `parallel` | 1.84 | 946 | 57.6 |
| `frame-serial` (the hub today) | 1.90 | 847 | 63.2 |
| `serial` | 1.93 | 823 | 66.1 |

The hub's simulation runs in `GgrsSchedule`, which the rollback crate already
runs serially, so it costs ~1.9 ms a tick in every arm, and `serial` adds only
3% less main-thread time over today. Main-thread time per second of the hub
today, by phase: `RunFixedMainLoop` 227 ms (the simulation is ~120 of it),
`Update` 193, `PreUpdate` 190, outside any schedule 159, `PostUpdate` 137,
`StateTransition` 53 (16 systems: about 0.8 ms a frame), `Last` 19,
`SpawnScene` 18. Standalone Mary-O (F22, parallel) is the opposite shape:
`RunFixedMainLoop` 673 of 968.

Reading (REASONED): the phone runs the hub app, so for the phone the levers
are the frame phases: fewer systems in them (P1, P3), the false changes and
re-derivations (P12, P13), and input (P11). The executor switch (P2) is the
standalone games' lever. `StateTransition` at ~0.8 ms a frame for 16 systems
is worth one Tracy look.

### F24 — The collision world is copied whole, by each system that asks

MEASURED (phone, all-games profile, main thread,
`symbolize_android_profile.py --inclusive --thread 8529`): under
`integrate_sim_bodies` (2.21% inclusive), `world_with_sandbox_solids` takes
0.58% (`world_with_moving_platforms`, 0.56%, is inside it). A generic
`clone` frame at 1.76% sits beside `integrate_body`, but the name is too
generic (and identical code is folded) to say what it copies.
Source read: `RoomCollision::solids`
(`ambition_platformer2d_world/src/collision.rs`) borrows the authored world
only when the room has no moving platform and an empty overlay; otherwise it
builds a new `ae::World`: a copy of the whole room, then the platforms, the
overlay's blocks, lines, gate solids, water and portal carves. The body step
does it once per room per step (`ComposedRooms`); about twenty other systems
(`git grep` of `.solids()`: brains' traversal, body modes, damage, held items,
boss ticks, pets, the traces, ...) each compose their own when they run.

Reading (REASONED): two steps, each without behaviour change. First, one
composed world per room per tick, kept on the room root and rebuilt when its
geometry, platforms or overlay changed, which every reader borrows. Then a
layered view (the authored world plus the dynamic parts, read in place) so
that nothing copies the room at all; `HostSurfaces` already reads two
sources in place this way.

**Step 1 done (2026-10-17).** `ComposedSolids` (`collision.rs`) keeps the
composed walls of each room in a few places; every reader of
`RoomCollision::solids` borrows them. A place is keyed by the change ticks of
the room's geometry and platforms, and by the contents of the overlay
(`FeatureEcsWorldOverlay::composes_like`): the overlay is cleared and filled
again each tick, so its change tick always moves. A reader whose key no place
holds composes its own, so the retire (`retire_stale_composed_solids`, after
the platforms advance) changes only how many readers share, never what one
collides with. `advance_moving_platforms` no longer marks an empty set
changed. MEASURED (debug, `mary_o_it` in 1-2, holding right and jumping, the
cache removed and put back in one run, 3 × 200 frames each, the box loaded by
a parallel `app_it`): compositions a frame 5.7-6.0 → 1.0; frame time 15.1 →
13.2, 15.2 → 13.1, 15.2 → 15.2 ms. In 1-1 there are none (no platforms, an
empty overlay: the borrow fast path). Witness
`the_readers_of_a_tick_share_one_composed_world_and_a_write_composes_again`
(red with no place kept, and red at "a reader after the write collides with
the old overlay" with the overlay left out of the key). Left: the one
composition a tick in a room with moving platforms (step 2, the layered
view), and the projectile view (`world_with_gate_solids_and_carves`) and
`hostable_surfaces`, which compose apart.

**Step 2 done by the portal work (da519ec6d, 2026-10-17).** `World::blocks`
and `World::chains` are now `ae::layered::Layered`: one shared base and a
small layer for each world. A clone of the room's world copies no block, and
`world_with_moving_platforms` and `world_with_sandbox_solids` add the
platforms and the overlay as a layer. What a composition still copies is
`water_regions` and `climbable_regions` (`Vec`s, a few entries in each room).

### Not a lever (keep out of this run)

Session-scoped App globals (C03: 30 counted plus 19 uncounted instances) are
reset only on session edges: correctness work (S6), no per-frame cost. The
`ambition_platformer2d` facade (209 `pub use`) costs compile time only.
Rollback costs about 0% offline (`check_distance: 0`).

## Proposals, ranked by structural leverage

| # | Proposal | Removes | Size | Behaviour change |
| --- | --- | --- | --- | --- |
| P1 | Per-game engine profiles: `BossCatalog` always present (empty); ledgers under Dialogue; grabs, mounts, match stocks as group members with capabilities; extension lowering installed by the capability that owns the phase; demo hosts call `for_profile` | about 115 systems a tick in Mary-O and Sanic | large | no, except decisions D3, D4 |
| P2 | DONE 2026-10-17: one executor policy (`serialize_main_schedules`), called by every foundation including the windowed demos, covering `Fixed*` and `StateTransition` (F22, F23): a standalone tick ~6× cheaper for Mary-O and ~2.4× for Sanic, the hub +3% | the multi-threaded executor cost of every frame and fixed system in the windowed demos | small | no |
| P11 | DONE for the clash pass, in-repo (F13): `press_all_action_update`; an upstream PR for the pass and the boxed lookups remains | 19-32k allocations a frame, ~80% of a standalone frame's | small | no |
| P12 | Derive on change, not every tick (F15, F16, F20): first writes that mark only real changes (the cluster step, the resources of F20), then the four systems; the change detector (`changes_by_site.csv`) finds the next ones and verifies each | per-tick re-derivations and false changes that wake change filters | small each | no |
| P14 | One composed collision world per room per tick, shared; then a layered view with no copy (F24) | ~0.6% of phone CPU in the body step, plus each other composer | medium | no |
| P13 | One keyed-reconcile helper for presentation entities (F21), and the remaining immediate-mode systems moved onto it | per-frame despawn and spawn of presentation entities | small-medium | no |
| P3 | Hub: gate each game's session gameplay by its scope (S2, in part: F31); install Ambition, physics debris and 3D lighting only for the games that use them (S2b); move shared techniques out of `ambition_demo_smash` | 82+ foreign systems while one game plays, 72 hub-only systems | medium | no |
| P15 | Render primitives apart from simulation objects: static visuals as built meshes (the terrain skin is done, F32; next the fixtures, unskinned tile blocks, the hidden sprite each skinned block keeps) | per-entity visibility, transform and extraction work for static art | medium | no |
| P16 | A repeat-allocation pass: the top call sites of `alloc_traces.csv` reuse their buffers | part of the ~4,900 allocations a frame in the hub (F25); mimalloc ~3.5% of phone samples | small each | no |
| P17 | Baked navigation graphs: a graph is a function of room geometry and body, so build it once (content time or a cache), not at each room entry | the 11 s phone build of 1-2's graph (autoplay, later NPCs) | medium | no |
| P18 | Each game's systems in its `RulesScopeSet`, not a condition on each system (F34) | a condition evaluation for each gated system | small | no |
| P6 | Change-driven systems: the two whole-world queries, input bindings, room isolation (F14) | full scans per tick; 444-2,773 allocations a frame in room isolation | small | no |
| P4 | Interned identities: an integer handle per body, made when its `SimId` is minted; read-model keys by `Entity` | String clones and sorts in 5+ per-tick systems | medium | no |
| P5 | Read-models built once per drawn frame, by change | repeat builds on slow frames; builds in headless hosts | medium | D2 |
| P7 | Plumbing: one gizmo trail group, Bevy debug gizmos off in shipping, developer tools only in developer builds, Kira channels | ~80 systems | small-medium | D6, D9 |
| P8 | Upstream query-state fixes (LDtk, Yarn, bevy_light, GGRS) | ~2% of phone CPU | small each | D5 (only for the LDtk workaround) |
| P10 | Phone render scale: a reduced main render target, scaled to the surface (`crates/ambition_render/src/quality.rs:190` has no phone path) | fragments, when the GPU is the limit; not the limit today (F11) | medium | D10 |
| P9 | Content to data: held items, the item enum, Ambition-only item systems | Ambition work in other games; engine edits for content | large | D7 |

## Implementation plans

### P2 — one executor policy (one commit, an A/B, one commit) — DONE 2026-10-17

- Rename `serialize_frame_schedules` to `apply_executor_policy(app)`
  (`runtime/src/lib.rs:741`), the only place that sets a main-world executor.
  It serializes each schedule that exists: `First`, `PreUpdate`,
  `StateTransition`, `Update`, `SpawnScene`, `PostUpdate`, `Last` and
  `FixedFirst`..`FixedLast`. It leaves the render sub-app and Bevy's own serial
  schedules alone; the GGRS schedules stay with the rollback crate (measure
  `SaveWorld`/`LoadWorld` in the same run).
- Call it from `install_windowed_foundation` as well; delete the second owner
  (`ambition_app/src/app/plugins.rs:348-374`); fix the false comment at
  `combat_schedule.rs:985`.
- A/B: one variable, `AMBITION_SCHEDULE_EXECUTOR` = `frame-serial` (today) |
  `serial` | `parallel`, read once, logged once, deleted after the decision.
  Standalone Mary-O needs a committed tick-only binary (`build_demo_app()`,
  `--ticks N`) so `[census] phases` is not dominated by rendering; 3+
  interleaved repetitions per arm; equal `AMBITION_SYSTEM_LIST` per arm; the
  `room_replay` test under each arm for equal state.

### P1 — per-game engine profiles (ordered commits, each green)

`supported_profiles.rs` iterates `SUPPORTED_PROFILES` and `Capability::ALL`, so
each new capability gets the step test, the not-installed probe and the
session-edge check from the existing harness.

1. **C1 Extension phases by owner.** `technique_execution` and `boss_conduct`
   into `BossExtensionPhasesPlugin`; `wielded_use` and `module_entity_tick` into
   `WieldedExtensionPhasesPlugin` (`extension_composition.rs:32-205`); install
   order inside each phase kept. Delete the second `BossCatalog` init; gate
   `drive_boss_animators` on the registry. Test: no port of an omitted phase is
   offered; a `boss_conduct` module is refused at admission. About 29 + 20
   systems.
2. **C2 Dialogue owns the ledgers and the dialog view.** The seven monolith
   ledgers into `NarrativePayloadLedgersPlugin`; `rebuild_dialog_view` out of
   `SimViewPlugin` into `DialogViewPlugin`. About 22 systems (only for a game
   that omits dialogue, D3).
3. **C3 Mounts.** `MountsPlugin` takes `combat_schedule.rs:776-800` and
   `install_mount_pose_systems`; the mount ports in the boss phases are
   installed only with mounts, decided by the composition root. 7 systems.
4. **C4 Match stocks.** `StocksMatchPlugin` takes `features/mod.rs:925-975` and
   the finish zoom; the Smash and versus rules assert it. 7 systems.
5. **C5 Grabs.** `CapturePlugin` takes `combat_schedule.rs:525-672`; the control
   gate chain splits on two capture-owned sets (same order). 14 systems, only
   after D4.
6. **C6 Game profiles.** A profile `platformer-without-bosses-mounts-stocks`;
   each game exports `ENGINE_PROFILE`; the Mary-O and Sanic hosts call
   `for_profile`; pack admission compiles against the schemas of the profile.
   About 55-85 systems a tick per standalone game; re-measure.
7. **C7** after D3 and D4: Dialogue, Grabs and the Smash moveset leave the
   standalone profiles.

## Decisions for Jon

Each of these changes what a player sees or what a reader gets. Nothing here
is done without a yes.

- **D1 — 30 Hz simulation on low tiers.** At most 13-17% of phone CPU (the
  simulation is 26-35%). Breaks unless fixed first: riders on rising one-way
  platforms (14 px a tick against 8 px slop), regions under 52 px, hitstop
  length, NPC jump prediction, AI and enemy cadence in ticks. Needs the
  tick-rate plan's P0 items.
- **D2 — Read-models per drawn frame.** RL, headless and trace readers must ask
  for them.
- **D3 — Standalone Mary-O and Sanic without Dialogue.** Both have
  `fallback_dialogue` rows. No standalone build installs a dialogue presenter
  today (only `ambition_app` installs the Yarn bridge), so those lines are
  never shown there now. Refuse such rows at admission, or ignore them?
  REASONED (code read): a standalone game can enter `GameMode::Dialogue` and
  not leave it. `DialogSimStatePlugin` is installed, its `DialogueNodeIndex` is
  unpopulated, so `entry_node` opens every non-self conversation
  (`ambition_dialog/src/context.rs:122`), and `interact.rs:395` sets
  `Dialogue`; the only system that returns to `Playing` is
  `sync_dialogue_game_mode`, which only `ambition_app` installs. Today the rows
  with dialogue are Hall characters (`tier: MainHall`), and the standalone
  level actors are hostile, so no reached path is known. Omitting Dialogue from
  the standalone profile (C2/C7) removes the path.
- **D4 — Smash moveset and grabs out of standalone Mary-O and Sanic.** Their
  own movesets author attacks and `smash.capture_attempt/pummel/throw`, so this
  means moving their fighter kits out of the standalone packs (the hub keeps
  them).
- **D5 — LDtk respawn one frame late** (a set condition) instead of waiting for
  the upstream fix.
- **D6 — Kira dynamic channels.** Changes how music layers crossfade.
- **D7 — Unknown boss ids refused at admission** instead of the invented
  fallback boss.
- **D8 — `SimId` minted by a spawn observer.** Moves where in the tick identity
  is made (determinism).
- **D9 — The per-room gizmo trail with two or more live rooms:** rebuild on
  retained gizmos, or drop.
- **D10 — Phone render scale** (P10): a lower internal resolution on the low
  tiers. Visible. Worth it only when a phone run shows the render thread or
  the GPU as the limit; today the main thread is (F11).
- **D11 — A catch-up limit** of N ticks per drawn frame (F12): below the limit
  the game keeps real time; above it, the game slows down instead of dropping
  more frames. Visible under load only.

Two items are ready and change no behaviour, but each needs a yes for
another reason:

- **D12 — `Serial` as the default executor of the windowed demos** (F22).
  One line in `install_windowed_foundation`. With
  `AMBITION_SCHEDULE_EXECUTOR=serial` these pass: Mary-O's game crate (208),
  Mary-O's integration tests (95, the room-replay tests among them) and
  Sanic's (42). Standalone Sanic, same binary, 3 reps per arm (MEASURED,
  12:40Z; a 2-rep run at 12:35Z gave 7.39 and 3.03): 7.21 ms of simulation
  per tick parallel, 2.98 ms serial; main thread 948 → 896 ms per second;
  wall time 14.1 → 12.0 s for the same capture. `frame-serial` (12:37Z, 2 reps) gives 7.18 ms: the saving is all
  in the `Fixed*` schedules, which only `serial` changes. It changes how every standalone game
  runs, so it should land with a play test, and the switch is then removed.
  The one visible difference in the hub captures was the ambient motes, an
  ordering artifact of `capture_scene`'s camera pin, not of the game (F22).
- **D13 — withdrawn.** The leafwing clash pass is skipped in this repo with
  no fork (F13). An upstream PR (skip the pass under `PressAll` in
  `process_actions`; lookups without boxing) is an outward action for Jon.

## A second opinion, weighed (GPT, 2026-10-16)

Jon passed on GPT's diagnosis. Its points against this page's evidence:

| GPT's point | Here | Verdict |
| --- | --- | --- |
| Mary-O pays for Ambition's capabilities; install less, not faster | F0, F1, P1 | agrees; MEASURED (-17% CPU with only Mary-O in the hub; 170 unused systems a frame in standalone Mary-O) |
| Read-models rebuilt per tick, so slow frames repeat them | P5, D2, F12 | agrees; REASONED (no limit of ours on catch-up, F12) |
| Phones have no reduced render target | P10, D10 | true (`crates/ambition_render/src/quality.rs:190`); not the limit of today's runs: the main thread is the limit and the render thread is idle 75% of the time (F11) |
| Transparent overdraw (5.3x on the HD 630) | journal 2026-08-29 | a GPU cost on a desktop iGPU; not measured on the phone; same reasoning as P10 |
| Effects composite bodies off-screen | — | agrees it is not a steady cost (direct part drawing since 10-05); spikes only; not measured here |
| Asset upload stalls cause the worst drops | — | plausible for p95; the phone runs did not separate spikes from the steady cost |
| Single-threaded executor for the standalone fixed step | P2 | agrees, and wider: the windowed demos run every schedule multi-threaded (F3), and F11 shows why it matters |
| Keep 60 Hz for now | D1 | agrees; D1 stays Jon's call |
| Do not rewrite movement, broadphase or AI | — | agrees; no system body of ours is over 1.4% on the main thread (F11) |

## A code review of the slices, acted on (GPT, 2026-10-17)

Jon passed on a review of the 10-16/17 slices. Each finding, with what was
done:

| Finding | Done |
| --- | --- |
| P0: the navigation graph key left out world inputs that the kernel reads (`1564bae`) | `geometry_stamp` folds the whole `World` through a serializer (11.5 µs a 400-block room, release; once for each world in a run). `144281d79`; witness `each_part_of_the_world_the_kernel_reads_moves_the_stamp` |
| P1: terrain mesh invalidation followed only blocks that came or went (`fd40043`) | Also a block's rectangle, the keep-out, the decor density, a picture's size (in the chunk stamp) and a part's picture handle. `407864224`; witness `the_skin_follows_each_of_its_inputs_changed_in_place` (one registered system: a `run_system_once` system sees each filter as changed) |
| P1: the incremental sprite check trusted a first generation it never checked (`0dc21f5`) | `SheetGeneration::rigs_checked`; an unchanged rig is taken unchecked only from a checked generation. `ba91ba1da` |
| P2: retired debris took budget places and kept Avian awake (`83bc8a8`); colliders built with debris off | `7aabe0e84` |
| P3: capability dormancy read THE room, so two live rooms ran every capability | `GoverningRules::any_live_room_uses`. `876c0aea5`; witness `a_capability_sleeps_while_no_live_room_uses_it` |
| P3: navigation first-build latency | Not changed: only the autopilot builds a graph today, on the async compute pool; no Mary-O body navigates. P17 (baked graphs) stays the direction |
| P2: profile the Mary-O 1-2 simulation on the phone | No device on this machine; measured on the desktop instead (F38): many walking bodies, each a full kernel step; the dormant-at-rest lever is recorded there |

## Run log (2026-10-16)

- 04:40Z armed. Inputs: four read-only audits of 10-15 (composition plan
  status, consolidation C03/C08/C09, GPT's 10-11 notes and the tick-rate audit,
  warts and queue) and a fresh system census; the phone baseline above.
- Android attribution: `profile_android.sh` now keeps every sample's call chain
  (`simpleperf report-sample --show-callchain`); the `-g` reports drop branches
  under 0.25%, so they could not name a system. `symbolize_android_profile.py`
  picks the library of the sampled build (it took the first of two installs),
  and has `--lines` and `--ours` (commit 1e1ca6525's parent).
- 05:30Z phone: the sample dump of the baseline (Jon) gives F0; the
  Mary-O-only run gives -17% CPU. Jon cannot run the phone again tonight; the
  rest of the run measures on the headless desktop.
- 05:30Z an agent starts the `perf_probe` feature (allocations per span,
  churn per source line).
- 05:40Z P1/P2 plans written (an agent's code read); F1/F2/F3 corrected (bosses
  can already be omitted; twelve ledgers; the windowed demos never serialize).
- 05:58Z `symbolize_android_profile.py`: a threads table and `--thread`. F11:
  the main thread is the limit (a first table summed it with the render
  thread, which has the same name; now split by thread id). F12: no catch-up
  limit. GPT's diagnosis weighed.
- 07:41Z `perf_probe` committed (6f14f4eb7): allocations per span, churn and
  value changes per source line. Findings F13-F20 from its runs (desktop,
  standalone Mary-O and Sanic, hub `basement_enemies`). Fixes with no
  behaviour change, verified by the probe or a test: Mary-O's star no longer
  re-folds the repertoire every tick (F15); gravity-zone visuals are no
  longer respawned every frame (F18). Next: the hub run with the change
  counter, to confirm F17.
- 09:13Z F17 cause found with the change counter and fixed (`first_room_art.rs`);
  wielded-item overlays updated in place (pixel-identical capture). Executor
  A/B: standalone Mary-O (F22) and hub (F23); switch committed, defaults
  unchanged. Sanic change census. Read-before-write fixes for the SFX
  authorizer, the narrative and host-intent ledger prunes and the trail gizmo
  bands, verified with the change counter (F20).
- 09:56Z Room isolation walks only when a mask can be stale (F14); HUD,
  feature-visual and hit-flash writes only of a change (F20); seat bindings
  projected only on a change; Mary-O and Sanic hosts without named boss
  encounters (F1: −13 and −11 systems a frame). Before/after table in the
  summary. Remaining large items need a yes (D12) or are P1's C1.
- 11:37Z Leafwing's clash pass skipped in-repo (F13; Mary-O −82%, Sanic −85%,
  hub −68% of allocations a frame); F7 filters; `--inclusive` and `--under`
  in the phone symbolizer; F24 (collision world copies). End-of-run check,
  `cargo test -p ambition_app --test app_it`: 1,250 pass, 5 fail, none from
  this run: the system-count ceiling (3,505 against 3,500; the leafwing
  replacement removes two systems and adds two), `EmbeddedPackWatches` not in
  the rollback ledger (two tests; the content live-reload work), and the FX
  sheet on disk without a `hit_soft` row (two tests). `ambition_combat` 740 pass;
  the monolith 1,406 pass, 1 fails, not from this run:
  `held_item_steps…sit_where_they_say` finds two systems in
  `HeldItemStep::Physics` (`right_ground_items`, added in 2479a4c37). At 12:33Z
  the test expects two, and it passes (MEASURED). The runtime
  crate (executor policy) 83 pass; `ambition_platformer2d` 10 pass.
- 12:30Z The executor arms' one picture difference traced (F22): the motes
  read the camera before `capture_scene` pinned it under a serial order. The
  capture tool now orders its pin first; all three arms are pixel-identical;
  `capture_scene`'s tests pass (7).
- 12:35Z Standalone Sanic executor A/B (`capture_sanic`, 2 reps, parallel
  vs serial): simulation 7.39 → 3.03 ms per tick, the same direction as
  Mary-O. D12 now has a measurement for both standalone games.
