# GPT 5.6 review of `30c7dfcb1` — the correction pass, 2026-08-24

⛔⛔ **READ THIS BEFORE ADDING ANY PARITY ROW.** The review's own summary of the
pattern, and it is correct:

> The agent is building sensible mechanics, but several tests prove a nearby
> SURROGATE road rather than the actual production authority. Clanking and
> helplessness are the strongest examples. I would make **"production-path poison
> before closing a parity row"** the immediate discipline for the next pass.

⭐ **P0-1 IS VERIFIED AT HEAD.** `advance_move_playback` spawns authored volumes
with `Hitbox` + `HitboxHits` + `StrikeVolume` + `StrikeRank` +
`SimId::strike_volume`, and a comment saying *"NO `HitboxLifetime` on purpose"*.
`arbitrate_attack_clanks` queries `With<HitboxLifetime>`. ⇒ **no authored Smash
attack has ever entered the clank system**, and every clank test spawned a
synthetic box carrying the component the production road refuses. That is a
fixture that cannot reach the case.

## Status of the four P0s

⭐⭐ **ALL FOUR ARE VERIFIED AT HEAD 2026-08-24 — the review is right on every
one, and the evidence is a grep rather than an argument:**

```text
P0-1 clank never reaches authored moves   ▢ arbitrate_attack_clanks: With<HitboxLifetime>
                                            advance_move_playback: "NO HitboxLifetime on purpose"
P0-2 helpless never reaches move starts   ▢ trigger_moveset_moves takes &ActorControl +
                                            &ResolvedAttackGesture — no InputState anywhere,
                                            and InputState is the only thing the gate clears
P0-3 sudden death ends on first hit       ✔ BOTH HALVES FIXED: a match already in sudden
                                            death returns before the tiebreak, so the spent
                                            clock cannot decide it; and
                                            open_the_sudden_death_round moved into the sim
                                            schedule (CombatSet::Settle, after
                                            MatchOutcomeDecided) beside the respawn placement,
                                            which writes canonical body state for the same
                                            reason. ⚠ multi-side sudden death still resets
                                            every survivor rather than the tied leaders — a P1
P0-4 zero-velocity items float            ▢ pickup/mod.rs:347 `if item.vel == Vec2::ZERO
                                            { continue; }` — and both the match spawner and
                                            the Z-drop create ZERO on purpose
```

⛔ **NONE OF THE FOUR IS FIXED YET.** The rows are reopened and the ledger no
longer lies; the code still does what is described above.

⚠⚠ **P0-4 WAS ATTEMPTED AND REVERTED — read this before trying again.** Removing
the `vel == ZERO` early-out works for the item itself: a supported item takes one
tick of gravity, predicts penetration and re-settles without moving, and a test
covering both halves (midair drop falls; resting item stays) passed and
poison-verified against the restored early-out.

⛔⛔ **BUT IT BROKE MINT BANKING.**
`death_restores_the_checkpoint::a_mint_banked_where_it_fell_comes_back_where_it_fell`
went red: *"exactly one occurrence must carry `slot:0/0`; found []"*. The banking
pass does NOT read velocity, so the cause is one step further out — a minted item
that now MOVES is no longer where the occurrence sweep expects it, or is gone by
the time it looks. ⇒ the early-out is load-bearing for something beyond physics,
and the review's *"just step every item"* is right in shape but incomplete: the
sibling road that decides an object has COME TO REST has to be settled in the
same slice. Reverted rather than shipped red.

## The review, verbatim

There is a lot of good work here, but I would stop the agent from advancing the
parity inventory for a correction pass. Several rows are currently marked
"shipped" even though their production paths have holes.

The good news first: the previous correction pass stuck. Fresh/respawn recovery
initialization is fixed, respawn grace now owns only its own invulnerability
reason, autolink uses the attacker's frame, Forward Smash rooting looks
reasonable, Robot v3's Pogo is character-specific, Pointed's Up-B is much closer
to the requested shape, Kernel Guide has its own identity, and the composable
defense-presentation design is still intact.

### P0 — clanking does not currently apply to actual Smash moves

This is the biggest finding.

`arbitrate_attack_clanks` queries:

```rust
Query<(Entity, &Hitbox), With<HitboxLifetime>>
```

But `advance_move_playback` deliberately spawns authored moveset volumes as:

```text
Hitbox
HitboxHits
StrikeVolume
StrikeRank
SimId
```

with the explicit comment: `NO HitboxLifetime on purpose` — because the authored
Active window owns their lifetime.

So the new clank tests are testing synthetic `HitboxLifetime` boxes. The actual
Smash jab/tilt/smash/aerial volumes do not enter the clank system.

That means these inventory claims are premature:

```text
Hitbox clanking             ✔
Attack rebound after clang  ✔
```

I would reopen them immediately.

More importantly, don't fix this with only:

```diff
- With<HitboxLifetime>
+ With<StrikeVolume>
```

because the arbitration itself has three problems waiting behind that gate:

1. It sorts by Bevy `Entity` while claiming deterministic ordering. The
   repository already has `SimId::strike_volume`; canonical gameplay ordering
   must use stable simulation identity, not allocator identity.
2. Arbitration is per volume, so two 2-volume attacks can generate multiple
   `AttacksClanked` messages and apply rebound multiple times to the same two
   fighters.
3. `StrongerWins` despawns only one weaker volume and deliberately lets the
   weaker move continue, including sibling/later volumes. That's a volume winning
   a contest the feature describes as an attack contest.

The clean shape is closer to:

```text
live clankable strike volumes
        ↓
stable SimId order
        ↓
find overlapping opposed attack/pulse pairs
        ↓
one deterministic resolution per attack pair
        ↓
trade:
    end both moves once
    rebound both once

stronger wins:
    end/refuse weaker attack coherently
    stronger continues
```

Use `StrikeRank`/`StrikeVolume` and the existing stable hitbox `SimId`; don't
infer clankability from a lifetime mechanism.

The acceptance test needs to use two real `MovePlayback` attacks, not manually
spawned hitboxes.

### P0 — "helpless" is currently mostly cosmetic for combat

`ddb3e3fa` did not actually enforce special-fall/helplessness at the action
authority.

`body_is_helpless()` is currently consulted by the HOME/player movement
integration path, where it clears an `InputState`.

But `trigger_moveset_moves()` does not read that `InputState`. It reads
`ActorControl` and `ResolvedAttackGesture` directly.

So after spending recovery and entering helpless state, a human can still
propose/start aerial attacks and Specials through the moveset resolver.

There is a second asymmetry: the autonomous actor integrator literally passes
`false` for helplessness, with a comment saying enemies don't author recoveries.
That comment is now false for Smash CPU fighters: the common moveset resolver can
spend `BodyJumpState::recovery_charges` on any fighter.

So today the rule is roughly:

```text
human:
    movement kernel knows helpless
    moveset authority doesn't

CPU:
    movement kernel explicitly says not helpless
    moveset authority doesn't
```

This needs one semantic rule.

Also, the current derivation takes *playing ANY move* to mean "not helpless."
That's broader than the intended condition. What should postpone helplessness is
the recovery move that spent the charge still playing, not some unrelated move.

I would move the derivation down to a layer both roads can use and make it
conceptually:

```text
recovery spent
AND airborne
AND recovery playback no longer active
→ helpless
```

Then enforce it at: moveset action-start authority + HOME movement gates +
autonomous actor movement gates.

Allowed while helpless: drift; fall / fast-fall as policy permits.
Forbidden: attack; special; jump; air dodge; other recovery.

Required production tests should cover human and CPU.

### P0 — sudden death ends on the first non-KO hit

The source comments say sudden death continues until the fight ends normally
through last-side-standing. The implementation doesn't.

After sudden death begins, `time_expired == true` forever and
`SuddenDeathEntered == current match`, but `decide_stocks_match()` still re-runs
`decide_on_the_clock(&fighters)` every tick.

Initially both fighters are at equal damage, so it's a draw and the latch
prevents resetting them again. Then fighter A takes even 1 damage without dying:
`A = 151`, `B = 150`. On the next decision the clock tiebreak makes B win and the
match settles immediately.

So the implementation is effectively `sudden death → first damage wins` rather
than `sudden death → continue untimed → last side standing wins`.

Once `SuddenDeathEntered::entered(active)` is true, the clock/tiebreak road needs
to be disabled for that match.

Poison test:

```text
tie at timeout
→ enter sudden death
→ weak non-KO hit
→ match remains live
→ actual KO
→ match settles
```

**Sudden death also crosses the rollback boundary incorrectly.**
`decide_stocks_match` emits `SuddenDeathBegan` inside the simulation schedule.
But Smash consumes it in literal Bevy `Update` (`open_the_sudden_death_round`)
and directly mutates rollback-canonical `BodyHealth` to set the starting percent.
That stage half is gameplay-authoritative and belongs inside the rollback
simulation schedule. During a rewind/resimulation, several simulation steps can
execute without ordinary `Update` replaying between them.

For 1v1, fixing those two is sufficient. Before 4-player/team sudden death,
there's another issue: the event currently resets all surviving fighters, not
only the sides tied for the lead.

### P0 — match-spawned items and airborne Z-drops float in space

`ground_item_physics` says:

```rust
if item.vel == Vec2::ZERO { continue; }
```

so zero velocity means "resting." But two newer roads use zero velocity to mean
"released/spawned without an initial impulse":

- **Match spawning** — `GroundItem { pos: point_above_the_platform, vel: ZERO }`.
  The comment says the item will fall from the authored point. It won't. Physics
  skips it forever.
- **Z-drop** — `Release::Drop => (kin.pos, Vec2::ZERO)`, and its comment likewise
  says `ground_item_physics` takes over afterward. It doesn't.

So `Vec2::ZERO` is currently being asked to mean both *supported and sleeping*
and *free body with zero initial velocity*. Those are different states.

I would probably remove the zero-velocity early-out and let every in-world item
receive gravity; an item already supported by a floor simply predicts penetration
and remains settled. That avoids introducing another rollback state merely as an
optimization. If performance later justifies sleeping, make support/sleep
explicit. Don't derive it from velocity.

Tests: zero-velocity item in midair → falls; airborne Z-drop → falls; match item
spawned above platform → falls onto platform; zero-velocity item resting on
platform → stays there.

### P1 — Exit Match is still offered after the match is already over

The comment says a decided match should not be offered. But the code calculates
`let offer = on_stage && active.is_some();` and never reads `StocksMatchSettled`.
So during the winner/No Contest result card countdown, `ActiveMatch` still exists
and Exit Match remains in the pause menu.

Make the offer: on Smash stage AND `ActiveMatch` exists AND `StocksMatchSettled`
says THIS match is not settled. The stamped API already exists to answer that.

### P1 — match time is currently the wrong time domain

Both the 8-minute match clock and item spawning derive elapsed time from
`ActiveMatch::ticks_since_activation(SimTick)`. But `SimTick` explicitly advances
while gameplay is suspended.

- **Pausing consumes the match clock.** `decide_stocks_match` isn't gated by
  `gameplay_allowed`, and even gating it would not fix the underlying elapsed
  value — the tick count jumps while paused.
- **Opening countdown consumes the match clock.** The 3-second 3—2—1—GO period is
  part of `ticks_since_activation`, so an 8-minute match is effectively 3 seconds
  shorter after GO.
- **Item interval starts at activation, not GO.** The first "every eight seconds"
  drop occurs at elapsed tick 480 = 3s countdown + 5s live combat.
- **Pausing over an item-spawn tick loses the drop.** `spawn_match_items` is
  `run_if(gameplay_allowed)` and tests `elapsed % every_ticks == 0`; if tick 480
  occurs while paused the system doesn't run and that spawn is gone forever.

This wants an explicit match gameplay clock: rollback-canonical
`MatchElapsedTicks` that advances while the live round's clock is running, does
not advance during pause, and begins at GO rather than activation. Then decide
explicitly whether hitstop/bullet-time affects it. I would not blindly reuse
`GameplayElapsed`, because that is scaled simulation seconds and answers a
different question. Use the same match clock for the time limit, item cadence and
other match-timed rules. One time authority.

### P1/P2 — D208's RNG needs a match/session context

The stateless design itself is good — `sim_random(domain, tick, salt)` avoids
shared-stream rollback/schedule hazards. But as currently used every match gets
the exact same sequence: tick 480 → same item, same point; tick 960 → same item,
same point; forever. That's deterministic pseudo-random scheduling, but it will
visibly look scripted across rematches.

Keep the stateless design; add a deterministic context/seed:
`sim_random(domain, match_seed, tick, salt)` where the seed is a synchronized
match/session fact. At minimum mixing the current `MatchInstance` gives
consecutive matches distinct sequences. I would not block the P0 corrections on
this, but I wouldn't use D208 as the foundation for "random stage" until it has
context.

### Rollback note on Exit Match

The current comments imply `Update` writes `MatchAbandoned` + clear message on
rollback = rollback safe. That's not quite true. Clearing abandoned-future
messages prevents stale commands; it does not replay an external menu command if
rollback crosses back over the tick where `Update` translated it. For local
proof-of-concept play this can wait. Before networked rollback uses the
pause-menu action, Exit Match needs to become a host-authoritative/synchronized
command or otherwise live outside rewindable match outcome. **Don't confuse
"message cursor is safe" with "external action is replayable."**

### The stature ledger is stale

`awaiting-maintainer-decision.md` correctly says *Open decisions: NONE* and
records the D32 ruling. But `queue.md` still says STATURE is waiting on Jon. That
should stop. The decision was: no adult standard height; `robot_v3` ≈ 48 and
intentionally short; stature is per-character; leave ambiguous characters
unchanged. If the agent doesn't have enough information to choose a particular
character's stature, that character is simply deferred for visual authoring. It
is not an open maintainer-decision blocker.

### What is genuinely good at this HEAD — preserve without reopening

Forward Smash now starts immediately and roots steering rather than letting the
human movement road run underneath it. Pogo belongs to Robot v3 instead of the
Smash floor. Pointed's Up-B disk/carry/source-frame direction looks structurally
right for the POC. Kernel Guide owns its character identity without a fake combat
kit. Respawn recovery initialization is fixed. Respawn grace no longer borrows
`Empowered`. Defense presentation is cause-based and composable. Damage-only hit
reaction and D202's single restriction phase still look healthier. Stateless
deterministic random access is a good primitive; it just needs a contextual seed.
