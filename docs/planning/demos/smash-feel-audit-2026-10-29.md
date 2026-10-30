# Smash feel audit: why it does not play like Ultimate, and the road there

Audited 2026-10-29 on `main` (2cda058a4). The feature inventory
([`smash-parity-inventory.md`](smash-parity-inventory.md)) says most Smash
mechanics are wired. This audit asks a different question: **why does a human
with a controller not feel Ultimate?** It measures the running game and the
authored data, and it names the gap to Ultimate for each finding.

Every claim is labelled:

- **MEASURED** — read off the running simulation (`fighter_motion`, today).
- **AUTHORED** — read off the data files with a script (frames at 60 Hz).
- **CODE-READ** — the rule as the source states it; no headless witness yet.
- **ULTIMATE** — Ultimate Frame Data / SmashWiki. Movement numbers come from
  [`../../related-work/smash-ultimate-fighter-reference.md`](../../related-work/smash-ultimate-fighter-reference.md);
  per-move frames are from memory of UFD's Mario page and are marked `(verify)`.
  `scripts/fetch_smash_ultimate_moves.py` can be extended to fetch frames.

## The short answer

Base movement is already in Ultimate's band (MEASURED, below). The feel gap is
in five places, ranked by how much each one changes a human's hands:

| # | Finding | Kind | Ultimate | Ambition |
|---|---|---|---|---|
| 1 | A hit victim can **attack 7 frames after being hit**, inside hitstun, and keeps 18% stick control | CODE-READ | no action of any kind in hitstun; only DI/SDI | `knockback_recoil_lock_time` 0.12 s, then the attack verb is preserved; `hitstun_control_scale` 0.18 |
| 2 | **Hitlag is a third of Ultimate's** | CODE-READ + arithmetic | `0.65 × damage + 6` frames: 8 f for a 3% jab, 13 f at 11%, 16 f at 15% | `hitlag_time` 0.070 s × scale (0.5–1.5): **2–6 frames** |
| 3 | **Aerials are half as long as Ultimate's, with short active windows, and most fighters author no landing lag** | AUTHORED | Mario nair 42 f total, active 3–28, 6 f landing lag; fair 56 f / 16 f lag | robot nair 22 f total, active 4–12, 6 f lag; admiral/goblin aerials 17–29 f with **no landing lag** |
| 4 | **Tilts are nearly jabs**; up/down smash start late | AUTHORED | Mario jab 15 f, ftilt 29 f, utilt 23 f, dtilt 25 f; usmash hits frame 9, dsmash 5 | robot jab 15 f, ftilt 19 f, utilt 20 f, dtilt 17 f; usmash hits 16, dsmash 13 |
| 5 | **No walk, no crouch**: any push past half stick is a full-speed initial dash; the match grants and permits no crouch | CODE-READ | tilt = walk, flick = dash; crouch everywhere; crouch cancel; crawl | `STEER_DEADZONE` 0.5 arms a dash on any edge; `abilities.crouch: false` in both floor and ceiling |

Lesser findings: 7 frames of coyote time (Ultimate 0), parry on the raise
(Smash 4; Ultimate on release), no tap jump, no short-hop button, no
walk/run split for a D-pad. The jump squat measured 2 frames (authored 3).

If only one thing is done: **#1 then #2.** Together they are why hits "do not
matter": the hit does not freeze, and the victim swings back before the attacker
can move. Combos, trades, frame advantage and the "crunch" of a connect all
follow from those two numbers.

The other half is presentation (P1–P7, below): one spark and one thud for
every hit, a flat white blink on the victim, one held pose per attack phase,
no dust anywhere. Those are what a longer hitlag would make visible.

## Evidence

### Movement is in band (MEASURED)

`cargo run -p ambition_app_tools --bin fighter_motion -- player_robot_v3 npc_pirate_admiral npc_alice goblin`
on the 480 px stage (k = 3 px/unit), 2026-10-29. Robot v3 against Mario and the
roster median:

| stat | robot | Mario | median |
|---|---|---|---|
| initial dash / run (u/f) | 1.917 / 1.750 | 1.936 / 1.760 | 1.914 / 1.720 |
| traction (u/f²) | 0.097 | 0.102 | 0.110 |
| full hop (u / frames) | 35.0 / 54 | 36.3 / 56 | 33.0 / 50 |
| short hop (u / frames) | 16.4 / 35 | 17.5 / 40 | 16.3 / 37 |
| gravity, fall, fast fall (u/f², u/f, u/f) | 0.107, 1.72, 2.78 | 0.087, 1.50, 2.40 | 0.092, 1.60, 2.53 |
| air speed / accel | 1.194 / 0.080 | 1.208 / 0.080 | 1.050 / 0.075 |
| fwd roll (u / total f) | 32.0 / 30 | 32.2 / 29 | 31.8 / 29 |
| spot dodge total | 23 | 25 | 25 |
| neutral air dodge total | 51 | 52 | 49 |

Deviations that are not the base body: the robot's gravity is 1.22× Mario's and
fall 1.15× (inside p90, but it reads heavier than Mario); the admiral falls at
1.21× the median; alice's air acceleration is 1.37× the median. These are the
per-fighter shapes from `scripts/smash_fighter_shape.py`, a first pass awaiting
Jon. The jump squat is 2 frames measured against 3 authored; check
`jump_squat_time: 0.05` against the tick rounding.

Conclusion: **do not retune `base_body`.** The movement feel gap is not here.

### 1. Attacking inside hitstun (CODE-READ)

`crates/ambition_combat/src/attack_support.rs:171` — after the hard lock
(`hard_lock_timer`, whose recoil cause is `knockback_recoil_lock_time` =
0.12 s, `feel.rs:209`), a body in hitstun keeps `attack_pressed` and
`pogo_pressed` and keeps `hitstun_control_scale` (0.18, `feel.rs:206`) of its
stick. The comment names the intent: "the Hollow-Knight 'get bopped out, then
fight back while flashing' feel". No Smash rule overrides it
(`crates/ambition_match/src/match_rules.rs` patches `slash_recoil` and the
movement body only; `game/ambition_demo_smash/assets/data/*.ron` names neither
field). Smash's `hit_repeat_window_scale: 0.0` does turn the post-hit i-frames
off, so the victim is hittable; it can also hit back.

In Ultimate a victim in hitstun has no actions: DI during hitlag, SDI, and
nothing else until hitstun ends (then air dodge/attack out of tumble). That is
what makes a hit an advantage state.

Not yet witnessed headlessly. The first roadmap step is the witness.

### 2. Hitlag (CODE-READ)

`hit_response.rs:544`: `hitlag = hitlag_time × clamp(0.46 + 0.54 × damage/11, 0.5, 1.5)`,
`hitlag_time` = 0.070 s (`feel.rs:189`). At 60 Hz:

| damage | Ambition | Ultimate (`0.65d + 6`, ×1 for a normal hit) |
|---|---|---|
| 3 (jab) | 2.5 f | 8 f |
| 6 (tilt) | 3.2 f | 10 f |
| 11 | 4.2 f | 13 f |
| 15 (smash) | 5.1 f | 16 f |
| 20+ | 6.3 f (cap) | 19 f+ (no cap; electric ×1.5, charged smash more) |

The shape is right (affine in damage, same for both bodies, decoupled from
knockback); the reference is a third of Ultimate's and the cap removes the top.
Ultimate also freezes the *attacker* for the same count, so a multi-hit reads as
beats. The presentation road exists (freeze in `pose_view`, `hit_flash`, camera
shake keyed to the same quantity), so the fix is the number, not a system.

### 3 and 4. Frame data (AUTHORED)

Startup/active/recovery windows from each moveset, converted at 60 Hz
(`scripts/smash_frame_data.py`). Ultimate column: Mario, UFD, from
memory `(verify)`. "total" = first actionable frame; "LL" = landing lag.

| move | robot v3 | admiral | goblin | Mario (verify) |
|---|---|---|---|---|
| jab | hit 3, total 15 | 4 / 17 | 2 / 13 | hit 2, total 15 |
| ftilt | 4 / 19 | 6 / 23 | 4 / 16 | 5 / 29 |
| utilt | 4 / 20 | 5 / 22 | 4 / 18 | 5 / 23 |
| dtilt | 4 / 17 | 5 / 20 | 4 / 16 | 5 / 25 |
| dash attack | 3 / 24 | 3 / 24 | 3 / 24 | 6 / 38 |
| fsmash | 18 / 43 | 20 / 48 | 17 / 38 | 15 / 46 |
| usmash | 16 / 40 | 18 / 44 | 14 / 35 | **9** / 39 |
| dsmash | 13 / 36 | 17 / 44 | 13 / 37 | **5** / 43 |
| nair | 4–12 / 22, LL 6 | 4–11 / 21, **LL none** | 3–9 / 17, **LL none** | 3–28 / 42, LL 6 |
| fair | 5–10 / 23, LL 11 | 7–11 / 23, LL none | 5–9 / 19, LL none | 16–19 / 56, LL 16 |
| bair | 6–10 / 25, LL 12 | 8–12 / 26, LL none | 6–10 / 22, LL none | 6–13 / 33, LL 6 |
| uair | 4–10 / 22, LL 8 | 5–10 / 20, LL none | 4–8 / 17, LL none | 4–8 / 32, LL 6 |
| dair | 7–13 / 29, LL 17 | 8–13 / 29, LL none | 7–11 / 24, LL none | 5–19 / 44, LL 12 |
| grab | 4 / 19 | — | — | 6 / 34 |

What the table says:

- **Every fighter's aerials are 17–29 frames.** Ultimate's are 32–56. An
  aerial that ends in 20 frames can be thrown twice in a short hop and never
  commits. The lingering-hitbox aerial (nair active for 25 frames) does not exist.
- **Only the robot authors landing lag.** Two of the three sampled fighters land
  from any aerial with no lag at all (`landing_lag_s: None`, so the Smash
  `LandingLagRules` scale has nothing to scale). The inventory's
  `Aerial attacks ✔` is true of the mechanism, not of the content.
- **Tilts are 2–4 frames longer than the jab.** In Ultimate a tilt costs twice
  the jab's recovery; here nothing separates "safe poke" from "committed poke".
- **Up and down smash start at 13–18.** Ultimate's up smash is the fast
  out-of-shield answer (frame 9 for Mario, 5–8 for many); here it is slower than
  the forward smash's purpose.
- The three fighters' tables are near-copies of each other (dash attack
  `3/24`, dash grab `6/33` everywhere). The roster has one frame-data shape.
- Grab total 19 vs 34: a grab that whiffs is nearly free.

### 5. Walk, dash and crouch (CODE-READ)

- `movement/integration.rs:805` `resolve_initial_dash`: a dash arms on any
  tick where the stick crosses `STEER_DEADZONE` (0.5) from a different held
  direction. There is no flick test. So a gentle push to 0.6 is an initial dash
  at `initial_dash_speed`; a walk exists only at ≤ 0.5 deflection (MEASURED
  "half-stick speed" 158 px/s = 0.875 u/f, under Mario's walk 1.155). In
  Ultimate the walk spans most of the stick's range and a dash needs a flick
  (the stick reaching the smash threshold within a few frames); that split is
  why walking up and tilting is a neutral-game tool.
- `match_rules.ron` abilities: `crouch: false` in `granted` **and**
  `permitted`, and `body_mode/mechanics/mod.rs:281` enters the crouch stance
  only with `abilities.crouch`. So no fighter crouches in a Smash match:
  `crouch_cancel_scale: 0.85` (room_rules.ron) has no customer, the crawl row
  (`S0.7`) is unreachable, and ducking under a jab is not a thing. The
  `crouch_speed_frac: 0.0` body patch suggests this is an oversight in the
  ceiling, not a ruling.
- `coyote_time: 0.120` (platformer_defaults.ron) is not patched by the Smash
  body: a fighter can jump 7 frames after walking off a ledge. Ultimate: 0.

### Input road (CODE-READ, no gap found)

60 Hz fixed tick; GGRS local `input_delay: 0`; attack action buffer 0.1 s
(6 f; Ultimate 10 f); flick threshold 0.8 inside a 4-tick window; directional
deadzone 0.3; jump buffer 8 f. Keyboard: a `SmashAttack` binding and the
`Walk` cap exist. Nothing here explains the feel; leave it.

## Presentation: what a hit shows and sounds like

Jon, 2026-10-29: "some of our simplistic posing and sfx and vfx could also be
the issue." Audited the same day with `moveset_render --overlay art`
(robot v3 vs `smash_duelist_a`: dash attack connecting, nair, forward tilt,
forward smash) and the FX/SFX roads. Contact sheets were built from the
per-tick PNGs; re-render with the command in each row to look again.

| # | Finding | Kind | Ultimate | Ambition |
|---|---|---|---|---|
| P1 | **One hit spark for every hit.** `VfxMessage::Impact { pos }` carries only a position; the marker is `hit_soft` at 0.6 scale, upright (`fx.rs:507-515`); the fallback is a 12 px yellow square. On the rendered dash attack it is an ~8 px white star. | CODE-READ + RENDER | spark size grows with damage, is rotated to the launch angle, has element families (slash, punch, fire, electric); a smash connect fills a quarter of the fighter | one asset, one size, one orientation |
| P2 | **One hit sound for the cast.** 155 of ~200 authored hit volumes name `player.hit` (a 120 ms brown-noise thud); the robot's 19 `slash.impact` cues are the only per-move set. Nothing scales a hit cue by strength at playback (no consumer of `hit_strength_fraction` in the audio crates). Nine `fighter.{sword,brawler,gunner}.smash_*` cues exist in the bank and **nothing names them**. | AUTHORED + CODE-READ | weak / mid / strong / kill impact families per element, a swing whoosh per move, and the kill-hit sound is a feel pillar | one thud; per-move whooshes only on the robot's blade |
| P3 | **The victim does not sell the hit.** Flash is a flat full-white body for the hitlag (2–6 f, so a blink); no per-frame jitter of the victim during hitlag; `hit` row is 5 frames, `launch` 7; a trail only at hard launch. The two bodies overlap through the connect. | RENDER + CODE-READ | victim shakes ±2 px every frame of hitlag, the launch pose is a distinct silhouette, smoke/dust at the launch | a white blink and a small slide |
| P4 | **Attacker posing holds one picture.** 6–7 art frames per attack; the dash attack shows the same lunge for ticks 3–8; the nair's silhouette barely differs from idle; no anticipation snap, no smear/overshoot frame at contact, no settle. The blade is a 2 px line while its hit volume is 40 × 28 px. The smash charge is the held pose plus an orange tint (`CHARGE_TINT`), no shake, no sparks. | RENDER | anticipation (the body stretches away), 1–2 smear frames through the swing, an overshoot pose at contact, a settle; charge shakes the body and sparkles | one pose per phase |
| P5 | **The slash VFX reads as a halo.** A translucent grey disc sits behind the body rather than an arc that travels with the blade (Jon's standing note: a slash travels and follows through). | RENDER | the arc is the swing | a disc |
| P6 | **Movement FX are procedural dots, and two beats have none.** CORRECTED 2026-10-29 (the first reading grepped `-v fx.rs`, which also hid `movement_fx.rs`): `emit_movement_fx` does send `Dust` on jump, landing, the traversal dash, rolls and ledge jumps, and a hard landing splats. What is missing: the initial dash start and the turnaround (not `MovementOp`s), and the dust itself is six procedural dots (`spawn_dust`) while the FX sheet's authored `landing_puff` / `skid_puff` rows go unused; no squash/stretch on land or jump. | CODE-READ | authored dust on every dash start, turnaround, landing, jump, roll; landing squash | dots on some beats |
| P7 | **Camera.** At the match zoom a fighter is ~50 px tall, so posing and sparks are read at thumbnail size. | RENDER | a tight two-fighter camera that zooms in on a close exchange | wide |

How this interacts with the simulation findings: P1–P3 are mostly invisible
today *because* hitlag is 2–6 frames (finding 2). R2 makes a connect last 8–19
frames; without P1–P3 that is 19 frames of a white blink. Ship them together.

## Status (2026-10-29, evening)

| Step | State | Commit |
|---|---|---|
| R1 hitstun lockout, R2 hitlag law, hitstun scale | ✔ shipped as `CombatRules` (`hitstun_locks_actions`, `hitlag: Affine`, `hitstun_time`), Smash declares them | 0f8c99ecb |
| R4 walk/dash threshold, crouch, coyote 0, parry on release | ✔ `dash_deflection` 0.8 (schema 387) | 71381014c |
| P1 spark carries the blow | ✔ tiered rows, scaled, turned to the launch | 6420ab194 |
| P3 victim shake, flash lasts the freeze | ✔ shake; flash ≥ hitlag | 1005e9ad7, this commit |
| P6 authored puffs, dash/turn skid | ✔ (squash/stretch open) | 1005e9ad7 |
| P2 tiered hit and swing sounds | ✔ 12 cues (reel in `untracked/sfx-candidates/smash-feel/`), tier pick in `resolve_strike_sfx` | 865e5e460 |
| R3 frame data | ✔ per fighter, in the facet (`move_timing`), 16 fighters; sanic has no facet; George is Rust-authored | 223dc85e7 |
| R6 buffer 10 f, tap jump, short-hop button | ✔ `action_buffer_s`; `ControlSettings::tap_jump` (off by default, no menu toggle yet); the `ShortHop` action already existed and is bound in the presets | e523311c7, this commit |
| P4 posing keys, P5 slash arcs | ✔ for the three polygons (Jon: the polygons are the Smash test players, not the robot): 12-frame rows with anticipation / smear / contact-overshoot / settle posed to each fighter's Smash windows, a shaking, glinting `smash_charge`, travelling arcs in the `*_strike_fx` rows (renderer c752183, generation 55069d31; review sheets `untracked/art-review/polygon-keys/`). Open: the beast's coil is subtle; the other fighters' rows are still one picture per phase | 96552c2c6 |
| P7 fighter camera | ✔ `MatchRules::cast_framing` (Smash: close to 45% of the stage view, 40 px margin; the engine keeps 75% / 48) | cad1bfd1b |
| P7 camera box, eased margins, floor room | ✔ `cast_framing.{open_hz,close_hz,floor_margin,bounds}` (Smash 4/4 Hz, 40 px, side 280 / rise 170 / fall 100); measured by `match-report --camera`, see § Camera diagnosis | e121708cf |
| P7 finishing blow | ✔ `zoom_camera_on_a_finishing_blow` + `launch_leaves_the_world` (last stock, predicted KO → finish zoom + 0.5 s freeze) | 9eaf73d32 |
| P6 squash/stretch | ✔ `BodyPoseView::squash`: lands wide and low, relaxing over the landing beat (deeper when hard); the jump squat presses | this commit |
| Tap-jump menu toggle | ✔ Controls page, "Tap Jump" | this commit |
| R6 authored walk speed, angled air-dodge lag | ✔ `walk_speed` (Smash 208 px/s), `air_dodge_up` → 0.73×–1.27× landing lag; schema 388 | 38e725306 |
| CPU fighters like an Ultimate CPU | in progress (sub-agent lane on its own branch). First merge d9128c7e4: `smash_it::every_authored_route_gets_pressed` went red after the feel pass; the cause was NOT the hitstun lockout (my first reading, refuted by trace: zero presses matured inside stun) but the recovery lens probing with the room box while the game kills at the room plus the blast margins, so a fighter launched past the room edge saw no route. The brain now reads the death line (`WorldView::blast`), and holds a press made in hitstun for the first actionable tick | d9128c7e4 |
| R5 play with Jon; posing keys beyond the polygons; Sanic's facet | open | — |

Nothing above has been played by a human yet. Every number is a first cut.

**The polygons' references (Jon, 2026-10-29):** pointed polygon ≈ Marth /
Lucina (`sword` template, Lucina body), pugnacious polygon ≈ Captain Falcon
(`falcon` template, Falcon body at 0.6), projectile polygon ≈ Yoshi's body with
some of Samus's and Toon Link's moves (`zoner` template, Yoshi body at 0.6).
Ultimate is a reference, not a copy. The CPU fighters are to feel like an
Ultimate CPU (a sub-agent lane on its own branch; see the fighter-brain plan).
The polygons' posing keys were fitted to the earlier fast/heavy clocks; the
sword and falcon clocks are close enough that the smear and contact frames
still fall in the active windows, but a re-fit is owed with the next art pass.

**Measured after the pass** (`ko_envelope feel pointed_polygon pugnacious_polygon`,
2026-10-29): hitlag is the genre's (jab 8 f, tilts 10, aerials 12, fsmash
16); the facet clocks are live (nair 38 f, fair 40, fsmash 42); hitstun
tracks Ultimate's 0.4 × KB at these launches (jab 10 f at 120 px/s, fsmash 29
at 370). On-hit advantage (`adv_f`) is still negative for most moves below
80% (jab −11 at 0%, ftilt −14, nair −31, fair −24; utilt +6, usmash +13 at
40%, bair +5 at 80%). That was a per-move BASE KNOCKBACK question (the
polygons' tilts and aerials launched 120–290 px/s at 0%, short stun against
24–40 frame moves). Answered the same evening with
`SmashFighterFacet::move_knockback` (a factor on each move's base launch on
the Smash stage, growth untouched; written by `scripts/smash_retime.py`:
forward/down tilt ×1.6, dash attack ×1.5, nair/fair ×1.5, bair/uair ×1.3,
dair ×1.4, up tilt and smashes ×1.0). Re-measured: ftilt −6 at 0% and −1 at
40%, dtilt −7 / −2, bair −7 / +2, utilt +6 / +21; aerials stay negative in
the air as Ultimate's do unless landed (the tool does not model landing). A
first cut for the play pass.
Also measured: `smash_tool match-report 30 --runs 2` plays through (KOs,
launches, techs; no panics) under the new rules.

## Roadmap

Two tracks. The simulation track (R) changes what a hit is; the presentation
track (P) changes what it shows. R1–R2 and P1–P3 are one feel change and
should be played together.

Order is by feel-per-effort. Each step names its instrument, because the
inventory's lesson is that a wired mechanism and a felt mechanism are different
claims. Every number below is a first cut for Jon to play, not a ruling.

### R1. Hitstun is a lockout (engine, E1, S)

Make the whole of hitstun a hard lock under a match rule, keeping Ambition's
"swing back while flashing" at home.

- Add `CombatRules::hitstun_locks_actions: bool` (or fold the recoil lock to
  the hitstun length when the rule is on). Smash declares it; Ambition keeps
  the recoil lock. Stick authority in hitstun becomes DI/SDI only
  (`hitstun_control_scale` → 0 under the rule).
- Keep: tech out of tumble, air dodge/attack **after** hitstun ends, fast fall
  not allowed in hitstun.
- Witness: a headless test — attacker lands a tilt, victim holds attack every
  tick, assert no move starts before `hitstun_timer` reaches 0 under Smash
  rules, and one starts after the recoil lock at home.
- Instrument: `ko_envelope feel` already prints on-hit advantage from the
  timers; after R1 it is true advantage. Read it per move at 0/50/100%.

### R2. Hitlag at Ultimate's scale (tuning + one rule, S)

- Give the Smash feel its own hitlag reference: `hitlag_time` ≈ 0.217 s
  (13 f at 11%) and raise `MAX_HITLAG_SCALE` so a 20% smash gets ~19 f; or
  author it as frames: `hitlag = (0.65 × damage + 6) / 60` under a match rule
  so the authored shape is Ultimate's. Ambition keeps 0.070.
- Add the per-hit hitlag multiplier from `S0.3` only if a move needs it
  (electric, charged smash ×1.2–1.5).
- Check the presentation holds at 3× the freeze: sprite freeze, hit flash, camera
  shake, SDI 3 px/tick (now 13 ticks: re-tune `sdi_step` down so SDI total stays
  in band).
- Instrument: `moveset_takes` per-tick recording already carries the freeze;
  compare a jab and a smash connect side by side.

### R3. Frame-data pass over every seated fighter (content, M)

One pass, one shape per archetype, measured against the UFD anchors
(`fetch_smash_ultimate_moves.py` extended to frames; put the table in
`docs/related-work/data/`).

- Aerials: totals to 30–55 frames; active windows that linger (nair) or
  are a single beat (bair, fair); landing lag on **every** aerial, 6–16 f;
  autocancel windows at the start and the tail.
- Tilts: recovery about 2× the jab's; jab 1 → 2 → 3 chain cadence.
- Smashes: up and down smash startup 6–12 f; forward smash stays slow.
- Dash attack 6–10 f startup, 35–40 f total. Grab 6 f startup, 30–35 f total;
  dash grab slower, pivot grab faster.
- Give the three archetypes (fast/light, mid, heavy) distinct tables; stop
  copying `3/24` and `6/33` across the roster.
- Instrument: `scripts/smash_move_sheet.py` + `smash_hitbox_census.py` for
  the pictures; `scripts/smash_frame_data.py` for the numbers beside UFD's.

### R4. Walk / dash / crouch (engine, E1, M)

- Dash on a **flick**: arm the initial dash only when the stick crosses a
  dash threshold (≈0.8) within N ticks (≈3) of leaving a lower band; a slow
  push past 0.5 walks. `AttackGestureTuning` already has the flick machinery
  (`flick_threshold`, `flick_window_ticks`); reuse the detector, do not write
  a second one. Walk speed = `walk_speed` authored per fighter (Ultimate
  median 1.11 u/f) rather than the run continuum at half stick.
- Crouch: `permitted.crouch: true` (and `granted`) in Smash's ceiling; keep
  `crouch_speed_frac: 0.0` for non-crawlers; this also makes crouch cancel
  real and gives `S0.7` crawl its customer.
- `coyote_time` patched to 0 by the Smash body.
- Instrument: `fighter_motion` gains "walk speed at 0.7 stick" and "dash from
  slow push" rows.

### R5. Per-fighter shape review with Jon (content, S, blocked on play)

The 60%-strength archetype shapes are unplayed. After R1–R4 land, play
three matchups (robot/admiral, alice/goblin, two robots) and tune: robot's
gravity (1.22× Mario reads heavy for a light protagonist), the heavies' fall
speeds, and the jump squat's 2-vs-3 frame rounding.

### R6. Small genre fixes (S each, after R1–R4)

Parry `OnRelease` as Ultimate (both are live; stage choice). Tap jump and a
short-hop button (`§9` rows). Attack buffer 10 f. Directional air dodge
landing lag 11–19 f by angle (one number today).

### P1. The hit spark carries the hit (engine E1 + art, M)

- `VfxMessage::Impact` gains what the resolved hit knows: damage (or the
  `hit_strength_fraction`), launch angle, and the volume's element/family
  (slash, blunt, pierce, fire, electric; a `HitVolume` field with a default).
  One send site per road already exists (`hitbox/mod.rs:1508`, `actor_hit.rs`,
  `damage/mod.rs`); pass the facts, do not infer them in the renderer.
- Three authored spark tiers per family (weak / mid / strong) in the FX sheet,
  drawn by the sprite renderer, scaled by strength between tiers and rotated to
  the launch; the kill tier adds the ring. ⛔ Authored assets, not a shader.
- Instrument: `moveset_render` on a jab, a tilt and a charged smash connect;
  the spark must be visibly three sizes.

### P2. Hit and swing sounds by strength (content, M)

- A hit cue per family and tier: `hit.<family>.{weak,mid,strong,kill}`;
  the playback picks the tier from the same strength the spark uses, and
  nudges pitch down with strength. Retire `player.hit` as the roster's hit.
- A swing whoosh per move class (jab, tilt, smash, aerial), authored once per
  weapon family (blade, fist, gun) and named by each fighter's moves; bind the
  nine unused `fighter.*.smash_*` cues or delete them.
- Audition candidates on a reel first (the SFX rule), then port exactly.
- Instrument: a `moveset_takes` run with the cue log; every connect must name
  a tier.

### P3. The victim sells it (engine E1 + art, S/M)

- Hitlag jitter: the presented pose (not the sim pose) offsets ±2 px on
  alternate ticks of hitlag, scaled by strength; the attacker holds still.
  `presented_pose.rs` already separates the two.
- Flash: keep the white pop, but let it fall off over the hitlag instead of a
  flat hold (`FLASH_HOLD_FRACTION` 0.8 → a curve), so a strong hit reads as a
  longer glow and a jab as a tick.
- Launch pose: `launch` and `tumble` rows need a silhouette that is not
  `hit`; add launch dust at the victim's feet on a grounded launch.
- Instrument: `moveset_render` of a tilt and a smash connect at 0% and 100%.

### P4. Posing keys per attack (art, M, per fighter)

Each attack row gets four keys: anticipation (body and weapon pulled the other
way, 2–4 f), smear (1–2 f, the weapon as a shape across the arc), contact
(overshoot: weapon past the target, body leaning in), settle (back to idle
over the recovery). Draw the blade with a width at the smear, not a line.
Smash charge: the body shakes 1 px on the row and the charge sparkles. Start
with the robot (its generator is the reference rig), then one fighter per
archetype. Frame budgets come from R3 so the keys land on the windows.

### P5. The slash is an arc that travels (art, S)

Re-author `slash_rise/fall/arc/poke` as a bright arc that follows the blade
across the active window and fades on the follow-through; delete the disc.
Jon's standing note on slash VFX applies.

### P6. Movement dust and squash (engine E1 + art, S/M)

The simulation already sends `Dust` on jump, landing, the traversal dash,
rolls and ledge jumps (`emit_movement_fx`). Draw it with the authored
`landing_puff` row instead of six procedural dots; add the skid
(`skid_puff`) on the initial dash start and the turnaround from the
published motion facts' rising edge (neither is a `MovementOp`). Squash on
landing and stretch on takeoff as a presented-pose scale over 3–4 frames.
⛔ No drop shadows.

### P7. A fighter camera (engine, M, after P1–P4)

Frame the two fighters tighter (Ultimate's camera: the pair fills the view
with a margin, zooms in on a close exchange, out on a launch), with the finish
zoom already built as the model. Mobile keeps the wide frame if it needs it.

#### Camera diagnosis (2026-10-29, late; Jon: "diagnose potential issues with how the camera is working in smash")

Instrument: `smash_tool match-report --camera` (a per-tick trace of the
resolved view against the seated bodies, reduced to the numbers below).
Baseline MEASURED on the duelist mirror, 2 × 30 s CPU-versus-CPU, room
640 × 480, blast fall 240 / side 400 / rise 240:

| | run 1 | run 2 |
|---|---|---|
| view width px (min / median / max) | 256 / 406 / 1256 | 256 / 497 / 1256 |
| zoom rate px/s (open / close) | 10 211 / −4 348 | 13 794 / −4 077 |
| ticks with the zoom moving > 60 px/s | 71% | 75% |
| ticks the view shows past the room | 54% | 52% |
| fighter height, fraction of the view (min / median / max) | 0.07 / 0.21 / 0.33 | 0.07 / 0.17 / 0.33 |
| fighter sits at (0 = top) | 0.55 | 0.57 |
| body-ticks cut off by the view's edge | 0.2% | 0.9% |

What the code did (CODE-READ, `camera_snapshot.rs`): the cast box ADOPTED an
outward edge at once and closed at 10 Hz; `base_view` was that eased box plus
the margin with no easing of its own, so a launch opened the view as fast as
the body flew; there was no camera box, so the view followed a launched body
to the blast line (up to 1 256 px across, twice the room, showing the void);
the cast was centred with no floor room; the only zoom event was the verdict's
finish zoom.

What the reference does. Nothing public gives Ultimate's camera code; the
primary source is the Melee decompilation (`doldecomp/melee`,
`src/melee/cm/camera.c`, read directly — Jon: "probably something we can
learn a lot of good lessons from"), and Ultimate's STAGE camera boxes are in
the LVD data (rubendal's SSBU-Calculator `ssbustagedata.json`). MEASURED from
those:

- Every stage has a CAMERA BOX inside its blast box: Battlefield camera
  ±170 / 130 / −80 against blast ±240 / 192 / −140, Final Destination
  ±180 / 130 / −60 against ±240 / 180 / −140 — the camera stops at ~70–75%
  of the side and rise blast distance and ~45% of the fall. A body past the
  box is framed AT the box (Melee clamps each subject's framing points to it),
  so the fighter leaves the frame before the line, into the magnifier bubble.
- Each fighter is framed by a box of its own (front / back / up / down
  extents) scaled by the cast count — 1 → 1.5, 2 → 1.32, 3 → 1.16, 4 → 1.0
  (`cm_803BCB9C`) — and by a per-stage ratio; the extents grow or shrink at
  most 0.5 units per frame, which is the lag on the margin.
- The bottom of the combined box is lowered by `10 + 390 · z` (z from camera
  distance), and a framing point clamped at the bottom bound is kept above the
  GROUND under it: the floor the fighters stand on stays in view.
- Pan is a per-frame fraction of the remaining distance, blended by how far
  apart the fighters are; the camera position (zoom) moves by one constant
  fraction per frame, the SAME in and out; no look-ahead, no dead zone.
- Ordinary hits never request a camera quake in Melee (KO / Star KO do, 22
  frames). Ultimate's hit shake is UNSOURCED.
- Ultimate's Finish Zoom fires on a hit the game predicts will win the match
  (knockback and angle, ignoring DI; 1-on-1 in practice); Special Zoom on a
  few authored moves. Durations UNSOURCED.

Design, shipped as `MatchRules::cast_framing` fields (the engine's defaults
keep the exploration camera byte-for-byte): `open_hz` / `close_hz` ease the
framed box both ways (Smash 4 / 4; engine adopts outward at once / 10 Hz),
with the view FLOORED at the cast's raw span and the clamp holding the eased
box grown to the raw one, so a launched body leads the view to its edge and
is never cut off; `floor_margin` (Smash 40 px) keeps the ground in view and
sits the fighters above the centre; `bounds` (Smash side 280 / rise 170 /
fall 100, ≈70% of the side and rise blast distance) is the camera box: the
cast is clipped to it, the view is capped to it and clamps inside it instead
of the room. Per-fighter front/back extents and the cast-count scale are NOT
built: with two polygons one margin and one floor number are the whole
difference, and the count scale is a four-player question.

After, MEASURED with the same instrument, same matchup, same seeds:

| | run 1 | run 2 |
|---|---|---|
| view width px (min / median / max) | 299 / 500 / 999 | 299 / 475 / 1 024 |
| zoom rate px/s (open / close) | 1 077 / −885 | 21 457¹ / −888 |
| ticks the view shows past the room | 63% | 84% |
| fighter height, fraction of the view (min / median / max) | 0.09 / 0.17 / 0.29 | 0.08 / 0.18 / 0.29 |
| fighter sits at (0 = top) | 0.47 | 0.50 |
| body-ticks cut off INSIDE the camera box | 0.5% | 0.9% |
| body-ticks past the camera box (leave the frame by design) | 269 | 316 |

¹ One tick at a respawn placement; the baseline had the same spike (10–14k).
The view now shows past the ROOM more often because the room is no longer
the clamp — the camera box is; "past the box" is 0 ticks by construction.
The remaining in-box cut-offs are bodies whose 48 px box straddles the fall
bound by ≤ 4 px (their centre is inside the box, their feet past it).
`every_live_fighter_stays_inside_the_frame` now skips a body past the camera
box and holds for every body inside it. Two bugs found by the instrument on
the way: the view was floored at the raw span but the clamp held the eased
box GROWN to the raw one (a taller union than the view — 17 px cut-offs),
and a view capped to the box could not fit a cast as tall as the box.

The finishing blow (same night): `zoom_camera_on_a_finishing_blow`
(combat, beside the verdict zoom) fires on the hit tick when a fighter on its
LAST stock is launched and `launch_leaves_the_world` (core, the ballistic
arc under the body's resolved gravity, sampled 32×) crosses a side or the
rise line before the body is back at floor level — Ultimate's Finish Zoom
rule (knockback and angle, no DI). It requests the finish zoom and holds
every fighter in `FINISHING_BLOW_FREEZE_S` 0.5 s of hitstop. A spike is not
predicted (whether it falls out is the recovery game). The launch is read
while STAGED, the one tick it is visible (the kernel consumes it next tick),
so the trigger keeps no memory and is rollback-safe. Not built: Ultimate's
red flash and slow-motion (presentation; the freeze stands in).

Still open after this: the off-screen bubble for a body between the camera
box and the blast line (art + HUD, needs Jon's eye) and a human pass on the
rates (4 Hz both ways is a first value; Melee's per-frame fractions are in a
binary table nobody has read).

### Not in this roadmap

Stages, items, modes, CPU ladder, presentation polish, and every `S0.*` packet
that is not R1 (the gate's own order stands). Knockback/KO bands were
re-measured 2026-10-18 and are in band; do not touch them until R1–R3 change
what a hit is worth, then re-run `ko_envelope feel`.

## Acceptance for "feels like Ultimate"

Not a byte match. The following should be true in a two-human match on the
flat stage, measurable with the named instruments:

1. A landed tilt at 0% leaves the attacker actionable before the victim
   (`ko_envelope feel` advantage > 0 for every tilt and aerial at 0–50%).
2. A smash connect freezes both bodies for 15+ frames and a jab for 8
   (`moveset_takes`).
3. No aerial can be thrown twice in one short hop; every aerial has landing
   lag (`smash_frame_data.py` census: 0 aerials with `LL none`).
4. A slow push walks; a flick dashes; a fighter can crouch (`fighter_motion`).
5. `fighter_motion` stays in band for every roster fighter (no regression from
   R4).
