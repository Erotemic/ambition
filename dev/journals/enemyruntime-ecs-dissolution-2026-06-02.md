# EnemyRuntime → ECS dissolution: lockstep parity makes big ports fast + safe (2026-06-02)

## What happened

Dissolving the legacy `EnemyRuntime` blob (held in `ActorRuntime::Enemy`,
shadowed by one-way mirror components) into authoritative ECS cluster
components, mirroring the player cluster pattern. In one session the
hard/risky core landed with **zero regressions**: the cluster components +
`EnemyMut<'a>` view, and the full physics/AI integration (`update` +
surface-walker + `wall_ahead`/`snap_pos_to_surface`/`fall_until_landed`,
~400 lines) ported onto the view.

The headline outcome the user flagged: it went *fast* and nothing broke.
The reason it could is a transferable technique, plus a process lesson.

## Technique: lockstep parity test for porting integration code

When you re-home a chunk of deterministic simulation code from one data
shape to another (here: `&mut EnemyRuntime` fields → `EnemyMut` component
refs), the safest, fastest verification is **not** "spot-check a few
behaviors" — it's to run the OLD and NEW code paths side by side with
identical inputs and assert the outputs are *bit-identical* every tick:

```rust
let mut enemy = EnemyRuntime::new(..);            // legacy
let mut scratch = EnemyClusterScratch::from_runtime(&enemy); // new, same start
for tick in 0..180 {
    let frame = ...;                              // same frame to both
    enemy.update(&world, .., frame);
    scratch.as_mut().update(&world, .., frame);
    assert_eq!(scratch.kin.pos, enemy.pos, "@tick {tick}");
    assert_eq!(scratch.kin.vel, enemy.vel, ..);
    // surface / attack / ai_mode / alive ...
}
```

Cover every branch (here grounded / aerial / surface-walker via three
brain ids). Because both run the *same operations* on the *same floats*,
exact `assert_eq` on `f32`/`Vec2` is correct — no epsilon needed. This
catches the only realistic failure mode of a mechanical port: a rename
typo (`self.kin.pos` written where the original read `self.size`). It
turns "did I transcribe 400 lines correctly?" from a fear into a green
checkmark. The parity test found nothing wrong — which is the point: it
*licenses confidence* instead of hoping.

The `from_runtime` adapter that builds the new shape from the old is the
small extra cost that unlocks this test (and doubles as the transition
bridge while consumers are migrated).

## What made the port itself mechanical

- The view's field names were chosen to *mirror the old struct's
  layout*: `EnemyMut { kin, status, surface, attack, config, motion }`
  with `surface`/`attack` being the same sub-structs. So the port was a
  consistent rename (`self.pos`→`self.kin.pos`, `self.alive`→
  `self.status.alive`, `self.archetype`→`self.config.archetype`,
  unchanged for `self.surface.*`/`self.attack.*`), not a redesign.
- Put `impl EnemyMut` in the same module as the original (`enemies.rs`)
  so it reaches the private gravity/jump consts, `approach`, and the
  surface predicates with zero new `pub` plumbing.
- Inherent impls can live in any module of the defining crate, so the
  view (defined in `ecs/enemy_clusters.rs`) gets its integration methods
  in `enemies.rs` — orphan rule is a same-crate non-issue.

## Process lesson (the confidence note)

I spent too many cycles up front hedging about scope and risk — "this is
huge, it might break the game, maybe I should scope it down." Once I
committed to the work and *built the safety net first* (components → view
→ port → parity test), the genuinely scary part (the physics integration)
landed correctly on the first compile-and-test. The lesson for next time:

**A large mechanical refactor is far less risky than it feels when you
have a differential/parity harness. Bias toward executing — build the
verification first, then port boldly.** Deliberation is not free; a
green parity test is worth more than an hour of worrying about whether
the transcription was faithful. The same move applies to any
shape-changing port of deterministic code (the player cluster migration,
boss-attack-state mirror cleanup, future NPCRuntime dissolution).

## Status / pointers

Verified-green checkpoint; remaining work (spawn wiring, `update_ecs_actors`
rewrite, ~30 consumer migrations, `EnemyRuntime` deletion) is mechanical.
Plan: `dev/reviews/enemyruntime-ecs-inventory.md`; memory:
`project_enemyruntime_ecs_migration`. Sibling prior art:
[`player-cluster-native-push-2026-05-28.md`](player-cluster-native-push-2026-05-28.md).
