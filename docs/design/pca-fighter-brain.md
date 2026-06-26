# The reactive fighter brain (PCA & beyond) — design note

How the Perfect Cell-ular Automaton (and any actor) fights with a strong,
reactive, **never-cheats** AI, and how to swap a learned policy in later.

This is engine machinery: the brain lives in `ambition_characters` (the `smash`
brain), drives the **same** ability/input pipeline a player uses, and is content-
agnostic. The PCA is just a content actor that selects it.

## The seam: `Brain` → `ActorControlFrame`

Every controllable actor — player, NPC, enemy, boss, possessed body, and future
remote/learned policies — writes one `ActorControlFrame` per tick. The brain is
a pure function of a read-only snapshot:

```
Brain::tick_with_actions(&ActionSet, &BrainSnapshot, &mut ActorControlFrame)
```

- **Observation space** = `BrainSnapshot` (`brain/snapshot.rs`): own pos/vel/
  facing/grounded/air-jumps, combat timers (cooldown/windup/active/recover/stun),
  the target's pos + alive, sim time + dt, max-run-speed, crowding, terrain.
  Everything in it is something a *player could perceive*; there is no privileged
  read of the opponent's buffered inputs or future state.
- **Action space** = `ActorControlFrame` (`actor/control.rs`): `locomotion`
  (grounded throttle) / `velocity_target` (free-mover), `facing`, and edge/sustain
  verbs — `melee_pressed`, `fire`, `jump_pressed`, `dash_pressed`, `shield_held`,
  `special_pressed`, `blink_pressed` + `blink_quick_dir`, `fly_toggle_pressed`, …
  The integration half decides what's *physically possible* (collision, cooldowns,
  world rules). The brain only signals intent — exactly the player's contract.

Per-actor **capability** is separate from **policy**: the brain emits abstract
intent ("melee pressed"); the actor's `ActionSet` resolves it to the concrete
effect (`ActorActionMessage`). Same brain + different `ActionSet` = different
attacks. That's what lets one policy drive many bodies (possession / RL).

## The smash brain pipeline (`brain/smash/`)

Five pure stages, each independently replaceable:

1. **observe** → `ObservationFrame` (distance, to-target, crowding, self timers).
2. **choose_mode** → `BroadMode` (Approach / Retreat / Engage / Reposition /
   Recover / Idle) with hysteresis (`mode_dwell_s`) so it doesn't oscillate.
3. **choose_action** → `SpecificAction` (Walk/Dash/Jump/Melee/Ranged/Special/…),
   gated by the `ActionSet` capability mask + range bands + verb cadences.
4. **apply_difficulty** → commit roll + aim jitter (the fairness filter).
5. **emit_inputs** → writes the `ActorControlFrame`.

Verb selection by range: a ranged-capable actor pokes at mid-range on a cadence,
then closes for the melee finish; a dash-capable actor bursts to close large gaps.

## Reaction latency — the never-cheats guarantee

`SmashState.obs_history` is a ring buffer of the opponent's recent positions.
Each tick the brain perceives the opponent **as it was `reaction_delay_s` ago**
(self state is read live). It therefore *cannot* frame-perfectly counter a sudden
dash/jump — it must "see" the move first, with a human-scale lag. Pure function
of the tick stream, so replay/RL stay deterministic. Proven headlessly
(`reaction_latency_delays_response_to_a_sudden_move`): after a teleport the brain
chases the stale position for ~`reaction_delay_s`, then flips.

## Difficulty knobs (`DifficultyProfile` + `SmashCfg`)

Per-actor, authored today via the archetype's Smash cfg (EASY/MEDIUM/HARD):

| knob | effect | EASY → HARD |
|---|---|---|
| `reaction_delay_s` | perception lag on the opponent | 0.30 → 0.05 s |
| `commit_probability` | chance to commit the chosen action (vs drop to Idle) | 0.55 → 0.98 |
| `accuracy` | aim-jitter on melee/ranged | 0.65 → 0.98 |
| `mash_speed_hz` | informational swing-rate hint | 1.0 → 2.0 |

Spacing/aggression knobs live on `SmashCfg`: `aggro_radius`, `engage_distance`,
`attack_range`, `too_close_distance`, `chase_speed`, `retreat_speed`,
`dash_to_close`, `crowding_threshold`. The PCA uses MEDIUM (150 ms reaction,
in the brief's 80–160 ms band) via `STRIKER_DEFAULT`.

Cooldowns/costs are NOT special-cased for the AI: melee uses the same
integration-side attack cooldown the player does; ranged/dash use brain-side
cadences; a glider special would pay its `ActionSet` cost like the player's.

## Dropping in a learned policy (the RL seam)

The `Brain` enum is dispatched by match and explicitly reserves `Remote`,
`Scripted`, `RlPolicy` variants (`brain/mod.rs`). To add a learned policy:

1. Add a `Brain::RlPolicy(handle)` variant (or a `StateMachineCfg` sibling).
2. Implement `observe(&BrainSnapshot) -> Observation` and
   `decide(Observation, seed) -> ActorControlFrame`. The smash stages are a
   ready reference for both the feature vector and the action head; you can
   reuse `observe`/`emit_inputs` and replace only stage 2–4 with the policy.
3. Feed it the **same** `BrainSnapshot` (with the `ObsHistory`-delayed opponent
   view if you want the trained policy to inherit the fairness constraint) and
   read the **same** `ActorControlFrame` — so the policy is physically bound by
   the identical ability/cost/cooldown pipeline. No sim code changes.
4. Seed any stochasticity from a **stable actor id** (not `Entity`, whose query
   order isn't stable) so "same observation + same seed ⇒ same action" holds for
   training and trace replay — the hand-authored brain already does this.

Because perception and decision are decoupled from the actor's sim code, today's
utility brain and a future policy are interchangeable behind one trait-shaped
seam.

## Status (PCA encounter)

Done: encounter gate (talk → challenge → fight), grounded reactive Smash boss
(footsies, whiff-punish spacing, jab, dash-to-close, ranged cadence when armed),
real 150 ms reaction latency, determinism, difficulty knobs.

Deferred depth (verbs whose `ActorControlFrame` bits already exist):
- **glider special** — CA-themed diagonal projectile via the data-driven
  ranged/special path (`special_pressed` already emits).
- **reactive block** — `shield_held` within the reaction window (needs an
  incoming-threat read added to `ObservationFrame` + a Defend mode).
- **blink dodge** — `blink_pressed` + `blink_quick_dir`.
- **fly / aerial-Smash** — emit `velocity_target` for a Floating body + 2D
  pursuit, so the PCA fights airborne instead of descending to the ground.
