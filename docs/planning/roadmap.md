# Roadmap — the phases, the registers, and Jon's open calls

**Rewritten 2026-07-05 by fable** as part of the planning consolidation:
`docs/planning/` is now the ONE source of truth ([`vision.md`](vision.md)
is the top; [`tracks.md`](tracks.md) is the live queue; the review docs
that used to front-end execution are archived in `docs/archive/reviews/`).
This doc holds the phase map, the demo matrix, the binding
decision/uncertainty registers, and the questions only Jon can answer.

**The ambition (Jon's words, binding):** a Unity/Unreal/Godot-class 2D
platformer engine on Bevy + Rust — ECS-native, composition and plugins,
ELEGANCE and BEAUTY as first-class constraints. **The oracle:** *could
another platformer be built by ADDING a content crate without editing
core?*

---

## Where we are (2026-07-09, measured)

42 workspace crates. **P1 (unification) and the P2 decomposition are both
COMPLETE.** One body pipeline, one damage resolver, bosses are actors,
movesets subsume all attack paths, the G-track landed end-to-end, the
momentum kernel rides chains AND ordinary blocks. The monolith is gone:
`ambition_runtime` (headless sim assembly) + `ambition_host` (windowed
wiring) + the `ambition` umbrella crate are real, `ambition_render` no
longer depends on the actor sim, `placements` is the sole authored-entity
channel, and two demo crates dep only the umbrella. Gate: 44/44 suites
green.

**What P2 still owes:** the netcode ladder's N0 rungs are DONE (N0.1 fixed
tick, N0.2 input stream, N0.3 determinism lints — 2026-07-09); N0.4 (the
desync canary) and N1 remain; collision CC3 (the fuzz oracle delta) is open;
the D-B `MODULES.md`
standard and the D-C mode-scope seam are unstarted; and one playbook exit
remains (exit 3 — a demo binary, which fable ruled interactive work). Details
+ drift findings:
[`tracks.md`](tracks.md).

## The phases

- **P1 — unification.** DONE (2026-07-05). The record lives in the
  archived reviews.
- **P2 — decomposition + doctrines (ACTIVE; decomposition DONE).**
  [`engine/decomposition.md`](engine/decomposition.md) is executed through
  E9 + the F-queue; what remains are the doctrine slices the demos need:
  [collision CC1–CC4](engine/collision-and-ccd.md) (CC1/CC2/CC5/CC6 done;
  **CC3 open**), [combat CM1–CM5/CM7](engine/combat-model.md) (all done), and
  [netcode N0–N1](engine/netcode.md) (**N0.1/N0.2/N0.3 done 2026-07-09;
  N0.4 + N1 remain**), plus D-B/D-C. Exit = decomposition playbook exits 1–5,
  of which only exit 3 (a demo binary) is still open.
- **P3 — demo wave 1: [Sanic](demos/sanic.md) + [Super
  Mary-O](demos/super-mary-o.md).** **UNBLOCKED — E5-finish landed
  2026-07-06 night (the demo gate is open; the shell smoke test is the
  reference assembly).** S4/S5 first — Sanic leads. Exit = both pass the
  doctrine exits.
- **P4 — demo wave 2: [Super Smash
  Siblings](demos/super-smash-siblings.md), then [Hollow
  Lite](demos/hollow-lite.md).** SSB pulls CM6 + N1 + FB1–FB4; Hollow
  Lite pulls the boss pipeline BD1–BD8 + respawn policy. Exit = SSB's
  verbatim exit criteria + Hollow Lite's fun-verdict exit.
- **P5 — relativity & the long game.** Slower-light L1–L4 (Tier-0 seams
  ride E4 in P2), moving/angled portals CC6–CC7, online netcode N2–N3,
  the further matrix tiers (MoneySeize, Celeste-slice, Metroid-slice,
  Braid-slice, Dead-Cells-slice), engine naming/semver/template (Q1/Q3),
  the docs-refresh track (mechanics/concepts/systems brought current —
  [opus], safe once this planning stack is the north star).
- **Ambition-the-game** runs through all phases as the first customer
  (hall, zones, story beats consume each engine capability as it lands)
  and gets its full build-out on the finished engine — the north star
  ordering unchanged.

## The demo-game matrix (test vectors; the four bolded are written in stone)

| Game | Stresses | Status |
|---|---|---|
| **Sanic** | momentum kernel, speed-rewarding level design | P3; furthest along |
| **Super Mary-O** | classic AABB baseline, powerup-equipment, scroll policy, sequencing | P3 |
| **Super Smash Siblings** | N bodies/N slots, movement-identity coexistence, full combat stack, match state outside core | P4 |
| **Hollow Lite** | exploration loop, boss-quality pipeline, respawn policy | P4 |
| MoneySeize | precision feel, coin economy, door-gated graph | P5 tier |
| Celeste-slice | assist modes, wind/force volumes, room gimmick pattern | P5 tier |
| Metroid-slice | item-gated traversal, map/minimap (Q10), saves | P5 tier |
| Braid-slice | snapshot/rewind (rides netcode N3.1) | P5 tier |
| Dead-Cells-slice | runtime room-graph assembly (Q8) | P5 tier |
| Rain-World-slice | rig-runtime animation, ecosystem AI | far edge |

## Decisions — MADE (binding; M1–M12 carried forward, new below)

M1 two-port body · M2 one control seam (possession=brain transfer) ·
M3 actors|props, no type axis · M4 relational everything · M5
frame-agnostic always (C4) · M6 content via install-time registries ·
M7 sprite-metadata combat volumes canonical · M8 LDtk owns space, RON
tuning, Yarn dialogue, tools not hand-edits · M9 time domains ·
M10 no pushout (sole exception: portal-close eviction) · M11 pre-release
replace-don't-bridge · M12 runtime = plugin group owning ordering.

New (2026-07-05):

| # | Decision | Where |
|---|---|---|
| M13 | **The sweep law** — path-dependent state changes evaluate swept, never sampled | [collision-and-ccd.md](engine/collision-and-ccd.md) §1 |
| M14 | **Blocks are surfaces** — a solid's boundary is a chain; AABB is the fast special case, not a privileged ontology | landed `0189338b` |
| M15 | **One damage axis, two death policies** (HpDepleted / Unbounded+blast) — percent and HP are the same meter | [combat-model.md](engine/combat-model.md) §1 |
| M16 | **Wearing is possession semantics** — the worn character's authored kit IS the kit; no fallback overlay | landed `0189338b` |
| M17 | **The no-cheat brain contract** — difficulty never reads privileged state or scales damage; skill = prediction + option quality under human-rate constraints | [fighter-brain.md](engine/fighter-brain.md) |
| M18 | **Fight quality is measured** — telegraph grammar + validator + playtester metrics gate boss authoring; taste passes are banked as data | [boss-design.md](engine/boss-design.md) |
| M19 | **Demo rules are mode-scoped plugins** — ambition hosts every demo; global-state demo rules are a design bug | [demos/README.md](demos/README.md) |
| M20 | **Determinism is a managed same-build contract** (fixed-tick option, input streams, desync canary); cross-platform float determinism is NOT promised | [netcode.md](engine/netcode.md) N0 — *flagged Q4 below for Jon's confirmation* |

## Uncertainty register (watch-items, updated)

- **U1 (stands):** the post-carve `ambition_actors` may or may not split
  further — re-measure, don't pre-commit.
- **U3 (stands):** LDtk at scale; W4/ADR-0021 keeps the backend swappable.
- **U4 (stands):** Bevy churn taxes 25 crates; the runtime group owning
  ordering is the shield.
- **U5 (resolved by policy):** shipped brains read `WorldView` only (M17);
  privileged channels are RL-research-only.
- **U7 (stands):** feel drift under unification — fixes are per-body DATA.
- **U8 (new):** L3 rollout cost (fighter brain) — budgeted + degradable by
  design; if snapshot cost makes it infeasible, L2+reads is still level ~7.
- **U9 (new):** the scoped-mode pattern vs. deep global systems (audio
  buses, save files) when a demo runs INSIDE ambition — expect a small
  "host services" contract to emerge; design it when the first wing lands.

## QUESTIONS FOR JON (open; answer tersely in place)

- **Q1/Q3 (carried):** who is 1.0 for; engine name/repo split timing.
- **Q2-name (carried):** endorse `ambition_actors` as the gameplay_core
  residue rename, or supply a name (E7 blocks on nothing else).
- **Q4 (✅ ANSWERED 2026-07-06):** determinism = **level 2, same-build
  now, cross-platform later.** Same binary/platform/input-stream is
  deterministic enough for tests/replay/desync canaries; cross-platform
  bit-exactness is not promised now but the architecture must not preclude
  it (guardrails: stable iteration, seeded RNG streams, no wall-clock in
  sim, no hash-order semantics, portable snapshot/input formats). Recorded
  in [`engine/netcode.md`](engine/netcode.md) §Q4-RESOLVED + N0.
- **Q5 (scoped):** online netcode stays post-1.0; local-N ships with SSB —
  confirm (implied by the 2026-07-06 SSB scope ruling: "no multiplayer on
  the first round" = no ONLINE; up to 4 local fighters incl. a second
  local controller when natural).

### Answered 2026-07-06 (Jon)

- **Q27 — authoring backends: DEFERRED** until truly needed. LDtk +
  parameterized generator entities suffice: bake quarter-circle
  **`SurfaceRamp`** entities for floor↔wall transitions of momentum
  bodies (four could compose a loop; `SurfaceLoop` stays for real loops).
  Slice lives in tracks.md; the W-carve still keeps the IR
  backend-agnostic so a future importer stays additive.
- **Q28 — parody names are POLICY** for all demo content. Names stand:
  *Sanic, Super Mary-O, Super Smash Siblings, Hollow Lite*.
- **Q29 — respawn triage line CONFIRMED** (trash mobs author respawn;
  named/unique actors take dead-stays-dead).
- **Q30 — fable-window order CONFIRMED**, with the addition (Jon): the
  hardest decompositions themselves are fable work — E4 is re-graded
  [★fable executes]. *(Historical: the fable window CLOSED 2026-07-06
  night with E4+E5 executed; the W3/E2 escalation valve is retired —
  E2's back-edges are pre-classified and W-a..W-e are ruled; the
  post-fable protocol is in tracks.md.)*
- **Q31 — the W3 vocab-arrow / authored-placement model RULED (Jon +
  GPT-5.5).** World IR stays PURE (zero runtime character/combat/demo
  deps); authored maps still declare content (spawns, the falling-sand
  spout) as **authored placement RECORDS over closed Tier-0 SCHEMAS**
  (preferred over opaque payloads); a **world→sim lowering seam**
  interprets records into behavior (sim/content → world, never reverse);
  the **base+delta seam for permanent world change is RESERVED** (the
  world is not immutable). Canonical: [`engine/architecture.md`](engine/
  architecture.md) §4b + Tier-0 note. The [W-a..W-e] sub-questions were
  all ruled 2026-07-06 night and the W-track is EXECUTED; `placements` is
  now the sole authored-entity channel (the F9.2 arc, 2026-07-09).
- **Q32 — SimView IS the observation boundary (Jon).** Presentation
  migrates toward SimView/observation facts, not raw sim reads;
  architectural churn is ACCEPTED when it removes long-term coupling
  (the long game). The E4 full dep-flip is blessed to proceed on this basis.
- **Q33 — tuning is not a planning blocker (Jon).** DI/quality/slope/
  brain-weight/visual-default values are knobs → data/playtest work,
  shipped BLIND; escalate only if the KNOB is missing. Recorded as a
  decision principle ([`decision-principles.md`](decision-principles.md)).

## Standing practices (unchanged)

Docs trustworthy or deleted · data-driven ECS · evaluate ecosystem crates
before custom (`bevy_ggrs`/`bevy_matchbox` join the standing list) · the
validation habit (real headless sim; BLIND feel commits) · parity harness
first, then port boldly · commit = checkpoint · wall-clock tables on
multi-phase runs.
