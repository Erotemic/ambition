# Track M — Super Mary-O (classic platformer acceptance demo)

Parody-original classic tile-platforming: teach-by-play opening, pipe secrets,
powerups, enemies, and a flag sequence without copied art or layout.

**Purpose:** prove that the conventional axis-swept AABB platformer is a simple
customer of the same engine that supports momentum and relativistic mechanics.
Classic behavior must be authored through engine seams rather than a privileged
"normal game" path.

## Current state

Landed:

- Mary-O Classic physics through reusable `AxisSwept` laws: separate
  acceleration/coast/skid rates, neutral-air momentum preservation,
  speed-banded launch, weak held-ascent gravity, strong released/fall gravity,
  gravity-zone covariance, and rollback-complete jump-arc state. Wall mobility
  and generic fast-fall were removed from her core kit pending dedicated wall
  jump and ground-pound abilities. Her whole profile is authored ONCE
  (`MARY_O_CLASSIC_AXIS_TUNING`) and substituted into every form, so small,
  tall, and fire cannot drift apart;
- launch bands ride as OFFSETS on `jump_speed`, so there is exactly one
  ground-jump height authority, and the top band's threshold sits inside her run
  cap — a running jump is her highest jump, as in the original;
- zero `coyote_time` and zero `jump_buffer` are DELIBERATE (Jon): the classic
  games grant no ledge forgiveness and no pre-landing buffer. Do not "fix" them;
- provider/demo shells, the authored level-1 room grammar, fixed-tick simulation,
  and the mode-scoped level clock;
- ?-block bonks that spawn real world-item pickups, with equip-on-touch through
  the shared item/equipment path;
- the grow-cap armor row, distinct tall worn identity, collider/body-size update,
  feet-planted grow/shrink behavior, and spark-blossom ranged move. ⚠ The
  blossom PICKUP was invisible from the day the spark form landed: the runtime
  bound `sprites/props/super_mary_o_spark_blossom.png` through `WorldItemArt`,
  but no generator target ever produced that file, so the item was collectible
  and undrawable. The target now exists in `super_mary_o_props.py` and publishes
  through `scripts/regen/sprites.sh`. Lesson: a `WorldItemArt` id that names a missing
  texture fails SILENTLY — nothing errors, the item is simply never drawn;
- breakable bricks through durable block-contact identity;
- crony enemies and shared stomp behavior;
- forward-only authored camera policy; and
- flag contact-height scoring, slide, walk-off, tally, dwell, and cyclic level
  restart.

Remaining acceptance work
(**this list is the single source; [`status.md`](../status.md) and [`tracks.md`](../tracks.md) refer here**):

- ✅ **The secret pipe and underground room** (landed 2026-07-21). A warp pipe
  between pits A and B drops into a coin vault dug under the ground slab, in the
  SAME `RoomSpec` rather than a second room — cross-room transition didn't exist
  yet (World 1-2 built it, below). The warp is a real `transit_body` (ADR 0024),
  not a position poke, so entering while wall-clinging reconciles correctly. Its
  8 coins are ordinary `currency` placements the shared economy collects.
- ✅ **A Solid Snake is the SHAPE its art is, per pose** (2026-07-25). Her
  collision box was a hand-guessed 28×32 rectangle for every phase of the
  withdraw cycle against a 117×49 sprawled sprite — wrong walking, wildly wrong
  boxed. The sheet is now the authority: `solid_snake` publishes per-animation
  body rectangles, and the new engine seam `character_sprites::posed_body`
  derives the collision box, sprite quad, and quad placement from the rectangle
  for the pose being shown, from one authored scalar (world units per sheet
  pixel). Walking she is 58×26, boxed 26×19; resizes are feet-anchored. The seam
  is opt-in per body (`SpritePosedBody`) and host-registered — a content crate
  cannot add the component and silently get nothing — and reads the sim-side
  pose pin (`ActorAnimOverride`), never the render-side locomotion picker, so
  the collision box does not depend on whether anyone is watching.
- ✅ **The brainless sliding shell** (landed 2026-07-21). A stomped crony leaves
  a shell: walking into a resting one launches it away from the touched side (so
  you aim it), walking into a sliding one stops it dead. A sliding shell runs
  down cronies and reverses at walls — the demo's one emergent combo. BRAINLESS
  is literal: the archetype's brain is `StandStill`; gravity, ground contact and
  walls are ordinary body physics. It shipped inert (matched actors on `Name`,
  but the spawner writes the tag onto `FeatureName`); fixed by matching
  `FeatureName` instead, guarded by a regression test that drives the real spawn
  path.
- ✅ **Title / results presentation** (landed 2026-07-21). A centred transient
  card (`WORLD 1-1  MARY-O x3` on entry/death, `COURSE CLEAR {score}` on the
  flag) expressed as one declared HUD slot (`HudSlotSpec::centered()`) rather
  than the engine's app-only `GameplayBanner`. A game publishes text into the
  slot only while it should be up; an unpublished slot draws nothing, so there
  is no hide path and no despawn.
- ✅ **HUD for score/coins/time/lives** (landed 2026-07-21), through the
  provider-declared HUD seam (`with_hud`). `MaryOLevelState` carries `score`
  (banked from `FlagPhase::Tallied`) and `lives`; coins read the shared
  economy's wallet through `PlayerHudFacts`, the same fact Sanic's rings use.
- ✅ **Lives are spent** (landed 2026-07-21). A death costs one life. ⛔ Lives go
  NEGATIVE and play continues forever by Jon's decision (*"for now, let's allow
  lives to go negative ... so no game over screen yet"*) — there is no run
  restart. Mary-O authors no death test of her own: lives are charged off
  `ActorDiedMessage`, the engine's authoritative attempt-lost fact — not
  `BodyLifetime.resets`, which several unrelated callers (including a room
  replay's own reset) also bump, and which drained a life per frame on both a
  pit death and on winning the level before this fix. The death beat carries a
  `life_spent` flag so a pinned death animation is charged once however many
  frames report it, not once per frame. Regression:
  `a_replay_reset_is_not_a_death_so_lives_cannot_drain`; the beat's
  dwell/position/`life_spent` are rollback-registered. `DEATH_DWELL` is derived
  from the death sting's length (four bars of 2/4 at 150bpm) rather than picked
  by feel, so a re-scored cue moves it.
- ✅ **A TALLER FORM REFUSES A WEAKER PICKUP, and it is played rather than
  asserted** (landed 2026-09-03,
  `game/ambition_demo_mary_o_app/tests/weaker_form_refusal.rs`). A Mary-O
  already carrying the beacon walks the floor past a second wand and does NOT
  take it — `form_rank` orders small/wand/beacon, and both halves of the rule
  are covered: `refuse_a_weaker_form_pickup` is the floor half and
  `without_downgrading` the block half. ⭐ **The control arm is the point**: a
  SMALL Mary-O collects the very same wand in the very same placement, so the
  refusal is proven to be about her form and not about the item being
  unreachable. ⚠ The placement is `two_rooms`' own DROP with one retry from
  below the face — four earlier attempts to place the block by hand each failed
  a different way (lifting by 100 landed her on it; clamping ignored body
  height; "place at current height" triggered the guessed-resting-height
  respawn), which is why the test reuses the authored drop instead of computing
  a position.
- ✅ **The course-clear sting plays** (landed 2026-07-25). `flag::play_victory_music`
  claims the priority tier from the grab to the tally so the cue covers however
  long the sequence takes; slide + walk-off + `LEVEL_CYCLE_DWELL` comfortably
  exceed its ~3.1s, so it finishes before the level loops. Same claim/release
  seam as the death music.
- ✅ **Landing on a resting shell KICKS it** (landed 2026-07-25). A stomp on a
  `Boxed` shell launches it away from whichever side she is more on; a shell
  already *running* is still stopped by a stomp (the catch-the-runaway tech).
  `KICK_GRACE_S` is spent by a wall bounce as well as by its timer, since a
  shell that turns around and comes back is a hit however recently it was
  kicked.
- ◐ **The deterministic scripted SEAM run** (landed 2026-07-21, not the
  acceptance run) (`ambition_demo_mary_o_app/tests/scripted_level_run.rs`).
  Boots the real demo app, walks her through the real `ControlFrame` seam,
  takes the secret pipe, banks the vault's coins through the shared economy,
  surfaces, and finishes on the flag into a settled tally and level cycle. The
  clock is pinned with `TimeUpdateStrategy::ManualDuration` (a fixed-tick host
  without one runs a machine-speed-dependent tick count per update); it is
  gated `#![cfg(not(feature = "input"))]` because under `input` the participant
  pipeline owns `ControlFrame` and repopulates it from device state, a
  different claim `app_it::participant_input` already owns. Traversal between
  beats is set up, not played, so the script isn't fragile to jump-tuning
  changes — what it exists to prove is that the seams connect.

  ⚠ **The acceptance clause it does NOT meet.** The Acceptance section below
  requires the scripted run to use a powerup through the real pickup/equipment
  path; this run only takes coins (the economy path, not equipment), and its
  set-up beats mean nothing proves the level is traversable spawn-to-pole.
- ✅ **The real level-1 acceptance run** (landed 2026-07-21, `d92791435`,
  `ambition_demo_mary_o_app/tests/level_1_acceptance.rs`). One state-aware
  controller with no positional set-up: spawn → bonk ?-block 0 → mount and take
  the milk → cross pit A → secret pipe → vault → bank all 8 coins → return pipe
  → surface → re-power at ?-block 1 → cross pits B and C → stair pyramid → pole
  → tally → replay to spawn, finishing with all three lives (a run that spends
  lives has shown the level survivable, not traversable). Every clause asserts
  against the BODY rather than the emitter's bookkeeping.

  Three bugs fell out of writing it, none visible to any prior test because
  every prior proof set her position past the terrain or asserted a value the
  emitter wrote:
  - the vault had no working exit (the return pipe's block floated 48px clear
    of the vault floor) — FIXED (`cbc6902d2`), guarded by
    `the_pipe_leads_into_a_sealed_vault_and_back_out`;
  - `reset_body_clusters` hardcoded the default body size, so a grown Mary-O
    who fell in a pit came back small while still wearing the cap — FIXED
    (`4e4bd0fd8`);
  - pit B opened directly into the secret vault instead of being a pit — fixed
    when the level was lengthened for World 1-2 (below); guarded by
    `no_pit_drops_into_the_secret_vault` rather than trusted, since the bug is
    invisible until someone falls in.
- ✅ **World 1-2, and the reason it could not exist before** (2026-07-25). The
  demo's world was a single `RoomSpec` because `RoomTransitionRequested` had <!-- cite-ok: past tense — records the event type the demo used before the change -->
  exactly one consumer (`ambition_app`), which no demo depends on — in this
  binary the message went into a channel nothing drained, so a second room was
  UNREACHABLE, not merely unauthored. That is also why the secret vault had to
  be dug under the surface of the same room.

  The readiness transaction, one-shot authorization, and commit now live in
  `ambition_platformer2d_runtime::room_transition`, carried by
  `PlatformerEnginePlugins` into every host — the same move made earlier for
  `RoomReplayRequested`. The commit's room-drawing step now writes
  `RespawnRoomVisualsRequested` (the channel the sandbox reset and room stager
  already used) instead of naming `ambition_render` directly. A host keeps only
  what it can answer, marker-gated so absence is honest: the asset contributor
  is `Skipped` on a headless host with no asset server, rather than waiting
  forever.

  1-2 is the underground level the V1 design names: unbroken roof, a coin
  shelf, and a five-tile chasm with no stepping stone, so its one new verb — an
  authored moving-platform sweep — is load-bearing exactly once. Entry/exit are
  `Walk` zones rather than a third pipe, and the exit returns her past pit B so
  going under is a shortcut, not a replacement route. Proven by playing
  (`tests/two_rooms.rs`): she takes the real DOWN verb, walks the vault floor to
  the shaft, and the assertion is that the authoritative room changed and her
  body is inside the new room's bounds. Poison-tested; a separate proof rides
  the ferry, since a silent carry failure would make the chasm impassable while
  every other test still passed.

  ⚠ Fallout: the engine group now supplies `AmbitionLoadPlugin` (the transition
  IS a load plan) — both demo hosts and three test hosts that had their own copy
  were panicking on a duplicate; there is now one owner.
- ✔ **crossing while GROWN is COVERED** — verified 2026-08-20 against HEAD, not
  counted. `she_crosses_wearing_the_form_she_earned` (`two_rooms.rs`) earns the
  star wand out of 1-1's own authored `?`-block by a real head contact, crosses
  to 1-2, and asserts `WornEquipment` still carries it — reading the row off the
  BODY rather than the tall sheet, so it cannot pass on a body that arrived
  naked before the deriving system ran. Its own doc names itself "the last of
  the four the continuity row asks about". Green.
- ▢ still open on 1-2: further authored levels.
- ✔ **ANSWERED 2026-08-14: the level can give it to you, and nothing tells
  you.** `a_grown_mary_o_bonks_a_question_block_and_wears_the_fire_flower`
  (`level_1_acceptance.rs`, default feature set) is green — a grown Mary-O
  bonks a 1-1 ladder block and ends up wearing `spark_blossom`. This was never
  a missing reward; it is a DISCOVERABILITY problem. Two authored facts explain
  the whole report: the first `?`-block can never pay it (a small Mary-O is
  always paid the wand, so the beacon needs a second block — 1-1's second is
  past pit A, behind a warp pipe); and the beacon is `ItemMotionPlan::still()`,
  so it waits on top of its block rather than walking to you the way the wand
  does. ⚠ *"you cannot bonk it from underneath standing still"* is a
  jump-technique fact, not the bug — a standing jump raises her centre 144.8px
  against a 96px underside (comfortable for bonking) but cannot get her ON TOP
  of a `?`-block (128px face, 17px margin), which is why `mount` (written for a
  64px pipe rise) could never reach one.

  ⇒ What remains is a PRODUCT question for Jon, not engineering: should the
  beacon walk to you like the wand, should 1-1 place a reachable second block
  earlier, or is "the reward waits up there and you must climb for it" the intended
  feel? Original investigation record below.

  ⭐ **FILED AS DECISION 46 (2026-09-03)** in
  [`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md),
  which is where a question whose next step is Jon's authoring judgement
  belongs. It arrived from the other end — writing the floor half of the form
  ladder's acceptance test — and carries one fact this row does not have.

  ⛔ **1-1's THIRD ladder block, at x=1920, STANDS OVER A PIT.** Measured by
  dropping a body into each column: x=192 and x=960 both settle on floor at
  y=400 and bonk normally; x=1920 lands her ON the block from above (centre 256,
  feet on its top face) and, entered from below the face, drops her to y≈969 in
  a 448-tall room — out of the world, dead, respawned at the level start. So
  there is no standing position under it at all.

  ⇒ That sharpens the option "should 1-1 place a reachable second block
  earlier": the third block is not merely awkward to reach, it cannot be bonked
  from the ground by any body. ⚠ It does NOT contradict this row's account of
  the SECOND block — "past pit A, behind a warp pipe" is a route fact, and the
  route is not what was measured here (the probe teleports into the column). The
  new fact is about the third.

- ✔ **RETIRED 2026-08-20 — the superseded "no way to get the fire flower" record.**
  It claimed *"nothing in the codebase has ever bonked a ?-block while
  GROWN"*, which the ✔ row above it contradicts outright:
  `a_grown_mary_o_bonks_a_question_block_and_wears_the_fire_flower`
  (`game/ambition_demo_mary_o_app/tests/level_1_acceptance.rs:1326`) does
  exactly that and has been green since 2026-08-14. ⛔ an open marker its own
  neighbour answers is worse than no row: it reads as work and sends the next
  session to re-investigate a closed question.
## Consumes

- runtime, provider lifecycle, and windowed host composition;
- the shared body/control path using the axis-swept motion model;
- item/equipment and canonical action/moveset execution;
- combat/contact vocabulary for stomps and sliding hazards;
- world IR — her rooms and loading zones are constructed as `RoomSpec` values in
  Rust rather than authored in LDtk. That is acceptable **because she is a demo**;
  LDtk is still the preferred path and is required for the Ambition game itself
  (see `demos/README.md` and ADR 0009). Do not grow the programmatic path — a
  missing authoring concept goes into LDtk + the tooling;
- the cutscene domain for presentation sequencing where appropriate;
- `SimView` for HUD and programmatic observation.

## Owns

`ambition_demo_mary_o` owns its levels, rules, lives/score/coins/timer, equipment
rows, enemy/content rows, shell prop, flag sequence, HUD, title, and results. A
need discovered while authoring becomes engine work only when it is a reusable
missing seam.

## V1 design

- **World:** three levels sharing one authored world: an opening grammar, an underground variant, and a moving-platform level.
- **Powerups:** a grow/armor equipment row and a ranged-action grant. Numeric effects fold through equipment parameters; behavior grants compose through action data.
- **Enemies:** ordinary actor rows for walkers plus a brainless sliding shell prop; this exercises the actors-versus-props distinction.
- **Camera:** forward-only scroll is an authored camera policy, not Mary-O-specific engine code.
- **Flag:** provider-owned gameplay state captures contact height and drives the body deterministically; presentation/results may use the cutscene domain without turning cutscenes into encounter logic.
- **Death:** level restart is authored session/game policy rather than a universal engine default.

## Acceptance

A scripted headless run completes the first level, reaches the pipe secret, uses
a powerup through the real pickup/equipment path, and never touches the
surface-momentum implementation. The visible app uses the same provider and body
state, including size and equipment presentation.

The demo app remains an explicit composition root, not a second implementation of
input, session lifecycle, sprite binding, or platformer simulation.
