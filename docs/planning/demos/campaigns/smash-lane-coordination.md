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
| M2 smash charge | MECHANICS | ▢ |
| M3 invuln/armor windows | MECHANICS | ▢ |
| M4 out-of-shield policy | MECHANICS | ▢ |
| M5 jab chains | MECHANICS | ▢ |
| P1 launch trail | PRESENTATION | ✔ `882fe8fa5` — `LaunchedBodiesView` publishes involuntary flight; Dust plume behind the velocity vector, sim-tick cadence |
| P2 i-frame blink | PRESENTATION | ✔ `0c29e9cf0` — `unhittable` on both body read-models is `body_vulnerable` inverted; the hit-flash overlay carries both cues, damage wins |
| P3 tech/parry cues | PRESENTATION | ◑ tech half done `1f96165eb` — own spark ring and cue, split from GetupRoll; parry half still BLOCKED on M7's successful-contact fact |
| W7 dizzy stars | PRESENTATION | ✔ `f04989c78` — second pooled `GuardBreaksView`; stars orbit the body's own up; the bubble now turns with the body too |
| P4 bubble shield | PRESENTATION | ✔ `e5210712b` — filled field in front of the body, shieldstun flare, near-break danger flicker (part of W7) |
| P5 charge pulse/SFX | PRESENTATION | ✔ `19ec18c42` — authored `smash_charge` row routed ahead of the move's chain, third overlay cue quickens with the fraction, latch/lock cues authored procedurally |
| M6 DI reaction window | MECHANICS | ▢ |
| C1 CPU survival DI/SDI | COORDINATOR | ✔ |
| C2 CPU tech | COORDINATOR | ▢ |
| C4 CPU presses into endlag (`BufferableSoon`) | COORDINATOR | ▢ — needs the buffer window as a perceived fact |
| C3 CPU charges smashes | COORDINATOR | ▢ (after M2) |
