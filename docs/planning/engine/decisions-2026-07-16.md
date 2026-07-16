# Decision record — 2026-07-16 recon consensus

**Parties:** fable (recon, [`recon-2026-07-15.md`](recon-2026-07-15.md)) and
GPT-5.6 (independent review), **mediated and ratified by Jon, 2026-07-16.**
This record supersedes the recon doc's §6 decision table. Its purpose is that
future audits do not reopen settled questions from LOC counts or terminology
alone. Every claim below that is source-level was verified at HEAD before
recording (file:symbol anchors, no line numbers).

## 1. Accepted decisions

1. **Content evictions (recon A1–A9), with the structural-shape requirement:**
   every eviction must TERMINATE in a structurally open ownership shape —
   a provider-owned catalog, registration, or presentation plugin — never a
   relocated closed table. The closed `ambition_items::Item` enum becomes an
   open provider-registered catalog; `ambition_render`'s named-content modules
   (`pirate_weapon`, `deep_dream`) become content-owned presentation plugins on
   a public render seam (the `portal_presentation` pattern);
   `sprite_sheet::game_assets` binding becomes content-performed registration.
   Reintroducing a leak must require re-closing an open seam (a loud API
   regression), not a string constant.
2. **`ambition_platformer_provider`** (working name; Jon may shorten): extract
   `crates/ambition/src/provider.rs` into its own crate AND consolidate the
   provider protocol — the `prepare_session`/`activate_session` pair is
   near-verbatim across all four providers (~100 LOC each). The `ambition`
   facade returns to pure re-exports.
3. **`ambition_sim_harness`**: extract `game/ambition_app/src/rl_sim/` (the
   reset/step/typed-action/observation gym seam) below the demo gate, with the
   app passing its plugin composition in through a builder seam.
4. **Cutscene/encounter separation** (M22, recorded in architecture.md):
   cutscenes are scripted systems with limited interaction; encounters are
   interactive systems with limited scripting. Separate domain models. Shared
   micro-primitives (clocks, gates, cursors, typed effect emission) only when
   concrete duplication naturally exposes them. No universal sequence DSL.
5. **Host registration stays explicit.** The two-line provider registration in
   `ambition_app` (`Cargo.toml` dep + the `shell_host.rs` plugin tuple) is the
   documented, intentional exception to the additive-content oracle. No opaque
   plugin discovery. Revisit only when a fifth+ provider makes it hurt.
6. **Boss carve deferred** until boss attacks/phase behavior converge onto the
   canonical moveset/action path; reassess whether a standalone crate is still
   justified after convergence.
7. **Menu-host extraction deferred to the second consumer** (P4: Smash
   Siblings pause/settings, Hollow Lite inventory). The app `menu/` directory
   is part reusable host machinery (page protocol, dispatcher, backend switch,
   parity harness), part opinionated Ambition/OoT inventory product (fixed
   layout, item/health/mana/equip policy). Real reuse pressure draws the line;
   speculative classification does not. E1e stands until then.
8. **Shrine:** one owner for the reusable shrine VOCABULARY (today it is
   scattered: `platformer_primitives::shrine` + `ambition_world` `ShrineSpec`);
   healing/save/presentation policy is provider-side. A named mechanic below
   the game is not automatically a content leak.
9. **Runtime/domain ownership is drift-repair** against architecture.md §3
   (domain crates own local sets/systems; the sim assembly orders sets).
   Audit every runtime-owned leaf registration into: legitimate cross-domain
   orchestration / domain-local (move to owner plugin) / temporary adapter
   awaiting a named port / app- or dev-specific residue. Known example of the
   drift: `ambition_runtime` init's `ambition_dev_tools` resources and
   schedules `sync_live_player_dev_edits_system` by name.
10. **Session-root campaign gates (two, independent, both required):**
    (a) ARCHITECTURE gate — leak-free sequential second session / provider
    switch: activate A, exercise, tear down, activate B (or A re-scoped),
    assert nothing (entity, relationship, cache, read model, raw handle)
    refers to the old scope; (b) N3.2 FUNCTIONAL gate — exact reconstruction
    parity: reset and restore lower through the same installed registry,
    room-derived state reconstructs deterministically, snapshot hashes agree,
    no restore-only or reset-only authority path. Neither substitutes for the
    other. `SceneEntities` and `MovingPlatformSet` fold into this campaign;
    for `SceneEntities`, first ask whether each handle should EXIST (derive
    via control/session relationships, session-scoped presentation roots,
    read-model observation) — do not move the bag of raw handles onto the
    root wholesale.
11. **Poison-test/policy governance (standing practice, Jon's ruling):** every
    NEW policy test must justify why the compiler, API design, or an ordinary
    behavioral test cannot enforce the invariant. Existing tests guarding
    concrete historical regressions remain. The repo stops growing a parallel
    static-analysis language of source scans and poison fixtures.

## 2. Rejected proposals

1. **The A0 named-content scanner** (recon §1). The structural-shape
   requirement (accepted #1) replaces it. Revisit ONLY on observed evidence of
   an agent placing content in the wrong place AFTER the obviously-correct
   seam existed — do not go searching for that evidence.
2. **Folding cutscene onto the encounter timeline** / any universal sequence
   DSL (see accepted #4).
3. **Wholesale move of the app `menu/` directory** into an engine crate (see
   accepted #7).
4. **Opaque/dynamic provider discovery** for the shared host (see accepted #5).

## 3. Immediate correctness repairs (small, standalone; not campaigns)

1. **Placement-lowering unification.** Verified fork at HEAD: session setup
   (`ambition_actors::session::setup`) and session reset
   (`session::reset`) lower placements through a LOCAL
   `PlacementLoweringRegistry::default()` built inside
   `features::ecs::spawn::spawn_room_feature_entities`, while room transition
   (`world::rooms::load`) and snapshot restore (`runtime::snapshot::restore`)
   use the App-installed registry that `WorldPrepSchedulePlugin` populates
   synchronously at plugin build. This is plain drift, not a bootstrap
   necessity (registration is immediate; activation runs later; install order
   is safe both ways). The narrow patch: (1) add the installed registry to the
   session builder/setup inputs; (2) pass it through initial room
   construction; (3) pass it through reset; (4) DELETE the no-registry
   production helper; (5) keep explicit standalone registries in focused
   tests; (6) prove activation, reset, transition, and authored restore use
   the same lowering function. Do NOT expand this patch into the whole N3.2
   campaign. Nuance, recorded honestly: no provider registers a seventh
   interpreter at HEAD (the closed six-family schema — see deferred #1), so
   the bypass breaks the intended extension oracle and reset/restore parity
   rather than a currently-exercised extension.
2. **Render dead-dep removals** (`ambition_interaction`, `ambition_input`,
   vestigial optional `leafwing-input-manager`) — opportunistic, anytime,
   essentially zero architectural risk.

## 4. Larger campaigns — agreed priority order

1. Placement-lowering unification (repair #1 above — runs first).
2. Extract + consolidate `ambition_platformer_provider` (accepted #2).
3. N3.2/session-authority: retire process-global mirrors; prove BOTH gates of
   accepted #10.
4. Structurally-complete content evictions (accepted #1), interleaved with
   1–3 wherever they do not conflict — they are parallel-safe filler work.
5. Extract `ambition_sim_harness` (accepted #3).
6. Converge boss behavior onto the moveset/action authority.
7. Domain-plugin ownership repair; runtime retains the global phase graph
   (accepted #9).
8. Split pure touch-input folding from the visual control overlay.
9. Render/read-model seam cleanups (recon C2–C5, C6's policy included).
10. Reassess: menu extraction (at the second consumer), boss decomposition
    (after convergence), the `features/` rename (deferred #2).

## 5. Deferred questions

1. **The authored-placement extension model.** `PlacementSchema`/
   `PlacementKind` is a closed six-family vocabulary and duplicate
   registration panics; providers cannot add a family without editing Tier 0.
   Options on the table: closed enum stays intentional (providers specialize
   WITHIN families), a typed provider-extension arm, or a separate
   provider-owned placement channel. **Prior art that this deferral does NOT
   reopen:** the 2026-07-06 Jon+GPT-5.5 ruling (architecture.md §4b) — the
   world IR stays pure, and the CLOSED Tier-0 schema is preferred over
   opaque/hybrid payloads. What is genuinely new since that ruling is the
   multi-provider host: the open question is only whether provider-OWNED
   families ever become a real extension seam. Does not block repair #1.
2. **R6e `features/` rename.** Jon: maybe; unsure `sim` is the right name
   (possibly too broad); low priority either way. Name it only when the
   subtree's ownership can be stated precisely. A module-only half-rename
   remains forbidden.
3. **The provider crate's exact name** (`ambition_platformer_provider` vs
   shorter) — Jon's call at extraction time.
4. **The menu reusable/product line** — drawn by the second consumer, not in
   advance (accepted #7).
5. **Boss crate justification** — re-asked after convergence (accepted #6).

## 6. Explicit non-goals (do not reopen from LOC/terminology alone)

- **No further `ambition_actors` crate split** (2026-07-10 ruling stands; the
  leaf modules are the existing residue queue, not a new carve).
- **No `ambition_engine_core` split** (measured coherent kernel; ~5s rebuild).
- **No sim_view types/builders inversion** (considered and rejected: splits
  types from their builders; the pull model quarantines the actors dep in
  exactly one crate).
- **No audio-triple merge** (`sfx_bank`/`sfx`/`audio` layering is justified).
- **No universal sequence DSL** across cutscene/encounter/dialogue.
- **No named-content scanner; no new parallel static-analysis language**
  (accepted #11 is the governing practice).
- **No opaque provider discovery** for the shared host.
