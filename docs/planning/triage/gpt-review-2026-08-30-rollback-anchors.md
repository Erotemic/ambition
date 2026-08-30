# GPT re-review 2026-08-30 — dynamic spawn anchors, schedule order, deterministic selection

Snapshot reviewed: `e3499e3d2f57`, 42 commits after the `23e472c39fd4` tree of
[the Rust-correctness review](gpt-review-2026-08-29-rust-correctness.md). Cargo was
unavailable in the reviewer's environment, so every finding below is source review
until this file says it was measured here.

**The re-review's own headline**, and it is the right one: *registering a component
for rollback is being confused with making the dynamically spawned entity
participate in rollback.* `rollback_component_clone::<T>` installs a CODEC.
`require_rollback::<T>` installs the ANCHOR. A type can have the first and lack the
second, and every registry-shaped check reads that as covered.

The repo already owns the assertion that catches it —
`assert_no_inert_registrations` in `game/ambition_app/tests/rollback_coverage.rs`,
which says in as many words that a registration on an unanchored archetype is
INERT. What it lacks is a POPULATION: it sweeps a booted room and a live match, and
its own doc comment already warns that state which only exists after an EVENT is
structurally invisible to it. That is the gap, and it is the gap the review found
twice independently.

## Rows

| # | Row | Rank | State |
|---|-----|------|-------|
| 1 | Quality seed runs in `PreStartup`, settings load in `Startup` | P1 | ▣ landed |
| 2 | `PortalShot` has a codec and no anchor | P0/P1 | ☐ |
| 3 | `FallingHazard` has a codec and no anchor (after an attempted fix) | P1 | ☐ |
| 4 | Sentry / Vortex: rollback lifetime, effective allegiance, deterministic targets | P1 | ☐ |
| 5 | One spawn seam for authoritative dynamic sim state | architecture | ☐ |
| 6 | Scenario-driven inert/coverage sweep that fires abilities | architecture | ☐ |
| 7 | `portal_fire_system` keeps only the LAST intent per tick | P1 | ☐ |
| 8 | `collect_ecs_pickups` / `collect_world_items` decide by query order | P1 | ☐ |
| 9 | Deterministic selection as a shared primitive (metric, then `SimId`) | architecture | ☐ |
| 10 | Fuse arming still reads `vel != ZERO` instead of `Release::Throw` | P1 | ☐ |
| 11 | Submerged: sweep knows the passability policy, penetration repair does not | P1 | ☐ |
| 12 | Fighter-brain L3 rollout — **measure before changing** | — | ☐ deliberately not a code change |

Rows carried forward from the 2026-08-29 review and NOT re-litigated here: Mary-O
`Local` room-transition state, quest/map-room edge detectors, held-projectile
attacker provenance, folding `HeldProjectile` into `ProjectileSpawnRequest`,
`ControlledSubject` vs `DrivenBodies` for custom item abilities, and the D199
swept-AABB / one-way policy. They stay open in
[their own file](gpt-review-2026-08-29-rust-correctness.md).

---

## ▣ 1 — The hardware quality seed migrated nobody

**Confirmed by measurement, not by reading.** Four arms were written against the
shipped schedule first and three of them were RED: an existing settings file on a
`Cpu` adapter resolved `High` where the policy says `Potato`, and the
`hardware_seeded` guard came back `false`.

The order that shipped:

```text
PreStartup   default UserSettings -> seed reads the adapter -> hardware_seeded = true
Startup      load_settings_at_startup  *settings = load_settings(path)
             ...which REPLACES THE WHOLE RESOURCE: tier back to the file's,
             hardware_seeded back to false
Update       resolve the budget from the un-seeded tier
```

and the system is startup-only, so it never runs again. The machine the feature was
written for — an existing install on an integrated GPU carrying a persisted `High`
from the OS-based default — migrated on **no boot, ever**.

⭐ **A fresh install migrated correctly**, because the loader returns early when
there is no file. That is why the pure unit tests on `seed_from_hardware` and a
first-run play-through both looked right: the only broken path was the one nobody
tests, an upgrade.

**Fix:** `PreStartup` → `PostStartup`. Still before the first `Update`, so the
resolved budget is seeded on frame one; now after the loader, so the file it has to
migrate is actually present.

**Guarded by** `seed_schedule_tests` in `crates/ambition_render/src/quality.rs`,
five arms on a real booted `App` rather than a direct call to the policy function:

- an existing un-seeded file receives its seed (RED before the fix)
- a tier the player CHOSE survives the seed (RED before — and only reachable at all
  once the load stops winning by accident)
- a fresh install with no file is still seeded (GREEN before; the arm exists so the
  move could not lose it)
- the first frame RESOLVES the seeded tier (RED before)
- the seed REACHES DISK, read back out of the file (RED before — without the write
  the flag is decorative and the player is re-examined every boot)

⚠ The review asked for exactly this shape and it was right to: the existing tests
call `seed_from_hardware()` directly, and no test of a pure function can see an
ordering defect between two plugins.
