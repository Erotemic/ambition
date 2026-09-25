# Maintainer decisions

This is the compact durable record of decisions Jon made explicitly. It is not
an investigation log. Git history retains the rationale that was present when a
decision was recorded.

Confidence means:

- **High** — proceed on this basis; reopen only with new concrete evidence.
- **Medium** — current direction; implementation or play may refine it.
- **Low** — tentative preference or deliberately deferred choice.

Agents may add a short consequence when a new decision would otherwise be
ambiguous, but do not paste the investigation that led to it. Open questions
belong in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

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
| 2026-07-29 | **Label occlusion / transition nameplates are LOW PRIORITY** — not touched until combat is good. | Low |
| 2026-07-29 | **Do NOT gate or redesign `Interact` yet — it needs a design discussion.** | High |
| 2026-07-29 | **Build the portrait target → art resolver.** | High |
| 2026-07-29 | **Replace the invented lab dummy enemies with real ones that already exist.** | High |
| 2026-07-29 | **DI matters — Smash-style physics is wanted in Ambition itself, not only in versus.** | High |
| 2026-07-29 | **Generic versus ends on HEALTH. Smash Siblings is a separate, specified mode.** | High |
| 2026-07-29 | **Smash Siblings HUD: per-character portrait, stock icons, percentage. No score.** | High |
| 2026-07-31 | **Each robot version is a DIFFERENT CHARACTER. Intended.** | High |
| 2026-08-06 | **Input FILTERING is keyed to the PAD; BINDINGS stay machine-wide.** | High |
| 2026-08-06 | **Dialogue claims only the TALKER's input by default. It no longer stops the world.** | High |
| 2026-08-06 | **A conversation is SUSTAINED, not modal: it breaks on damage or separation, holds its participants if they are capable of it, and barks when it breaks.** | High |
| 2026-08-06 | **Any seat may pause, and the seat that paused drives the menu.** | High |
| 2026-07-30 | Defer the `bevy_ggrs` patch-table leak; revisit once upstream merges the `GgrsFrameTiming` accessor to crates.io. | High |
| 2026-08-08 | **YES — a game may compose this engine WITHOUT a given capability. Capabilities are OPTIONAL.** | High |
| 2026-08-08 | **AMENDMENT to the bbox route: the target figure height is an AUTHORED number with a per-character override — not a compiled constant.** | High |
| 2026-08-08 | **TAKE THE BBOX ROUTE — size and crop the character quad from `body_pixel_bbox`, not the padded frame.** | High |
| 2026-08-08 | **Run a MUCH smaller suite on a docs-only change. Bias toward running fewer tests.** | High |
| 2026-08-08 | **"Declared no abilities" does NOT mean "inherit the dev kit". Character capabilities are AUTHORED, EXPLICIT and COMPOSABLE.** | High |
| 2026-08-08 | **The Perfect Cellular Automaton CAN fly.** | High |
| 2026-08-08 | **Sanic does NOT have blink.** | High |
| 2026-08-08 | **Hitstun needs a REDESIGN in its own session, not a tweak.** | High |
| 2026-08-08 | **The three named robot heavies are DEPRIORITISED, not decided.** | Low |
| 2026-08-08 | **⭐⭐ THE ROLLBACK WIRE FORMAT IS UNSTABLE BY POLICY. The latest build is compatible with itself and nothing else. STOP ASKING.** | High |
| 2026-08-08 | **Portal orientation is AUTHORED per portal, in LDtk. No global setting. Rotation is the default; scale inversion is opt-in and may be unsupported per game.** | High |
| 2026-08-08 | **It is FINE that 1-1's first `?`-block drops its wand into a pit.** | Low |
| 2026-08-08 | **You SHOULD be able to hit GNU-ton during its special.** ⚠ and the boss's authoring is stale generally — not worth deep work unless it is a symptom of an architecture problem. | High |
| 2026-08-08 | **The world keeps living while you die — but it must KNOW you are dying and stop attacking you.** | High |
| 2026-08-08 | **YES — a crawler's collision volume orients with its attachment, and it is the SAME concept as a body under different gravity.** | High |
| 2026-08-08 | **Make most of the Hall cast dormant.** | Medium |
| 2026-08-08 | **Defer all GNU-ton work.** | Low |
| 2026-08-08 | **⭐ MARK staleness where it is found, with EVIDENCE. Sweep later, separately.** | High |
| 2026-08-08 | **(b) Fix the one site; leave the authored-id rule to prose. Revisit the refactor only if it becomes important.** | Medium |
| 2026-08-08 | **⚙ OPERATING NOTE: Jon does not have design intuitions about rollback, and has delegated that design to agents.** | High |
| 2026-08-10 | **⭐⭐ A CHARACTER IS A REUSABLE AUTHORED TEMPLATE, NOT A SINGLETON PERSON — and the enemy-archetype system is deleted.** (answers queue D48; opened D73, closed 2026-08-13) | High |
| 2026-08-13 | **⭐⭐ THERE IS NO SEPARATE "CAN FIGHT" CHARACTER PROPERTY. A character can fight exactly to the extent that its body has abilities/capabilities that can produce combat effects.** | High |
| 2026-08-13 | **Carl Stargan does NOT fly. He fights — because his BODY has abilities, not because of a fighter flag.** (closes D96 5) | High |
| 2026-08-13 | **Skitters ARE Puppy Slug.** `SmallSkitter` and `under_town_skitter` are authored as `npc_puppy_slug`. (closes 2 of D96 1/3/3b) | High |
| 2026-08-13 | **`large_brute` becomes a REAL authored reusable character — a Goblin Brute, with its own Python sprite generator.** (closes the goblin-lab heavy casting call) | High |
| 2026-08-13 | **The dive-drill and its anonymous `Target` may be DELETED. Deletion over architecture for disposable AI-authored content.** | High |
| 2026-08-13 | **Previously unauthored body health is TUNING, not a blocked product decision — pick reasonable numbers and AUTHOR them.** (closes D96 7 and D96 8) | High |
| 2026-08-13 | **Camera orientation is a per-view observer policy. Preserve the existing world-fixed/external-observer camera and add an optional controlled-body/view-subject-relative mode.** | High |
| 2026-08-17 | **⭐⭐ KEEP THE LANDED PER-BODY HITLAG FREEZE. The old *"do not reintroduce a per-body zero-dt"* prohibition is SUPERSEDED. D114 is CLOSED.** | High |
| 2026-08-17 | **⭐⭐ KEEP THE PER-TURN GATE SMALL — `cargo test --workspace --lib` stays OUT of `gate_suite.py` DELIBERATELY.** "Gate" continues to mean an EXECUTABLE gate; the pre-push checklist is a separate validation tier. **D160 CLOSED as an intentional policy choice, and `awaiting-maintainer-decision.md` §9 is ANSWERED.** | High |
| 2026-08-17 | **ACCEPT the CPU showcase's current pacing. Do NOT retune stock count, knockback or damage. D128's pacing/product-acceptance blocker is CLEARED** — D128 stays open for its engineering/presentation defects. | High |
| 2026-08-17 | **REPLACE the `central_hub_main` developer-note sign — do NOT delete it.** The hub wants an orientation sign; only the authoring-language content was wrong. | High |
| 2026-08-17 | **Inventory becomes Morrowind-style: the OCCURRENCE owns, and EVERY inventory entry carries a COUNT — usually 1.** | Medium |
| 2026-08-17 | **A line count is a PROXY. Decompose where it makes sense, and stop making the monolith worse.** | High |
| 2026-08-17 | **Split-screen layout is ADAPTIVE WITH HYSTERESIS.** | Medium |
| 2026-08-17 | **Sprite sizing: give every scale a SHARED UNIT first, then revisit the quad-from-bbox route.** | High |
| 2026-08-17 | **Capability progression SPLITS BY WHAT THE VERB IS: physical verbs are BODY-owned, knowledge is PARTICIPANT-owned.** | Medium |
| 2026-08-17 | **⭐⭐ A MODEL-BACKED CHARACTER IS A REMOTE PLAYER, NOT A BRAIN IN THE TICK — and the question is DEFERRED.** | Medium |
| 2026-08-17 | **The 23 already-clipped sprite sheets are fixed CASE BY CASE, driven by the draw-time warning.** | High |
| 2026-08-17 | **A dropped held weapon PERSISTS OR NOT PER ITEM — authored, not global.** | Medium |
| 2026-08-17 | **ONE WORLD UNIT IS ONE BASE-GRID PIXEL — 16 units to a tile.** | High |
| 2026-08-17 | **A declared character height is a CONTRACT: art scales to it, and a tight tolerance WARNS when the scale drifts.** | High |
| 2026-08-17 | **Landmarks are OPTIONAL SLOTS on a character package — authored when useful, never required.** | Medium |
| 2026-08-17 | **PROMOTE `engine/character-authoring-package.md` to a live ledger row, with canonical height as its FIRST SLICE.** | High |
| 2026-08-17 | **Giant and multi-part bodies declare canonical height by the SAME rule — one vocabulary, no exemption list.** | High |
| 2026-08-17 | **Camera shake stays CONSTANT IN THE WORLD; the field is renamed to say so.** | High |
| 2026-08-17 | **A projectile respects the AUTHORED HURT VOLUME — the same geometry melee uses.** | High |
| 2026-08-17 | **DEFER the per-creature ability absence list until the cast is bigger.** | Medium |
| 2026-08-18 | **MINIMIZE POISON TESTS — poison only below ~60% certainty that the guard bites.** | High |
| 2026-08-18 | **AUTHORING A CHARACTER'S SIZE MUST BE CONSISTENT AND TRIVIAL TO TUNE — one number, and the geometry follows.** | High |
| 2026-08-18 | **MARY-O IS ONE BRICK TALL SMALL (16) AND TWO GROWN (32), AND HER SMALL ART IS REWORKED TO HALF THE GROWN HEIGHT AT THE SAME WIDTH.** | High |
| 2026-08-18 | **THE TALL SPRITE MAY BE VISUALLY WIDER; THE COLLISION WIDTH STAYS IDENTICAL FOR BIG AND SMALL.** | High |
| 2026-08-19 | **D166's CPU-GRAB WORK IS THE POLICY HALF FIRST: the FIGHTER CAPABILITY owns what a HOLD is WORTH.** The mechanical fixes (a start-gate on the option list, tighter grab spacing) wait behind it. | High |
| 2026-08-19 | **`body.action_buffer` STAYS — a registered rollback row with no writer, DOCUMENTED as declared-but-unfed.** | Low |
| 2026-08-19 | **AI SLOP HONOURS ITS AUTHORED SIZE CONSTANT — every slop shrinks from 73.9 × 48 to 28 × 18.2.** | Medium |
| 2026-08-19 | **THE `.loop` CUE DERIVATION TRAP IS LEFT RECORDED — no exemption list, no fallback.** | Low |
| 2026-08-19 | **`test_oiler_svg_rig.py`'s EIGHT AUTHORING-STRUCTURE TESTS STAY. Oiler's SVG is settled.** | Low |
| 2026-08-19 | **D162's THREE SHEET-MANIFEST COLLISIONS GO TO JON PAIR BY PAIR — written up, not resolved by rule.** | Medium |
| 2026-08-20 | **AVOID PUSHOUT is about PORTALS, not bodies. Jostle is allowed — but it may NEVER be a mandatory part of the movement kernel.** | High |
| 2026-08-22 | **RENAME THE BLAST ZONE OUT OF EVERY WORLD — `World.edges: WorldEdgeMargins { fall, side, rise }`, Rust and LDtk keys in ONE change.** | High |
| 2026-08-22 | **THE BAKED SHEET REGISTRY KEYS BY FILE ROOT — a renderer target string may not be a durable engine identity.** | High |
| 2026-08-22 | **`SeatRawFrames` STAYS GENUINELY RAW — build the stage model, and world-dependent semantics land AFTER the boundary, not before it.** | High |
| 2026-08-22 | **SWEEP THE CRATES A CARVE TOUCHES — do not widen the gate to `--workspace --all-targets`.** | High |
| 2026-08-22 | **A LEVEL'S POSITION IS OWNED BY THE LAYOUT TOOL — and ownership FOLLOWS THE LAYOUT MODE.** | High |
| 2026-08-22 | **HEIGHT OWNS WORLD SIZE; SOURCE-ART DENSITY IS A SEPARATE CONTRACT — and the 1.0 warning is REMOVED, not replaced.** | High |
| 2026-08-22 | **A HIT'S ART FOLLOWS BOTH THE VICTIM'S MATERIAL AND THE BLOW'S STRENGTH.** | High |
| 2026-08-22 | **IMPACT HITSTOP IS A BOUNDED MATCH-LEVEL REQUEST FROM THE CONNECT — it is combat presentation, not a slot-0 affordance.** | High |
| 2026-08-22 | **A BOSS'S HOARD IS PER-BOSS EVENTUALLY; FOR THE DEMO IT IS CURRENCY.** | High |
| 2026-08-22 | **`DebugLabel` IS DEBUG, AND IT KEEPS SHIPPING — the whole world is scaffold.** | High |
| 2026-08-22 | **PROXIMITY-GATE EDGE-EXIT LABELS — and label visibility is a GAME-SELECTABLE POLICY, not a fixed rule.** | High |
| 2026-08-22 | **DISABLE rust-analyzer — no second target directory.** | High |
| 2026-08-22 | **CORRECT THE LEVEL-1 CPU FOR FEEL — the easiest rung is bad at FIGHTING, not self-destructive.** | Medium |
| 2026-08-22 | **MARY-O KEEPS THE 56 px SHARED COLLISION WIDTH, AND HER SHORT-FORM CROWN RISES ~6 px.** | High |
| 2026-08-22 | **FIX MARY-O'S WALK DIP PROPERLY — add a pose field that lowers the TORSO without moving `foot_y`.** | High |
| 2026-08-22 | **THE MARY-O RESTART REPORT IS CLOSED — it was Mary-O, and it is believed RESOLVED.** | Medium |
| 2026-08-22 | **ADVANCE THE `dev/ambition_dev_measurements` POINTER PERIODICALLY — the cadence does not matter.** | Low |
| 2026-09-10 | **A PUBLISHED COLLISION SURFACE PARTICIPATES IN PROJECTILE COLLISION, AND A DESTRUCTIBLE'S SURFACE PLUS ITS HURT VOLUME ARE ONE COMPOUND CONTACT** — damage once AND apply the surface response. (Q96) | High |
| 2026-09-12 | **TAKE THE STRONGER LAST-GOOD-WORLD GUARANTEE — A10 IS THE NEXT MAJOR ARCHITECTURE PACKET.** A candidate scene is constructed and validated OFF TO THE SIDE and published only on success; a rejected candidate leaves the running world untouched. NOT arbitrary transactional rollback of arbitrary ECS commands — A10 stays bounded to typed construction recipes, constrained candidate construction, relationship/resource validation, one controlled publication boundary and explicit retirement of the old world. (Q113) ⚠ Provenance: ruled in the architecture review Jon forwarded 2026-09-12, not from a separate instruction. | High |
| 2026-09-12 | **A PLANNER THAT SCORES MOVE A WHILE THE EXECUTOR PERFORMS MOVE B VIOLATES THE ACTION MODEL WHATEVER THE MATCHUPS SAY** — so the truthful attack kit landed unconditionally, the `truthful_attack_kit` feature flag is gone, and **re-pricing CPU matchups is the EXPECTED consequence, not an objection to it**. Measured at the shipped duel: the admiral's dash attack went from 18 of 75 to 33 of 69 damage events and the duel now DECIDES at tick 2320 where it ran 3613 undecided. ⚠ What remains is Jon's and is TUNING, not this ruling: which utility / run / dash-attack parameters give the fighter quality he wants now that the brain reads the frame data of the move it actually takes. (Q117) ⚠ Provenance: ruled in the review that found the mismatch. | High |
| 2026-09-13 | **A CONTENT PUBLICATION MAY STOP AND REBASE A ROLLBACK TIMELINE THIS HOST OWNS — THAT IS NOT A REFUSAL.** A pending generation is admitted only against a timeline that is healthy AND locally rebasable; `External` and `Caller`-owned timelines are never replaced unilaterally, and a RECORDED divergence refuses outright. The admission is a transaction-lifetime LEASE re-asked at the activation — not a check at the instant somebody asked — and the stop-and-rebase happens in the same exclusive step as the publication, because a frame between the two resimulates new content on the old timeline. ⚠ Not closed: the interval between the publication breaker and `commit_content_generation` is narrowed by ORDERING and owned by nobody; making the commit fallible is rejected, since it would recreate the half-transaction this road exists to prevent. Owner: `ambition_content::reload::reload_tests::a_boundary_that_closes_after_the_breaker_still_publishes`, which records that the mutation can be inserted and flips the day one activation authority owns the whole boundary. (Q118) ⚠ Provenance: ruled across the 2026-09-13 architecture review and the canary measurements it asked for, not from a separate instruction. | High |
| 2026-09-13 | **A DEVELOPER MECHANICAL EDIT IS A PROPOSAL UNTIL THE TIMELINE'S OWNER ADMITS IT, AND THE DECISION HAPPENS BEFORE THE ADVANCE.** No live timeline → publish; a timeline this host owns → stop it, then publish (model 2, by precedent with Q118's road); `External`/`Caller`-owned or diverged → **refuse, meaning the authoritative value does not move**, with the proposal RETAINED so it publishes the moment ownership returns. Editor panels are MIRRORS of the authoritative mechanical values, never the authority. ⚠ Not closed: the acceptance the review asked for — a real GGRS canary that also carries the developer-tools chain — belongs to no single composition today. Owner: `game/ambition_app/tests/developer_edits_under_rollback.rs::the_canary_rig_has_no_developer_edit_road_to_admit`, which fails the day that harness gains the chain. (Q120) ⚠ Provenance: ruled in the 2026-09-13 architecture review. | High |
| 2026-09-13 | **ONE AUTHORITY ANSWERS *“is mechanical mutation legal around rollback?”*, AND THE CALLERS DIFFER ONLY IN TRANSACTION LIFETIME.** `ambition_platformer2d::rollback::mechanical_mutation_boundary` owns the fact; Q120 consumes it instantaneously and Q118 holds it as a lease. Q118 and Q120 had each implemented the predicate, and the duplication produced a real defect — the content lease re-asked HEALTH and not the OWNERSHIP condition that authorized publication. (Q118, Q120) | High |
| 2026-09-13 | **WITHDRAWN AS NOT A MAINTAINER'S CHOICE: WHAT A DEATH-RESET DOES WITH AN OBJECT IN YOUR CUSTODY IS ALREADY DECIDED, AND IT IS TEMPORAL RATHER THAN ITEM-KIND.** The reset restores the state at the checkpoint, PER OBJECT, by which side of the checkpoint each acquisition fell on — pinned by `a_death_returns_what_was_not_banked_and_keeps_what_was` (`game/ambition_app/tests/death_restores_the_checkpoint.rs`), whose beat 7 has two objects of the same kind in the same hand in the same frame reaching OPPOSITE answers. Any kind-shaped or road-shaped rule has ONE answer for those two and would have contradicted a shipped arm. ⇒ What remained was A10 engineering and is done: the transaction declares `superseding(id, id)` for an identity the baseline still holds and `reconstructing` for the rest, so the verifier names which body is unexpected. (Q124) | High |
| 2026-09-19 | **⭐⭐ PRIORITY ADJUSTMENT: SCRIPTS ARE NOT THE PRODUCT.** Static review and ordinary code inspection are sufficient for many architecture invariants — do NOT build a parser, witness generator, poison suite or permanent guard for every ADR statement. Scripts earn their place by giving genuinely valuable observability or by catching a repeatedly demonstrated failure class. Effort belongs on: structurally better Rust architecture; direct runtime/game observability; tools that help balance and polish combat; edit-to-play iteration; world/content authoring and beginning to lay out the open world; and current rollback correctness *where it affects real mechanics*. ⛔ **Netplay is a future goal and NOT a goal for this year** — do not burn large effort on speculative P2P-only problems, especially in architecture likely to be refactored first. ⚠ The removed `K` clone feature is the cautionary example: substantial engineering went into preserving and debugging a temporary affordance whose only purpose was to force the actor architecture to stop treating the player as special. Do not repeat that pattern. | High |
| 2026-09-19 | **⭐⭐ TWO EXPLICIT COMPOSITION MODES, AND THE GAME IS ESSENTIALLY IDENTICAL IN BOTH.** A game must be able to launch DIRECTLY or run INSIDE THE SHELL. The one meaningful semantic difference under the shell is that the game can RETURN to it; for parity the direct-launch build still shows the same "return to shell" menu item, disabled/greyed out because there is nowhere to return. ⛔ Shell presence must NOT alter ordinary game simulation, mechanics, capabilities, registries, content or game policy, and future platform-level overlays are not a reason to couple more shell behaviour into the game. Production shell sessions use the proper prepared/session lifecycle; explicit direct/headless/test compositions may hold scoped fixture/direct-entry authority where needed, but **no anonymous App-global fallback state returns**. Capabilities stay optional and composable: a capability an authored production content item REQUIRES and the composition lacks must REFUSE that content or its admission rather than silently pretending it works, while deliberately reduced tools and tests may omit capabilities explicitly. ⇒ Implement this architecture rather than continuing to census hypothetical composition variants. (Q146, Q144, Q108, Q106, Q100, Q97) | High |
| 2026-09-19 | **⭐⭐ AN ABILITY CONTACT IS INDEPENDENT BY DEFAULT — PROVENANCE IS EXPLICIT.** It credits a move's `Connected`/contact condition ONLY when it explicitly carries provenance identifying the launching move occurrence. ⛔ `None` must NOT mean *"credit whatever move happens to be playing now"*. ⛔ `Some(old_instance)` must never credit a different current occurrence — that was a BUG, fixed directly, and never a policy question. If Blink, Dive, Mark Recall, an empowerment or another mechanic is intentionally designed to count toward its launching move, thread the launching occurrence explicitly. (Q101) | High |
| 2026-09-19 | **⛔ NO GENERIC ONE-DIMENSIONAL ENGINE "DIFFICULTY" ARCHITECTURE, AND THE WHOLE TOPIC IS DEPRIORITISED.** Difficulty is primarily GAME POLICY expressed as presets over whatever that particular game cares about. For the Ambition exploration game an Easy/Hard preset might eventually adjust player health, incoming damage, and perhaps game-authored enemy behaviour — not important now. For Smash-like modes there may be no general difficulty at all: participants may have explicit HANDICAPS and CPUs have BRAIN/AI LEVELS, and those are separate concepts. ⇒ Participant-specific accessibility/assist/handicap state and game/match policy stay DISTINCT. ⚠ Most importantly: get the default/Normal game playing exceptionally well first. Preserve enough architecture not to be boxed in later; do not spend substantial current effort designing difficulty systems. (Q127) | High |
| 2026-09-19 | **⭐ FOR THE SMASH-LIKE GAME, FOLLOW SMASH: ORDINARY SCALING THROWS PARTICIPATE IN RAGE, AND SET-KNOCKBACK KEEPS ITS SET-KNOCKBACK SEMANTICS (as in Ultimate).** ⛔ Not a universal engine law: rage, and whether a particular move or throw is influenced by it, are game-level combat policy the engine must be CAPABLE of expressing. ⚠ If the CPU-duel benchmark changes when throws correctly obey rage, that is combat/AI/balance evidence — not a reason to preserve a mechanics inconsistency. (Q133) | High |
| 2026-09-19 | **⛔ DO NOT REMOVE GRAVITY SWITCHING. The LDtk-authored gravity switches are real game content and stay** — the symmetry/C4 room's four directional switches, the authored hub gravity flip, and any other legitimate encounter-authored gravity control. Developer ability to change gravity is also important right now and stays available. ⇒ What disappears, absent an actual current product use, is the separate unreachable `GravityFlipSwitch` overlap/pressure-plate vertical that nothing authors or spawns; do not preserve rollback/view/render/schema infrastructure solely for the dead plate. ⭐ Converge the legitimate roads: authored LDtk/encounter switches, developer/debug gravity controls and future gravity-changing mechanics should share substantial lower-level machinery for applying ambient gravity changes rather than each owning its own implementation of the same fact. If a gravity pressure plate is wanted later, build it as an INPUT feeding the shared mechanism, not as a revived parallel gravity implementation. (Q137) | High |
| 2026-09-19 | **⭐ ONE FRAME OF STALE UI IS ACCEPTABLE.** The UI does not need rollback merely because it displays rollback-owned game state; actions initiated through UI that affect authoritative simulation state need the appropriate deterministic/simulation ingress, and presentation may remain presentation. ⛔ Do not introduce duplicate authoritative inventory state or substantial optimistic-reconciliation machinery to hide one frame. Finish the existing fix simply and move on; spend no more architecture budget here unless playtesting demonstrates a UX problem. (Q140) | High |
| 2026-09-19 | **⭐ A CUTSCENE FADE CARRIES AN EXPLICIT AUTHORED START AND TARGET ALPHA** (`from_alpha` / target alpha or equivalent). ⛔ Do not rely on a hidden *"all cutscenes start black"* convention. ⚠ Cutscenes are currently low priority and the existing authored ones are not valuable enough to justify substantial preservation effort: make the semantics sane with minimal work, update the few authored sites, move on. (Q143) | Medium |
| 2026-09-19 | **⛔ NOT BLOCKED, DO NOT WAIT FOR A FURTHER RULING** on any of: **Q132** — one canonical live `SessionRoot`, and a prepared candidate has a DISTINCT candidate identity that must not masquerade as `SessionRoot`; **Q136** — choose ingress by semantic ownership, and current rollback correctness is engineering, not a maintainer policy blocker; **Q138** — an invalidated harness must REFUSE or FAIL rather than silently produce frozen observations; **Q139** — do not grow architecture merely to satisfy a static presentation-writer census; **Q122** — mechanical identity fingerprints MECHANICAL FACTS, not explanatory prose; **Q104** — content-authored movesets are the long-term authority and duplicate Rust tables are migration scaffolding, not permanent architecture; **Q110** — mechanical registry changes use proper explicit lifecycle/replacement semantics, and no universal silent overwrite is invented; **Q145** — derive room-transition ordering from actual transaction semantics; **Q141** — durability is per-item and authored, and a runtime-spawned item MAY be durable when explicitly authored that way. | High |
| 2026-09-24 | **AP19: A BODY'S DEFAULT ABILITIES ARE THE CONTENT PROVIDER'S DECLARATION.** Each provider declares its actor default; preparation resolves every character's `abilities` to authored-or-declared, so the blueprint carries a set rather than an `Option`, and `ambition_body_seed` holds no default. | High |
| 2026-09-24 | **AP12: ARM THE MELEE COOLDOWN.** The move road arms `BodyMelee::cooldown` from the authored profile (`BrainProfile::attack_cooldown_mult`), so the AI swing gates close as authored. This is an accepted behaviour change: AI swings get slower. | High |
| 2026-09-24 | **AP12, REFINED: THE PACE IS AUTHORED.** The ruling above rested on a base constant that never existed. Asked again, the answer was "author it somewhere": the profile authors `attack_cooldown_s` in seconds, unauthored means no floor, and the engine holds no pacing number. | High |
| 2026-09-24 | **W004: IMPLEMENT THE LUNGE STEP.** `LungeSpec::step_px` is carried into the attack move as windup self-motion. The velocity law is an engineering choice that needs play-tuning. | High |
| 2026-09-24 | **W026: THE PROVOKED POLICY IS RULESET/CONTENT-OWNED.** `default_provoked_policy()` stops being an engine answer; an explicit ruleset or content policy states what a provoked actor becomes. | High |

### The census numbers the 2026-09-19 ingress rulings were sized against

⛔ These are TRANSCRIPTION CHECKS, not a second classification. Each census owns
its own verdicts; the marker states what the ruling was sized against, and the
script fails if the two disagree. They moved here from
`awaiting-maintainer-decision.md` when `Q136` was ruled and deleted — a marker
pinned to a page that no longer states the fact is a check against nothing.

<!-- crossing-census: both_side_resources=56 rollback_registered=33 adjudicated_harmless=19 session_edge_only=3 filed=1 unclassified=0 -->
<!-- ingress-census: spent_resources=56 resource_crossings=3 written_messages=95 message_crossings=3 unlocated=41 unlocated_types=15 -->

⛔ **`both_side_resources` WENT 57 → 56 AND `adjudicated_harmless` 20 → 19 ON
2026-09-24, and the one that left is named:** `DeveloperRuntimeState`. AP17
(`5539d8667`) moved its HUD-flash decay out of the simulation into `Update`, so
it no longer crosses the boundary.

⛔⛤ **`unlocated` WENT 42 → 41 ON 2026-09-21, AND THE ONE THAT LEFT IS NAMED
RATHER THAN SUBTRACTED.** Diffed against `ace00e006`, the commit that wrote this
marker, by running `unlocated_message_systems` over both trees: the set lost
exactly `gravity_flip_switch_system` and gained nothing. That is `9732f9d45`
(Q137) deleting the unreachable gravity pressure plate — **the same commit, the
same afternoon, also took the alias census from 183 to 182**, and neither guard
was run against it. ⇒ One deletion, two stale transcriptions on two different
pages, and both were found by running the guards rather than by reading either
page. ⚠ A number that falls because its subject was correctly deleted is not
drift in the population; it is drift in the TRANSCRIPTION, which is exactly what
these markers exist to catch.


## Supplemental rulings that were previously stored as long-form sections

- **2026-08-15 — reset semantics:** the checkpoint is the reset baseline. A
  replay restores what the checkpoint promises; do not infer a second reset
  policy from entity lifetime.
- **2026-08-17 — item semantics:** physical occurrence, custody, entitlement and
  durability are separate facts. Dropped weapons persist or not **per item**, by
  authoring; ordinary drops may be room-scoped while story/unique items persist.
- **2026-09-02 — visual quality:** a lower quality setting may use fewer source
  pixels, but no mechanism may draw fewer pixels than the selected quality tier
  promises.
- **2026-09-05 — portal presentation:** portal presentation is a composition
  policy; Smash may disable the seamless presentation without weakening the
  reusable portal mechanism.
- **2026-09-05 — Limit:** the meter may fill from the obvious authored sources;
  generic meter validation must not encode one Smash balance doctrine.
- **2026-09-05 — demo items:** important demo items deserve real presentation;
  placeholder art is not a permanent design decision.
- **2026-09-05 — authorship:** what Jon explicitly authored is the demo's claim.
  Agents may polish execution, but should not replace that authored idea with a
  different move/content concept merely because it is easier to implement.
- **2026-09-10 — projectile contact with published surfaces (Q96):** a projectile
  must not know *"this is an ECS breakable"*; it must know *"the collision world
  published a surface with these collision semantics."* A
  `BreakableCollision::Solid` surface therefore participates in projectile
  collision. Where the same contributor supplies both a surface and a damageable
  volume at the same time of impact, they **coalesce into ONE compound contact**:
  damage the target once **and** apply the projectile's physical surface response.
  A bouncing shot damages a solid crate **and** bounces. *"Wall wins, therefore the
  crate is invulnerable"* is rejected. Where the surface lies before an INSET hurt
  volume, only the surface was reached — no damage yet.
- **2026-09-10 — projectile exemptions are POLICY, not FAMILY (Q96):** a ghost
  shot, phase shot, terrain-piercing round or one that ignores one-ways excludes
  appropriate **collision CLASSES**. Never a per-target carve-out.
- **2026-09-10 — contributor identity must be REAL identity (Q96):** not inferred
  from matching AABBs, and not from name strings such as `"ecs-breakable foo"`.
  This ratifies the projectile contact protocol's existing wording.

- **2026-09-19 — exactly one canonical live `SessionRoot` (Q132):** two
  published/canonical roots are INVALID. Normal lifecycle transitions — game A →
  menu → game B, game A → direct replacement by game B, and preparing game B
  while A is still live — must preserve that invariant. Preparing a replacement
  while the current session stays live is allowed, but the incoming candidate
  must carry a DISTINCT candidate/prepared-session identity and must not
  masquerade as a `SessionRoot`. The order is: prepare candidate B → A remains
  the sole canonical root → retire/terminalize A → publish B atomically as the
  new canonical root. Do not weaken the invariant for test or direct-entry
  convenience.
- **2026-09-19 — session-dependent mutable state must carry explicit scope
  (Q132, and it governs beyond it):** *if mutable state can legitimately hold
  different values for two sessions, generations, participants or timelines that
  could coexist during preparation, handoff, rollback, multiplayer or testing,
  it must carry the appropriate explicit scope rather than relying on anonymous
  App-global singleton identity.* Conversely, App-global mutable state is
  appropriate ONLY when simultaneous sessions would legitimately share exactly
  the same object or value. This does not require every piece of session state
  to be an ECS child of `SessionRoot` — explicitly keyed or scoped resources and
  other clearly owned state are fine. The property that matters is explicit
  identity and lifecycle ownership. Generally scoped: current room/world/session
  state, participant state, encounter state, simulation clocks and timeline
  state, checkpoint/restore state, transient progression, session
  request/admission queues, admitted mechanics/configuration, rollback
  authorities, cutscene/session gameplay state. Prepared IMMUTABLE data may
  instead be generation-scoped and may coexist across generations. Truly
  application-global infrastructure — render/device services, logging, asset
  infrastructure, networking transport, caches — stays global where that is
  genuinely its ownership. User/account settings and durable save data are
  SEPARATE AUTHORITIES: they must not silently become live simulation state
  merely because they are App-global, and a mechanical projection from them
  needs explicit admission into a session.

## Maintenance rule

When a decision is superseded, edit or replace the row. Do not append a second
page of commentary underneath it. Git history is the record of how the ruling
changed.

⛔ **AND A `Qnnn` A COMMENT DEFERS TO MUST RESOLVE — HERE, OR AS A LIVE QUESTION
ON [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).** An
answered question is normally DELETED from that page, which is right; what is
not right is deleting one that source comments send the reader to. Measured
2026-09-19 across the tracked tree: 119 distinct Q numbers are cited, 90 are
live questions, and 22 resolved to nothing at all — but **only Q117, Q118, Q120
and Q124 actually DEFERRED** (*"see `Q118` in the decision ledger"*, *"what the
ruling in `Q124` decides"*, a bare *"See `Q117`"*). The other eighteen carry
their own answer in the same paragraph and lose nothing, so the rule is about
the DEFERRAL and not about the number. (`Q123` is the near miss: three sites
name an *"open clause"* of it, but each QUOTES the clause verbatim and then
measures it, so nothing is being sent anywhere.) All four now resolve, and no checker was
added for it: the population that matters is four, and telling a deferral from
a self-contained citation is a prose judgement.
