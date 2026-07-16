

GPT 5.6 Says:

Here’s a message that opens the discussion without rehashing the prompt Fable already received:

I reviewed your recon against my own deeper pass through the current code. I think we are substantially aligned, but I want to compare a few conclusions before we turn the review into an execution plan.

Where I agree:

* The content evictions are real and worth doing. In particular, provider-owned named items, projectile identities, asset catalogs, art bindings, fighting-game techniques, and other flagship-game instances should not be compiled into reusable engine crates.
* `ambition_sim_harness` is a strong crate split. Programmatic reset/step, typed actions and observations, deterministic testing, RL, replay, fuzzing, and headless simulation form a coherent reusable engine face.
* The provider implementation currently hidden in `crates/ambition/src/provider.rs` deserves its own crate. My preferred working name is `ambition_platformer_provider`, though the exact name is negotiable. I think the extraction should consolidate the repeated preparation, exact activation, session construction, and cleanup protocol used by the different providers, rather than only moving the file.
* Explicit provider registration in the host composition root is fine. The two-line exception should be documented rather than replaced with opaque plugin discovery.
* A boss carve should be deferred until boss attacks and phase behavior have converged onto the canonical moveset/action path. After that convergence, we should reassess whether a standalone boss crate is still justified.
* Renaming `features` may eventually improve navigation, but `sim` may be too broad and the change is low priority until we can state precisely what that subtree owns.

Where I disagree:

* I do not want the named-content scanner. It feels like a high-maintenance heuristic that will generate false positives, exception lists, and more poison tests without enforcing the real architecture. I would prefer Cargo dependency direction, Rust visibility, typed registration, ownership boundaries, and ordinary integration tests. A scanner should only exist for an unusually specific failure that cannot be expressed structurally.
* More generally, I think the repository is spending too much effort building a parallel static-analysis language out of source scans and poison fixtures. Existing tests that guard concrete historical regressions can remain, but every new policy test should have to justify why the compiler, API design, or a behavioral test cannot enforce the invariant.
* Cutscenes and encounters should remain separate. The distinction I am using is: cutscenes are scripted with a small amount of interaction, while encounters are interactive with a small amount of scripting. They may share small lower-level primitives such as clocks, gates, cursors, or typed effect emission, but I do not think they should share one authored timeline or universal sequence DSL.
* I am not convinced the current app menu directory should be moved wholesale into an engine crate. Some of it is reusable menu-host machinery, but much of it is an opinionated Ambition/OoT-style inventory product with fixed layout, item, health, mana, and equip policy. I would extract only the demonstrably reusable layer and leave flagship-game policy in the app.
* I do not consider the concept of a shrine itself a content leak. A shrine can be reusable platformer vocabulary. Specific healing, save, presentation, and provider policy may need to move, but I would not classify every named mechanic as provider content.

There are also three architectural issues that I think deserve more weight than they received in the recon:

1. Session-root authority is not yet exclusive. `PlatformerSessionWorld` says live platformer state belongs to the exact session root, but process-global structures such as `SceneEntities` and room-derived moving-platform state still coexist with it. That affects provider switching, exact restore, multiple sessions, headless parity, and observer-relative semantics.

2. Initial activation/reset and normal room transitions do not appear to use one authoritative placement-lowering path. Some paths construct a local built-in `PlacementLoweringRegistry`, which can bypass provider-installed interpreters. I think activation, reset, transition, and restore must all lower through the installed App-local registry.

3. Runtime should retain authority over global ordering, but domain crates should increasingly own their own messages, resources, local systems, and public schedule sets. Runtime should order domain sets rather than know every combat, projectile, actor, and effect leaf function.

My tentative combined priority order is:

1. Extract and consolidate the platformer-provider lifecycle.
2. Unify placement lowering across activation, reset, transition, and restore.
3. Finish session-root ownership and remove stale process-global mirrors.
4. Perform the clear content evictions using structural boundaries, not a noun scanner.
5. Extract the simulation harness.
6. Converge boss behavior onto the canonical action/moveset authority.
7. Improve domain-plugin ownership while preserving centralized phase ordering.
8. Split pure touch-input folding from its visual control overlay.
9. Clean up the most valuable render/read-model seams.
10. Reconsider menu extraction, boss decomposition, and the `features` rename after those changes.

I would like your pushback on three points in particular:

* Do you see a reason the named-content scanner catches something important that structural enforcement cannot?
* Do you still think cutscene and encounter semantics are close enough to justify one timeline after applying the scripted-versus-interactive distinction?
* How would you rank session-root authority and the placement-registry inconsistency relative to the content-eviction work?

This leaves Fable room to defend its reasoning and focuses the next exchange on the substantive disagreements.

----

Fable Says

Verified — and it's worse than a style issue: initial session activation (session/setup.rs:117) and session reset (session/reset/mod.rs:230) lower placements through a locally-built, built-ins-only registry, while room transitions (world/rooms/load.rs:126) and snapshot restore (snapshot/restore.rs:20) use the App-installed resource that providers register into. GPT 5.6's claim is real, and I'm updating my ranking accordingly.

Here's the message to send back:

To GPT 5.6, from fable (mediated by Jon):

I've verified your two new architectural claims at source before responding, so the disagreements below are narrow and the concessions are real. Consensus first: content evictions (yes), ambition_sim_harness (yes), provider extraction as a protocol consolidation rather than a file move (yes — and my measurement supports yours: the prepare_session/activate_session pair is near-verbatim across all four providers, ~100 LOC of copy-paste per game, so there is a real protocol to consolidate, not just a file to relocate), documented two-line host registration (yes), boss carve deferred behind moveset convergence (yes), features/ rename low-priority and "sim" possibly too broad (yes).

On your three questions:

1. The named-content scanner — conceded, with one requirement that replaces it. You asked what it catches that structure cannot: honestly, exactly one class — baked identity with no dependency edge. enum Item's 24 variants, pirate_weapon.rs, the dialogue-blip constants are all seam-clean; no Cargo direction, visibility rule, or dep-test can see them, because the leak is a name and a closed set, not an import. But the right response isn't a noun scanner — it's that each eviction must end in a structural shape, not a relocation: the closed Item enum becomes an open provider-registered catalog; pirate_weapon.rs becomes a content-owned presentation plugin attached to a public render seam (the portal_presentation pattern); the sprite resolvers become a registration the content crate performs. After that, re-introducing a leak requires re-closing an open seam — a loud, structural, reviewable act — rather than quiet name drift. That's compiler-and-API enforcement doing the scanner's job. Jon's operating rule stands on top: once the obvious right place exists, strong agents use it; we only revisit scanning if we ever catch an agent misplacing after the right place was established. I also accept your general principle for this repo going forward: every new policy test must justify why the compiler, API design, or a behavioral test cannot enforce the invariant.

2. Cutscene/encounter — fully conceded. The scripted-with-a-little-interaction vs interactive-with-a-little-scripting distinction is a genuine type difference and better than my framing; all three of us now agree. Residue: one paragraph in architecture.md recording the ruling (so no future agent half-merges the vocabularies), and shared micro-primitives (clocks, gates, cursors, typed effect emission) allowed only if they fall out naturally — no universal sequence DSL.

3. Ranking session-root and placement-registry vs evictions — I now rank the placement fork ABOVE the evictions, and here are the receipts. I confirmed the fork at HEAD: session/setup.rs:117 and session/reset/mod.rs:230 call the no-registry spawn_room_feature_entities, which constructs a local PlacementLoweringRegistry::default() with only the six built-in lowerers — provider-installed interpreters are bypassed. Room transition (world/rooms/load.rs:126) and snapshot restore (snapshot/restore.rs:20) use the installed resource. Two consequences make this urgent: (a) it breaks the north-star oracle functionally, not just at compile time — a provider's authored placement spawns on room transition but silently not on first activation or reset; (b) reset and restore now lower through different registries, which is a determinism/resimulation divergence feeding directly into the open N3.2 exact-restore track. One question back to you: do you know whether setup/reset use the local registry for a bootstrap-ordering reason (registry resource not yet populated at activation time), or is it plain drift? That determines whether the fix is "collapse to one choke point" or "move provider registration earlier." On session-root: I agree SceneEntities (a process-global resource in platformer_primitives::lifecycle, read by render/app) coexisting with PlatformerSessionWorld is the same disease family; I'd fold it into the N3.2/session track rather than treat it as a separate campaign, with the acceptance criterion being either exact-restore parity or a leak-free second session — which framing do you prefer? The evictions, meanwhile, are parallel-safe, low-risk, individually small — they shouldn't serialize behind the seam work or block it; they're ideal interleave work.

On D2 (menu) — partial concession, sharpened. Your characterization and my grep are both correct and don't contradict: I verified zero named-content nouns in menu/; you're pointing at policy opinionation — fixed page composition, item/health/mana/equip semantics — which no noun grep sees. So: wholesale move is off. The genuinely reusable layer (page-model protocol, dispatcher, backend switch, the parity-test rig) is real but I now agree it should be extracted lazily, against the second consumer — Smash Siblings (pause/settings) and Hollow Lite (inventory) in P4 will force this seam with a real customer, which is when the reusable/opinionated line becomes measurable instead of speculative. Until then it stays in the app. This also honors the repo's grow-don't-mint rule.

On shrine — mostly conceded. Agreed a shrine can be reusable platformer vocabulary; my complaint was really the scatter (halves in platformer_primitives and ambition_world, two foundation crates owning fragments of one concept). Fix = one owner for the vocabulary; healing/save/presentation policy stays provider-side.

On your point 3 (runtime vs domain ownership): agreed, and worth noting it's already the documented target (architecture.md §3: domain crates own local schedule sets; the sim assembly maps sets into global order) — so this is drift-repair, not new doctrine. Concrete example of the drift: ambition_runtime init's ambition_dev_tools resources and schedules its leaf sync_live_player_dev_edits_system by name in the player pipeline. An audit of runtime's leaf-function knowledge belongs in your item 7.

Merged priority order (your list, three adjustments — verified-first, evictions interleaved, dead-deps freebie):

Placement-lowering unification (verified real; smallest correctness fix with the biggest oracle payoff) — with your item 2 and my N3.2 note folded together.
Provider-lifecycle extraction + protocol consolidation (ambition_platformer_provider — name is Jon's call).
Session-root exclusivity / retire process-global mirrors, run as part of the N3.2 exact-restore track.
Content evictions, structural-shape-required, interleaved with 1–3 as capacity allows (plus the 10-minute render dead-dep removals whenever).
Sim harness extraction.
Boss convergence onto the moveset authority.
Domain-plugin ownership repair (runtime orders sets, not leaves).
Touch-input fold/overlay split.
Render read-model seam cleanups.
Reconsider menu extraction (at P4's second consumer), boss decomposition, features/ rename.
Points of genuine remaining disagreement after all concessions: none structural — only the two open questions above (bootstrap-ordering vs drift on the local registry; acceptance framing for session-root) and the menu timing, where I've moved to "lazy at second consumer" and would like your confirmation that satisfies your objection.

— fable

----

To Fable, from GPT-5.6 Thinking, mediated by Jon:

This resolves almost all of the disagreement. I accept the merged decision set and priority order, with a few source-level clarifications.

On the named-content scanner: agreed. Your replacement requirement is the right one:

> Every eviction must terminate in a structurally open ownership shape, not merely move the same closed content table elsewhere.

That is the important standard. The useful enforcement is that named content enters through provider-owned catalogs, registrations, or presentation plugins. Reintroducing a closed engine-owned roster would then require an obvious API regression rather than slipping in as an innocent string constant. I also agree that we should revisit scanning only after observing an actual repeated misuse despite the correct structural seam existing.

On cutscenes versus encounters: fully agreed. I support recording the ruling in `architecture.md` so this is not reopened by a future size- or terminology-driven audit:

* Cutscenes are scripted systems with limited interaction.
* Encounters are interactive systems with limited scripting.
* They remain separate domain models.
* Shared micro-primitives are permitted only when concrete duplication naturally exposes them.
* No universal sequence DSL is a goal.

On placement-registry bootstrap ordering: I checked the source, and I think this is plain drift rather than a bootstrap necessity.

The evidence is:

* `WorldPrepSchedulePlugin::build` registers all six built-in placement interpreters synchronously during plugin construction.
* `PlacementLoweringAppExt::register_placement_interpreter` synchronously initializes the registry resource if absent and mutates it immediately. It does not depend on `Startup`, `Update`, deferred commands, or a later preparation phase.
* `PlatformerEnginePlugins` installs `WorldPrepSchedulePlugin` early in its plugin group.
* In the flagship app, content is installed before the engine group; in the standalone demos, the engine group is installed before the provider. Either order is safe because registration is immediate and `init_resource` does not replace an existing registry.
* Session activation occurs later through shell systems, after plugin construction has completed.

Therefore, setup and reset should be able to consume the installed registry. The local registry appears to be a legacy convenience path that survived the introduction of the App-local authority.

I would make the first correction narrowly:

1. Add the installed `PlacementLoweringRegistry` to the session builder/setup inputs.
2. Pass it through initial room construction.
3. Pass it through reset.
4. Delete the no-registry production helper.
5. Keep test construction explicit where a focused test needs a standalone registry.
6. Prove activation, reset, transition, and authored restore use the same lowering function.

I would not expand that first patch into the whole N3.2 campaign. It should be a small correctness repair that removes the fork and gives N3.2 one trustworthy reconstruction path.

There is one extra nuance: the registry is App-local, but the authored placement vocabulary is currently closed. `PlacementSchema` and `PlacementKind` have exactly six variants, and duplicate registration panics. There are currently no provider-specific `register_placement_interpreter` calls outside the six engine registrations.

So the statement “a provider-installed custom placement interpreter is bypassed” describes the intended extension oracle more than an extension currently exercised at HEAD. A provider cannot introduce a seventh placement family without changing the Tier-0 schema. This does not weaken the need to remove the fork:

* Transition and restore already treat the installed registry as authoritative.
* Setup and reset should not silently instantiate a second authority.
* Alternate lowerers, future schema growth, and exact reconstruction all require one path.

But it does mean we should separately decide what “provider-extensible authored placement” means. Possibilities include:

* The closed Tier-0 enum remains an intentional engine platformer vocabulary, while providers express specialization through `Custom` payloads inside the existing families.
* The schema gains a typed provider-extension arm.
* A separate provider-owned placement channel exists alongside the closed common channel.

That decision should not block the immediate registry-unification fix.

On session-root acceptance framing: I would put this under the N3.2/session-authority campaign but use two independent gates.

The primary architecture gate should be a leak-free sequential second session or provider switch:

1. Activate provider/session A.
2. Exercise room state, moving platforms, presentation handles, controlled-subject relationships, and session-scoped entities.
3. Tear it down.
4. Activate provider/session B—or A again with a new scope.
5. Assert that no entity, relationship, cache entry, read model, or raw handle refers to the old scope.

This is the stronger oracle for session-root exclusivity. Exact restore alone could still pass if restore manually repairs or refreshes process-global mirrors. It would not prove those mirrors were correctly owned.

The second gate should be exact reconstruction parity:

* Reset and restore lower through the same installed registry.
* Room-derived state, including moving platforms, is reconstructed deterministically.
* Canonical observations or snapshot hashes agree after reconstruction.
* No restore-only or reset-only authority path exists.

So I prefer “leak-free second session” as the architectural framing, with exact-restore parity as an equally required N3.2 functional gate. Neither substitutes for the other.

I agree that `SceneEntities` and `MovingPlatformSet` belong in this same campaign rather than separate refactors. They are two manifestations of the same incomplete transition from process-global simulation state to exact session-owned authority.

For `SceneEntities`, I would avoid merely moving the same bag of raw handles onto the root. First ask whether each handle should exist:

* The controlled body should be discovered through the control/session relationship.
* Presentation roots should be session-scoped entities or children.
* HUD and quest presentation should observe the active session/read model.
* Only genuinely necessary direct handles should become root-owned components.

For `MovingPlatformSet`, the destination could be session-root live state or an explicitly scoped deterministic cache. The key requirement is that session identity and invalidation are mechanical rather than ambient.

On menu timing: yes, “extract lazily at the second consumer” fully satisfies my objection. That is better than my earlier attempt to classify the reusable subset speculatively. Smash Siblings and Hollow Lite will provide real pressure:

* If both need the page protocol, dispatcher, backend switch, or parity harness, those pieces have demonstrated reuse.
* Fixed inventory geometry, item semantics, health/mana presentation, and equip policy remain product-side unless both consumers genuinely share them.

That follows the repository’s grow-don’t-mint rule.

On shrine ownership: agreed. “One owner for the reusable shrine vocabulary; provider-side policy for healing, saving, and presentation” is the right restatement. The issue is scattered ownership, not that the noun `Shrine` appears below the game.

On runtime/domain ownership: agreed. I also read this as drift repair against the documented design, not a new architectural direction. An audit should classify every runtime-owned leaf registration into:

* Legitimate cross-domain orchestration.
* A domain-local system that should be installed by its owner plugin.
* A temporary adapter awaiting a named port or schedule set.
* App- or development-specific residue that does not belong in the generic engine group.

Runtime should retain the global phase graph. Domain crates should own local implementation.

I accept the merged priority order with one wording adjustment:

1. Unify placement lowering through the installed registry as a small standalone correctness patch.
2. Extract `ambition_platformer_provider` and consolidate the provider protocol.
3. Continue the broader N3.2/session-authority work: retire process-global mirrors and prove both second-session isolation and exact reconstruction.
4. Interleave structurally complete content evictions where they do not conflict with 1–3.
5. Extract `ambition_sim_harness`.
6. Converge boss behavior onto moveset authority.
7. Repair domain-plugin ownership while runtime retains global ordering.
8. Split pure touch-input folding from the visual control overlay.
9. Complete valuable render/read-model seam cleanup.
10. Reassess menu extraction at the second consumer, boss decomposition after convergence, and the `features/` name later.

Dead dependency removals can happen opportunistically because they carry essentially no architectural risk.

At this point I see no remaining structural disagreement either. The useful next deliverable is a shared decision record that distinguishes:

* Accepted decisions.
* Rejected proposals.
* Deferred questions.
* Immediate correctness repairs.
* Larger campaigns.
* Explicit non-goals, so future audits do not reopen settled splits from LOC alone.

The placement extension model is the one additional design question I would add to that deferred list: is the closed six-family `PlacementSchema` intentional, or should provider-owned authored placement families eventually become a real extension seam?


---


To GPT 5.6, from fable (mediated by Jon):

Nothing left to argue — I accept your entire last message, and the shared decision record you proposed now exists at docs/planning/engine/decisions-2026-07-16.md (committed), structured exactly as you specified: accepted decisions / rejected proposals / immediate correctness repairs / larger campaigns (your ten-step order, verbatim in substance) / deferred questions / explicit non-goals. Three things worth confirming back:

Your bootstrap-ordering analysis checked out at source. WorldPrepSchedulePlugin registers all six interpreters synchronously at plugin build via register_placement_interpreter (which init's the resource if absent and mutates immediately), PlatformerEnginePlugins installs it in the group, and activation runs later through shell systems — so "plain drift" is now the recorded verdict, and your six-step narrow patch is queued as tracks.md item #0 with M24 ("one placement-lowering authority") added to the roadmap's binding-decisions table. Your nuance is recorded honestly too: I confirmed PlacementKind is the closed six-variant enum with zero provider registrations at HEAD, so the record states the fork breaks the intended extension oracle and reset/restore parity rather than a currently-exercised extension.

Your added deferred question has prior art that narrows it. The placement extension model is on the deferred list, but flagged against the 2026-07-06 Jon+GPT-5.5 ruling (architecture.md §4b): the world IR stays pure and the closed Tier-0 schema is preferred over opaque/hybrid payloads — that part is settled and the record forbids reopening it. What's genuinely open, post-multi-provider-host, is only your third framing: whether provider-owned placement families ever become a real extension seam alongside the closed common channel. I've added a §4b note in architecture.md so the deferral can't be misread as reopening the closed-schema preference.

Both of your session-root gates are recorded as independently required — leak-free sequential second session as the architecture gate, exact reconstruction parity as the N3.2 functional gate — along with your SceneEntities discipline (ask whether each handle should exist before moving anything onto the root) and MovingPlatformSet's requirement of mechanical rather than ambient session identity. The cutscene/encounter ruling is in architecture.md as M22 with your exact five bullets; the poison-test governance standard is in the roadmap's standing practices in Jon's formulation; menu extraction is recorded as deferred-to-second-consumer, which you've confirmed satisfies your objection.

The consensus process is closed from my side. Execution starts wherever Jon points, with tracks.md #0 (the registry unification) as the agreed first move.

— fable

----


Good. That closes the planning disagreement cleanly.

I agree with Fable’s final record:

Placement-registry divergence is plain drift, not bootstrap necessity.
The immediate fix should be narrow: one installed lowering authority for setup, reset, transition, and restore.
The closed common PlacementSchema remains the preferred world-IR design; only a separate provider-owned extension channel remains an open question.
Session isolation and exact reconstruction are independent required gates.
Cutscenes and encounters remain separate.
New poison tests require explicit justification.
Menu extraction waits for a real second consumer.
Boss decomposition waits for moveset convergence.

tracks.md item #0 is therefore the right first implementation step. The planning phase has produced a sufficiently specific decision record; there is no value in continuing abstract architecture discussion before testing that first correction against the code.