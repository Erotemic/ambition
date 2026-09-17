# Track H — Hollow Lite (exploration-combat + boss-quality acceptance demo)

Inspired by Hollow Knight's opening area (Forgotten Crossroads energy)
ending in a real boss fight (False Knight energy). Parody-original: a
small errant automaton in the ruins of a dead machine-colony.

> **Re-measured 2026-09-17 (and against `008b44120` on 2026-09-02 before that):
> STILL ENTIRELY UNBUILT, as this plan expects.** `git grep -il hollow` over
> `crates game tools examples dev` returns 19 files and NONE of them is this
> demo: they are a hollow shield mesh, hollow glyph boxes in two shell strings,
> LDtk schema/tooling vocabulary, and five comments citing Hollow Knight as the
> reference for combat feel or a wall bundle. The built demo customers are the
> nine `game/ambition_demo_*` crates — `{mary_o,sanic,smash,twintrack}` each with
> an `_app` sibling, plus `ambition_demo_pocket`, which is a provider FIXTURE and
> not a customer; see [`README.md`](README.md). Nothing here has been superseded
> by code.
>
> ⚠ **AND THE GATE ON H3 IS NOT ON THIS PAGE'S CRITICAL PATH BY ACCIDENT.** H3 is
> BD8, BD8 is the acceptance test for a pipeline whose measuring half (BD6, the
> playtester rig and its bands) is still OPEN, and BD6 is gated on **F1–F4** of
> [`../engine/fighter-brain.md`](../engine/fighter-brain.md) — the ladder
> authority, `read_weight`, representative rosters, and the mid-ladder utility
> progression. ⛔ That chain was unreadable until 2026-09-17: `boss-design.md`
> spelled the blocker `FB1–FB4` in both places it appears, and no page in `docs/`
> defines an `FB` slice. ⇒ H1, H2 and H4 do not wait on any of it; **H3 does, and
> it is the only slice whose exit Jon has to sign.**

**Purpose:** two proofs in one. (1) The exploration-combat loop —
interconnected rooms, melee-first combat with pogo, benches/saves,
currency-loss-on-death — is content on the engine. (2) **The boss-design
pipeline produces a fight Jon rates as actually fun, authored by an
opus-level agent** ([`../engine/boss-design.md`](../engine/boss-design.md)
BD8 — this demo is that pipeline's acceptance test).

## Consumes (by role) / Owns

**Consumes:** [the sim assembly]+[the windowed host] · [the movement
kernel] (axis-swept + pogo) · [the sim heart] (respawn policy: mobs
respawn OnRest, the boss dies forever — this demo is ADR 0022's real
consumer) · [the combat resolver] (melee/pogo/recoil, per-move
presentation facts, the focus/heal channel technique seam) · [the actor
vocabulary] (enemy brains; the boss pattern vocabulary + BD1 control-flow
atoms) · [the set-piece kit] (arena beats, encounter/phase state) ·
[the saved shapes] (bench saves, currency flags) · [the space IR]+[the
LDtk backend] (its ~10-room world) · the boss pipeline BD1–BD7 as
PROCESS (seeds, validator, playtester rig).

**Owns (`hollow_content`):** the world (well, loop, shortcuts, bench,
boss door), the rules plugin (currency, shade-drop-on-death, bench
respawn policy — mode-scoped), the focus/heal technique registration,
four enemy rows, the BOSS (authored through the pipeline — the BD8
acceptance), HUD, title/results.

**Engine prerequisites:** the provider/session lifecycle tracks, encounter lifecycle convergence, boss action convergence onto the shared moveset path, the fighter-brain/boss-quality foundations, and authored respawn/save policy.

## Design (v1 scope)

- **World:** ~10 interconnected rooms, one .ldtk world: a vertical well
  entrance, a loop with two shortcuts that unlock backward (the
  metroidvania contract), one bench (shrine=save vocabulary exists), a
  currency cache, the boss arena behind a heavy door.
- **Combat feel:** nail-analog melee with pogo (exists), directional
  slashes (exists), hit recoil both ways (exists via knockback), soul-
  analog meter charged by hits → one heal channel (a technique with a
  channel window — the focus/heal is the one new technique; [opus]).
- **Death rule:** drop currency as a shade-analog pickup at death site;
  bench respawn (respawn-policy: the PLAYER'S death policy is authored
  content, exercising the same enum actors use).
- **Enemies:** 4 archetypes on existing brains (crawler, lunger, flyer,
  shielded) — each teaching one verb, per the boss pipeline's
  answer-coverage philosophy applied to trash design.
- **THE BOSS:** authored by an opus agent through the full pipeline —
  seeds + control-flow atoms + telegraph grammar + validator + playtester
  metrics in band + BLIND ship → Jon's verdict. Target: 3 phases, one
  arena beat (BD2), one signature move contributed back to the seed
  library. A failed objective function, legible: it optimizes for
  something (guarding its hoard; repeating what last hit you) and
  over-commits.

## Slices

H1 world + traversal loop + bench/death rules [opus]; H2 enemy quartet +
focus/heal technique [opus]; H3 the boss (the BD8 acceptance) [opus +
Jon]; H4 hosting wing in ambition [opus].

**Exit:** doctrine exits + the quality one: the playtester rig report is
in band at 3 difficulty levels, the fight validator is green, AND Jon's
taste pass says the fight is fun (recorded verdict; a NO loops H3 with
his feedback banked into the seed library — the pipeline improving is
part of the exit).
