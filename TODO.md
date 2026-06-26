# Ambition TODO

Centralized work queue for multi-hour autonomous agent sessions.

This file is intentionally **not** a changelog. It is the place to keep unfinished work that is useful enough for an agent to pick up without re-discovering it from git history, scratch notes, or old overlay readmes.

## Operating rules

- **Sandbox-first.** The sandbox is the vertical slice: gameplay components should be assembled in test-arranged rooms before they are story-arranged into the final hub / first zone / boss sequence.
- **Outstanding work lives here.** This file is the centralized queue for unfinished behavior, docs, and validation work; completed behavior belongs in current system/concept docs or archive notes only when historical context is useful.
- **Verify before claiming completion.** Re-grep code and docs before closing an item. Many old TODO entries turned out to be complete or superseded.
- **Move, do not duplicate forever.** When a task lands, either remove it from this file or move the durable lesson to `dev/journals/`, `docs/current/`, focused system/concept docs, or `docs/archive/`.
- **Prefer agent-sized tasks.** Each accepted item should be concrete enough for a 1-4 hour autonomous session with clear files, tests, or validation commands.
- **Sprites + SFX ship with the entity.** When a task adds a new actor or prop that needs a visual, creating its sprite — and any necessary SFX — is implicitly part of that task. Do not call an entity complete on a placeholder / generic-fallback / invisible visual without filing a tracked sprite follow-up.

Useful companion docs:

- Current state: `docs/current/state.md`
- Current next moves: `docs/current/next.md`
- Planning sequence: `docs/planning/path-forward.md`
- Tech debt: `docs/planning/tech-debt-log.md`
- Mechanics status: `docs/mechanics/expressibility-checklist.md`

## Persistent autonomous-loop instruction

When you wake up here, pick the next task from this list and work on it without asking permission. Honor the long-running discipline (`[[never-stop-during-long-run]]` in memory): never stop until the time limit, even if the headline task feels finished. When you do close a task, leave this instruction in place so the next agent finds it.

## Jon's Bug List

- [ ] The attack hitbox direction is bugged in non-down gravity. 


## Jon's Polish List

- [ ] Portraits for all sprite character need to be generated.

- [ ] Need to hookup portraits in dialog.

- [ ] Little character talking sounds for dialog

- [ ] A better hit sfx for the player

- [ ] A better swing sfx for the player 

- [ ] A better footstep sfx for the player 

- [ ] A better jump sfx for the player 

- [ ] A better double jump sfx for the player 

- [ ] Spritesheet packing (and spritesheet splitting so we keep sheets efficient packed for memory).

- [ ] Cell-ular automata encounter that starts with encountering them and having to talk to them, only once you choose the challenge option does the fight / encounter begin. The boss has an advanced enemy fighter brain that we try to program to be smart and reactive (never cheating, we can improve the AI over time, but we should try to start with a strong candidate.). 

- [ ] We need a way to orient generic props in LDTK so we can ensure the switches in the C4-symmetry room actually make a symmetry.

- [ ] Make abilities like fly, blink, fireball cost mana. For now we can have the player character have an ability that constantly regenerates mana so we can still navigate and test things quickly in "god mode".

- [x] **Tie-a-knot trail mechanic** `[V?/D?]` - Add a feature where the player emits a trail that can drape onto collision, be pulled taut, and work through portals.

## Jon's List

- [ ] **Grapple hook feel pass** `[V?/D?]` - Clarify what the grapple hook is supposed to do and tune it so the reel-in is useful at real platforming distances. Current concern: the burst may be killed by drag too quickly to reach a wall beyond a short distance. Check any existing grapple review notes if they still exist.

## High-priority architecture / cleanup

- [ ] **Make boss special-attack effects actor-generic** `[V4/D3]` - Several special-effect consumers still read `ActorActionMessage::Special` but then process boss-cluster queries and anchor effects to boss state. Refactor the effect seam so player- or non-boss-sourced special messages can spawn the same class of attacks from the message owner's position, while preserving boss-specific attacks that genuinely need boss profile state.

- [ ] **Possession follow-ups** `[V4/D3]` - Finish the possession experience: implement the Vacate body-disappears policy, redirect possessed contact-body damage toward enemies, wire NPC possession through the NPC update path, and handle the boss case once `Brain::Player` is supported for bosses.

- [ ] **Sideways / arbitrary-direction gravity for the player** `[V5/D5]` - Make the player's movement model gravity-direction-relative instead of Y-down scalar only. Non-player actors already resolve a gravity vector; player integration, jump, fall, and ground detection need the same vector-relative model for wall-walking.

## Sprites, SFX, and visible polish

- [ ] **Held-item prop sprites for newer held items** `[V2/D2]` - Generate and wire in-hand + ground-item visuals for Bomb, Blink, Grapple, Fireball, Mark/Recall, and Gravity grenade. Wire through `ItemArt` and `item_pickup::item_sprite`. Keep `regen_sprites.sh` green on a fresh clone.

- [ ] **Fireball projectile sprite + bespoke SFX** `[V2/D2]` - Give the Fireball shot a fire-looking in-flight projectile sprite and bespoke whoosh/ignite SFX so it no longer reads as a laser.

- [ ] **Mark/Recall beacon sprite** `[V2/D2]` - Add a persistent beacon/glyph sprite at the `mark_recall::PlayerMark` position so the player can see the recall point.

- [ ] **FSM + T-Rex roar/scream telegraph SFX and facing polish** `[V4/D3]` - Add bespoke roar/scream telegraph SFX for the Flying Spaghetti Monster and T-Rex bosses, replacing generic placeholder cries. Check whether FSM needs `authored_faces_left` so it does not face away in-game.

- [ ] **Shark without rider attack-pattern pass** `[V?/D?]` - Make the unridden shark feel wilder than the pirate-controlled version. It should move both vertically and horizontally, have a charge attack, and explode if it hits a wall with too much force.

- [ ] **Goblin animation sprite expansion** `[V?/D?]` - Add enough goblin animation sprites to support their different actions as full smash-capable characters.

- [ ] **Player animation sprite expansion** `[V?/D?]` - Add more player animation sprites for action coverage. Defer this in autonomous mode unless explicitly prioritized.

- [ ] **Axe held-attack replacement** `[V?/D?]` - When the player holds the axe, replace the normal attack with an axe swing.

- [ ] **Per-boss scream sprite animation + quiet cry SFX** `[V?/D?]` - Add a per-boss visual scream animation and bespoke quiet cry SFX.

- [ ] **Boss transition readability** `[V?/D?]` - Make boss transitions more noticeable and longer-lived. Add a visual sprite for scream lines emitting from the boss; better sprites can polish it later.

- [ ] **Moving-platform ledge-grab collision bug** `[V?/D?]` - If a moving platform carries a ledge-grabbed player into a wall, the player should get hit / fall off instead of being pushed through the wall.

- [ ] **Moving-platform ledge-grab follow motion** `[V?/D?]` - A player ledge-grabbing a moving platform should move with the platform or be knocked off if the movement would push them through heavy collision.

- [ ] **Silksong-level input buffering** `[V?/D?]` - Add robust input buffering for platforming and combat actions.

- [ ] **GNU-ton sprite resolution pass** `[V?/D?]` - Increase GNU-ton's sprite resolution so it does not render pixelated.

- [ ] **Better door sprites + variable door sizes** `[V3/D3]` - Generate door frame variants keyed off `LoadingZone.size` so small, standard, tall, wide, and double doors render with appropriate art and alignment.

- [ ] **Wall-clipping bugs in the intro** `[V4/D4]` - Investigate and fix ongoing wall-clipping problems in the intro. Likely areas: high-speed/sub-pixel collision drift, thin-wall corner cases, and trace-replay-driven lock-wall debt. Cross-link with `docs/planning/tech-debt-log.md`.

- [ ] **Contextual button-label follow-ups** `[V2/D2]` - Wire gamepad-kind detection once the Bevy gamepad API is verified, and add regression coverage so HUD glyphs and gameplay input resolvers cannot drift.

- [ ] **Hazard ECS cluster conversion** `[V2/D3]` - Decompose `HazardRuntime` into ECS cluster components. Current path: `crates/ambition_gameplay_core/src/combat/hazard_runtime.rs`. Follow the boss pattern with `HazardKinematics` / `HazardConfig`, view helpers, and updated hazard systems.

## S - Active blockers and high-signal defects

- [ ] **Goblin encounter music transition still sounds like a section swap** `[V4/D3]` - Diagnose the audible seam first: compare generated and installed OGGs, run `audit_cue_balance.py`, inspect logs for gain behavior, then decide whether the fix is score arrangement, mastering, stem balance, or runtime fade policy. Related follow-ups: level report, live gain HUD, equal-power crossfade, and per-stem mastering.

## A - Sandbox expressiveness and mechanics

### Movement, traversal, and collision

- [ ] **Stitched / loading-zone-free room transitions** `[V4/D4]` - Prefer Gridvania-style side-scrolling exits and large connected spaces over door-heavy room hops. Make one robust prototype where camera, collision, and transition safety work across adjacent rooms without a door load. Decide per case whether the better authoring answer is stitched rooms or one large LDtk level.

### Combat, abilities, and interactions

- [ ] **Bubble shield dodge / roll extension** `[V3/D3]` - Bubble + Down should dodge; Bubble + direction should roll. Define resource cost, invulnerability frames, collision safety, and whether roll is a locomotion state or gameplay effect.

- [ ] **Structural: per-frame wholesale ability reset clobbers transient mods** `[V3/D3]` - `sync_live_ability_edits_clusters` resets `PlayerAbilities` from the editable loadout every frame. Add a base-abilities + transient-modifiers layer so temporary suppressions/debuffs/status effects compose cleanly instead of being clobbered after one frame.

- [ ] **Texture-accurate portal piece clipping** `[V3/D3]` - `sync_portal_body_pieces` currently clips with an opaque mask box. Clip the actual character atlas frame instead, accounting for feet anchors and render-vs-collision scale. Do the same for non-player actors.

- [ ] **Portalize hurt/hitboxes + dedup by logical owner** `[V4/D4]` - Route damage, contact, pogo, pickup, and sensor volumes through `compute_body_pieces`. A body half-through a portal should be hittable on visible pieces only; a hit overlapping both pieces of one owner should count once.

- [ ] **Wire `raycast_through_portals` into gameplay rays** `[V3/D3]` - Use the portal-aware raycast for grapple, aim assist, line-of-sight, and beams. Pass placed portals through to call sites currently using plain world raycasts.

- [ ] **Sized-projectile gradual transit + per-projectile policies** `[V3/D4]` - Give sized projectiles and thrown items the same AABB straddle/transit model as bodies. Add per-projectile policy: pass-through, die, bounce, stick, or ignore.

- [ ] **Attached-object chart consistency** `[V3/D3]` - Held items, weapons, riders, and weak points should use their owner's portal chart. A sword swing from a half-through actor should originate from the visible hand piece, not the authoritative center.

- [ ] **AI perception of portal pieces** `[V3/D4]` - Progress from targeting authoritative centers to perceiving/attacking visible exit-side pieces, and eventually treating linked portals as pathfinding graph edges.

- [ ] **Portal placement validity + crushing rules** `[V3/D4]` - Define and enforce portal surface validity, aperture overlap rules, unstable overlapping portals, and forced-into-blocked-geometry behavior.

- [ ] **Chart-aware portal VFX + camera polish** `[V2/D3]` - Add entry/exit ripples, textured portal sprites, optional wipe/dissolve transitions, and camera support for portal traversal.

- [ ] **Moving-platform / dynamic-geometry portals** `[V2/D5]` - Support portals attached to moving frames and apply moving-frame velocity correction.

- [ ] **Shop overlay and economy polish** `[V3/D3]` - Add a dedicated shop overlay UI if preferred over dialogue-list shopping. Expand stock, provenance/ethical-currency framing, and balance.

- [ ] **More enemy varieties across size and aggression bands** `[V4/D4]` - Fill missing low/medium/high combinations only when they create distinct encounters. Validate HP, aggro radius, damage, and LDtk brain IDs.

- [ ] **Actor aggression / NPC-enemy distinction polish** `[V4/D3]` - Check current actor, faction, NPC runtime, hostility conversion, and enemy runtime paths. Reduce special cases where possible so enemy-vs-NPC distinction is primarily aggressiveness/behavior, not separate conceptual species.

### Test rooms and progression laboratories

- [ ] **Save-point lab + persisted-switch test room** `[V4/D3]` - Add a save-point interactable distinct from regular switches, a reset-switches sub-room, and a broader test-state schema covering boss defeated, mob room cleared, and save restoration.

- [ ] **Additional body-mode traversal rooms** `[V3/D3]` - Author rooms that prove crouch, crawl, slide, morph, stand-up rejection, and compact traversal under real LDtk collision.

- [ ] **Tutorial refresher / quest reminder rule** `[V3/D3]` - Add a durable UI/UX plan and small sandbox proof so the game never permanently strands a player without a way to review tutorial controls, current quests, or what they were doing.

## B - Audio, generated assets, and authoring tools

> **Sprite convention:** adding a sprite is its own task, separate from the feature that uses it. Generated placeholder sprites are the shipping art until an artist pass, so make them presentable.

- [ ] **Boss music binding extension** `[V4/D3]` - Extend adaptive music cue bindings beyond mob encounters so boss phases such as Intro, Phase1, Transition, Phase2, Stagger, and Enrage can resolve to cue states.

- [ ] **In-engine music gain HUD** `[V3/D3]` - Add a dev HUD showing live per-bank / per-slot gains and post-master dB while music plays, so mix transitions can be validated by ear and by numbers.

- [ ] **Equal-power crossfade in runtime gain smoothing** `[V2/D2]` - Investigate a phase-driven sin/cos equal-power transition when two banks crossfade. Validate by ear and with level-report output.

- [ ] **Renderer: master per-stem outputs** `[V3/D3]` - If stem-driven runtime playback returns, make sure per-stem outputs receive enough mastering/auditing that state-level stem-gain changes are audible and balanced.

- [ ] **Generalize encounter music registration** `[V4/D3]` - Replace one-off encounter music wiring with a reusable structure for future encounters.

- [ ] **Sprite / tile wiring batch** `[V3/D3]` - Keep a visible list of assets that exist or are planned but are not wired into runtime/LDtk visuals yet. Examples: switch armed/disabled sprites, lock-wall tile, water-surface tile, ladder/vine/climbable tile, acid/lava tile, circuit/background tiles, creator/wagon/lab props, and placeholder raid-enforcer/general replacements.

- [ ] **Generated tile sprites for IntGrid layers** `[V3/D3]` - Replace colored placeholder rectangles for climbable, water, hazard, and solid layers with real tileset textures while preserving canonical LDtk/bevy_ldtk usage.

- [ ] **Textured portal sprite + in-flight-shot VFX** `[V4/D3]` - Replace the current simple portal bars/streaks with portal sprites/VFX for blue/orange portals and in-flight portal shots.

- [ ] **Puppy slug gun sprite** `[V2/D2]` - Generate a held-item prop sprite for the Puppy slug gun like the axe, javelin, and portal gun props. Keep `regen_sprites.sh` green on a fresh clone.

- [ ] **Loudness-normalize the music catalog** `[V3/D3]` - Normalize generated music cues toward a consistent target, ideally integrated LUFS if `pyloudnorm` is available. Use `tools/ambition_music_renderer/level_report.py` as the before/after measuring stick.

## C - Bosses, encounters, and story-arranged slice

- [ ] **Sandbox-side boss controller hook for `BossMovementKind`** `[V4/D3]` - Interpret `step.movement` into actual world transforms so bosses dash, orbit, reposition, or retreat instead of only firing attack verbs.

- [ ] **Boss intro sequence audit** `[V3/D2]` - Reproduce the suspected boss music / intro sequence issue before diagnosing. Check cutscene gating, encounter start, music request, and boss phase state.

- [ ] **Real central hub authoring** `[V3/D4]` - Resume story-arranged central hub content once the sandbox bar is met.

- [ ] **Basement / first-zone Gridvania authoring** `[V3/D4]` - Prefer a connected layout with side-scrolling exits and branching routes over door-heavy lab chunks.

- [ ] **Vertical intro lab maze pass** `[V4/D5]` - Explore the intro shape: vertical lab maze, side branches, factions fighting each other, the creator trying to escape with the player, and faction/boss pressure at the top. Keep as design work until mechanics and rooms support it.

- [ ] **Intro cutscene polish** `[V2/D3]` - Include a strong intro hook only if it fits the current tone.

- [ ] **Vanity-card playback + dev-menu trigger** `[V3/D3]` - Wire playback for the existing `assets/vanity_card/` art as a game-start intro sequence, and add a developer-menu item to play the vanity card on demand. Reuse the cutscene/banner presentation path in `ambition_render` if it fits.

## D - Engine, validation, and architecture

- [ ] **Restore and validate headless / RL build surfaces** `[V4/D4]` - Keep the current headless command green: `cargo run -p ambition_app --bin headless -- 120`. Separately restore the no-default / `rl_sim` feature path before advertising it as canonical.

- [ ] **Headless AI-playtesting bridge** `[V4/D4]` - Add a PyO3 binding or decide whether to adopt `bevy_rl` versus keeping the custom adapter.

- [ ] **Headless visual verification follow-ups** `[V3/D4]` - Extend the headless room-geometry renderer to overlay dynamically spawned mobs and optionally render actual sprites through a software backend for art/HUD verification.

- [ ] **Headless collision-invariant fuzz oracle** `[V4/D3]` - Add per-step invariants on top of `SandboxSim`: player AABB never fully inside Solid, never outside world bounds except through authored exits, no unexplained stuck state, ground flags agree with geometry, and no impossible teleport. Sweep all rooms via `SandboxSimOptions::with_start_room(...)`.

- [ ] **Lean headless compile after render split** `[V4/D4]` - Make the headless/RL/CI build compile without dragging unnecessary visible/render/audio/UI dependencies. Treat current `rl_sim` feature failures as part of this cleanup. Do not document no-default headless commands as stable until they compile.

- [ ] **Unified `ControlFrame` consumer seam** `[V4/D4]` - Move menu navigation, inventory, cutscene dismissal, and other consumers away from raw `ActionState<SandboxAction>` where practical so keyboard, controller, and touch all pass through the same mode-gated abstraction.

- [ ] **Move dev hotkeys into settings/menu surfaces** `[V3/D3]` - Keep hotkeys for developer speed, but every persistent toggle should have a canonical non-hotkey place in settings or dev UI.

- [ ] **Shared settings/menu intermediate representation** `[V3/D3]` - Create one `SettingsMenuModel` IR for categories and options, then render both cube and Bevy-UI settings menus from it so settings cannot drift between frontends.

- [ ] **Promote `KinematicPath` to typed components + index** `[V2/D3]` - Moving platforms have a working path contract, but future path-authored hazards/platforms may need a generic typed runtime index.

- [ ] **Continue Bevy ECS slice extraction** `[V4/D4]` - Shrink procedural orchestration and ad-hoc `SystemParam` bundles only with a focused validation plan.

- [ ] **Time-domain consistency sweep** `[V4/D2]` - Route gameplay timers and state-machine progression through `WorldTime::sim_dt()` where they should slow/freeze with sim time. Leave genuinely wall-clock systems alone. Also audit direct `clock.time_scale` mutation versus `ClockScaleRequest` routing.

- [ ] **Move boss special-attack tunings into `boss_profiles.ron`** `[V3/D2]` - Move special-attack numeric tunings out of Rust constants in `crates/ambition_content/src/bosses/specials/` and into authored boss data. Keep the Rust code as the behavior implementation, but make existing special tuning changes data-only.

- [ ] **Boss-encounter plugin seam** `[V4/D4]` - Make adding a boss/encounter require as little core-code editing as possible. Move encounter-level metadata into data where appropriate, and give bespoke set-pieces stable spawn, damage/kill-condition, dialog, and encounter hooks so core systems do not grow one branch per encounter.

- [ ] **Boss HP single source of truth** `[V3/D3]` - Collapse boss HP to a single writable source, preferably `BossEncounterState.hp`, with ECS boss status as a one-directional read model. Remove fallback paths where tests and live game disagree about authority.

- [ ] **Kill per-entity String allocations in hot paths** `[V2/D2]` - Finish the hot-path allocation audit. Known remaining candidate: `features/ecs/view_index.rs` rebuilding string keys every frame. Re-check damage ignore-target and actor identity paths before editing; some earlier allocation issues may already be fixed.

- [ ] **Dead-code & comment hygiene pass** `[V2/D1]` - Remove broad `#![allow(dead_code)]` where possible, narrow or delete genuinely unused helpers, and reword stale `OVERNIGHT-TODO` comments that describe completed migration steps. Keep genuinely open markers as-is.

- [ ] **Documentation health remains green** `[V3/D1]` - Run doc checks after doc moves, rewrites, or archive changes.

```bash
python scripts/check_doc_links.py
python scripts/generate_agent_index.py
python scripts/check_agent_kb.py
```

## Authoring tools

- [ ] **LDtk `paths describe` spatial tool** `[V3/D3]` - Add a read-only `paths describe --level X` command that summarizes reachable exits from spawn and traversal kind. Avoid misleading heuristics; platformer reachability over jumps and one-ways needs a careful model.

- [ ] **`connect_to` reciprocal Door zones should snap to destination floor by default** `[V2/D2]` - In `ambition_ldtk_tools`, make reciprocal `LoadingZone` insertion default to floor snapping for `activation: Door`, with an explicit opt-out. Update tests that assume raw y-placement.

## Proposed / agent drop-zone

Agents may append new ideas here freely. Jon promotes them into the accepted sections above or moves them to `TODO-drafts.md` / brainstorm docs.

> **Placement rule for agents:** when Jon says "add this to TODO.md", the item goes in the curated sections above. This drop-zone is only for unreviewed ideas an agent surfaces on its own during an autonomous run.

## Closed-work policy

Do not add completed work here just to preserve history. Put durable history in `dev/journals/`, `docs/current/`, focused system/concept docs, or `docs/archive/`. When closing an item, update the source doc too if it tracks the same state.

