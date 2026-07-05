# Track F — Super Smash Siblings (platform-fighter / SSB1 acceptance demo)

Inspired by Super Smash Bros (N64). Parody-original roster drawn from
ambition's own cast — the Hall of Characters IS the roster screen story.

**Purpose (Jon's words, binding):** prove that multiple controlled bodies
with different movement identities can coexist in one arena, share combat
semantics, and retain their own feel.

**Depends on:**
- E5-finish (`ambition_runtime` + host groups)
- E4 (`ambition_sim_view` / render edge cleanup)
- A-track moveset/ability model — COMPLETE (data + prefabs + techniques)
- E6/E7 actor residue cleanup
- Local input slot routing — netcode N1.1–N1.3
- Combat/projectile crate carve (E2) as needed
- The combat-model stack: CM1 (knockback growth + weight + Unbounded
  death policy), CM2 (DI), CM3 (smash/charge), CM4 (cancels), CM6
  (grab/throw/shield-stun), CM7 (frame data)
- Fighter brain FB1–FB4 (CPU opponents)

## Design (v1 scope)

- **Roster:** 4 characters spanning movement identities — the robot
  (axis-swept classic), Sanic (surface-momentum), a floaty heavy
  (gnuton-on-foot lineage), a technical sword character (knight lineage).
  Each = catalog row + moveset rows (prefabs + a few authored MoveSpecs:
  tilts, aerials, smashes, one special each). RETAINED FEEL is the test:
  the momentum body still rides; the classic body still snaps.
- **Stages:** 2 arenas in the demo's .ldtk — a flat+platforms classic and
  one with a moving platform. Blast zones = the world-AABB OOB rule read
  by the mode as KOs. Ledges = existing ledge-grab vocabulary.
- **Match state (mode-scoped, OUTSIDE engine core):** stocks, percent
  display (the `Unbounded` damage meter), respawn platform +
  invulnerability window, timer, results screen. 1–4 local slots
  (press-to-join), CPU fill from fighter-brain profiles L1–L9.
- **The percent HUD** reads the read-model's damage meter — presentation
  only; the sim never knows "percent" exists.
- **Camera:** N-subject bounding-frame policy (netcode N1.3).

## Slices

F1 rules crate + match loop (stocks/KO/respawn/results) headless-first
[opus]; F2 roster + movesets (4 rows; frame-data sanity via CM7 table)
[opus]; F3 stages + blast-zone/ledge integration [opus]; F4 local-N
join/CPU-fill front-end [opus]; F5 the feel pass queue (BLIND) + Jon's
tuning [Jon]; F6 hosting: the Colosseum wing in ambition (Phase D-C) —
walk in with any possessed character; the mode seeds slot 1 from the
possessed body [opus].

**Exit (Jon's, verbatim + sharpened):** one content crate + thin app; no
engine crate edits to add the demo; at least two different body profiles
fighting in one arena (we ship four); match state lives outside engine
core. Plus: a full 3-stock CPU-vs-CPU match completes headlessly with a
deterministic replay; DI measurably extends survival in that replay; and
the same characters remain playable in ambition's sandbox unchanged.
