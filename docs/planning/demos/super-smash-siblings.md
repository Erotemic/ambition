# Track F — Super Smash Siblings (platform-fighter / SSB1 acceptance demo)

Inspired by Super Smash Bros (N64). Parody names throughout (Q28 policy).

**Purpose (Jon's words, binding):** prove that multiple controlled bodies
with different movement identities can coexist in one arena, share combat
semantics, and retain their own feel.

**First-round scope (Jon, 2026-07-06):** NO online multiplayer. Up to
**4 fighters** in an arena — any mix of CPUs (fighter-brain profiles) and
at most one human player by default; **a second local controller joins as
a second human if the slot binding makes it natural** (it does — netcode
N1.1 is exactly this; treat 2-human as an expected outcome, not a
stretch). Percent-style damage display. A character select screen.

## What the demo CONSUMES from the engine (by role)

| Role | What SSB uses it for | Must exist first |
|---|---|---|
| [the sim assembly] + [the provider lifecycle] + [the windowed host] | app composition, exact session scope, mode scope, fixed-tick option | provider/session tracks 1–2; fixed-tick substrate landed |
| [the sim heart] | bodies, slots, possession, spawn/respawn primitives | landed foundation; session-root cleanup continues |
| [the combat resolver] | knockback growth + weight + `Unbounded` death policy (CM1), DI (CM2), smash/charge (CM3), cancels (CM4), per-move sfx/vfx (CM5), grab/throw/shield-stun (CM6), frame-data table (CM7) | CM1–CM7 |
| [the movement kernel] | BOTH movers in one arena — that's the whole point (robot/mary-o/goblin on axis-swept; sanic on surface-momentum) | landed |
| [the actor vocabulary] | fighter-brain CPU profiles L1–L9 | FB1–FB4 |
| [device→intent] | N controllers → N slots, join flow binding | N1.1 |
| [the space IR] + [the LDtk backend] | stages as the demo's own `.ldtk`; blast zones = the world-AABB OOB event | landed |
| [the observation boundary] | damage-meter + facts read for the percent HUD (the sim never knows "percent" exists) | E4 |
| [the authoring spine] + [the sprite-geometry authority] | roster rows, movesets, sheets | landed |

Anything beyond this list that turns out to be needed engine-side is an
**oracle-violation**: file it in tracks.md, land it as engine work, never
inline it in demo commits.

## What the demo OWNS (builds for itself, in `ssb_content`)

1. **The match rules plugin** (mode-scoped, M19): stocks (default 3),
   KO detection (consume the engine's OOB/fell-out event as "blast" per
   stage-authored blast-zone margins), respawn platform (spawn primitive +
   a brief invulnerability window + descend-on-input), match timer,
   sudden-death rule, results/victory screen state machine. All state on
   mode-scoped entities; a full CPU-vs-CPU match must run headless.
2. **The percent presentation policy**: per-fighter damage meter read from
   [the observation boundary], rendered as N% (meter × display scale);
   the `Unbounded` death policy on every fighter row is what makes the
   meter percent-like. HUD layout is demo-owned UI.
3. **The roster** — initial cast (Jon): **player-robot, goblin, PCA,
   mary-o, sanic** (grows as characters become expressive in the game).
   Standalone-vs-hosted rule: the STANDALONE demo authors its own catalog
   rows + archetypes + movesets for these five (sheets produced by the
   shared sprite tooling targets — tool reuse is fine; CRATE dependency
   on ambition content is not). The HOSTED demo (inside ambition) reads
   the host's installed catalog, which already contains these characters
   — the select screen offers the intersection of "roster ids the mode
   declares" with "ids installed". Per-fighter data each row owns: weight,
   moveset rows (tilts/aerials/smashes via prefabs + a few authored
   `MoveSpec`s, one signature special each), death_policy `Unbounded`.
4. **The character select screen**: a lightweight mode-scoped scene — a
   portrait grid from the declared roster (portraits = the catalog's hall
   sprites), cursor per joined slot, CPU-fill toggles + level picker per
   empty slot, stage pick, GO. Built on [device→intent] + `ui_nav`
   primitives; explicitly NOT on [the menu stack] (the host menu is
   Ambition chrome; a demo select screen is demo UI).
5. **Stages**: 2 arenas in `ssb_stages.ldtk` — a flat+platforms classic
   and one with a moving platform; blast-zone margins authored as level
   fields; ledges are the engine's ledge-grab vocabulary.
6. **The join flow**: press-start-to-join binding UI over N1.1's binding
   resource (slot ↔ device); default = slot 1 human, others CPU.

## Build order (each step lands with headless tests before any feel pass)

- **F1** `ssb_content` + rules plugin: match loop (spawn 4 fixture
  fighters → damage → KO on OOB → stocks → results) fully headless, on
  fixture bodies before the real roster exists.
- **F2** roster: five rows + movesets; CM7 frame-data sanity checks
  (startup/recovery bands per weight class); per-row signature special.
- **F3** stages + blast zones + respawn platform + ledge integration.
- **F4** select screen + join flow + CPU fill (FB profiles); second
  local controller path proven with two bound devices in a headless
  input-injection test.
- **F5** feel pass queue (BLIND commits; Jon tunes weights/kb bands).
- **F6** hosting: the Colosseum wing in Ambition through the common provider/session lifecycle; the controlled subject seeds slot 1 without a separate player path.

## Exit (Jon's, verbatim + sharpened)

One content crate + thin app; no engine crate edits to add the demo; at
least two different body profiles fighting in one arena (we ship both
movers across five characters); match state lives outside engine core.
Plus: a full 3-stock 4-CPU match completes headlessly with a
deterministic replay; DI measurably extends survival in that replay; two
local controllers can play a match; the same characters remain playable
in ambition's sandbox unchanged.
