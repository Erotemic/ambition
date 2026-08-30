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

## ▢ The architecture proposals, which stand on their own merits

The review is right that the capability is scattered across binaries, and none of
these depend on its environmental premise.

- ▢ **Extract `DeterministicCaptureSession`.** The hard part `moveset_render`
  solved — sim advances only on canonical fixed ticks while readback is serviced
  with `ManualDuration(ZERO)`, so GPU latency cannot change WHICH ticks are
  captured — lives in a `bin/`. ⭐ that is the reusable piece: room capture,
  match screenshots, character previews and visual regression all want it.
  `ambition_render::capture` stays the low-level texture/readback mechanism; the
  session belongs one layer above, because it needs app composition and stepping.
- ▢ **Promote `move_exercise.rs` out of `#[path]`-inclusion.** `moveset_takes`
  and `moveset_render` both textually include the same file because
  `ambition_app_tools` is deliberately not a library. It carries learned
  semantics — tilt vs smash stick magnitude, airborne preparation, back-air
  facing stabilisation, input-edge semantics, the charge hold/release schedule,
  canonical one-tick stepping, intended-vs-observed classification — and **the
  file's own comment says it should move out once it is reusable domain API.**
  It is. ⇒ `ambition_sim_harness` or a developer-harness crate above it.
- ▢ **A render-independent `BodyPresentationView` (pose / clip / clip_frame /
  facing / sprite row / mirroring).** ⭐ **the review is right that this is the
  biggest one for agent workflows.** Today `CharacterAnimator` exists only once
  the render layer loaded a `CharacterSpriteAsset`, and `BodyPoseView` is gated
  by `PlayerVisual`, which Smash seats never receive — so the inspector
  RECONSTRUCTS the frame cursor in JavaScript. A presentation read model built
  before rendering would let `moveset_takes` record the engine's actual animation
  decision with no rasterizer, and delete the duplicated cursor logic.
  ⚠ `docs/inspector.md` already names this as "the bounded piece of work".
- ▣ **A render-capability doctor — BUILT 2026-08-29.**
  `scripts/render_capability_doctor.py`, surfaced as `render_capability` on
  `/api/status`. It reads the loader and the ICD directory, which is what decides
  whether WGPU has a device behind it, and answers in a second rather than after
  a whole-game compose. ⛔ **IT REPORTS AND DOES NOT PROVE**: an ICD on disk is
  necessary and not sufficient, so the verdict is `likely`, never `available`,
  and it states that it created no adapter — an engine render succeeding is the
  authoritative answer. Four arms, and they are about what it SAYS rather than
  this machine's state: loader-without-ICD names the package, ICD-without-loader
  is a different answer, Lavapipe alone is enough, and a hardware ICD counts.
  ▢ **STILL OPEN from this row:** an explicit `auto | hardware | software`
  adapter request, so a CI/agent job's environment does not change when a driver
  appears or disappears.
- ▢ **A CPU diagnostic renderer** as an exportable artifact rather than a browser
  canvas — the inspector already implements sprite-sheet crop, body box, combat
  volumes, projectiles in JS, and `ambition_app_tools` already depends on
  `image`. ⭐ **label the two outputs separately**: an *engine render* is what the
  production Bevy graph drew; a *diagnostic render* is derived. The inspector is
  already careful never to pass one off as the other, and that must survive.

## What this review got right that is worth keeping

⭐ Its read of `moveset_render`'s core mechanism is correct and is the reason the
extraction is worth doing: the manifest can genuinely say *"this PNG is action
tick 17"*, and that property is what every future capture tool wants.
