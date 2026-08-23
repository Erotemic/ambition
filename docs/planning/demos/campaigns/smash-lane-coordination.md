# Smash feel push — lane coordination

**State:** OPEN. Started 2026-08-22 for a 168h run.
**Execution design:** [`smash-fun-push-2026-08-22.md`](smash-fun-push-2026-08-22.md) owns the slice designs.
**Feature truth:** [`../smash-parity-inventory.md`](../smash-parity-inventory.md).

This document exists because the run is worked by three seats at once. It owns
who may edit what, in which order, and how a slice reaches `main`. It does not
restate a slice design.

## The three seats

| Seat | Worktree | Branch | Owns |
|---|---|---|---|
| MECHANICS | `.worktrees/smash-parity` | `smash-mechanics` | combat/movement/moveset semantics and the facts they publish |
| PRESENTATION | `.worktrees/sidework` | `smash-presentation` | VFX, shaders, particles, SFX routing, everything that consumes a resolved fact |
| COORDINATOR | main checkout | `main` | merges, gates, CPU-vs-CPU behaviour, tuning, this document |

Jon's ordering rule for the whole run: **implement the mechanics that express the
tuning.** A knob nobody can turn is worth more than a value someone guessed.
Presentation scope is IN-MATCH ONLY — no announcer, results screen, or stage-art
campaign. The week goes depth on core verbs, not roster breadth.

## Ownership boundary

The boundary is not a directory list, it is a direction:

> **Simulation publishes a resolved fact. Presentation consumes it.**

- MECHANICS may add a field to a `*View` / read-model publisher when the fact is
  new. It may not decide how that fact looks.
- PRESENTATION may not re-derive a gameplay predicate. If the fact it needs does
  not exist, it says so in its handoff file and works the next independent slice
  rather than inferring the state from move names, animation rows, or velocity.
- Neither lane edits `docs/planning/**` except its own status line here.
- Neither lane merges to `main`. The coordinator merges.

Practical split of the usual files:

| Area | Seat |
|---|---|
| `ambition_combat`, `ambition_platformer2d_core::movement`, `ambition_entity_catalog::MoveSpec`, moveset authoring, `game/ambition_demo_smash/src/moveset.rs` | MECHANICS |
| `ambition_vfx`, `ambition_render/src/fx.rs`, `ambition_render/src/rendering/**`, `ambition_sfx` routing | PRESENTATION |
| `ambition_sim_view` | whichever lane ADDS the fact; the other consumes it after the merge |
| `crates/ambition_characters/src/brain/fighter/**` | COORDINATOR |

## Handoff

A lane hands work over by writing a file into the **main** checkout:

- `ready_to_merge_mechanics.md`
- `ready_to_merge_presentation.md`

Each names the **commit SHA** that is ready, one sentence on what it does, the
gate output, and anything the other lane now depends on. ⛔ The coordinator
merges the SHA in the file, never the branch tip. ⛔ The file's existence is the
signal: delete it once the merge lands.

A lane never polls the other lane and never blocks on a merge. It starts its next
slice on its own branch and merges `main` into itself when it needs something the
other lane landed.

## Build discipline

Both worktrees are prepared and must stay that way:

- each `target/` is bind-mounted to its own store (`scripts/setup_target_bindmount.sh`)
  — re-run after a reboot; a shared target dir surfaces stale rlibs as
  `undefined symbol: anon.*.llvm.*`, which reads like a code error and is not;
- assets are mirrored (`python3 scripts/mirror_assets_for_worktree.py`);
- **cap `cargo -j 4`.** Twelve cores, three seats. Two unthrottled monolith
  builds have put this box at load 25 and filled the disk before.
- ⛔ never `cargo test --workspace --tests`.

## Gates

Per slice, in the lane:

```bash
cargo check -p ambition_app --all-targets
cargo test -p ambition_demo_smash_app        # NOT covered by the app gate
```

Coordinator, after every merge: the two above plus the smash suite's **pass
count** compared to the pre-merge number. A compile failure prints no
`test result` line at all, so a grep for one shows nothing rather than red.

Before a push: `cargo test --workspace --lib`.

## Ordering — first 24 hours

### MECHANICS

1. **M1. Combat action input buffer.** `BodyActionBuffer` is already rollback-
   registered with attack/pogo/projectile slots and nothing writes it. Feed
   semantic press edges into it, decay it deterministically, and spend a slot
   only when the normal action authority accepts the action. This is the single
   largest responsiveness win available: today an attack pressed during endlag is
   simply dropped. ⛔ no per-move grace timers, no buffering of raw device input.
2. **M2. True hold/release smash charge** (campaign O1). Publish the resolved
   charge fraction as a fact; PRESENTATION's charge pulse consumes it.
3. **M3. Move invulnerability and super-armor windows.** `WindowTag::Invuln` and
   `WindowTag::Armor` exist in the authoring schema and the runtime consumes
   neither. Make hit eligibility read the active window. This also gives O4's
   blink a real fact to read on attacks.
4. **M4. Out-of-shield action policy** — one explicit shield-release policy that
   decides which actions may start during/after shield release; then shield grab,
   jump OOS, up-smash OOS through it. Add shield-drop lag as a knob on the same
   policy. ⛔ do not scatter per-move exceptions.
5. **M5. Jab 1→2→3 chains and rapid jab.** Authored successor selection through
   the existing cancel windows; no fighter-ID loop.
6. **M6. DI needs a reaction window.** Measured 2026-08-22: `di_input_local` is
   sampled at the hit-resolution frame — the stick the victim happens to be
   holding when the hitbox connects — and `apply_player_knockback` applies the
   rotated launch immediately. Both Melee and Ultimate read DI at the END of
   hitlag, which is exactly what makes hitlag a decision rather than a pause. As
   shipped, `di_max_angle` is a tuned rule no human can aim: reacting to a hit
   you have not been dealt yet is not an input. Resolve the launch at hitlag
   release, or store the pre-DI launch and rotate it when the freeze ends. SDI
   already reads the stick during hitlag correctly, so the seam exists.

   ⚠ Measured beside it, and worth a look while you are in there: a body with no
   hitstun keeps a written 4,800 px/s launch for exactly ONE tick — air control
   resolves horizontal velocity from the held stick on the next one. Hitstun is
   the only thing that gates that. Check that the hitstun control scale really
   does leave a launch intact for a body holding inward, because if it does not,
   knockback is cancellable by walking.

### PRESENTATION

1. **P1. Hard-launch smoke/speed trail** (campaign O3). Gate on the semantic
   launched/tumble/hitstun fact plus speed — never on world velocity alone.
2. **P2. Invulnerability/intangibility blink** (campaign O4). One resolved
   `unhittable` fact from the same state that controls hit eligibility. Dodge,
   ledge, tech/getup and respawn already grant it; move-invuln arrives with M3.
3. **P3. Distinct tech flash and parry flash/chime** (campaign O5). Split the
   successful-Tech arm from `GetupRoll`; the parry cue fires on successful
   contact, not on the window opening.
4. **P4. Filled shrinking bubble shield** (campaign W1), plus a shield-hit
   ripple. `ShieldRingsView` stays the presentation input; shield resource
   authority does not move into rendering.
5. **P5. Smash-charge pose, accelerating pulse, latch/loaded SFX** (campaign O2).
   Depends on M2's published fact — start it after M2 merges.

### COORDINATOR

Between merges: CPU-vs-CPU quality. Jon named it explicitly — *"I want the feel
of the game and cpu vs cpu to feel and look good"*. A CPU that never charges a
smash, never shields, and never techs makes every mechanic above invisible in the
demo's most-watched mode. Work `crates/ambition_characters/src/brain/fighter/`
options/observations as each mechanic lands, and keep the smallest option surface
that exposes the new verb. ⛔ do not start the AI-policy ownership migration.

Measured at the start of the run: the fighter brain has **no DI, no SDI and no
tech**. `MovementVerb` offers Approach/Retreat/Jump/Dash/Dodge/Shield/Blink/
Recover and nothing a reeling body can use, so every CPU took every launch
straight and never teched a knockdown. C1 closes DI and SDI; C2 is tech.

## Status

| Slice | Seat | State |
|---|---|---|
| M1 input buffer | MECHANICS | ✔ merged `7d99fae57` |
| M2 smash charge | MECHANICS | ✔ merged `2ec892149` |
| M3 invuln/armor windows | MECHANICS | ✔ `36c24ea69` |
| M4 out-of-shield policy | MECHANICS | ▢ |
| M5 jab chains | MECHANICS | ▢ |
| P1 launch trail | PRESENTATION | ✔ `882fe8fa5` — `LaunchedBodiesView` publishes involuntary flight; Dust plume behind the velocity vector, sim-tick cadence |
| P2 i-frame blink | PRESENTATION | ✔ `0c29e9cf0` — `unhittable` on both body read-models is `body_vulnerable` inverted; the hit-flash overlay carries both cues, damage wins |
| P3 tech/parry cues | PRESENTATION | ✔ tech `1f96165eb` + parry `bbf06b133` — parry flash/chime read `parry_flash_secs`, never `parrying()`; guarded at the publication seam too |
| W7 dizzy stars | PRESENTATION | ✔ `f04989c78` — second pooled `GuardBreaksView`; stars orbit the body's own up; the bubble now turns with the body too |
| W7 strong-hit flash | PRESENTATION | ✔ `94686e5ec` — `hit_strength_fraction` inverts the hitlag law in the kernel; fourth arbitrated overlay cue, proportional, no threshold |
| W7 near-KO trail tier | PRESENTATION | ✔ `466dfb028` — plume shifts smoke→ember above the near-KO speed, on P1's existing launched fact |
| §3 ground-bounce | PRESENTATION | ◑ `49ea1d7e5` — landing dust scales with the published impact; the launch-specific splat and the WALL splat need facts (see handoff) |
| Trail speeds re-measured | PRESENTATION | ✔ `486484969` — 330/550/770, stated percentiles of `--runs 5` peak launch; the old 650 never fired |
| Camera cut on elimination | PRESENTATION | ✔ `3db424c86` — inward cast edges capped by SPEED, not just rate; exponential easing jerked in proportion to the collapse |
| Camera cap regression | PRESENTATION | ✔ `a031c9530` — the cap now arms only on a cast POPULATION drop; a general speed limit throttled ordinary approach. Close rate 5→10Hz, which the gating made safe |
| Offscreen capture | PRESENTATION | ✔ `948f8a5fc` — engine `Display::Offscreen`; `match_shots` burst. ⚠ the smash shell had NO renderer and had never been seen; 3 demos still duplicate `DefaultPlugins`/`RenderMode` |
| Default attack swing | PRESENTATION | ✔ `155296974` — the red box was the unauthored-VFX placeholder, not a bug; now a reach-derived tapered sweep. 1 of 145 characters authors `attack_vfx` |
| P4 bubble shield | PRESENTATION | ✔ `e5210712b` — filled field in front of the body, shieldstun flare, near-break danger flicker (part of W7) |
| P5 charge pulse/SFX | PRESENTATION | ✔ `19ec18c42` — authored `smash_charge` row routed ahead of the move's chain, third overlay cue quickens with the fraction, latch/lock cues authored procedurally |
| M6 DI reaction window | MECHANICS | ▢ |
| M8 the fight stopped resolving | MECHANICS | ✔ `3af675f55` — a blanket 0.2s post-hit window was refusing the second Active window of a multi-window move |
| M9 the post-hit gate deletes the tech | MECHANICS | ✔ `3af675f55` — Burst exempt while tumbling |
| M7 successful-parry-contact fact | MECHANICS | ✔ `36c24ea69` — `parry_flash_secs` |
| C1 CPU survival DI/SDI | COORDINATOR | ✔ `0a564b8ef` — live, but its effect on KO count is below this measurement's noise floor |
| C2 CPU tech | COORDINATOR | ✔ — 15–56 techs per 30s match |
| C4 CPU presses into endlag (`BufferableSoon`) | COORDINATOR | ▢ — needs the buffer window as a perceived fact |
| P6 the camera cuts when a fighter leaves play | PRESENTATION | ✔ `694366fa6` — the easing WAS the cause; a speed cap now absorbs any size |
| P7 nobody has SEEN any of this | PRESENTATION | ▢ — an offscreen capture binary for the Smash demo |
| M10 split `evading()` so a ledge grab is not a dodge | MECHANICS | ▢ |
| C5 a guard has no home in the situation vocabulary | COORDINATOR | ▢ — see below |
| C6 the movement scores are a coupled system | COORDINATOR | ▢ — three hand edits, three reverts; see below |
| C3 CPU charges smashes | COORDINATOR | ✔ `dd6c7e79f` |

## Measurements this campaign made, that outlive it

- **An absence check against this repo's sprite assets needs `find -L`.** The art
  trees are symlinked into a worktree, and `find … | xargs grep -l` without `-L`
  searches only the downscaled tiers. A `smash_charge` row reported as authored
  nowhere is authored on eight fighters at every tier.
- **A body with no hitstun keeps a written launch velocity for exactly one tick.**
  Air control resolves horizontal velocity from the held stick on the next one.
  Any fixture that "launches" a body by writing `kin.vel` is racing the movement
  kernel, and two of them were.
- **A grounded body in hitstun still resolves velocity from held locomotion.**
  Holding a stick against a slide erases it, which is why the CPU's survival
  influence is airborne-only. Whether the kernel should scale that authority
  during hitstun is M6's neighbour and is still open.
- **`cargo check -p ambition_app --all-targets` does not reach another package's
  test targets.** A field added to a rollback-registered type broke
  `ambition_platformer2d_rollback_ggrs`'s lib test and nothing in the per-slice
  gate saw it. `cargo test --workspace --lib` is the tier that does.

## The instrument

```bash
cargo run -p ambition_demo_smash_app --bin match_report -- [SECONDS] [CHARACTER]
```

Counts what two CPUs actually do to each other: damage, moves, hitstun, tumbles,
knockdowns, evades, unhittable ticks, shields, parries, techs, the best charge
reached, and the hardest launch handed out against the body's own tumble
threshold. Run it after any change that claims to affect how a fight goes.

⭐ **It exists because three slices in two days shipped green and inert.** The
smash charge, directional influence and the tech each passed their unit tests
while nothing happened in a match, and each was caught by counting rather than by
asserting. ⛔ a unit test that drives a kernel with a synthetic input proves the
FUNCTION and says nothing about the WIRING — the tech's kernel tests are green
today and no body in the game can tech.

## What day one actually cost, and the rule it bought

Five mechanics shipped green and inert before anybody noticed: the smash charge,
directional influence, the tech, the launch trail, and — the one that explains
the rest — **a fighter's authored strong throw, which had never once landed.** A
blanket 0.2s post-hit invulnerability window outlived the 0.12s gap between two
Active windows of the same move, so the second was refused by the i-frames the
first armed. Eight ignored strikes in thirty seconds, and they were the eight
biggest. Every launch number anyone had measured was the weak pop.

⭐ **THE RULE: a number measured once is a coin flip, and a number measured by a
proxy is a different number.** Both were paid for on the same day —

- one 30s sample judged an option-scorer change; the suite went from two
  failures to four. `match_report --runs N` prints `min–median–max` now.
- `damage` read the LAST percent, and a KO resets a body to zero, so a run in
  which both fighters died reported as a run in which nothing happened.
- a test read distance travelled as a stand-in for top speed, and an opponent
  standing two pixels away was shoving the body.
- two guards raced the fight's PACE to reach their own question instead of
  seating one stock and asking it.

## What the CPU's directional influence is actually worth

Measured 2026-08-23, seven execution-noise streams of a 90s match, the reflex
switched off behind an env var and back on:

```
                 damage            unhittable            KOs
DI on      142–342–399        1258–1827–2505        2–3–4
DI off     268–311–471        1085–1455–2043        2–3–4
```

⇒ **the KO count is identical.** DI is live — the demo declares
`SMASH_DI_MAX_ANGLE = 0.31` rad (~18°, the genre's own figure), the stick is
held, and the rotation happens — but at this stage's blast margins its effect on
who dies is below the noise floor of seven samples.

⭐ So the honest claim for the slice is the one about BEHAVIOUR, not outcomes: a
reeling body used to hold whatever it had been walking toward, and now it holds
something chosen — which is visible in flight and is what a spectator reads as a
fighter reacting. ⛔ Do not write down that CPU DI improves survival. It has not
been shown, and the same seven-run rig is how anyone would show it.

⭐ **and the same rig should be pointed at every behaviour slice before it is
believed.** The differential — the reflex off, the reflex on, everything else
fixed — is the only thing that separates "the mechanic runs" from "the mechanic
matters", and four of this campaign's slices ran for days without mattering.

## P7 — nobody has seen any of it

Twelve visual features have shipped this week — launch trails, the i-frame blink,
the bubble shield and its hit flare, the low-shield read, the charge pose and
pulse, latch and loaded cues, dizzy stars, the tech spark, the parry flash and
chime, the strong-hit flash, the near-KO ember, landing dust — and **not one of
them has been looked at.** Every claim about them is a unit test over a published
fact. Four mechanics this week passed exactly that bar while doing nothing at
all.

Mary-O has a capture binary and Smash does not:

```bash
cargo run -p ambition_demo_mary_o_app --features capture --bin capture_mary_o \
    -- OUT.png 960x540 --warmup 120
```

`ambition_render::capture` is the game-agnostic half (target, camera adoption,
readback, PNG, exit); what each game supplies is which app to build and when its
world is worth photographing. The Smash version wants two CPU seats, a wait past
the 3-2-1-GO, and — unlike Mary-O's — a **burst**: several frames spaced N ticks
apart, because a pulse rate, a blink cadence and a trail are not judgeable from
one still. `request_capture` is single-shot, so the burst is the game side
resetting `CaptureProgress` and bumping the output path.

⛔ Three ways it writes a file having rendered nothing, all hit in one sitting on
another demo: `RenderMode::Headless` sets `backends: None` so there is no
RenderApp; disabling `winit` also removes the RUNNER, so `app.run()` performs one
update and returns; and cameras do not exist at `Startup` in a shell-composed
demo, so adoption must run every frame and the shot must wait for
`CaptureTarget::adopted > 0`. ⭐ On a machine with no display, print the pixel
histogram — a transparent PNG and a white scene preview identically.

## C5 — a fighter has nowhere to decide "guard, they are swinging"

The shield is offered in exactly one situation, `Disadvantage`, and gated on a
hostile being somewhere in a swing. So a fighter standing in the open watches an
attack wind up and walks into it.

⛔ **The obvious fix does not fire, and the reason is the vocabulary rather than
the score.** I added the guard to the `Neutral` arm, gated on a hostile in
`AttackStartup` with under 0.12s of startup left, and measured it with the reflex
switched off and on across five 90s streams: **byte-identical results, parries
0–2–5 either way.** A fighter is never IN `Neutral` while somebody winds up —
`classify` returns `Advantage` the moment a foe becomes punishable, and
`is_punishable` covers attack startup.

⇒ `Advantage` today means "they are committed, punish them", and it silently also
means "they are about to hit you". Those are the same fact read from two ends,
and the thing that separates them is whether their swing reaches you before
yours reaches them — which `Features::frame_advantage` already computes for the
ATTACK options and no movement option can see.

⭐ So the slice is not a score change. It is either a fifth situation, or giving
the movement options the frame-advantage read the attack options already have.
⛔ Do not attempt it by re-pricing `Shield`: measured 2026-08-23, re-pricing the
evade by hand took the smash suite from two failures to four, and this brain has
an evaluation rig (`brain::fighter::evaluation`, `scenarios`) for exactly this.

## C6 — the movement scores are a coupled system, and hand-editing them fails

Three attempts, three reverts, and the third is the one that explains the other
two. ⭐ **The measurement that made them legible costs nothing:**

```bash
AMBITION_FIGHTER_TRACE=1 cargo run -p ambition_demo_smash_app --bin match_report -- 30 2>trace
grep -o 'chose=Some([A-Za-z]*)' trace | sort | uniq -c | sort -rn
```

Thirty seconds of CPU-versus-CPU, as shipped:

```
341 Approach   142 Dodge   136 Recover   75 Shield   69 Retreat   17 Jump
```

**Dodge is the second most common decision in the game**, and every sampled one
was `Disadvantage` with `offered=[Dodge, Retreat, Jump]` — cornered, nothing
incoming, rolling because 0.75 outranks Retreat's 0.7.

⛔ **Paying that premium only against a live swing looks obviously right and is
not.** Dodge fell 142 → 58 and Retreat rose 69 → 216, exactly as intended. Then:
`Recover` fell 136 → 60, **nobody was ever knocked off the stage**, and **no CPU
held a smash in any of three matches.** A cornered fighter that retreats instead
of rolling STAYS cornered — and `charge_ticks_for` pays a full charge only in
`Advantage`/`EdgeGuard` and none at all in `Disadvantage`. The roll was
load-bearing for the OFFENSE, two steps away, through the situation classifier.

⇒ the two earlier reverts were the same lesson wearing different clothes:
re-pricing the evade across three arms took the suite from two failures to four,
and a guard added to `Neutral` never fired at all.

⭐ **The rule: a movement score is not a local number.** It changes which
SITUATION the body ends the next second in, and every other score is read in that
situation. ⛔ Do not hand-edit these; the brain has `brain::fighter::evaluation`
and `scenarios` for exactly this, and its own doc says survival and damage need a
match harness — which `bin/match_report` now is. Joining those two is the slice.

## The one rule three lanes learned separately in two days

> **A constant fitted to observed behaviour is a claim about that behaviour, and
> it stops being true when the behaviour changes.**

Three instances, one per seat, none of them a bug at the site that failed:

- `TRAIL_ONSET_SPEED = 650` was fitted when launches were being thrown away, so
  the launch trail could never fire. The real peak was 411.
- `damage` in `match_report` read the LAST percent, so a run in which both
  fighters died reported as a run in which nothing happened.
- the camera's exponential chase sits `closing speed / rate` behind by
  construction. It had always been 36 units back; nobody could see it until the
  fight started launching bodies vertically.

⇒ **write the sample size and the percentile into the source beside any constant
fitted to data**, and re-measure it whenever the thing it describes changes. The
presentation lane does this now; it is worth copying.
