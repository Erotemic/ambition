# Smash fun push — one-week implementation campaign

**State:** OPEN product campaign.
**Started:** 2026-08-22.
**Customer:** Super Smash Siblings.
**Timebox:** one aggressive overnight vertical slice, then roughly one week of implementation and tuning.
**Execution authority:** while this campaign is OPEN, use the order in this document for Smash feel work. Re-measure each row against HEAD before editing it.
**Product charter:** [`../super-smash-siblings.md`](../super-smash-siblings.md).
**Feature truth:** [`../smash-parity-inventory.md`](../smash-parity-inventory.md).

When this campaign closes, update shipped/partial rows in the parity inventory,
move durable combat rules to the owning engine/system documentation if needed,
and archive this execution document. Do not leave completed campaign prose as a
second Smash backlog.

> ⚠⚠ **SWEPT AGAINST HEAD 2026-08-24, AND MOST OF THIS CAMPAIGN IS DONE.** Every
> `O` slice plus `W1` and `W6` reads `✔` in
> [`../smash-parity-inventory.md`](../smash-parity-inventory.md), which D72 names
> as the CANONICAL feature truth — this file is only the execution order. A
> session working the headings in sequence would have rebuilt six shipped
> features, and `W6` argued from a claim about the inventory that the inventory
> never made.
>
> ⇒ **still open here: `W2` (autolink knockback, `▢` inventory line 117), `W3`,
> `W4` (`▢` line 201 — and gated on PLAY, not on effort), `W5`, `W7`, `W8`.**
> ⛔ re-grep an inventory row before working a slice; this file goes stale faster
> than the code it describes, and the inventory is the thing to trust.

> ⛔⛔ **A CORRECTION PASS LANDED 2026-08-24 (second GPT review), and its three
> findings share ONE shape** — worth reading before adding another slice, because
> every W-series feature here can make the same mistake:
>
> ```text
> a piece of state initialized by a later INCIDENTAL event
>     recovery_charges came from the landing refresh, so a body that never
>     landed had zero. `Default` is the SPENT state and two fresh-construction
>     paths spelled the fresh state with `..Default::default()`.
>
> a grant claiming ownership it does not STRUCTURALLY have
>     respawn protection borrowed `Empowered`, which is ONE component: granting
>     it overwrote whatever power-up the body carried, and ending the beat
>     removed every semantic in it. A marker cannot make a single-slot component
>     into two independent grants. ⇒ its own clock + `Invulnerability::RESPAWN`,
>     a reason bit, which is the type whose whole purpose is releasing one
>     reason and leaving the rest.
>
> a coordinate system reconstructed from the WRONG BODY'S facts
>     autolink rebuilt its attacker-local anchor from `knockback.dir` (the
>     VICTIM'S away-side) and the VICTIM'S gravity. Both coincide with the
>     attacker's in the ordinary case, so every front-contact same-gravity test
>     passed. ⇒ resolved at the PRODUCER, which holds the attacker.
> ```
>
> ⭐ **and each was invisible to a test that looked reasonable.** The ownership
> test used two DIFFERENT bodies, which is true of any implementation; the
> autolink tests were all front-contact and same-gravity; the budget was correct
> for every body that touched the floor first. ⇒ when a test's fixture cannot
> reach the case the claim is about, it is not evidence for the claim.

## Goal

Make the Smash demo feel substantially more readable, responsive and characterful in ordinary local play without waiting for the pending engine architecture migrations.

The week is successful when a short match visibly communicates:

- a smash attack can be held, visibly charges, and releases harder according to the held charge;
- hard launches leave a readable trail and feel different from ordinary movement;
- invulnerability / intangibility has a clear body tell;
- a successful tech and a successful parry each have distinct feedback;
- shielding reads as an actual shrinking bubble, shield stress is visible, and a break still produces the existing dizzy state;
- Pointed Polygon has a rising spin Up-B whose intermediate hits keep the victim in the move and whose last hit launches;
- KO → respawn → return-to-play has a coherent protected-platform beat if the campaign reaches the match-loop slice;
- the existing CPU/human, rollback and headless paths still use the same body/combat rules.

This is a product push. Do not turn it into the actor-monolith carve, simulation-phase migration, capability/runtime composition campaign, AI-policy relocation, public-facade cleanup, or a generic VFX architecture rewrite.

## Current state to preserve

A first pass must not rebuild features that already ship.

- Shield resource, decay, regeneration, shrink-to-poke, shieldstun, shield pushback, break and dizzy already exist.
- The current shield presentation shrinks and reddens, but it is a thin procedural ring. This campaign upgrades that presentation to a filled/soft bubble.
- Tech, wall tech, knockdown and getup already exist. Tech currently emits the same small dust / Dash-style cue family as getup roll; this campaign gives a successful tech its own readable beat.
- Parry mechanics already exist, including press-timed and release-timed policy. The missing part is a strong parry presentation cue.
- Hit sparks, KO burst and hit-strength camera shake already exist.
- 3–2–1–GO and winner presentation already exist.
- Smash moves already author `smash_charge_mult`. The payoff scaling exists, but the current runtime derives charge fraction from ordinary Startup timeline progress. The Attack hold/release does not actually stop and release the smash timeline. Treat charge input as PARTIAL until this campaign fixes it.
- Pointed Polygon's current Up-B is `polygon_rising_edge`: one rising hit plus a set upward impulse. The moveset runtime already permits genuine multi-hit moves by placing gaps between Active windows; contiguous Active windows intentionally hand their hit set forward and do not re-hit.
- Respawn placement and a timed untouchable grant already exist. The standable respawn platform/drop-off beat is the missing part. A previous action-spends-invulnerability attempt was reverted because held input can retrigger on materialization before the player has acted after respawn.

## Campaign constraints

1. **No architecture prerequisite.** Start feature work immediately on existing supported seams.
2. **One body, one rule.** Human and CPU fighters consume the same charge, defense, hit-reaction and recovery mechanics.
3. **Simulation owns gameplay; presentation consumes explicit facts.** Do not make a shader decide whether a body is invulnerable or whether a smash is charged.
4. **Presentation additions may be cosmetic and non-rollback state.** Gameplay facts that affect damage, velocity, hit eligibility or move timing remain deterministic rollback state.
5. **No raw participant-input dependency in move mechanics.** Charge consumes the body-generic resolved attack/control state used by every controller.
6. **Do not use `CapturedBy` for the spin attack.** Its intermediate hits need autolink/follow-up knockback, not grab ownership or capture suppression.
7. **Prefer a small semantic extension over a new manager.** Three product-driven vocabulary additions are allowed by this campaign: true smash-charge playback state, autolink hit reaction, and explicit presentation facts for charge/unhittable state.
8. **No second implementation of an existing effect.** Extend `ambition_vfx`, the body pose/read-model, or the existing overlay-material pattern where appropriate.
9. **Visual feel is allowed to iterate.** Tests should lock invariants and state transitions, not exact particle counts, blink rates, colors, or final balance numbers.
10. **Each slice lands independently green.** If one slice is blocked, record the blocker in this plan and continue with the next independent slice instead of opening a broad refactor.

## Priority map

The shared engine-primitive references are defined in the parity inventory:

- O1 uses `P01` true move charge.
- O2/O3/O4/O5 use `P14` only where presentation needs a missing resolved fact.
- W2/W3 use `P02` hit reaction policy for autolink.
- W4 uses `P12` only if repeated recovery is confirmed in play.
- W6 uses `P09` if shield-drop commitment is added.

### Overnight target — highest visible return

- [ ] O1. True held/released smash charge.
- [ ] O2. Smash-charge pose, accelerating blink, and charge-start/full cues.
- [ ] O3. Hard-launch smoke/trail feedback.
- [ ] O4. Invulnerability/intangibility body blink.
- [ ] O5. Distinct tech flash and parry flash/chime.

The overnight target is a vertical slice, not a demand to finish every row before the next calendar day. O1 is the only simulation-heavy item. O2–O5 should proceed independently if O1 takes longer.

### Week target — gameplay signature + defensive readability

- [ ] W1. Replace the thin shield ring with a real shrinking bubble presentation.
- [x] W2. Add a reusable deterministic autolink/follow-up hit reaction. — SHIPPED (`HitKnockback::follow`, `hit_response::autolink_velocity`).
- [x] W3. Re-author Pointed Polygon Up-B as capture-reading multihit → launch using W2. — SHIPPED.
- [ ] W4. Add a once-per-airtime recovery-use budget if Up-B is currently repeatable in air.
- [ ] W5. Add the standable respawn platform and drop-off beat.
- [ ] W6. Add shield-drop lag if playtesting shows defense has no meaningful release commitment.
- [ ] W7. Add shield-break dizzy stars / shield-stress polish if the main slices are complete.
- [x] W8. Tune the demo as a whole against a small recorded playtest matrix. — PLAYED 2026-08-24; its four findings are closed. See `demos/w8-playtest-2026-08-24.md` and status.md's "W8's four findings closed". ⛔ this box stayed unchecked while status recorded the playtest as done, which is the stale-prose trap the reviewer guide names.

### Explicitly deferred unless the week finishes early

Ceiling tech, pivot grab, grab-release animation depth, command grabs, sudden death, items, Final Smash, directional taunts, announcer, voice lines, screen/star KO variants, full results screen, Battlefield/Omega stage forms, advanced dash-dance/foxtrot/pivot surface, charge storage, and training-mode UI.

These are valid parity items. They are lower return for this campaign than making the existing core combat readable and satisfying.

---

## O0 — Baseline and instrumentation

Do this before behavior changes. Keep it short.

### Work

- [ ] Run the current Smash integration target:

  ```bash
  cargo test -p ambition_demo_smash_app
  ```

- [ ] Run the visible demo once after mirroring worktree assets if needed:

  ```bash
  python3 scripts/mirror_assets_for_worktree.py
  cargo run -p ambition_demo_smash_app --bin smash_demo --features visible
  ```

- [ ] Verify by hand that the current build has: ordinary smash attack, shield shrink/break/dizzy, tech, parry, Pointed Polygon Up-B, KO/respawn.
- [ ] Record only concrete defects discovered during this pass in this campaign document. Do not start a second parity inventory.

### Exit

The agent can reproduce the current demo and knows which proposed rows are presentation upgrades versus missing mechanics.

---

## ✔ O1 — True held/released smash charge — SHIPPED

### Problem

`MoveSpec::smash_charge_mult` currently scales damage/knockback by `MovePlayback.t` through the leading Startup window. The resolved input already carries press/held/release information, but `trigger_moveset_moves` starts a smash from the press and the move clock then advances normally. A player cannot hold Attack to keep charging and release to fire the smash.

### Settled design

A chargeable smash use has explicit playback state. The ordinary move timeline reaches an authored charge hold point, pauses there while the attack is held, accumulates charge time, and resumes on release or at the maximum charge time.

Prefer an authored move policy:

```text
SmashChargeSpec
  hold_at_s
  max_hold_s
```

attached to `MoveSpec` as an optional charge policy. Keep `smash_charge_mult` as the payoff multiplier. The authoring helper for Smash moves may stamp a sensible default policy so every existing fighter does not need hand-written boilerplate.

The current `charge_fraction_at(t)` timeline interpretation is replaced for a charge-active use by:

```text
charge_elapsed / max_hold_s
```

The released fraction becomes the multiplier authority for the rest of that move use. Do not let later timeline progress continue increasing the charge after release.

Only a move actually started through the Smash gesture enters smash-charge mode. A `MoveSpec` reused by another verb must not become chargeable merely because its multiplier is greater than one.

### Playback behavior

- [ ] Tap Attack: the smash reaches the hold point, sees release/no hold, and continues with near-minimum charge.
- [ ] Hold Attack: the timeline freezes at the hold point; `charge_elapsed_s` increases in owner proper time.
- [ ] Release Attack: freeze ends on that tick and the move continues.
- [ ] Reach maximum: auto-release even if Attack remains held.
- [ ] Charge fraction is frozen at release and applies uniformly to every hit generated by the move use.
- [ ] Hitlag / global pause must not advance charge in a way ordinary move proper time would not advance.
- [ ] A CPU that only taps Smash continues to work. CPU charge strategy is outside O1.
- [ ] Cancel rules remain authored; charging does not create a free cancel unless an existing window permits it.

### Ownership

- `MoveSpec` owns charge authoring.
- `MovePlayback` owns per-use charge timing and the released fraction. This component is rollback state already.
- The body-generic resolved attack/control state supplies held/released input. Do not query keyboard/controller state here.
- `advance_move_playback` remains the authority for the move clock and hitbox timing.

If the input observation and move-clock update need an explicit system edge, add that local schedule edge. Do not wait for the workspace-wide simulation-phase campaign.

### Tests

- [ ] Tap gives minimum/near-minimum multiplier.
- [ ] Half hold gives an intermediate multiplier.
- [ ] Full hold gives exactly the authored `smash_charge_mult` cap.
- [ ] Continuing to hold after max auto-releases and does not exceed the cap.
- [ ] Release fraction no longer changes while Startup/Active/Recovery continue.
- [ ] A non-Smash use of a move does not enter charge mode.
- [ ] Proper-time scaling affects charge consistently with move playback.
- [ ] Rollback/snapshot resolution preserves charge elapsed/released state.
- [ ] Existing non-charge moves remain byte/behavior equivalent where practical.

### Exit

With a human controller, holding a Smash input visibly delays the hit until release/max and changes the resulting launch strength.

---

## ✔ O2 — Smash-charge pose, accelerating blink and SFX — SHIPPED

### Goal

The player can read three beats without looking at debug state:

1. **latched** — charge has started;
2. **building** — charge is increasing;
3. **loaded** — maximum charge has been reached and release is imminent/available.

### Work

- [ ] Publish charge presentation from simulation/read-model as `None` or normalized `0..=1` charge fraction. Do not have rendering derive it from move names or Startup time.
- [ ] Route the existing `smash_charge` character row while charge is held. The art already exists for at least the established sheet vocabulary; verify consumer reachability before adding art.
- [ ] Reuse the existing character overlay-material pattern used by hit flash. Add a charge mode rather than spawning a second body sprite tree.
- [ ] Blink/pulse frequency increases monotonically with charge fraction. Start readable and slow; near full should pulse rapidly. Keep the exact curve a presentation tuning constant.
- [ ] The pulse must preserve the underlying character silhouette and facing.
- [ ] Add a generic **mechanical latch/cock** SFX when charge begins.
- [ ] Add a short higher-pitched **loaded/lock** cue once when max charge is reached. This is strongly recommended even though the original request only requires the start cue.
- [ ] Route cues through the existing SFX provider/source seam so character-specific overrides can be added later without changing charge mechanics.

### Acceptance

- The start sound fires once per charge attempt.
- The full sound fires once only if max is reached.
- Releasing early stops the charge pulse immediately and the actual attack animation continues.
- A CPU and a player look the same when they occupy the same charge state.

---

## ✔ O3 — Hard-launch smoke / speed trail — SHIPPED

### Goal

A fighter launched hard should carry a velocity trail for part of the flight. Ordinary running, jumping and intentional fast movement must not emit the same effect.

### Design

Gate the effect on a semantic **launched/tumble/hitstun** fact plus speed. Do not gate on world velocity alone.

Presentation may emit particles from a read-model each render/update cadence; it does not affect simulation and does not need rollback state. Avoid generating duplicate long-lived effects from rollback resimulation if the existing VFX subscriber already has a dedup/cosmetic boundary to use.

### Work

- [ ] Publish or reuse a body pose fact that distinguishes launched/tumble state from voluntary motion.
- [ ] Add `Smoke` as a `ParticleKind` only if the existing Dust recipe cannot produce a convincing trailing plume. Prefer extending the existing particle vocabulary over a special Smash-only particle system.
- [ ] Spawn particles behind the velocity vector, with emission density increasing above a hard-launch threshold.
- [ ] Fade emission when speed drops or tumble/hitstun ends.
- [ ] Keep existing hit spark and camera shake; this is a flight/readability layer, not a replacement for impact feedback.
- [ ] Optionally add a second denser threshold for near-KO launch speeds after the basic trail reads correctly.

### Tests

Test the gating predicate, not exact visual tuning:

- launched + high speed -> trail requested;
- launched + low speed -> no trail;
- voluntary high-speed movement -> no launch trail;
- returning to normal control stops trail requests.

### Exit

A spectator can identify a hard launch from the flight itself even after the initial hit spark leaves the screen.

---

## ✔ O4 — Invulnerability / intangibility blink — SHIPPED

### Goal

A body that cannot currently be struck because of i-frames has a clear blinking/tinting tell.

### Design

Publish one resolved presentation fact from the same simulation state that controls hit eligibility. Rendering should not inspect animation names or duplicate the vulnerability predicate.

The visual should cover the common body-generic cases relevant to Smash:

- dodge / spot dodge / air dodge intangibility;
- tech/getup invulnerability where the movement state grants it;
- ledge intangibility;
- timed respawn `UNTOUCHABLE` grant;
- other actual body invulnerability grants that use the shared health/state vocabulary.

Parry gets its own flash in O5; it may share the same overlay machinery, but a successful parry event should still read as a distinct beat.

### Work

- [ ] Add a resolved `unhittable` / `i_frame_visual` fact to the body presentation read-model. If there are semantically different states that require different colors later, use a small enum; do not pre-generalize a full status-effect renderer.
- [ ] Feed that fact from authoritative simulation state.
- [ ] Reuse/generalize the hit-flash overlay sibling or material.
- [ ] Blink at a stable cadence independent of frame rate. A sim-tick-derived phase is acceptable if it keeps visible behavior deterministic enough for capture/replay.
- [ ] Resolve overlay priority with damage flash and charge flash explicitly.

### Acceptance

- A dodge, tech, ledge i-frame and respawn protection show the tell for exactly their actual unhittable interval.
- The body stops blinking when it becomes hittable again.
- A merely raised shield after the parry window does not use the invulnerability blink.

---

## ✔ O5 — Tech and parry feedback — SHIPPED

These mechanics exist. This slice gives each a distinct visual/audio signature.

### Tech

Current `MovementOp::Tech | MovementOp::GetupRoll` shares a Dash SFX and a small dust burst. Split the successful Tech arm from GetupRoll.

- [ ] Tech emits a bright short flash/ring/spark at contact plus a small dust response.
- [ ] Tech gets its own crisp SFX cue.
- [ ] Wall tech may reuse the cue unless surface identity is already available. Do not add a surface enum only to color this effect.
- [ ] If the body is also in the i-frame blink from O4, the one-shot tech flash takes visual priority for its few frames.

### Parry

- [ ] Emit a high-contrast shield/body flash on a successful parry, not merely when the parry window opens.
- [ ] Emit a distinct chime/clang on the successful contact.
- [ ] Keep ordinary shield block feedback separate.
- [ ] Projectile parry/reflection should reuse the same semantic successful-parry presentation where practical.

### Exit

In a noisy CPU-vs-CPU match, a spectator can distinguish ordinary shield block, parry, tech and getup roll without reading state text.

---

## ✔ W1 — Real shrinking bubble shield — SHIPPED

### Problem

`render/rendering/bubble_shield.rs` currently draws a thin anti-aliased ring. The simulation already owns shield integrity and the read-model already publishes position, size, parry state and integrity.

### Work

- [ ] Preserve `ShieldRingsView` as the presentation input.
- [ ] Replace the thin ring texture with a soft filled bubble/field presentation: translucent interior, bright rim, and readable overlap around the fighter.
- [ ] Continue shrinking coverage from integrity and reddening toward break.
- [ ] Keep the parry-window color/read, but coordinate it with O5 so a successful parry has a stronger one-shot flash.
- [ ] Add a brief shield-hit pulse/deformation if shieldstun is available in the read-model cheaply. If it requires threading gameplay policy through rendering, add only the resolved presentation scalar/fact.
- [ ] Keep existing break shard burst and dizzy lock.
- [ ] If cheap, add dizzy stars orbiting the body while `break_timer > 0`. The existing `dizzy` character pose remains the base animation.

### Gameplay check

Do not retune shield health, decay, regeneration, poke coverage or break duration in the same commit unless visual playtesting exposes an actual balance defect. Separate mechanics tuning from the presentation replacement.

### Exit

At full health the shield reads as a protective bubble; at low health it is visibly smaller/redder; a break visibly transitions into the existing dizzy state.

---

## ✔ W2 — Autolink/follow-up knockback primitive — KERNEL SHIPPED 2026-08-24

**What landed.** `HitKnockback::follow: Option<AutolinkFollow>` and
`hit_response::autolink_velocity`, resolved inside the shared
`apply_body_hit_reaction` beside the ordinary launch — one velocity, written
once, under the victim's own body authority.

```text
anchor_local   attacker-local follow point (x forward along its facing,
               y toward its feet) — resolved through the VICTIM'S
               AccelerationFrame, so a move authored once works in a
               rotated room
carry          share of the ATTACKER'S own velocity handed over. This is
               what makes a RISING multi-hit work: the correction only
               closes a gap, and a fighter climbing at 600 px/s outruns
               any gap-closing term
pull           spring gain on the remaining gap (1/s) — how HARD this
               move grabs is a feel decision per move
max_speed      bounds the CORRECTION only. Clamping the carry would make
               a fast attacker's victim fall out of its own move
source_vel     the attacker's velocity at the pulse, sampled by the
               PRODUCER because the reaction holds a victim and no
               attacker entity
```

⛔ **NOT A CAPTURE, by construction:** no `CapturedBy`, no hold clock, no escape,
and the victim keeps every verb it had. What holds it is that each pulse re-aims
its velocity — a hit reaction like any other.

⭐ **two judgment calls, stated at the source.** Crouch-cancel does not scale an
autolink (crouching shortens a LAUNCH and there is nothing here to shorten), and
an autolink is **never a meteor**: the lock keys on *"velocity points toward the
feet"*, which is true of any anchor placed below the attacker, so a spinning move
that gathers its victim underneath would otherwise be charged the genre's meteor
silence for holding somebody.

⚠ **wire-format change, declared:** the follow is IN the hit fingerprint, because
two peers that disagree about whether a pulse holds or launches disagree about
the whole match. Schema **77 → 78**; only the version line of the baseline moved,
since no registration changed.

**Guards** — six kernel tests (`autolink_tests`) plus one through the real shared
reaction (`an_autolink_pulse_aims_the_victim_back_at_its_attacker`), and the
assertions are DIRECTIONS and DIFFERENCES rather than magnitudes: the same
knockback with the follow removed launches AWAY, and with it aims BACK; the carry
is measured against the same geometry with a still attacker; the frame test fails
a world-axis implementation that passes every other one.

✔✔ **AND THE AUTHORING PATH IS WIRED, so this is reachable from content rather
than a green field nothing can feed.** `AutolinkVolume` on the catalog's
`HitVolume` (serde-default, so not one shipped `.ron` changed) → `Hitbox` →
the producer. ⭐ **the producer is where the ATTACKER'S VELOCITY is sampled**,
and it has to be: the reaction holds a victim and no attacker entity, and the
velocity is a fact about the PULSE rather than about the move. A wiring that
carried the anchor and dropped the velocity would pass every kernel test and drop
its victim in play, so `an_authored_autolink_reaches_the_hit_payload_with_the_attackers_velocity`
drives the real producer and asserts the sampled value, with an unauthored swing
as its poison.

▢ **still open on this slice:** an authored MOVE that spends it. `W3` is that
move, and the campaign deliberately separates the primitive from its first
customer.

### Original specification



### Goal

Support authored multi-hit moves whose intermediate hits keep a victim close enough for later pulses while leaving the final hit free to use ordinary launch knockback.

This is the reusable mechanic Pointed Polygon's Up-B needs.

### Semantics

Add an optional hit-reaction mode to an authored `HitVolume`/knockback payload. Name it around **autolink** or **follow-owner** semantics. Do not call it capture.

An autolink hit should produce a target velocity based on:

- the attacker's current motion;
- the vector from victim toward an authored follow point around the attacker/hitbox;
- bounded correction strength/speed.

The intermediate reaction must remain deterministic, gravity/frame-correct and body-generic. Prefer velocity steering/set-knockback over teleporting the victim to the attacker.

The design must allow:

```text
intermediate Active pulse -> autolink
intermediate Active pulse -> autolink
intermediate Active pulse -> autolink
final Active pulse        -> ordinary launch
```

The existing move-runtime re-hit rule already supports separated Active windows. Preserve contiguous-window handoff semantics for moving one-hit tracks.

### Avoid

- no `CapturedBy` relationship;
- no disabling the victim's entire control model beyond the ordinary hitlag/hitstun the intermediate hits author;
- no Pointed-Polygon-specific branch in generic combat;
- no world-axis-only correction that breaks under non-default gravity;
- no post-collision teleport that bypasses body motion authority.

### Tests

- [ ] Autolink follows a stationary attacker toward the authored local follow point.
- [ ] Attacker velocity contributes to victim carry.
- [ ] Facing mirrors the local follow point.
- [ ] Non-default gravity rotates the authored local relation correctly.
- [ ] Speed/correction is bounded.
- [ ] Standard hit volumes retain current knockback byte/behavior.
- [ ] Separated Active windows can re-hit the same victim; contiguous track windows still do not.

### Exit

A small synthetic multihit reliably holds a victim through several separated pulses without a capture relationship and then permits a normal final launch.

---

## ✔ W3 — Pointed Polygon rising spin Up-B — SHIPPED 2026-08-24

`polygon_rising_edge` is now **four holding pulses then one launch**, authored
through a new shared combinator rather than a Polygon-shaped branch:

```text
multihit(strike(<the finisher, unchanged in character>), 4, Pulse { … })

pulse    2 damage · 0.035 s live · 0.030 s gap · autolink anchor (14, 6)
         carry 1.0 · pull 22 · max 900
finisher 7 damage · knockback 88 · growth 1.65 · launch (0.10, -1.0)
```

⭐ **why the rise needed this at all.** The move was ONE hit on the way up, so it
either connected once and sent the victim away or missed — the climb had no
reason to be long. The pulses make the rise the mechanic: each re-aims the victim
at a point just in front of the spinning fighter, so it comes UP with the move
and the finisher has something to launch. `carry: 1.0` is not a flourish — this
fighter rises at 760 px/s and the correction only closes a gap, so anything less
leaves the victim underneath its own move.

⛔⛔ **THE GAPS ARE LOAD-BEARING, NOT SPACING.** The move runtime's re-hit rule
lets SEPARATED Active windows strike the same victim again and refuses it across
a contiguous track — so a multi-hit authored as one long window, or as windows
that touch, lands exactly ONCE and the mechanic silently does not exist.
`every_pulse_is_a_separated_window_so_each_one_can_re_hit` is the guard, and it
is the one that would have caught that.

⭐ **`multihit` is a COMBINATOR over `strike`, not a second builder** — the
finisher stays an ordinary strike with an ordinary launch and the lead-in is
inserted in front of it, so a multi-hit cannot drift away from what a plain move
means. Four guards: the separation above, that the pulses HOLD while only the
last hit LAUNCHES, that the finisher is pushed BACK by the lead-in rather than
overwritten (and recovery still reaches the end), and the poison — zero pulses is
the plain strike, untouched.

⛔ still NOT a capture: each pulse is an ordinary weak hit whose reaction aims
inward. The victim keeps every verb, can DI, can tech the ending, and falls out
the moment the pulses stop reaching it.

### Original specification



### Intent

Replace `polygon_rising_edge` with a recognizable rising spin sequence:

```text
commit / rise
  -> first autolink catch
  -> several light autolink spin hits
  -> strong final launch
  -> recovery / landing commitment
```

The move should read like a temporary trap because the intermediate knockback keeps the victim in the hit sequence. It remains strike semantics.

### First tuning target

Use a short sequence, roughly four light intermediate pulses plus one final hit. Exact counts and timings are tuning, not an API contract.

- Intermediate hits: low damage, low ordinary launch, autolink reaction, modest hitlag/hitstun.
- Final hit: ordinary authored knockback with enough vertical/outward launch to end the sequence decisively.
- Keep the upward self-impulse, but tune it so the victim stays near the spinning fighter rather than falling below the hitboxes.
- Keep meaningful landing/recovery commitment.

### Presentation

- [ ] Use an existing spin/special-up row if one is actually authored and reachable; verify before adding art.
- [ ] Otherwise use the current Up-B pose plus repeated slash-arc VFX around the body. Do not block the mechanic on new sprite art.
- [ ] Intermediate hits use lighter hit feedback than the final hit.
- [ ] Final hit gets the ordinary strong-hit cue/camera response appropriate to its launch.

### Acceptance matrix

Test against at least:

- low damage, reference-weight victim;
- medium/high damage victim;
- light victim;
- heavy victim;
- victim slightly left/right of the first hit;
- rising attacker with horizontal drift.

The intended outcome is not a guaranteed combo from every possible edge contact. The common central connects should carry through to the final hit with high reliability.

### Exit

Pointed Polygon has a signature recovery/attack that visibly catches, spins through multiple hits, and launches on the final beat.

---

## ✔ W4 — Once-per-airtime recovery use — SHIPPED 2026-08-24

⭐⭐ **ITS PRECONDITION WAS CONFIRMED AT THE SOURCE, not by play**, which is
stronger: `MoveSpec` carries no cooldown, no cost and no per-airtime rule, and
`MoveGates` knew only `grounded` — which cannot tell the second use in one
airtime from the first. ⇒ **a fighter authoring a rising special could press it
forever and could only be killed by a launch that outran its own recovery. A
platform fighter in that state has no bottom blastzone.**

```text
BodyJumpState::recovery_charges   the budget, an INTEGER not a flag
MoveGates::recovery               what it costs: nothing, spend+freefall,
                                  or spend-and-keep-acting. For a smash up-B the
                                  SLOT authors it (`UpSpecial`), not the moveset
afford_recovery                   refuses the start; asked BEFORE a cancel tears
                                  the current move down, or the body is left
                                  with neither
refresh_movement_resources_…      gives it back when the body is RE-SEATED:
                                  landing, catching the ledge, being grabbed,
                                  a respawn — and deliberately NOT a hit
```

⭐ **AN INTEGER, as the slice asked.** The genre's default is one
(`DEFAULT_RECOVERY_CHARGES`); a fighter wanting two is an ordinary tuning
statement, and the budget is already an integer precisely so that costs nothing.
⛔ the count is a CONSTANT until a fighter wants a different one — authoring the
field before there is a customer is a knob nobody turns.

⭐ **AND THE SPEND SITE DELETED A DUPLICATE.** The cancel path and the plain
trigger each did the same three things — insert the playback, spend the buffered
proposal, spend the guard — in two copies, and this rule had to join them. One
`start_move` now does all four; adding a fourth line to a two-copy tail is how
this repository loses a rule down one road.

⛔ authored by the MOVE, never inferred from its name or its impulse: an
up-special that does not lift, and a side-special that does, are both ordinary
statements this way and neither is a special case in input code.

Schema 79→80. Three guards: all three affordability states (ordinary move never
asks · charges left allowed · none left refused), the bare-fixture case, and that
the landing-class refresh restores it — with a poison on the default being
non-zero, or that last assertion would hold for a refresh that restores nothing.

### Original specification

Do this after W3 only if direct play confirms the current Up-B can be repeated indefinitely in one airtime.

### Design

Add a small body-generic recovery-use budget/state reset by grounded/ledge recovery. The special move authors that it spends this budget. Do not special-case Pointed Polygon in input code.

A later fighter may author two recovery charges; avoid encoding the state as a Pointed-only boolean if a tiny integer budget costs the same.

### Tests

- first airborne recovery allowed;
- second denied while still airborne;
- landing resets;
- ledge recovery/reset policy is explicit and tested;
- grounded Up-B remains available according to the authored move gate.

---

## ✔ W5 — Respawn platform and drop-off — SHIPPED 2026-08-24

⭐⭐ **THE HALF THAT WAS A LIVE DEFECT LANDED FIRST, and it needed no platform.**
Respawn protection was a flat two-second timer that NOTHING could end, so a
returning fighter could attack while untouchable — a free hit every stock, taken
from whoever had just earned the knockout. Smash's platform releases you on your
first action for exactly this reason, and the release is the anti-camping rule;
the platform is the presentation of it.

⇒ **swinging spends the grant.** The trigger is a move's PLAYBACK appearing — a
body committing to something, not a held button and not a movement axis — so a
fighter still gets to fall in, drift and choose a landing under protection, and
loses it the moment it uses the window to attack from.

⛔⛔ **AND IT KEYS ON A MARKER, NOT ON THE TRAIT.** `UNTOUCHABLE` is a
CAPABILITY, not a claim about who granted it: Sanic's super state and Mary-O's
star hold the same one. Ending "the grant whose traits look like this" is release
by VALUE EQUALITY, which is not ownership — it would strip a power-up somebody
else gave the same body, and it would go wrong silently the first time a third
granter used the trait. `RespawnGrace` (`ambition_combat::stocks`, rollback-
registered, schema 78→79) is what the ruleset marks and the only thing it
removes. The guard's poison is a second body with the identical `Empowered` and
no marker: a value-equality implementation passes the first assertion and fails
that one.

⚠ **rollback: the grace is SIM state.** A rollback that lost it would resurrect a
fighter's invulnerability; one that kept it after the fighter acted would hand
back a grant already spent.

✔✔ **AND THE PLATFORM FOLLOWED, inheriting the release rule rather than
inventing a second one.** `hold_the_respawn_platforms` keeps one stationary
platform under each protected fighter, present **iff** that seat's body carries
`RespawnGrace`.

```text
representation  MovingPlatformState::from_sweep(.., dx: 0.0, speed: 0.0)
                pushed into MovingPlatformSet — the smallest existing standable
                thing, already rollback-canonical, and its renderer draws it
                for free
lifetime        = RespawnGrace's. ⛔ NO CLOCK OF ITS OWN: a platform with its
                own duration is how a fighter ends up standing on a beat it
                already spent
order           sorted by id, so the Vec is a function of WHICH seats are
                protected and never of query order — the resource is
                rollback-canonical and the visuals reconcile by index
collision       ordinary. Anybody may stand on one, and anybody standing on one
                when it goes falls — which is the genre's answer too
```

⭐ **the latch this closes:** the marker was removed by a SWING and nothing else,
while its `Empowered` expires on its own clock — so a fighter that never swung
would have kept the marker, and now the platform, for the rest of the match. The
system retracts the marker when the grant is gone, and the third assertion of
`the_respawn_platform_lives_exactly_as_long_as_the_grant` is exactly that case.

⚠ **and a hazard I talked myself INTO and back out of, worth writing down:** the
visuals reconcile by INDEX, so per-seat add/remove looked like it would alias one
platform's art onto another. It does not — `sync_moving_platform_visuals`
re-reads each visual's index every frame and updates transform AND size, so a
shifted index simply draws the right thing. ⇒ read a reconcile before pricing an
index hazard; "keyed by index" and "aliases on reorder" are different claims.

⛔ what is NOT built and is not needed: a per-owner solidity filter. Respawn
platforms are solid for everyone in this genre.

### Original specification



### Goal

Finish the match-loop mechanic already identified by the parity inventory: materialize a returning fighter on a temporary standable platform, hold the protected neutral beat there, then drop/release the fighter into ordinary play.

### Requirements

- [ ] Platform is an actual standable/kinematic world object or the smallest existing reusable platform representation that obeys ordinary body collision.
- [ ] Respawning fighter starts on it with zero launch velocity.
- [ ] Ordinary combat actions cannot fire while the platform owns the protected waiting beat.
- [ ] Movement/drop input or timeout releases the fighter; define the exact genre-like behavior during implementation from the existing control vocabulary.
- [ ] The current timed untouchable grant remains the protection authority.
- [ ] Once ordinary post-platform action begins, protection ends according to the chosen rule without consuming itself from stale held input on the materialization frame.
- [ ] Multiple simultaneous respawns remain spatially separated.

### Exit

KO → respawn has an intentional visual/gameplay beat instead of a fighter appearing in free air with only an invisible timer.

---

## ✔ W6 — Shield-drop lag — SHIPPED (this slice's own premise was false)

⛔⛔ **THE SLICE SAID *"the parity inventory lists shield-drop lag as absent"*.
IT DOES NOT, and did not** — the inventory row has read `✔` with the number in it.
Checked at HEAD 2026-08-24, the rule is wired end to end:

```text
ShieldTuning::drop_lag          0.0 baseline · 11/60 s on PLATFORM_FIGHTER
apply_shield                    arms drop_lag_timer when a guard is let go BY
                                ITSELF (an out-of-shield action already spent it)
movement/mod.rs                 decays it
features/ecs/attack.rs          consumes it into hard_lock_timer, beside the
                                knockback lock, the break dizzy and shieldstun
```

plus codec encoding, snapshot registration, a dev-tools slider, and two kernel
tests that assert BOTH sides (zero when no rule is declared, non-zero when one
is). ⇒ nothing to build.

### Goal

Dropping shield should cost a short authored/control-locked release window so holding guard has a readable commitment and shield pressure has a payoff.

### Constraints

- keep the lag body-generic and in movement/control state;
- do not make rendering or the settings menu own the timer;
- parry timing policy remains separate;
- expose the duration as a tuning knob on the existing shield/movement tuning owner.

### Tests

- releasing shield starts the lag;
- attack cannot begin during the hard portion;
- movement behavior during the lag matches the chosen tuning contract;
- dodge/roll escape behavior is explicit rather than accidental scheduling;
- a shield break uses its existing much larger dizzy lock, not shield-drop lag.

---

## ◐ W7 — Small presentation polish — FIVE OF SIX ALREADY SHIPPED

Checked against the inventory 2026-08-24 rather than worked in order:

- [x] Dizzy stars for shield break — `rendering/dizzy_stars.rs`, inventory ✔.
- [x] Strong shield-hit pulse / ripple — inventory ✔.
- [x] Distinct max-charge cue — `Charge-start/full cues and charge pose`, ✔.
- [x] Strong-launch trail density/shape — `Strong/near-KO launch trail tier`, ✔,
      layered on the base trail's own launch fact.
- [x] Strong-hit impact flash — ✔, and it already scales with strength.
- [x] **HUD damage-percent punch** — SHIPPED 2026-08-24. `HudStanding::emphasis`
      is a presentation PRIMITIVE (`0..=1`, default `0.0`, so every non-fighting
      HUD draws exactly what it drew); the renderer scales the value text by it.
      ⭐ the game derives it from `BodyCombat::hitstop_timer`, which is non-zero
      exactly when a hit lands and is already scaled by the damage — so the punch
      reads the SAME fact the player felt. ⛔ NOT a percent delta tracked in
      presentation: that is a second answer to a question the sim answers, and
      the two part company the frame a hit is blocked, absorbed by armor, or
      lands for zero.
      ⛔⛔ **the node carries its DECLARED font size**, because the scale is
      applied every frame: scaling whatever the font is NOW compounds, and a
      readout held under emphasis for one second draws about four thousand times
      its size. Invisible in a single-tick test, which is why the guard runs
      sixty.
- [ ] Pointed Polygon final-spin slash arc cleanup — new since W3 landed; the
      pulses currently draw the POKE tag and the finisher the arc, which is
      already the right split. Look at it in play before changing art.

⭐ **the HUD punch has an honest source waiting for it:** `BodyCombat::hitstop_timer`
is non-zero exactly when a hit just landed and is already scaled by the damage
(`hitlag_duration`). A punch driven off that reads the SAME fact the freeze does,
needs no new sim state, and cannot disagree with what the player felt. ⛔ do not
track a percent DELTA in presentation to derive it — that is a second answer to a
question the sim already answers.

Do not start announcer, voice or results-screen systems from this row.

---

## W8 — Playtest/tuning pass

### Small matrix

Run repeatable 1v1 matches in these modes:

1. P1 vs standing dummy/idle CPU for input/readability;
2. P1 vs CPU for ordinary pressure/defense;
3. CPU vs CPU for spectator readability and systemic behavior;
4. Pointed Polygon vs a light fighter;
5. Pointed Polygon vs a heavy fighter.

### Observe

Record concrete observations only:

- Can a player deliberately tap, half-charge and full-charge a Smash?
- Is charge state readable without a meter?
- Does the launch trail trigger on real launches and stay absent on fast voluntary movement?
- Are i-frames readable without obscuring attack art?
- Can block, parry, tech and dodge be distinguished at full match speed?
- Does low shield integrity look endangered before it breaks?
- Does the spin Up-B carry common connects through to the final hit across damage/weight ranges?
- Can Pointed Polygon recover infinitely if W4 was not needed?
- Does respawn protection behave correctly with an attack button held through KO/respawn?
- Are CPU fighters still obeying the same move and defense rules?

### Tuning policy

Tune data before changing mechanics. Keep final values in their existing owners:

- move timing/damage/knockback in movesets;
- shield values in shield/movement tuning;
- rule-set values in declared combat rules;
- visual rates/thresholds in presentation tuning/constants.

Do not bake a one-character feel correction into a generic kernel unless another character demonstrates the same need.

---

## Verification tiers

Use the narrowest sufficient test while iterating, then run the assembled gates before finalizing the campaign.

### Per combat/movement change

```bash
cargo test -p ambition_demo_smash_app
```

Run the directly owning crate tests as needed (`ambition_combat`, movement/core, render/VFX, content).

### Before push/finalization

```bash
cargo check -p ambition_app
cargo test -p ambition_demo_smash_app
cargo test --workspace --lib
```

When changing architecture/dependency ownership rather than gameplay behavior, also run:

```bash
cargo test -p ambition_workspace_policy
```

Do not run `cargo test --workspace --tests`.

### Visual gate

For every presentation slice, run the visible Smash demo and exercise the exact state. A green headless test cannot validate blink rate, bubble readability, launch smoke density or final-hit presentation.

---

## Campaign checkpoints

### Checkpoint A — first vertical slice

Required:

- true held/released charge works;
- charge start is audible;
- charge buildup is visible;
- hard launch has persistent flight feedback;
- i-frames blink;
- tech/parry have distinct cues.

If A is reached, the demo should already feel substantially richer without any new character mechanic.

### Checkpoint B — defense pass

Required:

- shield reads as a bubble rather than a ring;
- shrinking/reddening/break/dizzy remains correct;
- parry is visually obvious;
- shield-drop lag has an explicit keep/defer verdict from playtesting.

### Checkpoint C — signature move

Required:

- autolink primitive is generic and deterministic;
- Pointed Polygon Up-B uses it for multi-hit carry;
- final hit launches;
- ordinary hitboxes are unchanged;
- recovery-use budget has an explicit keep/defer verdict from actual repeatability.

### Checkpoint D — demo loop

Required:

- KO/respawn is coherent;
- no stale held input deletes protection on the materialization frame;
- one full local match can be played without a state/feedback hole large enough to obscure what happened.

### Checkpoint E — close campaign

- all mandatory tests green;
- visible demo exercised;
- remaining open items are moved to the parity inventory / standing backlog;
- completed execution history is compressed or archived according to `docs/planning/README.md`;
- any new generic mechanic has a focused durable description where a cold reader would look for it.

---

## What should trigger a design pause

The agent should continue autonomously through normal implementation choices. Stop a slice and record a blocker only if one of these becomes true:

- true charge requires a second move authority instead of extending `MovePlayback`;
- autolink can only be implemented by teleporting victims after collision or by abusing capture ownership;
- i-frame visuals require the renderer to reimplement hit eligibility rather than consume one resolved fact;
- shield bubble work starts moving shield resource authority into rendering;
- a proposed fix introduces a player-only simulation road;
- the respawn platform requires a second collision/integration path instead of using the existing world/body machinery.

In those cases, choose another independent campaign row while the blocked design is investigated. Do not expand the week into a workspace-wide architecture migration.

## Likely end-of-week state

The target is not feature parity with Ultimate. The target is a Smash demo whose existing mechanical breadth is finally visible in motion, with one signature character move demonstrating deeper platform-fighter combat vocabulary. If this campaign lands through Checkpoint C, the next product review should decide between more character identity, stage variety, match-loop ceremony, or another mechanics pass from the parity inventory based on play rather than architecture debt.
