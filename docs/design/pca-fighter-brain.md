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

## The duelist neutral game (`SmashCfg::DUELIST_DEFAULT`)

Grunts close and mash; a *duelist* plays neutral. Three opt-in knobs (zero on
the grunt defaults, so existing enemies are untouched) turn the Smash brain into
a 1v1 fighter, all added as post-processors in the same seam as
`maybe_substitute_ranged`/`dash` and all **frame-agnostic** (target-relative
only):

- **footsies weave** (`footsies_amplitude`/`_period_s`) — settle around a
  sinusoidally-modulated desired gap: dip into poke range on a rhythm, then back
  out to bait a whiff, instead of camping point-blank.
- **neutral hops** (`neutral_jump_cadence_s`) — occasional jumps that vary the
  approach vector and use vertical space.
- **platform-only vertical chase** (`vertical_chase_min`) — promotes the former
  hardcoded jump-to-close threshold to a knob. Duelists set it *above a hop's
  apex* so they only climb after a target genuinely on a platform, killing the
  flat-ground air-juggle cascade (two fighters leapfrogging each other's hops).

## Aerial play (flying bodies)

A new body-derived seam `BrainSnapshot.actor_aerial` (populated from the body's
`gravity_scale` — the same predicate the integrator uses for `is_aerial`, so
it's production-faithful, not inert) tells the brain to steer 2D
`velocity_target` instead of grounded locomotion + jump. The aerial path skips
the grounded movement refiners and runs `aerial_steer`: a **dive / perch**
oscillation — perch diagonally above-and-beside the target to zone (glider
ranged poke), then dive onto it for a melee, with a cross-up that flips the
perch side between dives. Uses the vertical stage space a grounded brawler
can't. Attack-verb selection stays shared with the grounded path.

## Reactive blink-evade

`can_blink` (capability-gated, cooldowned) lets the fighter dodge a **perceivable
lunge** — the opponent closing faster than a walk, estimated from the *lagged*
target history (reaction latency applies to defense too) — never a privileged
read of the opponent's attack flag, so a human could make the same read. It
blinks up-and-away (into open vertical space, wall-safe without wall geometry).
The body still needs the blink ability for the intent to resolve (capability vs
policy), exactly like the player.

## The non-degeneracy harness (the anti-degenerate guard)

`brain/smash/arena.rs` (test-only) runs two brains in a bounded kinematic arena
(floor, ceiling, walls, platforms; gravity + jump arcs for grounded bodies, 2D
`velocity_target` for flyers; melee knockback; **infinite health** — the bout
studies *movement*, not who wins) and records a trace. It then asserts the trace
is free of the degeneracy signatures: frozen/cornered, tiny looping path, dead
stage columns, one-note verb use. Assertions are **structural/statistical, never
byte-for-byte**, so they survive logic changes — the guard against degenerate
hand-authored (and future learned) policies. Two live guards: grounded duelist
vs duelist, and the flying PCA vs a grounded robot. (Brain-level model — it
proves the *policy* is non-degenerate; in-engine feel is verified separately.)

## Status (PCA encounter)

Done **in the brain crate** (all headless-tested): encounter gate (talk →
challenge → fight), real 150 ms reaction latency, determinism, difficulty knobs;
the duelist neutral game (footsies / neutral hops / platform-only vertical
chase); aerial dive-perch flight; glider ranged zoning; reactive blink-evade;
the non-degeneracy harness.

Remaining (in-game wiring + content — not yet done):
- point the PCA archetype at the duelist/aerial config and keep it Floating when
  provoked (reverse the S4a grounding) so it flies in the Noether Chamber;
- give the PCA body the glider (ranged ActionSet) + blink ability so those
  emitted verbs *resolve* in-engine (capability wiring, like any actor);
- **reactive block** (`shield_held` Defend mode) — the one verb still unbuilt;
- GUI verification of the in-world feel (headless covers the policy only).
