---
id: one-body-one-path
aliases: []
status: current
authority: durable-concept
last_verified: 2026-08-07
related_docs:
  - docs/concepts/invariants.md
  - docs/concepts/movement-collision.md
  - docs/adr/0020-mounts-and-vehicles.md
---

# One body, one path

**The player is an actor.** Every rule that fires for one controller kind fires
for all of them, through the same code, or it is a fork.

This is the repo's most-violated rule. It is stated in `AGENTS.md` because
everyone reads that; the detail lives here because the detail is long and the
rule is short.

## The bifurcation smell test

Before you write *anything* keyed to "player", or to "actor / enemy / boss" — an
attack, a hitbox, a damage rule, a VFX/SFX emit, a shield, a reset, a state
machine, a brain hook — ask:

> **Does the other controller kind already do this on its own code path?**

If yes, you have found a **FORK**. Your job is to UNIFY onto the single shared
seam and DELETE the other side. It is not to add a second site.

⛔ **Adding a parallel emission site, state component, system, or spec for an
effect that already exists elsewhere is a BUG, not a fix — even if it compiles
and every test passes.** A green test on a forked path is worthless: it proves
the fork works, which is the problem.

⚠ **If you genuinely cannot complete the merge in one pass**, do NOT add the
parallel path "for now". Route the new caller *through the existing seam* —
extracting one shared fn / system / event if none exists — and log the remaining
merge in `dev/journals/code_smells.md` with `BIFURCATION:` as the first word.

⭐ When a doc or a keystone says "unification", it means **delete one path**. It
does not mean "make the two behave similarly".

## What is unified today

This is a STATUS inventory, not the rule. It goes stale; the rule does not.

**Melee is unified end to end.** The STATE (`BodyMelee` / `MeleeSwing`), the
swing MODEL (`AttackSpec`), the slash VFX (`emit_melee_slash` in `combat::util`)
and the strike SPAWN are one path for the player and every actor. The spawn goes
through the moveset: `combat::moveset::trigger_moveset_moves` →
`advance_move_playback` spawns ONE gravity-resolved volume that drives both the
damage `Hitbox` entity and the slash, projected to body state by
`project_moveset_melee_to_body_melee`.

⛔ **Do not reintroduce** a `PlayerAttackState` / `ActorAttackState` split, a
second slash emit, or a per-frame player damage loop. Every melee is an
`"attack"`-verb moveset move riding `MovePlayback`.

**The movement driver is unified at the engine entry.** The player tick is ONE
system (`player_body_tick`) that calls the SAME combined body tick the actor uses
(`ae::update_player_with_tuning_clusters` ≈ the actor's
`update_body_with_tuning_clusters`). The two differ only in the input frame and
in the respawn POLICY.

**The two-clock precision-blink split is an INPUT affordance, not a simulation
structure.** Responsive aim during bullet-time is purely
`InputState::control_dt`: a human sets `control_dt = real frame dt`; a brain
leaves it `0` and runs everything at sim time. There is no second simulation.

## What is deliberately SEPARATE, and why

⚠ **`player_body_tick` and `update_ecs_actors` stay separate Bevy systems on
purpose.** What is shared is the body-tick engine entry, not the orchestration.
Merging the two orchestrators into one god-system is NOT the goal, and a change
that does it is not an improvement — it trades a legible seam for a large system
that no longer says which population it is stepping.

## The next elevation

The unified action/ability timeline — cancel windows, movement locks,
armor/i-frames, resource costs, hurtbox swaps, anim binding — layered on the one
strike seam that already exists. Not a second seam beside it.
