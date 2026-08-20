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
  through `regen_sprites.sh`. Lesson: a `WorldItemArt` id that names a missing
  texture fails SILENTLY — nothing errors, the item is simply never drawn;
- breakable bricks through durable block-contact identity;
- crony enemies and shared stomp behavior;
- forward-only authored camera policy; and
- flag contact-height scoring, slide, walk-off, tally, dwell, and cyclic level
  restart.

Remaining acceptance work
(**this list is the single source; status.md and tracks.md refer here**):

- ✅ **The secret pipe and underground room — LANDED 2026-07-21.** A warp pipe on
  the safe run between pits A and B; stand on its mouth, press Interact, drop
  into a sealed coin vault dug under the ground slab, and Interact at its far
  end to surface. The vault is part of the SAME `RoomSpec` rather than a second
  room on purpose: cross-room transition lives in `ambition_app`'s `world_flow`,
  so a room-graph secret would have worked only when Ambition hosted the demo
  and been dead in the demo's own app. The world grew downward
  (`SURFACE_HEIGHT` + `VAULT_DEPTH_TILES`) so the authored surface layout is
  byte-identical. The warp is a real `transit_body` (ADR 0024), not a position
  poke, so entering while wall-clinging reconciles instead of arriving still
  clung to a wall that is not there. Its 8 coins are ordinary `currency`
  placements the shared economy collects — they land in the HUD's COINS readout
  with no demo collection code. NOTE: authoring placements made this demo
  require the pickup lowering interpreter, so its two bare-`App` unit harnesses
  now add `WorldPrepSchedulePlugin`; the real app already had it.
- ✅ **A Solid Snake is the SHAPE its art is, per pose — 2026-07-25.** Jon:
  *"the bounding box of the solid snake should change when it is in its box
  (shell) … the sprite authoring should be the thing that gives how the collision
  and hurtbox should be placed relative to the visuals and vice versa."* It was
  a hand-guessed 28×32 rectangle for every phase of the withdraw cycle, and the
  art it stood for is a 117×49 sprawled serpent — so it was wrong even walking,
  and wildly wrong boxed.

  The sheet is now the authority. `solid_snake` publishes per-animation body
  rectangles (the generator's `animation_key_map` opt-in it simply never took),
  and the new engine seam `character_sprites::posed_body` turns the rectangle for
  the pose being SHOWN into three facts that cannot disagree: the collision box,
  the sprite quad, and the quad's placement. One authored scalar — world units
  per sheet pixel — is the whole input; everything else is read off the art.
  Walking she is 58×26, boxed 26×19, so a stomp genuinely makes her a smaller
  thing to kick and stand on. Resizes are feet-anchored (the +gravity face holds),
  so nothing is ever shoved out of geometry.

  The seam is opt-in per body (`SpritePosedBody`) and host-registered, so a
  content crate cannot add the component and silently get nothing. The pose it
  reads is the sim-side pin (`ActorAnimOverride`), never the render-side
  locomotion picker — a collision box that depends on whether anyone is watching
  is not a collision box.
- ✅ **The brainless sliding shell — LANDED 2026-07-21.** A stomped crony now
  leaves a shell instead of nothing. Walk into a resting shell and it launches
  away from the side you touched, so you aim it; walk into a sliding one and it
  stops dead, so it stays a tool rather than something you set loose and lose. A
  sliding shell runs cronies down and reverses at walls, which turns one stomp
  into the demo's one emergent combo. BRAINLESS is literal: the archetype's
  brain is `StandStill`, so nothing ever decides anything for a shell — its
  whole behaviour is three demo rules, and gravity, ground contact, and walls
  are the ordinary body physics every actor already gets. FIXED 2026-07-21 after
  playtest: it shipped inert. The demo matched actors on `Name`, but the spawner
  writes `"Feature actor enemy: {name}"` there and the bare name onto
  `FeatureName` — so shells were never tagged AND sliding shells matched no
  crony. Both now match `FeatureName`. The unit test was green throughout
  because its fixture hand-built `Name`; the regression test drives the real
  spawn path instead.
- ✅ **Title / results presentation — LANDED 2026-07-21.** A centred transient
  card: `WORLD 1-1  MARY-O x3` on entry and after every death, `COURSE CLEAR
  {score}` on the flag. Expressed as ONE declared HUD slot rather than a new
  surface — the engine's `GameplayBanner` renders only in `ambition_app`, so a
  demo could not use it. `HudSlotSpec::centered()` was the whole engine-side
  addition, and the card retires itself: a game publishes text into the slot only
  while it should be up, and an unpublished slot draws nothing, so there is no
  hide path and no despawn.
- ✅ **HUD for score/coins/time/lives — LANDED 2026-07-21** through the new
  provider-declared HUD seam (`with_hud`), four readouts in the reserved top
  surround the 4:3 profile already owed. `MaryOLevelState` grew `score` (banked
  from `FlagPhase::Tallied` when the level cycles, so it is a running total
  rather than the last banner) and `lives`; coins read the shared economy's
  wallet through `PlayerHudFacts`, the same fact Sanic's rings use.
- ✅ **Lives are spent — LANDED 2026-07-21.** A death costs one. ⛔ **CORRECTED
  2026-08-14: lives go NEGATIVE and play continues forever — there is no run
  restart and no game over.** This bullet said *"zero lives restarts the run (lives,
  score and clock all return to start)"*, which is the behaviour Jon's *"for now,
  let's allow lives to go negative and the user to play forever, so no game over
  screen yet"* explicitly ruled out; the same stale sentence was found in three
  places in the Rust source, including a test NAMED
  `..._and_zero_lives_restarts_the_run` whose body asserts `lives == -1`. Mary-O authors
  no death test: she watches `BodyLifetime.resets`, the counter the ENGINE bumps
  in `reset_body_clusters` on every respawn, so any future hazard that respawns
  her already costs a life with no new demo wiring. Running out of time is the
  demo's own rule and converges on the same path by asking the engine for a
  respawn instead of teleporting her. Poison-tested both ways: spending on the
  counter's VALUE instead of its EDGE drains a life per frame, and failing to
  refill the clock lets one timeout spend every remaining life on consecutive
  frames.

  ⚠ **CORRECTED 2026-07-21.** Both of those poison tests probed the EDGE
  DETECTION and neither probed the SIGNAL, which is where the bug was. Watching
  `BodyLifetime.resets` was itself wrong: six unrelated callers bump that
  counter, including a room replay's own body reset. So a death spent a life,
  requested a replay, and the replay's reset was read as a second death —
  unbounded, at frame rate. Grabbing the flag entered the same loop, because the
  level cycle also requests a replay: in the hosted app, WINNING drained the run.
  Lives now come from `ActorDiedMessage`, the engine's authoritative attempt-lost
  fact, which a replay does not publish — so the loop cannot form by construction
  rather than by guard. The engine gained `publish_kernel_reset_death` so that
  message finally covers the pit/drown/hazard death that never reaches the hit
  resolver. Regression: `a_replay_reset_is_not_a_death_so_lives_cannot_drain`.

  ⚠ **The drain came back through the other door — fixed 2026-07-25 (GPT-5.6
  review).** Once the death BEAT landed, she is pinned exactly where she fell for
  1.6s, which for a pit death means the hazard keeps reporting her. `begin`
  refused to restart a running beat, but `spend_lives_on_death` had no such guard
  and charged a life per frame of her own death animation. The beat now carries
  `life_spent`, so an attempt costs one life however many times it is reported —
  the same shape as the `replay_pending` debt beside it. Both, plus the beat's
  dwell and death position, are rollback-registered; none of them were, and the
  replay edge lived in a `Local<bool>` outside the envelope entirely.

  The dwell also did not actually suppress anything: every death system runs in
  `GameplayEffects`, a full phase after movement, collection, room-transition
  detection and damage, and the brain refilled the control frame it blanked
  before any of them next read it. She could walk, collect, and take a pipe while
  dead. That is now the engine's `ScriptedControl` seam, blanked immediately
  after the brains write, which the flagpole slide and Sanic's act-clear brake
  share.

  **Playtest 2026-07-25 (Jon):** the dwell was 1.6s against a 3.2s sting, so the
  death cue never resolved. `DEATH_DWELL` is now derived from the score — four
  bars of 2/4 at 150bpm — rather than picked by feel, and the constant says so,
  so a re-scored cue moves it.
- ✅ **The course-clear sting plays — LANDED 2026-07-25.** `mary_o_flag_victory`
  existed as a score and nothing asked for it. `flag::play_victory_music` claims
  the priority tier from the grab to the tally, so the cue covers whatever the
  sequence takes rather than a fixed window; slide + walk-off + `LEVEL_CYCLE_DWELL`
  comfortably exceed its ~3.1s, so it finishes before the level loops. Same
  claim/release seam as the death music, which is what stops the boss system's
  every-frame release from silencing it.
- ✅ **Landing on a resting shell KICKS it — LANDED 2026-07-25.** Jon: jumping on
  a shell parked under a block trapped her in an endless bounce, which the
  classic never does. A stomp on a `Boxed` shell now launches it away from
  whichever side she is more on (dead centre goes right, so it stays a pure
  function of the two poses). A shell already *running* is still stopped by a
  stomp — that is the catch-the-runaway tech, and it is why the kick is scoped to
  a resting shell.

  This reverses an earlier fix, deliberately: kicking on a top touch used to
  cause a kick-then-side-hit loop, and the fix then was "a stomp is never a
  kick". What makes the kick safe now is `KICK_GRACE_S`, which postdates it — and
  the grace is **spent by a wall bounce**, not only by its timer, because a shell
  that turns around and comes back is a hit however recently you kicked it.
- ◐ **The deterministic scripted SEAM run — LANDED 2026-07-21, but it is not the
  acceptance run** (`ambition_demo_mary_o_app/tests/scripted_level_run.rs`).
  Boots the real demo
  app, walks her through the real `ControlFrame` seam, takes the secret pipe,
  banks the vault's coins through the shared economy, surfaces, and finishes on
  the flag into a settled tally and a level cycle. Two things it had to learn:
  the clock is pinned with `TimeUpdateStrategy::ManualDuration`, because a
  fixed-tick host without one runs a machine-speed-dependent number of ticks per
  update and the same script then walks a different distance every run; and it
  is gated `#![cfg(not(feature = "input"))]`, because under `input` the
  participant pipeline legitimately OWNS `ControlFrame` and repopulates it from
  device state each frame — scripting the device layer is a different claim that
  `app_it::participant_input` already owns. Traversal between beats is set up
  rather than played: crossing the pits under scripted input would make this a
  platforming-precision test fragile to any jump tuning change, when what it
  exists to prove is that the SEAMS connect.

  ⚠ **The acceptance clause it does NOT meet.** The Acceptance section below
  requires the scripted run to use "a powerup through the real pickup/equipment
  path". This run never collects one; the only pickups it takes are coins, which
  go through the shared ECONOMY, not the equipment path. Its three set-up beats
  also mean nothing proves the level is traversable spawn-to-pole. The relocation
  at least no longer pokes `BodyKinematics` — it goes through `transit_body`
  (ADR 0024), so a beat cannot begin with stale attachment state.
- ✅ **The real level-1 acceptance run — LANDED 2026-07-21** (`d92791435`,
  `ambition_demo_mary_o_app/tests/level_1_acceptance.rs`). One state-aware
  controller, no positional set-up anywhere: spawn → bonk ?-block 0 → mount it
  and take the milk → cross pit A → climb the secret pipe → vault → bank all 8
  coins → climb the return pipe → surface → re-power at ?-block 1 → cross pits B
  and C → up the stair pyramid → the pole → tally → a real replay back to spawn.
  It finishes with all three lives, which is the point: a run that spends lives
  has shown the level is survivable, not traversable. **Nothing in the codebase
  previously proved any pit was crossable.**

  Every clause asserts against the BODY rather than the emitter's bookkeeping:
  the milk goes through the real ?-block → `WorldItem` → `collect_world_items`
  equipment path (30x48 → 30x72); the cap's effect is exercised by absorbing a
  hit that costs no health and no life; the ladder re-arms; the pole runs the
  flag sequence to a settled tally; the level replays her to spawn with a banked
  score and a fresh clock. That last clause could not have passed however it was
  written before 2026-07-21 — this binary drained `RoomReplayRequested` with
  nothing (tracks §2.5).

  **Three bugs fell out of writing it**, none of which any existing test could
  see, because every existing proof either set her position past the terrain or
  asserted a value the emitter wrote:
  - the vault had **no working exit** — the return pipe's block was derived from
    its interact band rather than the vault floor, floating it 48px clear so its
    top face sat above its own band. `the_pipe_leads_into_a_sealed_vault_and_back_out`
    stayed green by checking a body at the band's CENTRE, a point inside solid
    rock; `scripted_level_run` stayed green by teleporting her to exactly that
    unreachable point. FIXED (`cbc6902d2`);
  - a **body reset redefined the body**: `reset_body_clusters` hardcoded the
    default size into `base_size`, so a grown Mary-O who fell in a pit came back
    with a small collider while still wearing the cap. FIXED (`4e4bd0fd8`);
  - **pit B is not a pit** — it opened directly into the secret vault. **NOT
    LIVE at HEAD, verified 2026-07-25**: lengthening the level to fit the wide
    vault under unbroken ground moved pit B clear of it, and nobody noticed the
    report was answered. Now pinned by `no_pit_drops_into_the_secret_vault`
    rather than trusted, since the bug is invisible until someone falls in.
    [`../../archive/planning-superseded/2026-08-13/triage-room-replay-followups-2026-07-21.md`](../../archive/planning-superseded/2026-08-13/triage-room-replay-followups-2026-07-21.md) §5.
- ✅ **World 1-2, and the reason it could not exist — 2026-07-25.** The demo's
  world was one `RoomSpec` because it had to be: `RoomTransitionRequested` had
  exactly one consumer and only `ambition_app` registered it, and no demo depends
  on `ambition_app`. In this binary the message went into a registered channel
  that nothing drained, so a second room was not unauthored — it was
  UNREACHABLE. That is also why the secret vault had to be dug under the surface
  in the same room.

  The readiness transaction, the one-shot authorization, and the commit now live
  in `ambition_platformer2d_runtime::room_transition`, carried by `PlatformerEnginePlugins`
  into every host — the same move §2.5 made for `RoomReplayRequested`, stuck for
  the same kind of reason: not a dependency, ONE call. The commit DREW the new
  room (`spawn_room_visuals`), which named `ambition_render`. It now writes
  `RespawnRoomVisualsRequested`, the channel the sandbox reset and the room
  stager already used.

  A host keeps only what a host can answer, marker-gated so absence is honest:
  the asset contributor (`RoomTransitionAssetContributor` — "did the destination
  room's art arrive", which needs a sprite catalog and an asset server, so a
  headless host is `Skipped` rather than waiting forever) and the existing cover
  gate. The neighbor prefetch split the same way: a prepared
  `RoomConstructionPlan` is an engine artifact keyed by engine identity, so
  promoting one is engine business; the asset manifest stays with the host.

  1-2 itself is the underground level the V1 design names: unbroken roof, a coin
  shelf, and a five-tile chasm with NO stepping stone, so its one new verb — an
  authored moving-platform sweep — is load-bearing exactly once, the rule 1-1's
  own stepping stone follows. Entry and exit are `Walk` zones rather than a third
  pipe (the vault's pipes answer a directional press; an open shaft is a
  different affordance, not a competing one), and the exit returns her past pit B
  so going under is a SHORTCUT and the two routes compete.

  Proven by playing, not by bookkeeping (`tests/two_rooms.rs`): she takes the
  pipe with the real DOWN verb, then WALKS the vault floor to the shaft with no
  placement after that, and the assertion is that the AUTHORITATIVE room changed
  and her body is inside the new room's bounds. Poison-tested. A separate proof
  rides the ferry, because if carrying silently failed the chasm would be
  impassable and every other test would still pass.

  ⚠ Fallout: the engine group now supplies `AmbitionLoadPlugin` (the transition
  IS a load plan), so both demo hosts and three test hosts that added their own
  copy were panicking on a duplicate. One owner now.
- ▢ still open on 1-2: crossing while GROWN (the continuity proof covers coins,
  lives and score), and further authored levels.
- ✔ **ANSWERED 2026-08-14: THE LEVEL CAN GIVE IT TO YOU, AND NOTHING TELLS YOU.**
  `a_grown_mary_o_bonks_a_question_block_and_wears_the_fire_flower`
  (`level_1_acceptance.rs`, DEFAULT feature set) is GREEN — a grown Mary-O bonks a
  1-1 ladder block and ends up wearing `spark_blossom`. ⭐ **so this was never a
  missing reward; it is a DISCOVERABILITY problem**, and two authored facts explain
  the whole report. **The first `?`-block you meet can never pay it** — a small
  Mary-O is always paid the wand, so the beacon needs a SECOND block, and 1-1's
  second is past pit A and behind a warp pipe. And **the beacon is
  `ItemMotionPlan::still()`**, so it waits on top of its block rather than walking
  to you the way the wand does. ⚠ *"you cannot bonk it from underneath standing
  still"* is a jump-TECHNIQUE fact and not the bug: a standing jump raises her
  centre 144.8px against a 96px underside, so bonking is comfortable — what a
  standing jump cannot do is get her ON TOP (128px face, 17px margin, ~0.25s hang,
  and `air_coast_decel: 0` crossing only ~12px horizontally). That is why `mount`,
  written for a 64px pipe rise, could never reach a `?`-block.

  ⇒ **what remains is a PRODUCT question for Jon, not engineering**: should the
  beacon walk to you like the wand, should 1-1 place a reachable second block
  earlier, or is "the reward waits up there and you must climb for it" the intended
  feel? Original investigation record below.

- ✔ **RETIRED 2026-08-20 — the superseded "no way to get the fire flower" record.**
  It claimed *"nothing in the codebase has ever bonked a ?-block while
  GROWN"*, which the ✔ row above it contradicts outright:
  `a_grown_mary_o_bonks_a_question_block_and_wears_the_fire_flower`
  (`game/ambition_demo_mary_o_app/tests/level_1_acceptance.rs:1486`) does
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
