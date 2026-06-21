# Sprite/actor intent contract — holistic plan

*Author: Claude Opus 4.8 (1M) · 2026-06-21 · status: PROPOSAL (thinking, not started)*

This started as "make the door sprite stand on the ground" and grew into a
direction for `tools/ambition_sprite2d_renderer` (~72k LOC, the weakest part of
the game). After a round of Jon feedback the center of gravity moved, so this
doc is rewritten around the real prize.

## What this is actually about (revised after Jon's feedback)

Not LOC reduction. Not forcing every sprite onto bone rigs. The prize is:

> **Correct, rich RON output so the game can react to the *intent* of a sprite.**

Authoring stays **plural** — drawers, imperative PIL, YAML adapters, and bone
rigs are all legitimate ways to make pixels, each with its own charm. Bone rigs
are *one* nice, maintainable method, never mandatory. **No authoring path is
made legacy/dead until a replacement exists that produces a sprite Jon likes
*more* than the original.** Code shrinkage is only ever a side effect of de-
duping the shared spine.

The point of the rich contract is **expressiveness**: when sprites declare their
intent as data, generic game systems can do interesting things with them that
are impractical to hand-code per entity. The marquee example — the thing Jon
explicitly wants and doesn't yet know how to improve — is **boss attack
patterns.**

## The organizing principle

The door floated because its geometry was an accident of where pixels landed and
a constant welded it to its trigger box. The fix let geometry **emerge from the
pixels** (feet = lowest opaque row; the runtime plants it). Generalize:

> **Declare intent as data; let generic systems react. Measure what the pixels
> can express; author only what they can't.**

This applies at **two layers that meet**:

1. **Sprite/actor intent** *(renderer → game)* — measured: feet/contact row,
   body extent, per-frame hurt/hit windows, ground flag. Declared (pixels can't
   express): **sockets** (muzzle / hand_l / hand_r / weapon_tip / mouth), an
   explicit collision box distinct from the silhouette, hit-active frame
   windows, semantic kind/role.
2. **Behavior/attack intent** *(content RON → game)* — declared: projectile
   emitter patterns, telegraph/commit/recovery beats, attack-selection policy.

They meet at **sockets + frame windows**: the renderer says *where* (hand_r at
this pixel each frame) and *when* (active frames 4–7); the behavior layer says
*what fires from there* (a 7-shot aimed fan). Today the renderer **already
emits sockets — into `*_actor.ron`, which nothing reads.** Un-orphaning that
file is the single change that unlocks data-driven attacks.

## Current state (verified by deep-dives, 2026-06-21)

**Sprite metadata is split and drifts.** `body_metrics` (body bbox,
`feet_anchor_norm`, per-anim hit/hurtboxes) is measured and read — good. But
`collision_scale`/render-size is a Rust constant despite a standing
`TODO(gen2d-collision-aware)` (it's derivable from the measured body fraction);
the richest model (`*_actor.ron`: explicit collision, hurtbox, **sockets**,
anim event bindings) is **authored but orphaned**; and **bosses hard-code
`BossSheetSpec`** rows/anchor/scale that the RON also carries — silent drift on
regen.

**Boss attacks are ~half data-driven, and that's the ceiling.** The good half:
`boss_profiles.ron` authors per-phase telegraph→strike→rest timelines, movement
personality, damage scalars; per-frame melee hit/hurtboxes come generically from
sprite metadata. The wall:
- **Every projectile/spatial pattern is bespoke Rust** — 11 systems in
  `crates/ambition_content/src/bosses/specials/*.rs`, all params as `const`
  (`MC_RING_COUNT=12`, `SHOT_SPEED=780`, …). **Not one projectile number lives
  in data.** Adding a bullet ring/fan/wall/spiral/aimed-volley = a new Rust
  system + state component + plugin registration.
- `BossAttackProfile` is a closed enum and `volumes_for_profile` a closed match
  (melee shapes are hard-coded).
- Attack **selection never reads the player** — it's a fixed scripted loop; only
  movement reacts. No reactive choice, no multi-stage/conditional chaining.

The runtime that *executes* attacks is already generic and reusable: the
`Effect::Projectiles` executor, `EnemyProjectileSpawn`, the per-frame sprite-
hitbox sampler, and the `BossPatternStep` schedule. The gap is purely a **data
vocabulary** + one interpreter to replace the 11 bespoke systems.

**Authoring sprawl** (context, not the target): 4 paradigms, the
supersample→crop→assemble→emit spine duplicated ~3× (two RON emitters), helpers
reimplemented ~21×. The bone/rig system (`skeleton.py` FK+IK+keyframes+painters,
`rigdoc` + GUI + codegen bridge) is sound but used by only 3 of ~50 targets;
Emmy/Noether is a ~148-line character. It's a good front door for *new* work,
not a mandate for old work.

## A concrete attack-intent vocabulary (the expressiveness payoff)

Make a boss attack a **data block**, interpreted by one generic system. Sketch
(RON-ish, names illustrative):

```
Attack(
  id: "gnu_aimed_fan",
  anim: "hand_slam",                 // telegraph + active frames come from sprite metadata
  windows: [
    Emit( at: ActiveStart, socket: "hand_r",
          emitter: Fan(count: 7, spread_deg: 60, speed: 600, gravity: 0.0,
                       half_extent: (6,6), damage: 10),
          aim: AtPlayer ),           // Fixed(dir) | AtPlayer | Lead | Sweep(a,b)
    Melee( window: Frames(4..7), shape: FromSprite ),
  ],
  recovery: Secs(0.4),
)
```

Emitter shapes compose: `Single | Fan{count,spread} | Ring{count} |
Wall{count,spacing,axis} | Spiral{count,turn} | Burst{count,interval}`, each ×
an aim mode. This single vocabulary expresses every one of the 11 bespoke
specials as data, plus combinations they can't currently do (a *sweeping ring*,
an *aimed wall*, a two-window telegraph→punish).

And the selection policy — the "every boss a failed objective function" hook —
becomes data the brain evaluates over game state:

```
Selection(weighted: [
  (attack: "aimed_fan",  when: PlayerRange(Far),   weight: 2),
  (attack: "floor_slam", when: PlayerRange(Near),  weight: 3),
  (attack: "ring_nova",  when: Phase(Enrage),      weight: 1),
])
```

The renderer supplies `socket "hand_r"` per frame and the `ActiveStart` window;
the content RON supplies the emitter + aim + selection. New, interesting attacks
become a RON edit, not a Rust system.

## Pillars

- **A — The intent contract (the heart).** One sprite/actor manifest:
  measure-by-default, declare the exceptions, **un-orphan the sockets +
  collision + anim-event fields** so the game reads them. Derive
  `collision_scale` from the body fraction (`TODO(gen2d-collision-aware)`).
  Retire the Rust sprite constants and the `BossSheetSpec` drift. Generalize the
  door's grounded-render path to be **data-driven** (a `ground`/`feet` intent in
  the manifest), not a hard-coded `matches!(Door)`.
- **B — One rendering spine.** De-dupe the supersample→crop→assemble→emit
  pipeline and the ~21 helper copies behind a parity harness. Low-risk support
  work; *not* the headline, *not* about LOC.
- **C — Authoring stays plural.** Every method (drawer / imperative / YAML /
  rig) emits the full contract. Bone rigs are the recommended front door for new
  characters/props and the place to invest (richer part vocabulary, de-hardcode
  biped, multi-view) — but **migration is opt-in and taste-gated**, never forced.
- **D — Expressive boss attacks (the marquee consumer).** The data-driven
  emitter + aim + selection vocabulary above; one generic interpreter collapsing
  the 11 bespoke specials; open the melee enum to data shapes. This is what
  makes the contract *worth it*.
- **E — Prune, taste-gated.** Delete a path only after a replacement Jon likes
  more exists. Charm is preserved by default.

## Sequencing — safety net, then a thin vertical slice, then broaden

0. **Harness + drift guard.** Low-res (`scale`) render-hash per target (also
   unblocks fast tests) + a Rust-side parse test for the unified manifest
   (Python RON writers are looser than Rust's `ron`). Nothing else proceeds
   first.
1. **Vertical slice — one richer boss attack, authored as data.** Un-orphan
   sockets for *one* boss (Pillar A, minimal), add the emitter/aim schema +
   interpreter for *one* pattern (Pillar D, minimal), and author a new attack
   (e.g. an aimed fan with a real telegraph) replacing one bespoke special.
   Proves both contracts end-to-end and gives an immediate "attacks are more
   interesting" win **before** committing to breadth. (Vertical-slice-first +
   harness-first matches how this repo de-risks big work.)
2. **Broaden the contract (A) + spine (B).** Unify the manifest, derive
   collision, kill boss-spec drift, data-drive grounded sprites (closes the door
   follow-up: the slab's *measured* foot is the contact row; stoop/frame become
   declared parts, not a gold sill sitting on the floor).
3. **Broaden attacks (D).** Collapse the remaining specials into the vocabulary;
   add reactive/selection policy and multi-window attacks.
4. **Invest in authoring (C) + prune (E)** opportunistically, taste-gated.

## What is explicitly NOT a goal
- Not deleting imperative/charming sprites for being old.
- Not forcing bone rigs everywhere.
- Not LOC golf. Expressiveness and a drift-proof contract are the goals; less
  code is a welcome side effect of de-duping the spine.

## Open questions for Jon
1. **Vertical slice target:** which boss/attack should the slice prove on? (A
   projectile boss with clear sockets — gnu_ton hand_slam → aimed fan? FSM ring?)
2. **Where does attack-intent data live:** extend `boss_profiles.ron`, or a new
   per-attack RON library shared across enemies (not just bosses)? *(Lean:
   shared library — regular enemies want emitters too.)*
3. **Manifest format:** RON-only (game-native, strict parser = free drift guard)
   + a generated human-readable report, dropping the YAML/`actor.ron` siblings?
4. **How far to push selection-as-objective-function** now vs. later (weighted/
   reactive is easy; true "failed objective function" framing is a bigger idea).
