# GPT review — the inspector's render capability (relayed by Jon, 2026-08-29)

The review's thesis: `moveset_render` is blocked by a missing Vulkan ICD rather
than by any engine limitation, Lavapipe would unblock it, and three architecture
extractions would turn the moveset inspector into a general developer-engine
observation capability.

## ⛔⛔ THE ENVIRONMENTAL FINDING DOES NOT HOLD ON THIS VM — MEASURED

The review reports *"no Vulkan ICD at all under /usr/share/vulkan/icd.d"* and
recommends provisioning `mesa-vulkan-drivers` as the immediate unblock. On
`aivm-2404`, 2026-08-29:

```text
/usr/share/vulkan/icd.d/lvp_icd.json          present  (Lavapipe)
/usr/lib/x86_64-linux-gnu/libvulkan_lvp.so    present  (12M)
mesa-vulkan-drivers                           25.2.8-0ubuntu0.24.04.2, installed
```

⭐⭐ **AND THE PIPELINE RUNS.** `moveset_render --character projectile_polygon
--verb attack --frames 3 --stride 2` produced **three real engine PNGs and a
manifest**, with `intended Some("polygon_projectile_jab")` matching
`observed {"polygon_projectile_jab"}` and **9 zero-time pumps** — the separation
of simulation time from GPU time the review praises, working. The offscreen boot
ran 698 frames at **p50 5.8ms**.

⇒ **the immediate recommendation is already satisfied here, and no engine
redesign stands between an agent and engine PNGs on this machine.** ⚠ The
reviewer's own sandbox genuinely lacks Cargo and an ICD, and its report about
ITSELF is accurate; what did not transfer is the inference about the agent VM.

## ⭐⭐ WHAT ACTUALLY SENT THE REVIEW THERE, AND IT IS A REAL DEFECT

`moveset_render --character player_robot` fails, and it used to fail saying:

```text
moveset_render: no live rollback session for 'player_robot'
```

That message names the LAST condition in an `&&` of three. The true cause is the
FIRST: `player_robot` is not on the smash grid, so `smash_roster` seats nobody,
`staged` stays 0 — and "no live rollback session" reads as a broken session or a
missing GPU. ⛔⛔ **The loop's own comment had already learned this exact lesson
one layer down** (*"reported 'no move ever became active' — which reads as a
broken press rather than a press nobody was listening to"*) and then the failure
message repeated it one layer up.

▣ **FIXED 2026-08-29.** The report distinguishes the three conditions and, for
the common one, says the id is the suspect rather than the renderer:

```text
moveset_render: 'player_robot' seated nobody — the match staged 0 fighters, so
this is almost certainly a character id the smash grid does not carry rather
than anything about rendering.
```

▣ **AND THE STALE DOC IS FIXED.** `docs/inspector.md`'s *"The engine render (GPU,
on demand)"* section still said **ONE binary: `capture_scene`** and described
photographing a fighter STANDING, while the newer section below it described
`moveset_render` performing the move. The server has invoked `moveset_render`
for some time. Two sections of one document described two architectures; the
superseded one is rewritten and says what it used to claim.

## ▣ The architecture proposals — ALL FIVE LANDED 2026-08-29

The review was right that the capability was scattered across binaries, and none
of it depended on its environmental premise. ⭐ The one row that is not finished
is named inside its own bullet: an explicit `auto | hardware | software` request
shipped, but only `moveset_render` takes the flag.

⛔⛔ **THE RECURRING LESSON OF THIS FILE: EVERY MOVE MADE A LATENT RULE LIVE.**
Promoting `move_exercise` reddened an SDK contract; widening the pose read model
reddened a test that assumed one row. Both were correct guards doing their job,
and both were cheaper to answer than to waive — the SDK gaps got closed and the
test learned to ask for the PLAYER's pose rather than the only one.

- ▣ **`DeterministicCaptureSession` EXTRACTED 2026-08-29** into
  `ambition_sim_harness::capture`, behind an optional `capture` feature so a
  test, fuzz driver or RL agent keeps paying for no renderer. The hard part it
  owns is that **a slow adapter must not change WHICH tick a picture belongs
  to**: the sim advances only at the caller's canonical period, a pending
  readback is serviced with `ManualDuration(ZERO)`, and the session CHECKS the
  tick after every pump and refuses the frame if it moved. ⛔ that check was a
  `debug_assert!`, which COMPILES OUT OF A RELEASE BUILD — the profile the
  inspector's server prefers. ⛔ composition and the `Startup` ordering stay with
  the caller, because `PresentationSetupSet` is the product shell's and the
  harness sits below it; a no-op local anchor would order against nothing.
- ▣ **`move_exercise` PROMOTED 2026-08-29**, and its own doc had asked for it.
  Both binaries `#[path]`-included one file, so each compiled a copy and treated
  the other's half as dead code. It landed in the harness rather than a new crate
  because its dependencies are exactly the harness's.
  ⭐ **THE MOVE MADE A LATENT RULE LIVE**: `sim-harness-names-only-the-public-sdk`
  went red on `engine_core`, `combat` and `mount`, and that contract's own reason
  says what to do — *"if it needs crate-shaped facade paths, those are SDK gaps."*
  Four gaps closed (`actor::BodyGroundState`, `actor::MovePlayback`,
  `actor::RidingOn`, `sim::SimTick`) plus a new `capture` SDK module, rather than
  a waiver.
- ▣ **THE RENDER-INDEPENDENT PRESENTATION READ MODEL — DONE 2026-08-29, and the
  finding under it was smaller than the review framed.** `BodyPoseView` already
  WAS render-independent: a pure function of sim state, rebuilt every tick,
  declared rollback-DERIVED. It was gated on `With<PlayerVisual>` — granted in
  exactly one production place, the exploration player's avatar — so no
  `MatchSeat` fighter had one.
  ⭐ **MEASURED BEFORE: the recorded take on disk has `has_pose` true for 0 of
  13,947 bodies.** Every granted character body now carries `PosedBody` (a
  separate marker, because `PlayerVisual` means "the player's drawn avatar" and
  other presentation keys on it), and `moveset_render`'s manifest carries the
  engine's own decision beside each PNG — `tick 21 polygon_projectile_jab | pose:
  Idle | clip: jab`. Schema v135 → v136.
  ⛔⛔ **AND WIDENING A POPULATION MADE A LATENT RULE LIVE HERE TOO**:
  `gravity_symmetry_room` read *"the sole `BodyPoseView` in the world"*, true only
  while one entity could have one. Six bodies in that room have one now.
- ▣ **A CPU DIAGNOSTIC RENDERER — BUILT 2026-08-29.**
  `scripts/render_take_diagnostic.py` turns a recorded take into an SVG contact
  sheet — body boxes, combat volumes, projectiles, and the move/pose/clip of each
  tick — with **no WGPU, no sprite decode and no browser**. It existed only inside
  the inspector's canvas, so the one machine that could produce these pictures was
  one with a browser attached to a running server.
  ⭐ **SVG RATHER THAN PNG IS THE POINT**: geometry is what a take records, and
  rasterizing it would need the sheets decoded and a compositor — the work this
  avoids. Diffable, scalable, readable from the terminal that made it.
  ⛔⛔ **AND EVERY SHEET SAYS IT IS DERIVED, ON ITS FACE.** The inspector is
  careful never to pass a derived picture off as an engine render, and an exported
  file leaves the context that made that obvious — so the distinction lives on the
  picture. Five arms, and two are about the sampling: an evenly-spaced strip,
  because the first twelve ticks of a 150-tick take are the wind-up and a strip of
  them says the move does nothing.

## What this review got right that is worth keeping

⭐ Its read of `moveset_render`'s core mechanism is correct and is the reason the
extraction is worth doing: the manifest can genuinely say *"this PNG is action
tick 17"*, and that property is what every future capture tool wants.
