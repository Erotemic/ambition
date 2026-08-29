# Smash feel push — lane coordination

**State:** OPEN. Started 2026-08-22 for a 168h run.
**Execution design:** [`smash-fun-push-2026-08-22.md`](smash-fun-push-2026-08-22.md) owns the slice designs.
**Feature truth:** [`../smash-parity-inventory.md`](../smash-parity-inventory.md).

This document exists because the run is worked by three seats at once. It owns
who may edit what, in which order, and how a slice reaches `main`. It does not
restate a slice design.

## The three seats

⛔⛔ **RETIRED 2026-08-23 BY MAINTAINER DIRECTION.** Jon: *"we are not going to
use subagents anymore. We are working strictly on main now."* The worktrees and
branches below no longer exist as a working arrangement — do NOT create them, and
do not treat a row's SEAT column as a routing instruction. What survives is the
OWNERSHIP BOUNDARY (which code answers to which concern) and the status table,
both of which are still the best map of this campaign that exists.

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

- each `target/` is bind-mounted to its own store (`scripts/setup/target_bindmount.sh`)
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
| M4 out-of-shield policy | MECHANICS | ✔ shipped — `ShieldTuning::out_of_shield` + `OutOfShield`/`OutOfShieldAction`; the smash `MatchBody` declares `ShieldTuning::PLATFORM_FIGHTER`, which carries `OutOfShield::PLATFORM_FIGHTER`. ⭐ ONE implementation as of `4a70be7e0`: `OutOfShieldGate` lives in the movement kernel and both the kernel and the moveset trigger read it; combat keeps only `rises_out_of_shield`, the direction half |
| M5 jab chains | MECHANICS | ✔ shipped — `jab_string_continuations()` authored once and pushed into BOTH tables (the shared fighter kit and George's own), because the chain first shipped onto the shared table alone and the headline fighter had no `jab2` at all |
| P1 launch trail | PRESENTATION | ✔ `882fe8fa5` — `LaunchedBodiesView` publishes involuntary flight; Dust plume behind the velocity vector, sim-tick cadence |
| P2 i-frame blink | PRESENTATION | ✔ `0c29e9cf0` — `unhittable` on both body read-models is `body_vulnerable` inverted; the hit-flash overlay carries both cues, damage wins |
| P3 tech/parry cues | PRESENTATION | ✔ tech `1f96165eb` + parry `bbf06b133` — parry flash/chime read `parry_flash_secs`, never `parrying()`; guarded at the publication seam too |
| W7 dizzy stars | PRESENTATION | ✔ `f04989c78` — second pooled `GuardBreaksView`; stars orbit the body's own up; the bubble now turns with the body too |
| W7 strong-hit flash | PRESENTATION | ✔ `94686e5ec` — `hit_strength_fraction` inverts the hitlag law in the kernel; fourth arbitrated overlay cue, proportional, no threshold |
| W7 near-KO trail tier | PRESENTATION | ✔ `466dfb028` — plume shifts smoke→ember above the near-KO speed, on P1's existing launched fact |
| §3 ground-bounce | PRESENTATION | ✔ `49ea1d7e5` then `3e39edd02` — a CRASH (`Landed { involuntary }`) kicks brighter dust plus a ring and fires at any speed; the WALL splat reads `Contact::impact_speed` under `ContactKind::Side`. ⭐ the wall gets its OWN band (150–440) because gravity never accelerates a body into a wall: the hardest side arrival measured is 440 against a floor onset of 520, so sharing the floor's numbers would have shipped it inert |
| Re-fit after D190 | PRESENTATION | ✔ `9cedaf5db` — flight shifted DOWN not up (p50 289→213, p99 1500→1183): constant engagement means more SMALL exchanges. ⭐ both flight and landings are BIMODAL with an empty 1200–1499 band and a cluster above it whose six values are spaced by one tick of gravity — free fall. Near-KO 1500→1350 and splat full 1040→1330 now sit in the GAP, not at a percentile. Trail onset/full re-fitted 290/760→210/710. Wall band UNCHANGED on n=85. `HARD_LAND_SPEED` deliberately not re-fitted: it is an ENGINE constant the landing pose reads for every game |
| Hitlag scaled by hit strength | PRESENTATION | ⛔ NOT A SLICE — already implemented and LIVE. `hitlag_time: 0.070` is the REFERENCE, not a flat value: `hitlag_duration = hitlag_time * reaction_scale(knockback).clamp(0.5, 4.0)`. Measured 3×90s, 157 authored freezes: min 0.035s (2.1f) / p50 0.165s (9.9f) / max 0.280s (16.8f) — an 8× range, both ends reached. The one real genre divergence is that ours reads KNOCKBACK where Smash reads DAMAGE, so our freeze grows with the victim's percent and the genre's does not; that is inside `ambition_combat` and is the coordinator's to route |
| The three 'how hard' consumers | PRESENTATION | ✔ `186beb672` — they read TWO quantities and that is correct: the HIT's weight (hitlag, strong-hit flash, camera shake, all off `reaction_scale`) and the BODY's flight (launch trail). The KO beat read neither — a boolean — and now joins the flight family on the trail's own band. Live: KOs leave play at 577–1380 px/s |
| KO beat | PRESENTATION | ✔ `467690e7f` — ring + hot spark burst at the blast line, `WORLD_EXPLOSION`, elimination reads bigger off `FighterStockSpent::eliminated`. ⛔ no Star/Screen KO: the genre picks randomly and the games disagree on frequency. `KnockoutsView` captures the POSITION sim-side (reads messages before refreshing its last-seen record) because `place_respawning_fighters` teleports the body the same tick. Proven live: 4 KOs, 7 differing frames, max delta 212 |
| Camera lurches on a stock loss | PRESENTATION | ◑ `ef309d7b1` — the three biggest camera steps of a whole match were the three stock losses (143.7/86.3/82.1 vs p99 13.1); the elimination was already smooth. The cap armed only on a population DROP and a respawn is a member that neither left nor joined. Now arms on `travelled_under_own_power` too → 120.4/52.9/51.4. ⛔ the residual is NOT a camera defect — see handoff, the demo has no respawn interval |
| Camera re-looked against a real surround | PRESENTATION | ✔ `0ad2ac803` — the framing was NOT judged against a lie; the HUD was. Both rects are 16:9 full-bleed, and camera framing depends on the viewport's ASPECT, never its size: normalized framing is identical to three decimals across 5,296 samples in both conditions. What the default rect broke is absolute pixels — the HUD landed off-image, and every capture was CROPPED TO THE CENTRAL 80%. Framing itself is clean: 0/5296 outside frame, max 0.957 of half-width |
| Offscreen capture had no HUD | PRESENTATION | ✔ `0401e41ed` — `match_shots` never declared `HeadlessDisplaySurface`, so the layout resolver found no window, returned early, and every HUD slot laid out against a default rect. The resource existed for exactly this and `capture_scene` always used it. Shots now carry portraits, stocks and the reserved surround. ⚠ still no TEXT in an offscreen shot — not the percent, not the countdown — so the open suspect is font loading, not the HUD |
| Trail speeds re-measured | PRESENTATION | ✔ `486484969` then `6d5681d1a` — now 290/760/1500, the p50/p90/p99 of the speed a launched body actually FLIES at (n=9878 flight ticks). The old set were percentiles of `peak launch`, the speed at the launch TICK, which is not what the gate reads; near-KO had drifted to p90 of flight and burned one launched tick in ten |
| §3 launch beat distinct from sustained tumble | PRESENTATION | ✔ `8f1b3c47f` — `LaunchedBodyFact::launch_beat_secs` publishes the sim's own `recoil_lock_timer`; a white-hot spark flare rides the front of a launch over the grey plume. 27–45–63 launches and 317–535–711 beat ticks per 90s match, and the flare was confirmed on CAPTURED PIXELS by diffing the same match with it off |
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
| P7 nobody has SEEN any of this | PRESENTATION | ✔ `948f8a5fc` + `0401e41ed` — `bin/match_shots` on engine `Display::Offscreen`, and the shots carry the HUD once `HeadlessDisplaySurface` is declared. ⚠ still no TEXT in an offscreen shot; the open suspect is font loading |
| M10 split `evading()` so a ledge grab is not a dodge | MECHANICS | ✔ shipped — `BodyMotionFacts::ledge_intangible` is a SIBLING of `dodge_rolling`, not a refinement; `evading()` takes both, so everything asking "is this body untouchable" is unchanged while anything that needs to tell an edge from an evade now can |
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
  ⛔⛔ **AND IT HAPPENED AGAIN, BIGGER.** Found 2026-08-24: the actor monolith's
  own test target had stopped compiling — an argument added to
  `apply_body_hit_reaction`, and a leaf-membership row written as a 3-tuple — so
  **1,157 tests were dark**, and the one that then failed had drifted three ways
  behind the production chain. ⇒ **SWEPT the whole tier the same day and it is
  now clean: 67 lib test targets, every one compiling, every one green.** Run
  that tier after touching a shared type; the app gate is not a proxy for it.

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

### ⛔ THE FRAME-ADVANTAGE OPTION IS ELIMINATED — measured 2026-08-24, and reverted

Built it: `kit_fastest_startup` threaded into `movement_options` the way
`kit_lifts` already is, and `Shield` offered in the `Advantage`/`EdgeGuard` arm
at 0.85 whenever a hostile is in `AttackStartup` with less startup LEFT than this
body's fastest move needs. Compiles, 620 green, and **INERT**: over a 30s trace,
`Shield` was offered in `Advantage` **zero times** out of 129 decisions, and zero
in `EdgeGuard` out of 34. The whole offered vocabulary there is
`[Approach, Dodge, Jump]` (96), `[Approach, Jump]` (28), `[Approach]` (5).

⭐ **and the reason is that the read is RIGHT and the answer is "punish".** With
a jab this fast, a fighter can nearly always start before a startup finishes —
so "their swing is faster than mine" is almost never true, and it is not what
makes approaching wrong.

⇒ **what actually makes it wrong is ARRIVAL, not speed.** Approaching closes
distance while their hitbox is winding up, so the body gets there exactly as it
goes live. The question a movement option would have to ask is *"will I be inside
their reach when their window opens"* — and nothing published tells a brain what
an opponent's swing REACHES. `Features::reach_fit` reads the body's OWN coverage.
⇒ so the remaining slice is a new perceived fact (the foe's committed swing and
its reach), which is mechanics work, not a score change — and the fifth-situation
option cannot be judged until that fact exists either.

⛔ the branch was NOT shipped. This campaign has already paid five times for
green-and-inert code, and an unfired branch with a good comment is the same
thing. The measurement is the deliverable.

⚠ the decision mix has moved a long way from the one recorded under C6 below —
30s, same command: `431 Approach · 250 Dodge · 81 Recover · 29 Retreat · 7
Shield · 6 Jump`, against C6's `341 · 142 · 136 · 69 · 75 · 17`. Dodge is now
FIRST among defensive answers by a wide margin and Shield has nearly vanished, so
C6's own numbers should be re-taken before anything is concluded from them.

## C7 — THE PARRY IS UNREACHABLE FOR A CPU, and neither timing is the fix

Measured 2026-08-24. `match_report -- 30` prints the parry window's own share of
unhittable ticks:

```text
seat   invuln   evading   of-ledge   parry-window   i-frames
0         120       368         30             19          0
1         202       299         58             14          0
```

**33 window ticks across both seats in 30 seconds**, against ~128 ticks of
shielding, and five 30s runs produce `parries 0–0–1`. The genre's most rewarding
defensive option effectively does not happen.

⭐ **the diagnosis is right and the obvious fix is wrong.** A parry is a TIMING:
`ParryTiming::OnRaise` opens the window on the raise and it lasts
`PARRY_WINDOW_TIME` (0.15s, nine frames — generous; Ultimate's is about five).
The brain guards on `threatened` = *any hostile is attacking*, which is true from
the first frame of a wind-up, so the window is spent on the TELL and the body
holds a plain shield through the hit.

⛔ **BUILT AND REVERTED.** `SelfView::parry_window_s` published from the body's
own `AxisSweptParams`, and `threatened` narrowed to *a live hitbox, OR a wind-up
whose remaining startup fits inside my own window*. It fails on its own success
metric and costs a lot, five 30s runs against the same baseline:

```text
              baseline          raise-timed
parries     0–0–1             0–0–2        ← the point, and it did not move
shielding   35–123–200        30–73–85     ← 40% less guarding
unhittable  978–1006–1102     733–869–954
hitstun     614–748–1027      935–1024–1183
damage      231–270–393       252–301–393
KOs         2–3–4             1–2–4        ← a KO of median, gone
```

⇒ so it buys nothing and sells the shield. Reverted, `parry_window_s` with it —
an unused published fact is the same green-and-inert failure as an unfired branch.

⇒ **what this eliminates and what it leaves.** Raise timing is not the lever.
The remaining candidates, in order of what they'd cost:
  - ⛔ `ParryTiming::OnRelease` — TRIED AND ELIMINATED, same day. One word in
    `SMASH_MATCH_BODY`, five 30s runs: `parries 0–0–2`, against the baseline's
    `0–0–1`. Everything else lands inside noise (damage 232–270–324 vs
    231–270–393, KOs 2–3–4 either way, shielding median identical at 123). The
    mistiming simply moved to the other end, exactly as suspected: the CPU
    releases its guard once the threat has PASSED. Reverted — the shipped stage
    keeps the press-timed window, and which generation's rule it should play is
    Jon's feel call, not something to change for no measured gain.
  - ⇒ **a deliberate parry ATTEMPT as its own option** is what is left, and
    working out its shape is what resolves this row.

⭐⭐ **AND THE MECHANIC ITSELF SAYS WHY A CPU DECLINES IT.** The window opens on
a guard MOVEMENT and a body already holding shield cannot open a second one —
which is the genre's rule too: in Smash you parry by NOT having shielded early.
So parrying and blocking-early are MUTUALLY EXCLUSIVE, and the raise-timed
experiment above is exactly that trade, priced:

```text
             blocks early (shipped)   waits for the window
parries      0–0–1                    0–0–2
shielding    35–123–200               30–73–85
damage       231–270–393              252–301–393
KOs          2–3–4                    1–2–4
```

⇒ **the trade is real and it is bad for this CPU.** A body that cannot read its
opponent buys a near-certain block by guarding early, and pays a KO of median for
a parry it still mostly misses. That is not a mechanism gap — the parry works,
and a HUMAN who reads the wind-up gets it.

⇒ so the honest shape of what is left is a RISK APPETITE, not a rule: a CPU
personality that takes the gamble (`brain::fighter::profile`), where the cost
above is the point rather than a regression. ⛔ do NOT ship it as the default
duelist's behaviour — the numbers above are what that costs.
⛔ do not widen `PARRY_WINDOW_TIME` to manufacture the number. Nine frames is
already twice the genre's, and a window that catches by being wide is not the
mechanic.

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

## ⚠ The caveat on every CPU number in this document

Every measurement here is **George against George, or the demo's stand-in
duelists**. Both instruments seat through `smash_roster_at_levels`, and for at
least one ordinary roster fighter — `npc_pirate_admiral`, which `app_it`'s duel
guard seats successfully at rung 9 through a different path — **no fighter brain
ever takes the noise seed**: `ladder_rig` aborts on its own guard and
`match_report` prints empty rows.

⇒ **CORRECTED**: it is not two seating paths disagreeing. Both use the same
roster helper; what differs is the APP. `app_it` builds the FULL app and the rigs
build the demo shell, whose catalog carries `smash_george_booul` and two stand-in
duelists and nothing else. It is D189, and it is the reason D188's regression —
found on a character the demo shell does not carry — could not be reproduced in
either rig.

⭐ So read every number in this file as *"true of this matchup"* until D189 is
closed. The defects found through them are real; their generality is not
established.

## The sharpest form of the rule, and the lane that found it

> **A constant fitted to a PROXY for the quantity its gate reads is wrong the day
> it is written, and only looks right until the proxy moves.**

The launch trail's three speeds were percentiles of `peak launch` — the speed at
the tick a launch is *written*. The trail's gate reads the speed a body is
*flying* at, which is a different distribution: over 9,878 flight ticks it is
`p25 49 · p50 289 · p75 563 · p90 756 · p99 1500 · max 1902`. The shipped near-KO
threshold of 770 was therefore **p90** — one launched tick in ten burning as an
ember, under a comment reserving it for the top of the fight.

⇒ **fit a constant against the distribution the CONSUMER samples**, not against
the nearest number that was already being printed. And write the percentile and
the sample size beside it, which that lane now does as a matter of course.

## Two more forms of the same rule, both found by lanes

**Identical numbers mean "the watched matchup cannot reach the content" at least
as often as they mean "the change did nothing."** The mechanics lane authored its
first sweetspot example on the SHARED roster, measured five streams byte-identical
to pre-E1, and checked which matchup produced the number instead of concluding
the rule was inert — George carries his own moveset and never throws the shared
forward smash.

**Two genuinely distinct populations set a threshold in the GAP between them, not
at a percentile of one.** Gravity accelerates a body into a floor and never into
a wall, so landings (n=340, p50 299, max 1669) and side contacts (n=63, 54 of
them exactly 52 px/s from leaning on the lip, hardest real arrival 440) cannot
overlap. A wall splat sharing the floor's 520 onset would have shipped green and
never once fired.

⭐ And the one that should change how every capture in this campaign is read:
**`match_shots` never declared `HeadlessDisplaySurface`**, so the HUD resolver
found no primary window, returned early, and laid every slot against a default
rect. Every shot taken before that fix was showing a different layout —
convincingly. ⛔ A capture that is wrong in a way that looks right is worse than
no capture; the camera-framing work in particular was judged against a stage that
was not letterboxed into a real gameplay rect.

⭐⭐ **A failure that is BYTE-IDENTICAL across every commit you bisect is the
signature of a cause your bisect cannot vary — not evidence that it never
passed.** The mechanics lane saw two `app_it` tests fail in its worktree, walked
them back 35 first-parent commits, found the same bytes at every one, and
correctly reported them as not-its-own. Both pass in the MAIN tree at the same
code. The asset trees are gitignored, so a worktree does not carry them: they are
absolute symlinks into main's live assets, which change under a lane every time
the coordinator commits. So the lane was running its OWN CODE against MAIN'S
CURRENT ASSETS, and a bisect over commits could only ever move the code half. The
two tests it chose — a census of what the game can SHOW and a walk toward
geometry that comes out of content — read exactly the half that was pinned.

⇒ **the rule for every lane in a worktree: a test that reads assets or content is
measuring main's assets against your code.** Pure-Rust measurement (unit suites,
poison runs) is unaffected. Anything else goes to the coordinator to run on main.
And the general form is worth more than the instance: when a bisect finds no
transition anywhere in history, ask what the bisect was holding constant before
concluding the subject was always broken.

⭐⭐⭐ **TWO CONSTANTS WITH THE SAME VALUE MAKE A MEASUREMENT UNABLE TO ATTRIBUTE
ITSELF — and the move is to change ONE of them.** The grid sweep found six
characters whose median gap sat between 491 and 515 px with *nothing at all
between 295 and 491*: a hole, not a tail, and plainly a threshold. But
`DEFAULT_VIEWPORT_HALF.x`, past which a brain cannot see a foe, and
`PLATFORM_WIDTH`, past which there is no floor, are **both 480**. "Blind past
480" and "standing in opposite corners of a 480-wide platform" predict the
identical gap, and no amount of extra precision in the same measurement could
separate them.

Viewport to 2000, platform untouched: all six collapsed to 18–278 and every
silent fighter started fighting, while every fighter already inside 480 came out
byte-identical. ⇒ this is the same family as *"a constant fitted to a PROXY for
the quantity its gate reads"* and *"identical numbers are a finding, not a
pass"*, and the general form is worth more than any of the three: **interrogate
what the instrument was holding fixed.**

⛔ And the consequence for this campaign's own history: every CPU-versus-CPU
number taken before `d4c681a8b` was measured on a stage where some fighters could
not see each other. George-vs-George sat at gap 28 and is unaffected, so the
census work stands — but D188's regression evidence (`npc_pirate_admiral` falling
169% → 49% under the `frame_advantage` flip) was taken on a fighter at gap 502,
i.e. one that was blind for most of the match. **That number has to be re-taken
before it is allowed to block the fix again.**
