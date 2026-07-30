# Maintainer decisions

This file records decisions Jon made explicitly. It exists to distinguish
maintainer intent from agent analysis, consensus drafts, and inferred design.
Agent-written records may explain a decision, but they do not become Jon's
decisions unless they are represented here or Jon says so directly.

Confidence is Jon's current confidence in the decision, not a permanence promise:

- **High** — proceed on this basis; do not reopen without new concrete evidence.
- **Medium** — current direction, but implementation may reveal a better shape.
- **Low** — tentative preference or deliberately deferred naming/design choice.

Do not backfill confidence for older decisions by guessing. Add or revise a row
when Jon states a decision or changes his confidence.

| Date | Decision | Confidence | Notes |
|---|---|---:|---|
| 2026-07-16 | Perform the identified content evictions. | High | Each eviction must end in an open provider-owned catalog, registration, or presentation seam rather than moving a closed engine-owned table. |
| 2026-07-16 | Extract the reusable programmatic simulation surface as `ambition_sim_harness`. | High | Reset/step, typed actions and observations, headless testing, RL, replay, and fuzzing belong below `ambition_app`. |
| 2026-07-16 | Extract the platformer-provider lifecycle from the `ambition` facade and consolidate the repeated provider protocol. | High | Exact crate name remains open; `ambition_platformer_provider` is the working name. |
| 2026-07-16 | Keep cutscenes and encounters as separate domain systems. | High | Cutscenes are scripted with limited interaction; encounters are interactive with limited scripting. Shared micro-primitives are allowed only when naturally demonstrated. |
| 2026-07-16 | Keep provider registration explicit in the host composition root. | High | The explicit dependency plus plugin registration is intentional; do not add opaque plugin discovery. |
| 2026-07-16 | Defer any boss crate carve until boss behavior converges onto the canonical moveset/action path. | High | Reassess afterward whether a separate boss crate still exists as a coherent subsystem. |
| 2026-07-16 | Reject the proposed named-content scanner and stop adding poison-test ceremony by default. | High | Prefer Rust types, APIs, crate boundaries, visibility, and behavioral tests. A new policy test must justify why those cannot enforce the invariant. |
| 2026-07-16 | Keep the compiler term **lowering** for authored world IR becoming live ECS state. | High | Deserialization/import produces the IR; lowering materializes its canonical runtime representation. |
| 2026-07-16 | Repository-wide knowledge-base hygiene checks are CI/maintainer tools, not routine local validation. | High | Agent-facing docs should not attach them to ordinary code changes. |
| 2026-07-16 | Preserve historical journals as historical records during documentation cleanup. | High | Do not rewrite old journals merely to modernize present-day guidance. |
| 2026-07-16 | A full rename of `ambition_actors/src/features/` may be worthwhile, but the name `sim` is not settled and the work is low priority. | Low | Do not perform a partial rename or let naming block architectural work. |
| 2026-07-29 | **Label occlusion / transition nameplates are LOW PRIORITY** — not touched until combat is good. | Low | Jon: *"This is more of a debugging feature… a real game probably doesn't use it in this capacity. It's more like there's a door, and the player knows where it goes based on a map."* Engine SUGAR to make a game define its own presentation is welcome; the always-on nameplate is a dev affordance. Queue Z′13 / AC12 stay open at low priority; the two OCCLUSION defects (AB1, blink_run) were fixed anyway because a covered protagonist is not a naming question. |
| 2026-07-29 | **Do NOT gate or redesign `Interact` yet — it needs a design discussion.** | High | Jon: *"A smash game (versus) shouldn't have the concept of 'interact', in fact neither does Mary-O, or Sanic. Only Ambition has an 'interact'… it's something the ambition game might want, and its general to that game, but maybe not to the engine. There is certainly an architecture issue here, and I don't want to make a rash decision and introduce a worse abstraction."* So queue Z′4 is NOT a prompt-gating task: the real question is whether `interact` belongs to the ENGINE at all or to Ambition-the-game. Blocked on that discussion, deliberately. |
| 2026-07-29 | **Build the portrait target → art resolver.** | High | Jon: *"the resolver makes sense. The portraits are not a hub feature… a character portrait for a dialog box is ubiquitous for platformer 2d games, so it makes sense the engine has a mechanism to make it easy, although like most things with the engine, it should always be possible to ignore some part of it and roll your own. This is the Bevy ECS way."* So: engine mechanism, opt-out by construction — not a required path. Queue Y″6. |
| 2026-07-29 | **Replace the invented lab dummy enemies with real ones that already exist.** | High | Jon: *"The lab striker was an agent invention. I asked it to just use a goblin as the enemy there until I decided what I wanted to do about the Nazis… Just replace those enemies with a real enemy that already exist. The entire intro sequence is unpolished slop anyway."* Done: `Puppy Slug` / `Salvage Guard` / `Lab Raider`, each keeping its original brain so the room still teaches patrol / guard / striker. Queue AB5. |
| 2026-07-29 | **DI matters — Smash-style physics is wanted in Ambition itself, not only in versus.** | High | Jon: *"In smash DI is critical! I probably want to use some smash physics in ambition itself too, because I want that game to feel like a cross between smash subspace emissary and hollow knight."* ⚠ he did not name a NUMBER, so the exact `di_max_angle` remains a feel value for him; the decision recorded here is that DI is on and that Smash physics is a direction for the flagship game, not a versus-only affordance. Queue F0-J1. |
| 2026-07-29 | **Generic versus ends on HEALTH. Smash Siblings is a separate, specified mode.** | High | Jon: *"For a generic 'versus' fighting proof of concept I don't care. Probably use health, to make it a generic fighter. For Smash Siblings 3 stock, no items… character select screen, ability to have 1-4 players, have them toggle between real player or cpu, use a smash like, drag an orb onto your character to select, and then the fight boots into a single battlefield like 3 platform level. Its 3 stocks, and then when the game ends it goes back to the character select screen. We don't need items in a first pass."* So `DeathPolicy::Unbounded` stays uncalled for versus. Queue F0-J2. |
| 2026-07-29 | **Smash Siblings HUD: per-character portrait, stock icons, percentage. No score.** | High | Jon: *"A smash game would have a character portrait on the bottom for each character with an icon for each stock and their current percentage. There is no score, when you lose your stock you are dead."* This answers Z′10 by making it moot: the reserved-surround branch was about placing a SCOREBOARD, and there is no score. |
| 2026-07-30 | Defer the `bevy_ggrs` patch-table leak; revisit once upstream merges the `GgrsFrameTiming` accessor to crates.io. | High | The 2026-07-30 blind-agent run ranked this the highest-cost finding in the API campaign — a third party cannot compile the engine without copying `[patch.crates-io]` out of the workspace root, and it blocks before any API question. Deferred anyway, because the only way to drop the fork is the parallel accumulator `sample_ggrs_accumulator_phase` already rejected (it diverges during run-slow catch-up, stalls and rollback resimulation). Stating the rule IS done — `docs/sdk/README.md` carries the required entry and the pinned rev. Removing the need waits on upstream. Do NOT let this block slice B. |

The fuller multi-agent recon consensus, including accepted campaigns and explicit
non-goals, is in
[`engine/decisions-2026-07-16.md`](engine/decisions-2026-07-16.md).

Questions that are WAITING for a decision — scoped far enough that the choice is
real and the work after it is small — are collected in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md). Nothing
there is a decision; a row moves into the table above when one is made.
