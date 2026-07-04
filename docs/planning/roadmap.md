# Roadmap — from Ambition to a Unity/Godot-class 2D platformer engine

**Rewritten 2026-07-03 by fable** (previous engine-chain items 1–3 are DONE via the
unification arc; this version carries the full vision so any executing agent can see
the mountain, not just the next boulder). Companion docs:
[`engine/unified-actors.md`](engine/unified-actors.md) (the actor thesis + invariants),
[`engine/architecture.md`](engine/architecture.md) (the target crate stack —
rewritten 2026-07-04), ADRs 0009/0019.

> **⇒ EXECUTION FRONT-END (2026-07-04):**
> [`../reviews/fable-review-2026-07-04.md`](../reviews/fable-review-2026-07-04.md)
> holds the current work queue (R1–R6) and the binding adjudications (ability
> model, world seam, mount authoring, profile-key collapse, boss integrate-fold).
> The 2026-07-02 review is the frozen E-log record. P1/P2 of this roadmap execute
> as that doc's R-phases. (Facts drifted since 07-03: 25 crates now —
> `touch_input` extracted; `gameplay_core` measured ~99.5k.)

---

## The ambition (Jon's words, binding)

> A primary goal of ambition should be to create a game engine on the level of
> Unity / Unreal / Godot for 2D platformers, on top of Bevy and Rust. That means
> ECS-native and centered around the idea of composition and plugins. ELEGANCE and
> BEAUTY are first-class design constraints of the codebase.

**The oracle** (unchanged): *could another platformer be built by ADDING a content
crate, without editing core?* The stretch version: a demo repo expressing iconic
games in the engine. Version 1.0 will not be the full beautiful system — but if a
large subset of the crates are reusable so writing new games is easier, that is a
win.

**How this differs from Unity/Godot** — and deliberately so: those are *editors*
first. Our identity is **the Bevy way taken seriously**: the engine is a set of
composable crates + plugins; content is a Rust crate + RON + LDtk + Yarn; the
"editor" is best-in-class external tools (LDtk, Inkscape, the rig editor) speaking
through validated data seams. We compete on *architecture and expressibility*, not
on shipping an editor binary. (If that identity is wrong, that's question Q2 below —
it changes the roadmap materially, so it needs an explicit call, not drift.)

---

## Where we are (2026-07-03, measured)

24 workspace crates. The foundation layer is genuinely engine-grade and mostly
clean (verified by the 2026-07-02 four-audit review):

| Layer | Crates (LOC) | State |
|---|---|---|
| Foundations | `engine_core` 13.7k, `platformer_primitives` 3.4k, `characters` 16.6k, `portal` 4.6k, `time` 0.3k, `input` 2.4k, `combat` 1.0k, `interaction` 0.3k, `vfx` 0.5k, `sfx`/`sfx_bank` 1.2k, `sprite_sheet` 1.7k, `entity_catalog` 0.7k, `asset_manager` 2.2k, `menu` 4.8k, `ui_nav` 0.7k, `audio` 2.6k, `cutscene` 0.4k, `gameplay_trace` 1.5k | Reusable; frame-agnostic movement core with C4 conformance harnesses; `entity_catalog` + `interaction` are the exemplary open-vocabulary shape |
| The knot | `gameplay_core` **95.5k** | The remaining monolith: actor ECS sim + combat + world/LDtk + abilities + menus/persistence/dialog, entangled |
| Presentation | `render` 10k, `portal_presentation` 6.4k | Still imports the knot (D3 cut in progress) |
| Content / app | `content` 10.6k, `app` 24.6k | Content-installed rosters (enemies/bosses) prove the pattern; app is NOT yet thin assembly (C4) |

**Capability inventory (what already works, engine-grade):** one shared body
pipeline for player/actor (and soon boss) with capability-masked ability limbs
(run/jump/dash/blink/fly/shield/ledge/dodge/wall verbs); full gravity-frame
agnosticism (C4-tested, portals, gravity zones); ONE victim-side damage resolver +
shared knockback/stagger for every body (§A2, landed); relational
factions/targeting/grudges; possession as brain transfer over one control seam;
brains behind a universal `Brain::tick`; deterministic bit-identical replay
fixtures; content-installed enemy/boss rosters; Yarn dialogue with extensible
commands; LDtk multi-world authoring with round-trip tooling; a proper-time /
world-time clock architecture (ADR 0010/0011); headless ms-fast Bevy testing
culture; a sprite/rig/music generation toolchain.

**The active arc** (execution log in the review doc): dissolve the boss island
(A1 slice 3: AS4b→AS4c, then the AD2 attack-geometry conversion), collapse the
render taxonomy to actors|props (AD1), then the D3/D4 crate cuts. That arc is
~70% done and verified honest.

---

## The demo-game matrix (the expressibility oracle suite)

Each iconic game is a *test vector*: it names the engine capabilities it stresses.
"Have" means the mechanism exists body-generically today, not that the clone is
built. Tiers = the order the roadmap earns them.

| Game | What it stresses | Status |
|---|---|---|
| **Super Mario Bros (1985)** | tile world, run/jump feel, stomp-kill, powerup state, camera scroll policy, flagpole/level-end | **Tier 1 — first clone target.** Have: movement/feel/tiles/contact damage/relational kill. Missing: powerup-as-equipment chain (C1/C2), one-way camera policy knob, level-end sequencing (cutscene crate is embryonic) |
| **MoneySeize** | precision jump feel, coin economy, door-gated room graph | **Tier 1.** Have: nearly everything (BodyWallet, doors, room graph). This is the "feel calibration" clone |
| **Celeste** | coyote/buffer/dash/wall mechanics, assist modes, room-based death/retry, moving/crumbling platforms, wind | **Tier 1.** Have: all core verbs + difficulty-assist + safe-respawn. Missing: wind/force volumes, per-room gimmick authoring pattern (the content-registered-system seam C4 wants anyway) |
| **Metroid** | item-gated traversal, connected map + camera zones, save stations, minimap | **Tier 2.** Have: abilities-as-items direction (C1/C2), shrines=saves, camera zones. Missing: engine-grade map/minimap model, door transition polish |
| **Hollow Knight** | large-world streaming, benches, currency-loss-on-death, charms/equipment, NPC ecosystem | **Tier 2.** Mostly content + scale; stresses room streaming (Q7) and the equipment layer on C1 |
| **Dead Cells** | procedural level assembly from authored chunks, weapon rosters, meta-progression | **Tier 3.** Rosters = the proven install pattern. Real gap: runtime room-graph assembly (LDtk is static today) — Q8 |
| **Smash Bros** | movesets w/ frame-accurate hitboxes, hitstun/knockback/DI, N local players, ledges, platforms | **Tier 3.** Shockingly close: A2 IS hitstun/knockback/hitstop; AD2 makes frame-driven hitboxes first-class; possession/slots exist. Missing: percent-scaling knockback as data, N-player local input routing (SlotControls is built for it), stage/stock structure |
| **Sonic** | slopes, loops, momentum physics | **Edge tier.** REAL ENGINE GAP: the collision kernel is axis-aligned swept-AABB; slopes touch its deepest assumptions — Q6 |
| **Braid** | deterministic rewind, time manipulation | **Edge tier.** Bit-identical replay + the proper-time architecture are real foundations; missing: state snapshot/restore infrastructure — rides Q4 |
| **Rain World** | procedural animation, ecosystem AI, region streaming | **Edge tier.** The bone/rig toolkit and the Brain/WorldView/memory seams point here; furthest out |

The tiers are also the honesty check: **Tier 1 requires no new engine systems** —
only the current arc finishing plus C1–C4. If a Tier 1 clone needs a core edit,
that edit is the roadmap.

---

## The phases

### P1 — finish the unification arc (ACTIVE; the review doc's execution log is the tracker)
One body, one path, no islands. Remaining: AS4b spec-parity pin → size flip → AS4c
flight-limb fold; the AD2 attack-geometry conversion (`boss_attack_damage` dies);
AD1-T1 taxonomy collapse; 3e/3f/3g (possession map dies, `BossAnim`→`CharacterAnim`,
`BossConfig`→archetype data); then A7 (perception: `WorldView` becomes the only
world-out; brains can perceive their own stagger), A8/A9 (body-generic
`FrameEvents`→SFX/VFX + anim overlays), A10/B8/B12 residuals.
**Exit criterion:** `grep`ing for `Without<BossConfig>`/player-only branches in
mechanics finds only documented POLICY, and the convergence audit in
unified-actors.md reads all-green.

### P2 — the engine/content bisection completes (C + D sections of the review)
- **C1/C2**: item catalog + `HELD_ITEMS` → content-installed rosters (the proven
  pattern). This is also the Metroid/SMB powerup chain.
- **C3**: worlds/sprites/catalog embeds → a content-installed `WorldManifest`.
- **C4**: `PlatformerEnginePlugin` bootstrap (machinery-owned plugin group with
  DEFAULT ordering baked in); content hooks for named systems; app-thinness
  boundary test. **This is the single most Unity/Godot-shaped deliverable** — it is
  the `App::new().add_plugins(DefaultPlugins)` moment for platformers.
- **D3 T2**: materialize the read-model (post-AD1 taxonomy), re-create
  `ambition_sim_view` with real meat, cut the render→gameplay_core edge.
- **D4**: `ambition_world` extraction; the LDtk **content-registered converter**
  refactor (ADR-0009-shaped) — this is the "second game ships its own world" seam.
- **D5–D7**: menu stack unification, `character_sprites` down beside
  `ambition_sprite_sheet`, dialog runtime crate, C5–C7/C9/C12 vocabulary opens.
- Then the mechanics-knot extraction stops being hard (D1–D4 were the
  pre-inversions) — re-measure before attempting (see Uncertainty U1).
**Exit criterion:** the boundary test suite enforces: app = assembly, machinery
names no content, content edits rebuild only content.

### P3 — the first proof clone (the oracle made executable)
Create `demos/demo_smb/` (or MoneySeize — Jon's pick, Q12): a complete
SMB1-shaped game as ONE content crate + a ~100-line app crate on
`PlatformerEnginePlugin`. Build it *adversarially*: every place the demo needs a
core edit files an issue tagged `oracle-violation`, and those become the P3 work
queue. Ship 2–3 levels, powerups, enemies, a flag. **This phase is cheap and
should start the moment C4 lands** — it converts the oracle from prose to CI.
**Exit criterion:** `git log --stat` for the demo touches zero machinery crates.

### P4 — capability tiers, pulled by clones (never speculative)
Work through the matrix in tier order; each capability lands ONLY when a clone
demands it (design-balance rule: knobs when use cases land):
- **Tier 1 finish:** wind/force volumes; camera scroll-policy knobs; level-end /
  cutscene sequencing; the per-room gimmick pattern (content-registered systems).
- **Tier 2:** map/minimap model; room streaming policy (Q7); save-station +
  equipment layers on C1.
- **Tier 3:** procedural room-graph assembly over LDtk chunks (Q8); N-player local
  slot routing + percent-knockback data (the Smash demo IS the moveset system's
  acceptance test).
- **Edge tier (each gated on a Jon decision):** slopes (Q6), snapshot/rewind (Q4),
  rig-runtime animation.

### P5 — engine 1.0
What 1.0 means is Q1/Q12, but the mechanical criteria are already visible:
the foundation + engine crates get semver + docs + a `cargo generate` template;
the demo repo builds against a *tagged* engine version (not path deps); the
boundary tests + C4 harness + replay fixtures are the public conformance suite;
`ambition_content`/`ambition_app` are just the first customer.

**Sequencing note for executing agents:** P1→P2 is strict (the bisection needs the
unified vocabulary). P3 starts mid-P2 (needs C4 + C1, not all of D). P4 tiers are
independent of each other. When blocked on a fork, check the Questions list — if
it's there, it's Jon's call; log and move to parallel work, don't guess.

---

## Design decisions — MADE (binding; re-litigating these needs new evidence)

| # | Decision | Why |
|---|---|---|
| M1 | **Two-port body**: controller attempts, body enforces (I3) | The seam that makes human/AI/RL/remote interchangeable |
| M2 | **One control seam**: possession = brain transfer (`Brain::Player(slot)`) | No player-centrism; any body is playable |
| M3 | **actors\|props taxonomy** (I11, AD1): no Boss/NPC/Enemy type axis anywhere | Verified: the type axis was already dead in the sim; presentation follows |
| M4 | **Relational everything**: factions/grudges/damage; hazards player-scoped nowhere | Emergent play (lure the boss into lava) falls out |
| M5 | **Frame-agnostic always** (I10): `AccelerationFrame` + C4 harness for every reaction seam | The relativity principle, made mechanical |
| M6 | **Content enters via install-time rosters/registries** (RON + `OnceLock` install), never enums in machinery | Proven 4× (enemies, bosses, specials, dialogue ids) |
| M7 | **Per-frame sprite-driven combat volumes are canonical** (AD2); the sprite-metadata pipeline drives collision/hurtbox/attack | One geometry authority; Smash-class movesets need it |
| M8 | **LDtk owns space; RON owns tuning; Yarn owns dialogue**; tools not hand-edits | Editor-grade authoring without building an editor |
| M9 | **Time domains** (ADR 0010/0011): scaled `WorldTime`, proper-time per body, `ClockScaleRequest` only | Hitstop/slow-mo/RL/multiplayer all need it |
| M10 | **No pushout** (Jon's rule): bodies are never shoved out of geometry | Correctness emerges; the OOB class of bugs dies at the root |
| M11 | **Pre-release: replace, don't bridge**; behavior is not sacred; parity harness first, then port boldly | The whole E-log validates this |
| M12 | **Reusable engine bootstrap = a Bevy plugin group with owned ordering** (ADR 0019 → C4) | Composition/plugins is the engine's identity |

## Where the plan may DEVIATE — real uncertainty (not decisions, watch-items)

- **U1 — the mechanics knot.** The "~15 inversions then it's easy" estimate for
  extracting the 30k mechanics core predates D1–D4. Re-measure outward deps when
  P2 gets there; it may want to stay one crate longer than the topology diagram
  wishes. Extraction is a means (compile time, navigability), not an end.
- **U2 — `MoveSpec` will reshape.** AD2's frame-driven volumes + the
  clip-by-phase seam will likely make the moveset *sprite-metadata-first* with
  authored windows as the fallback — the current static-window schema is v1, not
  final. Don't over-invest in authoring tools for the current shape.
- **U3 — LDtk at scale.** Multi-world merging works today; nobody has measured
  50-map worlds, editor performance, or converter extensibility ergonomics.
  D4.3 may reveal LDtk limits that force a compiled-world intermediate format.
- **U4 — Bevy churn.** Each minor version (0.18→0.19…) taxes 24 crates. The
  engine's public API should wrap what churns most (schedules, messages) —
  argument for the C4 plugin group owning ordering.
- **U5 — perception (A7) has a design tension.** `WorldView` as the ONLY
  world-out is the elegant seam, but RL training legitimately wants privileged
  observations and the boss pattern wanted three bespoke inputs (E30 added them
  to the snapshot). The seam may become `WorldView` + a *declared* privileged
  channel rather than purity.
- **U6 — the app crate is 24.6k** and 40% of it is menu machinery in the wrong
  layer (D5). Thinning it will look like it's making the engine bigger before it
  makes the app smaller. That's expected, not scope creep.
- **U7 — feel drift under unification.** A2/A1 changed how hits feel (BLIND
  commits). If Jon's feel-checks reject something, the fix is per-body
  `BodyHitFeel`-style DATA, never a re-fork of the path.

---

## QUESTIONS FOR JON — the design calls only you can make

*(Each blocks or steers a phase; none block P1. Answer in place, tersely — this
doc is the record. Agents: if your fork maps to one of these, stop and do
parallel work instead of guessing.)*

**A. Product identity**
- **Q1.** At 1.0, who is the engine FOR? (a) your games only — polish optional,
  move fast; (b) published crates others build real games on — semver, docs,
  API-stability discipline become roadmap items. This decides most of P5.
- **Q2.** Is "content = a Rust crate + RON + LDtk + Yarn" the *permanent*
  authoring story, or does 1.0 eventually want a no-Rust content path
  (pure data packs / scripting)? Current trajectory says Rust-native is the
  identity (it's the Bevy way, and it's what the oracle tests). Confirm or redirect.
- **Q3.** Does the engine eventually get its own name/repo (with the demo repo
  beside it), and roughly when — at P3 (forces honesty early) or P5 (less churn)?

**B. Simulation contracts**
- **Q4.** Is **bit-identical determinism a public engine guarantee** (enables
  rollback netcode, Braid-rewind, RL reproducibility) or an internal test canary?
  Public = f32 discipline, stable iteration order, and snapshot/restore become
  API, not hygiene. This is the biggest hidden-cost question on the list.
- **Q5.** Multiplayer scope for 1.0: none / local-N-player (the Smash demo) /
  online rollback? (Local-N looks nearly free on SlotControls; rollback rides Q4
  and is a different animal.)
- **Q6.** **Slopes.** Committing to the bespoke swept-AABB kernel is currently
  implicit. Are slopes/curved terrain (Sonic-class, even Celeste-lite ramps)
  in the engine's 1.0 capability set? If yes, that's a deep, planned kernel
  extension (axis-role sweeps must generalize) — better scheduled deliberately
  than discovered. If no, we say so in the engine docs and Sonic leaves the matrix.

**C. World & scale**
- **Q7.** Metroid/HK-scale worlds: is **room streaming** (load-ahead, seamless
  camera handoff) an engine capability, or do we commit to rooms+transitions as
  the model and make transitions gorgeous instead?
- **Q8.** Is **procedural level assembly** (Dead Cells: stitch authored LDtk
  chunks at runtime) in scope for the engine, or a game-side concern built on a
  chunk-loading API we expose?

**D. Presentation & tooling**
- **Q9.** Is the **Python toolchain** (sprite renderer, rig editor, music
  renderer) part of the engine *product* (documented, fresh-clone reproducible,
  versioned with it) or Ambition-internal tooling? It's a genuine differentiator
  if productized, and a support burden too.
- **Q10.** Map/minimap: engine-grade system (Tier 2 wants it) or per-game UI?
- **Q11.** The menu/settings IR stack (post-D5): engine offering or app-layer
  reference implementation?

**E. Acceptance**
- **Q12.** Which clones are the **1.0 acceptance suite** vs aspirational? My
  proposal: SMB1 + MoneySeize + Celeste-slice = acceptance (Tier 1); Metroid-slice
  = stretch; everything else post-1.0. And: which is the FIRST demo (P3) — SMB1
  (broadest recognition) or MoneySeize (closest to current feel work)?

---

## Standing practices (unchanged, folded from the old roadmap)

- **Docs are trustworthy or deleted.** This plan included — the review doc's
  execution log is the ground truth for P1/P2 status.
- **Data-driven ECS; LDtk owns space; RON owns tuning/audio; tools not hand-edits.**
- **Evaluate ecosystem crates before rolling custom; document rejections.**
  Standing candidates when their use case lands: `bevy_asset_loader`, `bevy-tnua`,
  `big-brain`/`dogoap`, `vleue_navigator`. Don't adopt speculatively.
- **The validation habit** — a change isn't done until the real headless sim
  exercised it; feel-touching changes ship BLIND in marked commits for Jon.
- **Parity harness first, then port boldly; commit = checkpoint.**
