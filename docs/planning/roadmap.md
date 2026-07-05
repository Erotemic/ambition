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

## Where we are (2026-07-05, measured)

25 workspace crates. The unification arc (P1) is COMPLETE: one body
pipeline, one damage resolver, bosses are actors, movesets subsume all
attack paths, the ability model is three-tier complete, the G-track
(mounted giant, limb actors, possession-drives-limbs) landed end-to-end,
the momentum kernel rides chains AND ordinary blocks, `ambition_runtime`
exists with the engine plugin group. The monolith (`gameplay_core`,
~95k) is measured, mapped, and mid-carve. The two review-era docs that
tracked this are archived; their every open item is re-homed in
[`tracks.md`](tracks.md) (the porting audit is in that doc's header).

## The phases

- **P1 — unification.** DONE (2026-07-05). The record lives in the
  archived reviews.
- **P2 — decomposition + doctrines (ACTIVE).** Execute
  [`engine/decomposition.md`](engine/decomposition.md) (E5-finish, E4+W,
  E1–E3, E6–E8, host/app split, navigability standard) alongside the
  doctrine slices that demos will need:
  [collision CC1–CC4](engine/collision-and-ccd.md),
  [combat CM1–CM5/CM7](engine/combat-model.md),
  [netcode N0–N1](engine/netcode.md). Exit = decomposition playbook
  exits 1–5.
- **P3 — demo wave 1: [Sanic](demos/sanic.md) + [Super
  Mary-O](demos/super-mary-o.md).** Starts the moment E5-finish lands
  (S4/S5 first — Sanic leads). Exit = both pass the doctrine exits.
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
- **Q4 (M20 confirmation):** determinism as a *same-build* contract with
  fixed-tick as an option — confirmed? (Cross-platform bit-equality stays
  a non-goal.)
- **Q5 (scoped):** online netcode stays post-1.0; local-N ships with SSB —
  confirm.
- **Q27 (new):** authoring backends — is a Tiled or Godot-scene importer
  wanted as the SECOND backend (it would harden the W-carve IR seam), and
  roughly when? ("Maybe we use some godot authoring tools" — your words;
  the IR is being shaped so this is additive.)
- **Q28 (new):** demo naming sign-off: *Super Mary-O*, *Sanic*, *Super
  Smash Siblings*, *Hollow Lite* — and are parody names the permanent
  policy for all demo content?
- **Q29 (new):** the respawn default flip (dead-stays-dead) changes every
  existing sandbox enemy to never respawn unless we author `OnRoomReenter`
  per mob row. Plan: author trash mobs as Mob (respawn), leave named/
  unique actors on the new default. Confirm the triage line (see the
  respawn slice in tracks.md).
- **Q30 (new):** for the fable-window priorities (top of tracks.md) —
  reorder if your remaining fable days should go elsewhere.

## Standing practices (unchanged)

Docs trustworthy or deleted · data-driven ECS · evaluate ecosystem crates
before custom (`bevy_ggrs`/`bevy_matchbox` join the standing list) · the
validation habit (real headless sim; BLIND feel commits) · parity harness
first, then port boldly · commit = checkpoint · wall-clock tables on
multi-phase runs.
