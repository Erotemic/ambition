# Maintainer decisions

This is the compact record of decisions Jon made explicitly. Each row gives the
decision, its date and confidence, and only the consequence needed to apply it.
Git history keeps the rationale and the investigation.

Confidence means:

- **High**: proceed on this basis. Reopen only with new concrete evidence.
- **Medium**: current direction. Implementation or play may refine it.
- **Low**: tentative preference or deliberately deferred choice.

Open questions belong in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

## Decision ledger

| Date | Decision | Confidence |
|---|---|---:|
| 2026-07-16 | Perform the identified content evictions. | High |
| 2026-07-16 | Extract the reusable programmatic simulation surface as `ambition_sim_harness`. | High |
| 2026-07-16 | Extract the platformer-provider lifecycle from the `ambition_platformer2d` facade and consolidate the repeated provider protocol. | High |
| 2026-07-16 | Keep cutscenes and encounters as separate domain systems. | High |
| 2026-07-16 | Keep provider registration explicit in the host composition root. | High |
| 2026-07-16 | Defer any boss crate carve until boss behavior converges onto the canonical moveset/action path. | High |
| 2026-07-16 | Reject the proposed named-content scanner and stop adding poison-test ceremony by default. | High |
| 2026-07-16 | Keep the compiler term **lowering** for authored world IR becoming live ECS state. | High |
| 2026-07-16 | Repository-wide knowledge-base hygiene checks are CI/maintainer tools, not routine local validation. | High |
| 2026-07-16 | Preserve historical journals as historical records during documentation cleanup. | High |
| 2026-07-16 | A full rename of `ambition_platformer2d_actor_monolith/src/features/` may be worthwhile, but the name `sim` is not settled and the work is low priority. | Low |
| 2026-07-29 | Label occlusion and transition nameplates are low priority. Do not touch them until combat is good. | Low |
| 2026-07-29 | Do not gate or redesign `Interact` yet. It needs a design discussion. | High |
| 2026-07-29 | Build the portrait target → art resolver. | High |
| 2026-07-29 | Replace the invented lab dummy enemies with real ones that already exist. | High |
| 2026-07-29 | DI matters: Smash-style physics is wanted in Ambition itself, not only in versus. | High |
| 2026-07-29 | Generic versus ends on health. Smash Siblings is a separate, specified mode. | High |
| 2026-07-29 | Smash Siblings HUD: per-character portrait, stock icons, percentage. No score. | High |
| 2026-07-30 | Defer the `bevy_ggrs` patch-table leak. Revisit when upstream publishes the `GgrsFrameTiming` accessor on crates.io. | High |
| 2026-07-31 | Each robot version is a different character, on purpose. | High |
| 2026-08-06 | Input filtering is keyed to the pad. Bindings stay machine-wide. | High |
| 2026-08-06 | Dialogue claims only the talker's input by default. It does not stop the world. | High |
| 2026-08-06 | A conversation is sustained, not modal: it breaks on damage or separation, holds its participants if they can be held, and barks when it breaks. | High |
| 2026-08-06 | Any seat may pause, and the seat that paused drives the menu. | High |
| 2026-08-08 | A game may compose this engine without a given capability. Capabilities are optional. | High |
| 2026-08-08 | Take the bbox route: size and crop the character quad from `body_pixel_bbox`, not the padded frame. The target figure height is an authored number with a per-character override, not a compiled constant. | High |
| 2026-08-08 | Run a much smaller suite on a docs-only change. Bias toward running fewer tests. | High |
| 2026-08-08 | "Declared no abilities" does not mean "inherit the dev kit". Character capabilities are authored, explicit and composable. | High |
| 2026-08-08 | The Perfect Cellular Automaton can fly. | High |
| 2026-08-08 | Sanic does not have blink. | High |
| 2026-08-08 | Hitstun needs a redesign in its own session, not a tweak. | High |
| 2026-08-08 | The three named robot heavies are deprioritised, not decided. | Low |
| 2026-08-08 | The rollback wire format is unstable by policy. The latest build is compatible with itself and nothing else. Do not ask again. | High |
| 2026-08-08 | Portal orientation is authored per portal, in LDtk. No global setting. Rotation is the default; scale inversion is opt-in and may be unsupported per game. | High |
| 2026-08-08 | It is fine that 1-1's first `?`-block drops its wand into a pit. | Low |
| 2026-08-08 | GNU-ton should be hittable during its special. Its authoring is stale in general; do deep work only if the staleness is a symptom of an architecture problem. | High |
| 2026-08-08 | The world keeps living while you die, but it must know you are dying and stop attacking you. | High |
| 2026-08-08 | A crawler's collision volume orients with its attachment. It is the same concept as a body under different gravity. | High |
| 2026-08-08 | Make most of the Hall cast dormant. | Medium |
| 2026-08-08 | Defer all GNU-ton work. | Low |
| 2026-08-08 | Mark staleness where you find it, with evidence. Sweep later, separately. | High |
| 2026-08-08 | (b) Fix the one site; leave the authored-id rule to prose. Revisit the refactor only if it becomes important. | Medium |
| 2026-08-08 | Operating note: Jon has no design intuitions about rollback and delegates that design to agents. | High |
| 2026-08-10 | A character is a reusable authored template, not a singleton person. The enemy-archetype system is deleted. (D48, D73) | High |
| 2026-08-13 | There is no separate "can fight" character property. A character can fight to the extent that its body has abilities that produce combat effects. | High |
| 2026-08-13 | Carl Stargan does not fly. He fights because his body has abilities, not because of a fighter flag. (D96 5) | High |
| 2026-08-13 | Skitters are Puppy Slug: `SmallSkitter` and `under_town_skitter` are authored as `npc_puppy_slug`. <!-- cite-ok: a character id in the catalog data, not a Rust item --> | High |
| 2026-08-13 | `large_brute` becomes a real authored reusable character, a Goblin Brute, with its own Python sprite generator. | High |
| 2026-08-13 | The dive-drill and its anonymous `Target` may be deleted. For disposable AI-authored content, prefer deletion to architecture. | High |
| 2026-08-13 | Unauthored body health is tuning, not a product decision. Pick reasonable numbers and author them. (D96 7, D96 8) | High |
| 2026-08-13 | Camera orientation is a per-view observer policy. Keep the world-fixed/external-observer camera and add an optional controlled-body/view-subject-relative mode. | High |
| 2026-08-17 | Keep the per-body hitlag freeze. The old "do not reintroduce a per-body zero-dt" prohibition is superseded. (D114) | High |
| 2026-08-17 | Keep the per-turn gate small: `cargo test --workspace --lib` stays out of `gate_suite.py` on purpose. "Gate" means an executable gate; the pre-push checklist is a separate validation tier. (D160) | High |
| 2026-08-17 | Accept the CPU showcase's current pacing. Do not retune stock count, knockback or damage. (D128) | High |
| 2026-08-17 | Replace the `central_hub_main` developer-note sign; do not delete it. The hub wants an orientation sign. | High |
| 2026-08-17 | Inventory is Morrowind-style: the occurrence owns, and every inventory entry carries a count (usually 1). | Medium |
| 2026-08-17 | A line count is a proxy. Decompose where it makes sense, and stop making the monolith worse. | High |
| 2026-08-17 | Split-screen layout is adaptive with hysteresis. | Medium |
| 2026-08-17 | Sprite sizing: give every scale a shared unit first, then revisit the quad-from-bbox route. | High |
| 2026-08-17 | Capability progression splits by verb: physical verbs are body-owned, knowledge is participant-owned. | Medium |
| 2026-08-17 | A model-backed character is a remote player, not a brain in the tick. The question is deferred. | Medium |
| 2026-08-17 | Fix the clipped sprite sheets case by case, driven by the draw-time warning. | High |
| 2026-08-17 | A dropped held weapon persists or not per item, by authoring, not by a global rule. | Medium |
| 2026-08-17 | One world unit is one base-grid pixel: 16 units to a tile. | High |
| 2026-08-17 | A declared character height is a contract: art scales to it, and a tight tolerance warns when the scale drifts. | High |
| 2026-08-17 | Landmarks are optional slots on a character package: authored when useful, never required. | Medium |
| 2026-08-17 | Promote `engine/character-authoring-package.md` to a live ledger row, with canonical height as its first slice. | High |
| 2026-08-17 | Giant and multi-part bodies declare canonical height by the same rule. One vocabulary, no exemption list. | High |
| 2026-08-17 | Camera shake stays constant in the world; the field is renamed to say so. | High |
| 2026-08-17 | A projectile respects the authored hurt volume, the same geometry melee uses. | High |
| 2026-08-17 | Defer the per-creature ability absence list until the cast is bigger. | Medium |
| 2026-08-18 | Minimize poison tests. Poison only below about 60% certainty that the guard bites. | High |
| 2026-08-18 | Authoring a character's size must be consistent and trivial to tune: one number, and the geometry follows. | High |
| 2026-08-18 | Mary-O is one brick tall small (16) and two grown (32). Her small art is half the grown height at the same width. | High |
| 2026-08-18 | The tall sprite may be visually wider. The collision width stays identical for big and small. | High |
| 2026-08-19 | D166 CPU-grab work does the policy half first: the fighter capability owns what a hold is worth. The mechanical fixes (a start-gate on the option list, tighter grab spacing) wait behind it. | High |
| 2026-08-19 | `body.action_buffer` stays: a registered rollback row with no writer, documented as declared-but-unfed. | Low |
| 2026-08-19 | AI Slop honours its authored size constant (28 × 18.2). | Medium |
| 2026-08-19 | The `.loop` cue derivation trap stays recorded. No exemption list, no fallback. | Low |
| 2026-08-19 | The eight authoring-structure tests in `test_oiler_svg_rig.py` stay. Oiler's SVG is settled. | Low |
| 2026-08-19 | D162's three sheet-manifest collisions go to Jon pair by pair, written up, not resolved by rule. | Medium |
| 2026-08-20 | Avoid-pushout is about portals, not bodies. Jostle is allowed, but it may never be a mandatory part of the movement kernel. | High |
| 2026-08-22 | Rename the blast zone out of every world: `World.edges: WorldEdgeMargins { fall, side, rise }`, Rust and LDtk keys in one change. | High |
| 2026-08-22 | The baked sheet registry keys by file root. A renderer target string may not be a durable engine identity. | High |
| 2026-08-22 | `SeatRawFrames` stays raw. Build the stage model; world-dependent semantics land after the boundary, not before it. | High |
| 2026-08-22 | Sweep the crates a carve touches. Do not widen the gate to `--workspace --all-targets`. | High |
| 2026-08-22 | The layout tool owns a level's position, and ownership follows the layout mode. | High |
| 2026-08-22 | Height owns world size. Source-art density is a separate contract. The 1.0 warning is removed, not replaced. | High |
| 2026-08-22 | A hit's art follows both the victim's material and the blow's strength. | High |
| 2026-08-22 | Impact hitstop is a bounded match-level request from the connect. It is combat presentation, not a slot-0 affordance. | High |
| 2026-08-22 | A boss's hoard is per-boss eventually. For the demo it is currency. | High |
| 2026-08-22 | `DebugLabel` is debug, and it keeps shipping: the whole world is scaffold. | High |
| 2026-08-22 | Proximity-gate edge-exit labels. Label visibility is a game-selectable policy, not a fixed rule. | High |
| 2026-08-22 | Disable rust-analyzer. No second target directory. | High |
| 2026-08-22 | Correct the level-1 CPU for feel: the easiest rung is bad at fighting, not self-destructive. | Medium |
| 2026-08-22 | Mary-O keeps the 56 px shared collision width, and her short-form crown rises about 6 px. | High |
| 2026-08-22 | Fix Mary-O's walk dip with a pose field that lowers the torso without moving `foot_y`. | High |
| 2026-08-22 | The Mary-O restart report is closed and believed resolved. | Medium |
| 2026-08-22 | Advance the `dev/ambition_dev_measurements` pointer periodically. The cadence does not matter. | Low |
| 2026-09-10 | A published collision surface participates in projectile collision. A destructible's surface plus its hurt volume are one compound contact: damage once and apply the surface response. See the Q96 consequences below. (Q96) | High |
| 2026-09-12 | Take the stronger last-good-world guarantee. A candidate scene is built and validated off to the side and published only on success; a rejected candidate leaves the running world untouched. A10 stays bounded to typed construction recipes, constrained candidate construction, relationship/resource validation, one controlled publication boundary and explicit retirement of the old world. It is not transactional rollback of arbitrary ECS commands. (Q113) | High |
| 2026-09-12 | A planner that scores move A while the executor performs move B violates the action model. The truthful attack kit is unconditional, and re-pricing CPU matchups is the expected consequence. Which utility, run and dash-attack parameters give the wanted fighter quality is tuning. (Q117) | High |
| 2026-09-13 | A content publication may stop and rebase a rollback timeline this host owns; that is not a refusal. A pending generation is admitted only against a timeline that is healthy and locally rebasable. `External`- and `Caller`-owned timelines are never replaced unilaterally, and a recorded divergence refuses. The admission is a transaction-lifetime lease re-asked at activation, and the stop-and-rebase happens in the same exclusive step as the publication. Known gap: ordering alone narrows the interval between the publication breaker and `commit_content_generation`. Do not make the commit fallible. Tracked by `a_boundary_that_closes_after_the_breaker_still_publishes` (`game/ambition_content/src/reload_tests.rs`). (Q118) | High |
| 2026-09-13 | A developer mechanical edit is a proposal until the timeline's owner admits it, and the decision happens before the advance. No live timeline: publish. A timeline this host owns: stop it, then publish. `External`/`Caller`-owned or diverged: refuse (the authoritative value does not move) and retain the proposal so it publishes when ownership returns. Editor panels mirror the authoritative mechanical values and are never the authority. Known gap: no composition runs a real GGRS canary with the developer-tools chain; `the_canary_rig_has_no_developer_edit_road_to_admit` (`game/ambition_app/tests/developer_edits_under_rollback.rs`) fails when one does. (Q120) | High |
| 2026-09-13 | One authority, `ambition_platformer2d::rollback::mechanical_mutation_boundary`, answers whether mechanical mutation is legal around rollback. Callers differ only in transaction lifetime: Q120 asks once, Q118 holds a lease. (Q118, Q120) | High |
| 2026-09-13 | Withdrawn as not a maintainer choice: a death-reset restores each object in custody by which side of the checkpoint its acquisition fell on (temporal, per object), not by item kind. Pinned by `a_death_returns_what_was_not_banked_and_keeps_what_was` (`game/ambition_app/tests/death_restores_the_checkpoint.rs`). (Q124) | High |
| 2026-09-16 | GGRS does not start before the durable restore finishes. `maintain_local_session` refuses to create a rollback session while `durable_hydration_is_pending` (the save is unapplied and this world has the body that can apply it). Do not gate on the bare `SaveRestored` flag: a Smash match never raises it. Held by `a_conversation_on_the_first_tick_of_a_session_is_counted_exactly_once` (`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`). (Q135) | High |
| 2026-09-19 | Priority adjustment: scripts are not the product. Static review and code inspection are enough for many architecture invariants. Do not build a parser, witness generator, poison suite or permanent guard for every ADR statement. A script earns its place by valuable observability or by catching a repeatedly demonstrated failure class. Effort goes to: better Rust architecture; runtime and game observability; combat balance and polish tools; edit-to-play iteration; world/content authoring and open-world layout; and rollback correctness where it affects real mechanics. ⛔ Netplay is not a goal for this year: do not spend large effort on speculative P2P-only problems. Do not spend heavy engineering on preserving a temporary affordance (the removed `K` clone feature is the example). | High |
| 2026-09-19 | Two explicit composition modes, and the game is essentially identical in both: launch directly or run inside the shell. Under the shell the game can return to it; the direct build shows the same "return to shell" item, disabled. Shell presence must not change simulation, mechanics, capabilities, registries, content or game policy. Production shell sessions use the prepared/session lifecycle; explicit direct, headless and test compositions may hold scoped fixture/direct-entry authority. No anonymous App-global fallback state returns. A capability that authored production content requires and the composition lacks refuses that content or its admission; reduced tools and tests may omit capabilities explicitly. Implement this; do not census hypothetical composition variants. (Q146, Q144, Q108, Q106, Q100, Q97) | High |
| 2026-09-19 | An ability contact is independent by default. It credits a move's `Connected`/contact condition only when it carries explicit provenance naming the launching move occurrence. `None` does not mean "the move playing now", and `Some(old_instance)` never credits a different current occurrence. A mechanic meant to count toward its launching move threads that occurrence explicitly. (Q101) | High |
| 2026-09-19 | No generic one-dimensional engine difficulty architecture, and the topic is deprioritised. Difficulty is game policy expressed as presets. Smash-like modes may have participant handicaps and CPU brain levels, which are separate concepts. Keep participant accessibility/assist/handicap state distinct from game/match policy. Get Normal play excellent first. (Q127) | High |
| 2026-09-19 | For the Smash-like game, ordinary scaling throws participate in rage, and set-knockback keeps set-knockback semantics (as in Ultimate). This is game-level combat policy that the engine must be able to express, not an engine law. A changed CPU-duel benchmark is balance evidence, not a reason to keep the inconsistency. (Q133) | High |
| 2026-09-19 | Do not remove gravity switching. LDtk-authored gravity switches and developer gravity controls stay. The unreachable `GravityFlipSwitch` plate goes. Authored switches, developer controls and future gravity mechanics share one lower-level mechanism for ambient gravity changes; a future gravity plate is an input to it. (Q137) <!-- cite-ok: the plate this ruling removed --> | High |
| 2026-09-19 | One frame of stale UI is acceptable. UI does not need rollback because it displays rollback-owned state. UI actions that affect the simulation use deterministic simulation ingress. Do not add duplicate authoritative inventory state or optimistic-reconciliation machinery to hide one frame. (Q140) | High |
| 2026-09-19 | A cutscene fade carries an explicit authored start and target alpha. Do not rely on an "all cutscenes start black" convention. Cutscenes are low priority: make the semantics sane with minimal work. (Q143) | Medium |
| 2026-09-19 | Not blocked; do not wait for a further ruling. **Q132**: see the session-identity consequences below. **Q136**: choose ingress by semantic ownership; current rollback correctness is engineering. **Q138**: an invalidated harness refuses or fails; it never produces frozen observations. **Q139**: do not grow architecture only to satisfy a static presentation-writer census. **Q122**: the mechanical identity fingerprints mechanical facts, not explanatory prose. **Q104**: content-authored movesets are the long-term authority; duplicate Rust tables are migration scaffolding. **Q110**: mechanical registry changes use explicit lifecycle/replacement semantics; no universal silent overwrite. **Q145**: derive room-transition ordering from actual transaction semantics. **Q141**: durability is per item and authored; a runtime-spawned item may be durable when authored so. | High |
| 2026-09-24 | AP19: a body's default abilities are the content provider's declaration. Preparation resolves every character's `abilities` to authored-or-declared, so the blueprint carries a set, not an `Option`, and `ambition_body_seed` holds no default. | High |
| 2026-09-24 | AP12: the melee cooldown is armed and its pace is authored. The move road arms `BodyMelee::cooldown` from the profile's `attack_cooldown_s` (seconds); unauthored means no floor, and the engine holds no pacing number. Accepted behaviour change: AI swings get slower. | High |
| 2026-09-24 | W004: implement the lunge step. `LungeSpec::step_px` is carried into the attack move as windup self-motion. The velocity law is an engineering choice that needs play-tuning. | High |
| 2026-09-24 | W026: the provoked policy is ruleset/content-owned. An explicit ruleset or content policy states what a provoked actor becomes; the engine has no default answer. | High |
| 2026-10-01 | The potato tier stays at 1/16 linear scale, for characters too. It is meant to be humorously small while it keeps the gist of the sprite. Readability against `quarter` or a low-quality threshold is not an acceptance criterion. No fallback to `0_25x`. (Q69) | High |
| 2026-10-01 | Durable whereabouts are separate from the authored population. A persistent open-world character's current whereabouts are durable world state; its authored room is not a respawn tether. A respawning population occurrence stays where it is carried while it lives; when the population respawns, the replacement appears in its authored room. Wanting to go home is character policy, not an engine rule. Authored home ≠ durable whereabouts ≠ live room occurrence. (Q38) | High |
| 2026-10-01 | A rewind that un-defeats a boss also un-grants its reward. Boss defeat and the consequences it caused rewind together. Where the engine does not yet enforce this, it is known implementation debt, not an undecided behaviour. (Q51) | High |
| 2026-10-01 | Room replay uses the same rule for every boss family: if a replay makes the boss undefeated again, the consequences of its defeat created after that point are undone. The generic boss-progress road enforces it. (Q56) | High |
| 2026-10-01 | A portal is an aperture, and held items, riders and mounts, attached objects, projectiles and other composites obey it. No "hide, drop or teleport the whole object" shortcut for complicated configurations. This is the north star; not every clipping edge case must be solved at once. (Q64) | High |
| 2026-10-01 | Pause, map and inventory may open during dialogue; the conversation stays live underneath without navigation input. Map and inventory are mutually exclusive primary overlays. Input ownership needs explicit layering and focus between dialogue and an overlay. (Q75) | High |
| 2026-10-01 | A unique capability item may behave as an entitlement for now: dropping the world token need not revoke the capability. The demo inventory is a demonstration, not the final item model; build no significant architecture to make it physically rigorous. When real game development begins, the game distinguishes an unlock/entitlement from a physical item occurrence with custody and location. (Q45) | High |
| 2026-10-01 | Boss support as an independent capability is engineering, not a ruling. A game or profile that does not request bosses must not inherit boss machinery from historical topology. Extract the subsystem if it is mature enough, repair concrete blockers where that improves the engine, and do not force an artificial extraction. (Q48) | High |
| 2026-10-01 | A body/capability gate is evaluated per actor. A phase wall that requires worn Phase Boots is intangible for the actor who wears them and solid for one who does not. Collision evaluates the traversing actor's body/capability state; no actor mutates one shared wall for the party. (Q54) | High |
| 2026-10-01 | Conflicting durable switch commands in one step resolve deterministically through one mutation authority; system order must not choose the winner. A priority with a deterministic tie-break is acceptable; a clean semantic merge is preferred. With no authored collision today, narrow known policy debt is acceptable. (Q61) | Medium |
| 2026-10-01 | Opaque installation is prohibited, not the Bevy `Plugin` type. A capability may install its private systems and resources through a capability-owned plugin when the host/profile requests the capability explicitly, the plugin uses documented public schedule milestones, and the composition root controls whether the capability exists and the order between published boundaries. A plugin that silently installs unrelated capabilities or hides scheduling dependencies is not acceptable. Remove wording that reads "no plugin" as a ban on the `Plugin` trait. (Q73) | High |
| 2026-10-01 | `rm -rf` under a bound `target/` is permitted. The safety invariant is the target bind mount, not the path: verify it with `scripts/setup/target_bindmount.sh --status`, then delete `target/<something>` when appropriate. When the bind is missing, do not delete target contents. Prose and checks that encode the old blanket ban are to be updated. (Q77) | High |
| 2026-10-01 | Visual and mechanical geometry come from one authored source. In order of preference: (1) shared authored limb/weapon/body geometry, so disagreement is structurally impossible; (2) an explicit gameplay adjustment (hitbox inflation/extension) where disagreement is intended; (3) a small measured tolerance for rasterization, scaling and quantization effects, set from the pipeline's measured representational error. (Q80) | High |
| 2026-10-01 | The moveset owns mechanical attack timing. Startup, active interval, recovery and cancel windows come from the moveset/semantic move timeline, which feeds mechanics and then presentation. Sprite metadata is never a mechanical timing authority. Art may annotate active frames for authoring and checks, derived from or validated against the moveset; tooling shows drift instead of changing mechanics. (Q107) | High |
| 2026-10-01 | Per-move hitbox inflation is tuning, not a ruling. Bone-derived geometry gives the natural shape; a move may set `inflate`/`extend` where feel needs reach. No roster-wide value: inspect, play and tune each move, and keep zero where the derived geometry is right. (Q115) | High |
| 2026-10-03 | Refuse to wear or instantiate a character that is not in the prepared generation. No fallback to engine defaults, no read of the current App-global catalog, no other road around preparation. Invariant: a character admitted into simulation was prepared for that generation. Dynamic admission of a new character, if gameplay ever needs it, goes through an explicit preparation/generation transition, not an escape hatch. The point is to prevent N/N+1 authority mixing. (Q103) | High |
| 2026-10-03 | `LiveRoomInstance` stays a separate deterministic scope beside `SimId`; the room occurrence is not part of canonical `SimId` semantics. `SimId` is the authored/canonical identity; `(LiveRoomInstance, SimId)` is one live occurrence. Do not collapse them because the product allows one live occurrence of an authored room for now: authored identity must stay stable while live occurrences come and go, for persistence and reconstruction. (Q109) | High |
| 2026-10-03 | Shared durable world state is part of peer-deterministic state. Peers must not disagree on semantic durable facts: a boss defeated, persistent world mechanisms, shared inventory/world rewards where applicable, persistent quest/world consequences, and other shared facts that affect later simulation. This does not make the save file's byte representation the permanent network checksum format: the eventual peer protocol compares the canonical semantic durable state, not serialization details. The exact netplay representation waits until peer networking is active work. (Q129) | High |
| 2026-10-03 | Dialogue visit/history counts are per participant/player, not shared world state: "Alice has spoken to NPC X three times, Bob never" is valid. Shared world facts (NPC X is dead, the bridge is destroyed, the boss is defeated) are different. Per-player dialogue history belongs with the future reactive-character / relationship / memory model. (Q134) | High |
| 2026-10-03 | Wound persistence across room retirement is an opt-in actor policy/capability, not universal behaviour. An ordinary respawning enemy may be rebuilt fresh when its room retires and is recreated. An actor whose continuity matters (a named character, a persistent rival, an important NPC, another explicitly persistent actor) may opt in to keep its wounds/combat state. Do not make every transient combat detail durable to serve the persistent cases. (Q149) | High |
| 2026-10-03 | HUD state is per participant: each participant/view gets its own HUD, also when views merge onto one shared screen (a joined camera does not mean one player HUD). There is no global "primary-player HUD" arbitration. Music: remote play has no conflict (each listener has its own mix). For local play's one shared stream: (1) find each local participant's music candidate; (2) higher authored priority wins, also for a secondary participant; (3) on equal priority the primary participant's music wins. Priority is an explicit authored semantic property, never inferred from track names, room types, volume or implementation order. Keep the rule deterministic; no dynamic scoring system unless content later needs one. The primary participant is the fallback when nothing more important plays. (Q150) | High |
| 2026-10-03 | An ordinary participant death is local to that participant and the affected room; it does not rewind other participants' accomplishments. Alice in room A, Bob in room B, Bob defeats a boss, Alice dies: Alice's state/room rewinds as required, and Bob's still-live room and his boss defeat stay. The live world and durable state share one rewind horizon. An explicit whole-session operation (reload checkpoint, restart from checkpoint) may rewind the whole session; that is a different action from one participant's death. Do not rewind durable state globally and then repair the surviving rooms with reconciliation: the rewind boundary itself must be coherent. (Q151) | High |
| 2026-10-03 | The next persistent world-time customer is regrowth/restocking (harvest, leave, world time advances, come back, it has regrown), a cheap independent proof that persistent world time is a reusable engine mechanism and not breakable-specific. Scheduled NPC relocation (home in the morning, market later, tavern in the evening) comes after, because it also needs persistent actor whereabouts, character goals/behaviour, observation/memory and reactive dialogue. Order: breakable respawn → regrowth/restocking → scheduled persistent characters. (Q152) | High |
| 2026-10-03 | The moveset/attack definition owns prospective mechanical reach, also during startup: the hitbox/hurtbox geometry the move will produce, by phase. AI, collision and tooling read a derived view of that actual move geometry. No consumer infers reach from sprite bounds or from a second move-state guess. This extends the moveset timing authority (Q107) from *when* to *where*. (Q35) | High |
| 2026-10-03 | BODY route gates evaluate a body's capabilities/properties (what it can do, what it is: size, form, movement abilities), not its transient action. Predicates over the current action (crouching, dashing, attacking) are a distinct mechanism with their own authored semantics. There is no generic query over arbitrary actor state. Evaluation stays per actor (Q54). (Q58) | High |
| 2026-10-03 | Facing-gated interaction is a wanted capability, with authored semantics (which side the interactable accepts), never sprite geometry. Per-chest persistence is required. Persistence of a physical pickup (the world instance was collected) is required, and is a different fact from an entitlement (Q45). The per-breakable debris cue is wanted; its implementation is deferred. Each capability adds its authored field and its consumer in one change. (Q63) | High |
| 2026-10-03 | The planning-citation checker stays an advisory audit, not a universal hard gate on every change. A focused documentation/planning maintenance workflow may require it to be clean. (Q66) | Medium |
| 2026-10-03 | The evergreen shell owns how the player interfaces with the program: audio master/music/SFX volume, display/window, input bindings, reusable accessibility, localization. Each game owns its game settings: difficulty, gameplay modifiers, combat behaviour, camera policy and mechanics-specific accessibility. (Q68) | High |
| 2026-10-03 | Encounter music outranks ambient music by priority arbitration (ambient < encounter < boss), composed with the Q150 rule (authored priority; the primary participant breaks ties; deterministic). An encounter contributes a scoped music candidate (scope, track/cue, priority), not a subsystem-wide claim. The unused global `EncounterEffect::SetMusic` may be removed; an encounter's ability to influence music stays. (Q72) | High |
| 2026-10-03 | Keep mount/rider support and give it a concrete early customer (a character riding the dog; the robot commandeering a shark). Evaluate and repair `MountedBrainCache` and control transfer, including restoring the mount's previous brain on dismount. Specify: rider, mount, control transfer, the mount's previous brain, dismount/restoration, body and room lifetime, damage/death, participant ownership, animation composition. (Q76) | High |
| 2026-10-03 | Cast framing expresses a bidirectional desired target (zoom in and out toward a desired composition), not only a floor. Keep these separate: desired framing/subject scale, hard zoom bounds, room constraints, smoothing. Downstream stages clamp the desired target; they do not replace it. (Q86) | High |
| 2026-10-03 | Current usage is not a deletion criterion. "Unused", "no shipped customer" or "one customer" is never by itself a reason to delete an engine concept; heavy use is never by itself a reason to keep one. Keep, redesign or delete on semantic quality, architectural elegance, expressive/future utility and maintenance cost. Ambition is engine-first, and engine machinery may run ahead of content. A missing customer is evidence about test coverage and maturity, not about architectural worth. See "keep, redesign or delete" below and the [review guide](../reviewer-guide.md#keep-redesign-or-delete-usage-is-not-worth). (Q74) | High |
| 2026-10-03 | Do not promote external/launch-owned motion to a public cross-game semantic yet. A second independent use case will show which semantics are common, which are game-specific and which belong in the public capability vocabulary. This is a generalization heuristic, not a deletion rule: the local mechanism is not suspect because it has one customer. (Q34) | High |
| 2026-10-03 | The F9 rollback/debug pulse is not a maintainer workflow requirement and does not drive architecture. It is not a product semantic. Engineering may keep or simplify it while it stays clean. Shell state is not simulation state and needs no rollback; the pulse needs no durable cross-session semantics. (Q37) | High |
| 2026-10-03 | Cardinality is explicit, never inferred by merging. A logically singular named bark role/set has one owner: two providers that contribute the same singular role conflict, they do not concatenate. Content that wants several composable bark collections models an explicit plural collection. (Q52) | High |
| 2026-10-03 | `NOT RUN` is a first-class validation state, distinct from PASS and FAIL. The evidence distinguishes "not run, not required now", "not run, required before this handoff/merge/release" and FAIL. A gate blocks only where its policy requires it at the current boundary. Lanes have cadences: focused checks often, broad suites at a milestone or before handoff, expensive validation nightly. Do not repeat a long suite after every edit to keep receipts green. (Q59) | High |
| 2026-10-03 | The Limit meter resets on stock loss: a new stock does not carry the previous stock's Limit unless a future explicit game rule says so. (Q67) | High |
| 2026-10-03 | Do not delete the dependency seams that have no customer (`ambition_characters` → `ambition_causal`, `ambition_platformer2d` → `ambition_sfx_bank`, `ambition_touch_input` → `ambition_cutscene`, `rollback_ggrs::session::install_session`) for that reason. Re-evaluate each on its merits: is the boundary coherent and plausibly wanted, does it remove coupling or clarify ownership, is it simpler than the concept, does it duplicate authority, what does it cost, is there a better representation. This supersedes pruning on customer counts. (Q74) | High |
| 2026-10-03 | The generic fighter brain owns the AI mechanism and exposes reusable controls (reaction, tactics/aggression, prediction/evaluation, execution/error, other decision-policy parameters). Smash rules/content own the ladder: the mapping from "CPU level N" to those controls. Another game reuses the machinery without Smash's difficulty curve. (Q88) | High |
| 2026-10-03 | Remove the inert `read_weight`: a parameter that pretends to change behaviour and does not is misleading (not because it is unused). If rollout/read weighting becomes a real mechanism, add a correctly named parameter with the implementation that gives it meaning. (Q90) | High |
| 2026-10-03 | BODY-PROFILE work is active; do not delete it as stale. Remove it only when the experiment has answered its question and its conclusions live elsewhere, or when the approach is abandoned for architectural reasons. Inactivity or no production customer is not abandonment. (Q92) | High |
| 2026-10-03 | Make the intended renderer work durable: put the prepared renderer commits on a pushed branch or other durable lineage, repin the parent to that commit, then turn on the refusal gate. Important implementation must never be reachable only through agent-local submodule history. (Q95) | High |
| 2026-10-03 | A solid breakable is not semantically a `BlinkWall { Hard }`. When both need the same collision property, extract or reuse a generic solidity/barrier semantic that both compose. Share mechanism; do not conflate gameplay concepts. (Q102) | High |
| 2026-10-03 | Authored worlds may begin with an already-open chest, and more generally in meaningful non-pristine states (door open, bridge destroyed, pickup absent, NPC relocated). Authored initial state lowers into the same canonical state that gameplay produces for the same outcome, not into a parallel runtime representation. (Q105) | High |
| 2026-10-03 | Shell/menu history is outside the simulation timeline and needs no rollback. Peers that start a deterministic gameplay session share an agreed simulation epoch; time spent in the shell must not become peer simulation identity. The handshake waits for multiplayer. This confirms the session-relative `SimTick` engineering chose. (Q128) | High |
| 2026-10-04 | Weapon readiness is a generic semantic state that presentation exposes: approximately `ready` versus `recharging/unavailable` (optionally with progress). A trigger during cooldown must not look like a successful shot. Each weapon's or game's presentation chooses how to show the state (dimmed/disabled, a recharge bar, an audiovisual cue); the engine does not hard-code one treatment into every weapon. Queue row WEAPON-READINESS. (Q33) | High |
| 2026-10-04 | Recoil is a property of the weapon/action, not of a character. The gun-sword/laser sword recoils whoever fires it, a player-controlled body included. The maintainer likes the recoil and keeps it. No character-specific (Pirate) recoil case and no holder-specific suppression: `fire → the firing actor receives the weapon's authored recoil`. A character may modify recoil only through a future explicit gameplay rule. (Q40) | High |
| 2026-10-04 | Projectile launch points and every similar spatial interaction use authored semantic landmarks on the character/rig, never sprite bounds or arbitrary body offsets. Landmarks include hands, muzzle/projectile origin, feet, head, held-item sockets, weapon grips, rider/mount anchors and petting/contact points. A move may apply a move-specific offset from a named landmark. Presentation geometry is not authoritative for simulation: the landmark is semantic authored data that both simulation and presentation read. This extends the 2026-08-17 row ("landmarks are optional slots on a character package"): landmarks remain package slots, and they are an important rig capability, not a bespoke offset per interaction. See "rig landmarks" below. (Q41) | High |
| 2026-10-04 | Hazard wins over a ledge hang. Ledge custody/hanging gives no implicit immunity from a hazard volume: a hanging body inside a lethal hazard receives the normal hazard consequence. An exception needs an explicit gameplay rule. Queue row HAZARD-BEATS-LEDGE. (Q43) | High |
| 2026-10-04 | Do not keep a leaf-game name on a generic engine/public API because that game was its first customer. Rename `SmashChargeSpec` (and its authored field) to the reusable concept it represents, because current source shows it is generic (the Performer's trapdoor and Projectile Polygon's stored shot use it). Apply the same rule to any other generic mechanism with a historical game name. (Q44) <!-- cite-ok: the name this ruling retired; it is `MoveChargeSpec` now --> | High |
| 2026-10-04 | A parked/inactive TwinTrack exhibit does not consume an active simultaneity slot unless parking really consumes the scarce resource the limit represents. A limit describes running/active instances, not dormant storage. (No such slot exists in source today; this is the rule when one is added.) (Q47) | High |
| 2026-10-04 | Mirror symmetry is a correctness property. If nothing in the simulation breaks left/right symmetry, two CPUs with mirrored starting states and the same brain/profile behave identically up to reflection; for the Emmy Noether case this is required. Do not inject random variation, per-seat noise or arbitrary tie-breaks only to make CPUs look less alike. Variation comes only from a modelled asymmetric fact: geometry, observations, state/history, profile/personality, player action, or another explicit asymmetric input. See "mirror symmetry" below. Queue row MIRROR-SYMMETRY. (Q49) | High |
| 2026-10-04 | Do not manufacture demo content only so that every route-gate (or other capability) family has a current customer. Keep an elegant, semantically useful engine capability even when content has not caught up (Q74). (Q55) | High |
| 2026-10-04 | Pointer-driven settings/menu UI gives normal hover feedback. Hover, selection and keyboard/controller focus stay three distinguishable states. Ordinary UI affordance; no further escalation. (Q70) | High |
| 2026-10-04 | Do not prune art because it is unreferenced (Q74). Delete an art row or asset only when it is obsolete, duplicated, wrong or otherwise undesirable. George Booul's TRUE/FALSE (negative, sign-flipped) art is evidence of intended character vocabulary: use it for meaningful Smash gameplay where an elegant mechanic fits George's concept, without inventing mechanics only to consume every asset. The same principle applies to Pirate Admiral's bespoke FX. Owner: [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md#george-booul-truefalse-vocabulary-q81). (Q81) | High |
| 2026-10-04 | Hall-of-Characters actors may be non-interactive and lack dialogue for now. The Hall is also a visual showcase, a population/stress test, an asset/residency test, an animation/rig test and an AI/body/profile composition test. Adding dialogue where appropriate is expected future content work, not a blocker for the Hall architecture. The Hall's stationary policy must hold for every showcase actor through one population policy (queue row HALL-STILL). (Q85) | High |
| 2026-10-04 | Keep Smash's 10× opening countdown as a developer/debug affordance; it need not be ordinary product gameplay. Do not delete it because it is not in normal game flow or not used often. Re-evaluate it only if it becomes an architectural burden. (Q91) | High |
| 2026-10-04 | Keep the bespoke gauntlet fireball look; do not collapse it to the catalog energy ball. A character- or weapon-specific projectile look is legitimate authored vocabulary. It still uses the ordinary projectile and presentation road: bespoke art never earns bespoke simulation. Owner: [`engine/render-animation-and-vfx.md`](engine/render-animation-and-vfx.md). (Q42) | High |
| 2026-10-04 | Mary-O 1-1 gets a fourth reachable `?` block over solid ground so the fire-beacon interaction can be played naturally (the third ladder block stands over a pit). This is a level-content fix; the engine does not compensate for an awkward layout. Owner: [`demos/super-mary-o.md`](demos/super-mary-o.md). (Q46) | High |
| 2026-10-04 | A successful block awards 1.0 Limit (`LimitMeterFill::JONS_BASELINE.on_block`). This is the starting balance value, not a fixed constant: playtesting may tune it without reopening the policy. (Q71) | High |
| 2026-10-04 | Prefer clear authored spawn placement. Where a stage platform overlaps or ambiguously meets a respawn point, move the authored element that looks better (the platform or the spawn) until the spawn is clearly valid; do not teach generic runtime spawn logic to compensate. Today: `smash_platform_stage`, owner [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md) §10. (Q87) | High |
| 2026-10-04 | Thin testing/proof stand-ins (the Robot stand-ins) may keep intentionally incomplete move kits. Do not invent specials to fill each input slot. This covers stand-ins only: when a character becomes real game content (the eventual Robot), its move vocabulary is authored deliberately. (Q89) | High |
| 2026-10-04 | Keep or create a deliberate dense-melee development/stress room. Dense melee is a capability the engine must support (crowd interaction, targeting, collision, AI, VFX, camera, presentation capacity, rig/impostor scaling, combat readability), not a customer made up to justify architecture. It need not be polished game content. Queue row DENSE-MELEE-ROOM. (Q93) | High |
| 2026-10-04 | Editor/source-only products (for example LDtk editor-preview assets) that runtime gameplay does not consume are not packaged or resident. More broadly, the layout makes source/editor products and runtime products structurally distinct, and every quality tier, the highest too, is a named directory using the repository's tier vocabulary (`full`, `half`, `quarter`, `potato`). Packagers select meaningful roots instead of keeping exclusion lists. Audit consumers before moving anything. Queue row ASSET-PRODUCT-LAYOUT. (Q82) | High |
| 2026-10-04 | Requesting one runtime product must not admit hundreds of MB of unrelated runtime products into dependency or residency closure unless they are one runtime unit. Split the runtime packaging boundary of the ~442 MB shared sprite pack accordingly. The shared SOURCE pack may stay. The reason is dependency/residency semantics, not the number of current consumers. (Q83) | High |
| 2026-10-04 | Portraits take part in quality scaling like other presentation assets. Each quality level provides the cheapest portrait product that still does its UI job acceptably; this is not permission for unreadable potato portraits (a portrait is held to its UI job, not to the Q69 sprite rule). The proposed "full resolution only" patch is superseded. Queue row PORTRAIT-TIERS. (Q84) | High |
| 2026-10-04 | The final game wants a bespoke, degenerating "Mode Collapse" music loop. `crooked_ascent_boss` stays as the temporary authored fallback until it exists. This is an art/content follow-up and blocks no engine or gameplay work. Owner: [`game/bosses.md`](game/bosses.md). (Q148) | High |
| 2026-10-04 | Q62 (the 4,741-line `mary_o.ldtk` delta) is not a maintainer yes/no question: no one can judge an LDtk delta by line count. It is an engineering task: a domain-aware LDtk comparison that separates authored changes from serializer/editor churn, plus an investigation of why LDtk files still rewrite. Decide whether a change belongs only on that evidence. The map-assets submodule stays (it keeps churn out of the main history), but it does not replace understanding a diff. Queue row LDTK-SEMANTIC-DIFF. (Q62) | High |
| 2026-10-04 | Q78 (divergent sprite-renderer submodule history) is a git/content-forensics task, not a choice by recency or diff size: list each line's unique commits, say what source change each is, separate source from regenerated artifacts (with domain-aware diffs), find the semantic superset, keep unique work, push it, then repin the parent. Never "take the newest" or "run `submodule update` and accept the result". Shares tooling with Q62. Owner: [`engine/authoring-and-tools.md`](engine/authoring-and-tools.md#content-diffs-need-domain-aware-comparison-q62-q78). (Q78) | High |
| 2026-10-09 | Q166: a push to main does not wait for the lanes its change requires. Jon (relayed by NamekAmbition): "The answer is no. LLMs keep introducing these artificial constraints on things that can be fixed later and I don't want them." `scripts/required_checks.py` reports what a change owes and does not refuse a push. Deleting `scripts/install_pre_push_hook.py` and the `--pre-push` mode, which existed only to refuse, was proposed by an agent on that reason; Jon then answered "Deletion is fine" (relayed by NamekAmbition). <!-- cite-ok: the ruling records the deletion of this file --> | High |

## Consequences that need more than one row

- **2026-08-15, reset semantics:** the checkpoint is the reset baseline. A
  replay restores what the checkpoint promises. Do not infer a second reset
  policy from entity lifetime.
- **2026-08-17, item semantics:** physical occurrence, custody, entitlement and
  durability are separate facts. Ordinary drops may be room-scoped while
  story/unique items persist.
- **2026-09-02, visual quality:** a lower quality setting may use fewer source
  pixels, but no mechanism may draw fewer pixels than the selected quality tier
  promises.
- **2026-09-03, doc-only dependencies:** keep a dependency that is named only
  in a doc comment or intra-doc link, and keep the link. The link serves a
  reader; the cost is one manifest line.
- **2026-09-05, portal presentation:** portal presentation is a composition
  policy. Smash may disable the seamless presentation without weakening the
  reusable portal mechanism.
- **2026-09-05, Limit:** the meter may fill from the obvious authored sources.
  Generic meter validation must not encode one Smash balance doctrine.
- **2026-09-05, demo items:** important demo items deserve real presentation.
  Placeholder art is not a permanent design decision.
- **2026-09-05, authorship:** what Jon explicitly authored is the demo's claim.
  Agents may polish execution, but must not replace an authored idea with a
  different move/content concept because it is easier to implement.
- **2026-09-10, projectile contact (Q96):** a projectile knows only that the
  collision world published a surface with given collision semantics, never
  that a target is an ECS breakable. Where one contributor supplies a surface
  and a damageable volume at the same time of impact, they coalesce into one
  compound contact: a bouncing shot damages a solid crate and bounces. Where the
  surface lies before an inset hurt volume, only the surface was reached.
  Exemptions (ghost, phase, terrain-piercing, one-way-ignoring shots) exclude
  collision classes, never individual targets. Contributor identity is real
  identity, not inferred from matching AABBs or name strings.
- **2026-09-19, session identity (Q132):** there is exactly one canonical live
  `SessionRoot`. A replacement may be prepared while the current session stays
  live, but the candidate carries a distinct prepared-session identity and does
  not masquerade as a `SessionRoot`. Order: prepare B; A stays the sole
  canonical root; retire A; publish B atomically. Do not weaken this for test
  or direct-entry convenience.
- **2026-09-19, scoped mutable state (Q132):** mutable state that can hold
  different values for two sessions, generations, participants or timelines that
  coexist (during preparation, handoff, rollback, multiplayer or testing)
  carries an explicit scope, not anonymous App-global identity. App-global
  mutable state is right only when simultaneous sessions would share exactly the
  same value. Scoped resources need not be ECS children of `SessionRoot`;
  explicit identity and lifecycle ownership are what matter. Prepared immutable
  data may be generation-scoped. Render/device services, logging, asset
  infrastructure, networking transport and caches stay global. User settings
  and durable save data are separate authorities: a mechanical projection from
  them needs explicit admission into a session.
- **2026-10-03, keep, redesign or delete (Q74):** current usage is easy to
  measure, so it is attractive as a surrogate for architectural value. Do not use
  it as one: `unused ≠ bad`, `used ≠ good`. Decide on semantic quality,
  architectural elegance, expressive/future utility and maintenance cost.
  Delete or simplify because a concept is incoherent, duplicates authority,
  encodes the wrong abstraction, adds needless indirection, makes the system
  harder to understand or compose, is expressed more cleanly by another
  abstraction, costs more than its plausible expressive value, or no longer helps
  to express Ambition or a plausible future game. Finding a customer is a good
  way to exercise an abstraction; not having one is not evidence against it.

- **2026-10-04, mirror symmetry (Q49):** for a Noether-style scene
  (a left/right symmetric stage, mirrored starting states, one brain/profile,
  no asymmetric external input), mirrored initial conditions stay mirrored
  until an explicitly modelled fact breaks the symmetry. Each source of
  asymmetry in the simulation must be one of those facts. Iteration order,
  sign mistakes, left/right constants, `signum(0) = +1`, tie-breaks by list
  order or by `SimId`, unmirrored random state and geometry queries that
  prefer one side are defects in such a scene, not variety. Where a tie must
  be broken (two bodies grab one ledge on one tick), the rule that breaks it is
  an authored game rule, named as such. The regression is a per-tick
  reflection test, not an outcome test; owner
  [`engine/fighter-brain.md`](engine/fighter-brain.md#mirror-symmetry-is-a-correctness-property-q49).
- **2026-10-04, rig landmarks (Q41):** part-based characters are not only
  cheaper textures and reusable animation. They give semantic spatial
  composition: hands touch things, weapons attach consistently, projectiles
  leave from believable places, riders mount at authored anchors, petting and
  contact align. Build this as reusable rig/character architecture (named
  landmarks resolved per pose), not as per-scene offsets. A body without a rig
  still answers a landmark from its package, so the capability does not depend
  on rig admission. Owners:
  [`engine/runtime-rigged-sprite-animation.md`](engine/runtime-rigged-sprite-animation.md#semantic-landmarks-q41)
  and [`engine/character-authoring-package.md`](engine/character-authoring-package.md).

- **2026-10-04, asset layout expresses asset semantics (Q82, Q83):** do not
  rely only on packager exclusions to tell source/editor, runtime, quality
  tier and generated intermediate apart when a directory or product topology
  can say it; the layout should make the common mistakes hard. Owners:
  [`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md#products-are-laid-out-by-what-they-are-q82-q83)
  and [`../concepts/asset-management.md`](../concepts/asset-management.md).
- **2026-10-04, quality is a presentation policy (Q84):** a quality level may
  in time select a different implementation of a presentation-only system
  (textures, particles, animation detail, decorative populations, lighting,
  post-processing), not only a smaller texture. Simulation correctness and
  deterministic gameplay stay the same across fidelity choices. Not a
  directive to build those now. Owner:
  [`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md#quality-is-a-presentation-policy-q84).
- **2026-10-04, editor-format diffs need domain-aware reading (Q62, Q78):**
  for LDtk and similar formats a large textual diff is not a large semantic
  change; reviewers prefer the semantic comparison when generated ids,
  ordering or serializer behaviour make noise. The workflow is: domain-aware
  comparison → real changes versus churn → intended semantic history → durable
  lineage → repin the parent. Owner:
  [`engine/authoring-and-tools.md`](engine/authoring-and-tools.md#content-diffs-need-domain-aware-comparison-q62-q78);
  also in `AGENTS.md` (patch discipline).

### The census numbers the 2026-09-19 ingress rulings were sized against

These markers are transcription checks for the Q136 ruling.
`scripts/resources_crossing_the_rewind_boundary.py` checks `crossing-census`,
and `scripts/check_host_produced_sim_consumed_requests.py` checks
`ingress-census`. Each script owns its own classification; when a number moves,
update the marker in the same change and name what moved in the commit message.

<!-- crossing-census: both_side_resources=49 rollback_registered=29 adjudicated_harmless=18 session_edge_only=2 filed=0 unclassified=0 -->
<!-- ingress-census: spent_resources=61 resource_crossings=1 written_messages=94 message_crossings=1 unlocated=52 unlocated_types=18 -->

## Maintenance rule

When a decision is superseded, edit or replace its row. Do not append
commentary underneath it. Git history records how the ruling changed.

Keep each row's `Q` label. Source comments cite `Q117`, `Q118`, `Q120`, `Q124`
and `Q135` and send the reader here, so those labels must stay findable on this
page.
