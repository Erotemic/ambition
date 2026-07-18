# Tracks — current executable queue

This file is the live queue, not a completion ledger. The completed July 15–16
architecture campaign is summarized in [`status.md`](status.md) and preserved in
git history.

## 1. Quarantine external effects to confirmed GGRS frames

The headless simulation harness now performs complete GGRS save/load/resimulation.
Before online play, presentation and irreversible host effects must not observe
speculative or replayed frames.

- classify audio, VFX, analytics, save writes, achievements, and host I/O;
- buffer effect intents by GGRS frame;
- release only frames at or below the confirmed-frame boundary;
- discard abandoned predicted intents on rollback;
- prove a forced sync-test rewind emits each accepted effect exactly once.

**Exit:** repeated rollback cannot duplicate an external effect, and a Matchbox
transport can be attached without changing simulation systems.

## 2. Build the provenance + three-origin `ConstructionPlan` vertical slice

Immutable prepared content and exact snapshot identity are complete. The next
engine architecture task is Phase 3 of
[`engine/immutable-content-and-transactional-construction.md`](engine/immutable-content-and-transactional-construction.md):

- add explicit `SpawnOrigin` and internal stable `RecipeId`;
- plan one authored placement, one provider-staged actor, and one runtime-dynamic
  family through a common pure `ConstructionPlan`;
- validate identities and relationships before mutation;
- use the same recipes for ordinary spawn and reconstruction;
- remove `SimId` parsing as provenance authority for the selected dynamic family;
- prove deterministic plan dumps and planned-versus-committed roster parity.

**Exit:** failed planning leaves the active world untouched, all three origins
share one inspectable planner/executor, and the runtime-dynamic family can be
reconstructed without inferring its recipe from an id string.

## 3. Close Super Mary-O level 1

The engine-facing seams are already proven: world pickups equip through the shared
item path; the grow cap changes worn identity and collider size; the spark blossom
grants a real ranged move; bricks, cronies, flag scoring, tally, clock, and cyclic
restart exist.

Remaining customer work:

- secret pipe and underground room;
- sliding shell prop;
- HUD, title, and results presentation;
- a deterministic scripted run that completes level 1 through real controls,
  collects a powerup, and exercises its effect.

**Exit:** visible and headless customers use the same provider, body, item, and
level state with no Mary-O-only engine path.

## 4. Close one complete Sanic act

The provider persona, standard host input chain, transformation, ball dash,
surface-momentum route, lifecycle, and geometry/orbit/stranding oracles are
landed.

Remaining customer work:

- bits and drop-on-hit;
- at least one enemy with rolling/stomp outcomes through shared contact/combat;
- goal, HUD, results, and end-of-act sequence;
- one complete authored act;
- deterministic headless completion proving the rewarded high route is faster
  than the lower safe route under the same control contract.

Do not absorb movement/contact work owned by another active campaign.

## 5. Correct the fighter-rollout design before FB6

Do not implement the current FB6 text literally.

- A wall-clock cutoff cannot decide authoritative actions if replay/resimulation
  is expected to rerun the brain deterministically. Prefer a fixed work budget,
  with elapsed time as telemetry only, unless decisions become recorded external
  inputs.
- A live authoritative snapshot exposes facts outside the delayed `Perceived`
  contract. Rollouts need a hypothetical state reconstructed solely from allowed
  perceived facts, or a deliberately limited perceived-state forward model.

**Exit:** the determinism and no-cheat contracts are explicit enough that an L3
implementation cannot accidentally violate either one.

## 6. Finish the bounded boss animator fold

Verify and remove only genuine animation residue:

- converge `BossAnim`/boss frame projection toward the shared `CharacterAnim`
  vocabulary;
- retire obsolete `target_pos` or equivalent mirrors where they remain live;
- preserve boss decision policy and encounter orchestration where they are real
  domain responsibilities.

Do **not** reopen boss body integration: `integrate_boss_bodies` already delegates
to the canonical actor/body kernel and writes the canonical motion sweep.

## Parallel maintenance

Small non-blocking work may proceed when it does not collide with the active
campaigns:

- current demo/documentation corrections;
- generated module-map repair after structural changes settle;
- one structurally complete content eviction at a time when a real named family
  remains in a reusable crate;
- narrow test strengthening for teardown/resource clearing.

## Standing execution rule

Use Rust types, ownership, crate direction, visibility, and ordinary behavioral
acceptance tests before adding policy/scanner machinery. Historical journals stay
historical. Completed execution narratives do not remain in this live queue.
