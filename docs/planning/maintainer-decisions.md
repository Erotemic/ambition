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
| 2026-08-13 | Skitters are Puppy Slug: `SmallSkitter` and `under_town_skitter` are authored as `npc_puppy_slug`. | High |
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
| 2026-09-19 | Do not remove gravity switching. LDtk-authored gravity switches and developer gravity controls stay. The unreachable `GravityFlipSwitch` plate goes. Authored switches, developer controls and future gravity mechanics share one lower-level mechanism for ambient gravity changes; a future gravity plate is an input to it. (Q137) | High |
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

### The census numbers the 2026-09-19 ingress rulings were sized against

These markers are transcription checks for the Q136 ruling.
`scripts/resources_crossing_the_rewind_boundary.py` checks `crossing-census`,
and `scripts/check_host_produced_sim_consumed_requests.py` checks
`ingress-census`. Each script owns its own classification; when a number moves,
update the marker in the same change and name what moved in the commit message.

<!-- crossing-census: both_side_resources=55 rollback_registered=34 adjudicated_harmless=18 session_edge_only=3 filed=0 unclassified=0 -->
<!-- ingress-census: spent_resources=52 resource_crossings=1 written_messages=93 message_crossings=1 unlocated=47 unlocated_types=15 -->

## Maintenance rule

When a decision is superseded, edit or replace its row. Do not append
commentary underneath it. Git history records how the ruling changed.

Keep each row's `Q` label. Source comments cite `Q117`, `Q118`, `Q120`, `Q124`
and `Q135` and send the reader here, so those labels must stay findable on this
page.
