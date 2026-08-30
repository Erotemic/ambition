# The queue — standing execution ledger

**This file is the SPINE and the ledger `scripts/goal_guard.py` reads.** It is
intentionally self-replenishing. The literal open marker `▢` appears in this
header as well as on executable rows, so the guard never interprets an empty
snapshot of the queue as permission to stop.

Its name carries no date on purpose. This ledger outlives any one run: it was
`queue-72h-2026-08-08.md` until 2026-08-15, and a dated name on a file whose
whole property is that it never closes is how the guard's pointers went stale
before. A run rotates; the ledger does not. ⛔ If this file is ever renamed or
archived anyway, repoint `.goal/active.json` in the SAME commit — its checks
name this path, and a check whose subject vanished is the one failure that
looks like success.

> **Finish work, then promote the next highest-value verified item and keep
> going. There is no "the queue is empty, therefore stop" state.**

⛔⛔ **THIS LEDGER HAS TWO SHAPES, AND THE USUAL GREP ONLY SEES ONE.** Open work
lives BOTH as `- ▢ **D…` bullet rows AND as rows of the STAFFING TABLE below.
Measured 2026-08-28:

```text
grep -n '^- ▢ \*\*D' docs/planning/queue.md      7 rows   ← the usual command
rows of the table that are not ✔ / ☑              25 rows   ← invisible to it
```

⛔⛔ **AND THE TABLE IS SYSTEMATICALLY STALE BECAUSE OF IT — swept 2026-08-28,
and FIVE of the six rows checked were wrong:**

```text
D205  marked unstaffed          → already BUILT, both halves shipping
      ⛔ and BUILT ≠ RIGHT: 2026-08-28 the shipped aim read the DAMPED stick,
      fell back to FACING and sampled at the transit, so the Author's up-B
      fired him sideways. The aim WINDOW is the fix; see the row.
D195  marked unstaffed          → already BUILT (the strike pulse)
D196  marked unstaffed          → already BUILT (the special carries its intent)
D197  marked unstaffed          → already BUILT (the disposition is restored)
D198  marked unstaffed          → already BUILT (key and target are separate)
D250  marked unstaffed          → already BUILT (the shark authors a route)
D251  marked HALF-addressed     → both halves built
D188  marked unstaffed          → already FIXED (the scale is the kit's slowest)
D192  marked STAFFED / re-do     → already BUILT, as the representation it asked for
D183  marked lane work           → its two deliverables landed
D186  marked unstaffed          → REAL, and eleven fields wide rather than one
D246  marked BLOCKED            → its blocker landed two days earlier
D199  marked unstaffed          → STILL REAL. One of its three defects fixed
                                  2026-08-28; the other two are its restructure
```

⇒ somebody has been FIXING these and nobody has been marking them, because
nobody's open-work list contains them. ⭐ the one row that was still real is the
proof the sweep was worth doing rather than a formality.

⇒ **a session that regenerates its list with that grep alone reads a fifth of
the ledger.** Five of the twenty-five are Jon's own asks from 2026-08-25/26
(D203, D205, D246, D247, D248), and **D205 was marked `unstaffed — MECHANICS +
CONTENT` while both halves of it were already shipping** — nobody re-checked it
because nobody's list contained it.

⭐ **THE WHOLE POPULATION IS BOTH OF THESE:**

```sh
grep -n '^- ▢ \*\*D' docs/planning/queue.md      # the bullet rows
grep -n '^| \*\*D'   docs/planning/queue.md | grep -v '✔\|☑'   # the table
```

Document roles:

- **this queue** owns current execution order;
- [`tracks.md`](tracks.md) is the standing reservoir;
- focused plans own technical design and acceptance criteria;
- [`JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`](JONS_OBSERVATIONS_BUGS_AND_ISSUES.md)
  owns direct maintainer observations;
- [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) owns
  questions that genuinely require a product/feel decision;
- completed campaigns and migration evidence belong in `docs/archive`.

Before starting a row, inspect HEAD and confirm the named gap still exists. When
a row closes, remove its historical case file from this ledger, preserve useful
history in the archive if needed, promote another verified item, and continue.

⛔⛔ **A CLOSED ROW IS A RECEIPT, NOT A CASE FILE — and this file learned that
the expensive way.** On 2026-08-17 **2,584 of its 7,025 lines were closed rows**,
each carrying the full investigation that justified a fix nobody was going to
revisit. The form a closed row takes, and the whole of it:

> `✔ **D123 — what was wrong, in one sentence.** Fixed by `<commit>`: what the
> fix was. Guarded by `<test>`. ⛔ <a standing prohibition, only if one exists>.`

⭐ **the evidence lives in the commit message and in this file's own git
history** — `git log -p docs/planning/queue.md` recovers every word of it. ⚠ the
same rule applies INSIDE an open row: keep the current model at the top and
delete the layers it supersedes, because a stale `⇒ NEXT` sentence under a
correction is exactly how a later session re-does landed work. The narrow
exception is a sentence that would otherwise be **rediscovered at cost** — a
prohibition, an instructively wrong measurement, a design refused for cause —
and that is one clause, never a section. Full rule in
[`README.md`](README.md#queue-contract).

⭐⭐⭐ **JON ANSWERED EVERY OPEN MAINTAINER QUESTION — 2026-08-22. There are
ZERO open decisions.** All fifteen are recorded verbatim in
[`maintainer-decisions.md`](maintainer-decisions.md), with receipts in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).
⛔ **do not re-ask any of these, and do not infer around them.** The ones that
unblock a ledger row:

| § | ruling | unblocks |
| --- | --- | --- |
| 26 | **Full rename**: `World.edges: WorldEdgeMargins { fall, side, rise }`, Rust + LDtk keys in ONE change; no content migration (zero levels author a value). ⛔ `BlockKind` not in scope | D169 |
| 19 | **Sheet registry keys by FILE ROOT.** A renderer target string may not be a durable engine identity | D162 |
| 31 | **`SeatRawFrames` stays RAW.** Split is SOURCE-LOCAL vs WORLD-DEPENDENT — portal transforms go AFTER the boundary, beside fast-fall. ⛔ post-boundary table is NOT "confirmed" (GGRS predicts): `CanonicalSeatInput` / `TickSeatInput` | D175 · D180 |
| 30 | **DELETE the 1.0 height warn; do not replace it with a median.** Density becomes a separately DECLARED authoring profile | D165 |
| 29 | **Sweep the crates a CARVE touches**, not the workspace. ⛔ never `--workspace --tests` | D33 |
| 16 | **The layout tool owns a level's position, and ownership FOLLOWS THE LAYOUT MODE.** ⛔ do not bulk-rewrite the 52 specs | D163 residue |
| 18 | **A hit's art follows BOTH** the victim's material and the blow's strength — a ~10-emitter message change | D128 residue |
| 15 | **Impact hitstop is a bounded MATCH-LEVEL request** expiring on UNSCALED time; no owner, no hand-back path | D72 |
| 17 | **`DebugLabel` is debug and keeps shipping** — the world is scaffold. **Proximity-gate EdgeExit labels**, and label visibility is a three-valued GAME-SELECTABLE policy | D161 residue |
| 20 | **Boss hoards are per-boss eventually; CURRENCY for the demo.** ⛔ do not invent eight item definitions | D125 residue |
| 5 | **Correct the level-1 CPU for feel.** ⛔ its original evidence is RETRACTED — `0.84%` was 84% | fighter-brain |
| 14 | Mary-O: **keep 56 px**, **raise the short crown ~6 px**, **fix the walk dip with a torso pose field**. ⛔ do not zero the three numbers | D129 |
| 3 | **Disable rust-analyzer.** ⛔ no second target directory | — |
| 4 | The Mary-O restart report **was Mary-O and is believed resolved** — retired | D70 |
| 2 | Advance the measurement submodule pointer **every so often**; cadence does not matter | — |

⛔⛔ **AND ONE RULING GENERALISES BEYOND ITS ROW.** §17, Jon verbatim: *"we are
nowhere close to a real exploration game. This is part of the scaffold and
prototype, and if a GPT review claimed otherwise, then it was out of line."*
⇒ weigh a polish finding against the PROJECT'S STAGE before filing it.

D73 is closed and its working-memory documents are archived under
[`../archive/planning-superseded/2026-08-13/`](../archive/planning-superseded/2026-08-13/).
The successor strategy is
[`engine/engine-1.0-architecture-program.md`](engine/engine-1.0-architecture-program.md).
Do not reopen deleted character/archetype authority merely because archived
migration prose names it.

---

## Current execution order

### ⭐ CROSS-GAME SESSION CONTAMINATION, 2026-08-30 — a retired game's health was the next game's

- ✔ **D-SESSION-OWNERSHIP — Smash → title → Ambition and no door would open,
  because rollback health had no owner.** Retiring Smash's scope removed the
  canonical `SessionRoot` its still-installed GGRS session was policing;
  `enforce_session_contract` read deliberate teardown as *"canonical prepared
  content disappeared while the GGRS session was active"* and invalidated the
  timeline; and `RollbackSessionStatus` / `RollbackConfirmationState` were bare
  process globals, so Ambition read Smash's invalidation as its own and
  room-transition commit refused every transition forever.
  **Reproduced first**, both halves, before any implementation.

  Fixed by giving rollback authority an owner. `ActiveRollbackAuthority`
  (owner + generation + contract + status, one resource) states the whole rule
  in `installed()`: same `SessionScopeId` ⇒ the diagnosis carries (AC23);
  different ⇒ a fresh authority. Every read names a scope, so a stranger's
  authority answers `Unavailable`, never `Unhealthy`. Gameplay reads through
  `SessionRollbackConfirmation`, which cannot reach the state without naming the
  live scope. Diagnostics split into process-lifetime `RollbackDiagnosticHistory`,
  which gates nothing. Model and guards: **ADR 0027**.

  ⛔⛔ **THE ORDERING WAS A RACE, AND THE SHIPPED ORDER IS THE LUCKY ONE.**
  Nothing orders `LocalSessionSet::Maintain` against `GameplaySessionSet::Bridge`
  — both sit in `Update`, constrained only against unrelated sets. Today the
  bridge wins and `maintain_local_session` stops the session before the contract
  check sees anything, which is why no existing test caught this. Pinning the
  other admissible order reproduces it exactly. `SessionScopeSet::RetireAuthority`
  now names the ordering, but **ordering is hygiene**: the acceptance walk runs
  in BOTH orders and the ownership check is what holds in the adverse one.

  ⛔⛔ **AND `session_world_entity` PANICKED ON TWO ROOTS RATHER THAN PICKING THE
  LIVE ONE** — the opposite of what ownership is for, and already recorded once
  in `app/resources.rs` against a build-time root. A shell-routed host now
  SELECTS by active scope; uniqueness stays the authority only for direct entry.

  ⭐ **THE AUDIT'S RESULT IS ONE MESSAGE, NOT ELEVEN ACCESSORS.** The eleven
  session mirrors in `session::teardown` were reset only at retirement, and nine
  change the next session's behaviour if that reset is delayed or skipped — a
  dangling possessed-body handle, a `specs_loaded` latch that suppresses the next
  session's repopulation, a room-transition cooldown that refuses doors, a
  buffered interact nobody pressed. They are re-established on
  `SessionScopeActivated` in `SessionScopeSet::Activate`, before any provider
  builds the world that reads them: ownership without a check at any of the
  several hundred read sites, because **the value a session reads is one its own
  activation wrote.** Retirement reset stays, documented as hygiene.

  Guards, every one verified red first: the acceptance walk in both orderings
  (`shell_host_lifecycle::a_smash_session_does_not_take_ambitions_doors_*`), the
  whole multi-game walk under `SimulationHost::Rollback`, seven arms in
  `session_ownership_tests`, and
  `teardown::tests::activating_a_session_clears_what_a_skipped_teardown_left_behind`.

- ▢ **D-SESSION-OWNERSHIP residue — one dev knob still crosses a session, and
  `ambition_workspace_policy` has been red since 2026-08-26.**
  * `LocalSessionPolicy::check_distance` is raised for an F9 proof pulse and
    dropped back only when the pulse COMPLETES (`finish_completed_proof_pulse`).
    Quit to the title mid-pulse and the next game starts with rollback
    verification still raised. Left alone deliberately: it is developer tuning
    with one writer, and a pulse a human may want to span a relaunch is not
    obviously session-scoped. Decide before it becomes a "why is this build
    slow" report.
  * The policy crate aborted on `engine.actor-physics-facade` (watching a file
    6e39ac0a5 deliberately deleted) and on the archetype-free scanner (whose
    `ActorConfig` moved to `ambition_combat` in aa70a41e3). Both repaired here
    because they were masking the report — which then showed **12 pre-existing
    engine-policy failures** across `actor-orientation-facade`,
    `actor-platformer-facade-reexports`, `module-size`,
    `pose-writes-are-authority-only`, `runtime-applies-clock-reset`,
    `runtime-manifest-allow`, `velocity-writes-are-authority-only`. None touch
    the files above; all are residue from the recent carve commits and want
    their owners' judgement, not a sweep.

### ⭐ THE OFFICER'S DRAW, 2026-08-30 — the shot was never aimed wrong

- ✔ **D-OFFICER-1 — "the officer is still firing backwards" was TWO
  presentation defects, and the simulation was innocent throughout.**
  `officer_probe` had already established the aim, the timing (`fb9230363`'s
  0.348s), the body flip and `authored_faces_left` as correct. What was wrong:
  * **no `Discharge`**, so the shot defaulted to `Muzzle::BodyOrigin` — whose
    spawn offset is `origin + (0, -8)`, PURELY VERTICAL. The round was born at
    his sternum while the gun and its muzzle flare are drawn out at his hand,
    which is precisely the case `Muzzle::Hand`'s own doc says it exists for:
    *"so the shot leaves the barrel the player can see rather than the
    fighter's midriff."*
  * **no `visual`**, so an empty id resolved to `ProjectileArt::generic()` — the
    engine's orange-red QUAD. A pistol fired a debug placeholder.
  Fixed by authoring `Muzzle::Hand { ahead: 10.0 }` and
  `PISTOL_ROUND_VISUAL` on his `RangedActionSpec`, plus a new
  `pistol_round` projectile target and its catalog registration. Guarded by
  `the_officers_round_leaves_his_hand_and_carries_his_own_art`.
  **Verified in both directions** (`officer_probe left|right`, 2026-08-30):
  born at **+18.6 / -18.6 AHEAD of him**, `vel.x` +560/-560 agreeing with
  facing, 71 round-ticks each.

  ⛔⛔ **THE HANDOFF'S "+9.3 (AHEAD)" WAS NOT A MUZZLE OFFSET AND IT CLEARED THE
  REAL BUG.** The probe samples the first tick a round is VISIBLE, one tick
  after it spawns: 560 px/s ÷ 60 Hz = **9.33**. It was reporting one tick of
  TRAVEL as the spawn offset, so `BodyOrigin` — horizontally ON him — read as
  "ahead of him, as it should be". The probe now backs the travel out and judges
  where the round was BORN.

  ⛔ **AND THE GENERIC QUAD IS WHY THE FLIP LOOKED INNOCENT.**
  `projectile_visuals.rs` mirrors a `FlipToTravel` round with
  `flip_x = vel.x < 0.0`, which assumes art drawn pointing +x. The previous
  handoff named that as the prime suspect. It could not have been: a SOLID
  COLOUR QUAD IS SYMMETRIC, so the flip was a no-op that could be neither right
  nor wrong on screen. Real directional art is what makes the axis testable —
  `targets/projectiles/pistol_round.py` states its +x authoring, which is this
  category's standing convention.

  ⚠ **THE PROBE WAS ALSO BLIND IN ONE DIRECTION AND SAID SO BACKWARDS.** Once
  the muzzle moved to the hand, the `right` run printed *"NO ROUND EVER
  SPAWNED"* while the sim clock printed `impact_hitstop`: both seats are
  Officers standing close and the probe HOLDS the stick, so firing right walked
  into the sparring partner and the round landed on the same tick it spawned.
  A point-blank hit is a hit; a probe whose verdict depends on where the other
  fighter is standing is a broken instrument. It now stands the partner behind
  the shooter.


### ⭐ RESTART HANDOFF, 2026-08-30 — Quit to Title is FIXED and UNPUSHED, blocked on disk

Jon restarted the session here. `672bf257f` is committed and NOT pushed. Two
things remain, both cheap, and the FIRST one is a decision only Jon makes.

- ✔ **D-OFFICER-2 — the DRAW ANIMATION fired over his shoulder, and the fix
  had been sitting unrendered for three days.** Jon, 2026-08-30: *"The officer
  is still animating his gun firing backwards. The projectile fires the right
  way, the animation is the problem."*
  His sheet is drawn WEST and the whole sprite mirrors on `authored_faces_left`,
  so a per-clip disagreement cannot be expressed there — the body faced west
  while the gun arm, muzzle flash and beam went EAST.
  ⭐ **THE RIG WAS ALREADY CORRECT.** Solving the clip through the renderer's own
  `doc.solve` puts the gun hand at **-33 (west)** on the fire frames 6–8, the
  same side as `jab` (-30) and `attack_side` (-46). `f21eac9` fixed it on
  2026-08-27. What was wrong was the PUBLISHED ART, and the frame geometry says
  so without any eyeballing: in the stale sheet the flash frames kept
  `off.x ≈ 55` and grew to x=322 — EASTWARD; a fresh render keeps the right edge
  and drops `off.x` to 4 — WESTWARD. Republished with `--force`, then all four
  quality tiers, then verified in-engine at `AMBITION_QUALITY_PROFILE=medium`.
  ⛔⛔ **ROOT CAUSE: THE SPRITE CACHE KEY DID NOT COVER `data/`.**
  `unit_key = hash(CORE_SHARED : leaf_hash)`; CORE_SHARED hashed only `.py`, and
  `leaf_hash` only the target's `.py`/`.rig.json`. Nothing hashed
  `data/characters/**` or `data/motion/**` — so rewriting
  `officer_brawler_v1/clips/shoot.clip.json` did not move the key, the Aug-28
  publish hit the cache, and the corrected clip never reached a pixel.
  ⛔ **AND D-BUILD-1'S OUTPUT DIGESTS COULD NOT SEE IT.** `sheet_cache_fresh`
  asks whether the files on disk are the ones this cache PRODUCED — they were.
  Output hashing catches a SUBSTITUTED sheet; it cannot catch a faithfully
  cached sheet whose SOURCE has since changed. That is what a key is for, and
  the two guards answer different questions.
  Fixed in `leaf_hash`: it now hashes `data/characters/<name>/**` and follows
  that character's `motion_library` edge to hash the bound library. Verified by
  perturbing one `at_s` in `shoot.clip.json` — the key changes and returns
  byte-exact on restore.
  ⭐ Resolved PER-CHARACTER on purpose: folding `data/` into CORE_SHARED would
  make one clip edit invalidate every sheet in the roster, the same mistake the
  CORE_SHARED note already refuses for `sprites.sh` itself.
  ⚠ **A `--target X` PUBLISH IS NOT PROOF THE GAME CHANGED.** Generated art is
  gitignored, so this produced ZERO git diff; and Medium/Low load
  `sprites_0_5x/…`, which still held the old art until the tiers were
  regenerated. Republish, then regen tiers, then look at the tier the machine
  actually runs.

- ✔ **D-QTT-1 — `cargo test --workspace --lib` had never been run to
  completion, and the first complete run FAILED.** The disk blocker is gone
  (165.1 GB free against the 40 GB floor) and `672bf257f` is on `origin/main`.
  **Green 2026-08-30: exit 0, 50 crates, 3695 passed, 0 failed.**
  ⭐⭐ **WHAT IT CAUGHT WAS A MISSING ASSET WEARING A CODE BUG'S CLOTHES.**
  `ambition_render`'s
  `a_left_drawn_character_faces_the_way_they_are_going_like_a_right_drawn_one`
  panicked on `goblin_cave_dagger`, its canonical RIGHT-drawn sheet:
  `record_for_sheet_key` returned `None` because NO ROSTER LINE EVER PUBLISHED
  IT. The whole goblin weapon family was absent — the roster's own comment said
  *"install the rest"* and the weapon variants were the rest. Fixed by rostering
  and publishing them; the class is now caught by
  `scripts/check_published_sheets_are_present.py` (see D-RASTER-5).
  ⛔ GENERATED ART IS GITIGNORED, so "it works on my checkout" is the EXPECTED
  symptom of this class, not a surprising one: whoever rendered the target by
  hand has it and nobody else does. That is why it belongs in the bootstrap and
  not in a person's memory.
  ⚠ **AND KILLING A BUILD IS NOT FREE.** A `pkill -9` mid-archive left `syn`
  with an `.rmeta` and no `.rlib`; cargo's pipelining then started dependents
  that failed at LINK with *"can't find crate for `proc_macro2`"* — a fake
  error naming crates that were fine. Let `rustc` drain instead.


- ✔ **D-QTT-2 — the full `app_it` binary had never been swept.** It was
  OOM-killed twice at the default 6-way parallelism; this box has 15 GB and no
  swap and these are Bevy apps. Swept 2026-08-30 at `--test-threads=2` with
  ~13 GB still free: **508 passed, 0 failed, 21 ignored** in 689s.
  ⭐ **THE SWEEP EARNED ITS KEEP ON THE FIRST RUN.**
  `declared_art_resolves::every_declared_projectile_image_names_a_file_that_exists`
  failed: `ambition_content` declares the GNU-ton's apple as
  `sprites/gnu_ton_boss/gnu_ton_apple.png` and that file was ABSENT, so the
  boss's apple rain drew `ProjectileArt::generic()` — an orange-red quad — for
  every player. ⛔ Nothing crashes and nothing logs when that happens: `resolve`
  falls back silently. Same shape as D-QTT-1's `goblin_cave_dagger`; both are
  now rostered, published, and covered by
  `scripts/check_published_sheets_are_present.py`.
  ⛔ Do NOT poll for this run with `pgrep -f "test app_it"` — the polling
  command matches ITSELF and reports RUNNING forever after the binary has died.
  Match `app_it-` or read the log's last line.


⛔ **AND ONE THING THIS SESSION BROKE:** it deleted two untracked files from the
repo root, `errors.txt` and `tree-out`, that predated it. They were not its own
scratch and it did not look at them first. Unrecoverable — they were untracked.
If either mattered, regenerate it; nobody has a copy.

**What landed (`672bf257f`), so the next session does not re-investigate it:**
Jon's *"Smash 'quit to title' quits to a DIFFERENT GAME — often Ambition
itself"* was NOT a routing bug — `QuitToHome` was measured in isolation and does
one clean hop to `ambition_launcher`. Leaving Smash retracts its
`BindingLayout`, so `rebuild_maps_from_recipes` rewrites the seat's `InputMap`
on the same frame the shell routes home, and its `reset_all()` leaves the action
`Released` while the key is still physically down — leafwing's next read sees
`Released -> down` and calls it a rising edge. The launcher had just activated
with its cursor reset to row 0, so the phantom confirm launched row 0: Ambition.
Fixed by `ambition_input::Rebound` + `swallow_the_rebinds_own_edges`. The
receipt, including why Jon's second quit always worked, is in
[`JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`](JONS_OBSERVATIONS_BUGS_AND_ISSUES.md).

⛔⛔ **THE STANDING LESSON, which is bigger than this bug: A ONE-FRAME TAP PASSES
AGAINST IT.** The quit and the landing both happen while the key is still down;
the phantom arrives after. Every existing menu test tapped, so the whole suite
was blind to a defect a player hits every single time. **An input test that does
not HOLD is not testing input.** Same shape as D205's aim window and the select
screen's "a held stick roams; a tap still snaps" — this repo keeps finding
defects in the span between an edge and a hold.


### ⭐⭐ WEAK-HARDWARE FRAME, 2026-08-29. Jon: *"It should not be required."*

*"I want to make sure the game runs smoothly without a GPU if we don't have one.
It should not be required."* Measured on `calculex` (Intel HD 630, no discrete
GPU): **p50 51.0ms → 18.5ms, ~19.6 FPS → ~54**, with the first 100s
vsync-capped at 60. Full reasoning in
[`engine/performance-and-iteration.md`](engine/performance-and-iteration.md) —
the LAPTOP section, which is a different question from the desktop-hitch section
below it and on different silicon.

- ✔ **D-RASTER-1 — the quality budget traded away scene detail and had no knob
  for anything that scales with SCREEN AREA**, so a 1600x900 window rasterised
  at 3200x1800 with 4x MSAA. Fixed by `d99152290` / `7e4ec65c4`: `RasterBudget`
  (`max_scale_factor`, `msaa_samples`) per tier, settable independently of the
  tier, plus `ambition.local.toml` for per-machine declaration. Guarded by
  `high_and_ultra_raster_budgets_are_todays_behaviour`. ⛔ `max_scale_factor` is
  a CAP and must never RAISE a scale factor, or a tier meant to make things
  cheaper tells a 1x laptop to rasterise at 2x.

- ◐ **D-RASTER-2 — every 2D draw goes through the TRANSPARENT pass, so nothing
  can be depth-rejected.** ~5.3x overdraw (41,482,624 fragments against a
  7,818,240-pixel framebuffer) with `main_opaque_pass_2d` at **zero**. The
  largest remaining lever on weak hardware and the first that is engine work
  rather than a setting.
  ⛔⛔ **THE PREREQUISITE THIS ROW NAMED COULD NOT BE MET BY THE FILE IT NAMED.**
  It said to confirm from `draw_census.csv` WHICH entities produce the
  fragments. That file's columns were `sprites`, `sprites_visible`, `text2d`,
  `per_view_projections` — ALL COUNTS, no screen area — so it could not attribute
  overdraw at all. Across every recorded bundle peak `sprites_visible` is **76**,
  and 76 sprites cannot make 41M fragments unless some cover the viewport.
  Counting them again more precisely would never have said which.
  **Instrument landed 2026-08-30:** `report_draw_census` now also emits
  `sprite_area`, `sprite_area_max` and `sprite_area_unsized`; the census CSV
  writer derives its columns from the emitted keys, so `draw_census.csv` grows
  them with no parser change.
  ⚠ World units, not pixels — converting needs each sprite's view and
  projection, a per-view question this per-world pass has no business answering.
  The `sprite_area_max` / `sprite_area` RATIO is what identifies a few
  viewport-sized panels hiding among many small sprites, and a ratio does not
  care about the unit.
  ⚠ `sprite_area_unsized` counts visible sprites with no `custom_size`, whose
  extent comes from their image and cannot be resolved without the asset store.
  They are EXCLUDED, not guessed at zero, so a row with a large `unsized` has
  area columns that are a FLOOR.
  ▢ **REMAINING: capture a bundle on `calculex` and read the ratio**, then do
  the engine work. The standing hypothesis, from `per_view_projections=6` and
  `sync_parallax_layers` sizing each panel to its view's own rectangle, is that
  the fragments are the PARALLAX PANELS multiplied across views rather than
  character sprites — but that is an inference until the ratio is read, and this
  VM cannot capture it (see D-RASTER-3: no GPU, only lavapipe).


- ▢ **D-BG-1 — Jon, 2026-08-30: *"backgrounds aren't rendering"* — NOT
  reproducible after the asset regeneration, and the cause is therefore
  UNCONFIRMED.**
  Checked and found healthy: parallax art exists at all four tiers (36 layer
  PNGs each) with real pixels — `sky` is the only fully OPAQUE layer (alpha
  255; `far_backplate` peaks at 61 and `near_background` at 108), Medium's
  budget is `enabled: true, max_layers: Some(3)`, and `RUNTIME_PARALLAX_LAYERS`
  is ordered Sky→Far→Near→Foreground so `take(3)` drops only the front
  atmosphere and KEEPS the opaque sky. Every `(theme, layer)` id is registered
  in the manifest with an `Image` kind (guarded by
  `sandbox_image_manifest_registers_every_entity_and_parallax_entry`).
  **Rendered at Jon's own profile** (`AMBITION_QUALITY_PROFILE=medium`, via
  `moveset_render` on lavapipe): *"loaded 4/4 generated background/parallax
  layers for 'hub'"*, and the frames draw moon, stars, structures and water.
  ⚠ **THE HONEST GAP: the game was never run BEFORE the regeneration**, so
  whether the 184 stale tier files and the missing rostered sheets were the
  cause cannot now be established from this session. The prime suspect is that
  class (D-RASTER-5), since the tiers for weak hardware are exactly the ones
  that were stale. If it recurs, capture a frame BEFORE regenerating anything.
  ⛔ And note the check that would have settled it in one line:
  `spawn_parallax_layers` returns early when `assets.parallax_layers.is_empty()`
  — the log line above (`loaded N/4`) is the thing to read first.

- ▢ **D-REVIEW-STATUS — the GPT review findings have ONE status file now, and
  it is not a dated one.** ⛔⛔ The 2026-08-30 re-review found a live P0 that had
  fallen out of every list: gravity authority/ordering was open in the 08-29
  review, was not in the 08-30 file's carry-forward paragraph, and was still real
  in the source. Nobody dropped it — it stopped appearing anywhere a session
  would regenerate its list from, which is exactly the failure this ledger's own
  header warns about one register up. Measured 2026-08-30: four rows listed OPEN
  in the 08-29 file had LANDED, six were carried forward, and **three were named
  by neither carry list** (gravity, the trapdoor `UntilPressedAgain` hold, the
  quadratic `InputStreamRecorder`).
  ⭐ [`triage/review-findings-status.md`](triage/review-findings-status.md) is
  now the authoritative ledger; the dated `gpt-review-*` files are EVIDENCE and
  are deliberately not rewritten when a row later lands. This queue row is the
  spine's handle on it and closes when that file has no `▢` left.

- ▢ **D-RASTER-3 — nothing splits D-RASTER-1's 2.76x between the DPI cap and
  MSAA.** Both moved together. One interleaved A/B, three reps per arm; the
  knobs are already independent (`AMBITION_MAX_SCALE_FACTOR`, `AMBITION_MSAA` —
  both confirmed live at `quality.rs`) and no rebuild is needed between arms.
  ⛔⛔ **THIS ONE CANNOT BE RUN FROM `aivm-2404`, AND THAT IS NOT A SETUP
  PROBLEM.** The whole measurement is what a real rasteriser costs per pixel.
  The VM has no GPU at all — only lavapipe, a CPU rasteriser — so a DPI-cap arm
  measured there reports the speed of a software renderer that the shipped game
  never uses, and the MSAA arm is close to meaningless. `calculex` (Intel HD 630
  since the discrete card died) is the only host that can answer this, and it is
  the host the 2.76x was measured on.
  ⚠ Which also means: do not "unblock" this by running it here. A number from
  the wrong rasteriser is worse than no number, because it will be quoted later.

- ✔ **D-RASTER-4 — nothing picked a quality tier from the hardware.**
  `default_visual_quality_profile()` decides by target OS, so every desktop
  booted High — including one whose renderer is an Intel HD 630, which measured
  p50 51.0ms (~19.6 FPS) there. The OS was never the thing that made it slow.
  **The policy, in `ambition_persistence`:** `DetectedGpuClass` (a
  `wgpu::DeviceType` mirror that keeps `wgpu` OUT of persistence),
  `seed_profile_for_gpu` — Discrete→High, Integrated/Virtual→Medium, Cpu→
  **Potato**, Other→the existing default — and
  `VisualQualitySettings::seed_from_hardware`.
  **The seam, in `ambition_render`:** `seed_visual_quality_from_adapter` reads
  `RenderAdapterInfo` (Bevy inserts it in the MAIN world) on `PostStartup` —
  AFTER `load_settings_at_startup`, ahead of the `Update` pair that resolves the
  budget.
  ⛔⛔ **IT SHIPPED ON `PreStartup`, WHICH MIGRATED NOBODY.** The loader runs in
  `Startup` and REPLACES the whole `UserSettings` resource, so an existing file
  overwrote the seeded tier and restored its own `hardware_seeded: false`; the
  system is startup-only, so it never ran again. A fresh install (no file, the
  loader returns early) migrated fine, which is why every unit test and a
  first-run play-through looked correct. Reported by a GPT review 2026-08-30,
  fixed the same day. See `seed_schedule_tests` in `quality.rs`: five arms on a
  real booted App, including one that reads the file back off disk.
  ⛔ **A DETECTED DEFAULT IS A FIRST-RUN SEED, NEVER A PER-BOOT OVERRIDE.**
  Re-deciding each launch would silently undo the settings menu. TWO guards, both
  in the policy: a persisted `hardware_seeded` flag, and the profile still being
  the untouched default. The seam only names the adapter class.
  ⭐ **THE DECISION IS TESTABLE WITHOUT A GPU**, which is the only way it is
  testable on the machines it matters for — `seed_from_hardware` is pure.
  Guarded by `the_seed_tier_follows_the_adapter_and_not_the_operating_system`,
  `the_hardware_seed_fires_once_and_respects_a_chosen_tier` and
  `a_stored_profile_survives_a_reload_and_is_never_re_seeded`.
  ⚠ `Cpu` seeds Potato rather than Low: a software rasteriser is not a weak GPU,
  it is NO GPU, and its fill cost is paid by the cores running the sim.
  ⚠ Every system param is `Option`: a headless composition has no adapter, and a
  `Res` that matches nothing is a VALIDATION PANIC rather than a skip.


- ✔ **D-BUILD-1 — the sprite cache verified that an output EXISTED, not that it
  was the one it produced**, so `player_robot_v3_spritesheet.png` shipped
  byte-identical to v2's and every later run skipped it as fresh. Fixed by
  `efd253c49`: `sheet_cache_store` records the sha256 of everything a unit
  published and `sheet_cache_fresh` re-hashes and compares, so a substitution is
  self-healing on the next run. The toolchain joined the fingerprint too —
  `resvg-py` present or absent changes what the same source renders to.
  ⛔ Inputs were already content-addressed; only the outputs were on trust. A
  build that hashes what it reads and not what it wrote cannot tell "I made
  this" from "something is there".

- ✔ **D-BUILD-3 — a full sprite publish overwrote `player_robot_v3` with the
  `robot` rig's art; a single-target publish did not.** This is why the wrong
  robot kept coming back after being fixed. `SheetRecord`'s own doc had already
  named the failure: keyed by `target` (the RIG — 18 sheets share `robot`),
  *"give me sheet X"* is answered by whichever manifest loaded last.
  Fixed in the renderer submodule by `5ebe390` ("Two renderers were writing one
  sheet, and the last one to run won"): `validate_adapter_config_dirs` refuses
  any config that publishes as a module target's stem while rendering with a
  different generator — the ONE SHEET, ONE RENDERER invariant, which
  deliberately still allows `sandbag` (config and module share a generator, so
  both routes draw the same art).
  **Verified 2026-08-30:** the validator passes all 15 configs; each stem maps
  to exactly one entry in the discovery registry; `player_robot_v3` is a module
  target whose `render()` passes `target=TARGET_NAME`, so it stamps its own
  identity.
  ⚠ **THE ON-DISK STATE WAS STILL HALF-WRONG AND THE PIXELS HID IT.** v3's PNG
  was already the correct 3072x2468 art while its `.ron` still read
  `target: "robot"` / *"Auto-emitted from robot_spritesheet.yaml"* — a stale
  manifest left by the old collision, under the right picture. Since the
  registry is keyed by that field, v3 still collided with the real robot sheet
  while LOOKING fixed. Repaired by republishing the target.
  ⛔ A CORRECT IMAGE IS NOT A CORRECT SHEET. When this class recurs, compare the
  manifest's `target:` line, not the art.


- ✔ **D-BUILD-2 — the tool venvs lived inside the shared checkout and pointed
  at one user's Python**, so on a virtiofs checkout the venv was real for one
  user and interpreter-less for everyone else, and tool resolution fell back
  silently to a bare `python3` that cannot rasterise SVG. That is what published
  v2's art as v3. Fixed by relocating the store: `scripts/lib/tool_python.sh`
  resolves `$XDG_CACHE_HOME/ambition-tool-venvs/<tool>` BEFORE any in-repo
  `.venv`, keyed the way `target/` already was. Verified 2026-08-30 —
  `sprites.sh --check-toolchain` on the VM reports `resvg_py: present` and names
  the in-repo `.venv` as another user's, ignored.
  ⛔ `.venv/` was the same class of object as `target/` — machine-specific state
  in a shared tree — and had the opposite treatment. The generalisation is that
  ANY per-machine state living under a shared checkout is a single-user
  checkout wearing a disguise.


- ✔ **D-RASTER-5 — nothing in the bootstrap generated missing art.**
  998 sheets ~11 days stale on `calculex` and the parallax tiers absent, so
  Low/Medium drew old art or none — the tiers for weak hardware were the most
  likely to be broken, on the machines least able to notice.
  `run_developer_setup.sh` now runs `scripts/regen/assets.sh` (whose default
  categories already include `variants`), then `regenerate_missing_published_sheets`,
  then `verify_generated_content_is_current`. Verified 2026-08-30: the freshness
  checker reports *"quality tiers are current"* and all 163 rostered targets have
  published art.
  ⭐⭐ **FRESHNESS AND PRESENCE ARE DIFFERENT FAILURES AND ONLY ONE HAD AN
  INSTRUMENT.** `check_quality_variants_are_fresh.py` asks whether published art
  is OLDER than its source, which says nothing about art never published at all
  — A FILE THAT DOES NOT EXIST CANNOT BE STALE. New
  `scripts/check_published_sheets_are_present.py` closes that half, and the
  bootstrap GENERATES what it finds missing rather than only reporting, because
  the whole lesson of this row is that a checker nothing acts on is a checker
  nobody runs.
  ⛔ It asks each target what it DECLARES it installs
  (`Target.claimed_install_names`) instead of guessing `<target>_spritesheet.ron`
  — that guess is wrong for every subdir- or sub-name-installing target
  (`town_tileset`, `interdimensional_gate`, `gnu_ton_boss`) and a checker that
  cries wolf teaches people to skip the step.


## What the validator reported on 2026-08-1x — STALE, see above

```text
30 error:  lines   ALL false positives — 4 cross-world LoadingZone targets that
                   a single-file validator cannot resolve, and 26 that exist
                   only if you validate the raw map_assets copy instead of the
                   canonical symlinked path (the entity manifest sits beside
                   the symlink)
spawn_overlap      FALSE POSITIVE for mounts — it does not know a rider sits on
                   its mount, and it fires on every one
missing_level_wall GENRE-DEPENDENT — fires on mary_o_1_1 and sanic_speedway
                   where a bottomless pit is the design
```

▢ **the two that survived scrutiny:**
1. ✔ **`portal_lab`'s bottom edge was a FALSE POSITIVE.** `missing_level_wall` probes only the outermost cell row, while portal_lab's full-width floor sits five rows above the boundary. Fixed by asking whether a floor blocks a fall wherever it is. ⚠ only the BOTTOM gets this — the same idea on left/right is much noisier (46 open sides vs 6), because a corridor's side wall legitimately has a doorway gap.
2. **`SurfaceRamp` has no editor definition**, so a supported engine entity cannot be placed by an author. ⛔⛔ that reasoning covers only `SurfaceRamp` — the `defs.entities` warning names EIGHT entities, and the other seven (`GravityZone`, `GroundItem`, `Portal`, `PortalGunSpawn`, `ShrineSpawn`, `SurfaceChain`, `SurfaceLoop`) are placed and defined in four of six worlds but missing from `intro.ldtk` and `you_have_to_cut_the_rope.ldtk` — an author in the flagship world could not place a `Portal` used next door in `sandbox`.
   ✔ **RECONCILED 2026-08-18.** The seven were copied into `intro` and `you_have_to_cut_the_rope` via `def upsert-entity` from `sandbox`'s spec. All six worlds now carry 33+ defs and differ only in `SurfaceRamp`; every world validates with 0 errors.
   ⚠ **`SurfaceRamp` stays out on purpose** — 0 placements in any world, a deliberate product call.

✔ **`spawn_overlap` KNOWS ABOUT MOUNTS, 2026-08-18.** A pair joined by `mounted_on` is exempt, since position-identical is what a mount is and only the fields distinguish a relationship from a duplicate. Measured: sandbox 5→0, intro 3→0. Two unrelated spawns at one pixel still warn.

⛔⛔ **`level diff-specs` was reading nothing — found 2026-08-18.** It loaded only `.yaml`, and every area spec is `.ron`, so it reported success by finding nothing to check. ✔ fixed: the loader reads RON through the tool's own `ron_parse`, and `--all` globs `.ron`/`.json` too. ⚠ turning it on reveals real drift (52 of 54 specs differ) and it is NOT wired into CI yet.

▢ **the question underneath it is who OWNS a level's position** — the spec, or `world auto-layout` which arranges levels by the LoadingZone graph. The tool's own message says "live LDtk wins," suggesting the specs stopped being authoritative and nobody re-recorded them. ⛔ do not bulk-rewrite the 52 specs to silence it before that is answered. (`sanic_sandbox`'s off-grid Y is NOT drift — it's authored in `specs/sanic_sandbox_area.ron` and matches the live level.)

✔ **AND THE 30 ERRORS ARE 0, 2026-08-18 — RESOLVED, NOT SUPPRESSED.** Both causes were the validator being handed less than the runtime has: 4 cross-world targets (secondary worlds now default to sibling worlds beside the file, `--no-sibling-worlds` opts out) and 26 unknown entities (`mary_o.entities.json` sits beside the symlink, not the real file — the sidecar search now checks both).
⛔⛔ **the default belongs in the LIBRARY, not a CLI parser** — it was in the parser first and `repair` walked past it, so `entity set-field` still failed on errors it was meant to clear. Every entry point (`validate`, `repair`, `repair_and_validate`) now reaches one line.

- ✔ **D162 — REOPENED 2026-08-18 BECAUSE A REPORTER HAD NEVER RUN; CLOSED AGAIN
  2026-08-24 with every line of its boot inventory carrying a verdict.** (was
  first closed 2026-08-17, four standing boot warnings triaged)

⇒ **what closed it:** the sheet half landed under §19's file-root keying (848
keys, zero shadowed, guarded against the real table); the quasar warning and the
loading-zone warning are fixed and poison-verified; the redundant schedule edge
is a stated won't-fix; the 38-neighbour warning was argued and left; and
`npc_kernel_guide` turned out to be absent BY DESIGN.

⛔ **the two residues are not this row's, and neither is a defect.** The
`sanic_sandbox` off-grid origin points at D163, which is CLOSED and whose own note
records the origin as AUTHORED (`world_y: 3000`, spec and world agree) — a
pointer at a closed row reads as open work, which is what this ledger keeps
paying for. And the Kernel Guide's `CharacterDefinition` is D56's content call,
waiting on Jon, with nothing broken while it waits.

⛔⛔ **"no character id collides" was measured from a silence that meant "I did
not run."** `report_shadowed_character_sheets` is a `Startup` system;
`init_sheet_registry` is ALSO `Startup`, and Startup is unordered, so it ran
with `Res<SheetRegistry>` absent and printed nothing on every route — while the
registry logged 39 shadowed targets in the same boot. ⇒ moved to `PostStartup`.
⚠ "the catalog knows this name" was also the wrong filter: of the 39,
`robot`(15)/`goblin`(8)/`sandbag`(1) ARE catalog ids but they are shared RIG
adapters, not collisions — that filter would have reported 24 legitimate rig
shares as defects.

Asked against `ShadowedTarget::loser_image` instead, **three survive and are
real:**

```text
robot     robot_spritesheet.png      256x256  LOSES to robot_archivist        230x256
goblin    goblin_spritesheet.png     239x253  LOSES to goblin_brute_hammer    232x256
sandbag   sandbag_spritesheet.png    128x128  LOSES to sandbag_armored_review 256x256
```

⛔⛔ **the harm is not demonstrated the obvious way.** The character geometry
road (`record_for_target` → `record_index()`) keys by FILENAME ROOT and cannot
collide. What collides is the target-keyed `SheetRegistry` RESOURCE, whose four
consumers (`bosses/sync.rs`, `slash_visuals.rs`, `shrine_visuals.rs`,
`projectile_visuals.rs`) do not appear to resolve those three names — so three
real collisions exist in a registry whose current readers don't hit them.

⭐⭐ **the ambiguity is structural — measured across all 196 baked sheets:** 52
sheets have a `file_root` differing from their `target` (authored against a rig
adapter); 5 targets are each claimed by more than one file, 48 files between
them (`robot` 18 · `toon` 16 · `goblin` 9 · `sandbag` 3 · `ninja` 2). The
target-keyed registry cannot answer "give me sheet X" for any of those 48 —
which wins is load order. `record_index()` already keys by file root (196
unique keys) via `from_baked_table_by_file_root`.

✔✔ **THE SHEET HALF IS CLOSED (2026-08-24), and the answer was (b), which
dissolved (a).** Jon ruled §19 for FILE-ROOT keying on 2026-08-22; with it a rig
target is not a key at all, so `robot` and `robot_archivist` never competed and
there was never a stale manifest to retire — they are three distinct products
that happened to share a rig adapter. MEASURED at HEAD across the shipped baked
table: **848 keys, ZERO shadowed**, down from 39. Guarded by
`no_shipped_sheet_key_is_claimed_twice`, which asserts it against the real table
rather than a fixture.

⛔ and it can still go red, which is why it is asserted: the two spellings share
one namespace, so a packed atlas whose member is named for an existing file root
collides. Exactly one packed product exists today (`creator_lab_props`, 8
members, no collision), which is why the `product::member` spelling is NOT built
— see D200 §8e, where the record's lookup `key` was separated from its authored
rig `target` so that spelling has somewhere to live when a second one arrives.

✔ **a fifth boot warning found while measuring the above, fixed 2026-08-18.**
`mary_o::quasar: overlay not attached` printed on the FIRST frame of an
ordinary texture decode instead of only once it PERSISTED. ⇒ counted per
candidate, reported once past `QUASAR_ATTACH_GRACE_FRAMES` (60). Poison-verified:
unmodified a 150-frame boot now prints zero warnings; with attachment made
impossible it prints exactly one.

✔ **loading zone "did not fire" — CLOSED (`ad82531b7`).** WARN only on a
PRESSED door, DEBUG otherwise; verified 1→0 warnings, door still transitions
under `--press f`.
⛔⛔ **the first attempt at this fix was wrong in a way worth keeping**: it
filtered the message away whenever no press was buffered, which would have
silenced the instrument in the exact scenario that justified it — a broken
binding is exactly the case where the player pressed and `wants_interact`
still reads false. ⇒ a value a diagnostic REPORTS is a bad value to gate it on.

⭐⭐ **the flagship boot inventory, measured 2026-08-18 on `--route
ambition_gameplay` — every line now has a verdict:**

```text
sanic_sandbox off-grid Y     ✔ AUTHORED, not a defect — D163 closed and says so
GgrsSchedule redundant edge  ✔ WON'T-FIX — both memberships individually correct
SheetRegistry robot/goblin/  ✔ CORRECT and newly VISIBLE — the three above; keying is §19
  sandbag
room has 38 neighbours       ✔ LEAVE IT — warn_once!, names its constant, and its author
                                argued the case: "a cap that quietly drops work reads as
                                everything is prefetched"
npc_kernel_guide             ⊙ absent BY DESIGN — see below; the residue is D56's
```

⊙ **`NpcSpawn-0017` names `npc_kernel_guide`, and NOT registering it is the
design** — this row assumed the opposite. `register_declared_cast` iterates
`buildable_cast()` (the playable roster plus the build-only list) and its own
comment says why an exploration NPC is excluded: *"A bare registration for an
exploration NPC would incorrectly replace its archetype-authored body."* The
Kernel Guide is a hub NPC, so it is absent by intent, not by omission.

⇒ and it is not running on somebody else's kit. Its catalog row states its
spritesheet and manifest, `sprite_tuning`, `default_brain: patrol_peaceful`,
`default_action_set: peaceful`, its barks and its hall dialogue id; the intro
road resolves its sheet by id. That is the same road every hub NPC takes.

✔ **D56 ANSWERED AND BUILT 2026-08-24.** Jon: *"Kernel Guide gets its own
`CharacterDefinition`. Character identity is not sprite identity... Do not invent
a combat kit or capabilities merely to fill the definition."* The sheet-borrow
worry does not arise — a borrowed presentation is the normal mechanism and says
nothing about who the character IS. `authored/npc_kernel_guide.rs` states its
walk and its four health and NOTHING about its body or its verbs, which is what
leaves the archetype road in charge of both; guarded against Alice as the peer
that already migrated and the vault keeper as the one that has not.

- ✔ **D161 — CLOSED 2026-08-18. No loading zone prints an authoring id any
  more: 130 → 0, and a per-world ratchet in CI keeps it there.** (opened
  2026-08-17, found by CAPTURE of `intro_wake_room` — the flagship's opening
  room)

`spawn_loading_zone` in `crates/ambition_render/src/rendering/world.rs` renders
`zone.name` unconditionally for non-Door zones (Door zones get a
proximity-gated nameplate instead), and a zone's name is a level-authoring
identifier, not prose. Measured: of 151 named loading zones, 130 (86%) were
snake_case ids, 19 of those not gated by Door and so always shown.
⛔ CORRECTED 2026-08-17 — first published doubled (302/260/38) by counting
`game/ambition_content/assets/worlds/` and `game/ambition_map_assets/*/worlds/`
as separate, when they're the same worlds.

✔✔ **AUTHORED 2026-08-18 — 130 → 0.** Every loading zone carries
player-readable prose, each following its own level's authoring convention;
destinations came from the zone's own `target_room`, never from prettifying
the id (`wake_to_raid` has no good rendering that way).
⛔⛔ dismissing the bulk as "a developer sandbox where a diagnostic id is
defensible" was WRONG — `sandbox.ldtk`'s `central_hub_complex` is the world
manifest's `entry_room`, so 17 of the ids were in the game's first room.

✔ **the ratchet is `scripts/check_zone_name_ratchet.py`** (baseline
`dev/zone_name_ratchet_baseline.json`, now an empty map), per world, fails if
it observes no zones at all. Runs in CI. ⛔⛔ dedupes by real path — every
world under `game/*/assets/worlds/` is a symlink into
`game/ambition_map_assets/`, the same doubling trap as above. ⚠ `None` ≠ `{}`
in its baseline loader — an empty map is the goal state, not "never recorded."

⚠ the room's `→ corridor` label that looked correct in the same frame is a
`DebugLabel`, not a zone name — so the opening room also has duplicated
signage. ▢ whether a non-Door zone should draw an unconditional world label at
all (24 named zones are `EdgeExit` and always draw) is asked as §17 in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md), alongside:
12 rooms carry both authored signage and always-on zone labels, and
`gate_stack_lower` has fourteen `DebugLabel`s doing player-facing work.

⚠ a separate genuine dev-note leak, found in the same sweep: `central_hub_main`
carried a sign whose text described the LDtk authoring artifact rather than the
fiction — of 134 sign texts, only this one failed that test (42 others read as
the game's intentional `//`-prefixed lab-AI house style).
✔✔ **REPLACED 2026-08-17 BY MAINTAINER RULING.** Jon, verbatim: *"replace it,
don't delete it. The hub benefits from an orientation sign; only the
authoring-language content is wrong."* Landed via `ambition_ldtk_tools entity
set-field` (not a direct JSON edit, which he named as part of the decision);
EntityRef counts checked unchanged before/after.

- ✔ **D157 — CLOSED 2026-08-16. Mary-O had her whole smash moveset in her own
  platformer: `combat_actions` derived attack slots from the MOVESET and
  `ActionSet` and never read `AbilitySet`, so `abilities: Some([RunJump])`
  bought nothing — twenty-three distinct swings reachable.** Fixed:
  `combat_actions` takes the `AbilitySet` and ceilings the melee family with
  `abilities.attack`. ⚠ Projectile is deliberately NOT under it — a ranged
  verb is always an explicit grant.
  ⛔⛔ a test caught this and was argued away: `…peaceful_kit…` asserted
  `moveset_len == 0`, went red at 17, and was rewritten to agree with the 17
  on an intent nothing implemented. Guarded by `mary_o_at_home_can_only_run_and_jump`,
  `the_run_button_throws_a_spark_only_while_she_wears_the_lantern`,
  `the_demo_body_cannot_trigger_a_single_move_from_its_own_smash_table`.

- ✔ **D156 — CLOSED 2026-08-16. The Patent Clerk faced backwards: facing was
  authored three times and nothing in Rust read any of them** —
  `gravity_aware_flip_x` was `facing < 0.0` with no per-character term,
  assuming all ~800 baked sheets face +x.
  ⛔⛔ it was a FORK, not a missing feature — `animate_bosses` already XORed
  this term. Landed `SheetRecord::authored_faces_left`, lifted
  `data-rig-facing` onto `CharacterSpec` (`8c30de613`/`37ac258b6`/`fd4320071`;
  renderer `fac948b`→`9b445c5`).
  ⚠ `SpritePackCatalog::to_sheet_record` synthesizes from atlas rects and
  cannot know facing, so the pack now inherits the base manifest's. ⚠
  `facing: str = "west"` is the DEFAULT, so "the rig says west" can mean
  nobody set it — guarded by `every_baked_sheet_is_drawn_pointing_where_its_body_faces`,
  pinned to exactly eight declaring manifests.
  ▢ still open, small: the portrait tier declares no facing and was never
  checked; two pre-existing rig-validator failures at HEAD (Carl's
  paint-slice order, Noether's `head_base`/`head_features` naming) — only
  `validate` is red, `build` is fine.

- ✔ **D158 — CLOSED 2026-08-17, then SUBSUMED BY D159. Two taunting CPUs
  printed through each other because `stack_offset` was measured from each
  speaker's own head, and floor-to-air is one stack step** — the ordinary
  geometry for a platform fighter, not a corner. Its bespoke stacking
  mechanism is DELETED: a bubble is now a `WorldLabel` in the one ranked
  placement pass (D159).
  ⛔ do not reintroduce a second system that places bubbles — two placement
  passes that cannot see each other is how this bug happened, and each could
  truthfully report "no overlaps found."

- ✔ **D160 — CLOSED 2026-08-17 BY MAINTAINER RULING: the omission is
  deliberate. The cheap unit tier is a REQUIRED PRE-PUSH check, not a
  per-turn gate — two tiers on purpose.** Jon: *"keep the per-turn gate small
  … The workspace lib suite remains a required pre-push/finalization check …
  'Gate' should continue to mean an executable gate."* Three tiers:
  `scripts/gate_suite.py` per turn (stays cheap); `cargo test --workspace
  --lib` pre-push (required is not gated); feature-gated suites when
  touching the subsystem.
  ⛔⛔ **DO NOT ADD `--workspace --lib` TO `gate_suite.py`.**
  ⛔⛔ this row was once closed on a false premise — it claimed the sweep was
  "added to the stated gate," but what landed was a paragraph in
  `AGENTS.md`. Two commands run in one turn is not one command invoking the
  other; "in the gate" means a line in `gate_suite.py` or a CI job and
  nothing else. ⭐ when you add a check, name the TIER that runs it. ⚠ the
  omission had hidden two suites red on `main`, repaired in `ea5ca88df`.

- ✔ **D159 — CLOSED 2026-08-17. A name plate printed through a taunt because
  a speech bubble was a FOURTH label family that never joined the one
  placement pass.** `WorldLabelFamily` becomes `Signage · Fixture · Actor ·
  Speech`, ranked LAST by the module's own test (which family can move
  without anything visibly jumping?). D158's mechanism, its constants and
  `PendingSpeechBubble` go with it.
  ⭐ two defects fell out of the same shape: a 160-unit point radius gated
  stacking for text rendering ~336 wide, and displacement advanced in a
  fixed 11px quantum so a budget of "six steps" bought two lines of
  clearance — both replaced by one `max_displacement_px`. Guarded by
  `a_name_plate_and_a_speech_bubble_do_not_print_through_each_other` and
  `the_bubble_yields_to_the_name_plate_and_not_the_other_way_round`.

- ✔ **D155 — CLOSED 2026-08-16. Nobody got launched — two bugs on the shared
  floor, not the parameter tweak it looked like.** (1) every authored launch
  direction in the game was VERTICALLY INVERTED: `HitVolume::launch_dir`
  states `+y = gravity-down`, ~100 authored literals wrote against it, and
  `knockback_velocity` negated `y` to satisfy the opposite doc comment —
  every up-tilt/up-air/up-smash spiked victims into the floor. (2) a launch
  big enough to TUMBLE was resolved as a LANDING on the tick it was applied:
  the launched body kept its stale resting contact into the same step's
  `tick_knockdown`, read `on_ground == true`, and zeroed velocity — a 3269
  px/s launch moved zero pixels. Fixed by clearing `ground.on_ground` when
  `launch_into_tumble` returns true, gated on the tumble answer so
  `tumble_speed: 0.0` bodies (all of Ambition) are byte-identical.
  ⚠ every floor-game test set `on_ground = false` before launching, so the
  actual in-match situation — standing on the stage — was never stepped.
  Guards in `hit_response::launch_direction_tests`,
  `movement::tests::combat_actions`, `smash_in_the_host::launched`, each seen
  red pre-fix. Both fixes are in `ambition_platformer2d_core`, so Ambition
  gets juggling from the same floor.

- ✔ **D147–D154 — ALL EIGHT REVIEW FINDINGS CLOSED 2026-08-17.** (external
  structural review, 2026-08-16, read against `2381e3a7e`.) 6 of 6 reproduced
  when probed; D151 leaves one named residual, recorded on its own row below.

⚠⚠ **provenance, and it is the durable half of this row.** The reviewer states
they "couldn't independently run Cargo in this review environment" and
treated the commits' reported green suites as evidence rather than rerunning
them. ⇒ every finding was a READING, not a measurement, and each was probed
before being fixed — a finding that cannot be made to fail is a finding about
the reader, not the code.

- ✔ **D147 — CLOSED 2026-08-17 (`797aa480d`). Generic match activation knew
  the stocks ruleset's private latch** — D140's fix had inserted
  `StocksMatchSettled(false)` inside the GENERIC activation road, installing
  the resource even where that ruleset was never composed.
  ⭐ PROBED FIRST: the coupling was LOAD-BEARING (comment it out and match
  two ends with zero winners), so this REPLACED it. The latch now carries
  the `MatchInstance` it is about; a new match reads as undecided by
  construction. Guarded by `the_previous_matchs_verdict_does_not_settle_this_one`,
  `a_verdict_from_another_session_does_not_settle_this_match`,
  `adopting_a_seat_topology_does_not_un_decide_the_match`.

- ✔ **D148 — CLOSED 2026-08-17 (`f0da10217`). A team victory announced the
  last surviving teammate instead of the team** — the card decided which
  side's name to swap by COUNTING BODIES standing, but an eliminated fighter
  is despawned, so a team that lost a member early has one body at victory.
  Fixed using `PreparedMatch::seats_on_side` (how many a side HAS) rather
  than how many are standing. Guarded by
  `a_team_victory_names_the_team_and_not_its_last_survivor`.
  ⚠ the guard was rewritten once: the first version let CPUs fight freely and
  a hitlag change in another crate flipped the winner — a claim about a
  card's WORDING must not depend on combat tuning, so every elimination is
  now caused by the test on a fixed schedule.

- ✔ **D149 — CLOSED 2026-08-17. Move VFX bypassed `FxRequest`, so fourteen
  movesets hand-paired every sound** — `dispatch_move_events` wrote
  `VfxMessage::Effect` directly around the one abstraction that pairs an
  effect's visual with its sound. Of 145 authored `sfx(…)` calls, 74 merely
  restated the default pairing (deleted, verified against
  `effect_cue(FxId::new(effect))`, zero differed), 21 were `.loop` overrides
  (kept — ten would have gone silent otherwise), 50 were independent voices
  (untouched). Guarded by `a_paired_burst_is_heard_exactly_once`, which runs
  the real `dispatch_move_events` + `process_fx_requests`.
  ⛔⛔ the half-landing in between was a live feel bug: one commit switched
  the arm and left the 74 restatements standing, so every burst played its
  sound TWICE for one session — 412 app tests stayed green throughout.
  ⚠ measured residue, deliberately left: two moves throw the same effect at
  `±x` on one frame and are heard twice (a burst-count question, not a
  restatement).

- ✔ **D150 — CLOSED 2026-08-18 (re-opened and re-closed same day). A
  projectile changed allegiance when its firer despawned** — allegiance was
  reconstructed every tick from the firing `Entity`, so a fighter fires,
  loses their last stock, the body despawns, and next tick the shot turns on
  its own team.
  ⭐ the shot's PRESENTATION half already had the answer
  (`inherit_projectile_presentation_sources`: "the bolt routinely outlives
  the body that fired it, so the source is STAMPED at spawn"). Same
  treatment for allegiance: `ProjectileAllegiance { faction, team }`, frozen
  on the first tick of flight, registered rollback state. ⚠ the grudge is
  NOT frozen (a feud is something the firer holds now); the faction stamped
  is the authored one.

  **Audit of every attack-authorization still recomputed from the resident
  firer**, prompted by the review: faction+team now stamped; the self-hit
  guard is fine; KO credit (`attacker: owner_entity`) is open, D148's
  neighbor; the grudge read stays live and is correct
  (`dissolve_settled_grudges` already ends a feud on a health rule, not
  residency).
  ⛔⛔ found live in the audit: `indiscriminate` was `allegiance.is_none()`
  while the comment beside it said "a bolt that never had a living owner" —
  different sentences. A named firer that vanished before the stamp landed
  was promoted to environmental hazard PERMANENTLY (re-asking and re-failing
  every tick). Fixed: `indiscriminate` now requires no owner was ever NAMED.

  ✔✔ `stamp_new_projectile_allegiance` takes the side where the entity is
  BORN, installed as a monolith-side combat-chain system (the presentation
  stamp lives in `ambition_projectiles`, which depends on neither
  `ambition_combat` nor `ambition_characters`).
  ⛔⛔ needed TWO placements, not one — `Materialize` runs before `Settle` in
  the same tick, so a fighter eliminated on the tick they fire loses the
  body after the bolt exists and before any pre-step placement sees it.
  Second placement added right after the player materializer.
  ⭐ guarded by `a_shot_stamped_at_birth_survives_its_firers_elimination`
  (uses `run_system_once` to model the exact tick — a plain `app.update()`
  can't isolate it).
  ⛔ a NEW rollback name reddens TWO baselines:
  `scripts/baselines/rollback-schema-baseline.json` and
  `game/ambition_app/tests/rollback_schema_baseline.txt` — the second a
  per-crate run never sees.

- ✔ **D151 — CLOSED 2026-08-17 (`d21031fc4`). `MatchAbilities`' `None →
  permitted` bridge turned PERMISSION into a GRANT** — with `permitted = kit
  + wall jump` so one character keeps an authored wall jump, every
  UNAUTHORED character took the `None` arm and got it too.
  ⛔⛔ PROBED FIRST: the bridge was LOAD-BEARING — the versus duel's
  `at_most`, neither duellist authored abilities, so deleting it naively
  would leave both with nothing. Safe order: dress the cast first
  (`VERSUS_FIGHTER_KIT`, `at_most(…)` now referencing that constant), then
  `apply` reads `authored.unwrap_or(AbilitySet::NONE)`. `levelled` is
  untouched, so smash's fourteen fighters are byte-identical.
  ✔✔ **CLOSED 2026-08-26, and BOTH halves of this paragraph were stale.**
  Re-read at HEAD before working it.

  (1) *"`apply` cannot do it because it never receives that kit"* — it does:
  `MatchAbilities::apply(self, authored: Option<AbilitySet>)`, and the body is
  `authored.unwrap_or(NONE).union(granted).intersect(permitted)`. The production
  caller states the same equation in its own doc — `effective = (authored ∪
  granted) ∩ permitted` (`prepared_match::effective_abilities`). The safe order
  this paragraph prescribed also happened: both duellists carry
  `.with_abilities(VERSUS_FIGHTER_KIT)` and `versus.rs` declares
  `at_most(VERSUS_FIGHTER_KIT)` against that same constant.

  (2) *"guard that no seated character relies on the bridge"* — **the bridge is
  gone and the ask has REVERSED.** `smash_fighter_kit()` has no definition left
  in the tree; the adaptation is now `roster_seeded` folding
  `smash_seating_melee()` into the seat's `ActionSet` at seating time, and the
  demo states that as a LEGITIMATE layer responsibility rather than scaffolding:
  most of Ambition's cast authors `default_action_set: "peaceful"` on purpose,
  and seating one in an arena means adapting it. ⇒ relying on it is no longer a
  smell to fail loudly on.

  ⚠ **BUT THE OPERATOR HAS A SILENT EDGE, AND THAT IS WHAT GOT GUARDED
  INSTEAD.** `at_most` GRANTS NOTHING, so a character that authored no abilities
  intersects to `AbilitySet::NONE` and arrives on the versus stage unable to
  move, jump or attack — and nothing refuses it, because *"this fighter may do
  nothing"* is a legal conclusion for a ceiling. The guard therefore sits on the
  CAST, not the operator:
  `every_fighter_the_duel_can_seat_authors_the_abilities_its_ceiling_narrows`
  (`versus_stage.rs`), with a premise guard on the seat count. Poisoned by
  inverting the predicate — it names both duellists.

- ✔ **D152 — CLOSED 2026-08-17. Empowerment expiry was a per-game scheduling
  footgun** — a game had to install `run_empowerments` itself or a
  two-second invulnerability became PERMANENT; five adopters had each
  remembered, and smash's respawn protection is how it surfaced. Split:
  contact-harm interpretation stays a ruleset choice, ticking/expiry is a
  domain invariant now installed by the engine in a named
  `EmpowermentExpiry` set, at `GameplayEffects` (last of three phases,
  ordering-preserving). Guarded by
  `a_timed_empowerment_ends_in_a_composition_that_scheduled_nothing`.

- ✔ **D153 — CLOSED 2026-08-17. A missing required sprite page failed OPEN**
  — an absent page logged `error!` and `continue`d, letting the barrier
  report Ready with missing presentation revealed. `RoomAssetManifest`
  gained `unresolved: Vec<String>`; readiness counts each entry as
  settled-and-failed, refusing the reveal. Guarded by
  `a_required_page_the_realization_lacks_refuses_the_room`.

- ✔ **D154 — CLOSED 2026-08-17 (`97a5b76ea`). Authored VFX was only half
  body-local: position was transformed through facing/gravity frame, but
  artwork drew world-upright** — a left-facing fighter's `air_slice` landed
  in the right place pointing right. `FxPose` now rides the
  event/request/message, derived from the same two authorities (owner's
  frame, move's committed facing) as the offset, matching the angle the
  sprite renderer already stands a body up with. Identity is the default and
  all eleven emitters state it explicitly.

- ✔ **D140 — CLOSED 2026-08-16. A second match never started and never
  ended: "GO!" stayed up and nothing could win.** (Jon, reproducible — his
  own "I thought we had tests for that" was the finding.) Two defects met on
  one `if`: (1) `StocksMatchSettled` could not be RETRACTED between matches,
  so match two opened wearing match one's verdict — now retracted by
  ACTIVATION. (2) the announce card had two writers and no arbitration; the
  GO! card overwrote the victory card — the old guard protected the wrong
  half ("do not CLEAR the winner's card"). The ceremony now stops talking
  the moment the match is decided.
  ⭐ the sim clock is requested to `0.0` while a match is settled and `1.0`
  while one is live, safe because hitstop's sink reduces by `min`.
  ⛔ the guard is the SEQUENCE — two matches in ONE app. A test that builds a
  fresh app per match cannot fail this, which is why the existing ones
  passed.

- ✔ **D143 — CLOSED 2026-08-18. The stage's unarmed declaration reaches the
  seat; the publisher was reading its own deferred write.** (found while
  answering Jon's moveset census) `DeclaredCombatRules::unarmed_melee` was
  installed by the same system fifty lines below the read, through a
  deferred `Commands::insert_resource` — so on the frame the match is
  decided the resource did not exist. Fixed: `smash_declared_combat_rules()`
  is the one source now, publisher takes the floor from the value it's
  about to declare and inserts that same value (reading the resource would
  also have been wrong once it existed — on a second visit it holds the
  PREVIOUS match's declaration).
  ⭐ measured three ways in the shipped host
  (`smash_in_the_host::report_what_an_unarmed_fighter_swings_once_the_stage_has_armed_it`):
  `mary_o`, `sanic`, `npc_alice`, `npc_bob` had zero moves before the fix.
  ⛔⛔ the guard that should have caught this SUPPLIED the missing value
  itself — `the_match_gives_every_seat_a_kit_that_can_hit` used to pass the
  swipe in by hand; it now calls `smash_declared_combat_rules().unarmed_melee`,
  poison-verified. ⛔⛔ a test seating a peaceful fighter cannot be written
  today — D144 armed every selectable fighter, so the floor has no live
  subject; the guard is necessarily about the mechanism, not a character.
  ⚠ what is left is not plumbing: whether the peaceful cast should be armed
  by the stage at all, or re-authored as fighters, is filed in
  [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) §26 —
  the defect was real and fixed under either answer, but the next kit-less
  character seated finds it again.

- ✔ **D144 — CLOSED 2026-08-16. Every selectable fighter now has the full
  sixteen-press smash kit** (robot v3 12→16, goblin and the Oni 11→16, the
  automaton 8→16, Mary-O/Sanic/Alice/Bob 0→16). The up-B was the half that
  mattered: several fighters had NO special at all, no way back to the
  stage.
  ⛔ the census was wrong twice: asking whether a verb KEY exists reads a
  fallback as coverage, and asking only one posture invented a gap (George
  Booul's down-B is `airborne_only` by design). Fixed: a press is covered
  when SOME posture reaches a move of its own.
  ⭐⭐ a special owes an answer in BOTH postures (Jon: *"a down-b that has
  special airborne properties should also have an effect on ground — think
  Bowser"*) — `directional_verb_chain` already puts `special_air_down` ahead
  of `special_down`, so a two-form move is authored, not engineered.
  ⛔⛔ this changes nothing in Mary-O's or Sanic's own games — a move table
  is what the swing IS, the ability is whether the body may swing at all.
  ⭐ the census is a ratchet, `report_the_smash_kit_every_selectable_fighter_has`,
  reading its target from `SMASH_KIT.len()`.

- ✔ **D145 — CLOSED 2026-08-16. No projectile could hit anybody on the smash
  stage — melee and projectiles asked different questions about who may be
  hit.** Melee used `team_allows_damage`; the projectile loop used
  `damage_lands(firer_faction, victim_faction)` with no team, ever — both
  seats came back `ActorFaction::Player`, so every shot from every fighter
  was spared as an ally. Fixed with one call to the existing
  `damage_lands_between`, since `StrikeVictim` already carried the victim's
  `team`. Guarded by a fixture with the poison inside it: a body on the
  firer's own team, overlapping the same shot, that must not be hit.

- ✔ **D142 — CLOSED 2026-08-16. A match could only ever TAKE verbs away, so
  no stage could promise a fighter anything.** Jon: *"in smash all
  characters should be sure they are granted the basic smash abilities, but
  we want to do this in an elegant way."* Fixed: `MatchAbilities { granted,
  permitted }`, `effective = (authored ∪ granted) ∩ permitted`. Smash
  declares `levelled(SMASH_FIGHTER_KIT)` (granted == permitted); versus
  declares `at_most(..)`, the lone mask it always was.
  ⚠ one fighter changed in play: the automaton gains double jump, fast
  fall, dodge, pogo and ledge grab under smash's grant. Sanic authors
  `[RunJump]` on both iterations — his super form is speed, not a
  capability unlock.

- ✔ **D141 — CLOSED 2026-08-16. One fighter on the smash grid could not
  grab a ledge, and the ledge lived at home on two who should not have it.**
  The Perfect Cellular Automaton's authored kit was written for the duel
  arena on `AbilitySet::basic()` (ledge = false), and `fighter_abilities`
  was an INTERSECTION so the stage could not give it back. Two roads: a
  character's own game reads the catalog grant list; a smash seat reads the
  character definition ∩ the match's mask.
  ⛔ a row that authors NOTHING falls through to `sandbox_all` — Sanic was
  carrying ledge grab, swim, glide, dodge and a bubble shield invisibly
  around his own speedway. His row now authors `[SaneSubset]`, excluding
  those five by name.
  ⚠ what a runner's kit should actually be is still open.

- ✔ **D138 — CLOSED 2026-08-17, CONFIRMED BY JON: "Oiler fights in his new
  body now."** Body swap (`69eee645f`) moved him off the Python-drawn sheet
  onto the direct-SVG rig; kit followed (`bd6cbf775` + `95b45b6cc`, sprite
  submodule `3f7d265`) — sixteen moves, eight new side-view rig clips,
  eighteen of twenty-three effects bound, geyser as Up-B. Carried with one
  new authoring primitive (`moveset_authoring::strike_tag`) and no engine
  change or character-ID branch.
  ⛔⛔ **A SHEET SWAP OWES THREE REGENERATIONS, NOT ONE** — after the rig's
  sheet installed, `ultrapack.json` still carried Oiler at the TOON frame
  size, since tier atlases bake from whatever was in `$sprites_dir` when
  last packed. Run `--target <t>`, then
  `scripts/regen/quality_variants.sh --target <t>`, then the four ultrapack
  tiers.
  ⛔ author arm poses as ANGLES on the reachable circle, never as (x, y)
  guesses — Oiler's arms are 26.5px from a shoulder 25.6px above the hanging
  wrist, already near full extension at rest.

  ⭐ four cues derive a plain id (`oil_geyser_stream`, `invariant_loop`,
  `gate_calibration`, `portal_leak`) while the bank ships only the
  `.loop`-suffixed version; measured 2026-08-18: one live (already
  overridden via `vfx_cued`), three latent (zero Rust references today).
  ⛔⛔ the two obvious fixes both cost something: a guard that every derived
  cue must be shipped would need an exemption list for Oiler's own two
  re-strikes (an exemption list is a TODO list); falling back to
  `{cue}.loop` would retrigger the loop on those same re-strikes.
  ✔ **DECIDED 2026-08-19 — JON PICKED: LEAVE IT RECORDED, no guard, no
  fallback.** ⛔ this is decided, not deferred: do not propose the guard
  again unprompted.
  ⚠ a BALANCE pass with real eyes: Oiler lost the observed match 36% to
  5%, the design's direction but a bigger margin than intended.
  ⚠ **why this row was stale for a day**: `scripts/regen/sprites.sh` still listed
  `oiler` in `review_cues`, which would have overwritten the rig's sheet on
  every full run. That entry is now an explicit ⛔ refusal pointing at
  `tackon_targets`.

- ◐ **D125 — The systemic world substrate: what a thing IS, which occurrence it
  is, why it exists, and how long it lasts.**
  ⭐ **NO ENGINEERING WORK REMAINS UNDER THIS ROW as of 2026-08-28.** Its one
  standing *"still open"* was the released-actor placement pair, and its own note
  had already said the question is a product call. That went to
  `awaiting-maintainer-decision.md`; the row waits on an answer, not on a slice.
  ⚠ marked `◐` rather than closed because the SUBSTRATE milestone is broader than
  the items under it — do not read this mark as "the substrate is finished".

⭐⭐ **A LIMBED MOUNT CHAIN CLOSES THE LEDGER FOR FREE.** `gnu_ton_arena` authors a
boss riding a mount that has hands: possessing the boss makes the chain
**rider → mount → limbs** — three links, two relation kinds (`RidingOn`, then
`Limb`). The chain is marked `InCustodyOf`; `project_custody_onto_authored_occurrences`
records every marked room-scoped occurrence as `InCustody`; a room rebuild
consults that outlook and declines to author what somebody is holding. Measured
in `gnu_ton_arena` while riding: all four identities appear — the rider, the
mount, and both limbs. So nothing in the ledger is per-relation, and a fifth
attachment kind inherits the protection by being an edge.

⇒ **the closure iterates to a FIXPOINT instead of walking a fixed depth**: edges
are `(attachment → anchor)`; an attachment travels when its anchor does; iterate
until nothing changes, bounded by the edge count. Poison-verified: dropping the
limb edge fails only the limb test.

⚠ **`CapturedBy` is deliberately NOT an edge.** A captive is attached to its
captor by exactly this rule, but no composition can express a captor carrying one
through a door. ⛔ adding it would be a rule for a state nothing can reach. ⇒ **if
a room-based game ever gets grabs, that is the edge to add, and this clause is
the note that says so.**

⭐ **RE-CHECKED 2026-08-21, and the gate is now a FACT rather than a genre
argument.** The clause used to rest on *"capture is the platform fighter's"*,
which is a claim about intent. The checkable version:

```text
acquire_captures  registered ONLY by game/ambition_demo_smash/src/capture.rs
                  — no engine plugin, no Ambition composition
```

⚠ **and the check was worth running, because the obvious signal now points the
other way.** As of `f3611b93d` every fighter authors a grab and four throws,
`Grab` IS bound in the shipped input preset, and thirteen of the movesets
carrying one are AMBITION characters — including `player_robot_moveset`, on the
V3 incarnation the player wears in rooms, whose own comment says the robot is
*"the SAME character in Ambition and in Smash"*. So a body walking around a
room-based game holds a grab MOVE. What it does not have is a system to turn
that move into a `CapturedBy`.

⇒ the trigger fires the day a second composition registers `acquire_captures`,
and that is one grep, not a judgement about genres.

⭐⭐ **A POSSESSED ACTOR WAS DUPLICATING ITSELF — FIXED 2026-08-19.** Possessing an
authored enemy and carrying it through a door left two live entities behind one
`SimId`: `record_placed_ground_items` is the only writer of `AuthoredOccurrences`,
so the occurrence ledger was items-only and never recorded where a possessed body
went. Possession expressed "this body travels" by SWAPPING ITS LIFETIME (remove
`RoomScopedEntity`, insert `SessionScopedEntity`) while items suspend residency
instead (`InCustodyOf`, lifetime untouched) — two mechanisms for one idea, and the
ledger's projection was blind to the swapped one.

⭐ Fix: possession now keeps the room scope and adds `InCustodyOf`; `RoomResident`
is `(With<RoomScopedEntity>, Without<InCustodyOf>)`, so the custody marker alone
excludes a travelling body from a room change's sweep. The item domain's own
custody projection (`project_custody_onto_residency`) had been using
`Has<RoomScopedEntity>` as a proxy for "would a room change retire it" — true only
while there were two kinds of holder; a possessed body is a third (room-scoped AND
travelling), so it now asks the `RoomResident` roster instead, making custody
TRANSITIVE for free. ⚠ two queries, not one: a despawned holder's custody is
deliberately left dangling (a known remaining orphan), so conflating "gone" with
"travelling" would make that orphan follow the player through every door forever.

⭐ Three possession unit tests pinned the old mechanism and are rewritten to
assert *scope untouched, custody added and dropped* — the poison for
reintroducing a promotion, since a promote/restore pair looks identical at the
end and differs in the middle. The old promotion also let a possessed enemy from
a destroyed world survive into a new game still being driven; the fix removes
that too.

⭐⭐ **THE SAME BUG HAD A THIRD POPULATION: a mount ridden through a door was
destroyed** (measured in `pirate_sky_lookout`/`pirate_cove`) — a mount is an
ordinary authored room actor and nothing suspended its residency. The rule is
TRANSITIVE: a mount is in its rider's custody exactly while that rider is itself
travelling, and `RoomResident` answers the question at every link without
anything counting them. ⚠ the discriminating case is the negative one — an
AI-piloted sky rider is room furniture and keeps its mount, so a rule that gave
every mount to its rider unconditionally would stop every authored mount from
being retired with its room.

⛔⛔ two wrong-first attempts, kept for the lesson: (1) two owners wrote the same
fact on the same tick — a mount projection in `WorldPrep` granting custody and
possession's in `PlayerSimulation` retracting it, with no structural filter
separating the populations — fixed by deciding the whole non-item body population
in one pass, one component, one owner; (2) the rule first asked `RoomResident`
about the rider, a roster that EXCLUDES anything already wearing the marker the
system itself writes, so the chain converged one tick per link and a released
rider left its mount in custody for a frame — fixed by asking `RoomScopedEntity`
instead. ⚠ `bevy::log::warn!` inside the affected system printed nothing during
diagnosis because the default log filter swallowed the custom target — worth
remembering before trusting a silent log as "not reached".

⛔⛔ **the first fix was itself wrong in a way only rollback would show.**
`InCustodyOf` is a DERIVED component excused from the snapshot by "room residency
reprojected from `ItemCustody` every tick" — but a possessed body has no
`ItemCustody`, so writing the marker at the possess site created a population
nothing reprojects: a rewind past the possession drops it and the body silently
becomes a `RoomResident` again. Fixed by making the marker a projection of
`PossessionState` (which IS snapshot state), so the derived-component excuse is
true for both populations. Poison-verified: moving the write back to the possess
site fails only the rewind test.

⚠ one measured consequence, left as-is: a driven body enters `CustodyBaseline` as
`placement:… <- slot:0`, which the item domain's restore ignores (its loop is
keyed on live items) — harmless, guarded by
`a_checkpoint_taken_while_possessing_does_not_manufacture_an_item`.

⭐ the save/load consequence was checked rather than assumed: a driven body's
occurrence enters the ledger as `InCustody`, and a save taken mid-possession
would carry that row to a fresh process where nobody possesses anything. It does
not survive: `republish_custody`'s retract-by-resetting contract runs every tick,
ungated, and the row is gone one tick after possession clears — guarded by
`a_custody_row_with_nobody_holding_it_is_retracted_before_a_room_can_act_on_it`
(asserts both that the row is written at all, and that it is retracted).

⭐ possession was the only subsystem doing this — grepping the capability
(`remove::<RoomScopedEntity>` + a later `SessionScopedEntity` insert on an
already-created entity) across every crate finds one other hit,
`SessionSpawnScope::apply_to`, which is spawn-time ownership rather than a
promotion, so there is no second place with the same ledger blindness.

⚠ `PossessionState::restore_scope` is vestigial now and deliberately kept — it is
a field of a rollback-registered resource, so retiring it is a schema change that
belongs to a version bump, not to this fix. Release now touches no scope at all;
removing and re-inserting one would be a bug, since it could silently revert a
scope write some other system made while the body was driven.

⇒ guarded at three tiers:
`an_authored_actor_carried_out_of_its_room_and_back_does_not_meet_a_copy` (app,
the duplication), `an_item_carried_by_a_possessed_body_survives_the_door_too`
(app, the coupling), `an_item_held_by_a_possessed_body_travels_with_it` (unit,
the transitive rule).

⚠ **still open — two named pieces.** An actor RELEASED in a foreign room writes
no `Placed` row, so it is retired when that room is left and re-authored at home
on re-entry — defensible ("the enemy goes home"), but keeping it where it was
left needs BOTH a producer (the placement recorder is items-only; a released body
needs the same `republish_placements` call) and a consumer
(`construction::relocate_request` returns FALSE for anything but a ground item
today, so an actor request would be refused and rebuilt at its authored spot with
a warn). ⭐ the refusal is honest, not broken — the room build already declines to
pretend an unmovable family moved. ⛔ adding the producer without the consumer
would make every re-entry log a warn and teleport the actor home anyway; the two
land together or not at all. Whether an abandoned enemy should stay put is a
product call, not a defect. ⇒ **PROMOTED to `awaiting-maintainer-decision.md`
2026-08-28** with the two-halves cost, because a product call sitting in the
execution ledger reads as work nobody has got to — it had sat here since
2026-08-19.

✔ **THE PERSISTENCE CARRY LIST IS CLOSED — 2026-08-19.** A checkpoint
baseline used to require five hand-synchronised enrollments across three crates;
`OwnedItemsBaseline` missed durable adoption and demonstrated the failure. The
runtime now composes typed lifecycle + actor checkpoint plugins, each domain owns
its concrete baseline systems and rollback facet, and `DurableBaselines` is
deleted. Durable item adoption happens in the same restore that applies the saved
bag, and a final completion step raises the global restore latch only after every
domain adopter runs. That also fixed the hidden pre-load-bag ordering bug with a
fresh-process poison. No erased callback registry was introduced; see
`engine/instance-lifetime-provenance-and-persistence.md`.

⭐⭐ **THE CONSTRUCTION FEDERATION'S SECOND LANE LANDED CHEAPLY — 2026-08-19.**
Gravity zones (chosen because they are boring — not an actor, no relation
vocabulary, no execution services) moved out of `ActorConstructionParams` into
`shared_tangle::gravity::construction`, beside `GravityZone`/`OscillatingZone`.
Zero new dependency edges (the construction machinery and `SpawnSessionScopedExt`
are already in the same crate); the constructor takes RESOLVED parameters, not
`GravityZoneSpec` (which lives downstream in `ambition_platformer2d_world`);
`GravityPlugin` contributes catalog metadata exactly as `PortalGunPlugin` does.
The lane is not feature-gated, unlike portal-gun, proving the same composition
shape applies to a capability every composition has.

⚠ the cost, recorded because it is the number that will justify a different
composition shape someday: a second lane touched ELEVEN separate blocks of
`RoomFeatureConstructionPlan` (plan field, receipt field, prepare, roster claim,
struct literal, deterministic dump, binding assert, verify, rebuild-one, commit,
committed ids). ⛔ it does not yet justify type erasure — a universal registry
able to execute a new domain is what this seam exists to avoid. The cross-lane
collision check, previously a hand-written pairwise intersection, is now a fold
that claims each lane's ids into the roster, so composing a lane and checking it
are the same line.

⭐ **portal-gun lane installation vs. schema fingerprinting is settled**: two
authorities agree by composition, not by type. The executable lane is compiled in
by `#[cfg(feature = "portal")]`; the schema fingerprinting entry is contributed by
`PortalGunPlugin` at runtime. A composition compiling `portal` but installing only
`PortalSimulationPlugin` would fingerprint a gun-less world while its rooms still
built authored gun pickups — prevented by one line: `PortalSchedulePlugin`
installs `PortalPlugin` (simulation plus gun) and is the only place in the
workspace that installs portal simulation at all. A test in `portal_schedule.rs`
pins that coincidence and goes red the day the line changes.

⭐ **A THIRD INSTANCE ARRIVED 2026-08-17, from a domain this row had not
touched, and it is the cleanest statement of the row's thesis: D150.** A
projectile's allegiance was reconstructed every tick by querying the firing
`Entity`, so a shot in flight turned on its own team the moment its firer
despawned — body RESIDENCY was standing in for stable identity. The same
domain's presentation half had already solved this
(`inherit_projectile_presentation_sources`: *"the bolt is the emitter … routinely
outlives the body that fired it. So the source is STAMPED at spawn rather than
looked up at impact."*); only the combat half still counted who was standing.
⚠ D148 was the same error the same day — the winner banner decided "is this side
a team" by counting resident bodies, so a team whose other member had been
eliminated announced its last survivor's name. ⇒ three independent sites in one
campaign, none aware of each other, each fixed by asking a FROZEN record instead
of a live query — the argument for the substrate rather than three more point
fixes.

✔✔ **THE RESTORE FALSIFIER IS GREEN (2026-08-16, `13dd4d31b`)** — bank a reward
at a checkpoint, carry it to another room, drop it there, leave so that room
unloads, then die: it comes back into the hand that banked it, as the same
occurrence (`SimId` and `SpawnOrigin` from the authored record), with its
pedestal still empty and no duplicate. Driven end to end on the composed host
through authored LDtk items, a real `HealShrine` + `Interact`, real door
crossings and a real `ActorDiedMessage`.

⭐⭐ the missing mechanism was MATERIALIZATION: the custody restore was pure
*re-assignment* — it walked live objects and asked whether the checkpoint agreed
with where each one was, a question that cannot be asked about an object whose
entity no longer exists, and no room build could supply it either (an
`InCustody` row makes `outlook_for` answer `Suppressed` in every room, because a
thing in a hand is not a thing in a room). ⇒ every other reconstruction road in
this engine starts from a ROOM and asks what it owes; this one starts from an
occurrence resident in no room, so the authored definition has to be reachable BY
IDENTITY. No new rollback state; schema stays v32.

✔✔ **AND THE RUNTIME-MINTED CASE CLOSED TOO (2026-08-16, `88b611caf`).**
Materialization was bounded by "some room authors a record with this id"; a
runtime-minted instance (the throw's `SimId::spawned` arm, where the inventory
count table equips an item with no object behind the hand) had no record
anywhere and was lost on a death. The minimal durable description turned out to
be three things and no more:

```text
identity     the occurrence's own SimId
provenance   SpawnOrigin::Dynamic { parent, sequence }
definition   the item spec's authored id — a REFERENCE, never a copy
```

⛔ no position, no velocity, no component snapshot — that is rollback wearing
save's clothes. `ground_item_physics` refuses to step anything not `InWorld`, so
the hand supplies the place.

⭐⭐ the first prediction was one field short, and the missing one is the
durable-save lesson: `(identity, spec)` alone rebuilds an instance that cannot
say which spawner it descends from (the state `SpawnOrigin`'s doc refuses to let
anyone spell), so it would survive exactly one death and then be invisible to the
next capture. ⇒ a durable description that restores the thing is not sufficient;
it must restore the thing's ABILITY TO BE DESCRIBED AGAIN — identity and
provenance are now minted as one value.

⭐ snapshot-not-registry is MEASURED, not asserted: rebuilt as a growing registry
of every mint with the restore returning each row to its spawner's hand, the
banked-item fixture stayed GREEN and
`a_runtime_mint_the_checkpoint_never_saw_is_not_resurrected_by_a_death` went RED.
`MintedItemBaseline` answers HOW to rebuild; the custody baseline still decides
WHETHER and INTO WHOSE HAND — it lives in the item domain because the lifecycle
crate cannot see a `GroundItem`'s spec. Schema 32 → 33.

⛔⛔ **AN INSTRUMENT DEFECT: A FIXTURE HAD BEEN MEASURING A WORLD NOBODY CHOSE.**
`with_start_room` took a ROOM ID, but `central_hub_basement` (used throughout one
test's comments) is an LDtk LEVEL name; the option silently fell back to the
authored entry room instead of failing. Measured blast radius: of 40 literal call
sites, only that one didn't resolve as a real room id, plus one deliberate
negative test.

⚠ a written counter-argument in the code
(`game/ambition_app/src/app/resources.rs:~240`) argued the strict/tolerant split
was deliberate: a programmatic override from a library caller
(`Platformer2dSimHarness`) may legitimately miss, so falling back is correct,
while a CLI flag is already strict because it was typed by somebody who wanted
that room. The objection was answered by counting adopters: tolerance had zero
beneficiaries — every literal in the tree was a real room id.

✅ **LANDED 2026-08-17 (`3f116e88b` seam, `90187a559` migration).**
`with_start_room` stayed tolerant (a test, `unknown_start_room_does_not_panic_or_error`,
makes tolerance a promise) and `with_required_start_room` was added beside it,
refusing to boot and listing all 72 ids. All 24 files that name a room —
including the shared `tests/common::fixed_60hz_room_options` helper (fanning out
to ~20 more fixtures) and the `ambition_app_tools` `headless`/`rl_smoke` binaries
— were migrated to the strict form; one tolerant literal remains, which is the
promise's own negative test. `cargo test --workspace --lib` 4867/0, `app_it`
412/0. The defect this item was opened for is gone: the test that asked for
`central_hub_basement` now names a real room.

⛔ one literal wasn't a room id at all: `collision_invariant_oracle::run_episode`
used `""` as its OWN sentinel for "keep the LDtk-authored start" — never a
beneficiary of the fallback. Fixed by passing no start room rather than asking
tolerantly for one. Nothing else in the tree (every `const ROOM`/hand-listed room
array) relied on the fallback — the row's premise held. A stale comment in
`rl_smoke` describing a now-unreachable fallback branch was corrected.

⭐ **PROMOTED FROM THE RESERVOIR 2026-08-14** — seven focused plans for this
frontier already existed and were reachable only from
[`tracks.md`](tracks.md), not from this queue:
[`engine/instance-lifetime-provenance-and-persistence.md`](engine/instance-lifetime-provenance-and-persistence.md),
[`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md),
[`engine/capability-progression-and-world-gating.md`](engine/capability-progression-and-world-gating.md),
[`engine/platformer-navigation-and-reachability.md`](engine/platformer-navigation-and-reachability.md),
[`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md),
[`engine/persistent-actors-and-population.md`](engine/persistent-actors-and-population.md),
[`engine/agentic-character-runtime.md`](engine/agentic-character-runtime.md).

**Sequence — each step's identities are what make the next one expressible:**

1. ✔ **The substrate is already built, under names this plan does not use —
   measured 2026-08-14.** ⛔ do not build it.
   - *What authored thing is this?* → `WornCharacter(CharacterId)` on the body,
     resolved through `PreparedCharacterRegistry`; its doc splits NAMING a
     template from APPLYING it (`RecharacterizeBody`).
   - *Which runtime occurrence is this?* → `SimId`: deterministic, namespaced
     (`placement:` / `slot:` / `encounter:` / `spawned` / `strike`),
     `#[require(SimIdCounter)]`, dynamic spawns minted as `(spawner SimId,
     per-spawner counter)`. Every snapshot row, checksum projection and
     cross-reference keys on it.
   - *Why does it exist?* → `SpawnOrigin`, a component:
     `Authored{source,instance}` / `ProviderStaged{provider,room,instance}` /
     `Dynamic{parent: SimId, sequence}`, `parent` non-optional by design,
     verified against the construction plan roster and encoded into rollback
     blobs. Its module states the rule outright: provenance is data, never
     recovered by parsing an id string.
   - *How long should it last?* → four ENFORCED scopes, each owning a sweep:
     `RoomScopedEntity`, `ModeScopedEntity`, `RoundScopedEntity`,
     `SessionScopedEntity`, plus per-domain TTLs and `EncounterCleanupPolicy`.
     `round.rs` states the rule: "round scope is a LIFETIME, not a provenance —
     where an entity CAME FROM does not say how long it should live."

   ⛔⛔ **the one real gap was a FALSE DECLARATION, now deleted.**
   `RunScopedEntity`/`PersistentEntity` (with `spawn_run_scoped`/
   `spawn_persistent`) had zero producers, zero consumers, and no sweep read
   either marker — worse than a missing lifetime, because `lifecycle/mod.rs`
   directed new spawn sites to `SpawnScopedExt` and two of its four verbs
   silently did nothing. `RunScopedEntity` duplicated `SessionScopedEntity`;
   `PersistentEntity` was a second spelling of absence (every sweep culls on
   marker presence, so an unmarked entity already survives all four boundaries,
   and the marker falsely implied it OVERRIDES a scope). ⇒ the surviving rule:
   **a scope is spelled here only if a sweep enforces it.**

   ⇒ **what genuinely remains in step 1**, both listed as unresolved by the
   focused plan itself: **persistence policy** and **the explicit terminal
   transition**. ⛔⛔ "has no runtime cleanup scope" does NOT mean "durably
   persistent open-world object that is correctly saved and restored" — an
   unmarked entity merely survives this session's four boundaries; the
   durable-persistence system is still undesigned.

   ✔ the player-centrism smell in `room_transition/commit.rs` is gone
   (2026-08-19) — whether a transiting body survived the room change was
   inferred from a presentation-state proxy that only ever bought nothing or
   lost; now an unconditional `Some(subject)`, safe because `retire_outgoing`
   already skips an entity absent from its roster.

   ⛔ authored identity does NOT imply world uniqueness — "there is normally one
   Fia" is content policy, not the meaning of a definition id. ⛔ do not invent a
   universal `EntityId`; the separation between identity types is settled and
   deliberately not unified.

2. ◐ **Item custody.** ⛔ "item entities carry no `SimId`" was WRONG on the
   authored path — `construction::authored_ground_item_requests` builds every
   LDtk `GroundItemSpec` row with `SimId::placement(&spec.id)` +
   `SpawnOrigin::Authored`. The real defect was that the identity WAS destroyed
   at the first custody change: pickup called `despawn()`, throw called an
   unconditional `spawn_room_scoped`.

   ⛔⛔ the pickup fork was SYMMETRIC — neither population was correct.
   `collect_ecs_pickups` used `With<PlayerEntity>` (N couch seats, excluding a
   possessed body — the reported bug); `collect_world_items` used
   `Res<ControlledSubject>` (exactly one body — so seat two never picked up
   mushrooms, an unreported half). Unifying onto `ControlledSubject` alone would
   have cut couch collectors from N to 1. Fixed with a filter-plus-value
   population serving both, pinned by a test asserting a second seat AND a
   possessed body both collect (either half alone passes on the broken code).

   ⇒ **the instance / quantity / consumable split is now written on
   `ItemCustody`:** a `GroundItem` is an INSTANCE and keeps its identity across
   world → held → world; `PickupKind::Currency`/`Health` and the `OwnedItems`
   counts are QUANTITIES (two coins are the same coin — what survives is a
   number on the collector); a `WorldItem` is a CONSUMABLE whose despawn is a
   real end of life. A body equipped from the count table has no object behind
   its hand, so throwing turns a quantity into an instance, minted
   `SimId::spawned(thrower, counter.next())`.

   ⚠ `ItemCustody` IS rollback state, registered as such (clone + entity-SET
   probe since `InWorld` names no body, paired with `rollback_map_entities`);
   schema 29 → 30, both baselines updated. It gates drawing, physics and
   grabbability on later frames, taking over a job GGRS previously did through
   the entity anchor.

   ⛔ **the INVENTORY leg is explicitly NOT closed**, and says so in code:
   `OwnedItems` is a global count table with no row per object, so *whose
   inventory does a possessed body fill?* still has no answer, and
   `equip_held_spec`/`unequip_held` are labelled a migration seam rather than
   the model. ⭐ physical custody belongs to the body and the item instance;
   participant entitlement is a separate fact with a different owner and
   lifetime.

   ✔✔ **THE TWO WEAPON DROP SITES ARE MIGRATED (2026-08-19).**
   `damage/actor_hit.rs`'s dropped weapon and `damage/boss_hit.rs`'s signature
   gauntlet were the only two death drops the 2026-08-05 room-scope fix never
   reached — both spawned session-scoped with no `SpawnOrigin`, so a defeated
   pirate's gun-sword followed the player into the next room. Both now spawn
   through one `drop_held_weapon`. The coverage guard that should have caught
   this (`the_pickup_drop_table_is_complete`) scanned one file for one spelling
   and missed both inline `GroundItem` spawns; it now reads all four
   damage-path files for both collectible spellings, poison-verified to name
   `apply_actor_hit` the moment a drop is written inline again, and checks the
   production `RoomResident` roster rather than restating the rule.

   ⇒ **carried forward:** death drops still mint no `SimId`, deliberately — an
   identity would enrol the drop in `TransactionBaseline::capture`, whose
   roster a room-scoped entity leaves mid-transition; provenance without
   identity is the honest intermediate state. An orphaned custody is possible
   if a holder despawns while carrying (inert, bounded by room/session scope,
   no reaper built — no state has demonstrated the need). `ChestFeature::reward()`
   still has zero production callers — authored chest rewards are parsed,
   lowered onto the live component, and never granted.

3. ◐ **Capability-driven gating and platformer reachability — first slice
   landed 2026-08-14, driving the real kernel rather than a hand-enumerated
   capability list.** The design an agent arrived with (a closed-form reachable
   envelope with a hand-enumerated capability list) would have failed the same
   way as the deleted "airborne + below the lip ⇒ already dead" rule; instead
   `movement/recovery.rs` clones the body and drives its OWN kernel
   (`ae::step_motion`, pure, no Bevy `World`) over three fixed ordered efforts,
   reporting `Regained { steps, side }` or `NoSupportFound { reset }`.

   ⭐⭐ it states no rule about bodies — every capability the kernel implements
   is honoured because the kernel honours it, gated by the body's own
   `AbilitySet` and `AxisSweptParams`, so there is no capability list to fall
   out of date. ⚠ the effort is a reactive rule, not a search: hold the side,
   hold jump, re-press the instant the body stops rising — pressing every tick
   burns a whole air-jump budget in consecutive frames, pressing at apex chains
   the most height, holding between presses avoids cutting a variable-jump arc.

   ⇒ `recovery_capability_gap(..) -> Option<AbilityGrant>` answers "which
   capability blocks the route" in the engine's existing authoring vocabulary,
   skipping a grant that adds nothing (re-granting `AirJump` to a body that has
   the verb and spent the charge would refill the budget and misreport a spent
   charge as a missing verb).

   ⛔ **the consequence is open: nothing reads either function.** No call site
   outside the module's tests, nothing wired into the fighter's search, no
   fighter tuned. `NoSupportFound` carries `reset: Some(cause)` only when every
   effort ended in a world reset; the module doc says a brain, a validator, or
   an LLM decides what that means. Nothing added is rollback state.

   ⭐ the three pins, each with its falsifier: the body's own kit decides (same
   world/position/velocity, only `move_horizontal` differs, both terms
   asserted); the deleted rule's exact `doomed` state is answered by surfaces
   (poison: remove the one catch block, the identical body must report
   not-recovered); the probe is gravity-generic (room transposed, gravity along
   `+x`, non-steering body still fails).

   ⇒ **a deletion gate was named rather than taken**, on the fighter rollout's
   duplicate integrator (whose own doc concludes "the fix is not different
   constants, it is DERIVATION"). Delete it when three things hold: the brain
   can obtain a `&ae::World` without depending on `ambition_platformer2d_world`;
   one real kernel step per shadow step is measured affordable against
   `rollout_k × (1 + rollout_depth)`; and `ladder_rig --scenarios` re-runs green
   — the only instrument that has ever caught a shadow-physics divergence.

4. **Capability-driven gating — the GATING half.** The robot navigates
   because of body capabilities, equipment, physical properties and changed
   mechanisms — **never protagonist identity or a quest flag**. Reachability
   should answer engine/agent questions: can THIS body reach there, which
   capability blocks it, how do portals / moving platforms / gates change the
   route.
4. **Open-world residency and persistent populations, last.** World existence,
   room residency, full simulation and visibility become distinct; named
   actors/items/world changes survive room absence; spawned populations get an
   explicit lifetime policy. ⛔ background simulation stays DELIBERATELY
   UNRESOLVED until evidence says how much is needed. Different-room multiplayer
   then falls out of the model instead of becoming a special multiplayer world.

⇒ **prerequisites, both nearly met:** D71/D92's converged room-transition
transaction (done) and D116's view separation (in flight). Residency is the step
that needs them; the substrate itself does not, so step 1 may start now.

⛔ **NOT IN THIS ROW:** a quest/story framework, a dialogue engine, a generic
scripting layer, production networking, or substantial authored story. The target
is a large coherent 2D world the robot can traverse, alter, leave, return to,
save and reload — **authored story comes after that world exists**, not to
motivate it.

⭐⭐ **THE CUSTODY SEAM IS MEASURED (2026-08-15) AND IT IS ONE CLASS WIDE.** Full
partition and evidence: [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md).
Instance-capability is decided solely by whether `Item::held_item_id()`
resolves — 9 held weapons are both an instance and a count, the other 5 classes
are counts forever whose readers legitimately want a quantity. ⛔ so do not give
the count table a row per object.

⚠ the flattening is `items/pickup/mod.rs:616` and composes into an unbounded
duplication loop (equip from the count with no object behind it → throw → the
mint arm materialises a second axe). ⛔⛔ not fixed, deliberately: `owned.take` on
throw destroys thrown items on room exit, because the count is currently the
durable-save mirror of an instance, not entitlement.

✔✔ **A CARRIED ITEM CROSSES A ROOM BOUNDARY WITHOUT THE TRANSITION KNOWING ITEMS
EXIST — LANDED 2026-08-15.** `ItemCustody` is projected onto an `InCustodyOf`
marker; the roster a room change retires became `RoomResident =
(With<RoomScopedEntity>, Without<InCustodyOf>)`. The item never loses its room
scope (so a *reset* still destroys it, correctly), and losing custody makes it
resident in whatever room is active then — room residency carries no room id, so
"dropped in the destination" needs no memory. It is body-generic for free: a
room-fixture holder (unpossessed NPC) leaves the item resident so it dies with
its room; a despawned holder makes it resident again, closing the death-drop
orphan; a possessed carrier gets the travelling answer without anyone asking who
the player is. Poison-verified: reverting the roster to plain
`With<RoomScopedEntity>` fails the carried-item test exactly and leaves the reset
test green.

✔ deleted in the same slice: `world/rooms/load.rs`'s `commit_room_transition_geometry`
+ `RoomLoadResult`, a public 60-line second copy of the room commit with zero
callers. `derived.custody_residency` is a declared-derived row (recomputed
unconditionally each tick, no "already applied" gate), so schema stays 31.
Verified: app_it 363, Smash 18, monolith 1230, contracts 27/27.

✔✔ **CONTINUITY'S FIRST LEG LANDED 2026-08-15** — a rebuilt room now asks what
became of the occurrence it minted last time, via a DISPOSITION rather than "is
something with this id alive": `OccurrenceDisposition::{Authored (default),
Persisting, Consumed}`. Construction gained a sixth stated authority
(`occurrences: Option<&AuthoredOccurrences>`), and
`RoomFeatureConstructionPlan::prepare` retains only requests whose disposition
`authors_a_fresh_occurrence()`. ⭐ `Consumed` is spelled and read but has NO
PRODUCER, deliberately — the honest slot for permanent destruction, making
"ephemeral / resettable" the default rather than a special case, reserved before
the terminal cases exist.

✔ deleted: `RoomConstructionPlan::prepare(&World, ..)` — 84 lines, zero callers;
`RoomConstructionError::MissingService` went with it. The prefetch cache now
states the dispositions it froze and refuses a plan prepared against different
ones; `ConstructionPlan::prepare`'s `live` argument (previously
`&Default::default()`, false since the custody slice) now carries the suppressed
set, so a road reaching a suppressed identity gets `IdentityAlreadyLive` at
preflight instead of a duplicate.

✔✔ **ITS OWN NEW TEST FOUND TWO FAULTS, BOTH CLOSED 2026-08-15 — one instrument,
one real.** Real: a sandbox reset rebuilt the room and then emptied it —
`process_new_game_reset_request` retires every `RoomScopedEntity` and commits the
fresh room plan, `.chain()`ed with `clear_transient_on_sandbox_reset`, whose
transient sweep then despawned the items the reset had just authored (authored
ground items are room-scoped). ⛔ a plain `.chain()` carries an auto-inserted
`ApplyDeferred`, so a later system in the chain SEES what an earlier one just
spawned. Fixed by deleting the overlap: the transient sweep gained
`Without<RoomScopedEntity>`, since `retire_outgoing` already sweeps room scope
unconditionally and is the stricter of the two — this silently fixed the same
latent defect for an authored `PortalGunPickup`. Poison-verified both ways.

⇒ **what a sandbox reset restores, stated once:** the start room from its
authored records alone — every authored placement, feature and actor comes back
exactly once, including one that was being carried, because the reset destroys
the world those occurrences live in, hands included. ⛔ it restores nothing
outside room scope: a dropped weapon, a summoned ally, a placed portal and the
player's held state are retired by the transient sweep and never come back.

Instrument fault: the test *"a carried object survives a reset"* never ran the
reset it described — `reset_episode()` triggers `reset_sandbox` +
`ResetRoomFeaturesEvent{Manual}`, which restores room FEATURE state in place and
never touches `RoomScopedEntity`. The real reset (`process_new_game_reset_request`)
is reachable only through `NewGameResetRequested`, whose writers are the
kaleidoscope menu and tests — nothing on the input path. ⛔⛔ the failure was half
silent: a sibling assertion (`occurrences(..).len() == 1`) passed vacuously,
since the surviving carried item WAS the one occurrence, so a reset that did
nothing at all satisfied it. The test now drives the real reset with every
original assertion kept, plus one the count couldn't see: the carrier's hand is
empty afterwards.

⛔ the deletion gate (collapsing the two reset roads) was refused on purpose —
`session/reset/mod.rs:165` already argues one sweep cannot answer both "does this
survive leaving the room" and "does this survive replaying it".

⚠ **a real latent gap found and deliberately NOT fixed:**
`clear_transient_on_sandbox_reset` scopes hand-emptying to `With<PlayerEntity>`,
so a non-player carrier keeps a dangling `HeldItem` after the reset destroys the
object — the simulation is not player-centric and this road still is. Left alone
because the same loop also restores `ActionSet`/`StashedActionSet` and strips
`PortalGun`, genuinely player concerns. ⇒ relaxing the filter is a product call,
not a refactor.

⇒ **remaining legs of the same question:** the two terminal cases — destroyed
permanently ⇒ never recreate (needs a `Consumed` producer) and intentionally
resettable ⇒ may recreate (already the default). ⭐ verified 2026-08-18: still
ZERO producers of `Consumed` — a feature waiting on a product answer (what
destroys something permanently?), not a defect. Two carried risks:
`SimId::placement(id)` is a global namespace whose uniqueness is checked only per
room, so two rooms authoring one id would suppress both; and the ledger is not
experience-scoped, so a suppressed row can survive into a new session.

✔ **THE FIRST RISK IS GUARDED (2026-08-18), while it was still free.**
`validate.placement_id_collision` warns when one authored `id` names things in
two rooms — green on every shipped world (twelve entity kinds carry an `id`; the
only cross-room reuse is `LoadingZone`, deliberate because a zone's
`target_zone` resolves within its `target_room`). ⛔⛔ the collision is
reachable, not hypothetical: `authored_logic/prepared.rs` turns an authored
`placement:<id>` argument into `SimId::placement(..)` in production, so the day
content names `placement:return_door` it would mean seven zones. ⚠ four tests
pin it, since a guard green on all real data is otherwise indistinguishable from
one that cannot fire.

✔ **the cross-world half is checked too (2026-08-19)**, via the RUNTIME's merged
`RoomSet` (the file validator alone cannot load every world at once):
`no_two_rooms_in_the_merged_world_author_the_same_id` — 72 rooms merged, 358
distinct authored ids, 0 collisions. Carries three falsifiers: a rooms floor
(≥50), an id floor (≥300, fired at 294 when one authored kind was dropped from
the collection), and the collision assertion itself. `LoadingZone` ids stay
exempt for the same reason as the file validator.

✔ **a suppression written in one experience survived into the next — fixed
2026-08-18** by `session::teardown::reset_session_scoped_resources_on_retire`
clearing `SaveRestored`, whose latch was set true in
`restore_inventory_from_save` and set false nowhere, so a second experience in
one process never re-ran the durable restore and inherited the first
experience's ledgers. Guarded by `retirement_clears_the_save_applied_latch`
(asserts both that the latch survives an ordinary frame and is cleared on
retirement).

⛔ two costed-and-rejected shapes: keying the reset on the session generation,
and a scope-watcher — both more machinery than the seam that already existed
(the lifecycle that ends the session is the one place that cannot forget). ⛔ the
exemption-list answer is also wrong: scopes are authored per experience, so
hand-adding `.resetting::<AuthoredOccurrences>()` to today's two experiences
would make a third game's omission a silent bug — the ENGINE owns the invariant,
the composition owns the order.

⇒ the hazard that started this leg, recorded on `ItemCustody`: carrying an
**authored** placement out of its room and back yields the carried object **and a
freshly authored copy with the same `SimId::placement(..)`**. It could not arise
while the boundary destroyed the object.

⭐ but the hazard is the small statement of a much bigger question, and this is
the systemic-world pressure the project has been trying to reach:

> when authored placement **P** has produced a runtime occurrence that has since
> **moved**, been **consumed**, been **destroyed**, or entered **custody
> elsewhere**, how does world reconstruction know what should happen to P?

It sits underneath persistent items, moved NPCs, opened/removed mechanisms,
destroyed objects, relocated quest objects, persistent populations, room
streaming and save/load. Design owner:
[`engine/instance-lifetime-provenance-and-persistence.md`](engine/instance-lifetime-provenance-and-persistence.md),
where it forced two questions off the *deliberately unresolved* list.

**Falsifier:** `enter A → axe P exists → pick up P → carry to B → return to A →
P must NOT respawn, and the original occurrence still exists elsewhere.`
Terminal cases to follow: **destroyed permanently** ⇒ never recreate;
**intentionally resettable** ⇒ may recreate.

✔✔ **THE CHECKPOINT/RESET HORIZON LANDED 2026-08-15.** Seven beats of the
maintainer's rule hold end to end through production roads: an object acquired
before any checkpoint goes back on its pedestal; acquired-then-banked stays in
hand with the pedestal empty; and one death reaches two opposite answers about
two objects of the same kind in the same frame, separated only by which side of
the checkpoint each acquisition fell on — a result no `KeyItem => survives` rule
can produce.

⭐ the baseline is a projection of DOMAINS, not a resource: `lifecycle::horizon`
owns two messages and two sets and nothing else; `OccurrenceBaseline` and
`CustodyBaseline` are captured by their own domains from their own live
authorities. Both are checksummed rollback state (schema v32).

⭐⭐ three defects the fixture found by RUNNING it, not by reasoning:
1. restoring the ledger and emptying the hand DELETES the object — the room
   replay resets features in place and never re-runs authored construction.
   Fixed: a death is a checkpoint RESUME, recording the same
   `LifecycleIntent::Transition` a session-start resume records, so same-room
   re-entry rebuilds correctly.
2. custody is a FORKED relation (`ItemCustody` on the object, `HeldItem` on the
   body) — retracting one half left the body holding a ghost and refusing every
   future pickup. ⛔ the tempting generic repair ("empty a hand matching nothing
   in custody") would disarm every authored fighter, since a character
   definition's `held_item` needs no world object.
3. a hand must be EMPTIED before it can be FILLED — interleaved, the
   reinstatement was equipped over an occupied hand and `return_released_items`
   quietly undid it one phase later.

⭐ the gap that looked save-destroying is NOT, measured
(`a_banked_object_whose_room_unloaded_returns_to_the_hand_that_banked_it`,
`13dd4d31b`): a baseline row whose occurrence has no live entity (banked, carried
next door, put down, that room unloaded, then a death) would seem erased when
the restore overwrites the `Placed` row — but `republish_custody`'s
retract-by-RESETTING rule saves it in a case it wasn't written for: the custody
leg is rebuilt from live state every tick, so the unsupported `InCustody` row is
dropped and the home room authors the object at its pedestal. The player loses
the *acquired* property they banked (wrong but recoverable); the object is not
destroyed (which would not be).

⚠ that safety is CONDITIONAL and pinned by a characterisation test — it holds
only because nothing lets an `InCustody` row outlive live custody. ⛔ a durable
save that writes the ledger straight to disk breaks exactly that, and the
annihilation becomes real.

⭐⭐ **MAINTAINER DECISION 2026-08-15 — the CHECKPOINT is the reset baseline.**
Death/retry restores the latest committed checkpoint; traversal and unload
preserve current state. ⛔⛔ NOT `KeyItem => survives reset` — a key item
persists because acquiring it committed a checkpoint. ⇒ the owner must
distinguish THREE horizons: current occurrence state · state at the
reset/checkpoint horizon · durable save. Fixture and full text:
[`maintainer-decisions.md`](maintainer-decisions.md) and
[`engine/instance-lifetime-provenance-and-persistence.md`](engine/instance-lifetime-provenance-and-persistence.md).

⛔⛔ do not answer it by teaching the room loader to inspect inventories — that
is another composition census, and the landed slice's achievement is that room
transition never learned items exist. ⛔ no universal instance registry: the
abstraction belongs around the disposition of the authored occurrence, with
storage discovered from this customer.

✔ **SETTLED: the body owns its inventory and capabilities.** Participant
entitlements and possession-transfer policy are separate concerns with different
owners and lifetimes, so `OwnedItems` is a migration/compatibility
representation, not an undecided authority. ⛔ do not re-open it, and do not
start the `OwnedItems` migration ahead of persistent occurrence continuity,
which has the stronger product pressure.

✔ landed meanwhile, needing no product decision: stow and equip-swap left an
object recording `ItemCustody::Held` by a body with an empty hand — a third
state the enum doesn't have, so an authored axe silently ceased to exist through
the menu. Custody is now re-derived from the hand and RESET to `InWorld`.

✔✔ **ANSWERED 2026-08-18 with the registration assertion it asked for.**
`the_production_plugin_registers_the_custody_release` builds
`ItemPickupSimulationPlugin` and asks the SIM SCHEDULE whether
`return_released_items` is registered — poison-verified by deleting the real
registration (the behaviour test stays green since it exercises a hand-built
chain, while the new guard alone goes red). ⛔ the first draft of the guard was
itself the bug it hunts: it initialized the schedule against a FRESH `World`,
enumerated zero systems, and reported "not registered" for a system that is. It
now asserts a non-empty floor first, so "the enumeration is broken" can never be
read as "the registration is missing" — every count needs a zero floor.

⛔ **Bevy-crate extraction is a criterion applied at every step, never a
follow-up cleanup campaign.** Reach a coherent internal `Plugin` with owned
components/resources/messages/system sets and no upward registration; extract to
a workspace crate when dependency isolation is genuinely real; call it
independently consumable only after a small external-style `App` uses it with no
Ambition content or policy. Never carve because a file or crate is large — a
failure mode this repository has already named twice.

- ▢ **D72 — Push Super Smash Siblings as a product and engine customer.**

  **Current execution authority:**
  [`demos/campaigns/smash-fun-push-2026-08-22.md`](demos/campaigns/smash-fun-push-2026-08-22.md).
  Work its independent slices in order; if one blocks, record the blocker there
  and continue with another unblocked slice.

  **Canonical feature truth:**
  [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md). It owns
  shipped/partial/absent status, effort, implementation seam, and whether a
  feature is product work, `E1`, `E2`, or `WAIT`. Re-grep a row before changing
  it.

  **Product/architecture charter:**
  [`demos/super-smash-siblings.md`](demos/super-smash-siblings.md). Fighters stay
  ordinary bodies; human and CPU control share mechanics; presentation consumes
  resolved simulation facts.

  `E1` work is allowed to land with the feature in its current semantic owner.
  Give `E2` work a focused campaign. Do not wait for unrelated monolith,
  simulation-phase, capability/runtime-composition, or public-facade migrations.
  Do not expand a `WAIT` architecture merely to close a parity row.

  When the active campaign closes, update the inventory, archive the campaign,
  and replenish D72 from the highest-value remaining inventory rows rather than
  creating another standing Smash backlog.

  ⭐⭐ **THE CAMPAIGN'S MECHANICAL WORK IS DONE — 2026-08-24.** Every `O` slice
  and `W1`–`W7` are shipped and marked in both the campaign and the inventory:
  autolink knockback (kernel + authoring + an authored customer), the rising
  spin, once-per-airtime recovery, the respawn release rule AND its platform, the
  shield-drop lag that was already there, and the HUD punch. ⛔ **NOT ARCHIVED**,
  because `W8` is a PLAYTEST and it is Jon's — the file is where his observations
  land, and archiving it would send them nowhere.

  ⇒ **replenished from the inventory, smallest-first, so the next session has
  work that fits a slice rather than a campaign:**

  | row | why it is next |
  | --- | --- |
  | Z-drop / neutral drop (`✔ SHIPPED 2026-08-24`) | ⛔ THIS TABLE WAS STALE, the inventory was not. One enum inside `throw_held_item_system`, guarded by a CONTRAST fixture; `a_dropped_item_falls.rs` carries the app arm |
  | Self-damage / recoil move (`▢ S`) | an owner-side `on_hit` through the effect seam that already exists |
  | Stock + timer selectors (`▢ S` → **RE-PRICED, it is not S**) | ⚠ the CONFIGURATION half is genuinely nothing — `MatchRules::stocks` and `time_limit_ticks` are already read, and `apply_smash_match_rules` sets both from constants. The UI half is where the S goes wrong: **the control strip has no room on a phone.** Measured from `layout.rs`'s own constants — strip = viewport − 2×14, minus a 520px prompt, a 150px START and the gaps: **562px free on a 1280 desktop, 126px on an 844 phone, and 18px once paging shows its two 44px arrows** (which it does at phone size, by that file's own worked example). ⛔ and the two other edges are spoken for: the host owns the top-RIGHT corner (`TITLE_H`'s comment) and BACK owns the top-left. ⇒ **the genre's own answer is a SEPARATE RULES STEP** — Ultimate puts rules before character select — which sidesteps the strip entirely and is not an `S`. ⚠ and it should not jump the queue ahead of Jon's standing report that the select screen's *controls* do not feel good with a gamepad: two more cursor targets on a screen he already finds hard to drive is the wrong order |
  | Cannot-clank / transcendent hit (`▢ S`, wants a CUSTOMER) | ⛔ do not build it: the inventory row already says why, and re-measuring agreed. `arbitrate_attack_clanks` queries `&StrikeVolume`, which ONLY `advance_move_playback` spawns for an authored move window — a projectile carries none and never reaches the arbitration. The genre's transcendent hits are mostly projectiles, so **they are already transcendent by construction**. What is left is a MELEE move that wants to pass through a swing, and nothing authors one |
  | Pivot grab (`▢ S`, twice) | ⛔ BLOCKED on the §4 turnaround/pivot FACT; the capture side needs nothing. Do not start it from the capture end |

  ⛔ every one of these was re-read against the inventory on 2026-08-24, not
  copied from an older list — and the pivot-grab pair is listed WITH its blocker
  precisely because two rows name it and neither says so on its own.

  ⛔⛔ **AND ONE OF THEM WENT STALE ANYWAY WITHIN TWO DAYS, which is the reason
  this row names the inventory as the canonical truth.** Z-drop shipped on the
  very date this table was re-read and the table kept its `▢`. ⇒ **a duplicated
  status is a status that rots**: read the inventory row, not this column, and
  when they disagree the inventory wins.

- ▢ **D33 — Continue actor-monolith decomposition by coherent ownership.**

⭐⭐⭐ **FOUR MORE CARVES 2026-08-28, AND THE SESSION'S DURABLE OUTPUT IS THAT THE
CENSUS WAS BLIND THREE WAYS.** The candidate table ranked by file size and by
`crate::` occurrences; every top entry was wrong, because a `crate::` count sees
neither a FACADE HOP (`crate::features::X` where X lives in a peer crate — naming
the owner took `bosses` from 2 refs to 0 with no code moved), nor a GLOB (`use
super::*`, measured by DELETING it and reading the WHOLE error set, since rustc
stops resolving early), nor a `super::super::` PATH, which only the MOVE finds.

```text
ambition_boss_encounter::ecs + ::attack_moveset   the boss tick and its authoring
ambition_combat::ledge_trump                      beside the rule it enforces
ambition_combat::attack_support                   604 lines following the melee
                                                  path that left for ::moveset
ambition_conversation::banter                     a bark is the shortest
                                                  conversation there is
ambition_sprite_sheet::ActorSpriteMetrics         it IS sprite metrics (D117)
```

⛔ **AND TWO DELETIONS BEAT ALL OF THEM.** `crate::actor` is GONE — the ruling that
kept it (*"138 lines that OWN `AncillaryMovementBundle`"*) died with its premise
once the bundle moved to `shared_tangle::body`, where nineteen of its twenty fields
already lived. And `character_runtime` stopped republishing NINE
`ambition_characters::prepared` names across ~250 sites, 57 of them outside this
crate — which is exactly how a census reads the monolith as their owner.
⇒ `features/ecs/actor_clusters.rs` (1,409 lines) now names ZERO monolith paths in
any of the three shapes. ⚠ that is evidence it is the CENTRE, not that it is next:
no existing crate's subject is *"actor construction and the body-cluster query
aggregate"*, and minting one to receive it is Wave G's question, not a carve.

✔✔✔ **THE CARVE LANDED 2026-08-26. `crates/ambition_damage` EXISTS**, and it was
a MOVE rather than a design, because the day's five facade deletions had already
taken the module's outward edges from an apparent ~70 to zero.

```text
damage_apply.rs → ambition_damage/src/lib.rs     1,389 lines
damage_apply/tests.rs → src/tests.rs             47 arms, and they TRAVELLED —
                                                 unlike mount's, they needed no
                                                 construction road
inward refs                                      42, a rename (Wave E's rule)
re-exports from the new crate                    NONE; its own three were deleted
                                                 in the same commit
footprint                                        42 → 43 crates, 15 → 16
                                                 never-asked-for, DECLARED
```

⭐⭐ **AND THE CARVE MADE TWO THINGS SAY THEMSELVES THAT THE MONOLITH HAD HIDDEN:**
`ambition_damage` names `ambition_persistence`, because the assist-mode damage
scale reads gameplay settings — a damage resolver depending on a settings store,
free to notice only once the crate had to list its own deps. And
`resolved_body_knockback_velocity` turned out to have NO production caller:
inside a 95k-line crate `pub(crate)` hid that, and in a small one the compiler
says it.


⭐⭐ **`features/ecs/damage_apply` COSTED AT HEAD, 2026-08-26 — and the facade
deletion above is most of why it is now the candidate.** 3,705 lines, and the
PRODUCTION census (tests excluded, counting the PATHS it names and not the `use`
lines — Wave H's lesson) is **seven real outward items**:

```text
crate::actor::{BodyAnimFacts, PrimaryPlayerOnly}   NOT an edge — `ambition_characters`
                                                   and `shared_tangle::markers`
crate::features::MotionModel                       a SIBLING in `features::ecs`;
                                                   it travels with any carve of it
crate::avatar::PlayerSafetyState             ⎫
crate::{SafePositionContext,                 ⎬ ONE MECHANIC — four of the seven
        remember_safe_player_position,       ⎪ (see below)
        RoomTransitionCooldown}              ⎭
crate::avatar::PlayerBodyFrameOutput               real
crate::world::overlay                              real
crate::ActorDiedMessage                            real; a message it WRITES
```

⛔⛔ **AND MY OWN FIRST COUNT MISSED FOUR OF THEM, THE SAME WAY WAVE H's DID.** A
regex for `crate::x::Y` does not match `use crate::{a, b, c}` — four crate-ROOT
items imported in one brace. ⇒ **a census must read the IMPORT BLOCK as well as
the body**, because the root is exactly where a monolith parks things.

⭐⭐ **FOUR OF THE SEVEN ARE ONE THING: the last-safe-player-position memory.**
`SafePositionContext` gates the write, `remember_safe_player_position` performs
it, `RoomTransitionCooldown` is one of its gates, and `PlayerSafetyState` is
where it lands. Three of those four are parked at the MONOLITH ROOT — a
mechanic wearing the crate's address, which is exactly the shape `Mass` had
before the mount carve.

⇒ **the removal that unblocks this carve is that mechanic, not the carve
itself.** Its only monolith-owned input is `PlayerSafetyState`; everything else
in `remember_safe_player_position`'s signature is `_core`. Its consumers are the
runtime's room transition and reset (`sandbox_reset`, `room_transition/commit`,
`sim_core_resources`) and the monolith's damage road — two domains, neither able
to own it without the wrong edge, which is `shared_tangle`'s own admission test.

✔✔ **BOTH REMOVALS LANDED 2026-08-26, AND `damage_apply` IS DOWN FROM SEVEN
OUTWARD ITEMS TO TWO.** The safe-position memory went to
`shared_tangle::safe_position` and the actor-death announcement to
`ambition_combat::death_rules`. What is left:

```text
crate::avatar::PlayerBodyFrameOutput               THE ONE REAL EDGE — one query
crate::world::overlay::FeatureWorldOverlaySet      a DOC LINK; not code at all
crate::actor::{PrimaryPlayerOnly, BodyAnimFacts}   not edges
```

⛔⛔ **AND `crate::features::MotionModel` WAS A THIRD FACADE — 125 SITES.** It is
`ambition_platformer2d_core::movement`'s type, re-exported TWICE through the
monolith (`features` → `features::ecs::actors::motion` → `_core`), and
`MomentumMotion` was the same thing RENAMED from `SurfaceMomentumMotion` on the
way, so a caller could not grep the real type. ⭐ `damage_apply` spelled the SAME
type both ways in one file — `ae::MotionModel` twice and
`crate::features::MotionModel` twice — which is the tell a facade leaves.

⇒ **`damage_apply` NAMES NO MONOLITH TYPE AT ALL — production OR tests.** The
last edge was a SYSTEM, not a type: `publish_kernel_reset_death` read the
avatar's own `PlayerBodyFrameOutput` and moved there with its regression. Its
TESTS reached the monolith for exactly two names, and both were a FIFTH facade —
`pub use ambition_combat::events` re-exported up to `features`, 74 sites.

```text
what is left        crate::world::overlay::FeatureWorldOverlaySet — a DOC LINK
```

⭐⭐ **FIVE FACADES IN ONE DAY, AND EVERY ONE OF THEM INFLATED THE CENSUS THAT
PICKS THE NEXT CARVE:** `crate::physics`, `pub use ambition_combat as combat`,
`pub use ambition_projectiles::*`, `MotionModel` (a `_core` type re-exported
TWICE, and `MomentumMotion` renamed on the way so it could not be grepped), and
`features::events`. ⇒ **the monolith looked coupled to itself.** `damage_apply`
went from an apparent ~70 references to zero without a single system being
rewritten.

⛔ **THE TELL, EVERY TIME: ONE MODULE SPELLING ONE TYPE TWO WAYS.**

✔✔ **EIGHT MORE LEFT `features/mod.rs` ON 2026-08-26, AND FOUR HAD NO CONSUMER
AT ALL.** Six were whole MODULES forwarded under the monolith's address —
`events`, `hazard_runtime as hazards`, `path_motion`, `util`,
`collision as world_overlay`, `effect_bus as bus` — plus `PathMotion` and
`HazardRuntime` as bare types. `hazards`, `path_motion`, `world_overlay` and
`bus` were named by nothing in the tree.

⛔⛔ **AND THE GREP THAT FOUND FOUR OF THEM WAS WRONG ABOUT THE OTHER FOUR.**
`grep -rn 'features::util'` returns ONE hit and it is a doc comment; the real
consumers reach the facade through `use super::*` and
`use super::super::util::midpoint` — RELATIVE paths that no search for the
declared spelling can match. ⇒ **for a facade, the compiler is the census**:
delete the `pub use` and read the errors. This is the same lesson as the
import-block one above wearing a different hat — a name is not reliably spelled
the way it was declared.

✔ **BOTH OF `movement_fx`'s EDGES DELETED 2026-08-26** — the `BodyAnimFacts`
facade use (the file already named `ambition_characters::actor` directly for
`BodyCombat`) and `BLINK_IN_ANIM_TIME`, a root constant with exactly one consumer
in the whole tree. 1,116 lines and it says `crate::` nowhere. ⚠ ITS CONSUMERS
ARE THE MONOLITH'S OWN TICK AND THE APP's phase list — one of them is the crate
it lives in — so `shared_tangle`'s two-domain admission test does NOT pass and a
new crate would relocate lines rather than remove a representation. ⇒ PARKED as
carve-ready, not carved.

⭐⭐ **AND THE REAL PRIZE THE FACADES WERE HIDING: `ambition_sim_view`'s
DEPENDENCY ON THE MONOLITH IS 20/24 FACADE.** Of the 24 names it imported from
`actor_monolith`, **twenty** are re-exports of `ambition_combat` (13),
`ambition_platformer2d_core` (4), `shared_tangle` (2), `ambition_geometry` (1) and
`ambition_boss_encounter` (1). All twenty now name their owning crate.

⭐⭐ **ITEMS IS THE NEXT BLOCK AND IT IS NOT A FREE CARVE — CENSUSED 2026-08-27
AFTER DELETING ITS FACADE USES.** `items/` is 5,466 lines. Its apparent outward
coupling was ~40 references; repointing every FACADE use at the crate that owns
it (markers, `FeatureSimEntity`, `FeatureId`, `CenteredAabb`, `BodyKinematics`,
`HeldItem`, `BreakableFeature`, `ActorDisposition`, `DamageableVolumes`,
`FeatureEcsWorldOverlay`, `BossAnimationFrameSample`, and BOTH
`platformer_runtime::prelude` and `::collision`, which are re-exports of
`shared_tangle::prelude` and `_core::cast`) leaves this:

```text
crate::abilities          17   ranged · traversal · thrown   ← the real weight
crate::features            5   what is LEFT after the facades
crate::character_runtime   5   ActiveMatch · PreparedMatch · MatchItemSpawns · live clock
crate::construction        2 · crate::session 2 · crate::shrine 2
crate::world_facts 1 · crate::schedule 1 · crate::ability_cooldown 1
```

⇒ **items is genuinely the monolith's**, unlike `damage_apply` — which the same
treatment took from ~70 to zero. ⛔ do NOT price it as a move; the abilities
coupling alone is a design question about who owns a thrown/ranged item's verb.
⚠ THE FACADE DELETION IS STILL WORTH IT ON ITS OWN: the census is now honest, and
the next session argues with 35 real references instead of 40 mixed ones.

⛔⛔⛔ **AND THE "FOUR TYPES" GATE BELOW IS WRONG — CORRECTED 2026-08-27 BY THE
SESSION THAT WROTE IT.** That measurement read only the `features::{…}` brace
imports in `anim_index.rs` and `view_index.rs`. `ambition_sim_view` also names the
monolith from `facts.rs` (held items, ground items, item custody, world items,
item motion, held projectiles), `local_view.rs` (`control::body_driving_seat`) and
`lib.rs` (`features::advance_actor_anim_overlays`). ⇒ **the edge is smaller than
it was and is nowhere near closing**; ITEMS are the next real block, not a
leftover type. ⚠ THE LESSON IS THE ONE THIS ROW KEEPS TEACHING: a census that
reads one import FORM measures that form, and `grep` for a brace import cannot see
a fully-qualified path in a query type.

✔ **`ActorConfig` + `ActorTuning` + `BrainProfile` MOVED ANYWAY, 2026-08-27**, to
`ambition_combat::actor_tuning` — 47 files changed a path. ⛔ AND THE OBVIOUS
DESTINATION WAS WRONG: `ambition_characters` owns "what a character IS", and
`ActorTuning.movement` is an `ambition_combat::BodyMovementTuning`, so the floor
crate cannot name it. **A type follows the vocabulary it is built from** — the
same rule that sent switches to `ambition_encounter`.

⛔ **THE DELETION GATE AS ORIGINALLY (AND WRONGLY) STATED:**

```text
✔ SwitchFeature · SwitchOn    MOVED to ambition_encounter::switches 2026-08-26
▢ ActorConfig · ActorStatus   features/ecs/actor_clusters.rs
▢ configure_platformer2d_simulation_phases   schedule/ — TEST-only reach
```

⇒ move the last two and `ambition_sim_view` can drop
`ambition_platformer2d_actor_monolith` from its `Cargo.toml` outright — a CRATE
EDGE deleted, which is a different and larger thing than a carve.

✔ **THE SWITCH HALF LANDED, AND ITS DESTINATION WAS NEVER A JUDGEMENT.**
`SwitchFeature` and `SwitchActivated` carry `ambition_encounter::SwitchActivation`,
so the vocabulary named the crate. Five rollback declarations travelled with the
types — the obligation nothing enforces, because a declaration compiles fine in
the crate its type just left — and the stable names did not change, so the wire
did not move.

⛔⛔ **`rebuild_encounter_switch_index` STAYED BEHIND, and that is the reusable
half.** It reads a switch's `FeatureId`, which is `ambition_combat` vocabulary
`ambition_encounter` does not link; taking the system would have bought a CRATE
EDGE to carry one field read. ⇒ **a move splits at the vocabulary boundary, not
at the file boundary** — the types went, the query stayed, and each is now beside
what it is written in terms of.

⚠ **AND ONLY ONE LEDGER NOTICED.** `rollback_exit_oracle`'s presence-probe list
keys on FULL type paths and its failure was the single thing in the tree that saw
`SwitchActivationQueue` move; the schema baselines key on STABLE NAMES and were
silent, correctly.

⇒ **`ActorConfig` IS A CAMPAIGN, NOT A STEP: 127 refs across 45 files**, and it
is built from `features::ecs::actor_tuning::{ActorTuning, BrainProfile}`, so the
move drags those two with it. `ActorStatus` is the cheap one — 23 refs, and its
only vocabulary is `ambition_characters::actor::ai::CharacterAiMode`, which names
its destination.

⭐ **NEXT CANDIDATE MEASURED AT HEAD: `features/movement_fx.rs`, 1,108 lines and
TWO outward items, both removable.** `crate::actor::BodyAnimFacts` is a re-export
of `ambition_characters::actor` — which the file ALREADY imports directly for
`BodyCombat`, so it is the tell again — and `crate::BLINK_IN_ANIM_TIME` is one
`f32` at the monolith root whose only consumer in the whole tree is this file.
⚠ ITS DESTINATION IS THE OPEN QUESTION, not its coupling: it needs `ambition_vfx`,
`ambition_sfx`, `ambition_characters` AND `shared_tangle::camera_ease`, and
`ambition_characters` has already REFUSED the shared_tangle edge in its own
`Cargo.toml` (*"the edge was LEGAL… and legality is not the same question as
floor"*). So it is a new crate or it is shared_tangle, and shared_tangle's
admission test wants two consuming domains.
`damage_apply` held `ae::MotionModel` twice and `crate::features::MotionModel`
twice. Grep a suspect module for a type it names by two paths before designing
anything.

⛔⛔ **AND THE SECOND MOVE COULD NOT GO WHERE THE FIRST ONE DID, which is the
lesson worth more than either move.** The reflex after a successful move is to
send the next thing to the same place — but `DeathCause` is built from
`ambition_combat::HitSource`, and `shared_tangle` does not depend on combat.
Sending it there would have pointed a FOUNDATIONAL crate upward at a domain,
which is the edge shared_tangle exists to prevent. ⇒ **a destination follows the
VOCABULARY the type is built from, not the last place a move succeeded.**

⛔ **PRICE, so the next session does not re-derive it:** `PlayerSafetyState` and
`RoomTransitionCooldown` are BOTH rollback-registered and encoded, so the move
touches the path-keyed ledgers (registration turbofish, exit oracle, the JSON
baseline's `encoded_types`, the `SnapshotState` impls the orphan rule relocates).
Stable names must NOT move — it is a repoint, not a schema change. ~10 files.


✔✔✔ **2026-08-26, AFTER THE MOUNT CARVE: THE MONOLITH STOPPED REPUBLISHING ITS
PEERS, AND THE NEXT CANDIDATE CHANGED.** Three re-export facades were deleted —
`crate::physics` (a 15-line pure forward to `shared_tangle::{gravity,frame_env}`),
`pub use ambition_combat as combat` (a whole-crate alias on the PUBLIC surface,
carrying its own `TODO(compat-remove)`), and `pub use ambition_projectiles::*`
(a glob, split by NAME because that module also owns real code). 565 internal
sites and 64 external ones now name a real owner.

⛔⛔ **THE REASON THIS IS A DEFECT AND NOT TIDYING: IT WAS BREAKING THE CENSUS
THAT PICKS THE NEXT CARVE.** `damage_apply` measured 61 `crate::combat::`
references and read as deeply coupled; every one of them was `ambition_combat`,
which the three crates reaching through the monolith ALREADY DEPENDED ON
DIRECTLY. Re-measured with the facades gone:

```text
                lines   outward refs        was
damage_apply     3674          17      read as ~70
perception       1580          13
aggression        860          13
bosses           1852          36
actors           3601          43
damage           4399         120
spawn            3352         154
```

⇒ **`damage_apply` is the strongest candidate left: 3,674 lines against SEVENTEEN
outward references.** Mount carved at 1,871 against four.

⚠ **AND `crate::actor` IS NOT THE SAME DEFECT — MEASURED, NOT ASSUMED.** It is
138 lines that OWN `AncillaryMovementBundle` (110 of them) and re-export 27
vocabulary names from FOUR crates, so it composes something a single alias does
not. It stays. But a census cannot tell composition from republication, so the
resolution is written down ONCE here instead of re-derived per carve:

```text
crate::actor::BodyKinematics                    crate::platformer_runtime::body
crate::actor::{PlayerEntity, PrimaryPlayer,     shared_tangle::markers
               PrimaryPlayerOnly}
crate::actor::BodyAnimFacts                     ambition_characters::actor
crate::actor::BodyMelee                         ambition_combat
crate::actor::Body*  (19 movement clusters)     ambition_platformer2d_core
```

⇒ of `damage_apply`'s seventeen, the four `crate::actor::` refs resolve to
`_core` / `shared_tangle` / `ambition_characters` — already below the monolith.
**Its real outward edge count is thirteen.**

⛔ **NOT A DEFECT — the SDK facade.** `ambition_platformer2d` re-exporting
`ambition_combat as combat` is one hop and `architecture.md` sanctions it. The
rule is about WHO republishes: a curated SDK may, a domain crate republishing a
peer domain may not.


⭐⭐ **A COSTED CARVE CANDIDATE, MEASURED AND THEN UNBLOCKED 2026-08-26 — this
row asked for exactly this: design work backed by measurement rather than another
coupling table.** ⇒ **`ecs/mount/mod.rs` now names ZERO monolith paths**; four
outward edges were removed in one day and the remaining work is its test
fixtures. The remaining mass is `features` (44k), and 34k of it is `features/ecs`.
Its subtrees:

```text
ecs/damage        4400      ecs/mount        1871
ecs/actors        3601      ecs/bosses       1852
ecs/spawn         3352      ecs/perception    765
ecs/damage_apply  2253      ecs/aggression    646
```

⇒ **`ecs/mount` is the one with a real boundary**, and it is the only subtree
whose dependence on the rest of the monolith is countable on one hand:

```text
what mount IMPORTS from the monolith    ONE item, after 2026-08-26 —
                                        `brain_builders::dismounted_rider_
                                        brain_and_action_set`. `CenteredAabb`
                                        was reached through this crate's
                                        re-export chain and is now named from
                                        its real owner (`ambition_geometry` via
                                        `_core`, which mount already imports)
`crate::` references inside mount        4
module-path references TO mount          3 (2 in `ecs/mod.rs`, 1 in a test)
everything else it uses                  already in OTHER crates —
                                         `ambition_characters`, `_core`,
                                         `shared_tangle`, `ambition_boss_encounter`
```

⛔⛔ **AND THE FIRST THING A MOUNT CARVE MUST RESOLVE IS THAT `Mass` LIVES
THERE.** `pub struct Mass(pub f32)` is defined in `mount/mod.rs` with **27
external users** — a generic physics fact parked inside one mechanic. A carve
either drags it out or exports it from a mount crate, and the second is wrong.
⇒ move `Mass` first, on its own, and the carve gets smaller and honest.

✔✔ **DONE 2026-08-26 — and the module's own comment had already written the
rule.** `MountDied` sits in `shared_tangle::body` *"below both of the domains
that share it"*, and is *"imported, never re-exported: a `pub use` here would let
a caller keep spelling it `features::MountDied` and hide whose type it is."*
`Mass` is the same shape — writer is the character runtime's physical baseline,
reader is the mount pair's mass-weighted centre — so it moved to the same place
by the same rule, imported and not re-exported.

⭐ **THE DELETION IS THE PROOF, and the compiler named it**: two re-export lines
(`features/ecs/mod.rs`, `features/mod.rs`) had to go, and every caller now spells
the real owner instead of `crate::features::Mass`.

⛔⛔ **AND IT TOUCHED ALL THREE PATH-KEYED LEDGERS, exactly as that trap
predicts.** (1) the registration turbofish in `rollback_registration.rs`; (2) the
rollback EXIT ORACLE, which keys its immutable-component list by the FULL TYPE
PATH as a string (`rollback_exit_oracle.rs:460`) — invisible to the compiler and
red only when run; (3) the schema baseline, which did NOT change and must not:
⭐ **the stable name stays `mount.mass` though the type left `mount`.** It is an
identity on the wire, not an address, and renaming it would be a declared schema
change bought for tidiness.

✔✔ **DONE 2026-08-26 — THE MOUNT MODULE NO LONGER CALLS THE BRAIN BUILDER, and
`MountDied` was the precedent three lines above the import.** Mount already
ANNOUNCED the `(dead-mount, still-mounted)` dissolution and let another domain
react — `ambition_boss_encounter` turns it into a rider boss's
`External("mount_died")` phase. `rebuild_dismounted_rider_brains` now takes the
same road: mount announces, the brain builder answers. ⭐ **THE MESSAGE COST
NOTHING — it was already being written, for the boss bridge.** The reactor reads
`ActorConfig`, `HeldItem`, `CombatKit` and the prepared cast straight off the
rider entity, so nothing had to be threaded through the announcement.

⭐ **THE DELETION IS THE PROOF, and it is bigger than the one import.** Out of
`enforce_mount_rider_link` went the `use`, the `PreparedCharacterRegistry`
resource, three query columns (`HeldItem`, `CombatKit`, `BossConfig`) and the
rebuild block — **and with them the last `Res` in the file**: the dissolution
system takes no resources at all now.

⛔ **BUT THE ROW'S "imports nothing from the monolith at all" WAS OPTIMISTIC, and
the honest remainder was three things**, all inline paths rather than `use` lines,
which is how the earlier count missed them — ⭐ **and all three are now resolved,
2026-08-26:**

```text
crate::physics::ResolvedMotionFrame           ✔ ALREADY IN shared_tangle
                                                (`frame_env`) — mount was reading
                                                it through a monolith re-export,
                                                exactly the `CenteredAabb` case
super::actor_clusters::ActorClusterQueryData  ✔ NOT NEEDED — see below
crate::features::TemporaryControl             ✔ MOVED to shared_tangle
```

⭐⭐ **THE VIEW WAS THE COUPLING, NOT THE DATA — AND THE CENSUS IS THE WHOLE
ARGUMENT.** `ActorClusterQueryData` is **twenty-six columns wide**. Mount touches
**five**, and four of the five are already owned by other crates:

```text
kin       BodyKinematics       ambition_platformer2d_core
surface   ActorSurfaceState    ambition_platformer2d_core
ground    BodyGroundState      ambition_platformer2d_core
health    BodyHealth           ambition_characters
config    ActorConfig          ⛔ THE MONOLITH — and TWO FIELDS of it
```

⇒ both mount systems name their columns one by one now. `sync_riders_to_mounts`
needs no `config` at all, so **it names no monolith type whatsoever**; the
enforcer keeps `ActorConfig` alone.

⛔ **AND THE OBVIOUS SHORTCUT IS WRONG, MEASURED RATHER THAN ASSUMED.** The two
`ActorConfig` fields look like they have live twins already on the entity —
`spawn.size` beside `BodyBaseSize`, `tuning.is_aerial` beside
`BodyFlightState::fly_enabled` (`actor_clusters.rs:164` literally assigns it).
Both twins are WRITTEN AT RUNTIME: `posed_body.rs:127` moves `base_size` with the
STANCE, `body_clusters.rs:1290` grows it, and `fly_enabled` is toggled by
`abilities.rs:445` and constrained by `movement::authority`. ⇒ substituting them
would hand a grown rider, or a rider that had switched flight off, the WRONG BODY
on dismount. `ActorConfig` is the AUTHORED BASELINE and the live components are
not it.

✔✔ **DONE 2026-08-26 — `SpawnBaseline` LANDED, AND `ecs/mount/mod.rs` NOW NAMES
ZERO MONOLITH PATHS.** `ActorSpawnState` (`{pos, size}`, a plain field of
`ActorConfig` in `features/enemies`) became
`shared_tangle::body::SpawnBaseline { pos, size, gravity_scale }` — its own
component, beside `Mass` and `MountDied`, imported and never re-exported.

⭐⭐ **THE THIRD FIELD IS THE POINT.** The authored gravity scale was
hand-derived from `tuning.is_aerial` at THREE sites — spawn, `reset_to_spawn`,
and the mount dismount. It is recorded once now, where the body is built, and
the other two READ it. ⇒ the dismount no longer needs `ActorConfig` at all, and
two representations of one authored fact became one.

⛔⛔ **WHAT MADE IT WORTH DOING WAS BIGGER THAN THE CARVE: THE AUTHORED GRAVITY
SCALE WAS RE-DERIVED FROM `is_aerial` AT THREE SITES, BY HAND, EACH TIME.**

```text
actor_clusters.rs:847   gravity_scale = if is_aerial {0.0} else {1.0}   spawn
integration.rs:562      gravity_scale = if is_aerial {0.0} else {1.0}   reset_to_spawn
mount/mod.rs            gravity_scale = if is_aerial {0.0} else {1.0}   the dismount
```

⇒ **two representations of one authored fact, agreeing only by convention** —
the shape that cannot be attributed when one of them is wrong. ⚠ and the OTHER
construction path spells it the long way round (`actor_clusters.rs:523-528`
computes `gravity_scale` from the authored boolean and then recovers
`is_aerial = gravity_scale <= 0.001` from it), which is the same convention
travelling in the opposite direction. The authority underneath all of it is the
character's own `locomotion.baseline_free_flight`; everything after that is a
projection.

⇒ **naming the authored baseline ONCE was the fix, and the mount carve is only its
first customer.** ⛔ THE LIVE COMPONENTS COULD NOT BE THAT NAME: mount ZEROES
`gravity_scale` while riding, so the live value is the one thing it definitely is
not. The durable component cost one new stable schema name
(`actor.spawn_baseline`, `component-clone`, immutable after spawn like
`actor.config` beside it) and a declared bump to
`GGRS_ROLLBACK_SCHEMA_VERSION` **113** — cheap next to three sites agreeing by
convention.

⛔⛔ **AND THE CARVE IS STILL NOT A `git mv`, BECAUSE THE TESTS ARE NOT FREE.**
`mount/mod.rs` names nothing; `mount/tests.rs` names **41 monolith paths** —
15 `crate::features::`, 7 `super::super::actor_clusters::`, plus the combat
moveset, the boss attack systems and the character roster. They build real
riders through the monolith's construction road, which is exactly what a mount
crate would not have. ⇒ **the next step is the TEST fixtures, not the module.**

⭐ **AND THE SAME CHANGE THAT FREED THE MODULE MAKES THE FIXTURES TRACTABLE.**
They call `ActorClusterSeed::new(...)` — the monolith's whole construction road —
because the old query view demanded all twenty-six columns. Mount now reads
**nine components in total**: `RidingOn`, `Mounted`/`MountSlot`, `Mountable`,
`CenteredAabb`, `Mass`, `ResolvedMotionFrame`, `BodyKinematics`,
`ActorSurfaceState`/`BodyGroundState`, `BodyHealth` and `SpawnBaseline`. A rider
fixture is those spawned directly, and none of them is a monolith type. ⇒ the
fixtures CAN be rewritten rather than relocated.

✔✔✔ **THE CARVE LANDED 2026-08-26. `crates/ambition_mount` EXISTS.** Every price
below was paid as measured, and two things came out better than predicted:

```text
mount/mod.rs → ambition_mount/src/lib.rs    631 lines, four dependencies
                                            (bevy · _core · characters · shared_tangle)
the 15 arms                                 STAYED, as the monolith's
                                            `features::ecs::mount_pair_tests` —
                                            their fixtures build real riders
                                            through the construction road, so
                                            they test the carved crate FROM THE
                                            COMPOSITION. 1194 passed, unchanged.
re-exports                                  NONE. `features::MountSlot` is gone;
                                            the facade exposes `platformer2d::mount`
                                            as its own domain beside boss_encounter
the 106 inward refs                         a rename, exactly as Wave E predicted
```

⛔ **THE FOOTPRINT GREW BY ONE AND IS DECLARED, NOT WAIVED** —
`capability-footprint-baseline.json` now reads 42 crates / 15 never-asked-for,
with the reason beside it. ⭐ **AND THE HONEST FRAMING IS IN THAT REASON**: the
same code was linked before, inside the monolith; what changed is that it became
COUNTABLE. `ambition_boss_encounter` sits in the list for the same reason, and
the way out is the same for both and neither has it — **the closure should follow
the plugin a game INSTALLS, not the dependency its crate declares.**

⚠ **`ambition_mount` ships with no tests of its own**, stated in its own header.
The nine component-level arms are still owed; see the classification below.

⭐⭐ **THE CARVE'S FULL PRICE, MEASURED 2026-08-26 — and the estimate below was
short by one ledger row, found only by RUNNING `app_it`:**

```text
MOVES        mount/mod.rs        615 lines, ZERO monolith paths
             mount/tests.rs       9 of 15 arms (classified below)
INWARD       106 references to mount types inside the monolith, outside the
             module — and per Wave E's rule these are a RENAME, not a blocker:
               construction/mod.rs + tests   65   the authored road that BUILDS pairs
               features/ecs/spawn_actors.rs  11
               rollback_registration.rs       9
               everything else               21   (≤5 per file)
LEDGERS      8 rollback registrations by path + 4 exit-oracle rows (the estimate
             said 3); stable names UNCHANGED, so it is a repoint and NOT a
             schema change — `GGRS_ROLLBACK_SCHEMA_VERSION` deliberately stands.
             ⛔⛔ THE ORACLE ROW WENT RED WITH THE APP GATE ALREADY GREEN AND
             THE MONOLITH'S 1194 ALREADY GREEN: `rollback_exit_oracle.rs` keys
             its presence-only list on the FULL type path, and only
             `cargo test -p ambition_app --test app_it` asks it. ⇒ a carve is
             not verified by the gate; the four names were CanPilot, Mountable,
             MountedBrainCache, MountedSize, and their REASONS are untouched —
             a move does not change whether a payload is authored-and-immutable.
GATE         `capability-footprint-may-not-grow` GROWS BY ONE — the monolith's
             construction road depends on the new crate, so every consumer links
             it. ⭐ PRECEDENTED: `ambition_boss_encounter` did exactly this and
             sits in the "14 a movement-only game never asked for" list today.
             Declare it in the baseline with the same reason.
LOCKFILES    a new dep edge fails the contracts job until BOTH `Cargo.lock` and
             `fixtures/minimal_game/Cargo.lock` are refreshed
```

⇒ **the shape is the BOSS carve's**: the new crate owns the RULES (two systems +
the components), the monolith keeps CONSTRUCTION, which names those components to
build pairs. ⛔ do not try to take construction with it — that is the actor
spawn road, and it is the thing the monolith IS.

⭐ **AND THE 15 ARMS IN `mount/tests.rs` CLASSIFY CLEANLY — done 2026-08-26, so
the move is a sorting job rather than a judgement call:**

```text
TRAVEL (mount RULES, expressible in components)     9
  sync_riders_to_mounts_snaps_rider_to_mount_offset · dead_mount_dissolves_link_
  keeping_records · can_pilot_matches_authored_classes · mountable_defaults_are_
  total_control_and_clean_dismount · nonlethal_mount_death_splash_damages_the_
  rider_then_dismounts · lethal_mount_death_splash_kills_the_rider ·
  dismount_impact_leaves_rider_hp_intact · reviving_mount_re_arms_rider_to_
  mounted_brain · dead_rider_does_not_disturb_mount_records

STAY (cross-domain, and the monolith is where the other domain is)   6
  boss_rider_keeps_its_brain_… (BossConfig) · total_grant_routes_rider_
  locomotion_to_mount_but_not_fire and a_player_controlled_rider_pilots_the_mount_
  agnostically (the control seam) · the three gnu_ton arms (limbs, possession,
  the whole bridge)
```

⛔ **DO NOT WRITE THE NINE COMPONENT-LEVEL FIXTURES AHEAD OF THE MOVE.** They
would duplicate coverage that is green today, and a duplicate arm is a second
thing to keep in step. Write them AS the crate moves, where the compiler is
already forcing every line to be revisited.

⚠ **BUT CHEAPER IS NOT AUTOMATICALLY RIGHT, and this is the call to make
deliberately.** Today those arms build riders through the REAL construction road,
so they also prove a real pirate raider ends up mountable. A hand-spawned
component bag proves the mount rules and nothing about the bodies the game
actually makes. ⇒ the likely answer is BOTH: narrow component-level arms travel
with the carved crate, and the construction-road arms STAY in the monolith as
integration tests of the pair. Do not delete the second kind to make the first
kind possible.

⭐ **AND `TemporaryControl` WAS THE `Mass` SHAPE FOR THE THIRD TIME.** Fifty
lines, one dependency (`SimId`), and its own doc already said two domains share
it — *"the possessed actor / the rider"*. It moved to
`shared_tangle::temporary_control` beside `body::Mass` and `body::MountDied`,
imported and never re-exported, and the monolith's `pub use` went with it.
⛔ TWO of the three path-keyed ledgers moved (the registration turbofish, and the
`SnapshotState` impl, which the ORPHAN RULE forced into shared_tangle's
`snapshot_impls.rs` — that file's own doc predicts this: *"if a type moves, this
stops compiling rather than drifting"*). The stable name `actor.temporary_control`
did NOT change, and it is not in the exit oracle's immutable list because it is
mutated at runtime.

⛔⛔ **AND THERE WAS A FOURTH LEDGER, WHICH THE `Mass` MOVE NEVER MET.**
`scripts/baselines/rollback-schema-baseline.json` keys `encoded_types` by FULL
PATH, so `check_absence_contracts.py` reported the move as a NEW type entering
the wire format AND a STALE one leaving it. ⭐ **A MOVE IS THE ONE CASE WHERE
THAT PAIR IS NOT A WIRE CHANGE**: identical bytes, identical stable name, so
peers still agree — the baseline is repointed and `GGRS_ROLLBACK_SCHEMA_VERSION`
deliberately is NOT bumped. `Mass` escaped this ledger only because it is
registered `component-clone` rather than encoded.

⇒ **so a moved registered type touches THREE ledgers, or FOUR if it is ENCODED.**

⛔⛔ **AND MAKING IT TWO SYSTEMS OPENED A WIRING GAP THAT NEEDED ITS OWN ARM.**
Every mount unit test hand-lists its systems, so all 37 keep passing if the
reactor is dropped from `CombatSchedulePlugin` — a hand-listed chain pins the
FUNCTION, not the WIRING. ⇒
`a_dead_mount_rebuilds_its_riders_brain_through_the_real_schedule` drives the
REAL headless sim; poisoned by removing the reactor from the plugin it goes red
while the 37 stay green, which is exactly the gap it is for.

⭐ **AND THE ARM FOUND THE BUILDER'S OTHER BRANCH.** A pirate raider carries a
gun-sword, so a fallen raider gets the RANGED brain, not `MeleeBrute` — the
builder's own rule (*"keep the weapon live after the shark dies"*), which no test
had reached because the unit fixture holds no ranged item.

⭐ **A SECOND ARM WAS ALREADY VACUOUS AND IS NOT ANY MORE.**
`boss_rider_keeps_its_brain_and_emits_mount_died_on_dismount` asserts a BOSS
rider is REFUSED a rebuild — and once the rebuild left the enforcer, that arm
registered no system that rebuilds ANY rider's brain, so it would have passed on
a world where the rule did not exist. It registers the pair now, and deleting the
`BossConfig` skip turns it red.

⚠ **AND THE COUPLING IS NOT ONLY WHAT THE MODULE IMPORTS — the ledger names it
too.** `rollback_registration.rs` registers EIGHT mount types by
`crate::features::…` path (`can_pilot`, `mass`, `mount_slot` + its entity map,
`mountable`, `mounted`, `riding_on` + its map, `brain_cache`), so a carve moves
path-keyed rows in a registration ledger as well as code — the same three-ledger
cost a moved registered type always pays. ⇒ **that is the real price, and it is
still the smallest one on offer inside `features`.**

Use [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md).
Choose carves from current dependency and authority measurements, not an old LOC
target.

⛔⛔ **RE-MEASURED 2026-08-26, AND BOTH INSTRUMENTS ARE EXHAUSTED. THE EASY
CARVES ARE DONE.**

```text
module            lines   →out   in←   coupling/1k
time                805      1     1      2.5   ⛔ placement-REFUSED (below)
dev                1974     10     0      5.1   ⛔ placement-REFUSED
body_mode           827      6     1      8.5
gravity             561     13     0     23.2   ⛔ placement-REFUSED
…
character_runtime 12970     80   300     29.3
features          44265    368   419     17.8
enemy_projectile   1199    108     5     94.2   ⛔ worst — do not start here
```

⇒ **EVERY LOW-COUPLING MODULE IS LOW-COUPLED BECAUSE IT IS CORRECTLY PLACED, and
each refusal is written in the CODE rather than inferred:**

* `dev/trace` — `ambition_dev_tools`' own *"What stays elsewhere"* names it: the
  recorder samples live sim state and stays sim-side.
* `gravity` — `physics.rs` says the gravity MECHANIC (`GravityFlipSwitch`, the
  room-reset, the plugin, the visuals) *"stays sandbox-side"*; the frame math
  already left, to `ambition_platformer2d_shared_tangle::gravity`.
* `time` — its own header: *"Clock arbitration lives in
  `ambition_time::time_control`; this module emits GAME-POLICY requests"*. 313
  lines of production code and 492 of tests.

⇒ ⭐ **AND THE DELETION CENSUS — the instrument `affordances` taught this row to
prefer — FINDS NOTHING AT SCALE.** Every public type in the monolith, counted by
references across the whole tree: **three** are named only where they are
declared, and all three are `SystemParam` bundles or a `Local` used once in their
own file. There is no second `affordances`.

⇒ ⛔⛔ **SO A FUTURE CARVE IS DESIGN WORK, NOT A MEASUREMENT RESULT.** The
remaining mass is `features` (44k) and `character_runtime` (13k / 300 inbound),
and neither yields to a coupling number. Whoever takes this next needs a stated
OWNERSHIP boundary first — the row's own words — and should not expect the table
to nominate one.

⭐⭐ **THE PREVIOUS MEASUREMENT — RUN 2026-08-21.** The row
says *"choose carves from current dependency and authority measurements, not an
old LOC target"*, so: every top-level module of the monolith, by `crate::<module>`
references made OUT of it and made TO it from its siblings.

```text
module            lines   →out   in←   coupling/1k
dev                2000     10     0      5.0     ⭐ destination ALREADY EXISTS
affordances        1733      9     0      5.2     ⭐ nothing in the crate reads it
time               1330      2     9      8.3
body_mode           878      8     1     10.3
character_sprites  1811     12    12     13.3
…
enemy_projectile   1217    104     5     89.6     ⛔ worst — do not start here
construction       5897    232    42     46.5
```

⇒ **two candidates have ZERO inbound references**, which is the strongest signal
this measurement produces — nothing in the monolith would notice them leaving:

* ⛔ **`dev/trace` (2,000 lines) — REFUSED BY ITS DESTINATION, and I proposed it
  an hour before reading the refusal.** `ambition_dev_tools` does own *"the
  content-free half of the old `dev/` module"*, and its own **"What stays
  elsewhere"** section names this exact module: *"The gameplay `trace` recorder
  samples live sim state (`player`/`features`/`rooms`/`portal`/`game_mode`) and
  STAYS SIM-SIDE in `ambition_platformer2d_actor_monolith::dev::trace`."*
  ⇒ **not a candidate.** Zero inbound coupling made it look free; it is not
  misplaced, it is where it has to be, because it samples the sim.
* ✔✔ **`affordances` — CARVED, THEN MOSTLY DELETED (2026-08-21), and the second
  step is the lesson.** It moved to `ambition_sim_view` on the strength of a
  coupling measurement; a GPT review then asked whether anything READ the thing.
  Census: `PlayerAffordances`, `PlayerIntent`, `Aim`, `PogoTargetBelow` and the
  Attack/Jump/Shield/Dash/Special variant families are named NOWHERE outside the
  module — the only mention of the central product was its own rollback
  declaration. Only `NearestInteractable`/`InteractVariant` have a consumer (the
  portal adapter), plus the plugin and its system set (runtime, touch overlay).
  ⇒ **1,020 lines deleted**, three rollback declarations dropped,
  `GGRS_ROLLBACK_SCHEMA_VERSION` 60 → 61 because types LEAVING the schema is a
  real wire change. ⛔⛔ **a coupling measurement says a module CAN move; it says
  nothing about whether anything wants it. Count the consumers FIRST** — a carve
  is exactly how a dead subsystem earns a second life.

* ~~**`affordances`** (1,733 lines)~~ — *"what would each input do right now?"*, a
  table the HUD reads and, in its own words, *"gameplay code (today: nothing)"*.
  One consumer, zero siblings. ⚠ its natural home is the family that already owns
  the HUD read-model (`ambition_sim_view`'s `ControlPrompt`) rather than a crate
  of its own — ⛔ check that destination's stated contract first, which is the
  rule D136 yields.

⭐⭐⭐ **THE MONOLITH MEASURED TO THREE LEVELS — 2026-08-22, and it names the
subject this row has never had.** The row asks for current dependency
measurements; here is the whole tree, by production lines:

```text
module              lines   out→sibling modules      module              lines
features           44,940   441 refs / 20 modules    projectile          2,253
character_runtime  13,983    84 / 11                 encounter           2,183
avatar              7,783   172 /  9                 dev                 2,006
construction        5,895   232 /  5                 character_sprites   1,828
items               5,482    87 / 13                 control             1,676
world               5,145    50 /  9                 audio               1,377
abilities           5,109   156 / 12                 enemy_projectile    1,278
session             3,252    94 / 12                 body_mode             875
schedule            2,551    12 /  5                 time                  821
```

⇒ **`features` is 45% of the crate, and `features/ecs` is 79% of THAT** — 35,417
lines, 22,879 of them production, across 45 children. Its mass is two domains:

```text
features/ecs/…      prod    features/ecs/…      prod
actors/            3,271    damage_apply       1,396
spawn_actors       2,607    bosses/            1,214
spawn/             2,326    brain_builders     1,082
damage/            2,016    perception           754
actor_clusters     1,622    mount/               641
```

⇒ ~15,500 of 22,879 in eight children, clustering as **SPAWN** and **DAMAGE**.

⭐⭐ **AND THE DAMAGE FAMILY'S COUPLING IS RE-EXPORT LAUNDERING TOO — the same
finding as `body_mode` and `time_control`, now at scale.** `damage/` +
`damage_apply` + `damage_drops` is 3,788 production lines naming
`crate::features` 94 times and `crate::combat` 72 times. Resolved:

```text
crate::combat            = `pub use ambition_combat as combat`  ⇒ not monolith at all
crate::actor             = a 182-line re-export shell (5 `pub use ambition_*`, 1 own item)
crate::features::MotionModel / ActorSurfaceState   → ambition_platformer2d_core
crate::features::ActorFaction / BossBehaviorProfile → ambition_characters
crate::features::ActorStimulus / HitKnockback       → ambition_combat / core
crate::features::ecs (41 of the 94)                 → intra-family paths, not coupling
```

⇒ **the monolith's biggest domain is held by far less than its reference counts
suggest.** Resolving the residue to the last symbol — `BossVolumeContext` and
`BossAnimationFrameSample` are `ambition_boss_encounter`, `ActorAggression` is
`ambition_combat` — leaves **SIX monolith-owned names** holding 3,788 lines:

```text
CombatBanterRegistry      4 refs   features::banter (a 63-line module)
ActorClusterSeed          3        features::ecs::actor_clusters
damageable_volumes        2        features::damageable_volumes
actor_component_snapshot  1        features::ecs::actors
enemy_component_snapshot  1        features::ecs
(damage_apply            32        inside the family itself — not coupling)
```

⭐⭐ **AND FIVE OF THE SIX BELONG TO THE SPAWN DOMAIN**, which is the OTHER half
of `features/ecs`. `ambition_combat` already owns `hitbox`, `on_hit`,
`events::ActorStimulus` and `components::{ActorFaction, ActorAggression}`, so
damage APPLICATION belongs there by every other measure — but cluster seeds and
component snapshots are construction, and the monolith already depends on
`ambition_combat`, so exposing them upward would be a cycle.

⇒ **the next D33 slice is therefore SPAWN, not damage**, and the two move in that
order or together.

⛔⛔ **AND SPAWN CANNOT MOVE ALONE EITHER — `construction` AND SPAWN ARE MUTUALLY
DEPENDENT.** Measured to the definition site, both directions:

```text
construction (2 files, 2,197 prod)  →  features/ecs/actors/limbs.rs
                                          LimbSlot 41 · LimbRig 16 · Limb 9
                                       features/ecs/spawn/content_staging.rs
                                          RoomContentStagingRegistry 25
spawn (10,908 prod)                 →  construction/mod.rs
                                          ActorConstructionServices, …Registry,
                                          …Plan, preflight_planned_bodies  (~15)
```

⭐ everything ELSE either half names resolves outward: `crate::rooms::{RoomSpec,
EnemySpawnSpec, Authored}` are `ambition_platformer2d_world`;
`crate::character_runtime::{PreparedCharacterRegistry, CharacterDefinition,
CharacterBindings, CharacterBodyBlueprint, PreparedCharacterDefinition}` are
**`ambition_characters`** — so `character_runtime`'s 84 references from spawn
collapse to ONE owned symbol (`KitOwnership`) plus test helpers.

⇒ **THE ANSWER TO "WHICH CARVE IS POSSIBLE" IS: ONE, AND IT IS BIG.**
`construction` + spawn is ~13,100 production lines that move together because
they are one domain under two names, and damage's 3,788 follow because five of
its six holders are theirs. **~17,000 production lines, one slice, in that
order** — which is why every small carve priced today bottomed out. ⛔ do not go
looking for a smaller one inside `features/ecs`; this measurement is what says
there isn't one. ⛔ **so stop pricing carves from `crate::` counts on this
crate entirely** — three modules measured this way in one day and all three were
mostly laundering. Resolve every name to its defining crate first; the count that
matters is the one left over.

✔✔ **AND 66 OF THOSE 91 ARE GONE — THE LIMB VOCABULARY MOVED DOWN (2026-08-22).**
`LimbSlot`, `LimbRig`, `Limb`, `LimbRouteState`, `LimbIntents` and
`fan_out_limb_intents` → `ambition_characters::actor::limb`. **Zero new edges**:
the destination already had `ambition_platformer2d_core` and `bevy`, which is
everything the data half named. `limbs.rs` went 459 → 221 lines and keeps only the
half that reads a MOUNT — its kinematics, its clung surface, the `MountSlot` link
to the rider being translated.

⭐⭐ **THE TELL WAS IN THE DESTINATION'S OWN DOC, and it was a `Vec<String>`.**
`LimbRoute` — which AUTHORS which slots a strike drives — is *defined* in
`ambition_characters::brain::boss_pattern::profile` and only re-exported by
`ambition_boss_encounter`. It held `pub slots: Vec<String>` with a doc reading
*"`"hand_left"` / `"hand_right"` map to `LimbSlot` via `LimbSlot::from_route_str`;
unknown names are ignored"*. ⇒ **the crate that authors limb routes was spelling
its own vocabulary as strings for the sole purpose of crossing a crate boundary
that should not have existed** — so this was a UNIFICATION, not a relocation, and
`slots` is now `Vec<LimbSlot>`. `from_route_str` and its silent `filter_map` drop
are DELETED, and the RON authors `slots: [HandLeft, HandRight]` exactly as its
sibling `motion: SlamDown` already did. ⭐ a slot name that does not exist is now
a content LOAD error where it used to vanish without a word.

⇒ ⭐ **when a lower crate stringly-types a vocabulary, grep the string for the
converter — the converter's existence is the measurement.** Nothing else in this
row's tables would have pointed at limbs: they price COUNTS, and this edge was
visible instead as a type spelled the wrong way in the destination.

⭐⭐ **THE 26-COUNT WAS WRONG IN BOTH DIRECTIONS — re-measured 2026-08-24, and
the correction is smaller than it first looked.** Two errors cancelled into a
number that was roughly right for entirely wrong reasons:

```text
counted as coupling, is NOT:   25 of the 26 "survivors" were construction/tests.rs
missed by the same grep:        6 spawn fns construction calls through the
                                `crate::features::` RE-EXPORT, so a `features::ecs`
                                grep sees none of them
```

⇒ ⛔ **this row's own rule turned on the row.** It says to resolve every name to
its defining crate before believing a count, because three modules measured by
`crate::` counts in one day were all mostly laundering. The same measurement then
(a) counted a TEST file as domain coupling and (b) was defeated by a re-export
shell inside the very crate it was measuring. **Split production from test AND
resolve through re-exports, or the count means nothing** — a test reaching into a
sibling to build a fixture is a different fact from a domain depending on one,
and a `crate::features::` path can be a `features/ecs` symbol wearing a shorter
name.

✔ **ONE REAL EDGE DID DIE, and it is worth having.**
`spawn_static::spawn_ground_item_resolved_into` and `spawn_shrine_into` were
twenty-line component inserts filed under "spawning static things" by TOPIC while
naming `items` and `shrine` and never `spawn`, whose ONLY caller in the tree was
`construction/mod.rs`. That domain edge served nobody. ⇒ inlined at their one call
site and deleted; `construct_authored_ground_item` now constructs one instead of
delegating.

▢ **WHAT ACTUALLY HOLDS THE CYCLE, named to the file:** SIX functions in
`features/ecs/spawn_actors.rs` — `spawn_staged_actor_into`,
`spawn_runtime_minion_into`, `spawn_enemy_with_faction_into`,
`spawn_boss_with_overrides_into`, `populate_giant_host_into`,
`populate_giant_hand_into` — plus `is_limbed_host`, `giant_hand_plans` and
`GiantHandPlan` from the same file. That is ONE FILE (2,349 lines), not the spawn
domain. Asked of each the question that killed the two above — *does construction
call it because it is a spawn concern, or because that is where somebody filed
it?* — and the family splits:

```text
construction is the ONLY production caller    populate_giant_host_into
                                              populate_giant_hand_into
has real spawn-side callers too               spawn_staged_actor_into
                                              spawn_runtime_minion_into
                                              spawn_boss_with_overrides_into
                                              spawn_enemy_with_faction_into  (widest)
```

⇒ ✔ **the first two FOLLOWED the pair already deleted (2026-08-24).** Both were
thin wrappers over the shared `spawn_enemy_with_faction_into` carrying
GIANT-CREATURE knowledge inside the generic spawn file — the host is that call
plus `LimbIntents`/`LimbRouteState`, the hand is that call with an empty path
list and a non-hostile faction. Inlined at their one call site and deleted, along
with the re-export lines in BOTH shells (`features/ecs/mod.rs` and
`features/mod.rs` — the double layer is exactly what defeats a path-based count).

⇒ ⛔ **the other four are genuinely shared and STAY.** Moving them would relocate
a dependency, not remove one.

⚠ **AND TWO MORE CANDIDATES WERE MEASURED AND REFUSED, which is the part worth
keeping** — both looked construction-only under a grep that excluded the defining
file, and neither is:

```text
is_limbed_host        called FOUR times inside spawn_actors.rs itself
GiantHandPlan +       construction is the only consumer, but giant_hand_plans
giant_hand_plans        calls giant_hand_feature_id, which has NINE other users
                        there — the move swaps one edge symbol for another
```

⇒ ⛔⛔ **excluding the defining file from a reference count is the third way this
row's measurement has been fooled today** (after counting a test file and being
defeated by a re-export shell). Count IN the definition's own file; a symbol its
own module uses is not yours to take.

⇒ so the edge stands at four shared spawn functions plus the giant plan pair, and
"the cycle is broken" — which this measurement started out believing — is FALSE.
⛔ everything else construction names through `crate::features` resolves outward
(`ActorFaction` → `ambition_characters`, `ActorAggression` → `ambition_combat`)
or is the mount vocabulary, which is its own question.

⚠ **the older narrowing note, kept because its method is still right.** Applying this row's
own rule — resolve each name to its defining crate — construction's monolith-owned
references into `features/ecs` go **91 → 26**, and all 26 survivors are ONE symbol:
`RoomContentStagingRegistry` from `features/ecs/spawn/content_staging.rs`. The
reverse direction (spawn → `construction/mod.rs`, ~15) is untouched. ⇒ **the next
slice on this edge is named exactly: `RoomContentStagingRegistry`.** ⛔ do not read
the 106 `LimbSlot`/`LimbRig`/`Limb` spellings still in `construction/` as coupling —
every one of them now resolves to `ambition_characters`, which is the same
laundering trap `body_mode` paid for two entries down.

⭐ **ledger price: NO schema change, but the four-artifact table below is missing
one artifact.** All four registrations (`limb.rig`, `limb.member`,
`limb.route_state`, `limb.intents`) plus both `map.*` entity-mappings moved from
the monolith's `rollback_registration.rs` to `ambition_characters`. The `.txt`
fingerprint keys on BARE TYPE NAMES and carries no owner column, so **not one line
of it changed**, and no version bump. These are clone-only registrations with no
`SnapshotState` impls, so the orphan rule had nothing to adjudicate and
`rollback-schema-baseline.json` has no entry for them.

⛔⛔ **AND A FIFTH PATH-KEYED LIST FOUND THE MOVE THAT THE OTHER FOUR DID NOT —
`rollback_exit_oracle.rs`'s PRESENCE-PROBE WAIVER LIST.** It keys on the FULL TYPE
PATH, and it reddened with *"2 registration(s) carry a presence-only localization
probe and are not named in this test's list"* for `LimbIntents` and
`LimbRouteState` — after the gate, the monolith suite, the content suite and the
absence contracts were all green. ⇒ **add it to the artifact table**: a move
re-points its two entries, and the instrument is doing its job, not misfiring — a
waiver keyed by path cannot follow a type it does not know moved.

⚠ **and the app gate did not catch this one — another package's TEST targets did.**
`cargo check -p ambition_app --all-targets` was clean and zero-warning while
`cargo test -p ambition_platformer2d_actor_monolith` still had 24 unresolved
imports, in function-local `use crate::features::{…}` blocks a prefix grep does not
see. ⇒ a carve out of the monolith is not verified until the monolith's OWN suite
builds.

⭐ **one behaviour rule moved with the type instead of being copied.**
`LimbRouteState`'s edge memo was read and then written by the router in two
separate statements; it is now `begin_strike(active_move) -> bool`, which advances
the memo INSIDE the onset question. A caller that could read without advancing
would emit the strike edge every tick the strike is live — the exact bug the memo
exists to prevent — so the two steps are one call.

⭐⭐ **AND THE NAMED SURVIVOR IS ALREADY MEASURED: `RoomContentStagingRegistry`
NEEDS A HOME THAT DOES NOT EXIST (2026-08-22).** Resolving every name, as this row
requires:

```text
content_staging.rs (399 lines)  names, beyond std/bevy, exactly TWO things:
    crate::rooms::RoomSpec                → ambition_platformer2d_world   ✔ outward
    spawn_actors::SpawnActorRequest       → monolith-owned                ⛔ THE EDGE

SpawnActorRequest / SpawnActorKind  names, resolved to defining crate:
    ae::Vec2                              → ambition_platformer2d_core    ✔
    ActorFaction                          → ambition_characters           ✔
    BossBrain, CharacterBrain, CharacterId→ ambition_entity_catalog       ✔
    BossOverrides                         → ambition_boss_encounter       ✔
```

⇒ **the seed has ZERO monolith-owned dependencies** — it is a plain content-free
request (two `String`, two `Vec2`, a faction, an optional grudge, and a two-variant
kind). ⛔ **but no crate BELOW the monolith carries the five crates it needs.**
Exactly three carry all five and every one is disqualified by DIRECTION or domain:
`ambition_platformer2d` (the facade, above), the monolith itself, and
`ambition_sim_view` (a render read-model). The near misses are all above too —
`ambition_platformer2d_runtime` and `..._provider` each miss only
`ambition_entity_catalog` **and each already consumes the registry**, but both
DEPEND ON the monolith, so moving there inverts the edge.

⇒ ⚠ **the honest candidates below the monolith are two, and both cost something.**
`ambition_platformer2d_shared_tangle` is where the registry is already *documented*
(`construction/registry.rs` names it in a doc comment) — but it carries **none** of
the four and its own manifest says *"Foundation-only — still NEVER
ambition_platformer2d_actor_monolith"*, so it would take four new edges.
`ambition_boss_encounter` carries everything the SEED names with **zero new edges**
(it misses only `..._world`, which the seed does not need — only `content_staging`
does) — but a request that spawns ENEMIES as well as bosses does not belong in a
boss crate, and putting it there would launder the name to buy a clean graph.

⇒ **this is the `body_mode` verdict again, and it should be read the same way:
leave it until a home exists for an unrelated reason.** ⭐ but the shape is now
recorded exactly, so the next reader does not re-derive it: the blocker is NOT
coupling — the seed is already free — it is that **the five crates a content-free
spawn seed names have no common descendant.** That is an argument for a small
spawn-vocabulary crate, and it is the first time this row has produced one.

⭐⭐ **`body_mode` MEASURED, AND ITS COUPLING WAS RE-EXPORT LAUNDERING — 2026-08-22.**
The table above prices `body_mode` at 8 outbound `crate::` references. Reading
what those 8 references NAME: 11 of the 12 symbols already live outside the
monolith and are reached through its re-exports (`BodyKinematics`, `BodyModeState`,
`BodyFlightState`, `BodyJumpState`, `BodyGroundState`, `BodyEnvironmentContact`,
`BodyBaseSize`, `MotionModel` → `ambition_platformer2d_core`; `ResolvedMotionFrame`
→ `shared_tangle`; `CombatCapabilities` → `ambition_combat`). ⇒ **a `crate::` count
prices the SPELLING, not the dependency.** Resolve each name before believing a
coupling number: a module that reads only re-exports is already free.

⇒ exactly ONE symbol was genuinely monolith-owned: `SlotInteractionState`.

⛔⛔ **and one of the 12 laundered through a RENAME**, which no grep for the real
type would have caught: `crate::features::MomentumMotion` is
`pub use ae::movement::SurfaceMomentumMotion as MomentumMotion` two modules down.
⇒ resolve the name, do not pattern-match it.

⇒ **`body_mode` now has ZERO monolith-owned dependencies** (re-measured after the
move) and 2 inbound sites (`avatar/bundles.rs`, `rollback_registration.rs`). It is
unblocked. ⚠ but the consumer question is still open and D33's own rule says ask it
first: unlike `affordances` it is LIVE (the runtime schedules `update_body_mode`),
so the question is not whether anything wants it but WHICH CRATE should own a body
switching motion modes.

⭐ **AND THE ANSWER IS NARROWER THAN THE MODULE LOOKS.** Resolving what
`body_mode` actually needs, rather than what it names:

* `CombatCapabilities` is a **doc comment**, not code — one `///` line in
  `mod.rs`. There is no combat edge at all.
* `ResolvedMotionFrame` (`shared_tangle`) is the ONLY real non-core dependency:
  one query param in `mechanics/mod.rs` and two test constructions.

⇒ so the destination has to be a crate that may see `shared_tangle`. That rules
out `ambition_characters`, which refused that edge in writing and would need
`ambition_combat` too if the doc line were real. `ambition_combat` already has it
and would cost nothing — but a body switching between biped, morph ball and
flight is MOVEMENT, and putting it in the combat domain buys a free carve at the
price of the ownership question this row exists to answer.

✔✔ **`time_control` CARVED TO `ambition_time` (2026-08-22) — the destination
already owned half of the ADR.** The coupling table above prices `time` at 2
outbound; what it does not show is that ADR 0010 was living in TWO CRATES.
`ClockDomain`, `ClockState` and `ClockObserver` were already in `ambition_time`,
while *who may change a clock* sat in the monolith — and the policy half dragged
combat feel, `BodyCombat` and developer tooling in with it.

⭐ **the split was by ITEM and the measurement found it in one pass.** Of 532
lines, ONE 92-line system (`emit_player_time_intent_system`) carried all six
heavy dependencies and a 23-line ramp carried one; ~380 lines needed nothing but
`ambition_time`. So the arbitration moved and the intent stayed.

⛔ **the compiler named the one thing the measurement missed**, which is the
whole argument for letting it: `report_sim_clock_changes` logs through
`shared_tangle::world_log`, so it stayed behind rather than dragging the tangle
under a base crate. ⇒ **a coupling count finds the shape; only the build finds
the last edge.**

⭐ **and the seam got more coherent, not just shorter.** `ClockRequester::Player`
carried an `ambition_characters::brain::PlayerSlot` while `ClockDomain::PlayerClock`
beside it carried an `ambition_time::ClockObserver` — two newtypes for one fact
across one enum. The payload was never read (every construction was `PRIMARY` and
the policy matches `Player(_)`), so it now speaks the crate's own vocabulary and
the arbitration needs no character edge at all.

⚠ price, and it is the full D33 list: a new `ambition_time::register_rollback_state`
in the runtime's domain list, the `SnapshotState` impls following their types by
the orphan rule, three full-path lines in `rollback-schema-baseline.json`, and one
`file-contains` policy row that pinned the old path by name. The `.txt`
fingerprint did not move — it keys on the bare type name.

⭐ **a free deletion fell out on the way**: four `clear_message_on_rollback`
declarations were registered TWICE in the monolith, byte-identical blocks sixteen
lines apart.

⛔⛔ **AND MY OWN "ZERO DEPENDENCIES" WAS A `crate::` COUNT — the same error this
entry opens by naming.** Counting what the module REACHES, not how it spells it:

```text
ambition_platformer2d_core            22
ambition_platformer2d_shared_tangle    8
ambition_characters                    6   (SlotInteractionState, DrivingParticipant, ActorControl)
ambition_platformer2d_world            1   (CollisionWorld, in the system's params)
```

⇒ **four crates, and no existing home carries all four.** `ambition_combat` has
three and would need a new `world` edge on top of owning movement it has no
business owning; `ambition_characters` refused two of them.

⇒ **`body_mode` is NOT carve-ready, and the pure/glue split that would make it so
does not exist to be moved.** `mechanics/mod.rs` is one 296-line system function;
no file in the module is Bevy-free. Extracting a pure decision from that loop
would create one function with one caller purely to enable a future move — which
is the speculative shape this row keeps refusing. ⇒ **leave it until a
movement-domain crate above `core` exists for an unrelated reason**, and spend
D33's next slice on a module whose destination already exists.

✔✔ **AND IT MOVED (2026-08-22) — the per-slot input model is now one place.**
`SlotGestures` + `SlotInteractionState` → `ambition_characters::brain`, beside
`SlotControls`, `SlotControlLatches` and `SeatRawFrames`, which is what they are:
slot-keyed input tables. They carried **no dependency with them** — primitives and
a `PlayerSlot` — and the `PlayerEntity` / `PrimaryPlayer` re-export that sat beside
them in `control::components` is not theirs and stayed. ⭐ that mattered, because
`ambition_characters` had already REFUSED the `shared_tangle` edge in writing; the
carve had to be a SPLIT of the file, not a move of it.

⭐ **the orphan rule adjudicated the rest, as its own doc promised it would.**
`impl SnapshotState for SlotInteractionState` stopped compiling in the monolith the
moment the type left — trait foreign, type foreign — so the impl and the
`rollback_resource_canonical` registration moved to `ambition_characters` too.
Ledger price, exactly as the four-artifact table below predicts: the `.txt`
fingerprint unchanged, no version bump, one full-path line in
`rollback-schema-baseline.json` re-pointed.

⛔ **`character_sprites::assets` (1,808 lines) — MEASURED AND NOT CARVE-READY,
and the module's own doc was wrong about why (2026-08-22).** Its destination
exists and is named: `{anim, posed_body, attack_hitbox}` left for
`ambition_character_sprites` on 2026-08-09 and `assets` is the remainder. The
doc said it stayed because of three couplings; all three measure stale —
`Platformer2dAssetCatalog`/`ids` are `ambition_asset_manager` re-exports, the
`ambition_persistence` edge is ONE type (`TextureResolutionScale`) that
`ambition_sprite_sheet` already carries into the destination, and
*"bidirectionally coupled to the character-runtime materializer"* is not
bidirectional at all — `assets` names `character_runtime` in two doc comments
and nowhere else.

⇒ ⭐ **the real gate is DIRECTION, not coupling.** `character_runtime` calls
DOWN into `assets`, so carving it would give the monolith a dependency on
`ambition_character_sprites` — precisely the edge the 2026-08-09 carve was
shaped to avoid (its own note: keeping the registration behind *"would have made
the owner depend on the carve, which is the shape that lengthens the workspace's
serial compile chain"*). ⇒ **it moves when `character_runtime` stops calling
down, or it moves WITH `character_runtime`.** The corrected reasons are in the
module's own doc so the next reader does not re-derive them.

⚠ **and one sub-lesson, paid for by a compile error**: `crate::assets::platformer_assets`
is MOSTLY a re-export of `ambition_asset_manager::platformer_assets`, which is
not the same as being one — `scaled_asset_id` is a real adapter that takes a
`TextureResolutionScale` where the underlying function takes an
`Option<&str>` suffix. ⇒ resolve the NAMES, not the module.

⭐⭐ **THE FIFTH GATE, RUN ACROSS EVERY MODULE — 2026-08-21.** Which modules hold
rollback-registered types (asked of the monolith's `rollback_registration.rs`,
because a module can be rollback state without containing one line that says so):

```text
⛔ SCHEMA CHANGE (15)  features 49 · items 12 · character_runtime 6 · encounter 6
                       session 6 · time 6 · avatar 5 · affordances 4 · abilities 2
                       control 2 · gravity 2 · world 2 · body_mode 1 · projectile 1
                       shrine 1
✔ file-move price (13) ability_cooldown, assets, audio, character_roster,
                       character_sprites, construction, dev, enemy_projectile,
                       host, music, platformer_runtime, quest, schedule
```

⛔⛔ **THAT CONCLUSION IS WRONG, AND IT WAS OVERPRICING THE WHOLE CAMPAIGN
(corrected 2026-08-21).** The measurement is right — 15 modules do hold
rollback-registered types — but *"cannot be carved without rewriting the wire
format"* does not follow from it. **A MOVE IS NOT A SCHEMA CHANGE.** Read what
each artifact actually keys on:

```text
rollback_schema_baseline.txt   stable name + codec shape + BARE TYPE NAME   ← no crate path
   `actor.action_set  component-clone  ActionSet  …`                          ⇒ a move changes NO line
GGRS_ROLLBACK_SCHEMA_VERSION   bumped for byte changes under an unchanged
                               descriptor                                     ⇒ a move changes no bytes
rollback-schema-baseline.json  encoded_types + encoding_crates = FULL PATHS  ⇒ a move DOES change these
rollback_coverage.rs waivers   path SUFFIX                                    ⇒ a move DOES re-point these
```

⭐ **MEASURED, not reasoned** (the whole artifact, not a sample): of the 443
lines in `rollback_schema_baseline.txt`, **zero** contain `ambition_`. The 87
that contain `::` are all the fixed string `bevy_ggrs::Rollback` in the
description column. No line names one of our crates, so no move between our
crates can change one.

⇒ the real price of carving a rollback-registered module is **two ledger updates,
not a wire-format rewrite**: re-spell the paths in the JSON baseline and
re-point the waivers. The `.txt` fingerprint and the version constant do not
move, because the type is the same type with the same descriptor — the frozen
set it belongs to is *"which types are in the wire format"*, and a move keeps it
in. ⭐ **the waiver's REASON stays as written**; a move does not change the
answer to *"is this per-frame state?"*, only where to ask it.

⭐⭐ **PROVEN BY DOING IT — `affordances` LANDED IN `ambition_sim_view`,
2026-08-21.** 1,725 lines left the monolith carrying all four of its
rollback-declared types, and `the_rollback_schema_matches_its_recorded_baseline`
stayed GREEN with no baseline edit at all. Two reasons, and the second is
sharper than the correction above:

* the fingerprint records `stable name / kind / bare type name / reason` and no
  owning crate, so `OWNER = env!("CARGO_PKG_NAME")` changing from the monolith
  to `ambition_sim_view` moved nothing;
* **a DERIVED declaration is not in the wire format to begin with.** All four are
  `declare_rollback_derived_resource` — the declaration exists to tell the sweep
  *not* to snapshot them. `rollback-wire-format-is-frozen` reports 119 encoded
  types and none of these is one. ⇒ the ⛔ column above over-counts: a module's
  registered types are only a carve cost when they are AUTHORITATIVE.

⚠ **the real price was two things the ⛔ column never mentioned**: the five
lockfiles (`cargo tree --locked` exits 101 on a new dep edge, and the contracts
job crashes rather than failing), and a `#[cfg(feature = "portal")]` that turned
into a SILENT REGRESSION on arrival — `ambition_sim_view` has no `portal`
feature, so every gate resolved false and `portal_gun_active` became a hardcoded
`false` while the crate compiled clean. The gates are deleted; this crate's
dependency on the monolith names `portal` unconditionally, so the `not(portal)`
arm was unreachable-but-selected. ⇒ **grep the moved code for `cfg(feature` before
believing a carve is behaviour-preserving.**

⚠ **so the ⛔/✔ split above is really cheap vs cheaper**, and the campaign has
been avoiding 15 modules for a cost it does not pay. ⛔ what a carve DOES still
owe is the check that neither ledger went stale: `cargo test -p ambition_app
--test app_it` and `python3 scripts/check_absence_contracts.py --check`, because
`cargo check -p ambition_app --all-targets` passes green while both are red.

⚠ **the table above is a SNAPSHOT; the METHOD is the durable part** — the same
lesson the stranded-doc census records about itself. Re-run it for the module you
are about to move, because a registration added tomorrow will not announce
itself:

```sh
grep -c "crate::<module>::" \
  crates/ambition_platformer2d_actor_monolith/src/rollback_registration.rs
```

⛔ ask the OWNING crate's `rollback_registration.rs`, never the module being
moved. `affordances` contains zero lines mentioning rollback and has four
registered types — which is exactly how this gate got missed once already
tonight.

**THREE CANDIDATES TAKEN TO A VERDICT, and every one was decided by the
DESTINATION rather than by the coupling table:**

```text
dev/trace          ⛔ REFUSED  `ambition_dev_tools`: "the gameplay trace
                              recorder … STAYS SIM-SIDE"
character_sprites  ⛔ REFUSED  `ambition_character_sprites`: "the load-bearing
                              property is the DIRECTION: the actor crate does
                              not depend on this one". 11 PRODUCTION uses in
                              assets/actor_clusters/character_runtime would each
                              invert it. (Its 12 "outbound" refs were an
                              illusion — doc comments and a test-only fixture.)
affordances        ◐ LEGAL    contract met, concept whole, direction fine —
                              but 4 rollback-registered types make it a SCHEMA
                              CHANGE, not a file move
```

```text
audio              ⛔ REFUSED  `ambition_audio` is "content-free … HOSTS decide
                              which track to request" — and the monolith's
                              `audio` IS that host-side decision layer, with six
                              inward deps (actor, assets, music, rooms,
                              schedule, session)
```

⇒ **D136's thesis is now five-for-five**, and four of those caught THIS session
mid-reach. A stated contract is the cheapest refusal in the project.

⛔⛔ **AND THE COUPLING TABLE SHOULD BE READ UPSIDE DOWN — this is the session's
real finding.** Four of the five cheapest-by-coupling modules are refused or
schema-priced, and the reason is structural rather than bad luck: **a module has
low coupling because it is ASSEMBLY or an OBSERVER, and assembly is exactly what
this crate is FINISHED as.** `dev/trace` samples the sim; `audio` decides what
the host plays; `schedule` and `host` are the assembly outright.

⇒ so *"lowest coupling first"* selects for the modules that must STAY. The
carveable ones are domains with real internal mass — and those are the ones the
rollback gate prices as schema changes. That is not an accident either: a domain
with state worth rewinding is a domain, and a domain is what a crate is for.

⇒ **the practical ordering this yields**: stop looking for a cheap carve. Pick
the domain whose CONCEPT most wants to be whole, accept the schema price, and do
the baseline rewrite in the same slice. `affordances` is the smallest instance of
that shape on the board (4 registered types), which makes it the right REHEARSAL
for the ones that matter (`items` at 12, `features` at 49).

⚠ **coupling is NECESSARY, NOT SUFFICIENT — and the `dev/trace` entry above is
this row's own proof, earned the embarrassing way.** The table said 0 inbound and
a destination already existed; both were true, and the module still belongs
exactly where it is. A module nothing reads is easy to move; that does not make
it MISPLACED. Judge the result by Jon's test, not by this table.

⭐⭐ **AND IT IS THE FOURTH POSITIVE INSTANCE FOR D136, this time catching an
agent mid-reach.** The destination's stated contract killed a plausible carve at
zero cost — before a line moved, in the same session that wrote *"read the
DESTINATION's stated contract before moving anything into it"* into three other
crates. The rule is worth what that sentence claims.

✔ **`affordances` IS cleared — check DONE, 2026-08-21, not asserted.**
`ambition_sim_view`'s contract is specific: plain-data snapshots, pure functions
of sim state, **no `Entity`/`Handle` borrows in the ROWS**, no caching across
ticks. Read against the published type:

```text
PlayerAffordances   jump / attack / shield / dash / interact / special
                    — six enums, no Entity, no Handle, no Vec of borrows
Entity appears      only in `compute_player_affordances`'s QUERY params,
                    which every extraction system has
cross-tick cache    none (no `Local<…>` anywhere in the module)
```

⇒ **it satisfies the destination's stated contract**, which is what
`dev/trace` failed.

⛔⛔ **CORRECTION 2026-08-21: A FIFTH GATE FAILS, AND IT IS THE ONE THIS ROW
ALREADY WARNED ABOUT.** I recorded "all four gates pass, ready" and then went to
do it. Four affordance types are ROLLBACK-REGISTERED — from the monolith's
`rollback_registration.rs`, not from the module, which is why grepping the module
directory found nothing:

```text
declare_rollback_derived_resource::<crate::affordances::PlayerAffordances>
                                    …::intent::PlayerIntent
                                    …::interactable_proximity::NearestInteractable
                                    …::pogo_proximity::PogoTargetBelow
```

Their TYPE PATHS are in `scripts/baselines/rollback-schema-baseline.json`. ⇒ this
is **a wire-format change, not a file move**: the registration has to move too,
the baseline has to be rewritten, and `rollback-wire-format-is-frozen` reddens
until it is. That is exactly this row's own ⛔⛔ — *"the price of moving a
registered type: three path-keyed ledgers, and they are not in one place"* —
which I had read hours earlier and still walked toward.

⚠ **so the gate list below was INCOMPLETE.** A fifth belongs on it permanently:
**does anything register these types for rollback, from anywhere?** Ask
`rollback_registration.rs` in the OWNING crate, never the moving module.

⇒ still worth doing, at a materially higher price than "1,733 lines with zero
inbound". Price it as a schema change.

**The four gates that DO pass:**

```text
1 coupling        1,733 lines, ZERO inbound `crate::affordances` references
2 destination     `PlayerAffordances` is six enums; no Entity/Handle in the
  contract        rows, no cross-tick cache — `ambition_sim_view`'s rule met
3 concept whole   `interactable_proximity` is a pure query whose only write is
                  its own output resource, and it deliberately REUSES the
                  buffered-interact systems' `strict_intersects` rather than
                  restating it — "so the HUD label switches at exactly the
                  moment the interaction would fire". The rule stays with its
                  owner; this only observes it
4 dep direction   ⭐ the one the coupling table cannot see, and it resolves the
                  RIGHT way: `ambition_sim_view` lists the monolith under
                  [dependencies] while the monolith lists sim_view only under
                  [dev-dependencies]. sim_view is ABOVE, so it may already name
                  `features::ChestFeature`. No new edge, no inversion
```

⚠ **NOT EXECUTED — and after the fifth gate, that is a blocker rather than a
preference.** A wire-format change begun unattended, in the crate another session
is actively carving, is how a desync ships. ⇒ do it deliberately, with the
baseline rewrite and the absence contract in the same slice, and with somebody
watching the gate.

⭐ **THE ENDPOINT IS NOW STATED IN THE CRATE ITSELF — 2026-08-21.** A
decomposition with no stated endpoint runs forever, and the monolith's header
listed the domains it holds, which is an INVENTORY and cannot refuse anything:
every gameplay-systems-shaped thing matches it. Measured — the crate gained **49
files in the fourteen days it was being carved**, more than any other crate in
the workspace.

⇒ what it is FINISHED as is the **ASSEMBLY**: the schedule, the host, the
session, the module graph, and the cross-cutting types submodules reach through
it. Assembly is the one concept that cannot be carved out, because it is what
does the assembling. Every domain in the list — `world`, `abilities`, `combat`,
`gravity`, `items`, `music`, `quest`, actors and brains — is therefore a carve
CANDIDATE rather than a resident, and the crate now says so where somebody adds a
file: *is this assembly, or a domain that has not been given its crate yet?*

⭐⭐⭐ **LOC IS THE PROXY. THE WIN IS CONCEPTUAL DOMAIN SEPARATION.** Jon,
2026-08-21, on the measurements below: *"loc is the proxy. the real win is
conceptual domain separation."*

⇒ read every number in this row as a SYMPTOM, and judge a carve by one question
instead: **does the concept end up in the crate that owns it?** A slice that
moves 2,000 lines and leaves the concept split across two crates is worth less
than one that moves 200 and makes a domain whole. ⛔ do not price a carve by
what it removes from a per-crate ledger — that is how debt gets laundered rather
than paid.

⚠ the corollary, and it is why the LOC readings kept misleading: the ratchet
counts TESTS (39% of this crate), so a well-tested crate trips its own carve
alarm, and a carve that moves code moves its tests with it. Neither number says
anything about whether a domain is whole.

⛔⛔ **THE PRICE OF MOVING A REGISTERED TYPE: THREE PATH-KEYED LEDGERS, AND THEY
ARE NOT IN ONE PLACE.** Learned the hard way twice on 2026-08-21 — one carve
reddened an absence contract, the next reddened the app suite, both within an
hour of a green gate.

```text
scripts/baselines/rollback-schema-baseline.json   FULL TYPE PATH   caught by `rollback-wire-format-is-frozen`
rollback_coverage.rs  RESOURCE_WAIVED / WAIVED    PATH SUFFIX      caught by app_it
rollback_schema_baseline.txt / .rs                STABLE ID        unaffected — keys on `actor.surface_state`
```

⇒ **the gate is not enough for a relocation.** `cargo check -p ambition_app
--all-targets` and even the smash suite pass while two of these are stale; only
the app integration suite and the contract checker see them. ⚠ run BOTH before
committing a type move.

⭐ and when you re-point a waiver, **re-point the path and leave the REASON
alone**. The waiver answers a question — "is this per-frame state?" — and moving
a type does not change the answer, only where to ask it. Rewriting the
justification to match a diff is how a true waiver becomes a false one.

⭐⭐⭐ **HOW TO MEASURE A CARVE'S COUPLING — learned three ways in one session,
each by getting it wrong.** A module's coupling is not what it imports:

```text
use statements only    -> missed inline `crate::` paths in fn bodies   (capture, attempt 1)
`crate::` paths only   -> missed ABSOLUTE self-references to the        (footstool: 13
                          destination crate                             refs pointing home)
any mention of a crate -> FALSE blocker from a DOC COMMENT naming a     (hit_camera_shake:
                          crate above you                                nearly refused)
```

⇒ **grep the CODE (strip `///`, `//!`, `//`) for BOTH `crate::…` and
`ambition_…::…`, resolve each symbol to its defining crate, and count only the
ones defined ONLY in the monolith.**

⭐ **and domain moves COMPOUND**: `hit_camera_shake` was freed by the feel-tuning
move made hours earlier for a different carve. Do them in dependency order, not
by size.

**Candidates by that method (2026-08-21), zero monolith-owned type blockers:**

```text
891  bosses/tick.rs              ⚠ bosses already carved to ambition_boss_encounter
460  actors/limbs.rs             ◐ SPLITS — see below
420  attack.rs                   ⛔ MEASURED AND BLOCKED — see below
409  spawn/capability_lanes.rs
400  spawn/content_staging.rs
364  actors/conversion.rs
348  spawn/character_spawn_plan.rs
346  anim_helpers.rs
```

⛔ **`spawn_static.rs` IS NOT A CARVE CANDIDATE — it is a LOWERING SEAM, and the
scan flagged it for doing its job (2026-08-21).** 9 of its 14 public fns name
pickup/chest/portal/shrine because it turns authored `*Spec` values into live
components. A spawner naming many domains is correctly placed; this is the
false-positive case the caveat above predicts, confirmed by measurement rather
than by argument.

⭐⭐ **but measuring it found a FORK worth a row of its own.** The authored
vocabulary MIRRORS the runtime vocabulary variant for variant, and
`spawn_static` is the hand-written mapping between them:

```text
BreakableStateSpec      Intact/Cracking/Broken/Respawning  ⟷ ambition_interaction::BreakableState
BreakableCollisionSpec  None/OneWayUp/Solid                ⟷ ambition_interaction::BreakableCollision
BreakableTriggerSpec    OnHit/OnStand/Either               ⟷ ambition_interaction::BreakableTrigger
ChestStateSpec          Closed/Opening/Opened              ⟷ ambition_interaction::ChestState
InteractionKindSpec     Breakable/Chest/Door/Npc/Pickup/…  ⟷ ambition_interaction::InteractionKind
PickupKindSpec          ✔ DELETED 2026-08-21 (`d3bd6e95a`) — it WAS `PickupKind`,
                        byte-identical, in the SAME crate. No edge needed.
PortalChannelColorSpec  Cyan/Green/Magenta/…/Indexed       ⟷ ambition_portal2d::PortalChannelColor
```

**SIX pairs left (was seven), ~25 variants, kept in step BY HAND** — every spec lives in
`ambition_entity_catalog` and every runtime twin in the crate that owns the
behaviour. ⚠ *"mirrors X"* is a fork declaration, and a variant added to one
side and not the other is a silent lowering gap, not a compile error.

✔ **AND IT HAS NOT DRIFTED — checked, 2026-08-21.** All seven pairs match
variant for variant today (4/4, 3/3, 3/3, 3/3, 6/6, 5/5, 9/9). ⇒ this is a
LATENT hazard, not a live defect: nothing is broken right now, and the row must
not be worked as though something is.

⭐⭐ **AND THE HAZARD IS ONE-DIRECTIONAL — HALF OF IT IS ALREADY GUARDED.**
`spawn_static`'s mappings are exhaustive matches on the SPEC enum with **zero
catch-all arms** (`grep -c '_ =>'` = 0). So:

```text
new variant on the SPEC side     -> match is non-exhaustive -> COMPILE ERROR ✔
new variant on the RUNTIME side  -> nothing breaks; the variant is simply
                                    unreachable from authoring        ⛔ SILENT
```

⇒ the exhaustive destructure is already doing the work in one direction, which
is why nothing has drifted. The unguarded direction is a runtime variant with no
authored spec — content that cannot be authored rather than content authored
wrong, so it fails as *"the editor cannot place this"*, never as a crash.

⚠ **and a rename is not done until every PREFIX is swept.** Deleting that one
pair took three passes because the same type is spelled
`ambition_entity_catalog::…`, `ambition_platformer2d::entity_catalog::…` and
`…::placements::…`. Sweep the symbol AND every path that reaches it — this cost
a retry on three separate carves today.

⇒ **that makes the slice much smaller than "delete the spec enums"**: what is
missing is a check that every RUNTIME variant has a spec counterpart. ⚠ still
not urgent — zero drift today — and ⛔ still not a reason to add a dependency
edge from `ambition_entity_catalog` to `ambition_interaction`/`ambition_portal2d`
(see `attack.rs` above for what an edge costs).

⇒ worth its own slice, and it is a DELETION one: either the spec enums go and
the catalog names the runtime types directly, or the mapping becomes a derive
that cannot drift. ⛔ do not price it from `spawn_static`'s line count — the
duplication is in the two vocabularies, not in the mapper. ⚠ and do not price it
as URGENT either; the measurement above is what says so.

✔ **BOSS ANIMATION CARVED 2026-08-21 (`9ea8ea2fa`) — the first slice the split
scan found rather than a human.** `anim_helpers.rs` had 7 public fns of which 4
were boss-only; those four plus two private helpers are
`ambition_boss_encounter::anim` now, at no dependency cost (`ambition_sim_view`
was reading them THROUGH the monolith while already depending on that crate —
checked BEFORE moving, after `attack.rs` proved an edge is not free). What stayed
is not boss: chest, breakable, and the actor overlay advance.

⭐⭐⭐ **AND IT SURFACED A THIRD, SHARPER SIGNAL — `super::super::` REACHING INTO
ANOTHER CRATE.** The last compile error of that carve was the module calling
`super::super::bosses::boss_animation_keys_for_profile` — a RELATIVE path
climbing out through the monolith to reach a function that lives in
`ambition_boss_encounter::behavior`, i.e. the crate the code belonged in all
along. Code walking up and out to reach its own crate's function is the
strongest placement smell there is, and it is greppable:

⛔⛔ **AND THEN EVERY CANDIDATE IT PRODUCED WAS RESOLVED, AND ALL OF THEM ARE
FALSE — which is the result worth recording.** The raw counts looked
actionable (`update.rs` 27, `actor_hit.rs` 11, `spawn_actors.rs` 9,
`conversion.rs` 8, `bosses/tick.rs` 6, `boss_hit.rs` 4) and **not one crosses a
crate**. They resolve to `components`, `actor_clusters`, `perception`,
`damage_drops`, `enemies`, `npcs` — all monolith-internal, i.e. ordinary module
navigation in a deep tree.

⇒ **the signal is REAL but RARE: it has fired exactly once** (the boss-anim
carve, now clean) and the raw count is worthless without resolving the
destination. ⛔ do not work this list — there is nothing on it. Re-run the scan
after a carve, resolve each target's crate, and act only on the ones that leave
the crate.

⚠ this is the second scan in an hour whose headline was an artifact (after
"quest" matching inside "request"). A cheap grep over a large tree WILL produce a
confident-looking ranking of nothing. The caveat is not boilerplate; it is the
only thing standing between the list and a wasted session.

⭐⭐ **A CHEAP SCAN THAT FINDS SPLIT CANDIDATES: public functions naming a domain
their FILE does not.** Both splits found by hand this session have the same
tell — `apply_player_hit_events` in `damage_apply.rs`,
`route_boss_strikes_to_limbs` in `limbs.rs`. Grepping for it across the monolith
finds them automatically, and finds more:

```text
alien/total  module                        foreign domains named
   9/29      construction/mod.rs           mount 3, item 2, boss/enemy/shrine/limb
   9/14      features/ecs/spawn_static.rs  pickup 3, chest 2, portal 2, item, shrine
   7/10      features/ecs/damage_apply.rs  player 7          ← the split found by hand
   6/14      schedule/input_systems.rs     menu 4, player, cutscene
   5/7       features/ecs/anim_helpers.rs  boss 4, chest      ← "helpers" that are mostly boss
   3/5       avatar/systems.rs             player 3
```

⛔⛔ **THE FIRST VERSION OF THIS TABLE WAS WRONG, AND THE WAY IT WAS WRONG IS THE
WARNING.** It reported `construction/mod.rs` as **18/29 with "quest" nine
times** — the single largest signal in the scan — and every one of those nine
was `re`**quest**: `relocate_request`, `authored_actor_requests`,
`placement_requests`. A SUBSTRING match, not a domain. The corrected scan uses
word boundaries and the row drops to 9/29 with no quest at all.

⚠ a name-matching instrument will invent its own biggest finding if you let it,
and the finding will look like the most interesting one on the page. **Check the
top hit by eye before believing the list.**

⭐ **it independently rediscovered `damage_apply`**, which is the only reason to
trust it at all.

⚠⚠ **it is a NAME proxy and it produces CANDIDATES, never verdicts.** A
`spawn_boss_chest` inside a spawn module is perfectly placed and this scan will
flag it; the scan cannot tell a misplaced domain from a well-named one. ⛔ every
hit still needs the coupling measured — comments stripped, both path spellings,
symbols resolved to their defining crate — before anybody moves a line.

◐ **`actors/limbs.rs` SPLITS, and its own doc is the evidence (2026-08-21).**
The module says it is generic — *"any mount or actor with articulated parts"* —
and then reads boss types. **4 of its 13 items touch `BossConfig` /
`BossAttackState`**, one of them named `route_boss_strikes_to_limbs` outright:

```text
generic limb rig      9 items   ActorControlFrame + brain::  -> ambition_characters
boss strike routing   4 items   BossConfig, BossAttackState  -> ambition_boss_encounter
                                resolve_active_route, station_frame,
                                route_boss_strikes_to_limbs, LimbPhase
```

⇒ **the same shape as `damage_apply`**: a module whose NAME is one domain and
whose CONTENTS are two, so "move limbs.rs" is the wrong slice in exactly the way
"move damage_apply" was. The generic half's dependencies (`ActorControlFrame`,
`brain::`) are both `ambition_characters`; the boss half's are already
`ambition_boss_encounter`'s own — ⚠ and that crate sits ABOVE `characters`, so
the halves cannot travel together and the file cannot go to either crate whole.

⚠ **not attempted** — the split needs the boss half's own coupling measured
before it moves, and a 3-way change is not a thing to start on short runway.

⛔⛔ **`attack.rs` IS MEASURED AND BLOCKED — it needs a DEPENDENCY DECISION, not
a move (2026-08-21).** Every path it names resolves except one:

```text
crate::combat::*                   -> ambition_combat        ✔ becomes crate::
crate::physics::ResolvedMotionFrame -> shared_tangle          ✔ re-export
crate::time::feel::…               -> ambition_combat::feel  ✔ moved today
ambition_platformer2d_world::collision::CollisionWorld        ⛔ combat does NOT
                                                                 depend on world
```

⚠ **the edge is not free, and this was PROBED rather than assumed.** Adding
`ambition_platformer2d_world` to `ambition_combat` breaks the contract job
immediately: `cargo tree --locked` exits 101 because the edge rewrites the
lockfile — and there are **FIVE** of them (`./`, `examples/capability_demo`,
`examples/portal_tutorial`, `fixtures/minimal_game`,
`fixtures/external_consumer`). ⭐ the probe also dirtied the root `Cargo.lock`
and nothing else; it was restored.

⇒ two honest options, and it is a design call rather than a mechanical one:
**(a)** combat takes the world edge, accepting the footprint growth that
`capability-footprint-may-not-grow` exists to watch; or **(b)** the strike
resolution that needs `CollisionWorld` is threaded in by the caller instead, so
combat asks nobody for terrain. ⛔ do not add the edge just to make a carve
compile — that is the debt-laundering this row refuses.

⚠⚠ **TWO LIMITS ON THAT LIST, and neither is optional to check.**
1. the scan resolves TYPE-like symbols only, so a lowercase free function
   defined in the monolith can still block — `apply_body_hit_reaction` was
   exactly that, and it pinned a 1,922-line domain by being `pub(crate)`.
2. **zero blockers does not name a DESTINATION.** Capture, footstool and hit
   shake went to `ambition_combat` because it owns their concepts; `spawn/…`
   and `anim_helpers` may own nothing there. ⛔ a carve with no owning crate is
   a relocation, not domain separation — the thing this row exists to refuse.

✔✔ **THE CAPTURE CARVE LANDED 2026-08-21 (`8669740f5`), AND IT WAS NEVER A
COUPLING PROBLEM.** The whole 1,922-line module named exactly FIVE paths outside
its crate, and every one was fixed by putting a TYPE where it belonged — nothing
about capture changed:

```text
ActorSurfaceState               -> platformer2d_core   6c4592021
Platformer2dFeelTuningMonolith  -> ambition_combat     d6db434f4
apply_body_hit_reaction         -> ambition_combat     403a32155
                                   ⛔ it was `pub(crate)` — a whole domain
                                   pinned by a VISIBILITY MODIFIER
ActorFaction / CenteredAabb     already in characters/geometry, merely
                                reached through the monolith's re-export
```

⭐ **a dependency died with it**: `ambition_platformer2d_runtime` was registering
capture systems THROUGH the monolith and names `ambition_combat` directly now.
The demo reaches them through the facade, as a game should.

⭐ **and the boundary inside `damage_apply` is the reusable finding.** It splits:
`apply_body_hit_reaction` / `VictimStance` / `hit_response_tuning` are
body-generic and went to combat; `apply_player_hit_events`,
`publish_kernel_reset_death` and `void_pending_player_hits_at_lifecycle_
boundaries` all take `PlayerSafetyState`/`PlayerBodyFrameOutput` and stayed.
⇒ **"move damage_apply" would have been the WRONG slice** — applying a hit to
any body is combat's, applying one to the player's avatar is the game's.

**Where that leaves the numbers** (symptoms, per the framing above):

```text
                total     tests   NON-TEST
baseline      111,429    42,146     69,283
2026-08-21    118,655    47,521     71,134   (+7,226 / +1,851 non-test)
ambition_combat 19,817    5,733     14,084   ← where capture now lives
```

⚠ still over, and **still mostly tests** (+5,375 of the +7,226). The
behaviour-side gap is +1,851 in twelve days.

⭐⭐ **THE CAPTURE CARVE: FIRST ATTEMPT UNWOUND, AND IT NAMED THE CARVE
ORDER.** The prerequisite landed (`6c4592021`, `ActorSurfaceState` down to the
floor crate); the move itself did not, and the reason is worth more than the
move would have been.

**The domain:** `ambition_combat` owns the capture VOCABULARY (`captive_of`,
`CapturedBy`, `CaptureAttemptRequested`) and the monolith owns every SYSTEM that
reads or writes it — acquire, tick the hold, constrain the captive's body,
restrict the captor, release, pummel, throw. One concept, two crates, and the
systems in a crate that has nothing to do with grabbing.

**It compiles down to ONE blocking system.** Of 1,922 lines, every import was
already on `ambition_combat` — parent vocabulary, `hitbox`,
`platformer2d_core`, `shared_tangle::sim_id`, `characters::brain`. The whole
module moved cleanly except **`apply_capture_throws`**, which reaches UP twice:

```text
crate::features::ecs::damage_apply::apply_body_hit_reaction   (line 1919)
crate::time::feel::Platformer2dFeelTuningMonolith             (line 1857)
```

⇒ **the carve order is DAMAGE FIRST, THEN CAPTURE.** Applying a hit reaction is
combat's own subject and it is still monolith-owned, so capture cannot leave
before it — and splitting `apply_capture_throws` off to move the other 1,900
lines would leave the domain in two crates, which is the thing the carve is for.
⛔ do not retry capture alone.

⚠ **and the earlier reading that this module was "clean" was MINE and it was
wrong** — it came from the `use` statements, and both blockers are `crate::`
paths spelled inline inside a function body. A carve's coupling is not what a
file imports; it is every path it names.

⛔⛔ **STEP 3 WAS ATTEMPTED AS THE `encounter` CARVE ON 2026-08-17 AND IS
REFUSED — BOTH PRECONDITIONS BELOW ARE FALSE, AND THE MEASUREMENT THAT PRODUCED
THEM WAS TAKEN WITH THE WRONG INSTRUMENT.** The block below is the original
proposal; it is kept because the correction only makes sense against it.


⛔⛔ **NO LONGER RESOLVED — RE-MEASURED 2026-08-21 AND IT IS BACK OVER, BY MORE
THAN THE CARVES EVER REMOVED.** `largest_unit_lines` is a finding again:
**120,990, +9,561 over the frozen 111,429, against a +2,228 budget.**

```text
111,429   the frozen baseline (2026-08-09)
121,822   2026-08-17 morning        +10,393 over, ~5× budget
114,139   boss_encounter CARVED     725de8c26   −7,683
110,932   four modules RELOCATED    355874fe1   −3,207   ⭐ UNDER baseline
120,990   2026-08-21                +9,561 over  ⛔ the win lasted four days
```

⭐⭐ **AND THE CARVES ARE NOT WHAT FAILED — INFLOW IS.** Measured unit by unit
against the baseline tree (`git cat-file --batch` over both, lines not bytes):

```text
LEFT the crate   −11,773   boss_encounter −6,780 · conversation −2,547
                           persistence −1,331 · menu −812 · equipment −303
CAME IN          +21,334   features +9,899 · items +2,745 · world +1,814
                           session +1,350 · loose-at-src +1,160 · avatar +1,070
                           character_runtime +815 · construction +677 · …
                 ─────────
net              +9,561
```

⇒ **every carve did exactly what it promised and the crate still grew by half
again as much as they removed.** So "pick the next carve" is the wrong question:
at this inflow rate a carve buys about four days. ⛔ do not open another carve
row without saying what stops the refill — that is the actual finding, and it is
new.

⛔⛔ **AND THE SHARPER CORRECTION, measured after the above and softening it:
56% OF THAT REGRESSION IS TEST CODE.**

```text
              baseline    HEAD     delta
TEST            42,146  47,521    +5,375   ← 56% of the +9,561
NON-TEST        69,283  73,469    +4,186   ← the crate's actual behaviour
```

⇒ the crate's BEHAVIOUR grew 6% in twelve days while the ratchet reported a
9,561-line regression. The inflow finding above stands, but it is half the size
it looks.

⭐⭐ **THE REAL DEFECT IS THAT ONE NUMBER SERVES TWO QUESTIONS.**
`largest_unit_lines` lives in `compile_ratchet.py`, where counting tests is
CORRECT — tests compile, and `--all-targets` pays for them. D33 then reads the
same number as a DECOMPOSITION signal, where counting tests is wrong: a crate
that adds good tests trips its own carve alarm, and "carve me" and "well tested"
want opposite responses.

⇒ ⛔ **do not retune the ratchet** — it is right for its own question. **D33
should cite the NON-TEST number**, and this row now does. The instrument asks a
proxy question only when read by this row.

⚠ **the other measurement fact**, still worth stating before anybody prices a
slice:
* a carve moves the tests with the code, so LOC deltas are not a proxy for how
  much *behaviour* moved — the split above is.
* **`features/` is 40% of the crate** (48,477) and `features/ecs` alone is
  38,949 — a third of the whole monolith. It is not an ownership unit; it is a
  grab bag, and **32 loose files sit directly in `features/ecs` totalling
  17,049 lines** with no subdirectory to name what owns them (`spawn_actors` 2,607,
  `capture` 1,922, `actor_clusters` 1,622, `damage_apply` 1,588 …). D33's own
  rule — carve by *coherent ownership*, not by LOC — has no ownership to read
  there until those are grouped.

⭐ **the second slice created NO new crate** and cost no hop
(`critical_path_crates` 13 → 13 → 13, where the `conversation` carve had cost
12 → 13). It moved modules whose owning crate ALREADY existed and deleted a dead
1,336-line `persistence` module outright.

⚠⚠ **AND THE GAIN IS NOT PROTECTED YET, which is a consequence worth stating.**
A ratchet locks a win by being RE-FROZEN, and `compile_ratchet.py --update`
writes the WHOLE snapshot — so banking the 497 lines would also bank
`ambition_geometry`'s and `ambition_platformer2d_core`'s edit-cost regressions,
which are eight days of unrelated growth nobody has accounted for.

⚠⚠ **AND IT HAS DRIFTED — MEASURED AT HEAD 2026-08-18, one day later.** The
same count that produces the ledger's own 110,932 at `355874fe1` gives:

```text
355874fe1   110,932   the recorded win, UNDER the 111,429 baseline
HEAD        115,562   +4,630 in a day, and +4,133 OVER the baseline
```

⛔ **but a carve is the WRONG response, because the biggest single item is debt
that moved here BY DESIGN.** Where it went:

```text
+1,555  features/ecs/capture.rs (NEW)   the grab campaign — a whole mechanic
  +767  rollback_registration.rs (NEW)  the domain-owned rollback merge
  +589  world/authored_switch_commands{,/tests}.rs (NEW)   D136's own inversion
  rest  tests and ordinary feature growth
```

⭐ the +767 is a RELOCATION, and the other side of it is visible in the same
window: `ambition_platformer2d_runtime` went **17,652 → 15,554 (−2,098)** as the
`rollback/domains/*` files dissolved into the crates that own the state. That is
this repo's own rule working — *"the destination joins in the same commit"* — and
a per-crate line ratchet reads it as the monolith rotting.

⇒ **so the honest statement is not "the monolith grew 4,133 over budget"**; it is
that one mechanic (+1,555), one inversion this row asked for (+589) and one
ownership transfer that shrank a sibling by 2,098 (+767) account for most of it.
⛔ a session that reads only the total will carve something to pay for work that
was correct.

⇒ **it stays unfrozen on purpose, and the monolith may drift back up to
111,429 for free until the other findings are dealt with.** ⛔ do not re-freeze
to make the tool quiet — that is the laundering this row already paid for once.
⭐ the honest sequence is: account for the edit-cost regressions FIRST, then
re-freeze everything together and the size gain locks in with them.

⭐⭐ **AND A CARVE LANDED AND WAS MEASURED — `d6ac394ff`, GGRS ROLLBACK HOSTING
OUT OF THE GENERIC RUNTIME (merged 2026-08-19).** This is the row's own thesis
producing a number rather than an argument:

```text
ambition_platformer2d_rollback_ggrs  [NEW CRATE]  +6,440 lines
ambition_platformer2d_runtime                     −5,221 lines
largest_unit_seconds   ambition_platformer2d_runtime 253.3s
                    →  ambition_content              209.1s      ⭐ −44s
```

⭐ **the dearest single compile unit is no longer the runtime**, which is the
payoff a carve is supposed to buy and the one this ledger can see directly.
`edit_cost_seconds` fell ~76s for all three watched crates in the same window,
even though the workspace GREW by a crate and ~1,000 lines — because the lines
that moved left an expensive-per-line crate for a leaf.

⚠ **+6,440 out against −5,221 in is not bookkeeping error**: the difference is
new material the carve wrote (`reconcile.rs`, `registrar.rs`, `registration.rs`,
`codec_tests.rs`, `host_invariant_tests.rs`). ⛔ so this is NOT the debt-laundering
shape this row already paid for once — the destination joined in the same commit,
which is the test that distinguishes a carve from a move.

⚠ the new crate is UNPRICED (median placeholder, R² = 0.12), so its SECONDS
figure is a guess; the LINE movement above is exact and is what the claim rests
on.

⭐⭐ **THAT ACCOUNTING IS DONE — 2026-08-19 — AND THREE OF THE SEVEN
"REGRESSIONS" ARE THE WORKSPACE GROWING, SEEN FROM A FOUNDATION CRATE.**
`edit_cost_lines` is the total lines of a crate's REVERSE-DEPENDENCY CLOSURE, so
for a crate 49 of 59 crates depend on, it is very nearly a measurement of the
workspace:

```text
workspace                       456,072 → 535,388   +79,316  (+17.4%), 56 → 59 crates

ambition_geometry     edit cost 433,774 → 511,209   +77,435  = 97.6% of that growth
                      own lines                       +197
                      share of workspace  95.1% → 95.5%   (+0.4 pts)

platformer2d_core     edit cost 430,276 → 508,302   +78,026  = 98.4% of that growth
                      own lines                     +5,885
                      share of workspace  94.3% → 94.9%   (+0.6 pts)

actor_monolith        edit cost 251,085 → 285,581   +34,496
                      share of workspace  55.1% → 53.3%   (−1.7 pts)  ⭐ IMPROVED
```

⇒ **`ambition_geometry` grew 197 lines and its edit cost grew 77,435.** Nothing
about that crate got worse; the code behind it got bigger. ⛔ **so a carve is the
wrong response to those two findings and always would have been** — carving
geometry cannot move a number that is ~95% workspace size. The only levers are
reducing FAN-IN (fewer crates depending on the foundation) or not growing, and
neither is what "REGRESSED, something got bigger" suggests.

⭐ **the monolith's edit-cost SHARE fell 1.7 points**, which is this row's actual
thesis holding even while its absolute line count is over budget: the
decomposition is working and the per-crate absolute ratchet cannot see it.

⚠ **the accounting above uses LINE COUNTS, which are exact, and is therefore
independent of the ratchet's `UNPRICED` finding** — three crates
(`ambition_binding`, `ambition_boss_encounter`, `ambition_conversation`) are
priced at the population median because nothing has measured them, and size
predicts a crate's compile cost with R² = 0.12, so every SECONDS figure involving
them is off by an unknown factor. ⛔ measuring them means
`compile_collect.py` building the whole graph into its own target root, and this
volume sat at 93% with 36 GB free; that is the operation that filled it before.
⇒ **the guess is accepted for now, and no conclusion here rests on it.**

⛔⛔ **THE INSTRUMENT ASKS A PROXY QUESTION for foundation crates.** Absolute
`edit_cost_lines` conflates *"this crate became expensive to edit"* with *"this
workspace became bigger"*, and for a crate whose closure is 95% of the workspace
it measures only the second. `--report-only` now prints the share alongside the
absolute number so the distinction is visible without changing what the gate
fails on. ⇒ **re-freezing is still not the answer to the monolith's +5,887**;
what these three findings needed was to be read correctly, and now they can be.

⚠⚠ **AND IT DID DRIFT BACK — MEASURED 2026-08-18, ONE DAY LATER, AND PAST THE
BASELINE RATHER THAN UP TO IT:**

```text
111,429   frozen baseline (2026-08-09)
110,932   four modules relocated  355874fe1   ⭐ UNDER, for one day
112,357   2026-08-18                          ⚠ +1,425 back, +928 OVER baseline
```

⛔ **and the ratchet says NOTHING about it**, because `largest_unit_lines`
carries a +2,228 growth budget and 112,357 sits inside it. ⇒ **the win was never
protected and the instrument was never going to report its loss** — which is a
sharper statement of this row's own warning than the warning was.

✔ **AND THE REPORT NOW SAYS IT** — `largest_unit_lines` prints
`[frozen 111,429, +928, budget ±2,228 within budget]`. ⛔ **the GATE is
unchanged**: same 8 findings, same exit code. A budget answers *"is this worth
failing on"*; it does not answer *"are we where we thought we were"*, and only
the second question was unasked. ⛔⛔ **deliberately NOT a tightened budget** —
Jon's ruling stands (*"the compile ratchet is an INSTRUMENT, NOT A TARGET"*) and
a tighter budget makes it more of one. Five tests pin it, including that a
DIFFERENT crate taking the title is flagged rather than compared as one number.

⭐⭐ **the largest contributor is NOT a violation of this row's standing rule,
and reading the destination's contract is what settled it.**
`authored_switch_commands` is a runtime interpreter of authored world IR, and
`ambition_platformer2d_world` opens with *"Backend-agnostic authored world IR …
simulation crates interpret them through explicit lowering seams"* — an IR crate
refusing an interpreter in its own words. ⇒ same move as the four relocations'
three refusals: the contract turned a plausible destination into an obviously
wrong one at zero cost.

⭐⭐ **AND THAT ACCOUNTING IS NOW HALF DONE — `--diff` costs seconds and runs no
build.** The edit-cost regressions split cleanly into two causes, and only one
of them is this row's doing:

```text
crate                    +dependents   +lines     +seconds
ambition_geometry              +0     +60,697     +257.8s   ← workspace GROWTH
ambition_asset_manager         +0     +61,964     +292.0s   ← workspace GROWTH
ambition_encounter            +17     +34,969     +218.6s   ← STRUCTURAL
ambition_dialog               +12     +30,649     +206.0s   ← STRUCTURAL
ambition_platformer2d         +12     +18,541     +128.1s   ← STRUCTURAL
ambition_touch_input          +12     +18,553     +128.1s   ← STRUCTURAL
```

⭐ **`+0 dependents` means nobody new depends on it — its closure simply got
bigger**, i.e. ~61k lines of ordinary feature growth landed above it in eight
days. The monolith's −10,890 today sits INSIDE that, which is why the workspace
total is up while this crate is down.

⛔ **the `+N dependents` rows are the carves' own bill.** Relocating and carving
gives the destination new dependents, so the crate BELOW gets more expensive to
edit even as the monolith gets cheaper — the inverse of the laundering trap, and
the reason a per-crate ledger cannot score this row on its own.

✔✔ **ANSWERED 2026-08-17, AND THE ANSWER REFUSES THE QUESTION'S PREMISE.** Jon,
verbatim: *"like count is a proxy, decompose as it makes sense. try not to dump
things into it to make the problem worse."*
⇒ ⛔⛔ **the compile ratchet is an INSTRUMENT, NOT A TARGET.** Do not schedule a
carve to move a number, and do not re-freeze to make a tool quiet. The 17
dependents were never the real question — carve where OWNERSHIP says so, and if
the number happens to fall, good.
⭐ **the operative half is the second clause, and it is a STANDING rule that
binds every other row in this ledger, not just D33: new work does not land in
`ambition_platformer2d_actor_monolith` because that is where its neighbours
already are.** A feature whose owner is elsewhere goes to its owner even when the
monolith is the cheaper edit. That is what stops this row needing to exist again.

⛔⛔ **THE ROW'S SCOREBOARD SAID DECOMPOSITION WAS LOSING GROUND — and it was,
for eight days. ⭐ RESOLVED THE SAME DAY; the arc is below.** The
compile ratchet's baseline was frozen 2026-08-09. Since then:

```text
largest_unit_lines  ambition_platformer2d_actor_monolith
                    111,429 → 121,822   (+10,393, budget was +2,228)
```

⚠ **and the growth is BROAD, not one bad module**, which is what makes it a
plan-level fact rather than a cleanup task:

```text
features          +4,301   (+10,260 / -5,959 — the hub, churning hard)
items             +2,255
world             +1,121
avatar            +1,040
session           +1,038
construction        +691
character_runtime   +627   (+3,050 / -2,423)
dialog              +537   ⚠ grew even though conversation was carved OUT
```

⇒ **carving one module per session does not keep pace with ordinary feature
work.** That does not make the carves wrong — it means the row cannot be judged
by "did a crate leave this session", and the honest measure is this ratchet.

⚠ **the ratchet is a REAL gate, not advisory** — `compile_ratchet.py` exits 1
by default (verified without a pipe; its own comment says *"Gates that require a
special enforcement flag are too easy to run in advisory mode accidentally"*),
and `scripts/run_tests.py` runs it. So this regression is live, not dormant.

⛔ **DELIBERATELY NOT RE-FROZEN.** The tool offers *"if this is a deliberate
landing, say so and re-freeze"* — this is not a landing anyone declared, and
re-freezing would launder ten thousand lines off the ledger exactly the way a
carve launders doc-link debt off a per-crate one. It stays red until someone
either carves it back down or states the growth as intended.

> ⭐⭐ **THE NEXT CARVE IS MEASURED, 2026-08-17 — `encounter`, and its one edge is
> NOT REAL.** Doing what this row asks (choose from current measurements) over the
> monolith's fourteen top modules:
>
> ```text
>                      lines   outward `use crate::` edges
>   features           43018   17     ← the hub; not a carve, it IS the monolith
>   character_runtime  13788    3
>   avatar              7717    7
>   boss_encounter      6940    3     (cutscene_trigger, encounter, features)
>   encounter           2168    1     ← ⭐ and the one is a RE-EXPORT
>   schedule            2384    1     (character_runtime)
>   character_sprites   1808    2     (assets, character_roster)
> ```
>
> ⭐ `encounter`'s single edge is `use crate::features::FeatureEcsWorldOverlay`,
> in ONE file — and that type is DEFINED in
> `ambition_platformer2d_shared_tangle::feature_overlay`, BELOW the monolith.
> ⭐ AND IT IS NOT SCHEDULE-PINNED, which is the trap that cost `conversation` a
> whole slice: `EncounterSimulationSchedulePlugin` already owns its registrations
> and already uses a NAMED set (`WaveEncounterDriven`). Nothing to un-chain first.
> ⚠ inward edges are `audio/plugin.rs` and `boss_encounter` — ordinary, and they
> become a dependency on the new crate.

⛔⛔ **THE INSTRUMENT WAS WRONG, AND IT IS THE OPPOSITE OF THE TRAP THE
`conversation` SLICE RECORDED.** That slice learned *"measure `use` statements,
never `crate::` occurrences"* because this repo's doc comments cite paths so
densely that a path-grep measures PROSE. **True there, and it does not generalise:
`conversation` happened to write every edge it had as a `use`.** `encounter`
writes almost none of them that way — its dependencies are **inline
fully-qualified paths in system signatures and plugin bodies**, which a
`use`-grep cannot see at all. Both greps are wrong in one direction each. ⭐ **the
honest instrument is `crate::` paths on NON-COMMENT lines**, which costs one more
`grep -v` and is the only reading that saw this:

```text
module              lines   `use crate::`   crate:: in CODE   ← the honest one
  features          43018       18                25    the hub; it IS the monolith
  character_runtime 13788        3                13
  avatar             7717        7                 9
  boss_encounter     6940        3                 3    ← agrees, genuinely
  construction       5906        3                 8
  items              4985        5                14
  abilities          4881        8                11
  world              4452        6                12
  session            2940       11                18
  schedule           2384        1                 5
  encounter          2168        1                 9    ← ⛔ NINE, not one
  projectile         2127        3                 9
  character_sprites  1808        2                 3
```

⛔ **the `use`-grep undercounts EVERY module in the table and does not undercount
them uniformly** — `boss_encounter` reports honestly (3 = 3) while `items` hides
nine edges and `encounter` hides eight. So the old column could not rank
candidates even relatively. **No carve should be chosen off it again.**

⭐⭐ **BUT THE CORRECTED NUMBER IS ALSO NOT THE VERDICT — MOST OF THOSE NINE
RESOLVE BELOW THE MONOLITH TOO, AND THE FIVE THAT DO NOT ARE THE FINDING.** Every
name `encounter` reaches through a sibling module, chased to its `pub struct` /
`pub fn`:

```text
NOT real (sibling module is a pure re-export of a LOWER crate):
  features::{ChestFeature, EncounterMob, EncounterRewardChest, FeatureId,
             GameplayBannerRequested, Opened}         → ambition_combat
  features::{apply_gameplay_banner_requests, tick_gameplay_banner,
             update_ecs_hazards}                      → ambition_combat
  features::FeatureEcsWorldOverlay                    → shared_tangle
  actor::BodyKinematics                               → platformer2d_core
  actor::PlayerEntity · physics::BaseGravity          → shared_tangle
  schedule::Platformer2dSimulationPhaseMonolith       → shared_tangle
  character_runtime::PreparedCharacterRegistry        → ambition_characters
  rooms::RoomSet                                      → platformer2d_world
  trace::{GameplayTraceBuffer, GameplayTraceEvent}    → ambition_gameplay_trace

REAL — defined in the monolith, and each one blocks the move:
  features::spawn_encounter_mob        features/ecs/spawn/mod.rs:816
  features::EncounterMobSeed           features/ecs/spawn_actors.rs:2086
  features::{clear_encounter_reward_ecs, sync_encounter_reward_chests_ecs}
                                       features/ecs/encounter_rewards.rs:16,41
  features::FeatureWorldOverlaySet     world/overlay.rs:35
  world::gated_lock_walls::sync_authored_gated_lock_walls
                                       world/gated_lock_walls.rs:152
  crate::ActorDiedMessage              lib.rs:156
```

⛔⛔ **THE LOAD-BEARING BLOCKER IS `spawn_encounter_mob` — `drive_wave_encounters`
SPAWNS ACTORS THROUGH THE MONOLITH'S ACTOR CONSTRUCTION PATH** (`systems.rs:335`,
handing it an `EncounterMobSeed`). That is not an ordering nuisance a step-1.5
can name away; it is **actor construction**, which this plan's own Wave G says
leaves LAST, after the outer domains. A wave arena's whole job is *spawn these
characters, watch them die* — so `encounter` cannot precede the spawner it calls.

⛔ **AND IT IS SCHEDULE-PINNED AFTER ALL, IN THREE PLACES**, all in
`EncounterSimulationSchedulePlugin` (`encounter/mod.rs`): (1) an **anonymous
`.chain()`** interleaving `drive_wave_encounters` with two banner systems — the
exact shape step 1.5 deleted, though the mildest instance since both banner
systems are `ambition_combat` and a carved crate could still name them; (2)
`contribute_encounter_lock_walls` ordered `.after(crate::features::FeatureWorldOverlaySet)`,
**a set defined in the monolith** — step 1.5's lesson failing in the exact way it
warned about, *the ordering NAME must live where the module can still reach it*;
(3) the plugin registers `crate::world::gated_lock_walls::sync_authored_gated_lock_walls`,
**a foreign module's system**, deliberately, so the two roads into `gate_solids`
are visible in one place. (2) and (3) are real work; (1) is cosmetic here.

⭐ **AND `encounter` ALREADY HAD ITS CARVE — `crates/ambition_encounter` EXISTS.**
Lifecycle, commands, objectives, participants, timeline, waves, registry, music,
rewards, spec and staging all live there. The 2,168 lines still in the monolith
are what the module's own header calls the residue: *"Facade module … Gameplay-core
keeps the adapters that still touch LDtk, ECS spawning, player/body queries,
feature overlays, banners, save/quest plumbing, and schedule sets."* ⇒ **the
header was accurate and the row proposed re-carving what had already been carved.**
Six of the twelve files are three-line `pub use ambition_encounter::…;` compat
shims. ⚠ the name `ambition_encounter` is therefore TAKEN, which is by itself a
signal a candidate deserves a second look.

⚠⚠ **AND THE INWARD EDGES ARE BACKWARDS FROM WHAT THE ROW SAID** — they are the
laundered ones. `audio/plugin.rs:200` and `boss_encounter` (3 sites) name
`crate::encounter::EncounterMusicRequest`, which is a bare re-export of
`ambition_encounter::music`. So do `music/intent.rs` and `session/reset/mod.rs`
inside the monolith, and `ambition_app`'s + `ambition_demo_mary_o`'s tests through
`ambition_platformer2d::actors::encounter::` — while `ambition_platformer2d_runtime`
and `ambition_content` already name `ambition_encounter` directly. **Two roads to
one type, and the shorter one is the facade.** ⚠ symmetrically,
`encounter/switches.rs:57–59` reaches its OWN `SwitchFeature`/`SwitchOn` back
through `crate::features`, which re-exports them from `crate::encounter`
(`features/mod.rs:130`) — a re-export LOOP.

⇒ ▢ **THE NEXT SLICE HERE IS THE DE-LAUNDERING, NOT A CARVE.** Repoint every
`crate::encounter::EncounterMusicRequest` / `actors::encounter::` consumer at
`ambition_encounter`, close the `switches` self-loop, and delete the six compat
shim files — ~12 sites across four crates, no new crate, no lockfile, no
`critical_path_crates` movement. It removes both inward edges and shrinks the
residue to the adapters that genuinely cannot leave. ⭐ same shape as the LDtk
compat-facade deletion this row already banked: **what it buys is honesty and one
fewer historical path.**

✔✔ **THE DE-LAUNDERING LANDED 2026-08-17 — ALL SIX SHIMS ARE DEAD AND
`encounter/mod.rs` NOW RE-EXPORTS NOTHING IT DOES NOT DEFINE.** The facade went
from **39 exported names (26 of them `ambition_encounter`'s) to 13, all
monolith-owned** — the four adapters (`load_encounter_specs_from_ldtk`,
`contribute_encounter_lock_walls`, the switch table, the wave systems).
Measured with the honest instrument (`crate::` on NON-COMMENT lines):

```text
                         before   after
  encounter → siblings     40       38   sites   (9 distinct modules → 9)
  siblings  → encounter    29        6   sites   ← ⭐ THE RESULT
  ambition_platformer2d::actors::encounter:: consumers
                           11        5   sites   (all 5 monolith-owned)
```

⭐ **THE OUTWARD NUMBER BARELY MOVING IS THE POINT, AND IT IS THE MEASUREMENT
LESSON REPEATING.** De-laundering removes edges that were never real; the two it
dropped (`crate::features::SwitchFeature/SwitchOn`, a module reaching its OWN
types back through the hub) are the whole switch loop, and the distinct-module
count cannot move because `features` still carries 24 other names. **The
direction that changed is INWARD: 29 → 6, and all six of those now resolve to
`encounter/switches.rs`** — types the monolith genuinely defines. Every remaining
`crate::encounter::` in the tree names something encounter OWNS. ⛔ the residue is
2,145 lines, essentially unchanged — this bought honesty, not size, exactly as
the row predicted.

⚠ **the shim deletion was asserted structurally, not by eye:** the same grep for
`mod {events,lifecycle_reexports,music,registry,rewards,spec};` and their `::`
paths returns **21 hits on `HEAD`** and **3 after** — and all three are
`ambition_encounter::spec::default_encounter_reward()`, the owning crate's real
module. A grep that returns nothing only means something if you showed it
returning something first.

⇒ ✔ **AND THE CANDIDATE LIST WAS RE-RANKED — `boss_encounter`'s PRECONDITION IS
CLEARED AND IT IS STILL NOT THE NEXT CARVE.** It reached three sibling modules
(`features`, `cutscene_trigger`, `encounter`); the `encounter` edge was those
three `EncounterMusicRequest` sites and is now **gone — two modules, not three**.
⛔ **but "3 = 3" was never a size, it was a count of MODULES, and chasing them
kills the candidate:** `boss_encounter` carries **155 inward `crate::boss_encounter::`
sites** from its siblings, and its outward edges land on **boss vocabulary the
monolith itself defines inside the `features` hub** — `features/ecs/boss_clusters.rs`
(`BossConfig`, `BossEncounter`, `BossRef`, `boss_is_cleared`), `BossOverrides`
(`features/ecs/spawn_actors.rs:111`) and `sync_boss_reward_chests_ecs`
(`features/ecs/encounter_rewards.rs:98`). ⇒ **the boss's own data model lives in
`features`, so a carve moves the boundary, not the code.** The next slice here is
either that (relocate `boss_clusters` to `boss_encounter`, where it belongs) or a
different candidate entirely — not a `boss_encounter` Cargo.toml.

✔✔ **THE RELOCATION LANDED 2026-08-17 — THE BOSS DATA MODEL NOW LIVES IN
`boss_encounter`, AND THE BIDIRECTIONAL EDGE IS ONE-WAY.** `boss_clusters.rs`
(430 lines) moved to `boss_encounter/clusters.rs`; `BossOverrides` moved out of
`features/ecs/spawn_actors.rs` to sit beside the components it tweaks; and
`sync_boss_reward_chests_ecs` moved into `boss_encounter/rewards.rs` — a file
that existed **only** as a code-free placeholder whose own doc said *"boss
reward-chest sync now lives in `crate::features`"*. ⛔ **no re-export was left
behind**: `features` no longer names any of the ten symbols, and all 55
`features::<boss symbol>` call sites across seven crates were re-pointed.

Measured with the honest instrument — **`crate::` on NON-COMMENT lines, counted
in SITES, both directions**:

```text
                              before   after
  boss_encounter → siblings     51       25   sites  ← ⭐ the result
      of which crate::features   49       21
      new (arrived with the moved code: platformer_runtime, combat)  —  2
  siblings → boss_encounter    155      201   sites  ← ⚠ UP, and correct
  features/mod.rs exported names  280     270          (−10, all boss)
  features tree                43,017  42,339  lines   (−678)
```

⭐⭐ **THE INWARD NUMBER GOING UP IS THE HONEST OUTCOME, NOT A REGRESSION.** Those
46 new `crate::boss_encounter::` sites are the same edges that were already there
reading `crate::features::BossConfig` — the hub was laundering a boss dependency
as a features dependency. Relocation does not delete an edge a caller genuinely
has; **it makes it say whose it is.** The number that had to fall is the outward
one, and it did: 49 → 21.

⭐ **AND NONE OF THE 21 SURVIVORS IS BOSS VOCABULARY.** Twenty of them name types
that live BELOW the monolith and are merely re-exported by the hub —
`BodyKinematics` (`platformer2d_core`), `CenteredAabb` / `ChestFeature` /
`FeatureId` / `Opened` / `FallingChest` / `BossRewardChest` / `GameplayBanner`
(`ambition_combat`), `FeatureSimEntity` (`shared_tangle`). The twenty-first is
`MountDied`, genuinely defined in `features::ecs::mount` — a real cross-domain
message, not laundry. ⇒ **the fable-review 2026-07-15 blocker is cleared**: it
named exactly this file (*"the single blocker is that boss cluster ECS components
live in features/ecs while catalog/behavior/sprites live in boss_encounter"*).

⚠ **NOTHING REFUSED — but two things were checked FOR a refusal and passed.**
(1) `BossOverrides` looked construction-pinned, living in the spawn module; it is
not — it is a plain `Component` of authored tweak DATA, written once at spawn and
read only by `update_boss_encounters` / `sync_boss_encounter_entities`, both in
`boss_encounter`. `spawn_actors.rs` now imports it like any other component it
inserts. (2) `sync_boss_reward_chests_ecs` looked table-pinned, sharing a file
with `sync_encounter_reward_chests_ecs` and `clear_encounter_reward_ecs`; it is
not — those two share only the file's `use super::*`, not a table, and the boss
one has exactly **one** production caller (`boss_encounter::systems`). Its mob
siblings stayed put: their `EncounterMob` wave vocabulary is `encounter`'s, not
the boss's.

⭐ **ONE PRE-EXISTING VIOLATION SURFACED, which is the facade-deletion hazard
AGENTS.md names.** Splitting a grouped import
(`use …monolith::features::{BossClusterRef, FeatureEcsWorldOverlay}`) left
`FeatureEcsWorldOverlay` on its own line and `engine.f2-consumers-use-canonical-crates`
fired — the edge had been hiding inside a mixed brace. Fixed by naming its real
home (`ambition_platformer2d_shared_tangle::feature_overlay`), not by waiving.

⚠ **and two ROLLBACK ORACLE strings had to move with the type**, because they are
`std::any::type_name` text, not paths a compiler checks:
`…::features::ecs::boss_clusters::BossConfig` and
`…::features::ecs::spawn_actors::BossOverrides` → `…::boss_encounter::clusters::*`.
A relocation of a rollback-registered component always owes that edit.

The absence was asserted structurally, not by eye: `boss_clusters` as a path went
**62 sites → 1** (the survivor is a comment in `boss_encounter/mod.rs` recording
where the module came from), and `features::<boss symbol>` went **55 → 0**.
Green: `cargo check --workspace --all-targets`, monolith lib (1,245),
`ambition_app --test app_it`, `ambition_workspace_policy` (34), and
`check_absence_contracts.py --check` (29 of 29).

✔✔✔ **THE CARVE LANDED 2026-08-17 — `crates/ambition_boss_encounter`.** 7,635
lines left the monolith; a second relocation of `features/ecs/bosses/` that an
earlier reading of the 201 inward sites thought was needed first was never
needed — inward sites are callers naming the domain, not a blocker.

```text
largest_unit_lines   ambition_platformer2d_actor_monolith
                     121,822 → 114,139   (−7,683; still +2,710 over the frozen
                                          111,429, was +10,393 this morning)
critical_path_crates      13 → 13        ⭐ NO new hop
```

⛔⛔ **THE 201 INWARD SITES WERE NEVER A BLOCKER, AND THE DIRECTION ERROR IS THE
FINDING.** An inward site is a CALLER naming the domain; after the carve it spells
`ambition_boss_encounter::` instead of `crate::boss_encounter::` and compiles
unchanged — a rename, not a dependency the departing crate must satisfy. **Only
OUTWARD edges block a carve**, because those are the ones cargo refuses. So
`features/ecs/bosses/` never had to move: it CALLS the boss domain, it is not
called BY it. ⇒ **count both directions, adjudicate on the outward one.**

⭐ **the outward list had TWO real names, and both moved DOWN, not across.**
Measured with the honest instrument (`crate::` on NON-COMMENT lines, in SITES);
each distinct path chased to its `pub struct` / `pub fn`:

```text
module            lines   out sites/mods   in sites/mods   ← the ranking that chose it
  features        42,343     579 / 21        499 / 20   the hub; it IS the monolith
  character_runtime 13,788    87 / 12        288 / 10
  avatar           7,717     164 /  8         85 / 14   ⛔ 15 of 164 are one type in
                                                          character_runtime — pinned
  boss_encounter   7,635      24 /  3        201 / 12   ← ⭐ CHOSEN
  construction     5,906     245 /  6         39 /  4   ⛔ Wave G leaves LAST
  items            4,991      85 / 13         44 /  4
  abilities        4,881     136 / 10         30 /  4   ⛔ calls spawn_runtime_minion
  world            4,452      55 /  9         53 /  4
```

⛔ **`out mods` alone would have picked the wrong one** — `character_sprites` (3)
and `boss_encounter` (3) tie, and `world` (9) looks worse than `avatar` (8) while
being far cleaner. The number that decided it was the outward SITES chased to a
DEFINITION: eleven of `boss_encounter`'s thirteen distinct paths resolved to crates
already BELOW the monolith and the hub was merely re-exporting them
(`BodyKinematics`, `CenteredAabb`, `FeatureId`, `FeatureSimEntity`,
`GameplayBanner`, `ChestFeature`, `Opened`, `FallingChest`, `BossRewardChest`,
`falling_chest::settled_chest_center`). The two that were real:

* `CutsceneTriggerQueue` → `ambition_cutscene`, beside the script format it
  triggers. `crate::cutscene_trigger` is deleted, not re-exported.
* `MountDied` → `ambition_platformer2d_shared_tangle::body`, below BOTH domains
  that share it — the mount coupling WRITES it, the boss crate READS it. Same move
  and same reason as step 1.5 putting `FeatureInteractionSet` there. ⛔ imported
  privately into `features/ecs/mount`, never re-exported, so nothing can keep
  spelling it `features::MountDied`.

⭐ **the ORPHAN RULE adjudicated one more file, exactly as `snapshot_impls.rs`'s own
header promised**: `impl SnapshotCursor for BossEncounter` stopped compiling the
moment the type crossed the crate line and moved to `clusters.rs` with it. The wire
format did NOT change — `rollback-wire-format-is-frozen` reports the same 357 names
and 85 encoded types.

⚠ **the umbrella, not a new edge, is how a demo reaches it.** Naming
`ambition_boss_encounter` directly from `ambition_app` / `demo_mary_o` /
`demo_sanic` reddened three `game.*-umbrella-only` policies, correctly:
`ambition_platformer2d` re-exports every domain crate under a short name for
exactly this. Only `platformer2d_runtime`, `platformer2d_provider`, `sim_view` and
`ambition_content` declare the edge — and the runtime allowlist gained its entry in
the same commit, **the fifth time that list has lagged a runtime dependency**.

⚠ **the ledgers a carve launders, all moved in the same commit:**
`check_doc_link_ratchet.py`'s `CRATES` gained `ambition_boss_encounter` (monolith
109 → 107, new crate 2, total 191 → 192 — the carve is link-neutral);
`capability-footprint-baseline.json` 43 → 44 crates and 16 → 17 never-asked-for,
with the argument written in; three lockfiles; `engine.toml`'s runtime allowlist
plus two now-stale absence strings re-pointed so they still guard something.
⚠ **and one honest cost UP:** `ambition_geometry`'s `worst_edit_cost` goes 48 → 49
crates (+17.5s) — one more compilation unit sits above it.

⇒ ✔✔✔ **THE RATCHET IS GREEN — `largest_unit_lines` 114,139 → 110,929, BELOW
the 111,429 frozen on 2026-08-09 for the first time since it was frozen.**
`critical_path_crates` stayed **13** — no new hop, because no new crate was
made. Landed 2026-08-17.

⛔⛔ **BOTH NAMED CANDIDATES WERE REFUSED, AND THEY WERE REFUSED FOR THE SAME
REASON `encounter` WAS.** Chasing every outward site to its `pub struct` /
`pub fn` — and splitting PRODUCTION from TEST, which no previous measurement on
this row did:

```text
module   lines   real-out PROD   real-out TEST   re-export (not real)
  items   4,991      30 sites         5              49
  world   4,452      11 sites        26              17
```

⭐ **the split is what the earlier column was hiding.** `world` looks far
cleaner than `items` by production sites (11 vs 30) — and 20 of `items`' 30 are
one thing, `ItemPickupPlugin` registering **eighteen `abilities` systems and two
`shrine` systems** into its own sets, which is step 1.5's shape and mechanical to
fix. ⛔ **but both keep a `construction` edge**: `items/pickup/mod.rs` rebuilds a
carried object from `construction::authored_occurrence_request` +
`ActorConstructionParams::GroundItem`, and `world/rooms/{stage,transaction}.rs`
stage actors through `ActorConstructionPlan` / `verify_rig_composition`. That is
**actor construction, which Wave G says leaves LAST** — the identical blocker
(`features::spawn_encounter_mob`) that refused the `encounter` carve this
morning. ⚠ and `world`'s is BIDIRECTIONAL: `construction/mod.rs:47` imports
`crate::world::placements::ActorPlacementContext`.

⇒ **the direction rule holds and gains a corollary: only OUTWARD edges block a
carve, and the one that blocks BOTH remaining outer domains is the same one.
Until `construction` moves, `items` and `world` are not carves.**

⭐⭐ **SO THE SLICE MEASURED ALL FORTY MODULES WITH THAT INSTRUMENT INSTEAD, AND
THE ANSWER WAS NOT A CARVE AT ALL — IT WAS FOUR RELOCATIONS INTO CRATES THAT
ALREADY EXISTED.** Ranking every top-level module by real production outward
edges surfaced a population nobody had counted: **modules with ZERO of them,
whose owning crate is already in the tree.**

```text
                                          real-out   destination
  persistence   1,336  DELETED — dead       0/0      (nothing; see below)
  menu            809  → ambition_menu      0/0      map.rs was already there
  dialog          672  → ambition_conversation 0/0   the dialogue authority
  equipment       388  → ambition_items     0/0      it IS an item
                ─────
                 3,205  measured 3,207 in the monolith's line count
```

⛔⛔ **`persistence` WAS DEAD, and the falsifier is the finding.** Its 1,336
lines were an eleven-line re-export adapter, a 36-line settings facade, and
`settings/model` — 1,289 lines of pause-menu vocabulary. **All eight of its
public names (`SettingsPage`, `SettingsItem`, `SettingsAction`,
`SettingsOutcome`, `DevToggleSnapshot`, `apply_action`, `apply_display_mode` (⚠ GONE — left the monolith at `355874fe1`; the logic is `ambition_settings_menu::settings::apply` now),
`PLAYER_DAMAGE_SLIDER_MAX`) have zero code references anywhere in `crates/` or
`game/`** — every hit outside the module is a doc comment. ⭐ the instrument was
shown WORKING first: the same grep run on `TextureResolutionScale` and
`reconcile_equipment_grants` returns real call sites. The remaining 25
`crate::persistence::settings::…` paths were all re-exports of
`ambition_persistence::settings::{TextureResolutionScale, AudioSettings,
TriggerEdgeState}` and were repointed at their real home.

⭐ **and each destination was chosen by what the crate ALREADY OWNED, not by
where there was room.** `ambition_menu::map` already held `MapMenuState` and the
monolith held the renderer that imported it — the two halves of the Map tab were
a crate apart. `ambition_conversation` already owns the conversation authority
the Yarn runtime is driven by. `ambition_items` already owns the item catalog
`equipment` grants verbs from.

⛔⛔ **AND THE DESTINATION CHOICE IS WHERE THIS SLICE ALMOST WENT WRONG — THREE
OTHER "OBVIOUS" HOMES WERE REFUSED BY THE CRATES THEMSELVES.** `ambition_dialog`
declares itself *"content-free — the host maps `DialogState.active` onto its own
session mode"*, which is precisely what the moved glue does, so it cannot host
it. `ambition_settings_menu` is the *renderer-agnostic* IR and carries no
`bevy`. `ambition_menu`'s manifest says its trimmed bevy feature set is
*"load-bearing for the WHOLE workspace"*. ⇒ **read the destination's stated
contract before moving code into it; a crate that refuses your dependency is
telling you the code does not belong there.** The Map tab passed that test
because its three new edges (`ambition_input`,
`ambition_platformer2d_shared_tangle`, `ambition_platformer2d_world`) widen
nothing: `ambition_menu` already sat downstream of `ambition_platformer2d_core`
through `ambition_ui_nav → ambition_input`, so **no crate joined
`ambition_geometry`'s or `ambition_platformer2d_core`'s rebuild set** — both
`edit_cost` ledgers moved DOWN, not up.

⭐ **two declared dependency EDGES died with the moves, which a line count cannot
see.** The monolith no longer names `ambition_menu` at all, and `bevy_yarnspinner`
+ `yarnspinner` left its manifest entirely (its `ui` feature now only FORWARDS to
`ambition_conversation/ui` + `ambition_dialog/ui`). ⚠ one edge was added on
purpose: `ambition_platformer2d` now re-exports `ambition_conversation as
conversation`, because a game reaches a domain crate through the umbrella —
`ambition_platformer2d::conversation::dialog::YarnBridgePlugin`.

⚠ **the ledgers a relocation launders, all moved in the same commit** — and this
shape launders *better* than a carve does, because there is no new `Cargo.toml`
to remind anyone: `check_doc_link_ratchet.py`'s `CRATES` gained
**`ambition_items` and `ambition_menu`** (monolith 107 → 103; the two
destinations were carrying 5 and 3 unlisted); four lockfiles under `fixtures/`
and `examples/` refreshed; **seven** workspace policies re-pointed rather than
deleted, two of which had to change SIDES —
`game.lib-menu-keeps-map` REQUIRED `src/menu/map` in the monolith and is now
`game.lib-menu-gone` forbidding `src/menu`, and
`engine.actors-settings-surfaces-controls` asserted the persistence facade
re-surfaced `controls` and is now `engine.actors-persistence-facade-gone`
forbidding the whole directory. ⭐ `capability-footprint-baseline.json` needed
NO edit — 44 crates / 17 never-asked-for, unchanged — and
`rollback-wire-format-is-frozen` reports the same **357 names, 85 encoded
types**: nothing that moved was rollback-registered.

Green: `cargo check -p ambition_app --all-targets`, `cargo test --workspace
--lib`, `ambition_app --test app_it` (412), `ambition_demo_smash_app` (32),
`ambition_workspace_policy` (34), `check_absence_contracts.py --check` (29/29).

⇒ ▢ **NEXT, and the row's shape has changed.** The ratchet is green, so the next
slice is not a race against a number. The remaining outer domains are
`audio`+`music` (1,842 lines, ZERO real outward edges either direction, and
**nothing else in the monolith references them** — three `Platformer2dAudioPlugin`
adds and one rollback-oracle string are its whole consumer set) and
`character_roster`/`cutscene`; each wants a home that will accept it, and
`audio` additionally wants the monolith's `audio`/`web_audio` persona features
forwarded to wherever it lands, which is the real cost there. ⛔ **`items` and
`world` are blocked behind `construction` and should not be attempted again
until it moves** — and the `abilities`-registration half of `items` (20 of its 30
production edges) is a step-1.5 slice that can land independently and would
make `items` a genuine candidate the moment Wave G opens.
 Prefer boundaries that improve capability closure, compile isolation,
public API shape or change amplification.

⭐⭐ **THE `audio`+`music` CANDIDATE IS MEASURED, 2026-08-18 — AND IT NEEDS NO NEW
CRATE, because the destination already exists and its contract ACCEPTS it.**

```text
1,842 lines   972 production, 870 tests
    3         outward `use crate::` statements to the rest of the monolith
    1         of those is a GENUINE edge — and it is test-only
```

The three, chased to their definitions rather than counted:

```text
crate::rooms::RoomMusicRequest             → ambition_platformer2d_world   RE-EXPORT, below
crate::assets::game_assets::GameAssetConfig → ambition_sprite_sheet        RE-EXPORT, below (test)
crate::session::data::{MusicRegistry,…}    → ambition_audio               RE-EXPORT, below
crate::session::data::{fixture_*_registry} → session/data.rs:51,57        ⚠ REAL, pub(crate), TEST
```

⇒ **one real edge, `pub(crate)`, and only tests use it** — so the carve's actual
cost is finding a home for two fixture builders, not untangling a dependency.

⭐⭐ **AND THE DESTINATION IS `ambition_audio`, WHICH ALREADY OWNS THE PARTS.**
It opens *"Content-free audio data/runtime layer"* and already ships `library`,
`render`, `music`, `mix`, `web_unlock` plus three Bevy plugins. The monolith's
`audio/plugin.rs` (422 lines) is largely a COMPOSITION of that crate: it installs
three `ambition_audio` plugins and initialises eight `ambition_audio` resources.
Monolith-specific state is four items — `RadioStationState`, `AudioEnvironment`,
`DefaultMusicStarted`, `MusicIntent`. ⇒ same shape as the `boss_encounter`
relocation: an existing owner, no new crate, no new hop.

⛔⛔ **AND A STALE DOC COMMENT NEARLY REFUSED THE CARVE FOR THE WRONG REASON.**
`music/mod.rs` describes itself as carrying *"authored goblin cue data"*. Under
the read-the-destination's-contract rule that reads as an instant refusal —
content-free crate, authored content, done. ⚠ **it is not true any more.**
Grepping the two modules for named game content finds ZERO string ids naming a
track, boss, room, or character; the only tuning value is one
`LARGE_BRUTE_DELAY_SECONDS = 3.5`. The goblin cue left long ago and lives at
`game/ambition_content/src/music.rs` (`FIRST_GOBLIN_CUE_ID`,
`MOB_LAB_ENCOUNTER_ID`). ⇒ **the destination's contract must be read against the
CODE's present content, not the source module's description of itself** — a
stale self-description is a refusal the code no longer earns, which is the
inverse of the failure this row usually catalogues and costs just as much.

⛔⛔ **AND THE `use crate::` COUNT DID NOT PRICE THE MOVE — the EXTERNAL crate
deps did, and they split the candidate in two.** Three outward intra-crate edges
looked like a free relocation. What actually costs is what each module imports
from OTHER crates, because those become the destination's new dependencies:

```text
audio/environment.rs   bevy · bevy_kira_audio · ambition_audio::library        ← nothing else
audio/plugin.rs        + ambition_platformer2d_shared_tangle · ambition_dev_tools
music/intent.rs        + ambition_encounter · ambition_platformer2d_world
```

⇒ **moving the whole 1,842 lines would make `ambition_audio` — today a leaf on
`ambition_sfx`, bevy, bevy_kira_audio, ron, serde — depend on `ambition_encounter`
and `ambition_platformer2d_world`.** No cycle (checked: none of the five names
`ambition_audio` back), but a foundation acquiring mid-level dependencies is the
carve making the graph worse, which is exactly what Jon's *"try not to dump
things into it"* rules out.

⭐ **so the shippable slice is `audio/environment.rs` ALONE** — 238 production +
181 test lines, realtime channel-attenuation DSP whose only outside import is
`ambition_audio::library::{amplitude_to_decibels, MusicChannel, SfxChannel}`.
It already reaches INTO the destination for every type it uses; moving it adds
the destination not one new dependency. `plugin.rs` and `music/` stay until
someone wants a music-direction crate ABOVE `ambition_encounter`.

⭐ **the transferable half: an intra-crate `use crate::` census answers "what
would break", and the EXTERNAL import census answers "what would the destination
inherit".** Only the second one prices a crate boundary, and this candidate looks
free by the first measure and expensive by the second.

⚠ still not measured: whether `ambition_audio` can carry the monolith's
`audio`/`web_audio` persona features (it has `kira`), and the five-lockfile /
contracts-job bill every crate-boundary change here has paid.

⭐⭐ **MEASURED 2026-08-15, and the headline is a REFUTATION worth more than the
carve it refused.** `conversation` is the strongest candidate by every import
measure — **1,836 lines, zero edges out, zero edges in** — and its own header
claimed *"the carve is a Cargo.toml"*. ⛔ **it is not.**
`features::FeatureInteractionSchedulePlugin` performs every registration it has
and interleaves three of its systems into **ONE anonymous `.chain()`** with the
switch and chest systems, every interleave load-bearing and documented only in
prose at the call site. ⇒ **a module with zero inward imports can still be pinned
by the SCHEDULE.** Step 1.5 is therefore a `ConversationPlugin` owning those
registrations and stating the cross-domain order as **named sets** — a
simulation-ordering change that wants a session able to run the suite.

✔✔ **STEP 1.5 LANDED 2026-08-16 (`bc187bc98`) — THE CHAIN IS GONE AND THE CARVE
IS NOW GENUINELY A CARGO.TOML.** `FeatureInteractionSchedulePlugin` held ONE
anonymous `.chain()` of **ten** systems across four domains (this row said
eleven; it was ten), and the whole cross-domain ordering contract was adjacency
in that tuple plus five prose comments. `FeatureInteractionSet` now names the
phases — `NarrativeIntake · Actuate · Continuity · CutBarkCast · HoldProjection ·
WorldObjects · SwitchIndex` — and every rationale lives on the variant it
explains rather than beside the system it happened to precede. It follows the
existing `ProgressionSet` / `PlayerInputSet` template rather than inventing a
shape: the phase owner chains the SET LIST once, each domain states only which
phase it is in.

⭐⭐⭐ **AND THE PLACEMENT IS THE TRANSFERABLE LESSON — the set vocabulary lives
in `shared_tangle`, BELOW the monolith, on purpose.** A set enum defined in
`features` would have re-pinned `conversation` by the schedule the moment it
stopped importing `features` — *the same bug, one level up.* ⇒ **when you name an
ordering so a module can leave, the NAME has to live somewhere the module can
still reach after it has left.**

⭐⭐ **RE-MEASURED 2026-08-17: THE CARVE IS STILL READY, and the numbers are
now exact.**

```text
conversation -> monolith    0 real `use` statements      ⇒ the new crate need
                                                          not depend on the one
                                                          it leaves. THIS is the
                                                          direction that decides
                                                          a carve.
monolith -> conversation    1 `use` (features/ecs/interact.rs:19,
                            `DialogueDispatch`) + 35 inline paths
                                                        ⇒ ordinary, and becomes
                                                          the monolith depending
                                                          on a crate.
size                        2,734 lines = ~2,023 code + 711 test
                            (the row said 1,836; it grew, it did not rot)
```

⛔⛔ **AND A MEASUREMENT TRAP WORTH MORE THAN THE NUMBERS.** `grep -r "crate::"`
over this module reports edges to `participant_seat`, `features`,
`character_runtime`, `items` and `dialog` — and **every single one is a DOC
COMMENT**. This repository's `//!` and `///` blocks cite paths so heavily that a
path-grep measures PROSE, not dependency. I nearly filed this row as stale on
that reading. ⇒ **measure `use` statements, never `crate::` occurrences.**

⚠ the remaining cost is the one the repo already knows: a new crate is a new dep
edge, which is FIVE lockfiles and the contracts job — see
a new dep edge fails the contracts job, and one of the five lockfiles is
invisible to `git status`.

⭐ **`ConversationPlugin` owns** `ActiveConversation`, the `ConversationCutBark`
port channel, the `ConversationEnded` ledger install, the `Update` presentation
pair and its three sim systems. ⚠ **only ONE of the seven `NarrativeInputPlugin`
installs moved, and that is the seam rather than a shortfall**: a ledger payload
belongs to whoever CONSUMES it — three are `features` types a carved crate could
not name, three more are applied by `features::bus` and `items::narrative`.
**Conversation provides the mechanism, not the vocabulary.**

✔ **four schedule-graph tests assert the edges AS THE PLUGIN COMPOSES THEM** —
set-to-set dependencies, nesting in the containing phase, each system's
membership, and that nothing sits in the phase outside a named set — all four
probe-falsified by breaking the composition rather than reasoned about. ⭐ that
is the shape that beats a hand-listed chain, which pins the function and not the
wiring.

✔✔ **STEP 2 LANDED 2026-08-17 — `crates/ambition_conversation` EXISTS AND THE
CARVE REALLY WAS A CARGO.TOML.** The measurements held on re-verification: zero
`use crate::` in the whole module, ten files, 2,734 lines. ⭐ **not one line
inside the moved code changed shape** — `use super::authority::…` resolves to the
crate root exactly as it resolved to the parent module, so every internal path
survived `mod.rs` → `lib.rs` untouched. The single edit inside the carved code
was a `warn!` `target:` string that still spelled the monolith. Everything else
was manifests and `crate::conversation::` → `ambition_conversation::` at the
CALLERS: seven files in the monolith, two in `ambition_platformer2d_runtime`, six
in `ambition_content`. **Name from the module's own header, which proposed it in
2026-08-07**; the `ambition_platformer2d_*` prefix belongs to crates that are
platformer-shaped, and conversation continuity is not.

⭐ **NOTHING had to move to `shared_tangle` first.** Step 1.5 had already put the
only shared vocabulary the crate reaches upward for — `FeatureInteractionSet`,
`SimScheduleExt`, `SimId` — below the monolith, which is precisely the lesson it
was recorded for. Everything else the crate names (`ambition_characters`,
`ambition_combat`, `ambition_dialog`, `ambition_input`, `ambition_interaction`,
`ambition_platformer2d_core`, `ambition_time`) was already beneath it. The carve
found no second pin.

⛔⛔ **AND IT COST A CRITICAL-PATH HOP — `critical_path_crates` 12 → 13, which
this plan predicted would stay at 12.** MEASURED, not inferred: recomputing the
first-party height with `ambition_conversation` folded back into the monolith
gives 12 and with it carved gives 13. The lengthened chain is `conversation →
ambition_dialog → ambition_ui_nav → ambition_input → ambition_platformer2d_core →
ambition_geometry` — inserting a layer under `ambition_dialog` pushed that whole
tail down one hop. ⭐ **this is exactly the regression that number is guarded for:
every size metric can improve while the serial chain, and so the wall clock, gets
worse.** ⚠ read it in HOPS — rustc releases a dependent at the predecessor's
`rmeta`, so a chain edge serialises only the frontend, and this repo has already
measured a naive chain-of-durations overshooting a real build by 2.2x.
⚠ **the ratchet baseline was deliberately NOT re-frozen**: it is frozen at
`208cf8acf937` (2026-08-09) and reports NINE findings, eight of them eight days
of unrelated growth. Re-freezing under a carve commit would launder them.
⚠ **and this crate's seconds are a PLACEHOLDER** — it is unpriced, estimated at
the population median 2.9059 ms/line, and size predicts compile cost with
R² = 0.12. `scripts/compile_collect.py` is what makes it real.

⇒ **the five-lockfile / contracts cost arrived exactly as documented and cost
nothing to pay.** Root, `fixtures/minimal_game` and `examples/capability_demo`
changed and are committed; `examples/portal_tutorial` did not move (predicted);
`fixtures/external_consumer` was re-resolved and is gitignored, so it is
correctly absent from the diff. `capability-footprint-may-not-grow` went RED on
`ambition_conversation entered the consumer's closure` and the baseline moved in
the same commit — **15 → 16 unwanted crates, and the carve did not CAUSE that, it
NAMED it**: the same code was already linked inside the monolith under a name the
counter could not see. `ambition_workspace_policy` went red on
`engine.runtime-manifest-allow` — **the fourth time that list has lagged a runtime
dep by one file**, which its own comments record three times already.

⭐ **the "is it really a carve" check is CARGO ITSELF, and the probe is the
finding.** Adding `[dependencies] ambition_platformer2d_actor_monolith` back to
the new crate does not fail a policy — **cargo refuses to resolve the workspace
at all**, naming the cycle. ⛔ so a denylist for that edge would be a check that
cannot fail. ⭐⭐ **but `[dev-dependencies]` is a real hole and cargo allows it on
purpose** (the monolith relies on that itself), so one test reaching back for a
fixture would rebuild the whole monolith to build this crate's tests. ONE policy
guards that — `engine.conversation-does-not-depend-on-what-it-left`, probe-
falsified by adding the dev-dep and watching it fire. The source-text twin that
was drafted alongside it was DELETED before landing: it guarded the same hole
twice, and this repo's own rule is that a guard with no failure behind it is
ceremony.

⇒ ▢ **STEP 3 IS STILL OPEN AND IS NOT WHAT IT LOOKED LIKE.** `ambition_dialog`
does NOT become a `[dev-dependency]` of the monolith, because `dialog.rs` (135
lines, `ui`-gated) is still a production namer. ⭐ that file is the next slice and
it is clean — **zero `crate::` edges**, naming only `ambition_dialog`,
`ambition_input` and `shared_tangle`. Its cost is a `ui` feature on the carved
crate forwarding to `ambition_dialog/ui`, which is why it stayed out of a move
that was otherwise a manifest. ⛔ and it buys no footprint either: the monolith's
edge to `ambition_conversation` is unconditional, so `ambition_dialog` and
`ambition_ui_nav` reach a movement-only game regardless. Shedding the capability
needs OPTIONALITY at the monolith's edge, not another move.

⇒ **every other leaf is NOT YET on this plan's own scorecard:** `menu` is the
sole namer of `ambition_menu`, but the crate also arrives through render and the
host, so the consumer footprint stays flat — the lesson `ambition_ui_nav` already
paid for. `affordances`, `gravity`, `snapshot_impls` and `action_scheme` remove
**no Cargo edge at all.**

✔ **one real deletion landed with the measurement: the LDtk compat facade.** Six
lines of blanket `pub use` whose own doc named the plan it waited on. ⚠ **the plan
said one consumer remained; it was EIGHT** — a `| head`-truncated grep had hidden
seven, which is the documented absence-grep footgun. ⭐ **and the facade's
optionality was a FICTION**: `game_assets` takes a `WorldManifest` — LDtk
vocabulary — in an **ungated public signature**, compiling only by reaching the
type through the monolith's re-export, whose own LDtk edge is unconditional. The
optionality was purchased by laundering, so declaring the dependency is what made
the facade deletable. ⛔ **no measurement moves** (`closure_size` 42,
`never_asked_for` 15) and that was predicted, not discovered — what the slice buys
is honesty and one fewer historical path.

✔ **two policy entries went with it**, having become checks that cannot fail:
`engine.portal-core-no-content-roster` forbade two `…actor_monolith::ldtk_world`
spellings that can no longer exist. Replaced by the one live name, which neither
portal crate names, so the rule stays green and now guards something.

⚠ **the measurement itself needed a correction to be trustworthy:** the monolith's
44-module graph had to be re-derived **with comments stripped**, because log
targets and doc citations otherwise score `ambition_platformer2d` — a crate
**above** the monolith — as a production edge from ten modules.

⭐ **THE MEASUREMENT, TAKEN 2026-08-14 — and it says the opposite of what size
says.** 117k lines across 33 top-level modules. Counting module-to-module
references (`crate::<mod>` from inside a sibling), the entanglement ranks:

| module | ← depended on by | → depends on | note |
|---|---|---|---|
| `features` | **19** | 18 | 41.7k lines, and the LAST thing to carve |
| `control` | 12 | 4 | |
| `avatar` / `character_runtime` / `boss_encounter` | 11 | 9 / 9 / 2 | |
| `schedule` | 10 | 6 | |
| `session` | 7 | 14 | highest OUTBOUND — a composition root, not a domain |

⛔ **so `features/` is the wrong place to start even though it is a third of the
crate** — it is the most entangled module in both directions, and carving by size
would begin exactly where change amplification is worst.

⛔⛔ **BUT THE "SIX LEAVES WITH ZERO INBOUND EDGES" HALF OF THIS MEASUREMENT WAS
WRONG, AND THE WAY IT WAS WRONG IS THE LESSON (verified 2026-08-14).** A
`crate::<mod>` grep is a LITERAL text measure, and it has two blind spots that
both produced false leaves:

- **`equipment` is not a leaf at all** — `action_scheme.rs` names
  `crate::equipment::reconcile_equipment_grants` three times, registering it and
  ordering two systems `.after` it. The grep simply missed it.
- **`dev` is not a leaf either** — its sibling consumers reach it as
  `crate::trace::`, through `pub use dev::trace;`. ⭐ **a RE-EXPORT ALIAS makes a
  module invisible to a name-based coupling census.**

⇒ **all six "leaves" are LIVE**, and four are live only from OUTSIDE the crate
(`gravity`, `affordances`, `menu`, `quest` — `menu` has zero sibling consumers and
is reached only by the app, runtime and shell host). ⚠ so the crate-placement
question about `menu` and `quest` living in an *actor* monolith is real, but it is
a PLACEMENT question, not a dead-code one. `character_roster` is **vestigial by
design**: a private `mod` with `pub(crate) fn catalog()`, consumed by five files
that are all tests, and its own doc says so. Leave it.

⇒ **PROMOTED OUT OF THIS ROW, because both are cross-crate and neither is a
carve:** the two write-only findings below are the highest-value D33 work
available, and both are DELETIONS rather than moves — which is the only kind of
decomposition progress this repository counts.

⭐⭐ **the genuine change-amplification finds were elsewhere, and they are large:**

1. ⛔ **`PlayerAffordances` is WRITE-ONLY.** `compute_player_affordances`
   recomputes it every sim tick and **nothing in the workspace reads it**. Its doc
   says *"the HUD reads it to label each on-screen button"* — but
   `ambition_touch_input` states in its own comment that labels *"now come from
   the CONTROLLED subject's action scheme via the `ControlPrompt` read-model, not
   the fixed smash-vocabulary affordance table"*. ⭐ **the consumer migrated and
   the producer stayed.** ~1200 lines are recoverable, plus three
   `declare_rollback_derived_resource` calls — so it owes a rollback schema
   re-baseline. ⚠ `interactable_proximity` and `InteractVariant` must SURVIVE for
   the portal adapter.
2. ⛔ **`GravityFlipSwitch` is spawned in exactly one place: its own unit test.**
   `gravity/plugin.rs` says `gravity_flip_switch_system` is *"intentionally NOT
   registered"* because nothing spawns the switch — yet the component is still
   rollback-registered by the runtime, queried by `ambition_sim_view::facts`, and
   given a visual by `ambition_render`. **Three crates carrying a mechanic that
   cannot occur.**

✔ **and four genuinely dead items were deleted** (each verified zero-consumer
first): `ambition_persistence::quest::registry::push_room_entered_quest_event_for_room`
— the abandoned half of a fork, whose first parameter is not a `SystemParam`, **so
it could never have been registered as a Bevy system**; the monolith `quest`
module's whole-namespace `pub use ambition_persistence::quest::*`;
`menu::map::ui::spawn_map_menu`, whose sibling's doc cited a process-resident
direct-entry host that does not exist; and `affordances::variants::IconId`, an
**uninhabited** `enum IconId {}` behind a default method that could only ever
return `None`.

⚠ **three caveats before anyone acts on this table.** (1) "No sibling depends on
it" is not "nothing depends on it" — these are `pub mod`s and the app or another
crate may consume them; check outward edges before moving anything. (2) A move to
a workspace crate must clear the ORPHAN RULE, which is what actually adjudicates
placement. (3) ⛔ **AGENTS.md says this crate is not awaiting a size-driven
carve** — the value of a carve is capability closure, compile isolation and public
API shape, so a leaf that nothing consumes may simply be DEAD, and measuring that
is cheaper than moving it.

- ◐ **D71/D92 — Finish the real room-transition transaction path.** ⭐ **THE
  CENSUS IS CLOSED (2026-08-14): the shipped rollback host now opens a readiness
  transaction on every room change — 21 changes / 21 transactions, was 24 / 0.**
  One semantic `RoomTransitionIntent`, all four origins on it,
  `RoomTransitionRequested` DELETED, readiness moved host-side, the confirmed
  commit gated on the same transaction. ⭐ **AND THE TWO PATHS ARE NOW ONE**
  (2026-08-14): `RoomTransitionApplication` is the only implementation of *"put
  this RECORDED subject in this PREPARED room"*, reached by the eager host as a
  `SystemParam` and by the confirmed host through a `SystemState` on `&mut
  World`. `load_room` (24 params), `apply_room_transition_resets` (⚠ GONE — folded into the one room-transition application at `a2b6652e7`, D71),
  `RoomConstructionPlan::apply_to_world` and `resolve_transition_subject` are
  DELETED; the eager system went from 16 `SystemParam`s to 2. The fork had
  already cost a live defect — the shipped rollback host never cleared room
  carryover, so a door carried enemy projectiles and a modified gravity into the
  next room — measured RED and now green. What remains under this row is the
  CANCELLATION asymmetry, the prefetch/latency MEASUREMENT, and predicted-intent
  readiness.

⭐⭐ **2026-08-14: THE SHIPPED HOST HAD NO INSTRUMENT, AND THAT IS ALSO JON'S
MAGENTA FLASH.** `RoomTransitionLoadPhase::Committed` has exactly one writer (the
EAGER commit) and one reader (the presentation adapter's retirement gate). The
confirmed route never set it — `retire_committed_room_transition` nulled `active`
in `PreUpdate`, so the adapter's next `Update` took its *"no active transition"*
teardown branch. **Three things live behind that gate**, and the shipped game got
none of them: the `UnclaimedFeatureViews` **settle wait** (the cover comes down
when the room has been DRAWN, not one frame after it was built), `minimum_visible`
(the anti-strobe floor), and `RoomTransitionTelemetry::record` — **the only site
computing `request_to_ready`, `asset_wait`, `commit_to_first_target_frame` and
`prefetch_hit`, with ZERO samples on the shipped host.**

⇒ **so item 2 could not be answered with numbers because the instrument was never
reachable** — and the right move was to make it reachable rather than to
manufacture a measurement. ⭐ **this is very likely Jon's open observation
*"changing rooms flashes magenta squares for a brief moment"*** (2026-07-30): its
2026-08-09 fix was protecting only the fixed-tick host. ⚠ **the player-visible
half is INFERRED, not seen** — the code path is established; a `capture_scene`
through the Hall door, before and after, is what would confirm it.

⭐ **AND THE MEASUREMENT, TAKEN 2026-08-14 NOW THAT THE PHASES ARE READABLE**
(`cargo test -p ambition_app --test app_it -- hall_transition_cover --nocapture`,
the Hall door — the worst case in the game):

```text
preflight = 1.676ms   manifest = 14.687ms   barrier = (0 settled, 164 total)   prefetch_hit = false
```

⇒ **the transition is ASSET-BOUND, not construction-bound, and by an order of
magnitude.** Construction preflight is 1.7ms; the asset manifest is 14.7ms; and at
commit **zero of 164 assets have settled**, so the entire remaining wait is
loading. ⛔ **so do not optimise construction** — the 1.7ms is not where the door
feels slow. ⚠ `prefetch_hit = false` for the Hall is EXPECTED and is not a defect
to fix: the budget of 4 neighbours is deliberate, and unbounded hub prefetch was
measured on 2026-07-30 at p99 1372ms frames and 1803 MB resident images. ⭐ **a
hub is not idle time, and the door's wait is COVERED by the load foreground** —
that sentence is the correction to an earlier confident analysis of mine that said
the opposite.

⇒ **cancellation, per route — and the asymmetry is real:**

- *rollback rewinds past the request* — **refused by construction, correctly.**
  `ConfirmedRoomTransitionIntent::get()` filters on confirmed frames and GGRS never
  rewinds one, so no transaction can open against a rewindable intent.
- *load fails* — handled asymmetrically but deliberately (headless retires; a
  windowed host keeps it resident to retry).
- *supersession / stale epoch / session change* — handled.
- ⛔ *the player presses Cancel* — **MISSING, and the affordance LIES.** Cancel is
  Retry wearing another name: it drops the transaction and leaves the INTENT
  pending, so `begin` reopens the identical crossing next frame — Escape during a
  Hall load *restarts* it, discarding a prepared plan and manifest. It cannot clear
  the intent from `Update` (rollback state); the deterministic channel for a player
  intent is the input stream the sim reads. ⚠ that carries a product decision, so
  it is written at the decision site rather than improvised.
- ▣ *void crossing (recorded body gone)* — **FIXED 2026-08-16.** The eager host
  now gives this the same terminal meaning as `CommitOutcome::Cancelled`: consume
  the exact pending intent and retire its transaction instead of reopening an
  impossible crossing forever. Death closes the adjacent fixed-tick race too:
  `open_death_interlude` retracts the dead body's crossing, the detector excludes
  `OutOfPlay` bodies so it cannot refill the slot later in the same tick, and the
  eager loader retires the now-orphaned transaction. Rollback hosts deliberately
  do not infer cancellation from speculative intent absence.

⚠ **one more finding, unactioned:** `RoomTransitionLoadState` and
`PendingLifecycleCommit` are plain `init_resource`, **not experience-scoped**. The
Quit route retires the transaction so the common path is covered, but a
quit-to-home that happens before the load foreground exists was not traced.
⭐ experience-scoped state is `app.experience_owns(..)` — and note the trap:
`releasing` a resource that is read as a plain `Res` PANICS; the reset verb is
`resetting`.

⭐ **the shape, stated once:** `begin` opens a transaction on a confirmed intent,
and **only a successful commit or a void subject can close the INTENT.** Every
other "abandon" closes the TRANSACTION, which the still-pending intent immediately
reopens. ⇒ the convergence is the outcome enum, not another cancel path.

Use [`engine/room-transition-loading.md`](engine/room-transition-loading.md).
Exercise a real movement-kernel → loading-zone → readiness/commit path, keep
rollback-host transitions on the same transaction, and close only currently
reproduced provenance/carry/P2P gaps. This is also prerequisite architecture for
D116's eventual different-room participants.

⭐ **the gap was re-measured and RE-DESCRIBED on 2026-08-14, and the description
was the part that was wrong.** The census still holds (fixed-tick 11/11/0,
rollback 24/**0**/24, shipped host `ConfirmedFrameBoundary present=true`), but
the row claimed the rollback host *"bypasses the canonical construction plan"* —
it does not. `commit_transition` prepares and applies the same
`RoomConstructionPlan` and reuses `validated_spawn`. What the shipped route never
runs is the READINESS transaction: asset-readiness authorization, the
presentation cover, the unpresented-failure state, and prefetch accounting. ⛔ an
agent acting on the old sentence would hunt a second constructor that is not
there and leave the real difference — the shipped game changes rooms with no
cover and no failure reporting — untouched.

⛔⛔ **AND THE PLAN'S OWN "DELETE `RoomTransitionRequested`" SENTENCE WAS FALSE
TOO; corrected 2026-08-14 with a production census.** Loading-zone detection is
ONE of FOUR writers — the others are checkpoint resume (`shrine.rs`), Mary-O's
level-completion flag, and the loading UI's Retry. Migrating detection and
deleting the type breaks all three. ⭐ the census also handed the slice its real
prize: the MESSAGE cannot name its subject, so the commit re-resolves
`ControlledSubject`-or-primary a frame later, while `LifecycleIntent::Transition`
already records `subject: SimId` at detection. The richer contract wins, all four
origins move onto it, and the dedup key becomes `(subject, target_room, arrival,
edge_exit)` — ⛔ NOT target-room-only, which would collapse two doors into one
room at different arrivals.

- ▣ **D121 — The browser ran a DIFFERENT application, and the source already
  said this would happen.** LANDED 2026-08-14; **HUMAN-CONFIRMED 2026-08-15.**

See [`engine/web-platform-parity.md`](engine/web-platform-parity.md) — **four
separate defects, not one.** (1) `run_web()` hand-spelled the composition and its
copy lacked `AmbitionShellHosted` / the shell host / an initial route /
`install_ambition_shell_visuals` ⇒ blank canvas; now ONE
`compose_ambition_visible_game` both hosts call, and the third hand-spelled
composition is deleted. (2) The browser registered NO `game://` source, so every
`game://worlds/*.ldtk` resolved through a source that did not exist. (3)
`--served` published ONE implementation crate's `assets/` — measured, the served
tree had **no `worlds/` directory at all**; it now consumes
`package_asset_guard.py compose`, the same seam Android and Steam Deck use, and
names no crate. (4) `index.html` claimed a keyboard capture the app never
requests and never cleared it. Pinned by a composition contract (+ its
uncomposed-App poison) and an `AssetId` platform-parity audit over the production
manifest: **967/967 entries name the same file on both platforms.** ⚠ `.meta`
404s were NOT the bug and were deliberately left alone — no `.meta` file exists
under either root and none is expected. ⚠ the embedded-assets `web` persona is
still unaudited for defect (3).

⭐ **JON RAN THE REAL SERVED BUILD, 2026-08-15.** Verbatim: the browser boots and
visibly runs Ambition; the shell and launcher are visible and functional; served
asset publication works in the browser; arrow keys navigate menus; a gamepad
navigates menus. This objective is CLOSED on a human browser, which is what it
said it needed. What the same session found is NOT this objective and must not be
folded back into it: gameplay movement was dead (D123, fixed and awaiting
retest), Hall of Characters appeared to stick at 99%, and the opening music
crackled and then audibly "caught up" while startup was heavy — all three moved
to D124.

- ▣ **D123 — Gameplay input was owned by a DEVELOPER INSTRUMENT, so the shipped
  browser could not play.** LANDED 2026-08-15; ⛔ awaiting Jon's browser retest.

⭐ **the symptom made no sense until you knew where the latch lived**: arrows and
a gamepad both navigated menus, and neither moved the character. Not a keycode
question — the same gamepad failed, and menus prove device input reached
leafwing. `ControlFrameLatch`, the primary device→tick bridge, was installed by
`dev::rollback_observatory` (behind `dev_tools`), and `HostInputBindingsPlugin`
skipped it under GGRS *because* the observatory owned it. Desktop-dev enables
`dev_tools`; the web persona does not. So the browser had a live GGRS session,
live leafwing actions and seat latches, with no primary latch — and
`capture_latched_local_input` takes it as `Option`, where absent means "nobody
feeds me" and it declines to publish. Seat zero told the simulation the player
was holding nothing, every tick, in silence. Menus were unaffected because menu
frames never enter the session.

⛔ **A DEVELOPER INSTRUMENT MAY NEVER BE LOAD-BEARING FOR GAMEPLAY.** The device
host owns the bridge now, in the same arm as the seat latches; the observatory's
copy is deleted, so there is nothing to double. Pinned in
`ambition_platformer2d_host`, which cannot depend on `ambition_app` and therefore
cannot borrow an observatory — a GGRS host assembled there is the shape the
browser ships — with a frame-stepped poison so the claim is not a tautology.
`web_persona_boot` measures the real persona and fails on a latch that is missing
OR unfed (an accumulator left behind reproduces the bug with the resource
present): `primary device latch = false` → `device_seen: true`.

⛔ **THE ACCEPTANCE IS A HUMAN IN A BROWSER, and nothing else closes it**: arrows
navigate menus AND move the body; a gamepad navigates menus AND moves the body;
`dev_tools` still absent from the web persona.

- ⏸ **D124 — What the browser exposed. BOUNDED AND RESTING at Jon's direction.**

Plan: [`engine/portable-preparation-and-load-explainability.md`](engine/portable-preparation-and-load-explainability.md).
⚠ **that link was missing until 2026-08-15**, so D124's own 484-line plan was
reachable from nothing — the row and the plan existed and did not know about each
other. ⭐ its frame is **portability, not "optimize wasm"**: Brotli, `wasm-opt`,
AudioWorklets and cache headers are measurements, not this campaign.

⭐ **the harvest was a CONTRACT, not an optimisation: asset loaded ≠ CPU resident
≠ GPU resident.** `texture_is_ready` routes on `AssetServer::get_load_state` —
`Some` means ask the server, `None` means the handle is main-world-owned so ask
`Assets<Image>` for presence — and three systems dropped `Assets<Image>`
entirely. The load barrier explains itself (`asset_stall_report`) and phase
timings are portable (`bevy::platform::time::Instant`, because `Time<Real>`
advances once per frame and so measures zero within one).

⛔ **DO NOT RESUME THIS AS A PERFORMANCE CAMPAIGN.** Jon, 2026-08-14: the browser
is an architecture TEST FIXTURE while the engine is decomposed; it does not decide
what gets built next. ⭐ **the test for any tempting task: would we want this
abstraction if the web target disappeared tomorrow?** Semantic asset readiness,
cross-platform phase telemetry, canonical asset publication, host-owned input and
an explainable load barrier pass it. Brotli, wasm audio scheduling, Hall
streaming, a generic residency scheduler and byte shaving do not.

⛔⛔ **and the one change this row must NOT take**: making sprite sheets
`RENDER_WORLD`-only. **Seven** main-world `Assets<Image>` readers exist and
**four use PRESENCE as their readiness signal**, so the flag would turn
"successfully uploaded" into "never loaded" forever and characters would vanish
the moment their textures arrived. ⚠ that was one commit away, caught by review,
and the mistake was a census that counted one reader.

⇒ blocked on Jon's browser retests: does Hall of Characters leave 99%, and does
the opening music still crackle. Case file archived at
[`../archive/planning-superseded/2026-08-14/d124-browser-exposed-preparation.md`](../archive/planning-superseded/2026-08-14/d124-browser-exposed-preparation.md).

- ✔ **D120 — A platform capability is enabled beside the DEPENDENCY that needs
  it, not at the app. CLOSED 2026-08-14; the rule survives, the row does not.**

⭐ **THE RULE:** when a new target-specific need appears, enable it in the crate
that DECLARES the dependency, and let the app forward a semantic capability. A
future consumer of an Ambition runtime crate should be able to ask for browser
support without knowing what that crate depends on.

✔ verified at HEAD: `ambition_platformer2d_runtime` — which owns `bevy_ggrs` —
declares `web_platform = ["bevy_ggrs/wasm-bindgen"]`, and the app's `web` and
`web_served_assets` personas forward into it. The wrong-half fix
(`getrandom_02` declared at the app) is deleted.

⚠ **`getrandom_03` / `getrandom_04` still sit at the app and that is CORRECT** —
their owners publish no forwarding feature, so the app IS their nearest owner.

Case file: [`../archive/planning-superseded/2026-08-14/d120-platform-capability-placement.md`](../archive/planning-superseded/2026-08-14/d120-platform-capability-placement.md).

- ⏸ **D119 — CLOSED 2026-08-14. The archived-work recovery is done: every item
  archived mid-flight on 2026-08-13 had already been closed by a DIFFERENT
  campaign deleting the thing it was waiting on.** ⭐ nobody re-read the item
  after the road it depended on disappeared — this ledger's oldest failure
  mode; its standing rule is *grep for the thing a row says is missing before
  working it.* Measurement record:
  [`../archive/planning-superseded/2026-08-14/d119-archive-recovery.md`](../archive/planning-superseded/2026-08-14/d119-archive-recovery.md).

  ⚠ **two things survive, both Jon's, neither blocking:**

  1. Three of the run's goal checks in `.goal/active.json` grepped files
     deleted in `5e382342d` — `grep` on a missing path exits 2, `!` inverts it,
     the check reports satisfied. ✔ the goal PREAMBLE was rewritten 2026-08-14
     to scope all of `docs/planning` and route by document ROLE rather than
     filename. ⛔ **the checks themselves were deliberately NOT edited by the
     agent they judge** — quietly rewriting your own success criteria is not a
     repair.
  2. ✔ **DECIDED 2026-08-14: `WornCharacter` STAYS. The `CharacterIdentity`
     rename is REJECTED.** `WornCharacter(CharacterId)` answers *which authored
     character template does this body currently instantiate?* — not *which
     unique runtime occurrence is this?* (`SimId` answers that; D125 makes the
     distinction rigorous). It must stay legal for two bodies to hold
     `WornCharacter(Fia)` at once, and for `RecharacterizeBody` to change the
     worn character while the runtime occurrence stays the same.
     ⚠ if the "worn" metaphor is ever disliked, **`CharacterForm` or
     `CharacterTemplateRef`** preserve the distinction; `CharacterIdentity` is
     specifically the name to avoid.

- ⏸ **D64 — Mary-O / LDtk authoring. RESTING as a successful ACCEPTANCE
  BASELINE, not a running campaign (2026-08-15).** A new level can be created
  through LDtk without adding ordinary Rust level registration: authored rooms
  need no Rust routing to exist, destinations and warp tubes are authored, one
  shared `ldtk_entity_contract.json` makes the Rust prover and the Python
  validator refuse exactly what the real converter refuses, and a ratchet
  guards the level roster. That is an Engine 1.0 milestone.
  ⛔ **do not keep adding Mary-O tooling because the lane existed.** The next
  LDtk improvement must come from actual content-authoring friction.
  Preserved rules: `.ldtk` is the authoritative spatial source · tools edit it
  additively and in place · destructive bootstrap regeneration must not return
  · Rust and Python validation must agree · game-specific semantics stay
  provider-owned rather than growing a central engine taxonomy.
  ⛔⛔ **a row was filed as unstarted when it had already landed**, written from
  a `▢` in Jon's observations file without grepping HEAD. **A marker in a
  maintainer's file is a REPORT, not a measurement.**
  ⛔ **the Mary-O presentation guards do not run in the ordinary suite**
  (`#![cfg(feature = "visible")]`: 36 tests bare, 44 with it). Run the
  `visible` suite before and after any Mary-O visual work.
  ⚠ **the hole is the whole workspace's, measured 2026-08-14**: 24 crates hide
  **629 tests** behind features with no automatic runner —
  `.github/workflows/test.yml` is `workflow_dispatch` only and the per-turn
  gate is one integration target, both deliberate. What runs is a MAINTAINER
  decision; ⛔ do not enlarge `gate_suite.py`, and ⛔ do not add a job to a
  workflow that does not run and call the hole closed.
  ⚠⚠ **THE HOLE IS LIVE AND IT COST AN HOUR ON 2026-08-28.**
  `cargo test -p ambition_touch_input` reports **4 passed**; the same crate with
  `--all-features` runs **45**, and three of them were red from a change made an
  hour earlier. ⇒ **a small test count for a big crate is the tell** — ask whether
  the number is plausible before reading a green.
  ⭐ **AND ONE EXISTING TOOL ALREADY BUILDS EVERY TEST TARGET**: `sweep_target.py
  --apply` marks its live set with `cargo test --no-run --workspace`. It found
  three broken test targets that day — this one, a demo naming a deleted module in
  its own `tests/`, and a grouped `use` of a name a re-export had stopped
  publishing. ⛔ that is a REPORT of an existing tool, not a new runner, and it is
  not `gate_suite.py`'s job. ⛔⛔ do not reach for `cargo test --workspace --no-run`
  by hand instead: it filled `/dev/vda1` to zero twice that day, which is the
  failure the sweep exists to undo.

---

## Waiting on an external fact or maintainer decision

These are real unresolved items but are deliberately **not** `▢` queue work.
⭐ **a `✔` row here is one Jon has since answered**, kept in place for one pass so
anyone who came looking for the question finds the ruling instead of a gap.

- **D23 — projectile collision feel:** authored hurt geometry versus coarse body
  box; see [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) §26.
- **D50 — dropped held-item lifetime:** room-scoped versus persistent-world
  semantics; see the decision inbox.
- **D53 — Android suspend/resume:** validate the residual behavior on a real
  device before opening another source fix.
- **D54 — reported visual/VFX issue:** needs the requested reproduction.
- **D70 — Mary-O restart observation:** current tested paths do not reproduce it;
  needs game/room/time context.
- **D42 / D47 — character sizing/rig art:** currently principally authored
  rig/body-inset and visual-review work unless a reproduced engine defect appears.
- ✔✔ **D114 — CLOSED 2026-08-17 BY MAINTAINER RULING. Hitlag freezes the BODY
  that is in it, on both roads, and the old per-body-zero-dt prohibition is
  SUPERSEDED.** `818218949` gave the actor road
  `let sim_dt = if combat.is_in_hitlag() { 0.0 } else { dt };`, so a hit between
  two actors freezes both — which it never did, and CPU-versus-CPU froze
  nobody. Jon, overruling the prohibition: *"hitlag is a combat/body semantic,
  not something that should depend on whether a body happens to occupy the
  primary local-control road."* ⛔⛔ **the three options this row used to offer
  are void — every one preserved a per-road distinction, and the distinction
  WAS the defect. If hitlag ever feels too sticky, tune its DURATION or SHAPE;
  restoring a controlled-body/actor asymmetry is forbidden.** Ruling in
  [`maintainer-decisions.md`](maintainer-decisions.md); record in
  [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) §6.

---

## Regressions repaired 2026-08-14 (diagnosed by Jon, fixed with falsifiers)

| | status |
|---|---|
| Shield+Attack held-item throw | ✔ **closed**, confirmed by Jon on retest |
| F1 geometry coherence | ◐ **fix + regression coverage landed (D116 M2a); AWAITING JON'S VISUAL RETEST** — every rigidly attached row now takes one `PresentedPose::delta()`, and a test pins that. ⛔ an automated test is not a picture: the FIRST fix was also green, and it had merely moved one attached family to the presentation clock and relocated the shudder |
| Smirking eye beam | ✔ **closed** — never a replay fault; a contact predicate measuring centre distance, so its severity scaled with body size |
| Smirking same-room replay | ◐ **behaviour restored, class still open** — persistence ordering and one constructor divergence are fixed; same-room replay is still a second constructor |

⭐ **the recurring SHAPES moved to `dev/benchmark-candidates/`**, which is where
AGENTS.md routes durable lessons; this ledger keeps status, not case files. Two
generalised:

- [`two-constructors-for-one-population`](../../dev/benchmark-candidates/two-constructors-for-one-population-2026-08-14.md)
  — a constructor is TOLD a fact, a reset RE-DERIVES it from a proxy, and *"leaving
  the room and re-entering fixes it"* is the tell handed to you for free.
- [`an-absent-component-reads-as-no-value`](../../dev/benchmark-candidates/an-absent-component-reads-as-no-value-2026-08-14.md)
  — a narrow population's `None` arms silently serve "not covered", so widening
  it fixes three consumers and makes a fourth live and wrong.

The remaining two are already invariants elsewhere: an action spends an input
edge where it COMMITS (portal adapters), and a bare set tuple is not a sequence
(`ContentDialogueFollowupSet → ContentRoomReplayResetSet → RoomReplayApplied`
needed `.chain()`; the comment above it already claimed the order).

⛔ open item, carried forward: same-room replay converges on canonical
reconstruction, sequenced after the instance lifetime/provenance model — see
[`engine/same-room-replay-is-a-second-constructor.md`](engine/same-room-replay-is-a-second-constructor.md).

- ▢ **D127 — Deterministic authored gameplay logic and orchestration. M0 COMPLETE; M1 MET FOR BOTH HALVES; M2's PREPARED-CALL half landed 2026-08-17; the `when … then` RULE FORM is deliberately absent for want of a customer.**

⭐⭐ **ACTIVE TRUTH, checked against `7e7552c4b` on 2026-08-17 — everything below
is evidence, and several of its older claims are no longer true:**

```text
M0  ✔ complete
M1  ✔ met for BOTH halves — conditions AND commands have a domain-owned
       provider contract (PublishCondition / PublishCommand on App, private
       catalogs, no central enum edited to add a provider)
M2  ◐ the PREPARED CALL landed: PreparedCondition / PreparedCommand, private
       fields, no public constructor, id+arity+kind validated at prepare time,
       authored text NOT retained, an authored reference prepared into SimId.
       ⛔ the generic `when … then` container was DESIGNED AND CUT on purpose —
       zero adopters. It needs a REAL CUSTOMER before it is built.
M5  ▢ diagnostics, untouched
```

⇒ ~~two follow-ups are named and are NOT this row's next action:~~ **BOTH DONE
2026-08-26.** ~~`gated_lock_walls`
still rebuilds its condition arguments every tick instead of holding a
`PreparedCondition`~~ ✔ **— and `ConditionCatalog::prepare`'s own doc names this
caller as its reason for existing** (*"a caller that spells its own question in
Rust (a lock wall asking `world.flag_set`)"*), so the road was built FOR it and
never adopted. `ConditionCatalog::ask` was already there too. The cache now holds
the wall AND its `PreparedCondition`, prepared when the room is cached.

⛔⛔ **AND THE ADOPTION INTRODUCED AN ORDER THE OLD CODE COULD NOT HAVE.**
Evaluating fresh every frame is immune to registration order; preparing ONCE is
not. A provider registering after the first room is cached would have left its
walls holding `None` forever — a gate that never opens because of startup
SEQUENCE rather than because of the world. ⇒ an unpreparable question leaves the
wall standing (the same safe direction an unanswerable one takes) and is
RETRIED, and both arms are guarded:
`a_wall_whose_question_cannot_be_prepared_yet_stands_and_is_retried` seats an
empty catalog, asserts the wall stands, then publishes the provider and sets the
flag and asserts it opens. Poisoned by dropping the retry — the second arm
reddens. ⚠ **caching a validated thing buys speed and costs freshness; name what
can arrive late before you cache.**

and ~~`ambition_conversation::dialog::authored_commands`
still owns a second text→`AuthoredArg` conversion that `prepared::prepare_args`
now generalises~~.

✔ **THE SECOND ONE IS DONE, 2026-08-26, and it was a DELETION rather than a
wrapper.** The dialog road's Name / Number / Truth arms were a byte-identical
copy of `authored_logic::prepared`'s — the Truth arm's comment included — which
made TWO AUTHORITIES ON WHAT `true` MEANS. That is precisely the accident that
comment warns about one layer up: a fifth spelling accepted by accident on one
road and not the other, with nothing to say which was right.
`prepare_authored_arg` is the one public conversion now.

⛔ **THE REFERENCE ARM STAYED BEHIND, deliberately, because it is not a
conversion.** It is a CAPABILITY statement about that surface — Yarn hands a
command its parameters as quoted text, and a quoted string is not an identity —
so authored dialogue refuses references in its own words instead of borrowing a
message written for a road that can mint them.

⛔⛔ **AND THE POISON EXPOSED A HOLE IN THE GATE, the second one found today.**
`cargo test -p ambition_conversation` runs 25 tests; `--features ui` runs 35.
The whole `dialog` module — the authored-command road AND its 262 lines of
tests — is behind a NON-DEFAULT feature, so the plain per-crate invocation
compiles none of it and the first poison came back green over a deliberately
broken Truth arm. Under `--features ui` it reddened
`a_command_published_by_a_foreign_domain_is_requestable_from_authored_yarn`
immediately. ⇒ **when a crate has features, a poison that passes is evidence
about the FEATURE SET, not about the code.**

⭐⭐ **M2 acceptance is met for ONE call, not for a `when … then` rule,
deliberately.** `authored_logic::prepared` turns one authored line
(`<id> <arg>…`) into a `PreparedCondition`/`PreparedCommand` with no public
constructor: validation cannot be skipped, the runtime parses nothing, the data
is immutable, and a reference is minted as a real `SimId`
(`SimId::encounter`/`SimId::placement`) rather than a spelling.

⭐ deletions that made this a slice rather than an addition: `KERNEL_FACES`'s
switch→signal pairing and the `SwitchActivated` loop (kernel switches now
author `on_activate: encounter.signal encounter:symmetry_attunement
gravity_down`, performed by the `encounter.signal` command provider); Yarn's
hand-written `YarnStateMirror` bridge (`flag(id)`, `inventory_has(item)`, both
mirror slices, the per-frame inventory refill, a duplicate item-spelling
normaliser, `legacy_dialog_alias`); and `yarn_vocabulary`'s
`cmd_set_flag`/`cmd_clear_flag` (replaced by
`<<command "world.set_flag" "<id>" true>>`). Authored content now asks with
`condition("domain.question", <arg>)` and tells with
`command("world.set_flag", …)` — a domain publishing either adds nothing to any
bridge or vocabulary table. ⚠ the spawn-side half of `KERNEL_FACES` survives
with cause: it builds the encounter's own `Objective::All`, a puzzle stating
its own win condition rather than a table saying which switch does what.

⛔⛔ **the belief that forced the Yarn mirror was FALSE**: three module headers
said Yarn functions cannot be Bevy systems and so cannot reach `&World`;
measured, `bevy_yarnspinner` runs the interpreter from an exclusive system and
threads `&mut World` down to `YarnFn::call_with_world`, and
`SystemId<In<P>, O>` implements `YarnFn`. ⚠ two honest limits stand: a Yarn
*function* call's arity must match exactly (commands, dispatched by name with a
parameter list, do not have this limit), and every authored argument arrives as
TEXT, parsed against the published descriptor's kind.

⭐⭐ **the rollback trick both catalogs share, which is why either could be
waived from rollback at all:** `publish` is PRIVATE on both catalogs, reachable
only through `PublishCondition`/`PublishCommand` on `App`, and a simulation
tick holds a `World`, never an `App` — so "immutable once the simulation
starts" is a property of the TYPE. The command half additionally answers
AUTHORITY (`run` is private; the only public road in is the
`RunAuthoredCommand` message, read by one system) and introduces no new kind of
write (the runner writes into the existing, already-rollback-cleared
`SetFlagRequested` channel). Wire format 358 → 359 stable names, schema v35 →
v36.

⛔ **the named deletion gate (`KERNEL_FACES`'s pairing half) was refused with
cause until its two prerequisites existed — both since PAID (2026-08-17), see
the M2 block above**: a second command (`encounter.signal`) and an authored
LDtk surface carrying a command WITH its arguments (M2's job). ⚠ the condition
half hit the same wall and narrowed rather than invented: `LockWall.gated_by`
names a flag, not a whole condition, because "an authored surface is much
harder to take back than to widen."

Plan: [`engine/authored-gameplay-logic-and-orchestration.md`](engine/authored-gameplay-logic-and-orchestration.md).
Maintainer-identified capability gap: **authoring is strong for nouns —
characters, items, rooms, encounters, sprites, music, platforms, portals,
capabilities — and weak for verbs and relationships over time**: *"when two
switches are active, power a lift"*, *"when an item is placed here, open a
gate"*, *"latch this once true"*, *"wait for a semantic event, then act"* all
currently fall through into bespoke Rust.

Doctrine: **Rust extends the engine's vocabulary; authored gameplay content
composes vocabulary that already exists.** The deterministic simulation still
determines what is true — authored rules invoke explicit semantic domain
operations and never mutate arbitrary ECS state.

⛔ this row does NOT demote D125 or the first capability-aware reachability
customer — D125 is what makes a condition like "item occurrence X is held by
body Y" answerable at all.

⛔ **NOT authorized:** a `UniversalRuleVM`, Lua/Rhai, arbitrary ECS reflection, a
universal scene graph, a central `EngineEffect` enum, or replacing any existing
encounter/cutscene/boss/moveset representation. The several partial
condition → effect systems already in tree are evidence and candidate
customers, not defects.

⇒ **M0 findings (14 systems inspected 2026-08-15), in order of consequence:**

1. ⛔ the shared substrate does not own a universal SEQUENCER, and existing
   domain sequencers (`EncounterScript`'s monotonic cursor,
   `tick_gate_portal_phase`'s reversible timer, `BossPatternState`'s subroutine
   stack) are not to be forcibly unified — the substrate is conditions +
   commands + prepared references + preparation + discovery. A reusable
   control-flow backend stays an optional later experiment, not a current
   priority.
2. ⭐ the gap was on the CONDITION side — no shared condition/predicate type
   existed anywhere; the effect side already had 5+ typed command buses (a
   monolithic `GameplayEffect` enum was built and already deleted — "no god
   enum" is a repeated experiment, not a taste).
3. ⛔ boss patterns are the TEMPLATE, not a customer — they already ship
   authored `.ron`, compile-time cross-ref resolution, a design validator, and
   a cursor that snapshots the resolved timeline. Copy them; do not migrate
   them.
4. ⛔ M4 (is a program counter rollback state?) is not deferrable — three
   shipped answers already exist (register cursor+program, register nothing
   and rebuild, or waive it); whichever the shared form picks changes ≥2
   shipped systems.
5. **moving-platform gating was REJECTED** as a customer — the plan's own
   headline example, and it has nothing to delete (pure addition).
6. no BT/behavior-tree crate is in the lockfile; not refuted, no longer near
   front.

⭐ doctrine correction: a central *authoritative* census every new domain must
edit is bad; a derived *read-only* discovery index that domains contribute
descriptors to is good and required. ⛔⛔ do not sacrifice discoverability in the
name of avoiding central authority. Recorded in
`simulation-authority-and-determinism.md` and
`inspection-diagnostics-and-workbench.md`.

- ✔✔ **the gate portal's rollback waiver was a REAL DESYNC, fixed 2026-08-15.**
  The input rewound and its integral did not: the switch is
  rollback-registered, the phase is that switch integrated over time and was
  not, so a rewind left the integrator permanently ahead — and the consumer
  refuses a room crossing, so it is authoritative, not cosmetic. Fixed by
  deleting `GatePortalConfig.phase` and registering a `GatePortalPhases`
  resource with a value projection (a presence-only probe would have passed
  while reproducing the defect). Schema 31. ⭐ generalisable shape: "a
  registered input with an unregistered integral."
- ✔ **the `Brain` cursor's `_` arm — MOSTLY DISSOLVED.** Six brain families
  looked like they failed to rewind; `rollback_component_cursor`
  clone-snapshots the whole component, so they rewind fine and what they lack
  is desync DETECTION.

- ✔✔ **D133 — CLOSED 2026-08-16, promoted to a prerequisite 2026-08-17, and
  CLOSED AGAIN 2026-08-19 for the case that reopened it. The durable save
  horizon: what the world remembers about occurrences now survives closing the
  program.**

  The on-disk form IS the checkpoint's own description, serialized:
  `AmbitionGameSaveData` gained three `#[serde(default)]` lists
  (`AuthoredOccurrences`, `CustodyBaseline`, `MintedItemBaseline`, save version
  3 → 4) field for field from the checkpoint slice. A load is a checkpoint
  RESUME — adopt the ledger and baselines, write one `ResetToCheckpoint`; the
  road a death already takes rebuilds the world. ⛔⛔ a fixture-found defect: a
  session builds its start room before any file is read, so
  `record_placed_ground_items` republished the stale position over the loaded
  row. Fixed by an INVARIANT — an occurrence comes to rest here only if its row
  says `InCustody` or already `Placed` here, because an object cannot change
  rooms without being carried. Schema 33 → 34 (rename only); the save file is
  not rollback state.

  ⛔⛔ **PROMOTED TO A PREREQUISITE 2026-08-17, then RE-MEASURED 2026-08-19
  against HEAD because the recorded cause was wrong**: a runtime mint not in a
  hand at save time (lying in a room, in flight) was undescribed and lost, for
  THREE independent reasons, each alone sufficient — `live_minted_descriptions`
  filtered out anything not in custody; restore enumerated `CustodyBaseline`
  rows, keyed by custodian, so a dropped item had none to enumerate; and the
  materializer passed `Vec2::ZERO` on the belief no position was needed (false
  — `OccurrenceWhereabouts::Placed { room, at }` already records it).

  ✔✔ **CLOSED 2026-08-19 — a dropped runtime mint comes back where it fell,
  falsifier green and poison-verified**:
  `death_restores_the_checkpoint::a_mint_banked_where_it_fell_comes_back_where_it_fell`
  mints, picks up, drops, banks, dies, and asserts the object is back at the
  position it fell. Fixed across all three causes — `live_minted_descriptions`
  no longer filters on custody; `OccurrenceContinuity` now carries the
  checkpoint's `MintedItemBaseline` beside the ledger, and the reinstatement's
  unsettled-debt arm builds a `GroundItem` request at the ledger's own
  position. The fix cost no wire format — `MintedItemDescription`'s shape did
  not change, only which mints get one. ⭐ the trick was to die WITHOUT
  LEAVING: the death road writes `ResetToCheckpoint` and the room is torn down
  and rebuilt around the body, forcing the same rebuild a round trip would.

  ▢ **what is left is a HARNESS GAP, not a blocker**: a death does not return
  the player to the checkpoint's room (so there is nowhere to look from
  without a hand traveling with the body), and `walk_to` cannot come back
  through a loading zone (the return crossing never fires in 90 frames). The
  falsifier exists and is `#[ignore]`d with both blockers written on it. ⚠ a
  mint out of the INVENTORY resolves through `held_spec_by_id` (the item
  catalog), not `ambition_characters::brain::held_item_by_id` (the brain
  registry) — the narrow lookup answered `None` and lost it a second time.

  ⚠ Jon's dropped-weapon ruling is the product requirement behind this: a
  unique weapon stays where it fell. Also still open: `Consumed` round-trips
  and still has no live producer.

  ✔ **the headless-persistence residue is CLOSED 2026-08-19** —
  `PersistenceSchedulePlugin` was installed only by
  `AmbitionGamePresentationPlugin` ("visible binary only"), so an RL episode or
  a headless test could reach a checkpoint and never write a file. The durable
  horizon is SIM state, so the sim composition now installs the writer, paired
  with `PersistenceRoot::isolated()` (not optional — the default root is the
  player's real platform save dir, and a windowless host already redirects
  audio the same way).

- ✔✔ **D132 — CLOSED 2026-08-19. The same item had two persistence authorities
  and they had never been asked to agree.** ⭐⭐ measured first, and the
  prediction was wrong about which history breaks: the save/load/mint/bank/die
  scenario ended with the player holding AND owning it, the count decremented
  at no beat — that was coincidence, not the defect.
  ⛔ the actual defect: `OwnedItems` was not checkpoint state at all. A pressed
  pickup `grant`ed a catalog row beside taking custody, so one acquisition left
  TWO records and only the object's rewound — acquire after the checkpoint,
  die, and the menu equips a phantom whose throw mints a SECOND real weapon
  (measured: one javelin thrown twice via `<<give_item>>` produced two
  objects, `slot:0/0` and `slot:0/1` — player-triggerable duplication).
  ⭐ closed by DELETION, both halves poison-falsified: the `grant` is gone (the
  object is the record) and `OwnedItems::count` PROJECTS the equipped slot.
  `OwnedItemsBaseline` joined the three baselines a commit already writes
  (protocol 39 → 40), and the mint now spends the row it came from — spending
  without the baseline would be annihilation (a death would retract the minted
  instance while the quantity is already gone).
  ⛔⛔ three recorded answers in `two_persistence_authorities_for_one_item` had
  to be rewritten because they were measurements of the defect — e.g. the save
  reading `1` and the hand holding `1` "agreeing" was the bug (one javelin
  described twice); it now reads `0` and `1`.

  ✔✔ **THE OWNERSHIP QUESTION IS RULED, 2026-08-17.** Jon, verbatim: *"eventually
  we are going to switch to a Morrowind style inventory, so the occurrence is
  the owner, but inventory likely isn't a count it's a set with a count. I
  suppose it will depend on it the item is unique or not."* Correcting his own
  wording: *"and when I say set with count I mean dict… ie each item if has a
  count. for most items it will be a count of 1. note this could also be a
  collection of structs. whatever datastruct makes sense. I'm a python guy not
  a rust guy."*

```text
world       an item is an OCCURRENCE with identity   (held, dropped, placed)
inventory   an ENTRY carrying a COUNT                and the count is usually 1
```

  ⭐⭐ the shape is uniform: an entry is `(item, count)`, most counts are 1,
  twenty arrows are ONE entry with count 20 — uniqueness decides whether two
  entries may MERGE, not how either is stored. ⛔⛔ "dict" is PYTHON VOCABULARY
  for the shape, not a mandate for `HashMap` — the Rust representation is the
  implementer's call.

  ⇒ the rule to design is the CROSSING: a pickup merges into an entry or adds
  one, a drop MINTS an occurrence, and a unique item's identity survives the
  round trip — the same "minted instance not in a hand" case D133 covers.
  ▢ **the one genuinely open sub-question**: what makes two entries MERGEABLE —
  an authored uniqueness flag on the definition, or emergent distinguishing
  state (enchanted, named, partly spent). ⛔ do not answer it by inference.
  ⛔ do not build the general set-with-count today — "eventually" and "likely"
  are his words: the direction is settled, the schedule and exact stacking
  rule are not.
  `a_granted_quantity_survives_the_death_that_retracts_the_instance_minted_from_it`
  is the poison against retracting the row at the reset instead.
  ⭐ the seam, measured on D125: 5 of 6 catalog classes are counts forever; the
  problem is the nine held weapons/abilities that are an instance and a count
  at once. The answer is NOT a row per object — it is deciding which authority
  owns those nine and making the other DERIVE.

- ✔ **D134 — CLOSED 2026-08-16. The workspace-policy suite was 12 violations
  red and nothing ran it; it is 34/34 green.** The twelve were four different
  things wearing one label: one HAZARD (a `HashMap` fixed anyway, so ordering
  is the TYPE's property), two REAL rule violations (`Option<&MotionModel>`
  plus a second the rule could not spell), two POLICY IMPRECISIONS (a
  pre-spawn seed and an off-sim scratch — neither is an entity, so there is no
  authority to route through), one contract that had OUTLIVED its subject, and
  seven that were one wrong fact stated twice.
  ⭐⭐ `runtime → ldtk` was never an upward dependency — `cargo tree` settles it
  in one line: the ldtk crate's transitive closure contains zero occurrences of
  the runtime or the monolith. Both rules changed, with the argument written
  into their own `rationale` fields. ⚠ `bevy_ecs_ldtk` stays denied in both —
  the runtime may compose the adapter, never the backend it exists to contain
  (poison-tested red).
  ⭐ the durable lesson, on its THIRD instance in this file: deleting a
  compatibility facade reddens a boundary policy, every time, and that policy
  is the second file nobody remembers to edit.
  ⚠ one blind spot left standing deliberately: the movement rule matches
  SPELLINGS, and `Option<&ae::MotionModel>` escapes it. Recorded in that
  policy's rationale rather than widened into a crate this slice did not
  analyse.

- ✔ **D137 — CLOSED 2026-08-17 (`f7e34225d`). The doc-link ratchet was RED and
  is GREEN — no crate has risen and `--check` exits 0 for the first time since
  the row opened.**
  ⛔⛔ **`--check` IS LOAD-BEARING, measured by poisoning the baseline**: the
  bare command prints "ROSE from 5" and **exits 0**; only `--check` exits 1. A
  step wired with the obvious invocation would have been a gate that CANNOT
  FAIL.
  ⛔⛔ **a false "it is missing" claim was published here.** The ratchet had
  been in CI all along (`format-and-clippy`, `ccf254ff2`); a duplicate step was
  shipped then removed. The tell was in the output: the grep printed the
  workflow file TWICE and both lines were attributed to the step just written.
  ⇒ check the file as it was BEFORE the edit (`git show <commit>~1:<path>`).
  What was genuinely missing and is now wired: `cargo test -p
  ambition_workspace_policy` in `engine-tests`.
  ⛔⛔ **a CARVE LAUNDERS DEBT off any ledger keyed by crate name, and the
  ledger congratulates you for it.** The conversation carve took the monolith
  122 → 109 and invited banking it — those thirteen were RE-HOMED into a crate
  the list did not name. Adding `ambition_conversation` put 11 still-broken
  links back on the books, moving the honest total 182 → **193**. ⇒ the rule
  written into `CRATES`: **the destination joins in the same commit.** ⛔ never
  run `--update` to clear someone else's rise.
  ⭐ per-turn gating is answered NO by measurement — 51 s warm, **338 s after
  touching one crate** — so the gate runs pre-push or in CI. ⚠ the cost grows
  with every carve.
  ⚠ **the residual debt is NOT repaired** (193 broken links): fixing what a
  session broke is not paying it down. ⛔ do not bulk-delete the brackets —
  that converts a detectable break into an undetectable stale sentence. The
  stake, in the ratchet's own words: *"a deletion that leaves its references
  behind turns a doc comment into a description of a world that no longer
  exists — which in this repository is where the reasoning lives."*

✔✔ **D166's FIRST THREE SLICES LANDED 2026-08-19, in the 2026-08-19 GPT
review's own order.**

```text
1  action LEGALITY vs UTILITY   `ActionLegality` filters the option list; a
   (the review's item 1)        press the body cannot consume is not offered.
                               capture_probe: 54 presses/33 wasted -> 9/0,
                               holds 1 -> 2.
2  what a HOLD is worth         `capture_value` prices a capture from the
                               OPPONENT — guard, percent, hitstun, airborne —
                               never from a constant. The reverted
                               throw-damage pricing is pinned by a test.
3  capture RELATION vs POLICY   pummels/hold-age/escape left `CapturedBy` for
   (the review's item 3)        `SmashHoldState`; protocol 38 -> 39.
4  George SELECTS, not owns     his facet moved to the character-authoring
   (the review's item 2)        submodule; the demo's pack points at it.
5  every fighter has a grab     12 of 14 had `capture: None`; now none do.
```

▢▢ **THE CARVE IS COSTED — measured 2026-08-19, the boundary is now a
CHECKABLE CLAIM.**
⛔⛔ **THE SIZE IS WRONG BY 3× — re-measured 2026-08-28.** `brain/fighter` is
**3,679 lines (1,734 of them non-test)**, not 10,644. The whole `brain/` tree is
12,443, so the recorded figure is close to the WHOLE MODULE rather than the thing
being carved out of it — a mis-scoped measurement that has made this slice look
three times its size for nine days. ⭐ and its OUTWARD reach is small and all
sideways or downward: `perception` (5 kinds), `actor::attack_gesture`,
`actor::ActorFaction`, plus its own submodules; the only globs are test-local
`use super::*`; and the two crates it names that sit ABOVE `ambition_characters`
(`ambition_platformer2d`, `ambition_content`) appear ONLY IN PROSE — one doc line
each, no dependency.
⇒ **the blocker was never the size.** It is the four edges below plus the encoded
tag and the monolith's eight call sites.

✔✔ **AND TWO OF THE FOUR CLOSED THE SAME DAY, WITHOUT THE SEAM.** The row says
edges 1 and 2 *"collapse to ONE"* because `attack_kit` travels with the fighter
brain — that assumed the KIT is fighter vocabulary. It is not, and
`AttackBinding`'s own doc says so: *"the ordinary gesture vocabulary, not a
fighter-only bypass: a verb plus a direction is exactly what a human's stick and
button produce"*. The field is FILLED by the actors-side snapshot builder from the
body's live `ActorMoveset`; the brain is merely told it.
⇒ `AttackCandidate`, `AttackBinding`, `AttackVerb` and `ActionLegality` live in
`brain/attack_kit.rs` now — a sibling of `snapshot.rs`, not a child of `fighter`.
That closes **edge 1** (the snapshot's field) and **edge 4** (the codec's reach
for `AttackVerb`) with no registration seam and no wire change: the encoder writes
the tag as a literal `0/1/2/3`, so it names the VALUES, not the type's path.
⇒ **what remains is ONE concept, not four**: the `StateMachineCfg::Fighter`
variant, which `state_machine/mod.rs` declares, `brain/mod.rs` maps to the string
`"fighter"`, the codec tags `3`, and the monolith matches at eight sites. ⭐ the
ratchet stayed green throughout, and its `brain/snapshot.rs` CARVE-OUT is gone —
an exclusion kept past the edge it excused is a hole, and that file is guarded now
(poison-verified both ways).

⭐⭐ **AND THE CARVE IS NOT WHAT ITS NAME SAYS — measured 2026-08-28. THE FIGHTER
BRAIN IS ALREADY TWO CRATES, and the thinking is already above the floor:**

```text
ambition_combat::brain::fighter        7,398   the THINKING — `tick_fighter`,
                                               dispatched from the monolith's
                                               `brain_tick`
ambition_characters::brain::fighter    3,596   the DATA — cfg, state, options,
                                               situation, profile
ambition_combat::brain::smash          4,529   (the same shape, one arm over)
```

⇒ **the slice is not "carve the fighter brain out of the floor" — it is "let the
DATA join its own behaviour"**, which is one crate move, not a new crate. And the
thing pinning the data down is precisely the enum variant: `StateMachineCfg::
Fighter { cfg, state }` is why `FighterCfg`/`FighterState` cannot leave, while
`tick_fighter` left long ago. ⚠ the seam is still the blocker; it is just a much
smaller and better-understood one than "10,644 lines want a home".

`brain/fighter` was recorded as 10,644 lines; the GENERIC brain named it
in five places (widened from an initial undercount of three — the contract's
`fighter::` grep under `brain/` couldn't see a rollback codec that hand-writes
`StateMachineCfg::Fighter` by field, or `brain/mod.rs`'s string mapping):

```text
1  brain/snapshot.rs          `attack_kit: Vec<fighter::options::AttackCandidate>`
                              — every brain pays for a field one of them reads
2  brain/state_machine/mod.rs `StateMachineCfg::Fighter { cfg, state }` — the
                              shared brain cannot compile without the fighter
                              one; needs a registration seam
3  brain/smash/emit.rs        ✔ CLOSED 2026-08-19 — `TILT_DEFLECTION` moved to
                              `crate::actor::attack_gesture`
4  brain/snapshot_impls.rs    `Brain`'s rollback cursor codec hand-writes the
                              per-variant tag and fields
5  brain/mod.rs                maps the variant to the string "fighter"
```

⛔ **RE-MEASURED 2026-08-28 AND FOUR OF THE FIVE POINTERS NEED CORRECTING** —
which is the point of re-grepping a costed row before working it:

```text
1  brain/snapshot.rs           ✔ STILL THERE, line 87 (plus a doc mention at 163)
2  brain/state_machine/mod.rs  ✔ STILL THERE, lines 79-80 — `Box<FighterCfg>` and
                               `Box<FighterState>`
3  brain/smash/emit.rs         ✔ CLOSED, as recorded
4  brain/snapshot_impls.rs     ⛔ THAT FILE DOES NOT EXIST. The codec is
                               `crates/ambition_characters/src/snapshot_impls.rs`
                               (crate root, not under `brain/`), tag byte `3`
5  brain/mod.rs                ✔ STILL THERE, line 164
+  actor/character_catalog/    ⛔ A SIXTH the list never had: `resolver.rs:173`
   resolver.rs                 constructs the variant
```

⚠ **and the carve's real blast radius is OUTSIDE this crate**: the monolith names
`StateMachineCfg::Fighter` at eight sites (`brain_tick.rs`, and seven in
`features/ecs/brain_builders.rs`), all of which a `Brain::Capability(BrainId)`
dispatch has to replace. ⭐ the ratchet
`the-generic-brain-does-not-grow-new-platform-fighter-edges` is GREEN, so the
boundary has not drifted — only the FILE PATHS in this table did.

⭐ `ambition_platformer2d_core` does NOT depend on the fighter brain (its one
`fighter::` hit is a doc reference) — the one edge that would have made the
carve impossible does not exist.

⭐⭐ **the seam has a proven shape already in this tree**: `SmashHoldState`
(2026-08-19) solved the same problem one domain over — platform-fighter state
that used to ride inside a generic thing now rides BESIDE it as the
capability's own component, registered through `RollbackRegistrar` by the
capability's own domain. Applied here: `Brain::StateMachine(StateMachineCfg::
Fighter { cfg, state })` (a closed enum) becomes `Brain::Capability(BrainId)`
plus `FighterCfg`/`FighterState` as the capability's own components. That
single change removes all five edges. ⚠ the cost is a dispatch that finds a
registered brain rather than matching an arm.

⭐⭐ edges 1 (`attack_kit`) and 2 collapse to ONE: `attack_kit` is read only by
`tick_fighter`, called only from the `StateMachineCfg::Fighter` arm, so it
travels with the fighter brain the moment that variant stops being a variant
of a shared enum. ⇒ **one blocker remains: the brain-registration seam.**

⇒ the ratchet is `the-generic-brain-does-not-grow-new-platform-fighter-edges`
(`scripts/check_absence_contracts.py`), pinning the (now four) remaining edges
and refusing a fifth. Order: `smash/emit.rs` (done), then `snapshot.rs`, then
the state-machine variant that needs the seam.

⛔⛔ **the review's prohibition is NOT yet satisfied and is deliberately still
open**: *"I would specifically avoid adding things like `Features {
grab_value: ... }` to the generic scorer. That just moves the smell into a
struct."* A `capture_value` feature WAS added to `brain/fighter`'s `Features`
and works — but `brain/fighter` is not yet a platform-fighter CRATE, so the
point stands and is what the carve has to relocate. The review withdraws its
earlier hold on carving: *"I no longer think 'do not carve yet' should be
treated as indefinitely binding… the carve has now been earned."*

⚠ one number in that review is superseded: it says the CPU spent 36.5% of a
match in grab range "without one free-body Grab press" — true on the tree it
read; as of `15a2c1b8e` the CPU throws grabs from free bodies and lands holds,
pummels and throws. Its architectural conclusion is unaffected.

- ▢ **D166 — THE CHARACTER-AUTHORING BOUNDARY IS CHOSEN BUT NOT YET LOAD-BEARING.
  (from the 2026-08-18 GPT review; the boundary itself is now WRITTEN DOWN)**

`tools/ambition_sprite2d_renderer` is the character-authoring submodule under a
stale name, and as of `7a28709` its README says so in one block at the top:
**it owns character-specific authored MATERIAL and VALUES; this repository owns
the schema, preparation and runtime meaning those values conform to.** The test
for where something belongs is *"is this a VALUE an author chose, or a RULE the
engine enforces?"* ⛔ no rename and no second submodule until the seam is real.

⭐⭐ **AND THE SEAM THE REVIEW ASKS FOR ALREADY EXISTS — measured, not assumed.**
`ambition_characters::prepared` (2,168 lines) is exactly the pipeline the review
draws, and its own header draws it the same way:

```text
CharacterDefinition          authored, decomposable, may reference
      │ prepare_character              validates + flattens
      ▼
PreparedCharacterOverrides   PARTIAL — `None` still means "ask the catalog"
      │ Plugin::finish                 folds the catalog in, ONCE, transactionally
      ▼
PreparedCharacterDefinition  COMPLETE, immutable, no inheritance left
```

⇒ **so the work is not to establish a seam; it is to find what BYPASSES one that
is already built.** Stating it the other way round would have produced a second
half-built pipeline beside a good one.

⭐ **the review's named anti-pattern is ALREADY ABSENT, checked from three
angles** — post-registration reach-in mutation of a character definition:

```text
get_mut::<CharacterDefinition> / .definition_mut / catalog get_mut   0 hits
ResMut<CharacterCatalog | PreparedCharacter* | CharacterDefinition>  0 hits
&mut CharacterDefinition | &mut PreparedCharacterDefinition          0 hits
```

⇒ **games already consume prepared data.** The immutability the review asks for
is enforced by the type, not by convention, so that half of the ask is done.

▢ **what is genuinely open, then:** a fighter's `SmashRepertoire` is authored as
a game-side Rust literal — `george_booul_moveset.rs:556`,
`ambition_demo_sanic/src/smash_moveset.rs:415`,
`pirate_admiral_moveset.rs:461` — rather than as an authored character-package
facet. That is the "scattered game-side Rust constants" the review means, and it
is the one that matters because it is where the next character's values will go.

⚠ **`SmashRepertoire` lives in generic `ambition_characters` and its vocabulary
is not generic** (`ForwardSmash`, `NeutralAir`, posture-sensitive Down-B).
⛔⛔ **do NOT move it for purity.** It is a good abstraction with provisional
ownership; the restitch point is the first real character-owned `smash.fighter`
facet, and moving it before that costs a migration and buys nothing. The
intended direction, recorded rather than built:

```text
Smash capability   defines SmashFighterFacet / SmashRepertoire semantics
        ↑
character package  authors George/Alice/… values
        ↓
Smash preparation  produces runtime MoveSpecs / fighter data
```

⇒ **the generic engine should not need to know Smash move-slot taxonomy** — and
until a facet seam exists, it does, which is the whole content of this row.

◐ **THE RESTITCH POINT HAS PARTLY ARRIVED — status 2026-08-26, recorded rather
than acted on.** `SmashFighterFacet` is real and George authors one on disk
(`tools/…/characters/george_booul/smash_fighter.ron`), compiled through the
demo's pack. What now rides the facet:

```text
capture kit   grab / pummel / throws        ON THE FACET (shipped)
fighter BODY  gravity, fall speed, gait…    ON THE FACET (2026-08-26), reaching
                                            the seat as `MatchParticipant::body`
sixteen moves the composed `SmashRepertoire` STILL Rust, and DELIBERATELY
```

⛔ **the moves are not the next slice, and the refusal is written into the facet
file itself**: *"do NOT flatten George's sixteen composed moves into RON —
representation and ownership are separate questions"*. The Rust table is BUILT by
composing `strike` / `impulse` / `on_hit` / `committed_tail` / `feel`, and that
composition is the design. ⇒ this row's remaining content is the OWNERSHIP
question for a SECOND character's values, not a representation change to the
first.

⭐⭐ **THE CUSTOMER ARRIVED, AND IT NAMED THE SEAM PRECISELY (2026-08-18, the
grab campaign).** Capture was built to Jon's plan, landed end to end, and then
put enough load on the transitional generic structures to show exactly where the
line is. Five concrete pressures, each measured rather than argued:

```text
BrainSnapshot.captured_for        a capture_* field on the GENERIC snapshot
SpecificAction::CaptureStruggle   a capture verb in the generic action enum
sample_capture_escape             a reader placed specially at BOTH blanking
                                  sites, because no single seam exposes
                                  participant input AND actor-brain output
capture_candidate                 Smash effect KEYS read inside the actor
                                  monolith's option-kit builder
CapturedBy.pummels_landed         platform-fighter state on a generic relation
```

⛔⛔ **and the sharpest evidence is a number the generic scorer CANNOT produce.**
The fighter option scorer prices a move by what it does on CONTACT. A grab does
nothing on contact — its worth is that the opponent is HELD, which depends on
the throw it sets up, the escape risk, the captive's percent and the stage. That
value was briefly modelled as *"the grab is worth its forward throw's damage"*,
and `capture_probe` measured what it bought: the CPU grabbed from **110px with a
42px reach**, nine attempts in sixty seconds, **none of them inside its own
range, zero holds**. The number was reverted to the honest zero.

⇒ so the missing piece is not a weight or a feature — it is that **"how valuable
is holding somebody" is platform-fighter policy living in a scorer shared by
every actor in every game the engine runs.** That is the customer this row was
waiting for, and it is what the capability should own:

```text
generic engine        semantic control transport · body facts · perception
                      · control-hold machinery · temporary relationship and
                      body-constraint primitives · damage / launch
platform fighter      fighter action vocabulary · SmashRepertoire · capture
                      eligibility · pummel / throw / escape rules · what a
                      HOLD is worth to a decision
character package     grab geometry and timing · pummel · throws · weight ·
                      hit/hurt geometry · presentation bindings
```

⚠ **the stopping rule the grab work was held to, worth keeping**: *if a fact
would make no sense in a radically different game that merely has actors and
temporary relationships, it does not belong in the generic actor/character
layer.* The five pressures above each fail it, and each was recorded rather than
deepened once that was clear.
⭐⭐ **AND THE FIRST FACET LANDED — 2026-08-18, George's capture kit as CONTENT.**
Not the sixteen-slot repertoire, and that restraint is the finding rather than a
shortfall: the capture kit is the only part of a fighter's authoring that came
out as pure VALUES — six numbers of geometry, four timings, three payloads, no
helper composition in the middle of them.

```text
ambition_characters::smash_fighter             the facet's SEMANTICS + the schema
ambition_demo_smash/assets/fighters/george.ron the character package's VALUES
CaptureKitAuthoring::into_repertoire           preparation → the SAME MoveSpecs
```

⭐ **the capability is NAMED before it is a crate.** A schema registration must
name an owner, so registering `smash_fighter` writes the ownership down where a
tool can print it — `SMASH_FIGHTER_CAPABILITY`, in the crate the types still live
in. ⛔ it is not a licence to start the carve, and the standing note on both
sibling modules was updated to say so in as many words rather than being left to
read as satisfied.

⭐ **a SECOND pack in the workspace, which is what proves the pipeline is a
CAPABILITY** rather than `ambition_content`'s private loader: the demo compiles
its own pack through `ambition_platformer2d::content`, with no dependency on the
game crate. ⭐ the E9 oracle caught the one leak this needed — parsing `pack.ron`
was `ron::from_str` at every pack owner and `ron` is not a facade re-export — and
it was closed at the FACADE (`ContentPackDraft::from_manifest_ron`), which also
turned Ambition's own manifest panic into a diagnostic.

⛔ **what stayed in Rust, and why it is not a TODO.** The ordinary slots are
authored by COMPOSING `strike` / `impulse` / `on_hit` / `committed_tail` / `feel`,
and George's file states a law about the shape of his whole table in a
`debug_assert` beside them — the one that refused a `0.14` grab for landing in
the gap that IS this character. That composition is the design; flattening it
into RON would trade authored reasoning for a wall of numbers.

⚠ **the ⛔ note this closes.** `smash_capture` used to end with *"if a capture
param ever becomes authorable as loose RON, wiring `ParamSchemaRegistry` is a
precondition of that change"*. They are authorable now and they are not LOOSE:
typed serde with `deny_unknown_fields`, read by the content compiler before a
pack may reach a runtime, so a misspelled `knockback_grouth` names the file and
the field at COMPILE time. The precondition was met by a stronger road than the
one it named.

▢ **still open on this row** — the platform-fighter POLICY half, which is the
part the grab campaign proved the generic scorer cannot answer: *what is holding
somebody WORTH.* The facet moved the VALUES; the decision that spends them is
still `ambition_characters::brain::fighter`'s, and the five pressures above are
still where they were recorded.

⭐⭐ **AND THE OPEN HALF WAS RE-MEASURED, 2026-08-18.** `capture_probe -- 60` with
no `--force`, George versus Alice, found the grab landing from a free body ZERO
times in 3600 ticks despite 1313 ticks (36.5% of the match) inside its own 42px
range: `REACH_TOLERANCE = 2.0`, so `reach_fit(44, 110) = 1 − 66/88 = 0.25` — the
grab survives the "cannot reach" filter at 2.5× its own reach while every shorter
move is filtered to zero, so **at long range the grab is the last option standing
and wins by default**, and the body is at long range precisely BECAUSE it just
threw a smash and is in its recovery. ⇒ **the grab is chosen exactly when it
cannot be pressed, and never when it could work** — at close range the jab and
the smashes beat it, because they carry damage and it carries none.

✔✔ **BOTH HALVES WERE THEN BUILT AND MEASURED, 2026-08-19 — the numbers above are
superseded, the causal story above them is not:**

```text
                        the run above   +policy   +airborne   +legality
grab presses                        7        85          54           9
  wasted while committed            6        34          33           0
grabs started                       1        22          14           9
holds established                   0         1           1           2
```

⇒ `capture_value` prices a hold from the OPPONENT — guard, percent, hitstun,
airborne — rather than from damage the move does not deal; `ActionLegality` stops
the brain offering a move the body cannot begin. ⛔⛔ **`REACH_TOLERANCE` was
neither widened nor narrowed.** The spacing corrected itself once the grab had a
reason to be thrown at the right moment instead of being the last option standing
at long range — which is this row's own thesis confirmed, not a tuning change.

⇒ so the missing term is not "grabs are worth their throw's damage" (tried and
reverted, above) and not a wider tolerance. It is that a grab's worth in this
genre is **that it beats a shield and leads to a throw** — which is
platform-fighter policy, exactly what this row says the capability should own,
now with a match to point at instead of an argument.

⭐⭐ **AND THE SAME MEASUREMENT FOUND A COMBAT INPUT BUFFER THAT IS DESIGNED,
ROLLBACK-REGISTERED, AND WIRED TO NOTHING.** Checking `trigger_moveset_moves` and
`ResolvedAttackGesture` alone found no latch and looked like absence — wrong,
because `AxisManeuverState`'s own field doc said the design outright:

```text
"Buffered MOVEMENT actions (jump/burst/blink press windows). Combat
 buffers (attack/pogo/projectile) stay on the shared BodyActionBuffer."
```

⇒ so the movement half is REAL (`buffer_jump`, `buffer_burst`, `buffer_blink`,
`coyote_timer`, all inside the rollback-registered `MotionModel`), and the combat
half is designed but unused:

⭐⭐ **RE-GREPPED 2026-08-26 AND THIS HALF IS STALE — THE BUFFER SHIPPED.** The
measurement below said `BodyActionBuffer` had zero field writers, zero `tick`
callers, and was paying rollback rent for nothing. At HEAD it is the live combat
leniency:

```text
buffer_combat_action_presses   ticks it on the OWNER'S clock, arms
                               attack/grab/pogo/special from the resolved
                               gesture, and re-proposes a held press each tick
trigger_moveset_moves          reads `action_buffer.grab/.special/.pogo`, and
                               ACCEPTING a press is what spends the slot
                               (`ProposedVerb::spend`)
MotionModel                    "both halves are live, and they decay on the
                               same clock" — its own doc
```

⇒ **a press in the last frames of recovery now starts the move on the frame
recovery ends**, for a person and a CPU alike. The rent is being paid for.

⛔ THE ORIGINAL MEASUREMENT, kept because the WARNING generalises: *"a
canonical-codec component with zero writers is paying rent — implement it or
retire it"*. That is the right question to ask of any registered type, and this
one was answered by implementing.

```text
BodyActionBuffer { attack, pogo, projectile }   on every actor          [2026-08-18]
production reads/writes of its FIELDS            0
BodyActionBuffer::tick callers                   0
```

▢▢ **THE FIX IS A DECISION, AND IT IS NOT MINE TO TAKE UNILATERALLY — three
candidates, costed, 2026-08-18. ⇒ PROMOTED 2026-08-28 to
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) as entry
35, which is where a question of this kind belongs; it had been sitting inside a
10,000-line ledger for ten days, which is the same as not asking. The costing
below stays here as the evidence.** All three were reached by asking where "a grab
cannot be thrown from 2.5× its reach" is expressible.

```text
1  per-move tolerance on MoveFrameData   a NEW generic field on the shared
                                         frame-data type, for one genre's
                                         verb ⇒ exactly the pressure the grab
                                         campaign was told not to add
2  ask the REAL question instead of the  REACH_TOLERANCE = 2.0 is a PROXY for
   proxy: tolerance = what the body can  "can I close the gap during startup".
   close during the move's own startup   Deriving it from `startup_s` + whether
                                         the move commands an approach is
                                         engine physics, not genre policy —
                                         and it fixes EVERY move in EVERY game,
                                         which is also its risk
3  the capability ranks its own verbs    what D166's table actually says. ⛔ it
                                         is the carve, and the carve was
                                         explicitly deferred out of product work
```

⚠ **(2) is the one that is principled, and its blast radius is the whole reason
to stop here**: it changes how every CPU in every game this engine runs spaces
itself. ⛔ `ladder_probe` is NOT the instrument for it — it measures self-KO time
against a passive opponent, which is stage awareness, not spacing. The honest
instrument is `capture_probe`'s move histogram plus the fighter option tests,
and a before/after over several seeds.

⇒ **what a reader should do next**: pick one. (1) is cheap and adds a generic
field for one genre. (2) is right and wide. (3) is right and is a carve. Nothing
here is blocked on more measurement — the measurement is done.

⚠ **CORRECTION 2026-08-26 — option (2) costs ONE THING THE TABLE DOES NOT NAME,
and it is not a magnitude, it is an INPUT THE SCORER DOES NOT HAVE.** Re-read at
HEAD before picking. The startup half is free: `AttackOption.frames.startup_s` is
already in hand at the scoring site and the neighbouring `frame_advantage`
feature reads it. The CLOSING-SPEED half is not.

```text
options.rs:454   coverage_fit(c.frames.coverage, foe_local, foe_extent)
generate_options sees   view.self_view: PerceivedActor
PerceivedActor carries  pos, vel, facing, half_extent, faction, phase, ...
                        and NO top ground speed — `grep 'run_speed' options.rs` 0
options.rs:312   "Nothing here knows whose body it is."   <- deliberate
```

⛔ **AND THE AVAILABLE VELOCITY IS THE WRONG ONE.** `me.vel` is what the body is
doing, not what it CAN do, so a standing fighter reads a closing speed of zero,
a tolerance of zero, and refuses every attack whose reach does not already span
the gap exactly. The question "how far can I close during my own startup" is
asking about a CAPABILITY, and the capability is in the body's tuning.

⇒ **so option (2) is really: thread the body's own top ground speed into
perception, then derive.** That is a new `PerceivedActor` field on a struct that
describes FOES as well as self — which is arguably right (an opponent's closing
speed prices the same spacing question from the other side) but is a wider change
than "derive it from `startup_s`" reads. ⛔ Nothing here changes the
recommendation; it changes the PRICE, and the price is the reason this is still
Jon's pick rather than an obvious cleanup.

✔✔ **AND ONE OF THE REVIEW'S ASKS WAS ALREADY DONE — measured before touching
it.** *"Move authoring has historically duplicated `Vfx(...)` and matching
`Sfx(...)` events … converge on one semantic authored effect request with default
companion sound, while preserving explicit override and explicit silence."*

```text
MoveEvent::Vfx / MoveEvent::Sfx spelled in game/          0 files
                                (the one hit is prepared.rs DERIVING the cue
                                 inventory, which is the seam working)
MoveEventKind::Vfx / ::Sfx authored                       15 / 11
```

⇒ **D149 already made a `Vfx` event carry its companion sound**, so the surviving
`Sfx` events are the explicit standalone/override half the review asks to
preserve, not leftover pairing boilerplate. ⭐ and the doubling risk the
convergence CREATED is guarded by `ambition_content::moveset_sound` — an oracle
built from the two real systems (`dispatch_move_events` + `process_fx_requests`)
rather than a data test, whose one claim is that an authored burst is heard
EXACTLY ONCE. ⛔ nothing to converge here; re-doing it would re-introduce the
doubled jab that guard exists to catch.

- ◐ **D136 — COMPOSITION BOUNDARIES ARE ASSUMED, NOT STATED — so whoever
  installs a thing first decides who pays for it. (PROMOTED from `tracks.md`
  2026-08-16, with five instances measured in one night as its evidence.
  ⭐ ITS EXIT WAS WRITTEN AND MET 2026-08-28 — every active carve destination
  states its refusal, 3 of 18 → 9 of 18 — so what remains is the standing habit,
  not a task: a NEW destination states one when it becomes active.)**

⭐⭐ **AND THE FIRST POSITIVE INSTANCE — 2026-08-17, boundaries that were STATED
did the work, which is this row's thesis run forwards instead of backwards.**

Relocating four modules out of the monolith (`355874fe1`), three "obvious"
destinations REFUSED the work in their own words, before any code moved:

```text
ambition_dialog          declares itself CONTENT-FREE
ambition_settings_menu   renderer-agnostic, carries no bevy
ambition_menu            its manifest says the trimmed bevy features are
                         "load-bearing for the WHOLE workspace"
```

⭐⭐ **AND A STATED CONTRACT DID THE WORK FOUR TIMES IN ONE DAY — 2026-08-26,
which is this row's thesis at its cheapest.** `shared_tangle`'s header carries an
ADMISSION TEST: *"A type belongs here only when at least two distinct domains
consume it and neither can own it without creating the wrong dependency edge.
Being generic, potentially reusable, or awkward to place is not sufficient."* Four
types were admitted against it today without anybody arguing:

```text
Mass               character runtime writes it · the mount pair reads it
MountDied          mount announces · boss bridge AND the brain rebuild read
TemporaryControl   possession · the mount pair
SpawnBaseline      respawn · the mount dismount
```

⚠ **AND THE FOURTH TESTED THE WORDING, which is worth recording.** `SpawnBaseline`'s
two consumers are BOTH inside the actor monolith today — `reset_to_spawn` and the
mount dismount. It qualifies because the test says **DOMAINS**, not crates:
neither can own it without the other depending on it, and that is true whether or
not they currently share a `Cargo.toml`. ⇒ **if somebody reads "domain" as
"crate", this admission looks wrong** — so the distinction is now written down
next to an instance rather than left to the reader.

⭐ **AND ONE MORE DESTINATION NOW STATES ITS CONTRACT — 2026-08-21.**
`ambition_combat` is an ACTIVE carve destination (it received `feel.rs` the same
day) and had six header lines describing what it holds and not one sentence about
what it refuses. A destination that says nothing accepts everything. Its refusals
are now written where the next carve reads them, and both are EVIDENCE rather
than aspiration:

```text
refuses presentation     enforced, not promised: `bevy` is taken with
                         `default-features = false`, so nothing that must DRAW
                         can compile here. It NAMES cues (`ambition_sfx` /
                         `ambition_vfx` supply the vocabulary) and something
                         downstream decides how a named cue looks
refuses body lifecycle   spawn/despawn/residency/possession are the actor
                         layer's; this crate reads bodies and writes what
                         happened TO them
```

⚠ the header also records why `feel.rs` was a CORRECT arrival under that
contract — hitlag and hitstop are rules a fight obeys, not decoration on one — so
the next borderline case has a worked example rather than only a rule.

⭐⭐ **AND THE ROW FINALLY HAS AN EXIT, because its thesis is COUNTABLE — measured
2026-08-28.** Of the eighteen crates that are carve destinations, **three state
what they REFUSE** (`ambition_combat`, `ambition_platformer2d_shared_tangle`,
`ambition_binding`) and fifteen say only what they hold. That is this row in one
number, and it is why the row could never close: it was a principle with no
count.

▢ **EXIT: every ACTIVE carve destination states its refusal.** Not all eighteen —
a crate nobody is carving into does not need the sentence, and inventing refusals
for it would be fabrication dressed as documentation.

✔ **Three more written 2026-08-28, and each cites the event that proves it** —
the row's own method, since a refusal nobody has tested is an aspiration:

```text
ambition_characters       refuses the actor INTEGRATION layer (already a
                          checked absence contract) and anything the SHEET
                          derives — `ActorSpriteMetrics` was refused here first
ambition_boss_encounter   refuses GENERIC body geometry. A boss was the first
                          customer of AABB overlap and `CombatGeometry`, not
                          their owner, and holding them made a whole seam read
                          as boss vocabulary. 531 lines left this month.
                          ⇒ the test: would the SECOND consumer be a boss?
ambition_sprite_sheet     refuses anything `ambition_characters` must read (it
                          depends on that crate, so hosting one inverts the
                          edge) and refuses to say how tall a body STANDS —
                          the catalog decides that; the sheet supplies the
                          pixels it is measured against
```

✔✔ **AND THE LAST THREE FOLLOWED THE SAME DAY, so every ACTIVE carve destination
now states its refusal and this row's exit is MET:**

```text
ambition_mount          refuses body LIFECYCLE (a pair is a relation between
                        bodies somebody else made — the same rule its own
                        "the monolith still builds pairs" note reads from the
                        other side) and per-species mount rules
                        ⇒ the test: would it read as nonsense for a DIFFERENT
                        pair of bodies?
ambition_damage         refuses the ATTACK side, refuses to decide whether two
                        bodies are ENEMIES, and refuses to draw — the last one
                        enforced by `ambition_render` being absent from its
                        manifest. ⚠ and NAMES its own borderline edge,
                        `ambition_persistence` for the assist-mode scale, so the
                        next arrival is weighed against a boundary rather than
                        against one precedent.
ambition_conversation   refuses presentation (projections rebuilt FROM
                        `ActiveConversation` belong to `ambition_dialog`),
                        refuses named content — banter arrived as the RULE for
                        talking over each other, never the words — and refuses
                        to derive who is speaking from a player slot
```

⇒ **9 of 18 destinations state a refusal, and the nine that do not are the ones
nobody is carving into.** ⛔ do not "finish" this row by writing the other nine:
a refusal invented for an inactive crate is fabrication dressed as documentation,
and the four times this row's own evidence shows a stated boundary DOING the work
were all boundaries somebody had tested.

⛔⛔ **AND THE SHARPER LESSON, from the crate literally named `shared_tangle`: a
DEPENDENCY refusal is not an ADMISSION RULE.** Its header already said it depends
on no monolith, content, presentation, app assembly or devtools — and turned away
nothing that respects those edges, so anything awkward to place qualified. That
is how a tangle becomes one. Measured: 13 files gained in 14 days, second only to
the monolith it is carved from.

⭐ **the admission rule was already written — on ONE TYPE, where nobody placing a
second one would look.** `MountDied` says it: *"it lives HERE, below the domains,
because two of them share it… a message owned by one of the two would make the
other depend on it for a type carrying nothing but a pair of entities."* Promoted
to the crate header as the test it always was: **two real consumers in different
domains, today** — not "generic", not "might be shared later", not "awkward where
it was". ⛔ with the corollary that matters to an active carve: moving something
here without a second domain that reads it LAUNDERS the debt — the carve looks
finished while the concept is now split across two crates instead of one.

⛔⛔ **AND THE THIRD INSTANCE IS THE STRONGEST: A BOUNDARY CAN BE STATED, READ,
AND OBEYED — ON THE WRONG AXIS.** `ambition_characters` already refuses named
world content (*"the actual cast of bosses/enemies stays in `ambition_content`"*).
Measured 2026-08-21: the **15,928 lines** of platform-fighter policy D168 wants
carved out contain **ZERO** references to a `CharacterId` or the character
catalog. Every line passed the stated rule. Nobody broke anything.

⇒ **the axis was content-vs-vocabulary; the axis that matters for a FLOOR crate
is *would a game that is not a platform fighter still want this?*** Content-free
and genre-specific are not the same thing, and only one of them is a reason to
live in a crate every composition links — including a movement-only game with no
fighters in it. That question is now the crate's stated admission test, with a
⛔ against reading the size of `brain/fighter` as permission to add the next one
beside it.

⇒ **the pattern for the rest of this row: the rule usually EXISTS, buried on the
first type that needed it — or stated plainly on an axis that cannot see the
problem.** Promoting or re-aiming it costs nothing and is not doc-writing; it is
moving a decision to where the next person looks. Three destinations done on
evidence (`ambition_combat`, `shared_tangle`, `ambition_characters`), each a
different failure: no refusal at all, a dependency refusal mistaken for an
admission rule, and a correct refusal on the wrong axis.

⇒ **every one of those is a composition boundary written down where the next
person looks**, and each turned a plausible move into an obviously wrong one at
zero cost. ⭐ the rule this yields is small and practical: **read the
DESTINATION's stated contract before moving anything into it** — the failure
this row catalogues is discovery-by-collision, and a stated contract is how a
boundary gets discovered by READING instead.

⚠ the counter-case is in the same commit: `items` and `world` could not move
because `construction` imports `world::placements` BACK — a bidirectional edge
nobody declared, found only by chasing it.

⛔⛔ **A SEVENTH INSTANCE, 2026-08-28, AND IT IS THE ROW'S COMPLAINT RATHER THAN
ITS THESIS: FOUR CARVES IN ONE DAY AND NOT ONE DESTINATION CONTRACT WAS
CONSULTED, BECAUSE THERE WAS NOTHING TO CONSULT.** What decided each home was a
FACT found by grepping, and what refused two of them was the dependency graph
found by compiling:

```text
boss ECS      → ambition_boss_encounter   it already owned the profiles, the
                                          catalog and the anim helpers the module
                                          was calling through a facade
ledge_trump   → ambition_combat           `DeclaredCombatRules::ledge_trump_pop`
                                          was already there
attack        → ambition_combat           the melee LIFECYCLE had already left for
                                          `combat::moveset`; its own module doc
                                          said "there is ONE melee path"
banter        → ambition_conversation     a bark is the shortest conversation
                                          there is — reasoned, not read

REFUSED, by the graph and only by the graph:
damage_predicates ⇸ combat   needs boss_encounter, which depends on combat
brain_effects     ⇸ anywhere names ambition_app, mount and projectiles
```

⇒ **the reasons were good and none of them was WRITTEN DOWN ANYWHERE A READER
COULD FIND FIRST.** Every one had to be reconstructed by grep, and the two
refusals surfaced as compile errors after `git mv`. That is discovery-by-collision
with a friendlier compiler, which is exactly what this row exists to end.
⭐ the cheapest fix is the one the row already found: a destination crate's header
saying what it is FOR. `ambition_combat` says *"reusable, content-free combat
MODEL"* in its manifest description and that sentence answered three of the four
placements above — it was just never read as a contract because it lives in
`description`, where nobody looks.

⭐⭐ **A SIXTH INSTANCE, 2026-08-19 — and it is the row's failure mode in its
purest form: a boundary drawn from a real hazard by whoever installed the thing
first.** `PersistenceSchedulePlugin`'s own doc said *"for visible builds.
Headless / RL drivers omit this plugin so they never read or write user files."*

```text
the hazard, real     writing the PLAYER's files
the line drawn       persist only in visible builds
what that cost       an RL episode, a fuzz run or a headless test could reach a
                     checkpoint and never write one — the durable horizon is SIM
                     state, so this was a capability that existed only when
                     somebody was watching
```

⇒ **the two were conflated, and separating them is the whole fix**: any
composition that SIMULATES installs it, and a non-player App owes its own
`PersistenceRoot` — `isolated()`, the same redirection a windowless host already
makes for audio. Both halves are now written on the plugin itself, where the
next person looks, and asserted in one test because installing one without the
other is worse than neither.
⚠ **the shape to recognise**: the sentence naming the hazard was CORRECT and the
sentence drawing the line was not, in the same doc comment. A stated boundary is
only as good as the question it answers — *"who may write the player's files"*
is answerable; *"which builds persist"* was a proxy for it.

⭐⭐ **THREE MORE INSTANCES 2026-08-17, and one of them is the row's thesis
RESOLVED for a single capability — which is what a worked example looks like.**

```text
D152  empowerment EXPIRY was every game's to install, and five games each
      remembered. A sixth that forgot got PERMANENT invulnerability.
      ⇒ resolved: the ENGINE installs the lifecycle in a named set; the ORDER
        stays each game's. "What is engine-owned is the INVARIANT, not the
        order" — which is exactly this row's distinction, stated by the code.
      ⚠ and the honest residue: the five sat in THREE MUTUALLY EXCLUSIVE
        phases, so one shared set has one position and per-game re-placement
        would be a schedule CYCLE. Not every boundary can be stated without
        moving something.

D149  `process_fx_requests` is installed by the HOST, not by the crate that
      writes the channel. So a headless fixture in `ambition_combat` that
      asserted on the visual went BLIND the moment the producer moved onto the
      paired request — the crate could no longer test its own effect.
      ⇒ a capability whose CONSUMER lives above its PRODUCER cannot be
        verified where it is written.

D33   the conversation carve (in flight) is this row in its Cargo form: a
      module with zero outward imports that was nonetheless pinned — first by
      the SCHEDULE (fixed in step 1.5), now by nothing.
```

⭐⭐ **AND THE MONOLITH'S OWN `ldtk_runtime` FEATURE WAS A FICTION — measured
2026-08-18, which is the sharpest instance this row has.**

`bevy_ecs_ldtk` and `bevy_asset_loader` are declared OPTIONAL in
`ambition_platformer2d_actor_monolith`'s manifest and gated behind
`ldtk_runtime`. Exactly one module named both UNCONDITIONALLY, so:

```text
cargo check -p ambition_platformer2d_actor_monolith --no-default-features
  → 4 errors, ALL FOUR in src/assets/loading.rs
```

⇒ **turning the feature off did not yield a smaller crate; it yielded a crate
that would not compile.** The manifest stated a boundary and the code did not
honour it — this row's title with the two halves in one crate instead of two.

⭐ **and the module was reachable only because a dead parameter kept it alive.**
`SimulationSetup` carried `sandbox_data_asset`, `sandbox_asset_collection` and
`asset_server` purely to clone two handles into `_`-prefixed locals that dropped
on the next line. That keeps NOTHING loaded — the resources holding those handles
are what keep the assets alive, and they outlive the call by construction. Five
of the seven call sites already passed `None, None`. Deleting the three params
took the provider's only `AssetServer` dependency and its only mention of the
LDtk asset type with them, and `#[cfg(feature = "ldtk_runtime")] pub mod loading`
then compiled clean.

⚠ **the footprint ratchet did NOT move** (44 linked / 17 unwanted, unchanged) and
saying otherwise would be the easy overclaim here: no Cargo edge changed, because
the optional dep was already declared optional. What changed is that the
declaration is now TRUE. The ratchet measures the sentinel's closure; it cannot
see a feature that is unusable, which is why this instance needed a build to find
rather than a manifest read.

⭐ guarded by a `run_tests.py` job that runs that exact build — the CONDITION, not
a grep proxy for it. ⚠ it sits in the exhaustive plan with the other
feature-variant jobs (a distinct feature set is a distinct dependency graph), so
it catches this on Jon's periodic sweep, not on every backbone run.

⇒ **the pattern across all three is that the boundary is discovered by whoever
trips over it**, which is the row's title restated. ⭐ **D152 is the template**:
name the invariant, install it below, leave the ORDER to the composition — and
say out loud which part could not be preserved.

Plan: [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).
Card text: *"Make optional capabilities honest in Cargo dependency closure and
runtime/plugin assembly; a minimal consumer should not silently inherit unrelated
domains."* ⚠ **it was reachable from `tracks.md` and from NO ledger row** — the
same strandedness that made seven designed Engine 1.0 plans invisible on
2026-08-14. Promoted rather than re-derived.

⭐⭐ **it is promoted now because five independent slices on 2026-08-16 all turned
out to be the same failure**, which is a much stronger argument than the card
could make on its own:

```text
D128  the engine cannot ship the art IT draws — every sprite-registration site
      is a GAME system, so `spawn_explosion` reaching for `generic_explosions`
      works only if some game happened to declare it
D132  the durable-save leg is installed by the visible-binary-only presentation
      assembly, so ONE OF TWO persistence authorities does not exist in any
      headless harness — which is why they had never met in a test
D131  a crossover match reads each seat's percent against the pool that seat's
      HOME GAME authored — a DATA value crossing the boundary, not a rule
      (CLOSED — see D131, below)
D134  `runtime → ldtk` was forbidden by two policies nothing ran; the EDGE turned
      out legitimate and downward — but only because a facade deletion converted
      a laundered edge into a declared one, for the THIRD time in that file
D135  the canonical session world carries an authoring-format-specific field, and
      five RON-only games construct `::default()` for a world they never install
      (CLOSED — see D135, below)
```

⇒ **the through-line: none of these is a bug in the ordinary sense.** Each is a
place where *"who is this for?"* was answered by whoever installed it first, and
never written down. ⭐ that is why the composition-shaped slices have been paying
more than the feature-shaped ones this week — naming the schedule order let
`conversation` leave, giving the engine a home for its own art made effects
reachable, and making a load a checkpoint resume removed a whole reconstruction
road.

⚠ **D131 sharpened the umbrella and cost it a member.** Its composition failure
was real and was **not a rule reaching a foreign body** — it was an authored
NUMBER (`max_health`) read by another game's rules, so nothing a system-scoping
mechanism could have caught. ⇒ **the umbrella has two shapes, not one**: (a) a
value authored under game A's rules read as universal by game B, and (b) a
global SINGLETON whose owner is whoever installed it last. D131 also MEASURED an
instance of (b) and left it standing on purpose (later fixed — see "death
rules", below). ⛔ two shapes measured is the argument for a scoping mechanism;
one was not, and D131 deliberately did not build one.

⚠ **the standing number to move**: `capability-footprint-may-not-grow` reads
**42 crates linked, 15 a movement-only game never asked for**. ⇒ **a slice that
claims this row should say what it did to that number, or say why the number is
dominated by something it did not touch.**

⚠ **D135 (CLOSED — see below) was the first executable instance, and it answered
the standing number above with a NO**: the footprint did not move, because
`ambition_platformer2d_ldtk` was held in a movement-only game's closure by the
MONOLITH — seven production files needing nine symbols (`WorldManifest`,
`LdtkProject`, `ActiveLdtkProject`, `LdtkHotReloadState`, `poll_ldtk_file_changes`,
the `field_*` readers) — and not by the session world at all. ⇒ the next instance
to take was a monolith carve at the world-manifest/asset-catalog seam.

⇒ **THE WORLD-MANIFEST INSTANCE LANDED 2026-08-16, AND THE COUNTER STILL READ
42/15 — because the premise above named ONE edge and `cargo tree -i` found
FOUR.** ⛔ *"the monolith holds `ambition_platformer2d_ldtk`"* was true and
incomplete: `ambition_platformer2d` itself declared the backend
**unconditionally**, `ambition_platformer2d_runtime` declares it, and
`ambition_platformer2d_provider` declared it while naming **zero** of its
symbols. Cutting the monolith's edge alone was never going to move this number.

What the slice did:
- `WorldManifest`/`WorldSource`/`world_bevy_asset_path` moved OUT of
  `ambition_platformer2d_ldtk` into `ambition_platformer2d_world::world_manifest`,
  **no re-export left behind**. The type named nothing from the LDtk crate: an
  `AssetId`, four paths/strings, a bool, and a `ron_rooms` field that already
  pointed at the world crate — its sibling `ron_room::RonRoomSource` lived there
  the whole time.
- the provider's **dead** LDtk edge was deleted (measured: no `.rs` file in that
  crate named a symbol from it).
- the facade's LDtk edge is `optional` again and `ldtk_map` is gated on it —
  which the manifest's own ⛔ note had made conditional on exactly this move.
- ⚠ `ambition_platformer2d_world` gained `ambition_asset_manager`
  (`engine.world-ir-dependency-allowlist` amended, not waived). Free by
  measurement: that crate was already in the sentinel's closure, and it is a leaf
  with zero `ambition_*` dependencies, taken without its `bevy` feature.

⭐⭐⭐ **A PLUGIN THAT IS ADDED AND THEN DECLINES TO RUN IS STILL ADDED**
(`d0ed12edb`, 2026-08-16) — this is the sharpening D135 earned and did not get.
D135 made the LDtk spine's six systems decline to RUN in the five RON games, via
`run_if(ldtk_world_installed)`. But the plugin was still ADDED: its six index
resources were still initialized, its systems were still in the schedule graph,
and **`root.ldtk_runtime_index` was still a row in those games' snapshot schema —
the fingerprint two peers must agree on.** ⇒ `run_if` stops EXECUTION; it does not
stop PRESENCE, and presence is what the wire format counts.

⇒ `PlatformerEnginePlugins` no longer adds `LdtkRuntimeSpinePlugin` and
`register_engine_rollback_state` no longer registers the index; both moved behind
`LdtkWorldPlugin`, which **Ambition — the game that actually has an LDtk world —
adds after the engine group.** The row's registration is byte-identical (same
name, kind, projection), so the LDtk composition's schema dump is unchanged and
no schema bump is owed.

⚠ **an honest deferral recorded with it**: the registration lives in the RUNTIME
crate rather than in `ambition_platformer2d_ldtk`, because the floor trait
`RollbackRegistrar` carries only the RESOURCE method and this index is a
COMPONENT on the session root. **Widening the floor is a separate slice**, and
saying so beats a facade.

✔✔ **RELOCATION OF THE LDTK HOLDERS IS EXHAUSTED, 2026-08-16 → 2026-08-18 —
production refs to `ambition_platformer2d_ldtk` in the monolith went 5 → 0.**
`WorldManifest` and the hot-reload watcher moved out (not LDtk-shaped); the
remaining five files were genuinely LDtk vocabulary
(`LdtkProject`/`LdtkLevel`/`ActiveLdtkProject`/`field_*`), so the cure was
INVERSION onto the room IR rather than further relocation. `EncounterTrigger` and
`LockWall` — markers the room-IR converter had deliberately dropped as "read by
their own consumers off the raw `LdtkProject`" — now emit through `RoomSpec`, so
`load_encounter_specs_from_rooms`, `authored_gated_lock_walls` and
`authored_switch_commands` all read `&RoomSpec` instead of the project.
`SwitchCommandSpec` and the lock-wall fields ride as their own typed family
alongside the CLOSED Tier-0 `PlacementSchema` rather than folding into it, so no
fingerprint/replay schema event was owed. Verified live (boot census unchanged at
`1 encounter entit(ies)`) and poison-verified at each step (the converter
emitting nothing turns the relevant loader/lock-wall/switch tests red); the
change signal moved with the data too (`ActiveLdtkProject::is_changed()` →
watching the room set), because a reload that rebuilds rooms under an unchanged
room id would otherwise serve stale derived state forever.

⚠ **one surviving production edge is a different KIND of thing, not a missed
inversion**: `assets/loading.rs` declares
`Handle<bevy_ecs_ldtk::assets::LdtkProject>` to load the file — an asset-loading
declaration that the app loads the file, not a consumer reading world facts off a
project instead of the room IR. Whether an asset collection in the monolith
should name the format at all is a separate boundary question.

✔ **the dep is `optional` now** in the monolith's manifest, taken back only
through `[dev-dependencies]` for tests.

⛔⛔ **AND IT MOVED THE COUNTER BY NOTHING — 44 crates / 17 unwanted before and
after — which is the finding, not a disappointment.** Asked why, `cargo tree -f
"{p} :: {f}"` says the sentinel builds the monolith with
**`ldtk_runtime, portal, portal_ldtk` already on**, so an optional dep is simply
enabled. **Two crates hard-code those features with no gate of their own:**

```text
ambition_sim_view              features = ["ldtk_runtime", "input", "portal"]
ambition_platformer2d_runtime  features = ["headless", "input", "portal_ldtk"]
```

⇒ **that is this row's thesis with a name.** The observation crate and the
runtime each decided that a movement-only game wants LDtk and portals, and wrote
it into a manifest — *"who is this for?"* answered by whoever declared it first,
in the one place nobody reads. ⭐ **and the optional dep is still a precondition,
not a wasted step**: with it unconditional the counter could not move no matter
what those two did.

✔✔ **AND THEY CAN NOW — AND THEY ARE GONE (2026-08-22). `ambition_platformer2d_ldtk`
AND `ambition_portal2d` LEFT THE MOVEMENT-ONLY CLOSURE: 44 → 42, `never_asked_for`
17 → 15**, and
third-party `bevy_ecs_ldtk` went with it. The ratchet is re-frozen at the new
floor, so a regression back cannot pass silently.

⭐ **it was THREE edges, and the row only named two.** The third was
`ambition_platformer2d_runtime` naming the LDtk crate UNCONDITIONALLY for its own
`LdtkWorldPlugin` and one `Option<LdtkRuntimeIndex>` slot on
`PreparedPlatformerSource` — five lines of code. That dep is `optional` now
behind a runtime `ldtk` feature, forwarded through the provider and the facade
and folded into `all_capabilities`, so a game that asks for every capability has
an LDtk world and a movement-only game has neither the feature nor the crate.

⭐⭐ **and the reason the other two could fall is that the probe result had gone
STALE.** This row recorded *"the MONOLITH itself does not build without
`ldtk_runtime`"*. Re-probed: `cargo check -p ambition_platformer2d_actor_monolith
--no-default-features --features visible` is **zero errors**. The cfg-gating
landed in the four months between; the recorded blocker outlived it. ⇒ **re-run a
probe before believing a blocker, exactly as you would re-grep a ▢.**

⚠ **the content fingerprint did NOT move, and that had to be checked.** The
provider writes `world.runtime-index` from the installed index; without the
capability there is none, and an absent index writes byte-for-byte what a
RON-authored game already wrote (one `area\t<id>\t\t-` row per room). The
provider's own comment states that equality, which is what makes this a
link-closure change rather than a content-identity one.

⛔ **and a mid-course error worth keeping**: I first read *"`ambition_sim_view`
no longer hard-codes the features"* off a single-line grep. It does — the list is
a multi-line TOML array, and `grep 'features.*ldtk'` cannot see it. The row's
original diagnosis was right all along.

⭐ **PORTAL FOLLOWED THE SAME DAY, and it was the same shape four more times.**
`ambition_portal2d` was named UNCONDITIONALLY by the monolith, the runtime,
`ambition_sim_view` and `ambition_projectiles`, and the runtime registered its
rollback row ungated — while the portal CODE was already `#[cfg(feature)]`d
everywhere. Only the manifests said otherwise. ⇒ **when a capability has a
feature and the code obeys it, check whether the DEPENDENCY does.**

⚠ **and making the dep optional exposed a lean nobody could see while it was
on**: `ambition_projectiles::diagnostics` uses `bevy::prelude::info` and was
getting `bevy_log` through `ambition_portal2d`'s bevy features. It names the
feature itself now. **Feature unification is not a dependency** — it is a crate
borrowing a neighbour's manifest, and it only shows up when the neighbour leaves.

⛔ **the counter moving is NOT the whole acceptance test — the sentinel has to
BUILD.** It did not, for two call sites in the monolith
(`try_projectile_portal_transit`, `portal_list`) that the earlier probe could not
reach because they only fail once `ambition_projectiles`' own surface is gated.
`cargo check` the sentinel, and run its tests, before believing a shrink.

⛔⛔ **EXCEPT THREE OF THEM, WHICH THE MONOLITH NEVER NAMES — measured
2026-08-22.** The baseline says every unwanted crate is *"reachable via
`ambition_platformer2d_actor_monolith` alone"*. For three it is not:

```text
ambition_settings_menu   0 monolith files   ← ambition_game_shell ← load_presentation ← facade
ambition_sfx_bank        0                  ← ambition_sfx ← ambition_audio ← game_shell
ambition_ui_nav          0                  ← ambition_dialog ← ambition_conversation ← facade
```

⇒ they wait on a gate in `ambition_sfx`, `ambition_dialog` and the game-shell
chain, **not on the monolith carve**.

⛔⛔ **BUT "THREE SMALL PIECES OF WORK" WAS WRONG — I WROTE IT FROM AN ERROR COUNT
AND THEN READ THE SURFACES.** A compile-error count from a failed build is a
FLOOR, not a measure: cargo stops early, so "4 errors" is the first batch and not
the cost. Reading what each actually holds:

```text
ambition_ui_nav        `RowPress` is a FIELD on a dialog runtime struct, plus
                       MenuFocusState / DialogChoiceSlot / resolve_selectable_row_
                       interaction across runtime.rs and systems.rs  ⇒ woven in
ambition_sfx_bank      `ambition_sfx` takes `fnv1a_64` from it — the SfxId HASH,
                       plus SfxBank/EntryRecord/Codec/BankError        ⇒ foundational
ambition_settings_menu ONE file (`game_shell/pause_menu.rs`): FOUR of the enum's
                       61 variants (Mute / Master / Music / Sfx volume) plus
                       `apply_settings_option`                         ⇒ a boundary CALL
```

⇒ **none is mechanical.** The first two are genuine dependencies; the third is a
real capability question — and the honest framing is that the shell REUSES the
settings IR rather than duplicating four volume rows, which is good design. What
it costs is a 2,423-line UI crate in every game's closure so that the pause menu
can change the volume. ⇒ *"a game shell whose pause menu cannot change volume"*
is the actual question, and it is Jon's.

⚠ **and `ambition_sfx_bank` is NOT a decomposition node** — I checked, because the
baseline's `ambition_binding` entry would have licensed reclassifying it instead
of counting it. Its introducing commit is *"sfx: introduce binary sound bank
format and runtime contract"*: a new format, not code the sentinel was already
linking under another name. The `ambition_binding` precedent does not apply and
the count stands.

⚠ **and `ambition_cutscene` is the near-miss worth naming.** Its whole hold is
ONE type — `CutsceneTriggerQueue`, used in two files of `ambition_boss_encounter`
— so it looks like a one-line gate. It is not worth taking: `boss_encounter`
itself is named by **50 monolith files**, so it stays in the closure regardless,
and splitting "a boss that triggers a cutscene" out of the boss domain to win one
crate is a capability claim made for a number.

⛔ **THE OTHER TWELVE REALLY DO WAIT ON THE CARVE — verified 2026-08-22, not
inherited.** Having moved two, I checked whether the other thirteen share the
property that made these cheap. They do not: **12 of the 15 are UNCONDITIONAL
monolith deps** (`audio`, `boss_encounter`, `conversation`, `cutscene`, `dialog`,
`encounter`, `items`, `persistence`, `projectiles`, `settings_menu`, `sfx`,
`vfx`) and the monolith has **no feature that would gate any of them** — its
whole feature list is `causal · portal{,_render,_ldtk} · {desktop,android,web}_platform ·
visible* · web* · headless · dev_tools · dev_hot_reload · static_* · android*`.
LDtk and portal were cheap because the CODE was already `#[cfg]`d and only the
manifest lagged; for these twelve there is nothing to lag behind. ⇒ this
baseline's 2026-07-30 note is correct and now re-verified — **do not re-probe
them one by one; the next move on this ratchet is the §4 carve.**

⚠ price: two tracked lockfiles (`minimal_game`, `capability_demo`) and one
gitignored one (`external_consumer`) — regenerate all of them, and Outlander
needs `CARGO_TARGET_DIR` pointed somewhere writable to verify.

⛔⛔ **THE ORIGINAL PROBE, KEPT AS HISTORY — its conclusion is the one corrected
above. Probed, 2026-08-18.**
Dropping `ldtk_runtime`/`portal` from `ambition_sim_view` fails to compile, and
**not in `sim_view`**: the MONOLITH itself does not build without `ldtk_runtime`.
Its own subsystem-gate comment already admits this — *"Code inside these
subsystems is not yet cfg-gated end-to-end, so disabling them today only works
when paired with `--features visible`"* — so the manifest lines are a SYMPTOM of
that, not the cause.

⭐ **and the ungated surface is much smaller than that sentence suggests.**
Measured: **two files**, and one of them is a comment.

```text
assets/loading.rs   30 LINES — one `use bevy_asset_loader::prelude::AssetCollection`
                    and one field `Handle<bevy_ecs_ldtk::assets::LdtkProject>`
session/setup.rs    a doc comment mentioning `bevy_ecs_ldtk`; no code
```

⇒ **the slice is: gate `Platformer2dStartupAssets` behind `ldtk_runtime`**, then
its four consumers, each of which needs its own feature to gate on:

```text
ambition_platformer2d_provider  lifecycle.rs:1181   Res<'w, …>  (NOT optional — the one to look at first)
ambition_platformer2d_actor_monolith  session/setup.rs:20,91   Option<&…>
game/ambition_app  setup_systems.rs:82, plugins.rs:292,327     Option<Res<…>> + `init_collection`
```

⚠ **the counter is the acceptance test**, and nothing before that last consumer
lands will move it — which is why this is written as one slice rather than four.

⇒ **what still holds the edge, and the cost of each**, in the order they must
fall (the last two cannot be cfg-gated cheaply — the code has to move):

```text
runtime    LdtkRuntimeSpinePlugin is in PlatformerEnginePlugins unconditionally,
           and LdtkRuntimeIndex is rollback-registered there. SMALL, and it is
           the exact successor to D135 — a format installs its own spine.
monolith   LdtkHotReloadState + poll_ldtk_file_changes, in features/mod.rs and
           persistence/settings/model. ~35 refs. NOT LDtk: a debounced mtime
           watcher over an Option<PathBuf> whose ctor takes an asset catalog.
monolith   menu/map/systems.rs builds map nodes by walking LDtk levels. It wants
           room metadata instead; needs a world rect on the room. MEDIUM.
monolith   world/gated_lock_walls.rs walks the project for LockWall entities.
           MEDIUM — the same inversion, onto the room IR.
monolith   encounter/loading.rs + encounter/systems.rs read LDtk levels/fields
           to build encounter specs. LARGE, and already planned: the comment in
           systems.rs says "W4 will route encounter loading through RoomEmission
           instead of the project".
```

⭐ **the sortable finding for the umbrella**: of the nine symbols, only THREE are
genuine format vocabulary held by production code (`LdtkProject`,
`ActiveLdtkProject`, `LdtkLevel`, reached through `field_string`/`field_f32`);
`LdtkVocabulary` is named by a TEST only; and `WorldManifest`,
`LdtkHotReloadState` and `poll_ldtk_file_changes` are engine concepts wearing an
LDtk name. One of the three is now gone. **⛔ the lesson for the next instance is
the measurement, not the move: run `cargo tree -i` for the crate you mean to
evict BEFORE choosing which code to carve.**

- ✔ **D135 — CLOSED 2026-08-16. The canonical session world carried an
  authoring-format-specific field (`runtime_rooms: LdtkRuntimeIndex`) and five
  RON-authored games filled it with `::default()` for a world they never
  install.** Fixed: the field became `Option<…>`, private, `None` by default,
  installed only by the LDtk road — taking with it a `demo_fixture` re-export,
  three setup systems, and six systems that had rebuilt against an empty index
  every tick in five games (now `.run_if(ldtk_world_installed)`). Guarded by two
  tests: a RON game has no LDtk index, asserted only beside a positive check that
  the LDtk-authored game installs a real, non-empty index — the negative alone
  would pass vacuously for a broken implementation. ⛔ an absence or population
  count taken with a repo-wide grep must exclude worktree clones (a grep swept
  `.claude/worktrees/`, inflating this row's own site counts). ⛔⛔
  `capability-footprint-may-not-grow` did not move (still 42 crates linked, 15
  unwanted) — dominated by the MONOLITH, not the session world; the next slice
  is a monolith carve at the world-manifest/asset-catalog seam (carried by
  D136, above).

- ✔ **DEATH RULES STOPPED BEING A PROCESS-GLOBAL `Resource` (2026-08-16)** —
  three games each inserted one in `Plugin::build` and the last one won, so
  every Smash match in the shipped host ran under Mary-O's death rules. Fixed by
  routing through `mode_scope`, the existing mechanism that already scopes a
  hosted game's systems and entities to its own rooms: a game declares into
  `DeclaredDeathRules` under the rooms it governs, `governing(mode)` answers
  "whose rules apply here?" through one `SystemParam`, an unclaimed room reads
  `LevelReset::Never`, and a second declaration over one scope panics at build
  rather than picking a winner. ⛔ `ExperienceScopeBuilder` does not fit this: it
  releases state on route departure with no entering half, so `DeathRules` would
  be deleted forever on the first departure. `sync_hosted_sanic_wallet_shield`
  (⚠ GONE — consolidated at `03d4c8d22`) was the same bug in miniature —  a
  system whose population was every `PrimaryPlayer` in the process rather than a
  global — and got the same fix.

- ☑ **D131 — CLOSED 2026-08-16. Four fighters were being divided by 1, 1, 60 and
  100.** `damage_percent()` is `accumulated / max`, and `max` was each
  character's HOME GAME's authored pool — Mary-O and Sanic author
  `max_health: 1` because they are one-hit-kill platformer protagonists, so one
  point of ordinary damage read as **4200%**. Fixed: the MATCH declares what
  100% means (`MatchRules::pool_over(authored)`, applied at both seat sites),
  deleting the 2026-07-31 per-character workaround that stamped a reference onto
  only the three ids that demo registers. ⛔⛔ the swap-to-P2 control pointed at
  the wrong cause: it proved the cause travels with the CHARACTER, but what
  travels is the authored VITALS, not a system — the crossover-plugin hypothesis
  was falsified in the same run (zero deaths, zero replays). What crossed the
  boundary was a NUMBER, not a rule.

- ☑ **D130 — CLOSED 2026-08-16 BY LOOKING. (a) There was no tofu — it was the
  STAGE FLOOR** (tiles and blurred parallax HUD chips, photographed at 3x with
  bevelled highlights and dark borders); it read as tofu because
  `--route smash_gameplay` with no roster puts the camera at its default
  position with no subject, so the floor sits alone with no scale cue. The HUD
  font fallback was innocent. **(b) FIXED** — `capture_scene` grew a step that
  carries a POSITION: `--press touch:XxY` sends the pair of real `TouchInput`
  messages winit emits, driving the same phone road the product ships. Cause:
  key taps are edges with no position, and the tool's bare `Enter` fired
  wherever the cursor sat. Guarded by
  `the_capture_tools_documented_taps_seat_two_cpus_on_two_fighters`.

- ▢ **D129 — The sprite pipeline CUTS ART AT THE LOGICAL FRAME AND NOTHING NOTICES.
  (opened 2026-08-16 from a maintainer observation, measured the same day)**

Jon: *"Super sanics spikes are clipped by the sprite renderer. This might need a
structural fix. We should not be able to clip sprite artwork so easily."*
⇒ **true, and it is not one character.**

✔✔ **GUARD LANDED 2026-08-16** (renderer `6228c58`) — the renderer now WARNS,
at draw time, when a frame's drawing runs off the logical frame, naming the
animation, frame and edges. It warns rather than raises because 52 sheets
already trip it and a fatal check would block regenerating anything until the
whole roster is redrawn; whoever fixes the art can make it fatal. Seven tests.

⛔⛔ **the sheet count is a BUILD-TIME observation, not a property of the
repository** — `clipped_frame_edges` runs from `sheet_build.py:943` on the
drawing canvas BEFORE padding, and the shipped PNG no longer carries the
evidence because the packer already trimmed it. Re-measuring means a full
regen of all 196 sheets (gitignored art), so `52 of 196` is a SNAPSHOT dated
2026-08-16/17 — anyone quoting it later should say so. ⭐ the practical
consequence: this row closes by REDRAWING, and the measurement comes free
with the redraw — the guard fires on the next build of any sheet somebody
touches.

⛔⛔ **the criterion that finds real cuts is: a truncated shape does not
TAPER** — compare the edge line to the widest the shape reaches a few lines
in; a tip narrows on its way out, a cut arrives already near full width.
Denominator-free, unlike the two criteria tried first: *"touches a
logical-frame boundary"* flagged 74 of 133 uselessly (with `auto_crop` the
frame is fitted to the art, so touching is normal), and this row's original
*"≥6 opaque px in a run covering >25% of the edge"* hid an unchosen
denominator and made the original **"23 sheets"** count untrustworthy.
Verified against the live alternative explanation (auto_crop merely hugging
the art) by the opaque-width profile of the topmost rows — a real tip tapers
from nothing, a cut starts wide:

```text
super_sanic  idle     top rows ->  12 14 17 18 20 22 24 25   ⛔ no tip: CUT
sanic        idle     top rows ->   0  0  0  0  3  6  8 11   ✔ a taper
sanic        jump     top rows ->   0  0  0  0  0  0  0  0   ✔ touches, NOT cut
player_robot_v3 idle  top rows ->   0  7  9 11 11 13 13 13   ✔ and off.y=73
```

⚠ **the taper test is right about walking poses and WRONG about a
bottom-anchored resting pose** — flat shoe soles arrive at the boundary at
full width and never taper, so an idle can read as cut and not be. The
discriminator for that case is drawing the same painter into a TALLER canvas
and checking whether ink actually appears past the boundary;
`bottom_center_canvas` is a plain paste, not an ink re-anchor, so nothing
downstream can put lost pixels back.

⛔⛔ **RE-MEASURED AGAIN 2026-08-26 BY RENDERING EVERY TARGET, AND THE LIST BELOW
IS OBSOLETE — the roster has moved on.** A sweep of all 186 registered targets
(`python -m ambition_sprite2d_renderer sheet <t>` each, the guard printing at
build time as designed) found a DIFFERENT population: **40 of 186 targets warn**,
worst first —

```text
hunny_horror_boss 59 · trex_enemy 57 · perfect_cellular_automaton 53
davy_hylbert 42 · le_beast 39 · pipi_tau 36 · flying_spaghetti_monster_boss 29
smirking_behemoth_boss 26 · paul_diracula 26 · puppy_slug 17
hypatia_prime · georg_canter · data_lovelace 13 each · mary_o_v2 12
super_mary_o 10 · paradox_barber 8 · carl_stargan 8 · super_mary_o_tall 7
mary_o_v2_tall 5 · galwah 5 · super_mary_o_fire 4 · busy_beaver 4
mary_o_v2_fire 3 · leib_knives 3 · five pirates 2 · mantis_lancer 2
snakes_on_a_cartesian_plane 2 · eight more at 1
```

⭐⭐ **AND SEVEN OF THE TEN NAMED BELOW AS WORST ARE GONE.**
`ninja_shadow_oni_leader` (was 73), `ninja_shadow_duelist` (70),
`player_combat_review` (108), `player_traversal_review` (100), `raid_enforcer`
(52), `fascist_enforcer` (53), `robot` (34), `player_extended` (30) all render
CLEAN today. ⇒ **the art was redrawn and nobody updated this row**, which is
exactly what its own warning about snapshots predicted.

⛔ **BUT `trex_enemy` (57) AND `puppy_slug` (17) ARE STILL CUT** — I said
otherwise before the sweep reached them, reading a partial result as a whole one.
`super_sanic` is genuinely fixed and out of the list; those two are not.

⭐ **THE GUARD IS NOT VACUOUS — CHECKED DIRECTLY**, because "everything is clean
now" is the shape that usually means the check stopped working:
`clipped_frame_edges` returns `['top']` for a synthetic shape that arrives at
the edge at full width, `[]` for a taper and `[]` for an empty frame.

⛔ **THE ONE THAT MATTERS IS `perfect_cellular_automaton` (53 frames)** — a
SELECT-SCREEN FIGHTER, so it is player-visible, unlike the bosses and NPCs above
it. That is where this row should start, and here is its diagnosis:

```text
frame            492 x 684, and `crop: enabled=False` — auto_crop is OFF
cut LEFT   36    slide#1-4, knockdown#3-4, +30 more
cut RIGHT  17    punch#3, dash_attack#3-4, smash_forward#5-6, special#6, +11
```

⇒ **both cuts are HORIZONTAL REACH on lunging poses**, not a spike or an
antenna, so `super_sanic`'s answer (scale one named appendage) does not
transfer — there is no single element to shrink.

⛔⛔ **AND "just widen the frame" IS REFUSED FOR THE SAME REASON IT WAS ON
`super_sanic`, checked here rather than assumed.** `pca_gameplay.py` authors
`ATTACK_HITBOXES` in ABSOLUTE draw coordinates (`_lens_attack(68, 78, 34, 28)`)
and `body_metrics(fw, fh)` normalises them by dividing by the frame
(`feet_x / fw - 0.5`). Changing `fw` moves **every** authored hitbox on the
character. ⇒ either the art gives, or the hitbox table is re-authored against the
new frame in the same commit — and the second is a bigger change than it looks.

⚠ **so this is ART AUTHORING WITH A GAMEPLAY CONSEQUENCE, not a constant sweep:**
shortening a lunge's drawn reach makes the drawing disagree with the reach its
hitbox still has. Worth Jon's eye before somebody picks a number.

✔✔ **CLOSED FOR `perfect_cellular_automaton` 2026-08-28 — and the refusal above
was WRONG about where the coupling lives, which is why it read as blocked.** The
claim was that `pca_gameplay.py` authors hitboxes in ABSOLUTE draw coordinates,
so a wider frame moves every one of them. It authors them in the LOGICAL RIG,
and `PADDING` is the single knob that produces both:

```text
pca_gameplay.PADDING = 18                       ← one constant
_px(v)                = (v + PADDING) * RENDER_SCALE      every hitbox
FRAME_SIZE            = (RIG + 2*PADDING) * RENDER_SCALE  the frame
perfect_cellular_automaton.py:173                         every art anchor
```

⇒ frame, art and hitboxes all move together BY CONSTRUCTION — the coupling the
note warned about is what `PADDING` was built to handle, and `noether_gameplay`
already runs the same pattern at 28. **`PADDING = 45`, and all 53 clipped frames
are gone**, measured by asking the tool rather than modelling it:

```text
18 → 53 clipped (left 36, right 17)   492x684   7 pages
30 → 25 clipped (left 24, right 1)    564x756
40 →  1 clipped (right 1)             624x816   7 pages
45 →  0                               654x846   7 pages
```

⭐ **the page count does not move, because the packer trims transparent margin
before packing** — so the atlas cost of this fix is nothing.

⛔⛔ **BUT IT SURFACED A DEFECT BIGGER THAN THE ROW, and this is the part worth
carrying: A FRAME IS A DIVISOR OF HOW BIG A CHARACTER IS.**
`catalog_join.rs:154`, the branch taken when `body_kind` has no default height:

```text
scale     = ldtk_max × collision_scale / FRAME_H
collision = body_px × ldtk_max × collision_scale / FRAME_H
```

⇒ widening PCA's frame with **no art change whatsoever** took its body from
**67.8 to 54.8** and broke `duel_pca_body_is_sprite_authored_not_the_tiny_ldtk_box`
— a test that had every right to fail. ⇒ **D129 cannot be closed by widening
frames alone**: on that road the only cure for a cut is a bigger frame, and a
bigger frame silently resizes the character. **27 of 134 catalog rows are on it**
(every `Wide`/`Floating`/`Crawler` not authoring a height).

⇒ **the fix that comes with the un-clipping, and the pattern for the rest of this
row**: author the character's `standing_height` in the same commit. PCA now
carries `standing_height: Some(67.8)` — the height it had before the frame moved
— which is the same move the three pirate heavies got on 2026-08-22, and it takes
a select-screen fighter off a road where a publishing detail decides its size.
✔ app gate clean, `ambition_demo_smash_app` 39/39, and
`print_the_two_render_size_publishers` still reports 0.0% disagreement over 146
characters with PCA at exactly `28.1x67.8`.

▢ **THE OTHER 25 ARE THE REST OF THIS ROW, AND ONE THING MAKES THEM HARDER THAN
PCA — census 2026-08-28.** Every catalog row whose `body_kind` has no default
height and which authors none:

```text
Wide      bear_mauler · boss · dark_lord · flying_spaghetti_monster_boss
          hand_saint · hunny_horror_boss · le_beast · mantis_lancer
          mockingbird_boss · ninja_heavy · raptor_stalker · smart_house
          smirking_behemoth_boss · trex_enemy · viking_heavy_shieldmaiden
          viking_heavy_warrior
Floating  imperfect_cellular_automaton · burning_flying_shark · stochastic_parrot
Crawler   puppy_slug · puppy_slug_variant2 · puppy_slug_velvet
no sheet  giant_gnu · giant_gnu_hands · gnu_ton_boss   ← reach NEITHER branch
```

⛔⛔ **AND "just author the height it has today" DOES NOT WORK FOR THEM, which is
the trap to avoid.** PCA had a number to preserve because a test pinned its size
in one room. These do not: on the legacy road the size is
`body_px × ldtk_max × collision_scale / FRAME_H`, so **a character is a different
size in every room whose spawn box differs** — that is the defect, not a detail
of it. ⚠ the sizes `print_the_two_render_size_publishers` prints for them are
computed against `LDTK_PLACEMENT = (28, 44)`, a generic humanoid box **no boss
room uses**, so quoting that table as "today's size" would resize a boss without
anybody deciding to.

⇒ **the honest next step is a measurement, not an edit**: for each of the 25,
find its actual spawn box across every level it is placed in. Where a character
has ONE box, its height is determined and authoring it is a pure road-swap with
no look change — the same move PCA and the three pirate heavies got. Where the
boxes DISAGREE, the character has no size today and picking one is Jon's, and the
disagreement itself is the argument for why.

⭐ **and the prize is a deletion**: once no row takes it, `catalog_join.rs:154`'s
`_ =>` arm goes, and with it the last place where a level-editor rectangle and a
sheet's publishing padding decide how big somebody is. The three `no sheet` rows
do not block that — they return `None` before either branch.

✔ **AND THAT MEASUREMENT IS DONE — 2026-08-28, every `.ldtk` in
`game/ambition_map_assets` scanned for each of the 22's spawn boxes. 19 of 22
have EXACTLY ONE, so for them this is a pure road-swap with no look change:**

```text
one box, (48,80)   bear_mauler · boss · dark_lord · flying_spaghetti_monster_boss
                   hunny_horror_boss · mantis_lancer · mockingbird_boss
                   raptor_stalker · smart_house · smirking_behemoth_boss
                   trex_enemy
one box, (32,48)   imperfect_cellular_automaton · hand_saint · le_beast
                   ninja_heavy · puppy_slug_variant2 · puppy_slug_velvet
                   viking_heavy_shieldmaiden · viking_heavy_warrior
```

⛔⛔ **THE THREE THAT DISAGREE ARE THE ARGUMENT, and the worst is the one Jon has
already complained about.** `npc_puppy_slug` is placed with **eight different
boxes** — `(48,22)`×6, `(32,48)`×5, `(64,32)`×2, `(48,32)`, `(64,16)`, `(52,66)`,
`(42,42)`, `(28,44)` — so the same crawler is eight different sizes across the
game, and `(64,16)` versus `(52,66)` is a factor of four in the axis that drives
the derivation. `stochastic_parrot` has three, `burning_flying_shark` two.

⇒ **19 rows can be authored with no decision at all; 3 need one number each from
Jon**, and for those three the disagreement above is the whole case for asking.

✔✔ **THE NINETEEN ARE AUTHORED — 2026-08-28, and every one reproduces the size it
already had.** Read from the production function rather than derived: an ignored
report (`print_the_height_each_legacy_row_already_has`) asks
`sprite_body_collision_for_character_id_in` with each character's REAL spawn box,
so the number a row carries is measured and not computed by hand. Before and
after agree to the rounding authored (≤0.1px): `npc_bear_mauler`
`121.86x72.56 → 121.93x72.60`, `npc_hunny_horror_boss` `119.25x154.80` unchanged,
`npc_puppy_slug_velvet` `68.79x30.86 → 68.88x30.90`.

⭐⭐ **THE LEGACY BRANCH'S POPULATION IS 25 → 3.** Every remaining row is one whose
spawn boxes DISAGREE, which is the argument rather than an omission:

```text
npc_puppy_slug            EIGHT boxes, (64,16) to (52,66) — a factor of four
stochastic_parrot         three
npc_burning_flying_shark  two
```

⚠ **and the three GNU-ton rows are not a fourth case**: `npc_giant_gnu`,
`npc_giant_gnu_hands` and `npc_gnu_ton_boss` publish no sheet, so
`sprite_body_collision_for_character_id_in` returns `None` before either branch
is reached. They do not block deleting `catalog_join.rs:154`'s `_ =>` arm.

✔ **AND UN-CLIPPING BECAME SAFE FOR NINETEEN CHARACTERS THE MOMENT THEY AUTHORED
A HEIGHT — `npc_trex_enemy` is the first, 2026-08-28.** 57 of its 60 frames were
cut (every idle on the left, the tail swipe and death on the bottom), and its
root sits at a FRACTION of a work frame rather than at a padding constant, so
the fix is the frame and the fraction moved together: work `832x640 → 1000x760`,
frame `416x320 → 500x380`, root `x 0.33 → 0.4426` / `y 0.72 → 0.6063`. Zero cut
frames, still one page. ⭐ its derived height is **exactly 70.50 either way**,
because the authored number is now the input — the whole point of the previous
paragraph.

⚠ **its collision WIDTH moved 131.6 → 143.3, and that is the fix rather than a
side effect**: the measured body grew because the tail that used to be cropped is
now drawn, and `every_characters_drawing_is_the_size_of_the_body_it_collides_with`
is the assertion that these two must describe one creature.

⛔⛔ **AND A LARGE SHARE OF THE 40 WARNING TARGETS ARE NOT DEFECTS — the row's own
caveat, now MEASURED and with a one-render test that settles it.**
`hunny_horror_boss` is the sweep's worst (59 frames) and warns on the BOTTOM
only. Rendering it into a canvas 25% taller changes **nothing**: same warning,
same 59 frames, and the measured body bbox is byte-identical at `103x133` inside
a `144x150` frame. ⇒ **no ink exists past the boundary**; the bear is
bottom-anchored, its soles are flat, and the taper test cannot tell that from a
cut. Exactly the failure this row wrote down and never applied.

⭐⭐ **THE DISCRIMINATOR, and it costs one render:** grow the target's
`FRAME_SIZE` and re-read `body_pixel_bbox` in the generated `.ron`. Unchanged ⇒
the guard is describing the crop, not a cut. Changed ⇒ real, and the new bbox is
the art you were missing (`npc_trex_enemy` went `131.6 → 143.3` wide the moment
its tail stopped being cropped).

⚠ **the edge pattern predicts it well enough to triage by**, checked across five
targets 2026-08-28:

```text
BOTTOM only, on a stand/rest      suspect — a flat sole arrives at full width
ALL FOUR, on a rest               suspect — auto_crop fitted the frame to the art
LEFT/RIGHT on a lunge or a roll   REAL (perfect_cellular_automaton, davy_hylbert,
                                  pipi_tau — rolls and deaths on both sides)
TOP on an idle or a jump          REAL (le_beast)
side/bottom on a tail or a limb   REAL (npc_trex_enemy)
```

⇒ `hunny_horror_boss` (59) and `smirking_behemoth_boss` (26, all four edges on
`rest`) are suspect and should be MEASURED before anybody redraws them; the
headline "40 of 186 targets warn" is an upper bound on the work, not the work.

⚠ **AND THE TWO CONFIRMED-REAL ONES LEFT HAVE A THIRD FRAME SHAPE — priced 2026-08-28,
not attempted.** `davy_hylbert` (42) and `pipi_tau` (36) warn identically (crouch
bottom, roll left AND right, death both) and are sibling generators: both
`FRAME_W = FRAME_H = 128`, both drawing a pose at ABSOLUTE canvas coordinates
with no root fraction and no padding constant. So neither of the two fixes that
worked today transfers:

```text
perfect_cellular_automaton   ONE constant (`PADDING`) produced frame, art AND hitboxes
npc_trex_enemy               a frame plus a ROOT FRACTION, moved together
davy_hylbert / pipi_tau      absolute pose coords — enlarging the frame leaves the
                             character in the corner, and `FaceGuide(source_width=
                             FRAME_W, …)` normalises the PORTRAIT crop against it
```

⇒ their fix is a canvas grow PLUS a pose translation PLUS the same shift applied
to `FaceGuide.center_x/center_y`, on two files. That is the *"art authoring with
a gameplay consequence"* this row already flags, and it wants an eye on the
result rather than a mechanical edit.

⚠ **`le_beast` (39, TOP on idle and jump) IS THE SAME SHAPE, not a fourth** —
checked 2026-08-28, correcting an earlier line here that called it a third one.
`FRAME_W = FRAME_H = 128`, `auto_crop=False`, and every stroke at an absolute
canvas coordinate (`(64 + side * 10, 108)`). ⛔ and the cut is on the TOP, which
is the awkward direction: extra room added to a canvas lands at the BOTTOM unless
the whole drawing moves down, and there is no single origin to move it by — the
constants are spread through `_draw_character`. ⇒ three targets, ONE remaining
technique, and it is the expensive one.

⛔⛔ **DO NOT UN-CLIP `npc_puppy_slug` (17 cut frames) UNTIL ITS HEIGHT IS
AUTHORED.** It is one of the three still on the legacy road, so a wider frame is
a DIVISOR change and would resize it — which is exactly the trap this row found
on the automaton. Its two crawler variants already author heights and are safe.

▢ **so the arm's deletion gate is three numbers**, and they are Jon's — see
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) entry 36.
⛔ do NOT pick them by taking the most common box: the puppy slug's two most
common are `(48,22)`×6 and `(32,48)`×5, which are 22 and 48 in the axis that
drives the derivation, so "most common" is a coin flip dressed as a measurement.

⚠ **the list below is the 2026-08-16/17 snapshot, kept as the record of what was
true then:**

**Re-measured with the taper criterion: 52 of 196 sheets, with frame counts.**
Worst first — `ninja_shadow_oni_leader` 73 frames (all four edges),
`ninja_shadow_duelist` 70 (all four), `player_combat_review` 108,
`player_traversal_review` 100, `trex_enemy` 57 (bottom+left), `super_sanic` 54
(top — Jon's report, now fixed, see below), `raid_enforcer` 52,
`fascist_enforcer` 53, `pulse_voyager_captain` 48,
`perfect_cellular_automaton` 45, `goblin_shaman_staff` 39,
`tech_bro_disruptor` and `goblin_cantina_chieftain` 35, `robot` 34,
`robot_guardian` 33, `m_leblanc` 32, `player_extended` 30, and 35 more with
fewer frames each (`pirate_admiral` only 2, `oiler_vfx` 1). The control that
makes it causal: base `sanic` is clean and only `super_sanic` (same body,
`spikes_up=True`) is cut.

⭐ **23 sheets collapse to far fewer causes by source YAML** — eight
(`robot`, `player_extended`, both player `*_review` sheets,
`robot_caster`/`diver`/`miner`/`runner`) are auto-emitted from
`robot_spritesheet.yaml`; the two `sandbag_*_review` from
`sandbag_spritesheet.yaml`; `ninja_shadow_oni_leader` and
`ninja_shadow_duelist` share `ninja_spritesheet.yaml` — ~15 authoring sources
total, the robot family the largest (cut edge `top`, the antenna spike in
`player_robot_v3.py`; `robot_spritesheet.png` is embedded in
`ambition_asset_manager`, so this ships). The current player draws
`player_robot_v3`, the CLEANEST sheet measured (margin 20) — name which sheet
a character actually draws before calling any of this player-visible.

✔✔ **super_sanic — the sheet Jon reported — FIXED 2026-08-18** (renderer
`39d79a7`): the raised fan and back blade are scaled by a named
`SUPER_SPIKE_FIT = 0.76`, the largest value measured that leaves every frame
whole (swept 1.00→61/181 cut, 0.85→4, 0.80→2, 0.76→0), poison-verified
against base `sanic` (0 of 181) so a later change that shrinks every spike or
grows the frame cannot pass. ⛔ **"just make the frame bigger" is refused
with cause**: `auto_crop` is deliberately OFF for this sheet so
`ATTACK_HITBOXES` coordinates match draw space — growing the frame or
shifting the body would silently move every authored hitbox. For a sheet
with authored hitboxes in draw space, the ART is the only thing that can
give — worth checking before proposing a canvas change on any of the other
~15 authoring sources.

✔✔ **Mary-O's walk-frame clipping FIXED 2026-08-18** (renderer `a17b8bf`):
every walk frame put her foot up to a fifth of a tile below her own standing
line (`+dy` is down; the trailing leg's `leg_back_dy=+1.0` at toe-off pushed
the foot through the line instead of lifting off it; the passing pose's
`bob=+0.4` was the same mistake at a third the size) — a leg-reach sign bug,
not a framing one. Fixed to `+0.00` on both forms; 7 frames left the
clipping guard's list (`mary_o_v2` 14→13, `mary_o_v2_tall` 11→8,
`mary_o_v2_fire` 6→3); the three canonical images are byte-identical
through it. ⭐ the guard asserts against her OWN idle, not the frame height —
her standing line moves with per-form scale, so "below the frame" would
have been a proxy for "below the floor." Poison-verified.

▢ **what is still cut on Mary-O, and it is NOT a pose** — measured
2026-08-18: three numbers that should be one, at 6px/authored-unit on a
192px published frame:

```text
                    drawn sole    authored collision_bottom_px    foot socket
small (one brick)     194 px               190 px                   176 px
grown (two bricks)    190 px               192 px                   176 px
```

⇒ the small form's sole is 2px below its own frame (the sliver still cut on
every frame of that sheet) and 4px below its collision box; the grown form's
sole is 2px above its box; and both forms' `foot_r`/`foot_l` sockets are the
same hardcoded `output_px(88.0)`, 14–18px above where either foot actually
is. ⛔ **not fixed here on purpose** — every repair moves where she STANDS,
and that is D165's call (Jon, by eye: "small Mary-O is one brick, grown is
two"); the measurement was the missing part, which of the three numbers is
authority is his, and the sockets are the one no form currently derives.
`grow`/`shrink` (both sheets) and `death#0` (top) are separate frames with
their own reasons.

✔✔ **the renderer's own suite was RED and is now 1 failed / 620 passed**
(triaged 2026-08-18, ten of eleven were bad tests, not code — read what a
check computes before believing what it reports):

```text
✔  test_no_raw_imagedraw      found a REAL defect en route: the boot thruster's
                               nozzle ellipse replaced the bloom's alpha instead
                               of compositing (61 of 396 player frames lost glow)
✔  test_svg_parts_cache  x2   message named a dependency (resvg_py) that WAS
                               installed — `_native_resvg_callable` requires
                               `inspect.isbuiltin` by design, the tests' pure-Python
                               fake was refused, and the fallback (CairoSVG,
                               absent) raised a message about resvg instead
✔  test_actor_contract        a dead exemption plus a proxy question
✔  test_character_notes  x2   written for a schema that changed under them
✔  test_robot_slash_hitboxes x2  froze one build of the art as the requirement
✔  test_geometry_gui          held a reference the drag handler REPLACES
✔  test_portrait_product      froze which clips a boss draws from
✔  test_every_registered_character_target_has_local_actor_metadata  its exemption
                               was dead (rig-doc targets carry metadata fine) and
                               its question was a proxy (module constant vs a
                               function); the fix's first draft guessed the
                               attribute name and returned `{}`, staying red for a
                               new reason that looked like the old one
✔✔ test_rig_codegen_and_scale THE ONE REAL BUG — see below
```

✔✔ **the one real bug, CLOSED 2026-08-18** (renderer `6162a4a`): a target
generated from a rig document rendered a different image from the document
it came from. `RigDocument.render_at` downsamples through
`resize_transparent_sprite(reducing_gap=3.0)` specifically because a Lanczos
kernel's negative lobes leave a pale halo on a silhouette over transparency;
`rigdoc_codegen` still emitted `img.resize(..., LANCZOS)` — the exact call
the interpreter had deliberately stopped making. Alpha difference across the
body was 15 against a tolerance of 2; both roads now take the same
downsample path including the `SS == 1` short-circuit. Every character
published from a rig document had been shipping the halo the interpreter
removed.

⛔ **do not bulk-fix a red suite** — each failure is a different question,
and two are "is the assertion or the art right", a look-at-it call. One
remaining failure (Mary-O's visual baseline) stands against an uncommitted
side-view strap edit in the worktree — it belongs to whoever lands that art,
in the same commit.

✔ **AND THAT SURVIVOR IS GREEN NOW — re-run 2026-08-26, and the whole file
passes (12/12).** `python -m pytest tests/test_rig_codegen_and_scale.py` in
`tools/ambition_sprite2d_renderer` reports
`test_generated_matches_rigdoc_render` PASSING, so the two roads emit one picture
again. ⛔ what fixed it is not recorded here and the row below never named the
cause, so treat the paragraph that follows as the SYMPTOM DESCRIPTION it was —
useful if it ever comes back, not a live finding. ⚠ and the shape it describes is
worth keeping for that day: `39x78 vs 43x82` is a UNIFORM 2px fringe on all four
sides, which is a stroke-width or antialias-radius difference rather than a
transform — *"every 1px shift makes the match WORSE"* is exactly what a dilation
looks like when you try to explain it as a translation.

~~▢ **and one survivor is genuine, left red on purpose:**
`test_generated_matches_rigdoc_render` compares the rig document's own
renderer against the module generated from it, and they disagree on every
clip:~~

```text
alpha delta       max 11, >2 on 355 px, >8 on 8 px   (the tolerance is 2)
visible delta     max 19.8/255 on 137 px, mean 0.32
alpha bbox        rigdoc 39x78 vs codegen 43x82 — a 2px fringe all round
NOT a translation — every 1px shift makes the match WORSE
```

⇒ composited, the two pictures are indistinguishable; the disagreement is a
sub-pixel rasterization difference, and what introduced it is unknown. ⛔ do
not "fix" it by re-basing the assertion on a visible-difference metric —
that would hide that two roads meant to emit one picture no longer do.

⛔ **why nothing caught this at all**: the drawing canvas IS the logical
frame, so overflow is clipped at draw time before anything downstream can
see it — the packer's post-trim losslessness check compares trim geometry
against the logical frame and cannot see ink that was never drawn. The guard
has to be at draw time, in the renderer, stating the invariant once over
discovery rather than once per sheet.

✔✔ **the fix has two separable halves, and (b) is RULED 2026-08-17: CASE BY
CASE, DRIVEN BY THE WARNING.** (a) the draw-time check [landed above]; (b)
the sheets that are already clipped — a sheet is fixed when its clipping is
actually visible in play, and whether that's by growing the logical frame or
re-authoring the art is a per-sheet call. ⛔ **do not open a bulk campaign
and do not bulk-grow frames** — the population stays known-bad by design,
driven sheet-by-sheet by the warning.

⛔⛔ **this orders against `BodySource::SpriteAuthored` migration**: the
renderer's measure-by-default principle ([`engine/sprite-renderer.md`](engine/sprite-renderer.md))
reads a clipped sheet's geometry faithfully off art that was already cut, so
migrating a clipped body to `BodySource::SpriteAuthored` would bake the cut
into its collision box. Fix the clipping first for any body in both lists.

- ◐ **D128 — Can this engine carry a serious platform fighter through ORDINARY authoring? (product-pressure vertical slice, opened 2026-08-15; ⭐⭐ EVERY ENGINEERING LINE IS ✔ AS OF 2026-08-18 — what is left is Jon playing one match)**

⭐⭐ **the executable list below is EMPTY.** Pacing was ruled, respawn
placement and asset composition landed, CPU symmetry landed, and all four
presentation defects are closed — 5 and 6 fixed and photographed, 7 was
fixed three hours after it was reported, 8 verified live. ⛔ **so do not
open this row looking for work**; it stays ◐ only because its QUESTION is a
product one and the answer is a person watching a match, not another
capture.

✔✔ **PACING IS ACCEPTED — RULED 2026-08-17.** Jon, verbatim: *"A three-stock
CPU showcase finishing in under ~40 seconds is certainly not too long; if
anything it is brisk. **Do not retune stock count, knockback, or damage**
around that partial 20-second frame. … Human-vs-human balance can be judged
separately later."* ⭐ he also corrected how the row read its own capture —
it led with a 1200-tick frame (180%/124%, nobody dead) and buried the fact
that at 2400 ticks (~40s) the match had COMPLETED and returned to CHOOSE
YOUR FIGHTER. ⇒ **when a capture sweeps time, the acceptance question is
answered by the LAST frame, not the most alarming one.**

⇒ **so what is left in this row is ENGINEERING, not acceptance:**

```text
✔  seat-independent respawn placement           FIXED 2026-08-18 (defect 3)
✔  standalone smash-app asset composition       FIXED 2026-08-18
✔  the residual presentation defects            5 FIXED + photographed · 6 FIXED (it DID
                                                reproduce; the 08-18 "not reproduced"
                                                was a 40x40 scan window hunting a 19x19
                                                artifact) · 7 was already fixed 08-16,
                                                3h after it was reported · 8 FIXED + live
✔  same-character CPU symmetry — FIXED 2026-08-17, see below
⛔ NOT on this list: stock count, knockback, damage. Ruled. Do not retune them.
⛔ NOT on this list: another ladder run.
```

⚠ **"another capture is done" was true of the ACCEPTANCE question and NOT of
the defects.** Captures on 2026-08-18 confirmed defect 5 live, verified
defect 8's fix end to end, and — once the warmup landed on **360**, the tick
the report actually names — caught defect 6 in the act; two earlier warmups
(300, 420) settled nothing. ⭐ the documented tap recipe still works: nine
taps seat two CPUs and start a match unchanged.

✔✔ **CPU SYMMETRY: TWO CPUs WEARING ONE CHARACTER WERE THE SAME MIND, AND
THAT IS FIXED (2026-08-17).** The fighter brain seeded its noise stream from
difficulty alone (`0x5F37_7A11 * (level+1)`), so any two CPUs on one rung
drew byte-identical noise and mirrored each other exactly on a symmetric
stage — every other template in `brain_builders.rs` already varied off
`seed_from_id(&enemy.id)`; the fighter was the sole outlier. Fix: seed from
`PreparedSeat::feature_id` (`"<character>#seat<n>"`), distinct per
participant rather than shared by every twin; no clock, no process-global
RNG, no Bevy `Entity`, so replay determinism holds
(`the_same_participant_rebuilds_on_the_same_stream`). ⛔⛔ needed a SECOND
site: `project_authored_fighter_ladder` was rebuilding `FighterState` with
the same level-only constant on `Added<Brain>`, so it now CARRIES
`state.noise` across the rebuild.

⭐⭐ **Emmy Ethereal now AUTHORS the old (shared-stream) behaviour as her own
trait**, not inherited from the bug — `CharacterDefinition::preserves_mirror_symmetry`
(`.preserving_mirror_symmetry()` in `authored/npc_emmy_noether.rs`) drops the
participant term and keys on the character, so only her twins share a
stream. It does not zero the seed and touches nothing else; it synchronises
NOTHING per tick (the mirror breaks correctly once observations diverge —
`the_same_seed_shown_a_different_world_may_decide_differently` is the
falsifier). No `if character == Emmy` in the AI: generic code reads a bool
the character authored, riding `ActorConfig` (`rollback_component_clone`).

⭐⭐ **measured on the real stage**: under the old shared stream, two same-character
CPUs stayed an EXACT mirror image (equal-and-opposite about the midline,
identical y, to the float) for a whole match. ⛔⛔ the first draft of that
test was VACUOUS — it measured DISTANCE, which passes with the defect fully
present since two fighters spawned apart drift regardless of their brains.
The metric that answers the question is **MIRROR ERROR**,
`|(x0−mid)+(x1−mid)| + |y0−y1|`. Both the fix and the ladder projection's
carry are poison-checked.

✔✔ **and Emmy is now pinned in the full host too**
(`game/ambition_app/tests/smash_cpu_cognition.rs`, ungated) — the standalone
smash app cannot seat her at all (no `ambition_content`), so this is the
first place *"the character a player can actually pick off the grid gets the
shared stream"* is asserted. ⭐ measured in the full host, rung 5, two CPUs
on one character:

```text
                     streams     mirrored for       match ran
npc_emmy_noether     IDENTICAL   2576 of 2576 fr    2576 fr   a stalemate
npc_pirate_admiral   DIFFERENT    488 of 1548 fr    1548 fr   they fight, and it ends
```

⚠⚠ **488 frames is ~8.1s, and Jon reported it from play**: *"it took a while
for Booule to desync, but they eventually did. And Emmy never desynced.
Still the desync for non-Emmy CPUs probably should happen sooner."*
⛔⛔ **the cause is NOT the seed, and two fixes were built, measured and
reverted**: the stream has exactly ONE consumer (press-timing jitter, only
on a decision that commits to an attack), so a different RNG cannot separate
two bodies doing the same thing. Per-participant decision phase: 488→220fr
but broke five behavioural guards in `the_stage_kills` (a 0-4 tick offset
changed whether attacks connect) — reverted, too high a price. Cadence
drawing from the stream: 220→219fr, nothing — reverted.
⇒ ✔ **RE-MEASURED 2026-08-26 AND THE SPAWN FIX DID IT — 488 frames became
188.** `probe_when_the_mirror_breaks` (`the_stage_kills.rs`, print-only) runs
the same mirrored two-CPU match and reports the FIRST observed tick past 1px of
reflection error:

```text
before (ledger, pre-spawn-fix)   488 frames ≈ 8.1s
now                              188 frames ≈ 3.13s, at 3.56px of error
```

⇒ **asymmetric CIRCUMSTANCES were the answer, exactly as this paragraph
predicted**, and the two reverted randomness fixes were right to be reverted: a
stream with one consumer cannot separate two bodies doing the same thing.
⚠ Jon's own words were *"the desync for non-Emmy CPUs probably should happen
sooner"* — 8.1s → 3.1s is a direct answer to that, and whether 3s is soon ENOUGH
is his call rather than a number to keep chasing.
⛔ the probe reports WHEN; its sibling assertion
(`two_cpus_wearing_one_character_stop_being_a_perfect_reflection`) must stay a
WHETHER-question, because pinning when would pin the tuning of a demo. ⛔ do not build a
third randomness fix before reading the note at
`brain_builders::fighter_cognition_seed`.

⚠ **the catalog PREVIEW brain still seeds level-only, correctly** — a
preview has no match and therefore no participant; the note at
`character_catalog/resolver.rs` forbids copying that back onto a
construction road.

⭐ **the state acceptance was given against a two-CPU match captured
2026-08-17, AFTER D155 gave the game working knockback** (34%→180% in
thirteen seconds, real exchanges with hit VFX, the stock loop closing on its
own) — every feel judgement recorded anywhere in this row before that date
was made on a build where nobody was ever launched, and is void.

⛔ **the eight named defects from the 2026-08-16 photo session — do not
re-derive their status, do not re-run the capture:**

```text
1 self-KO on every stock  ◐ substantially repaired — the CAUSE was architecture
                            (RecoveryLens, 2026-08-15); the 2026-08-17 ladder
                            re-run reads d0 no self-KO / d12 first at 21.8s.
                            ⛔⛔ the RecoveryPolicy ledge-grab diagnosis this row
                            used to name is RETRACTED and banned — a photograph
                            falsified half the CPU-suicide finding.
2 camera loses the fighter ✔ CLOSED — `frame_the_cast` always framed every live
                            seat; three downstream sites threw it away (room
                            clamp, stable_center, 8 Hz ease). Guarded by
                            `every_live_fighter_stays_inside_the_frame`.
3 both seats respawn at    ✔ CLOSED 2026-08-18 — `respawn_placement` takes the
  ONE overlapping point       SEAT; seats alternate outward from the centre, so
                              the arrangement is symmetric at any roster size
                              and no seat is privileged. Guarded by
                              `every_seat_comes_back_to_its_own_point_on_the_platform`
                              (no two within a body width, symmetric about the
                              centre, every seat still ON the platform). The
                              pre-existing `a_respawn_is_above_the_stage_centre`
                              asserted `respawn.x == centre.x` — the defect
                              stated as an invariant — so that clause was
                              corrected rather than deleted.
4 winner card names a SEAT ✔ CLOSED by D140/D148 — the card reads
                            `WINNER: Robot v3`, a team keeps its team name.
5 barks draw as a          ✔ CLOSED 2026-08-18, photographed both ways. Overlap
  screen-wide caption         half closed via D158→D159 (a bubble is a
                              `WorldLabel` in the one ranked placement pass).
                              Scale half: before 535px of a 1280px frame (41%
                              of a 640-wide stage), after 185px in a centred
                              column (−65%) — the cause was that a bark had NO
                              WIDTH AT ALL (`spawn_speech_bubble` set only
                              `font_size`). ⚠ width bound only, never height:
                              `TextBounds` truncates past its bound on wrap, so
                              a height bound would silently eat a long bark.
6 untextured olive quad    ✔ CLOSED 2026-08-18 — it reproduces, and the fix is
                              that art the engine already ships now gets asked
                              for. Cause: `spawn_impact` draws a bare rectangle
                              BY DESIGN as its no-decoded-sheets fallback — not
                              a failure (`note_effect_miss` logged nothing).
                              The art (`generic_action_fx`: hit_soft/hit_hard/
                              hit_metal/hit_energy) was on disk the whole time;
                              `spawn_hit_marker` now draws `hit_soft` at 0.9x
                              `FX_DEFAULT_WORLD_SIZE`. ⛔⛔ 2026-08-18's first
                              verdict ("not reproduced") was an INSTRUMENT
                              ARTIFACT: the scan flagged any 40x40 window >95%
                              one colour, and the artifact is 19x19 — a
                              systematic scan is only as fine as its window.
                              ⇒ not taken here: `ImpactMaterial` and the
                              sheet's four hit rows are two vocabularies still
                              unjoined — asked as §18 in
                              [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md),
                              Jon's taste call (`hit_hard` is a strength
                              distinction, not material, so material picks the
                              row explains only 3 of 4 rows).
7 VFX authored against no  ✔ CLOSED 2026-08-18 — was ALREADY FIXED 2026-08-16,
  size reference              three hours after it was reported (`d6d5810b8`,
                              same day as the `39dc7a39b` report): inline
                              `render_size = BVec2::splat(132.0 * scale)` is
                              now `FX_DEFAULT_WORLD_SIZE = 56.0` plus a
                              per-move `Vfx { scale }`. At the measured 1.594x
                              zoom the old 132 units drew at 210px; the new
                              largest VFX in the warmup-360 frame is 107px (56
                              units at the Admiral's authored scale 1.20)
                              around a 46-unit fighter. ⇒ before photographing
                              a defect, `git log` the file it names — an
                              observation older than the commit that fixed it
                              is not evidence about today.
8 capture_scene prints no  ✔ CLOSED 2026-08-18, verified live on a real
  pose for a 2-CPU match      two-CPU match: `seat 0 at (350.4803, 276.0000)` /
                              `seat 1 at (233.9185, 276.0000)`, where it printed
                              NOTHING before. Reports each SEATED body, sorted
                              by SEAT (not query order, which would make two
                              captures of one match differ). When there is
                              neither a primary player nor a seated body it now
                              says `NO SUBJECT … this image proves nothing
                              about a pose` instead of printing nothing.
```

✔ **standalone `game/ambition_demo_smash_app` CLOSED 2026-08-18** — it
composed no asset install at all (no `PlatformerAssetsPlugin`), the only
demo shell that never joined the pattern `ambition_demo_mary_o_app` and
`ambition_demo_sanic_app` already use
(`PlatformerAssetsPlugin::for_experience(SMASH_EXPERIENCE).with_room(...)`
then `PlatformerPresentationPlugin`, AFTER `compose_smash_shell` because the
plugin READS the catalogs it registers). ⚠ Smash reached through the shell
(`ambition_app`) is a DIFFERENT, always-fine composition — say which binary
any claim is about.

⚠⚠ **what is and is not verified**: ✔ 33 headless tests still pass and both
feature configurations compile. ⛔ the plugin's `build()` was NOT executed —
the crate's tests are gated out under `visible`, so `cargo test --features
visible` links and runs ZERO of them; the ordering rests on matching two
working shells and on the plugin's own panic naming the mistake, not on
booting art-less. ▢ **a windowed run is what closes the loop, and it is the
one thing this session could not do.**

⛔ **answered and must not be re-asked**: *"do the two authored kits read as
different fighters?"* — YES, inside four seconds. *"is the VFX/SFX road
reachable?"* — YES (`ebc8877ee`). *"has anybody watched a match?"* — YES,
twice.

⭐⭐ **the outcome is pinned, not just the mechanism** — `the_stage_kills` (17
tests, green) includes `a_second_match_on_the_same_stage_counts_in_and_ends`,
CPU vs CPU, one stock, so the end arrives from a single CPU-produced launch
reaching the blast zone rather than a test writing a velocity. ⇒ the
05:45 capture showing both fighters at 180%/124% with 3/3 stocks was a
mid-match MOMENT, not evidence of a broken KO.

✔ **the capture is done — 2026-08-17, three frames of one two-CPU match**
(George Booul vs Pirate Admiral, documented taps, 1280x720): 420 ticks
34%/36% both fighting; 1200 ticks 180%/124% still nobody dead; 2400 ticks
back at CHOOSE YOUR FIGHTER, every slot NOT PLAYING. The stock loop closes on
its own; percent climbs and fighters engage (34%→180% in 13s with hit VFX) —
first time true since D155. ✔ shown to Jon and accepted 2026-08-17 (see the
pacing quote above).

✔ **the capture found one real presentation defect, closed same-day as
D158→D159**: speech bubbles stacked illegibly (two CPUs taunting at once is
the ordinary case on this stage) — fixed by making a bubble a `WorldLabel` in
the one ranked placement pass, not by retuning the stack offsets.

⛔ **not a defect, checked before reporting**: George Booul renders as a
white ghost — that is his authored art (the select-screen portrait grid
shows the same ghost).

◻ **HISTORY, done twice (2026-08-16, then 2026-08-17 after knockback
worked) — do not run a third capture to establish status; read the ACTIVE
TRUTH above.** The 2026-08-16 capture (21 frames, two matches) found the
documented nine-tap command seats the WRONG pair: `747x121`/`425x121` are
grid cells 3/0 (Sanic and Player Robot v3, the generic-kit floor), not the
authored pair. ⛔⛔ **and the correct cell coordinates have moved AGAIN,
since 2026-08-20** — George Booul is cell 1 (`touch:518x105`) and the Pirate
Admiral cell 4 (`touch:761x105`) as of 2026-08-27, when a 21-fighter grid
re-flowed to SIX columns and moved every column rather than shifting cells
along by one; the previous recipe (`touch:479x121` / `touch:801x121`) landed in
the GAPS between cells and both taps missed. Cell 1 and cell 4 are still the
same two fighters — that they were NAMED is what caught it. The 2026-08-20
drift before that was an appended fighter taking the roster grid from 15 cells
to 16 and re-flowing it to six columns. ⚠ quote the
CELL, not the pixels. ✔ **DONE 2026-08-26, and it was ONE literal, not two —
`capture_scene`'s header no longer prints a recipe at all** (it says *"keep
coordinate recipes covered by host tests because the UI layout can move"*), so
only the test carried literals. `PORTRAIT_A` was landing on grid CELL 0
(`player_robot_v3`) instead of cell 1 (`smash_george_booul`), which is exactly
the drift this row predicted. ⛔⛔ **AND THE ASSERTION IS WHY NOBODY SAW IT: it
said the two picks DIFFER, and two wrong fighters differ perfectly well.** The
test names the pair now (`WANTED = [smash_george_booul, npc_pirate_admiral]`) and
resolves the cells back through the live grid, so the next roster change reddens
it instead of quietly re-aiming the documented capture. Poisoned by restoring the
old literal: it reports `["player_robot_v3", …]`. ~~the two literals still
need updating to the current cells (~15 min)~~ — otherwise every future look
through the documented command answers the standing "do the kits behave
differently" question with the wrong pair. That 2026-08-16 capture's own
headline: the two authored kits DO read as different fighters, but every
stock in both matches was lost to the void at ≤6% damage — since fixed by
D155's knockback repair; the pacing/self-KO status above is current.

✔✔ **first fighter landed and verified (George Booul, 2026-08-15)** — the
gap was mostly AUTHORED REPERTOIRE, not missing engine: `special_*`,
`Cancelable`/`OnHit`, per-volume `on_hit`, `motion_scale` tails, multiple
Active windows and per-move Sfx/Vfx were all already sufficient and unused.
The one real gap — a move could not displace its owner at a chosen moment,
nor command a speed rather than add to one — became
`MoveFrameData::{lift_speed, lift_at_s}`, now used by the recovery probe as
`RecoveryLift`. ⚠ two gaps named rather than half-built: no per-airtime
budget for an airborne self-impulse move, and `WindowTag::Invuln`/`Armor`
parse but have no consumer.

✔✔ **second fighter landed too (Pirate Admiral, 2026-08-15)** — a
materially different lateral/grapple recovery concept; recovery routes are
now evaluated through the real movement kernel (`movement/recovery.rs`)
rather than ranked by a static "this is the recovery move" property. ⛔ do
not redispatch "make a second fighter".

✔✔ **the CPU lane landed and was measured (2026-08-15)**: distinct attacks
used per match went 5-6 of 16 → 9; all four of George's specials now appear;
the duelist's whole vertical game (air_up/air_down/smash_up/tilt_up) was
absent and is now present. Two causes, both initial hypotheses wrong: (1)
the attack stick wrote a facing-relative `+x` into a field the resolver
already multiplies by facing, so a forward/back attack chosen while facing
left came out reversed AND at full deflection (a FLICK, never a tilt); (2)
the kernel's `.first()` route-search fallback endorsed a recovery in only 3
of 100 `Situation::Recovery` decisions — deleted outright.
✔ Jon's three couch items also fixed: camera close eased (was 237-361 units
in one frame against 33-49/frame open ramp, now 68.9); the match-end race
(Smash despawning the eliminated body while `decide_stocks_match` read sides
off bodies that still existed) fixed by ordering; the countdown was firing
into `GameplayBannerRequested`, which nothing drew — the winner card had the
identical defect with a unit test that only asserted the message.

✔✔ **VFX/SFX road landed 2026-08-16 (`ebc8877ee`): an effect is a NAME, and
the engine ships the art it draws.** Design owned by
[`engine/render-animation-and-vfx-extension.md`](engine/render-animation-and-vfx-extension.md).
`FxId` is the authored row name on the
wire (`SfxId`'s FNV-1a hash, borrowed not re-typed). `ambition_sprite_sheet::fx`
resolves name → (sheet, row, `vfx.<family>.<row>` cue); `GameAssets.fx` is
the engine-owned home. FIVE reconstruction tables deleted (`ExplosionKind`,
`move_vfx_kind`, `explosion_anim`, `explosion_sfx`, plus five
`CharacterAnim::from_name` aliases spelling FX rows as Idle/Walk/Run/Hit/
Slash). The "no world at validation time" constraint that looked like it
would force a hand-kept 189-row table dissolved instead: `build.rs` already
bakes every `*_spritesheet.ron`, so `ambition_sprite_sheet::fx::is_authored_effect`
is a pure, world-free oracle, and `expand` takes it as a parameter rather
than naming the crate. **189 authored rows ↔ 189 cues, one for one, across
all twelve FX sheets, no sheet off by one.** ⚠ none of the underlying art is
in git (gitignored) — a fresh clone needs `./scripts/regen/sfx.sh` and
`./scripts/regen/sprites.sh` to get it; the roster commit is the durable half.
⛔⛔ this is what exposed the standalone-Smash-app asset-install gap above —
same defect shape one level up.

⚠ **the product question, still standing** (this is what the row's ◐ is
waiting on):

> can someone watch a CPU-vs-CPU match and immediately see several
> mechanically distinct attacks, aerial choices, specials, an intentional
> recovery move, expressive movement and convincing impact — and conclude
> this engine can elegantly support a serious platform fighter?

**Scope:** one existing body, ≥8 materially distinct attacks (rotated clones
do not count), a real authored Up-B in the ordinary moveset architecture, a
launcher, a punish/kill move, a mechanically interesting aerial, authored
SFX/VFX through normal content mechanisms. ⛔⛔ CPU usage is part of
acceptance: the generic policy layer must actually use the repertoire (≥5
distinct offensive move ids, aerials, a special, the authored Up-B for
recovery). ⛔ no character-ID conditionals in AI — derive affordances from
move data (coverage, startup, reach, launch direction, commitment, impulse)
before adding annotations; only a technique whose behaviour cannot be
inferred from static geometry (teleportation) may expose its own affordance.

⛔ **not authorized:** grab/throw architecture · shields/parries as a
subsystem · ledge-rule parity · many characters · balance · animation
redesign · combo scripting · networking · no character-specific system to
compensate for a missing generic mechanic. ⚠ an authored Up-B is the one
sanctioned link to `RecoveryPolicy` reachability (its default presses only
`side ∈ {0,±1}` plus jump) — do not begin the general navigation graph.

⇒ **why this lane runs with the other two** (maintainer's exception): the
combat lane is orthogonal enough to the systemic-world and rollback lanes to
stay independently integrable. ⛔ narrow or pause it the moment it starts
changing the same authority boundary as another live lane.

- ✔ **D211 — CLOSED 2026-08-24. `Exit Match` STAYED OFFERED AFTER THE MATCH WAS
  ALREADY OVER. (opened and closed 2026-08-24, from the GPT 5.6 correction-pass
  review, P1)**

The pause menu offers an abandon action whose whole meaning is "end a match that
is still running". Once `StocksMatchSettled::settled(active)` is true the match
has ended on its own, and the action either does nothing or races the settlement
it duplicates.

⇒ **withdraw the action on the settled condition, not on a mode label.** The
condition already exists and is already the authority the settle path reads —
see `features/stocks_match.rs`. ⛔ do not gate it on a menu state or a game-mode
enum: those are proxies, and the row is about the OUTPUT (is this match over),
which is exactly the discipline the same review asked for.

✔ **LANDED.** `offer_to_exit_the_match` now reads `StocksMatchSettled` beside
`ActiveMatch`. ⭐ **AND THE DOC COMMENT ALREADY CLAIMED THIS** — *"A match that
has been DECIDED is still active... offering to abandon it then would be offering
to stop something already stopped"* sat above `let offer = on_stage &&
active.is_some()`, which does not check it. A comment describing a rule the code
does not implement reads as coverage.

⛔ **THE TEST HOLDS THREE THINGS AT ONCE** — still on the gameplay route,
`ActiveMatch` still installed, offer gone. The offer is ALSO withdrawn when the
route changes and when the match resource goes away, and both happen a few
seconds later, so a test asserting only the absence passes for the wrong reason.

- ✔ **D212 — CLOSED 2026-08-24 (`f4cee3e63`). TWO CONSUMERS ASKED "HOW LONG HAS
  THIS MATCH BEEN RUNNING" AND GOT DIFFERENT ANSWERS. (opened and closed
  2026-08-24, from the GPT 5.6 correction-pass review, P1)**

The timeout and the item-spawn cadence each measure elapsed match time their own
way, and neither excludes the opening countdown or a pause. So a match paused for
thirty seconds is thirty seconds closer to timing out, and items keep their
schedule through a freeze.

⇒ **ONE live-match clock, owned once and read by both.** The clock excludes the
opening countdown and every pause. ⛔ not a third timer beside the two — the
deletion is the proof: both existing readings go away in the same slice.

⚠ items are currently OFF by default (`roster.item_spawns = None`, Jon
2026-08-23), so the cadence consumer is dormant. It is still a consumer, and the
clock is what makes turning items back on a one-line change rather than a
re-derivation.

✔ **LANDED.** `character_runtime::live_match_clock::LiveMatchTicks`, counted by
one system in `WorldPrep` so both readers downstream see THIS tick's number.
Both private readings are deleted, including the spawner's hand-written
`elapsed == 0` — which was standing in for "not during the countdown" and stops
being true the moment an interval is shorter than a countdown.

⭐ **THE FOUR REASONS A TICK DOES NOT COUNT ARE EACH THE CONDITION ITSELF**: no
active match · `opening_phase` still `Counting` · `sim_dt == 0` · already
settled. `sim_dt` rather than a pause flag, because "is the world moving" is what
the clock wants and a menu state would miss every other reason it stopped.

⛔⛔ **AND IT COSTS WIRE FORMAT, which the thing it replaces did not.**
`time_remaining`'s own doc calls itself a pure function of `(activated_on, now)`
and names that as why a match clock needs no snapshot bytes. `SimTick` advances
while paused — deliberately, it is the netcode timeline — so "how long was this
stopped" is written nowhere else and a rewind must restore the count. Registered;
schema v87.

⛔ **THE CEREMONY HALF IS PINNED ON THE PRODUCTION PATH**, not in a unit test:
`PreparedMatch` has no constructor outside `prepare_match`, and adding one so a
fixture could set a countdown would be scaffolding on a production type.

- ✔ **D213 — CLOSED 2026-08-24 (`bd854bd74`). `sim_random` HAD NO MATCH-CONTEXT
  SEED, so two matches drew the same sequence. (opened and closed 2026-08-24,
  from the GPT 5.6 correction-pass review, P1)**

`sim_random(domain, tick, salt)` is stateless and rollback-safe, which is the
whole point — but its inputs carry nothing that distinguishes one match from the
next. Two matches that reach tick N in the same domain draw identically.

⇒ **fold a match/session context value into the seed.** It must be canonical
simulation state (it decides draws, so a rewind has to reproduce it) and it must
be stated where a match begins, beside the roster and the rules.

⛔ THE POISON IS TWO MATCHES, NOT TWO DOMAINS. The correlation assertion for this
belongs on the RAW draws — a check written on reduced indices compared values
differing only by modulus, passed, and could not have detected a shared salt.

✔ **LANDED.** `sim_random(domain, context, tick, salt)` — four axes, four
questions. The context is `MatchInstance::random_context()`, the activation stamp
mixed with the session id, both already canonical state a rewind restores. The
session half matters because a fresh session restarts the sim clock: without it
the FIRST match of every session is identical.

⛔ **`context` GETS ITS OWN MULTIPLIER rather than being added to the tick.**
Stamps are ticks, so consecutive matches carry nearby numbers, and `tick +
context` makes match A at tick 10 draw what match B drew at tick 9. Asserted
directly.

⚠ **ONE LINE IS UNREACHED BY ANY TEST** — `spawn_match_items` passing the context
— because `PreparedMatch` has no constructor outside `prepare_match`, so no
fixture can give the spawner a rules table. Same blocker as D212's ceremony half.
⇒ **if a third row hits it, add the constructor**; two is a coincidence, three is
a missing seam.

- ✔ **D214 — CLOSED 2026-08-24. MULTI-SIDE SUDDEN DEATH RESET EVERY SURVIVOR,
  not the tied leaders. (opened and closed 2026-08-24, from the GPT 5.6
  correction-pass review, P1)**

Sudden death exists to break a TIE. With three or more sides alive at timeout the
current path carries every survivor into the extra round, including sides the
timeout had already put behind — so a player who was losing on stocks gets an
even restart.

⇒ **carry the tied leaders only.** The tiebreak already computes the ordering it
needs; the row is about what the sudden-death round is POPULATED with, not about
how the winner is decided.

✔ **LANDED.** `SuddenDeathBegan` now names its `contenders`, drawn from
`leading_sides` — and `clock_outcome` is REBUILT on that function rather than
sitting beside it, so a Winner is exactly its one-element case and the two
readings cannot disagree about who was tied with whom. The stage puts the
contenders on the authored damage and retires the rest with the same
`FighterEliminated` an exhausted fighter is out with, so
`take_eliminated_fighters_out_of_play` clears those bodies and
`last_side_standing` decides the round among the contenders.

⛔ **"LEAVE THE NON-CONTENDERS ALONE" IS WORSE THAN THE BUG** — they would keep
their own low damage while the tied sides go to 150%, so the side that LOST the
tiebreak enters the round ahead. Both wrong readings are poisoned: carry every
survivor, and skip the else arm.

- ◐ **D215 — MECHANISM LANDED 2026-08-25 (`e06333002`), AWAITING A CUSTOMER. A
  hit could only hurt; there was no way to author a volume that PUSHES and
  nothing else. (opened 2026-08-25, promoted from the smash-parity inventory at
  Jon's direction: "keep pushing smash parity for the mechanics")**

`smash-parity-inventory.md` rows *Windboxes / flinchless push* and *Vacuum /
suction hitboxes* are one primitive, not two: a hit reaction that applies
VELOCITY without damage, hitstun or shield interaction. Suction is the same
thing with the push aimed inward.

⭐ **THE SEAM IS ALREADY THERE, AND `autolink` IS THE PRECEDENT** — measured
2026-08-25, so the next session does not re-derive it:

| where | what it already does |
| --- | --- |
| `ambition_entity_catalog/src/lib.rs:306` | the authored volume carries `autolink: Option<AutolinkVolume>` — a per-volume REACTION OVERRIDE, exactly the shape a windbox wants |
| `combat/src/moveset/mod.rs:1225` | carries the authored field onto the spawned `Hitbox` |
| `combat/src/strike.rs` | `Hitbox` holds it for the live volume |
| `combat/src/hit_reaction.rs:165` | ⭐ the branch point: `match knockback.follow` already chooses HOLD instead of LAUNCH, and writes ONE velocity under the victim's own body authority |

⇒ **a windbox is a third arm of that same `match`.** ⛔ but the reaction is only
half: the other half is UPSTREAM, in the damage-apply path, which must not spend
damage, hitstun, a hit-repeat slot, or shield on a gust. Find that before
writing the arm — the parity row's own words are *"keep contact/faction/shield
arbitration in the normal hit path"*, so the volume still resolves targets the
ordinary way and only its REACTION differs.

⛔ **IT NEEDS A CUSTOMER, and that is the acceptance criterion.** The inventory's
sibling row *Cannot-clank/transcendent hit* is marked UNBLOCKED AND DELIBERATELY
UNBUILT for exactly this reason. Author one real move that uses it — a gust that
pushes a fighter off the ledge is the genre's own example — and let the move
prove the primitive. A field with no authored consumer is a mechanic that ships
green and inert, which this demo has now done three times.

⚠ a windbox that hits repeatedly is the POINT (a gust pushes for as long as you
stand in it), so check what the hit-repeat window does to it rather than
inheriting the ordinary answer.

✔ **LANDED, AND IT WAS HALF-BUILT ALREADY.** `damage_floor` had already been
taught that a damageless volume is a windbox, so such a volume was authorable and
already LAUNCHED. What it still did was stun its victim and spend its hit-once
slot. The slice is therefore only the remaining difference: `flinchless` (the
pulse declines to charge hitstun) and `repeating` (an authored opt-out of the
hit-once set). Three tests, each poisoned.

⛔ **THE FIRST DRAFT WAS WRONG AND IS WORTH REMEMBERING**: `WindboxVolume` was
given its own `push` vector, which duplicates `knockback` + `launch_dir` — two
ways to author one thing. A gust is thrown the same way a punch is; the type
carries only what the ordinary fields cannot say.

⛔ **AND `flinchless` DECLINES TO CHARGE STUN RATHER THAN CLEARING IT.** `= 0.0`
is the obvious spelling and would have made a windbox the best combo breaker in
the game.

▢ **WHAT REMAINS IS A CUSTOMER**, and it is a character-design decision rather
than an engineering one — see `awaiting-maintainer-decision.md`. ⚠ until a move
uses it this is an UNADOPTED mechanic, which this demo has shipped three times;
`match_report` should show a windbox connecting before the row closes. ⭐ the
inventory's separate *"Vacuum / suction hitboxes"* row needs no further work —
same primitive, launch aimed inward — so one authored move closes both.

- ✔ **D216 — CLOSED 2026-08-25 (`003654071`). THE UP-TILT PERCENT GUARD WAS
  MEASURING ITS OWN FIRST STRIKE. (opened and closed 2026-08-25)**

`an_up_tilt_launches_much_further_at_a_high_percent` landed both strikes on ONE
victim, and the second arrived while the body still carried the first —
`hitstop_timer` read 0.088 at contact, so the launch was 495.8 px/s where
`base + growth * damage / weight` says 3269.

```text
struck FIRST   1427%  ->  3096.9 px/s, rises 398px   (what the formula predicts)
struck SECOND  1427%  ->    495.8 px/s, rises  38.1
```

Same damage input, same victim weight (1.0), same empty stale queue, same
attacker damage. ⛔ waiting for the thaw was NOT enough (23.2px, still red): a
body launched once differs from a fresh one in more ways than the freeze, so each
percent now gets its own match. The `* 10.0` bar is untouched and the ratio is
now ~100x.

⭐⭐ **THE LESSON IS THE ELIMINATION.** It was first attributed to the character
lane on the strength of a sound elimination — reverting every file the
match-clock lane touched did not move the numbers — and that conclusion was
wrong. **"Not mine" is not "theirs."** Both lanes were innocent and the test was
measuring itself; a third suspect nobody had named was never eliminated because
nobody thought to name the FIXTURE.

- ✔ **D218 — CLOSED 2026-08-25. SHIELD TILT: THE GUARD LEANS, AND THE LEAN
  COSTS. (opened and closed 2026-08-25, from the smash-parity inventory)**

The coverage rule already sank a spent guard until the head and the feet came
out from behind it — symmetrically, around the body's centre, with nothing the
player could do about WHICH end they lost. Tilt is the answer to that, and it is
one number: `ShieldTuning::tilt_range`, a fraction of half-height, `0.0` on
every body that declares nothing.

⭐ **IT COMPETED WITH NOTHING.** Past `SPOT_DODGE_STICK` (0.5) the stick is
already a roll or a spot dodge; below it, while shielding, the stick did
nothing at all. The genre puts shield shift in exactly that dead band, so the
threshold that already existed partitions the input and no new gesture had to
be invented. The rule itself is just the stick, so a body whose evade is spent
or on cooldown still leans rather than going inert.

⛔⛔ **THE SHIFT HAD TO COST, AND THAT IS THE WHOLE MECHANIC.** A tilt that only
ever covers MORE is a free upgrade every player would hold the stick for
permanently — indistinguishable from raising `min_coverage` and strictly worse
than it, because it hides in an input instead of being declared. So the band
MOVES rather than widening: lean toward the feet and the head is exposed by the
same amount. The cost arm is asserted beside the benefit and poisoned
separately (P1 stop shifting, P2 widen instead of shift); P2 passes every
benefit assertion, which is exactly why it needed its own arm.

⛔ **ONE AXIS, and it is not a simplification.** Coverage is measured along the
body's own gravity; the lateral question is already answered by which side the
body FACES. A left/right tilt would be a knob with no rule to bias.

⭐ **RESOLVED ONCE, READ TWICE.** `apply_shield` writes
`BodyShieldState::shield_tilt`; the hit test shifts its band by it and
`ShieldRingsView` shifts the drawn bubble by the same half-height. Deriving it
at each consumer would let the picture show a guard the hit test does not use —
the same disagreement `break_total` was introduced to prevent. A guard that is
DOWN holds no lean, so the next raise does not start already leaning.

⭐ **TWO TEST HALVES, because a pure-function test proves the FUNCTION and not
the WIRING.** `guard_covers_hit` is exercised against a tilt handed to it
directly under all four gravities; a kernel test then drives a real gentle
stick through `update_player_with_tuning_scratch` and reads the resolved value.
The kernel half caught its own fixture: the body was AIRBORNE for all three
readings — `on_ground` is re-derived every step, so setting it by hand held
only until the body's real height was consulted. The falsifier beside the
assertion is what surfaced it; the tilt values had been right the whole time.

⛔⛔ **THE MECHANIC SHIPPED INVERTED FIRST, AND THE TEST AGREED WITH IT.** The
first draft negated the stick — `-stick_up * range` — on the assumption that
`+y` is up. It is not: `LocalAxes` is documented `+y toward-feet`, which
`wants_drop_through` confirms by naming its parameter `descend` and firing on
`y > 0.35`, and the spot dodge confirms again with `stick.y > SPOT_DODGE_STICK`.
The kernel test was written from the SAME wrong assumption — it fed `-0.3` and
called it "down" — so it passed against inverted code and proved nothing about
direction. ⭐ THE CORRECT RULE HAS NO SIGN FLIP AT ALL: the stick's y is
already in the axis and sense the coverage rule measures on, so a negation was
the whole bug. Caught only by reading a NEIGHBOURING consumer of the same axis,
not by any test. ⇒ when a value's sign decides a mechanic, assert it against the
axis's documented name, and go read another consumer of that axis first.

⚠ **AND A FILTERED TEST RUN THAT NEVER RAN THE TEST.** `--lib guard` matched
three neighbours and not `a_tilt_trades_one_end_of_the_body_for_the_other`,
whose name contains no "guard". It reported `ok` and it had measured nothing.

Wire format v92 (`BodyShieldState` gained a field, `ShieldTuning` gained one).
⭐ the schema dump names TYPE PATHS, not fields, so neither baseline moved
except the version line — the version bump is the only thing carrying this
change to a peer, which is precisely what it is for.

- ✔ **D219 — CLOSED 2026-08-25. GUARD + DOWN DROPS THROUGH A SOFT PLATFORM,
  AND THE TERRAIN IS WHAT DECIDES. (opened and closed 2026-08-25)**

⭐ **NO NEW GESTURE.** Guard + down was already the spot dodge. On a one-way
surface the genre gives the same press to the platform drop, so the surface
under the body arbitrates and the player learns one input, not two.

⛔⛔ **THE MECHANISM THE ROW SAID TO REUSE WAS UNREACHABLE.** Drop-through lives
INSIDE `handle_jump_buffer_clusters`, behind
`if !current_press && state.buffer_jump <= 0.0 { return; }` — so a body with no
jump press and no buffered one returns long before it. Any declaration of the
guard gesture would have been swallowed silently. It needed its own arm ahead
of that gate, sharing the OUTCOME (`begin_drop_through`, one function) and not
the entry. ⇒ "reuse the existing mechanism" is a claim about the OUTCOME; check
what GATES it before believing the road is open.

⛔ **AN EXPLICIT DECLARATION, NOT A FALLTHROUGH ON `out_of_shield`.** That gate
reads a game with no policy as "restricts nothing", which for this action points
the wrong way: it would hand a platform drop to every exploration body that has
a shield and a one-way surface, waking a mechanic all of those worlds were tuned
without. `ShieldTuning::platform_drop` sits beside `air_guard`, which is not an
out-of-shield action either and for the same reason — which game a stage
reproduces is a declaration, not a permission.

⭐ **ONE QUESTION, TWO PHASES.** The control phase must stand the evade down or
it eats the press; the simulation phase must fire the drop. They are separate
top-level passes, so the condition is ONE function both call
(`platform_drop_requested`) rather than two copies that can drift — two copies
that disagreed would produce a press that cancels the dodge and drops nobody.

⚠ **A FIXTURE THAT DECLARED HALF A RULESET.** The test set the platform-fighter
SHIELD but left `spot_dodge_time` at the engine default, and `apply_dodge` falls
through to the ROLL when that window is zero. So guard+down chain-rolled the
body 455px in 90 ticks, off the end of the floor, and read as "the drop fired
when it should not have". Declaring the window fixed it. ⇒ a fixture that
authors half a ruleset is not that ruleset, and the half it omits is the half
that changes what an input MEANS.

⭐ Every arm asserts its OWN premise: the landing check moved inside the helper
after one arm was found measuring a body that had never reached the platform.

Wire format v93 (`ShieldTuning` gained a bool).

- ✔ **D220 — CLOSED 2026-08-25. THE SHIELD ROLL DID THROW THE FIGHTER ACROSS
  THE STAGE, AND MY OWN INSTRUMENT IS WHAT HID IT. (Jon's observation
  2026-08-24, wrongly closed as "does not reproduce" the same day)**

⭐⭐ **THE ROLL NEVER TOOK ITS VELOCITY BACK.** It sets `vel` once at the start
and the timer runs out; nothing clears it. So the body rolled 124px, then
COASTED at the full 530px/s for the rest of the 0.42s cooldown, then rolled
again. Measured on a ~480px stage, held guard+direction:

```text
before   per roll-cycle 229.7px   3 seconds -> 1339px   (~2.8 stage widths)
after    per roll-cycle 114.8px   3 seconds ->  804px
```

⛔⛔ **THE EARLIER "DOES NOT REPRODUCE" WAS AN INSTRUMENT ARTEFACT, AND I HAD
ALREADY WRITTEN DOWN THE REASON.** The probe measured through `App::update()`,
which is a FRAME and not a sim tick — a caveat recorded in that same probe, with
the ground roll explicitly cleared as "slow enough to survive it". It was not:
**11.2px is ONE TICK of a roll** (530 ÷ 60 = 8.8px), read as the whole roll.
Four "refuted" numbers in the observations file were all the same error and are
withdrawn there.

⇒ ⭐⭐ **A MEASUREMENT THAT EXONERATES THE SUSPECT DESERVES MORE SCRUTINY THAN
ONE THAT CONVICTS IT** — it is the one that ends the investigation. The kernel
was available the whole time and gives a tick that is a tick.

⭐ **THE FALSIFIER THAT MADE IT CERTAIN:** the same held press on a body with
`dodge = false` travels **0px**, and run speed is 270 against a roll speed of
530. So the guard does stop walking, and every one of those 1339px was the roll.

⛔⛔ **IT MAY NOT END BY ZEROING `vel`, AND THE TREE ALREADY KNEW.** A comment at
the expiry site records that exact attempt erasing a struck body's knockback —
`an_up_tilt_launches_much_further_at_a_high_percent` saw a victim rise 4.5px at
0% and 0.0px at 1427% — and prescribes the fix: *"the push has to be tracked
separately from whatever else moved the body."* So `AxisManeuverState` now
stamps `dodge_roll_push`, and the shed runs only while the body is still going
the roll's way and no faster than the roll pushed it. ⭐ THE ASYMMETRY IS
DELIBERATE: shedding too little leaves a body coasting a few frames, shedding
too much deletes someone's knockback, so the doubtful case does nothing. The
naive version is the second poison and it reproduces the historical bug exactly
(launch 3000 -> 0).

⛔⛔ **THE FIRST SHED RULE SHIPPED WRONG (`f79ad7c9b`), AND ONLY AN EMERGENT
TEST SAW IT.** It allowed the shed whenever the body was going the roll's way
"no faster than the roll pushed it" — which a body launched the roll's own way
at 300px/s satisfies, so that launch was deleted. `the_stage_kills::
every_live_fighter_stays_inside_the_frame` failed on its PREMISE guard: with
launches being eaten, NOBODY in a whole match was ever knocked out of the room
(0 body-frames, needs >20). ⭐ THE PREMISE GUARD IS WHAT CAUGHT IT — the
assertion the test exists for still passed.

⇒ ⭐⭐ **THE TEST IS EQUALITY, NOT A BOUND.** The roll's push does not decay
while the roll runs — gravity acts along `down`, not `side`, and nothing applies
friction to it — so a side speed still exactly equal to the push proves nothing
else touched it, and any other value proves something did. Fixed forward.

⚠ **MY THREE-ARM UNIT TEST COULD NOT SEE IT** because arm 3 used a 3000px/s
launch, which is FASTER than the roll and so was refused for the wrong reason.
A fourth arm now uses a launch SLOWER than the roll, with an assertion that it
IS slower so it cannot silently become a copy of arm 3. ⇒ when a rule is a
comparison against a magnitude, the arms must straddle that magnitude.

⚠ **WHAT REMAINS IS A TUNING NUMBER, NOT A DEFECT.** The roll's own 124px is a
quarter of the stage; `DODGE_ROLL_SPEED` is one value and Jon plays it tomorrow.
Not pre-emptively lowered — halving a knob and fixing a bug in the same change
makes neither measurable.

Wire format v94.

- ✔ **D221 — CLOSED 2026-08-25. ASDI: THE NUDGE A MULTIHIT CANNOT DENY YOU.
  (opened and closed 2026-08-25, from the smash-parity inventory)**

⭐⭐ **THE DISTINCTION IS WHAT IT IS PAID PER, and that is the whole reason the
genre has both.** This tree's SDI is already automatic — `sdi_step` shifts a
frozen body every tick of hitlag from the held stick, no flick counting. So the
question the row raises ("keep it distinct from already-shipped SDI") has a
sharp answer: SDI is paid per TICK, so a heavy hit with a long freeze lets a
defender travel far and a one-tick multihit gives them nearly nothing. ASDI is
paid once per HIT whatever the freeze was worth. That is exactly the case SDI
cannot answer.

⛔ **AT THE END, NOT THE START.** The defender has the whole freeze to choose a
direction, and the stick held when it lifts is the one that counts. Paid at the
start it would be indistinguishable from one more SDI tick — which is the second
poison, and it reddens the arm asserting nothing moves DURING the freeze.

⛔ **A LATCH, NOT `hitstop_timer <= dt`.** "Is this the last tick of hitlag?" can
be asked from the timer, but only by a reader that runs BEFORE the decay — and
`decay_reaction_timers` is a separate system whose order against the body step
is not declared anywhere. `BodyCombat::asdi_owed` is answered by two consecutive
steps of ONE function and cannot be silenced by scheduling. A fresh hit that
re-arms the freeze simply banks it again, which is correct: a new hit owes its
own displacement.

⭐ **THE TEST MEASURES A DIFFERENCE, NOT A POSITION.** Every arm runs the same
three steps twice, once declaring the step and once not, so gravity, velocity
and the SDI shift all cancel and what is left can only be this rule. Four arms:
not during the freeze, paid on the first free step, paid ONCE, and a body
declaring nothing is untouched.

⭐ **THE BORROW WIDENING WAS THE REAL SURFACE AREA.** Banking a latch made
`step_body` take `&mut BodyCombat`, and the compiler walked the whole chain —
two integration roads, an enemy update, and a player query column that had to
become `&mut BodyCombat`. Nothing was hidden; the type system enumerated it.

⛔⛔ **AND THE APP GATE COULD NOT SEE ANY OF IT.**
`cargo check -p ambition_app --all-targets` was GREEN while twelve errors sat in
the monolith's own `cfg(test)` files — the gate builds its dependencies as
LIBS, so their test files are never compiled. Six of the twelve were fixtures
passing `&BodyCombat` to a signature that now wants `&mut`, and two were the
monolith's OWN exhaustive destructures of `BodyCombat`.

⇒ ⭐ **WIDENING A SHARED SIGNATURE `&` → `&mut` IS A CROSS-CRATE CHANGE WITH A
BLIND GATE.** The lib build proves nothing about the fixtures that call it. After
widening, run the crate suite of every crate that calls the function, not the
app gate.

⭐ **AND TWO EXHAUSTIVE DESTRUCTURES CAUGHT THE NEW FIELD** — `BodyCombat` makes
every field declare whether the shared decay ticks it and whether `reset()`
clears it. Both answered in the same commit rather than defaulted into.

Wire format v95.

- ✔ **D222 — CLOSED 2026-08-25. THE JAB LOCK: A DOWNED OPPONENT IS A POSITION
  TO READ, NOT A FREE RESET. (opened and closed 2026-08-25)**

A weak launch landing on a body already in knockdown re-pins it where it lies
instead of throwing it — `jab_lock_speed` 320 for Smash, `jab_lock_limit` 3,
and `0.0` disables the rule for every other world.

⭐ **ASKED AT THE ONE LAUNCH GATEWAY**, beside `launch_into_tumble`, and the
comment already sitting there argues the case: whether a body is prone is
model-private maneuver state, the reaction that resolved the knockback does not
hold it, and "asking it at the one gateway every launch already passes through
is what keeps it from being a follow-up call some caller forgets."

⛔ **A SPEED THRESHOLD, NOT A MOVE LIST.** A jab is worth a few hundred px/s and
a smash thousands, so "poke a downed opponent" separates itself from "commit to
a launch" without naming a single move — which is the exemption-list failure the
out-of-shield policy was written to avoid.

⛔⛔ **THE BOUND IS THE MECHANIC.** Unbounded, this is an infinite: weak hit,
re-pin, forever. The dangerous wrong version is not "it never fires" but "it
always fires", and a happy-path test would not notice either. The limit is
poisoned separately and reddens on its own arm.

⭐⭐ **AND THE WIRING ARM WAS PASSING FOR THE WRONG REASON — the poison caught
it, not the test.** Arm 6 originally asserted that a pinned body does not MOVE.
Removing the gateway call entirely left it green: a prone body's velocity is
zeroed by the knockdown itself, so it stays put whether or not the pin fired.
The observable had to be `jab_locks`, which one function writes and only the
gateway can reach. ⇒ **when the state under test already suppresses the thing
you are measuring, position proves nothing** — measure the side effect that only
the new code can produce. Arm 7 then checks the gateway does not swallow
everything: a launch far above the threshold still leaves the floor.

⭐ Also corrected the **Cannot-tech hit property** row to ◐: the architecture it
asks for is already built and by a DERIVED answer — `tumble_untechable` is
stamped at the launch and the tech system owns eligibility. What is missing is
only an authoring OVERRIDE, which wants a customer.

Wire format v96.

- ✔ **D223 — CLOSED 2026-08-25. THE EDGE CANCEL: RECOVERY ENDS WHEN THE GROUND
  IT WAS OWED TO GOES AWAY. (opened and closed 2026-08-25)**

`DeclaredCombatRules::edge_cancel_recovery` — `Some(true)` for Smash, `None`
everywhere else. Land an aerial on a platform lip, slide off, and the landing
lag is over. That is the genre's reward for spacing a landing on purpose.

⭐ **IT IS THE SAME COMMITMENT SEEN FROM THE OTHER SIDE.** Landing lag exists
because an aerial that touches down mid-move should cost you. A body sliding off
a lip is no longer touched down, so there is nothing left for it to be paying —
and charging it anyway freezes a body in MID-AIR, the one state the lag was
never describing.

⛔⛔ **IT COULD NOT LIVE IN `resolve_aerial_landings`, AND NOT FOR STYLE.** The
lag OUTLIVES the playback: charging it cancels the move, so a body paying
recovery has no `MovePlayback` at all and that system's query cannot see it. The
cancel is its own body-generic system over the two components every body has,
chained straight after the landing that charges the lag — so a body that lands
and leaves the ground in one frame is charged and then released, never released
and then charged.

⛔ **A RULE, NOT A PER-MOVE FIELD.** Every move's lag cancels or none does.
Authoring it per move would be an exemption list, and the genre applies it to
the whole cast.

⚠ **AND THE GATE WAS BLIND AGAIN, IN THE SAME CRATE THIS TIME.** A bulk edit put
`edge_cancel_recovery: false` into two `DeclaredCombatRules` literals that want
`Option<bool>`; `cargo check -p ambition_app --all-targets` reported clean
because both live in `ambition_combat`'s own `cfg(test)` files. Second time
today. ⇒ the crate suite, not the gate, is what proves a shared type's change.

- ✔ **D224 — CLOSED 2026-08-25. TWO CROUCH ROWS CLOSED WITHOUT WRITING ANY
  PRODUCTION CODE: ONE STALE, ONE ALREADY TRUE. (opened and closed 2026-08-25)**

⭐⭐ **BOTH WERE ▢ ON WORK THAT WAS ALREADY DONE**, which the goal warns has cost
this project four sessions. Grepping first cost ten minutes and saved building a
mechanic twice.

**`Crouch walk` was stale on BOTH of its claims.** It said `Crouching` never
reaches the locomotion law — it does: `integration.rs` scales the ground speed
CAP by `crouch_speed_frac`, landed 2026-08-24 by the very measurement the row
quotes, guarded by
`a_crouch_costs_speed_only_where_a_ruleset_asks_for_it`. And it implied nobody
had adopted it — Smash declares `crouch_speed_frac: 0.0`, *"in every Smash,
crouching stops you outright"*. So the smaller hurtbox and the shortened launch
are paid for, and the "free defensive win" the row describes no longer exists.

**`Run cancel into crouch` was already true, and MEASURING is what settled it.**
Its stated blocker was the stale claim above. With that gone, the question was
only how FAST — the cap is approached through `approach(.., accel * dt)`, so a
run might have bled off slowly. Measured at 270px/s top speed:

```text
tick  1     2    3    4
     183   97   10   0      stopped on the fourth tick
```

⇒ no production code. ⭐ **BUT IT EARNED A TEST**, because the existing crouch
test starts a body ALREADY crouching: it pins the steady state and says nothing
about the TRANSITION, which is the whole mechanic here. The new guard poisons
against the specific wrong version the kernel comment already names — scaling
the ACCELERATION rather than the cap, which coasts at run speed and still
satisfies the steady-state test because it eventually arrives.

⇒ ⭐ **A ROW WHOSE BLOCKER IS A CLAIM ABOUT THE CODE IS TWO CHECKS, NOT ONE**:
is the claim still true, and if not, is the thing it blocked now free?

- ✔ **D217 — CLOSED 2026-08-25. THE INITIAL DASH SHIPS, AND THE THING THAT
  BLOCKED IT WAS THE DASH EATING KNOCKBACK. (opened and closed 2026-08-25)**

`LocomotionTuning::initial_dash_time` — 14 frames for Smash, `0.0` for every
other world, which therefore keeps ground speed as one continuum and is
byte-identical.

⭐⭐ **ONE EDGE DOES ALL OF IT.** A steer direction that DIFFERS from last
tick's starts the phase: the initial dash, the free reversal dash-dancing needs,
and the foxtrot's re-tap, out of one condition. A HELD direction never
re-triggers, which is what lets the phase expire into a run. ⛔ THE DASH SETS
THE SPEED rather than `approach`ing it — a dash is AT speed on its first tick.
Both poisoned and red.

⛔⛔ **FINDING ONE: IT COST THE FORWARD SMASH.** `running` is a SPEED test, and a
dash is at full speed immediately, so a body became "running" the tick it
pressed a direction; the move selector reads that fact, so forward + attack came
out as `player_robot_dash_attack` instead of `smash_forward`. ⇒ **A DASH IS NOT
A RUN** — the genre's own distinction — so `running` now excludes the dash
window, which is exactly what `run_commit_frac` was always naming.

⛔⛔ **FINDING TWO: IT DELETED KNOCKBACK, AND THAT WAS THE MISSING KO.** A
grounded fighter launched while holding a direction had its launch REPLACED by
dash speed:

```text
holding neutral   no dash 1187   dash 1187      (untouched)
holding toward    no dash 1313   dash  270      (replaced)
holding away      no dash 1313   dash    0      (erased)
```

Nobody could be knocked off the stage, so a one-stock match never ended.
⭐ **SAME CLASS AS THE GROUND ROLL'S SHED** — a maneuver reaching into a shared
velocity it does not own — and the same asymmetry answers it: a dash may only
SPEED YOU UP, so a body already travelling faster is carrying somebody else's
velocity and the dash leaves it alone.

⭐⭐⭐ **AND THE INSTRUMENT CHAIN IS THE REAL LESSON, BECAUSE STEP TWO KILLED
STEP ONE.**

```text
1. kernel probe    a flipping body travels 675px where a steady one does 1339
                   ⇒ "CPUs oscillate and never close distance"      PLAUSIBLE
2. match instrument real fighters flip once every ~21 steered ticks,
                   which costs ~7% of travel                        REFUTED (1)
3. kernel probe    a launch held into becomes 270; held away, 0     FOUND IT
```

⇒ **THE FIRST STORY WAS COHERENT, MEASURED, AND WRONG.** Had I shipped it as the
explanation — or tuned the window against it — the real defect would have stayed
in. ⭐ The refuting step cost one column in `match_report`, which is the tool
already built for per-tick match tallies; that column (`steer flips`, with a
`steer held` denominator so "rarely flips" cannot mean "rarely moves") is the
first direct read this repo has on what fighter brains do with the stick, and it
stays.

⚠ **`initial_dash_time` IS ONE NUMBER IN ONE PLACE.** 14 frames is taken from
the genre, not measured against this game; `0.0` restores the previous ground
feel exactly.

⇒ **FOXTROT AND DASH DANCE CLOSED THE SAME DAY, WITH NO PRODUCTION CODE.**
Both were predicted to fall out of the entry rule, and driving them end to end
is what turned that into a fact:

```text
foxtrot      tap → let the phase expire → neutral → tap again   re-arms ✔
dash dance   alternate directions: >=4 re-arms in 24 ticks,
             drift under a quarter of a run speed               a dance ✔
```

⭐ The neutral tick is what makes the re-tap work: `prev_steer_dir` resets to
zero, so the next press is a CHANGE again. Poisoned by narrowing the rule to
reversals only, which reddens the re-tap arm exactly. ⇒ three parity rows from
one condition, which is what "one root, eight dependents" was supposed to mean.

- ✔ **D225 — CLOSED 2026-08-25. THE TURNAROUND: THE HALF THAT MAKES THE DASH'S
  FREE REVERSAL WORTH ANYTHING. (opened and closed 2026-08-25)**

`LocomotionTuning::turnaround_time` — 3 frames for Smash, `0.0` everywhere
else. Reversing out of a COMMITTED run delays the facing flip; reversing inside
the dash window stays free.

⭐⭐ **THE PAIR IS THE MECHANIC.** A game where every reversal is free has no
ground game; one where every reversal is slow has no dash dance. D217 shipped
only the free half, which is why this is its completion rather than a new row.

⛔ **IT DELAYS THE FACING FLIP AND NOTHING ELSE.** What the body does with its
velocity meanwhile is the ordinary run law's business — inventing a skid here
would be a second opinion about ground speed.

⛔⛔ **ARM ON THE REQUEST EDGE, NOT ON THE CONDITION.** A body still running and
still asking to reverse satisfies "should be turning" EVERY tick, so a
condition-armed phase re-arms the instant it expires and the facing never flips
at all — the body turns forever. `prev_steer_dir` is the same edge the dash is
entered on. That version is poison T1 and reddens the "never completed" arm.

⚠ **3 FRAMES IS WHAT THE PROVING GROUND TOLERATES, NOT A FEEL MEASUREMENT.** At
7 frames `smash_it` lost two premise guards to one symptom: seat 0 stopped ever
being knocked off the stage in a 3600-tick match while seat 1 went off 57 times
— a CPU matchup that traded became one-sided.

⭐ **AND THIS TIME IT WAS NOT A CORRUPTION.** After the dash's knockback bug I
went straight at that class first: a body launched MID-turnaround keeps its
launch (1400 vs 1313, the gap being the dash correctly not re-arming). Then the
discriminating test — 3 frames green, 7 frames red — said it SCALES, which with
corruption ruled out means tuning rather than structure. ⇒ the opposite verdict
from the dash, where length did NOT matter and that pointed at the real defect.
Same two experiments, different answer, and the order is what made the answer
trustworthy.

⇒ **PIVOT GRAB AND REVERSE AERIAL RUSH ARE UNBLOCKED**: both were written as
"after the turnaround fact exists", and it does.

⚠ **AND THE MONOLITH FIXTURE BROKE FOR THE FOURTH TIME TODAY.** Every field
added to a shared struct needs `prepared_match/tests.rs`, and
`cargo check -p ambition_app --all-targets` cannot see it — the gate builds
dependencies as LIBS. ⇒ when touching `MatchBody`/`MovementTuning`, run the
monolith crate suite BEFORE believing the gate.

- ✔ **D226 — CLOSED 2026-08-25. THE REVERSE AERIAL RUSH EMERGED, ONCE THE
  TURNAROUND STOPPED EVAPORATING ON TAKEOFF. (opened and closed 2026-08-25)**

The row said RAR *should emerge* from turnaround → jump → back-air and forbade
an RAR state. ⭐ MEASURING THAT SEQUENCE FIRST is what found the gap, because it
produced the exact opposite of a rush:

```text
running right    facing +1, vel 270
after the tap    turning, facing +1, vel 183
jump + 6 ticks   airborne, facing STILL +1, vel_x -178
```

⇒ **AN AIRBORNE BODY MAY NOT TURN** (`can_turn` is grounded-or-flying), so a
fighter who jumped mid-turnaround carried its OLD facing forever: the phase was
being ABANDONED, not resolved. One rule fixes it — **a turnaround is a ground
phase and leaving the floor FINISHES it**, so the body takes into the air the
facing it was already paying for. The flip is the phase's own, not the stick's,
so a player who let go on the way up still gets what they bought.

⚠ **AND THE ROW'S OTHER HALF IS NOT TRUE OF THIS ENGINE.** "Momentum carries
you" is the genre's version; here the air stop assist halts a released stick
dead. The rush is bought by holding FORWARD after the jump — the reversed facing
sticks precisely BECAUSE airborne bodies cannot turn — rather than by drift.
That is written into the test, since it is a real difference from the game this
is modelled on and the next person will expect the genre's version.

⇒ Four parity rows now come out of the one direction-change edge: the initial
dash, the foxtrot, the dash dance, and — through the turnaround — this.

- ✔ **D227 — CLOSED 2026-08-25. THE PIVOT GRAB, WITH NO MOVE OF ITS OWN.
  (opened and closed 2026-08-25)**

A move thrown while `turning_around` resolves its aim against the FLIPPED
facing, so the existing forward grab points the other way. The row promised
"capture itself needs no new mechanic" and that held exactly.

⭐ **ONE RULE, STATED ONCE, NOW COVERS BOTH**: a turnaround is finished by
whatever you commit to out of it. Jumping resolves it in the movement kernel
(the reverse aerial rush); acting resolves its DIRECTION in
`resolve_attack_gestures` (this).

⛔⛔ **THE FIRST IMPLEMENTATION WAS IN THE WRONG PLACE AND READ PERFECTLY.** I
put the flip at the move SELECTOR, beside `attack_dir_from_axis(...)`. It
compiled, it looked right, and it changed NOTHING — the direction is already
decided by the time that code runs, because `resolve_attack_gestures` folds
facing into the aim earlier in the same chain.

⇒ ⭐⭐ **ONLY THE WIRING TEST COULD HAVE CAUGHT THAT.** The pure rule
(`attack_dir_is_relative_to_facing`) passes either way, and a test written
against it would have "proved" a mechanic that does not exist. The wiring test's
BASELINE arm is what localised it: leftward-press-while-facing-right still read
`Back` correctly, so the fault was the turnaround being unread rather than the
direction maths being wrong. ⇒ when a fact must reach a decision, test that the
DECISION changed, never that the function computing it is correct.

⚠ **AND TWO BUILDS DIED TO THE FOREGROUND TIMEOUT TODAY.**
`cargo check -p ambition_app --all-targets` now exceeds two minutes on its own
under contention, and a SIGTERM mid-compile surfaces as
`failed to parse process output: rustc ...` — which reads like a toolchain fault
and is not one. ⇒ never chain work AFTER the gate in one foreground command.

- ✔ **D228 — CLOSED 2026-08-25. A SHIELDING FIGHTER GLIDED ACROSS THE STAGE,
  AND "RUN CANCEL INTO SHIELD" WAS THE BUG REPORT. (opened and closed
  2026-08-25)**

⛔⛔ **MEASURED FIRST, AND THE ROW WAS A LIVE DEFECT.** A body running at
270px/s that raises its guard was still doing 270 SIXTY TICKS LATER, guard up
throughout. The whole ground-speed block — friction included — sits inside
`if ctx.can_move_horizontal`, which a raised guard turns off. **"May not steer"
is not "may not stop."** Now 270 → 143 → 17 → 0: planted in three frames.

⚠ **AND THE FIRST MEASUREMENT CORRECTED THE ROW ITSELF.** Shield + DIRECTION
made the body speed UP to 530 — that is the roll (Jon, 2026-08-23: a direction
behind a guard chooses an evade). The cancel is shield with a NEUTRAL stick, and
a probe that assumed otherwise would have "found" the mechanic already working.

⛔⛔ **THIRD TIME TODAY FOR ONE SHAPE — A MANEUVER REACHING INTO A VELOCITY IT
DOES NOT OWN.**

```text
ground roll     shed its push at expiry     bound: speed still EXACTLY the push
initial dash    set speed on frame one      bound: may only SPEED YOU UP
shield brake    braked a planted body       bound: only speed it could WALK to
```

Each time the failure mode is deleting somebody's knockback, and each time the
answer is to bound the effect by what the maneuver itself put in. ⭐ THE GUARD
WRITTEN FOR THE ROLL THIS MORNING CAUGHT THIS ONE:
`a_ground_roll_ends_stopped_but_never_eats_a_launch` reddened the moment the
brake existed, because a launched body holding its guard is grounded, not
evading, and therefore looked "planted".

⚠ **AND TWO OF MY OWN TEST ARMS COULD NOT SEE THE POISON, BOTH BY SAMPLING THE
WRONG MOMENT.** The roll arm read velocity on tick ONE — a single frame of
friction barely dents 530px/s, so the bad version passed. The launch arm read
`peak`, which is the launch's own first tick and survives any braking after it.
⇒ **WHEN THE EFFECT IS GRADUAL, AN INSTANTANEOUS SAMPLE CANNOT SEE IT**: measure
the distance covered, not the speed at a moment.

- ✔ **D229 — CLOSED 2026-08-25. THE TEETER, AND A POISON THAT CORRECTED MY OWN
  EXPLANATION. (opened and closed 2026-08-25)**

`teeter_margin` publishes `BodyMotionFacts::teetering`: supported where you
stand, but your LEADING FOOT is over air. A quarter of the footprint for Smash,
`0.0` everywhere else. ⛔ A FACT, NOT A RULE — collision, speed and refusals are
untouched, which is what the row asked for.

⛔⛔ **SUPPORT IS ANY LATERAL OVERLAP** (`perpendicular_overlap`), so a body
hanging 14px past a platform with 15px of half-width is still FULLY supported. A
first attempt leaned the whole body by `half_width * margin` and found no edge
at any position, because that shift never lifted the probe's trailing edge clear
of the platform.

⭐⭐ **AND THEN THE POISON CORRECTED THE FIX'S OWN STORY.** I had written that the
answer was a STRIP under the foot rather than a SHIFTED body. Poisoning it back
to a shifted body PASSED — because both put the probe's trailing edge at
`min + cut`, and since any overlap counts the far side never matters. The two
are equivalent; the real variable was the AMOUNT, not the shape. The comment now
says that, and the poison is a genuine alternative instead — CENTRE-over-air,
which a reasonable person might have written and which reddens the lip arm.

⇒ ⭐ **A POISON THAT PASSES IS NOT ALWAYS A WEAK TEST — SOMETIMES IT MEANS YOUR
TWO "DIFFERENT" IMPLEMENTATIONS ARE THE SAME PROGRAM.** Check that before
strengthening the assertion, or you will write an arm defending a distinction
that does not exist.

⭐ Four arms, and the third is the one a careless probe fails: the SAME body on
the SAME spot facing INWARD is not teetering, because the lean goes the other
way and there is floor under it. That separates "about to leave an edge" from
"near an edge". Measured: brink begins at x=492 for a 15px half-width on a
platform ending at 500.

⇒ **§4 GROUND MOVEMENT AND NEUTRAL IS NOW COMPLETE**: initial dash, foxtrot,
dash dance, turnaround, reverse aerial rush, pivot grab, run cancel into crouch,
run cancel into shield, teeter. Wire format v99.

- ✔ **D230 — CLOSED 2026-08-25. B-REVERSE AND WAVEBOUNCE ARE ONE RULE WITH TWO
  SETTINGS. (opened and closed 2026-08-25)**

`special_turn` turns a fighter around when a special is pressed BACKWARD;
`special_turn_reverses_drift` makes that same turn take the drift with it. Both
`None` everywhere but Smash. ⛔ The row forbade a fighter-specific velocity
hack by name, and one special-start policy with two knobs is what avoids it: a
game that wants the turn without the launch-cancel is not forced to take both.

⛔ **MOVE SELECTION IS UNTOUCHED.** The move was already chosen from the same
`AttackDir::Back`; this is where the BODY answers. Keeping the two apart is what
the row means by "keep move selection and momentum rule separate".

⛔ **THE DRIFT, NOT THE WHOLE VELOCITY.** Reversing `vel` outright would flip a
launch the fighter is riding — the fourth appearance of that shape today, and
the first one I wrote correctly on the first try.

⭐⭐ **AND THE POISON FOUND A HOLE IN MY OWN TEST.** Deleting the
`AttackDir::Back` gate entirely left all four arms GREEN, because every one of
them pressed Back: a rule that turned you on ANY special would have passed. Arm
5 presses FORWARD with both knobs on and asserts no turn.

⇒ ⭐ **WHEN A RULE IS GATED ON AN INPUT, AT LEAST ONE ARM MUST SUPPLY A
DIFFERENT INPUT.** Otherwise the gate is untested and the suite is asserting
"the rule fires", never "the rule fires ONLY THEN". Same family as the pivot
grab's baseline arm earlier today.

⭐ Arm 4 earns its place too: the drift knob ALONE does nothing, because there
is no turn to strengthen — without it a reader could take it for a second
mechanic rather than a modifier.

- ✔ **D231 — CLOSED 2026-08-25. FAST-FALL AFTER A LAUNCH WAS ALREADY TRUE, AND
  PROVING IT HONESTLY WAS THE WORK. (opened and closed 2026-08-25)**

`tick_knockdown` strips control for the tumble's duration and hands it back
whole, so fast-fall is REFUSED inside the tumble and returns the moment it ends
(863 → 931 px/s). No production code. Both halves are guarded, because either
alone is a different game: refused forever is a fighter who cannot come down,
permitted always is a launch you can cancel.

⛔⛔ **MY FIRST REFUSAL ARM PASSED FOR THE WRONG REASON.** It pressed three ticks
into the tumble, while the body was still RISING — and fast-fall on a rising
body does nothing whether or not control is suppressed. The arm was green with
the suppression intact AND with it poisoned away. Launching FLAT, so gravity has
the body descending well inside the window, is what made it discriminate.

⇒ ⭐⭐ **AN ARM THAT ASSERTS A REFUSAL MUST BE TAKEN IN A STATE WHERE PERMISSION
WOULD VISIBLY DO SOMETHING.** Third time today a test arm sampled the wrong
moment — the roll's first tick, the launch's peak, and now the top of an arc.

⭐ **AND WHEN A POISON WILL NOT REDDEN, COUNT INSTEAD OF GUESSING.** I burned
three attempts hunting the line that swallows the press. Instrumenting the gate
to print every press that REACHED integration showed exactly ONE out of two —
which proved the mechanism before its address was known. Poisoning
`tick_knockdown` at its entry then reddened it immediately.

- ✔ **D232 — CLOSED 2026-08-25. THE LEDGE IS NO LONGER SAFE ON CONTACT.
  (opened and closed 2026-08-25)**

`LEDGE_GRAB_VULNERABLE_TIME` — two frames of exposure at the catch, before the
earned intangibility begins. ⭐⭐ WITHOUT IT THE EDGE IS AN UNCONDITIONAL SAFE
POINT: reach it and you are untouchable, so an opponent covering the ledge has
nothing to hit and the recovery is decided entirely off-stage.

⛔ **IT DELAYS THE WINDOW, IT DOES NOT SPEND IT.** The earned invuln is HELD at
full value while the exposure runs. Ticking both clocks would quietly shorten
every earned window by the vulnerability, so a fighter who bought 0.5s of edge
with a long recovery would silently get less than it earned.

⛔ **A MODULE CONSTANT, AND ONLY BECAUSE THE MODULE SAYS SO.** `LEDGE_HANG_LIMIT`
documents the convention for the whole ledge vocabulary — "a lone authored field
would be the odd one out". Following an existing stated convention beats
inventing a declaration seam for one number. Only bodies with the `ledge_grab`
ability can reach it.

⭐ **AN EXISTING TEST WENT RED, AND IT WAS THE RIGHT KIND OF RED.**
`a_ledge_grab_is_intangible_without_reading_as_a_dodge_roll` asserted
intangibility on the very frame of the catch. Its real claim — untouchable for
its OWN reason rather than the dodge roll's — is untouched; only the timing
moved. ⇒ REPAIRED, not weakened: it now asserts the catch frame IS exposed, then
that the window is the grab's own once the exposure passes, so the test gained a
guarantee instead of losing one. The poison that ignores the exposure reddens
that new arm exactly.

Wire format v100.

- ✔ **D233 — CLOSED 2026-08-25. EDGEHOG VS TRUMP IS ONE COMPARISON.
  (opened and closed 2026-08-25)**

`ledge_occupancy` — `Trump` (default, today's behaviour) keeps the NEWEST grab;
`Hog` keeps the body that got there first. Whoever sorts first keeps the edge,
so the two generations' rules are the same authority read in opposite
directions.

⛔ **NO SECOND RULE ABOUT WHO MAY GRAB.** A hog that refused the grab outright
would be a second ledge authority, which the row rules out by name. The loser is
knocked off by the same path with the same pop either way, and the `SimId`
tiebreak stays ascending in both so a same-tick contest is still deterministic.

⭐ **THE TEST IS THE SAME FIXTURE TWICE, ONE DECLARED RULE APART** — identical
camper, identical newcomer, identical edge. That is what makes this a POLICY
rather than two mechanics. The Trump arm declares `Trump` EXPLICITLY rather than
leaning on the neighbouring test that declares nothing, because "the rule works"
and "the default is unchanged" are different claims; the third arm covers the
undeclared case and poisoning the default reddens only that one.

⚠ **AND A LESS-INDENTED PATTERN IS A SUBSTRING OF A MORE-INDENTED ONE.** A
replace on `            ledge_trump_pop: None,` also matched inside the 16-space
copy and duplicated a field. ⇒ for repeated struct literals, edit BY LINE, not
by substring — three of today's compile errors came from this one habit.

- ✔ **D234 — CLOSED 2026-08-25. THE DOUBLE-JUMP CANCEL, AND THE BEST VERSION
  YET OF A BOUND I HAVE WRITTEN FIVE TIMES TODAY. (opened and closed
  2026-08-25)**

An aerial thrown out of an air jump kills the rest of that jump's rise —
`double_jump_cancel`, Smash only. It turns a double jump from a commitment into
an approach: rise, throw, land where you chose.

⭐⭐ **THE OWNERSHIP BOUND MOVED INTO THE PUBLISHER.** Zeroing a climb is the
exact shape that has deleted knockback three times today, so it needs the usual
bound — cancel only what the jump itself could have produced. This time
`BodyMotionFacts::air_jump_rising` carries it: the fact means "rising on a jump
I OWN", and a body riding a launch simply reports `false`. ⇒ THE CONSUMER NEEDS
NO JUMP TUNING AT ALL, and no consumer can form a second opinion about whose
velocity it is. Strictly better than the four call-site bounds before it.

⛔⛔ **AND A POISON FOUND THE GAP THAT FRAMING CREATES.** Dropping the bound from
the fact left the CORE suite green — the combat test injects the fact directly,
so it never exercises the derivation. The bound guarding against knockback loss
was itself unguarded.

⇒ ⭐ **MOVING A BOUND INTO A PUBLISHED FACT MOVES THE TEST THAT GUARDS IT.** The
consumer's test cannot see it any more. A core arm now launches a body at four
times its jump speed and asserts the fact refuses the climb, with a non-vacuity
arm confirming the fixture is really rising.

Wire format v101.

- ✔ **D235 — CLOSED 2026-08-25. JON'S ROLL PLAYTEST, AND THE ONE TIMER THAT
  MEANT FOUR THINGS. (opened and closed 2026-08-25, from a GPT 5.6 review)**

Jon: *"roll distance is input/history-dependent AFTER the roll has already
begun"*. Reproduced in a unit test, with numbers:

```text
same roll, guard HELD        106px
same roll, guard RELEASED     33px
fresh roll                   106px over 13 ticks
roll spammed 3x before        27px over  4 ticks
```

⭐⭐ **TWO CAUSES, ONE ROOT: `dodge_roll_timer` MEANT FOUR DIFFERENT THINGS.**
Animation (`anim` reads `dodge_rolling`), i-frames (`evading()` read the same
fact), movement ownership, and commitment (`evade_cancel_tail`). `spend_evade`
returned the STALED window into it, so a spammed roll got SHORTER instead of
unsafe — while its own header said *"STALING WEARS THE I-FRAMES, NOT THE
DISTANCE"*. ⛔ The comment stated the contract; the code never implemented it.

⇒ SPLIT: `dodge_roll_timer` keeps its name as the AUTHORED MANEUVER clock
(travel, endlag, animation, commitment) and the new `evade_invuln_timer` carries
the staled i-frames. ⛔ The review proposed the opposite assignment — facts on
the invuln clock — which would have ended the roll ANIMATION early on a stale
roll. Reading the consumer decided it.

And an accepted evade now owns its horizontal travel: the gate was
`shield_held && on_ground`, so letting go of the guard handed the ordinary
friction law a velocity the roll had authored.

⛔⛔ **MY FIRST VERSION OF THE TEST PASSED AGAINST THE BUG** — the fixture spawned
above the floor, so the roll's first EIGHT ticks were an AIR roll and measured a
different law entirely. Printing `on_ground` per tick found it; the gate had been
working from tick 8 the whole time.

Also closed here: the WAVEBOUNCE reflected `kin.vel.x` (world X) though its own
comment said *"only the run axis turns"* — now `body_frame.side`, with a rotated
gravity arm; and an interrupted LEDGE CATCH revoked nothing, so a fighter hit
during its two exposed frames went intangible in MIDAIR after the exposure ran
out. Only the UNVESTED grant is revoked — a window already open is what makes a
getup safe, and both directions are poisoned.

Wire format v102.

- ✔ **D236 — CLOSED 2026-08-25. THE KEYBOARD DROVE PLAYER ONE NO MATTER WHO
  CLAIMED WHAT. (opened and closed 2026-08-25, from a GPT 5.6 review)**

Character select recorded the truth all along — `LocalChannelPlan` has said
"channel 0 = Pad(0), channel 1 = Keyboard" since the cards were claimed — and the
BINDING layer below it did not read it. Every non-primary seat was built
`gamepad_only()` and the primary always got a keyboard-and-pad preset. ⇒ a couch
where a PAD took card one and the KEYBOARD took card two gave the keyboard player
NO controls, while the keyboard went on driving player one as a second pad.

⭐⭐ **DEVICE ELIGIBILITY WAS FUSED INTO THE BINDING BASE**: `Preset(id)` MEANT
"keyboard and pad" and `GamepadOnly` MEANT "an extra seat". That shape cannot
express a composition character select produces happily. ⇒ `BindingBase` is
DELETED for an orthogonal `BindingSources` scope, and `KeyboardPreset::
gamepad_only_map()` with it — both builders were the same two halves composed, so
one `map_for(sources)` replaces the pair. The scope also bounds OVERRIDES, since
`apply_override` inserts a binding when no same-class one exists: replaying a
keyboard remap onto a pad seat would hand the alias straight back.

⛔⛔ **AND SETTING IT AT SPAWN WAS NOT ENOUGH — THE SECOND HALF OF THE BUG, which
the review did not have.** Seats are spawned while the LOBBY is up, before any
roster declares anything, so they all start on the no-plan default and the
match's frozen plan never reaches them. The keyboard player kept the gamepad-only
seat the lobby gave them and moved 0.00px. ⇒ **A FACT SET AT CONSTRUCTION IS NOT
A FACT THAT FOLLOWS ITS AUTHORITY**: the plan is decided AFTER the entities
exist. Every seat's scope now re-derives, primary included.

⛔ `SeatActiveDevices` answered the same question independently and defaulted the
keyboard to PRIMARY — wrong glyphs and wrong per-pad filtering even once control
was fixed. It now reads the frozen plan, and `None` there means NOBODY: a keypress
during an all-pad match belongs to no fighter's seat.

⛔⛔ **AND A THIRD: THE SCOPE WAS APPLIED BUT NEVER RETRACTED.** Guarding the
refresh with `if roster.is_some()` reads as harmless, and means the value is
never undone when its authority departs — quitting a match whose primary seat was
on the keyboard returned to the launcher with NO PAD AT ALL. The loop now runs
unconditionally and `sources_for_channel` answers "no plan" with the pre-match
default, so ONE path both applies and undoes.

⇒ ⭐⭐ **ONE RULE FOR ALL THREE**: a derived value is RE-DERIVED every tick from an
authority allowed to say "nothing declared" — not written once when a decision
arrives, and not left standing when it departs.

⚠ TWO MORE THINGS I GOT WRONG AND THE TESTS CAUGHT: returning `Unified` for every
unclaimed seat put one keyboard on all the LOBBY seats — the same alias, one
screen earlier, caught by the EXISTING forward test in a minute. And the two-pad
arm overclaimed: a poison on the no-plan branch PASSED, because in an all-pad
match every channel IS named, so that branch is dead along this fixture's path.

- ✔ **D237 — CLOSED 2026-08-26. THE 24-HOUR DEEP REVIEW (GPT 5.6, snapshot
  `cab773ccf`) IS FULLY CONSUMED.** ⚠ its old headline — *"seven closed, six
  open, one refused"* — was the count from the day it opened and had been stale
  for a day: the body now records every finding closed, the refusal OVERTURNED
  (13), one item (12) PARKED with a named reopen condition, and one (16) that was
  always guidance rather than work. ⛔ **the two that still read as open were
  both shipped under D241 and never marked here** — (7)'s input-order recogniser
  and the wavebounce half — which is the four-session failure mode this file
  warns about, caught by re-reading rather than by trusting the header.
  (opened 2026-08-25; closed 2026-08-26)**

⭐ CLOSED 2026-08-25, each measured → built → poisoned → verified: (1) match input
device authority [D236]; (2) roll ownership + the evade timer split [D235]; (3)
authored ZERO-DAMAGE WINDBOXES — the hitbox layer preserved the authored `0` and
`damage_apply.rs`'s `damage.max(1)` undid it one seam later on the shared road,
so a repeating gust took a point of health per pulse and would eventually kill
what it was only meant to push; (9) wavebounce reversing world X [D235]; (10) the
unvested ledge grant [D235]; (14) BARK DETERMINISM — the draw was salted with
`Entity::to_bits()`, which is ALLOCATOR HISTORY: two peers that spawned one cast
in a different order disagree, and ROLLBACK HIDES IT because a rewind reuses the
same ids. Now `sim_salt_for_name(SimId)`, with an arm proving two independently
built identities for one fighter agree and a different fighter still differs.
Plus the parity row PIVOT SMASH, which was already true — the pivot went in
where facing is resolved, so every attack family inherited it.

⭐⭐ **(13) CLOSED 2026-08-25 — AND THE REFUSAL BELOW WAS WRONG, WHICH IS THE
INTERESTING PART.** The review was right that `kin.vel.x = outward.signum() * pop`
is world X. It was refused on the claim that its INPUT was world-X too — and that
claim was read off the NAME. `LedgeContact::wall_normal_x` is a body-LOCAL side
sign: the producer computes `world_normal.dot(frame.side).signum()`, and
`probe_ledge_grab_in_frame`'s own doc says *"`wall_normal_x` is historical naming:
it is now the side-face normal expressed in the controlled body's local side
axis"*. ⇒ it WAS the one-line projection the review proposed, and the refusal cost
the fix a day.

⛔⛔ **THE LESSON IS THE FIELD NAME.** A stale NAME is a claim about behaviour
exactly like a stale comment, and it is worse: a comment is read sceptically and a
type signature is believed. Two greps — the producer and the doc — would have
settled it. ⇒ on any "the input is in the wrong space" refusal, READ THE PRODUCER.

⚠ THE ORIGINAL REFUSAL, kept as the record of how it read at the time:

⚠ **AND (11) COST A FIXTURE, WHICH IS THE MOST USEFUL THING IN THIS ROW.**
Unifying the threshold changed exactly FOUR arming decisions across a 3600-tick
match — and that moved which fighter is knocked out first, reddening
`every_authored_route_gets_pressed`. ⛔⛔ TWO PROBES SAID IT COULD NOT: CPUs steer
at exactly `signum()`, and a probe printing where the two COMPARISONS differ
showed disagreements only at `x=0.000`, where the gate is shut. Printing the
whole ARMING PREDICATE instead found the real values — 0.342, −0.258, −0.376,
−0.158, off the survival/DI stick rather than the movement verb. ⇒ **PROBE THE
DECISION, NOT AN INPUT TO THE DECISION.**

⇒ The fixture paired George — the only fighter with an authored route home —
against one without, so it silently required GEORGE to be the one knocked out.
Its own comment records two earlier behaviour changes that flipped it, each
answered by doubling the window, and calls that *"a hand-kept ledger"*. Now a
MIRROR MATCH: whoever loses carries the route, and the chaotic variable is gone
without widening anything or weakening the claim.

⭐ **ALSO CLOSED 2026-08-25 — (6) THE LIVE MATCH CLOCK COUNTED FRAMES, NOT
GAMEPLAY.** It handled `sim_dt == 0` correctly, and passing that BINARY case is
what made it look finished. Time scaling is not binary: every impact hitstop
RAMPS it (`0.917`, `0.750`, `0.983` are all real values from one match trace), so
on those ticks every fighter timer advanced by a fraction while this clock
advanced a whole tick — the 8-minute timer and the item cadence both ran FAST
against the gameplay they were timing. Now microseconds of SCALED time in the
same single `u64` (snapshot shape unchanged; integers, because a float
accumulator is snapshot state whose replay depends on summation order). Wire
format v103. ⇒ second finding today of the shape **"the binary case was handled,
so nobody checked the continuous one"** — the other was `evading()`.

⭐⭐ **AND (8) + HALF OF (7), CLOSED 2026-08-25.** (8) THE SPECIAL-TURN MUTATED
THE BODY DURING PROPOSAL: it flipped `facing` and reversed drift while still
RESOLVING which move the press would start — and that resolution returns `None`
for a fighter with no authored special in that direction. So Back+Special turned
the body and threw nothing, and a BUFFERED press turned it again every tick (the
test holds four ticks for exactly that reason). Now proposed where decided,
applied at the two `start_move` sites after every refusal gate — and BEFORE the
start impulse, because the turn reverses the drift the fighter ARRIVED with and
running it after would reverse the move's own impulse too.

⇒ ⭐ THIRD INSTANCE TODAY OF ONE SHAPE: an effect applied where a decision is
COMPUTED rather than where it is COMMITTED. The others were the input scope set
at spawn instead of re-derived [D236], and the same scope applied but never
retracted [D236].

(7) HALF: the two declared bits were GATED — `special_turn_reverses_drift` only
acted with `special_turn` on — which made exactly one real technique
undeclarable. Ungated, one rule now yields four outcomes: ordinary special /
turnaround-B / B-reverse / **WAVEBOUNCE** (momentum turns, facing does not). ⛔ an
existing arm asserted the drift knob alone "does nothing, because there is no
turn to strengthen" — that sentence WAS the assumption, so it was updated, not
worked around. ⚠ THE RECOGNISER WAS THE REMAINING HALF and read as *"honestly absent"* here.

✔✔ **IT SHIPPED THE SAME DAY AND THIS PARAGRAPH WENT STALE — verified at HEAD
2026-08-26.** `AttackGestureState::special_turn_window` and
`apply_special_turn_flicks` are live in `ambition_combat::moveset`; the four
outcomes fall out of two toggles (a back BEFORE the press flips facing; a back
flick AFTER it also reverses drift; both = wavebounce), and the window is the
ruleset's own `flick_window_ticks` rather than a new knob. Wire v111. See
**D241**, which records it. ⇒ nothing here is open; the sentence above is kept
because *"a game DECLARES which technique"* was the framing the recogniser
replaced, and knowing that it was replaced is the useful part.

◐ **(4)/(5) — BUILT, MEASURED, REVERTED, AND THE REVERT IS THE FINDING.
2026-08-25.**

⭐⭐ **(5) IS CONFIRMED IN PRODUCTION, not by reading the schedule.** A real
hostile enemy next to the real player, real schedules, 300 frames: **the player
served 0.036s of its own hitlag, no other body served any, and the match froze
for ZERO frames.** The fixture is committed and IGNORED —
`ambition_app/tests/a_hit_on_the_player_freezes_the_match.rs` — because it goes
GREEN with the fix and the fix is not landed.

⭐ **THE ARCHITECTURE IS SETTLED AND IT WORKED.** `ResolvedBodyHit { victim,
hitlag_seconds }` in `ambition_combat::hitbox`, registered in
`SimCoreResourcesPlugin` (⛔ NOT in a schedule plugin — its writers are on the
PLAYER road and the ACTOR road, which different compositions install separately;
registering beside one panics the other, a crash this repo has already shipped
once). Published by `publish_resolved_hit` beside the REACTION, never beside the
resolution — ⛔ publishing beside `resolve_body_hit` reports `0` for every hit,
because the reaction is what charges the hitlag, and that was measured too.
`request_impact_hitstop_on_resolved_hits` then reads the resolver's own answer
and has no opinion about which road resolved the hit or on which frame.

⛔⛔ **AND THE FIRST ATTEMPT FROZE THE WORLD FOREVER — THE DIAGNOSIS IS WORTH
MORE THAN THE FIX.** Measured on `smash_in_the_host`: one victim, SEVENTEEN
consecutive resolutions across twenty-three ticks, identical hitlag, the world
alternating frozen/moving with `until_tick` tracking `tick + 2` for 180 ticks,
and three smash fixtures never reaching the gait they wait for. Probing the
SOURCE settled it: every one of them was **`HitSource::Contact`** — one fighter
leaning on another, one damage a tick.

⇒ **`ResolvedBodyHit` IS A BROADER CHANNEL THAN `LandedBodyHit`, and that is
its price.** `LandedBodyHit` comes off the hitbox sweep, so it is strikes and
shots BY CONSTRUCTION; this comes off the RESOLVER, which also serves contact
attrition, hazards and the blast zone. A consumer that wants a CONNECT must now
say so — the message carries `source`, and the freeze arms on
`Melee | Projectile | Pogo` only. Standing in lava is not a hit connecting.

⛔ A REFRACTORY PERIOD WAS TRIED AND DROPPED. "Refuse to arm while already
frozen" halves the duty cycle and no more, and it contradicts a documented
mechanic the module's own test states (overlapping connects extend). The source
filter removes the cause instead.

⭐⭐ **AND (5) IS PROVEN END TO END — THE SPACING WAS THE FIXTURE'S BUG.** At 60px
the automaton's contact footprint SHOVES the player away before anything it aims
can reach: 900 frames produced five hits, every one `Contact`. At **26px** its
glider connects, and a `Projectile` IS a connect — so the bout freezes, and it
can only have frozen from a connect, because the contact attrition hitting the
same player arms nothing. Poisoning the PLAYER ROAD's publish reddens it. The
fixture is live (`a_hit_on_the_player_freezes_the_match`), not ignored.

⇒ ⚠ **A FIXTURE'S SPACING IS A PARAMETER, and "the enemy never lands one" was a
conclusion about the number rather than about the game.**

✔✔ **CLOSED (4) AND (5) — the split landed and its first customer is the freeze.
Re-verified at HEAD 2026-08-26.** `ResolvedBodyHit { victim, hitlag_seconds,
source }` is published beside `publish_reaction` (the reaction is what CHARGES
the hitlag, which is why publishing it beside the RESOLUTION reported zero for
every hit), `request_impact_hitstop_on_resolved_hits` replaced the landed-hit
reader in `combat_schedule`, and `a_hit_on_the_player_freezes_the_match` is the
arm the old file could not have. ⛔ **and the source had to be CARRIED**: the
first version froze the world permanently — 185 resolutions holding `tick+2` for
180 ticks — and printing `event.source` said why, every one was
`HitSource::Contact`. The consumer filters to CONNECTS (`Melee | Projectile |
Pogo`). ⇒ probe the SOURCE, not the count.

~~▢ STILL OPEN: **(4) `LandedBodyHit` means OVERLAP and
new consumers read it as a RESOLVED CONNECT** — needs the producer/resolved
split.~~ ⚠ NOT PURELY GEOMETRIC ALREADY: the producer refuses a PARRIED hit
("no `LandedBodyHit`: the attacker's move did not connect"), so the seam is
half-drawn rather than absent.

**(5) IMPACT HITSTOP — CONFIRMED AT HEAD 2026-08-25, AND IT IS PLAYER-FACING.**
`request_impact_hitstop_on_landed_hits` runs in `CombatSet::Settle` and reads
`BodyCombat::hitstop_timer` off the victim. The schedule's own comment says
player-victim hits are handed to a FIFO "the player resolver (which runs in NEXT
frame's PlayerSimulation) drains" — so for a PLAYER victim that timer has not
been written yet. ⇒ **THE MATCH FREEZE WORKS WHEN A CPU IS HIT AND NOT WHEN THE
HUMAN IS**, which is a feel difference a player can notice without knowing why.
The existing arm is even named `two_cpus_trading_hits_freeze_the_world_with_no_
player_in_it`, and every test in that file INJECTS `hitstop_timer` on the victim
before firing the message — so none of them can see the ordering at all.

⛔ THE FIX IS NOT "ADD THE HOLD TO THE PLAYER RESOLVER": that system carries a
comment saying it is at Bevy's param ceiling, twice, and enlarging it is exactly
what (16) warns against. ⇒ the resolver should PUBLISH a resolved hit carrying
its hitlag, and a small system turns that into the freeze — which is (4)'s split
arriving through its first real customer rather than as a refactor. ✔✔ **(7) THE INPUT-ORDER RECOGNISER — CLOSED, and this paragraph was the stale
one.** It read *"the four techniques are declarable now, but which one a PLAYER
asked for still cannot be read from stick-then-button order"*. It can:
`AttackGestureState::special_turn_window` is armed where the press is ACCEPTED
and `apply_special_turn_flicks` spends it on a later flick, so the ORDER is what
selects the outcome — and it reads `ActorControlFrame::steer_axis()`, the stick
the player is holding, rather than the damped locomotion a rooted move zeroes.
Recorded under **D241**; verified at HEAD 2026-08-26.

✔ **(11) CLOSED — re-verified at HEAD 2026-08-26: both sides of that comparison
read ONE threshold now.** The dash writes `prev_steer_dir` through a
`deadzoned()` helper keyed on `STEER_DEADZONE` (`integration.rs:615`) and the
turnaround reads `steer_stick.x.abs() > integration::STEER_DEADZONE`
(`abilities.rs:287`), so the mismatched-threshold edge cannot be true on every
tick any more. ⭐ and the fix deliberately SHARED the dash's memory rather than
adding `prev_turn_steer_dir` — a second rollback field bought for a difference
nothing can observe, since the facing snaps to the stick when the timer expires.
~~the turnaround edge shares
`prev_steer_dir` with the initial dash, whose deadzone is 0.5 against the
turnaround's 0.1 — an analog reversal near -0.2 re-arms forever;~~ **(12) — MEASURED AND
PARKED, NO ADOPTER.** The seam is real and I proved it for the ROLL (107px held
vs 33px released), where it is fixed directly. But the review's dash-attack
customer does not exist at HEAD: `dash_attack` sets `roots_steering: false`, and
every authored `start_impulse` is `None` or a pass-through destructure — so NO
move both roots steering and authors a carry. For the moves that DO root
(grounded attacks), friction toward zero is what planting looks like and is
correct. ⇒ generalising "steering permission" apart from "axis magnitude" now
would be an abstraction with one adopter that already has its own fix. ⚠ REOPEN
WHEN: a move roots steering AND authors momentum it expects to keep. **(15) — RESOLVED AS A CONTRACT FIX, 2026-08-25.** The review is right that a
`bool` cannot express "once per hit", and right that one representation was
documented as another. But the CODE is the correct half: `asdi_owed`'s own doc
says a fresh hit RE-ARMS the freeze rather than queueing behind it, so hits
during hitlag extend ONE episode and the body is displaced once when it ends —
the beat a player reads. A per-hit counter would pay a multihit several
displacements out of a single freeze, which is a different mechanic. ⇒ the
heading now says ONCE PER FREEZE (its test was already named "once when the
freeze lifts"), and both places state why. ⛔ do not change the state to satisfy
a sentence that was only ever a description. **(16) the feature-hit gateway grows by SystemParam packing** — guidance
for the next change to it, not a campaign.

- ✔ **D238 — CLOSED 2026-08-26. THE DEEP REVIEW, CHECKPOINT 2 (GPT 5.6):
  FOURTEEN MORE, AND ONE SENTENCE THAT NAMES THE PATTERN.** Thirteen shipped;
  **(20) is the only one left and it is not this row's to answer** — it turned
  out to be the same missing fact as D179's contact bound and D241's #20, all
  three now routed to [decision 34](awaiting-maintainer-decision.md). ⚠ two
  paragraphs below still read as open and were verified DONE at HEAD: (27)
  latches `MovePlayback::started_grounded` and (29) has all four halves. The
  authoring cleanup this row asked for — `autolink + windbox` as a SUM — landed
  the same day. ⚠ (25)'s endpoint (an item that RIDES its platform rather than
  merely falling off it) is a named future step, not a defect.
  (opened 2026-08-25; closed 2026-08-26)**

⭐⭐⭐ **THE REVIEWER'S OWN DIAGNOSIS IS THE MOST VALUABLE LINE IN EITHER REPORT,
and it matches what I found independently all day:** *"a number of recent
mechanics are locally tested at the point where they are authored, but their
semantic distinction is LOST AT THE NEXT SHARED GATEWAY."* Its examples:

```text
windboxes           lose their REACTION KIND at `pending_launch: Vec2`
shield transitions  lose their CAUSE at the active/inactive bool
recovery helpless   loses its EPISODE by deriving from a resource COUNT
rooted moves        lose the RAW STICK by rewriting the input frame
match commands      lose their ROLLBACK STATUS as ordinary Bevy messages
```

⇒ every one is a fact that was true where it was decided and unrecoverable one
seam later. My own day found the same shape three more times (a scope set at
spawn, a scope never retracted, a body mutated during proposal) — so this is the
project's dominant defect class right now, not a coincidence of one review.

⭐ ALSO WORTH RECORDING: the reviewer looked for MISSING ROLLBACK REGISTRATION on
every field added this week — evade staling, shield tilt, ASDI, maneuver state,
settled items, match clock, sudden death — and found none. The registration
discipline is holding; it is SEMANTIC OWNERSHIP that is not.

⭐⭐ **(17) CLOSED 2026-08-25 — THE COMMENT STATED THE RULE AND THE CODE ASKED A
DIFFERENT QUESTION.** Drop lag's own comment says the cost is for *"you simply let
go"*; the condition was `was_up && !active`, which is EVERY way a guard can end.
Under `air_guard: false` (Smash) a fighter dropping through a platform with Shield
still held had its guard forced down and was billed the full 11-frame release
penalty — and `drop_lag_timer` feeds `hard_lock_timer`, so a platform drop
hard-locked the body for letting go of a button nobody let go of. The same
misreading caught a shield that BREAKS. Now `!input.shield_held` is the
authority: the hand says whether the player released. Both directions poisoned.

⚠ **AND IT RE-TUNED THE CPUs, WHICH IS THE INTERESTING PART.** Removing a penalty
nobody earned changed how matches play out: `the_cpu_throws_its_authored_recovery
_during_a_match` went red because George now takes longer to find himself
offstage. Widened 1800 → 3600, MEASURED (2100 fails, 2400 passes, so half again
past the turn) and with the CAUSE recorded in the test — which is what separates
an honest widening from the "hand-kept ledger" its sibling warns about.
⇒ **REMOVING A SPURIOUS MECHANIC RE-TUNES EVERYTHING BUILT ON IT**, exactly as
waking a dormant one does.

▢ LIVE MOVEMENT (the rest of the pass): ⭐⭐ **(18) CLOSED 2026-08-25 — AND THE
COMMENT NAMED ITS OWN PRECONDITION.** It read *"before this becomes an action
gate, the roll needs a timer that is ITS OWN"*, because `dodge_roll_timer` is
shared with the SPOT DODGE and gating on it silenced fighters that had only
spot-dodged — it even recorded the launch test that broke. HEAD arms
`dodge_roll_endlag_timer` only for a roll, so the precondition is MET; the gate is
in, and the named test (`an_up_tilt_launches_much_further_at_a_high_percent`)
PASSES with it. A roll's punish window was canonical state nothing consulted — a
mechanic recorded and not implemented.

⭐ **AND (37), FROM THE GAP PASS, CLOSED WITH IT** — same family, same file:
`spend_evade` promises the stale count "only starts coming down once the body
actually stops" and the decay ticked from the moment the evade was ACCEPTED, so a
0.22s roll spent ~18% of Smash's 1.2s forgiveness performing the maneuver the
delay exists to charge for. Gated on the MANEUVER clocks (not the i-frame clock —
forgiveness must not speed up because the fighter has been spamming). ⚠ THE
EXISTING DECAY TEST SEEDS STALE STATE ON AN IDLE BODY, so it never ran an accepted
evade through its own maneuver: the fourth fixture today that omitted the state
its bug lived in.
**(19) ROOTED MOVES ERASE STICK HISTORY** — `damped_by_move_motion` zeroes
locomotion before the kernel sees it, so `prev_steer_dir` records 0 and merely
HOLDING a direction through a rooted attack grants a free initial dash when it
ends.

▢ **(20) INITIAL DASH AND SHIELD SETTLING STILL INFER OWNERSHIP FROM MAGNITUDE**
— and the reviewer is right that the ROLL got this right and these two did not:
a launch WEAKER than the dash speed is overwritten (`-200` becomes `+270`,
reversing a live launch), and the shield brake eats anything under
`max_run_speed`. ⛔⛔ **AND THE REVIEW'S PROPOSED SEAM IS NOT AVAILABLE — CHECKED 2026-08-25.** It
says `BodyFlightState::carried_run` already carries the external component, so
these should modify the locomotion-owned part. But `carried_run` is ZEROED the
instant `carried_hold` expires, and the code says why in its own words:
*"SURRENDERED. The floor was owed because the body could not answer for the
momentum; control is back, so it stops being owed."* ⇒ it describes momentum
imparted WHILE CONTROL WAS ABSENT, and it is surrendered exactly when control
returns — which is precisely when a player dashes or raises a guard. **The
distinction the review wants the implementation to already possess expires before
the window these bugs live in.**

⇒ **SO THE FIX IS THE ROLL'S, NOT THE CARRY'S**: remember what the maneuver
ITSELF put in and take back only that (`dodge_roll_push` is the pattern). ⚠ FOR
THE DASH THAT IS A FEEL CHANGE AND NEEDS JON: the dash currently SETS the run
axis, which is why it can delete a launch; making it ADD a bounded push instead
means dashing while carrying momentum no longer produces a fixed speed. The
shield brake can be fixed the same way with no feel change — brake only the
speed the brake itself is responsible for.

⛔⛔ **RE-READ 2026-08-26: BOTH BOUNDS ARE ALREADY THERE, AND THEY ARE A THIRD
COPY OF ONE MISSING FACT.** The dash floor is at `integration.rs:862` and the
shield brake at `:987`, each with its own comment saying *"anything faster than
its own run is somebody else's velocity — a LAUNCH"*, each written after a
measured regression that deleted knockback. ⇒ these are not unbounded; they are
bounded by a MAGNITUDE, and so is a third site nobody had connected to them —
`body_contact`'s *"faster than one walk-tick is not walking"*, which D179 closed
as a feel question the same day.

⭐⭐ **THREE CONSUMERS, ONE MISSING FACT, AND THE GENRE ALREADY NAMES IT.** A
launch that has DECAYED below the threshold is indistinguishable from walking at
all three sites, and the thing that would distinguish it is TUMBLE — a state a
body is IN, at any magnitude. `movement/knockdown.rs::launch_into_tumble` is the
one entry point and the whole knockdown → tech → getup cycle sits behind it.

⛔⛔ **CORRECTION 2026-08-26: THE FACT IS NOT DORMANT, AND I SAID IT WAS.** The
Smash stage authors `tumble_speed: 500.0` (`ambition_demo_smash/src/lib.rs:275`)
and `match_report -- 30 --runs 3` measures **121–203–401 tumbling ticks a run**.
`DEFAULT_TUNING.tumble_speed` is `0.0` and a test PINS that zero, so the fact is
live in SMASH and absent in AMBITION — and the three magnitude bounds live in the
kernel BOTH use. ⇒ the question moved and improved; see
[decision 34](awaiting-maintainer-decision.md), which now costs three answers
including a cheap one (read tumble where it exists, keep the magnitude where it
does not). ⚠ **RUNNING THE GAME IS WHAT CAUGHT IT** — the claim came from another
row and I repeated it without a match.

⇒ **(20) IS NOT BLOCKED ON A FEEL CHANGE TO THE DASH.** It is blocked on
[decision 34](awaiting-maintainer-decision.md), which now asks the real question:
wake tumble and re-tune knockback for the whole cast, or keep three thresholds
guessing. ⛔ do not fix one of the three sites in isolation — that leaves two
copies of the same guess and a reviewer will find them next week.

⭐⭐ **(21) CLOSED 2026-08-25 — AND THE FIX IS THE OPPOSITE OF BOTH OBVIOUS
ONES.** The registration comment claimed `clear_message_on_rollback` *"restores
the channel with its cursor, so the resim reads the request again"*; the backend
`.clear()`s the buffer, so an Exit Match consumed on a speculative frame was GONE
after a rewind. ⛔ AND SNAPSHOTTING IT FAILS FROM THE OTHER SIDE: the ask is made
OUTSIDE the simulation, so a resimulation cannot re-make it, and rewinding a
resource that holds it throws it away exactly as the clear did. ⇒ what survives
both is a latch that does NOT rewind and NAMES ITS MATCH
(`MatchAbandonRequest::stop(&active)`): a rewind leaves the ask standing so the
resim reaches the same verdict, and the next match ignores it because the
instance differs — which is the job the channel-clear used to do. The
`MatchAbandoned` message is DELETED, with its three registrations, its
`clear_message_on_rollback` and its baseline row; wire v106. ⚠ the waiver in
`rollback_coverage` carries the reasoning, because "why is this NOT rollback
state" is exactly the question a later session will ask.

◐ **(22) HALF CLOSED 2026-08-25 — the RETURN, which is the half that cannot be
taken back.** The countdown armed on `StocksMatchDecided`, which a SPECULATIVE
frame can write, into a `Local` GGRS never rewinds — so a rolled-back decision
still sent the player to the lobby out of a live match. It reads
`StocksMatchSettled` now (rollback STATE, stamped with its match) and arms only
when `ConfirmedFrameBoundary::fully_confirmed`, so by the time it commits the
settlement can never be simulated again. ⛔ THE STAMP ALSO REPLACED the
`decided.clear()` on leaving the stage: a previous match's verdict cannot arm the
next one. Straddling arms; the poison sends the player home on a prediction.

⭐⭐ **AND THE CARD'S HALF CLOSED 2026-08-26 — BY MOVING THE VERDICT ONTO THE
LATCH.** The blocker was real: `announce_the_winner` needs the VERDICT and
`StocksMatchSettled` said only WHETHER, so guarding it the same way risked
DROPPING the announcement rather than delaying it — a reader that keeps its
cursor is still bounded by a two-frame channel. ⇒ **STATE HAS NO CURSOR.**
`StocksMatchSettled` carries `(MatchInstance, MatchVerdict)` now, which is what
its own doc always claimed it was (*"the outcome for match X"*), and the card
reads the latch on the RISING EDGE — the message fired once, and a state read
would otherwise rewrite the slot every tick.

⛔ THREE `settle` SITES AND THE WIRE: `Copy`/`Eq` had to go (a `Winner` carries a
side label), the verdict is tagged into the snapshot beside the instance, and the
schema goes to v112. Blast radius was three call sites, not the twenty-eight
references the type has.

▢ LIVE SMASH PARITY: ⭐⭐ **(23) CLOSED 2026-08-25** — the gate
carried the sentence *"AND IT STILL SPENDS THE LOCKOUT BELOW: a player who mashes
into an untechable launch should not be free to keep mashing"*, and the lockout
was only charged where `tech_press_timer` EXPIRES, which an untechable press never
arms. Mashing into a launch too hard to tech cost NOTHING. One gate, three
outcomes now, in the order a press is judged. ⇒ THIRD TIME TODAY that a COMMENT
STATED THE CORRECT RULE while the code beneath asked a different question —
shield drop lag, roll endlag's precondition, and this; ◐ **(24) HALF CLOSED
2026-08-25 — the half that needed no new state.** The helpless branch RETURNED
BEFORE restoring the preserved tech edge, justified as *"a helpless body has no
floor game to tech into — it has not been hit"*. ⛔⛔ THE PREMISE IS FALSE EXACTLY
WHERE THE VALUE EXISTS: `tech_press` is `Some` ONLY while TUMBLING, and a tumbling
body has been hit by definition. A fighter that spent recovery, went helpless and
was then launched into a wall lost its tech — punished twice for one decision, and
specifically for having already spent recovery. Poisoned.

⭐⭐ **THE REMAINING HALF CLOSED 2026-08-25 — THE EPISODE IS BUILT.**
`BodyJumpState::post_recovery_helpless` is armed by the SPEND that takes the last
charge (only the spend knows it was the last), suppressed while the recovery move
plays, cleared by an accepted hit, and cleared with the ordinary landing-shaped
refresh. `body_is_helpless` reads the EPISODE, not the count. ⛔ CLEARING IT
REFUNDS NOTHING — the charge stays spent and so does the double jump, pinned by
its own assertion. Wire v107.

⛔⛔ **AND THE FIXTURE HAD TO BE REPAIRED, NOT THE RULE.** The existing arm set
`recovery_charges: 0` and called that helpless — which is the resource reading
the change is about. A body with no charges that never spent one THIS AIRTIME is
not helpless, and there is now an arm saying so; without it the gate could quietly
go back to reading the count.

The table it was built from:

```text
spend last recovery      arm post_recovery_helpless
recovery move playing    episode suppressed
move ends airborne       helpless becomes effective
accepted hit             episode CLEARED — charge stays spent
land / ledge / respawn   cleared with the ordinary refresh
```

⛔ DO NOT restore the recovery charge or the spent double jump on hit; those are
recent, deliberate corrections. ⚠ `BodyJumpState` is snapshotted, so this is a
schema bump plus four seams — a fresh session's work, not a tail-end one.

◐ **(29) THE WINDBOX PRIMITIVE — 29a CLOSED 2026-08-25.** The flinchless arm
declines `hitstun_timer` and `recoil_lock_timer` was assigned four lines later
UNCONDITIONALLY, so a volume whose contract is *"moves you and leaves you in
control"* removed all authority for several frames, and a REPEATING gust
refreshed that every pulse. Now it declines the lock by the same asymmetry the
stun arm states — and does NOT discharge one, which is poisoned in both
directions (clearing it would make a gust the best combo breaker in the game).

⭐⭐ **29b CLOSED 2026-08-25 — THE LAUNCH CARRIES ITS KIND.** The gateway asked
`jab_lock()` and `launch_into_tumble()` from SPEED ALONE, so a weak gust pinned a
prone body where it lay and a strong one sent it tumbling — both against a volume
whose contract is *"moves you and leaves you in control"*. ⇒
`BodyFlightState::pending_launch_flinchless`, staged and drained AS A PAIR
(`stage_launch` / `take_launch`) so a writer cannot set the vector and forget the
kind, which is the whole hazard of a flag beside a vector. Wire v108. ⭐ ONE
PRODUCTION WRITER, and it already held the fact: `hit_reaction` was reading
`knockback.flinchless` four lines below the line that staged the launch. ⚠ a
fixture that sets `pending_launch` by hand still gets `false`, which is what an
ordinary knockback means — so the many kernel fixtures that do say what they
meant. Straddling arms: same speed, same state, one flag apart.

✔ **29c — CLOSED, and the marker below was left on it. The finding is the
paragraph after: the fact was already at the site.** ~~the parry
producer and `resolve_body_hit` receive no fact saying the contact is a windbox,~~
so a gust can be PARRIED, or blocked for shield integrity and shieldstun and
pushback — against an authored `no shield`. ⇒ the volume needs a declared
guard interaction (`NormalStrike` / `IgnoreGuard` / `Unstoppable`), read by both.

⭐⭐ **29c FULLY CLOSED IN CODE 2026-08-25 — AND THE "GATEWAY" WAS NOT CLOSED
AFTER ALL.** The review proposed a new `DefenseInteraction` enum threaded to both
seams. It is not needed: the producer already sets
`flinchless: hitbox.windbox.is_some()`, so "this is a push, not a strike" RIDES
THE KNOCKBACK to both damage roads — and `Option<GuardUnderFire>` already means
"no guard participates in this hit". **The channel existed; nobody had used it to
say so.** ⇒ CHECK THAT FIRST on the remaining gateway items (29b, resolved-hit,
recovery-helpless, directional melee): the fact may already be at the site.

⭐⭐ **THE BLOCK HALF IS PINNED NOW, AND NOT BY BUILDING THE TUNING.** The
recorded blocker was real — the road fixtures run on `ShieldTuning::OFF`, where a
BLOCK leaves `depleted`, `stun_timer` and `break_timer` all zero, so a guard that
declines a gust and one that engages it for nothing are the same observation. ⇒
what separates them is whether a guard is OFFERED, and that is now ONE decision
in one place: `GuardUnderFire::offered_to`. It was written TWICE, with
near-identical prose, in the two roads D203 exists to stop rules being written
twice — and there is nowhere else a `GuardUnderFire` can be built, so the
compiler carries the wiring the fixture could not. Straddling arms plus the
`None`-knockback case (a damage-only tick still meets a raised guard).

⭐ **29c PARRY HALF CLOSED 2026-08-25.** A gust could be PARRIED, producing no
push at all — the producer asked `shield.parrying()` for every strike volume
alike. THE FACT WAS ALREADY ON THE `Hitbox` (`windbox`), so this was ONE
CONDITION rather than a new channel: the parry half was cheap precisely because
nothing had to cross a gateway. ⭐ THE BLOCK HALF CLOSED THE SAME WAY: the fact
was already riding the knockback, so it needed one constructor rather than a new
channel — see above.

✔✔ **AUTHORING CLEANUP — DONE 2026-08-26, AS A SUM TYPE.** The type permitted
`autolink + windbox` together while its own doc called the combination
contradictory (*"mutually exclusive in meaning, not in type"*) and the runtime
resolved the windbox first and said so in a comment. ⛔ **A COMMENT IS NOT A
SCHEMA.** Both `Option` fields became one
`reaction: Option<VolumeReaction>` — `Autolink(_) | Windbox(_)` — on the AUTHORED
`HitVolume` and on the runtime `Hitbox` alike, with `autolink()` / `windbox()`
accessors so the twelve read sites did not change shape.

⭐ **THE CHANGE IS NET NEGATIVE LINES**: 41 fixture literals wrote the pair
`autolink: None, windbox: None` and now write one `reaction: None`. ⛔ **AND IT
COST NO WIRE FORMAT**: `Hitbox` is registered `rollback_component_clone_entity_ref`,
not encoded, and appears in no `encoded_types` row. Done while it is still latent,
which is what the paragraph this replaces asked for.

✔✔ **(29) CLOSED — verified at HEAD 2026-08-26, and all four halves are in.**
The row still listed three of them as open; they are not:

```text
zero damage      closed 2026-08-25 (damage_apply's `.max(1)` undid the authored 0)
recoil lock      `hit_reaction.rs:275` is inside `if !knockback.flinchless`
jab-lock/tumble  `BodyFlightState::stage_launch(velocity, flinchless)` +
                 `take_launch() -> PendingLaunch{velocity, flinchless}` — the KIND
                 rides the gateway, and its doc names this review item
parry + block    `flinchless` was already riding the knockback; one condition each
```

⇒ **two facts crossing two gateways, exactly as the finding predicted** — not four
`if windbox` exceptions.

▢ ALSO: ◐ **(25) HALF CLOSED 2026-08-25 — AN UNSUPPORTED ITEM NOW FALLS.**
`SettledItem` was lifted only by CUSTODY transitions, so an item that landed on a
MOVING PLATFORM — which the composited collision world deliberately lets it do —
stayed fixed in world space once that platform left. Support is re-validated each
tick against the SAME probe `ground_item_physics` uses, so "what counts as
support" has one definition rather than two that can disagree.

⚠ **THE ENDPOINT IS STILL SUPPORT IDENTITY**: this makes an unsupported item
FALL; it does not make a supported one RIDE. Carrying an item with its platform
needs which-block plus a local offset.

⛔⛔ AND A POISON MADE ME DELETE AN ARM I HAD WRITTEN. Waking EVERY settled item
each tick leaves the fixture green: physics re-settles in the same tick and
detects the block before moving anything, so the item does not drift by a
thousandth of a pixel. **The two implementations are the same program along every
path that world can take.** The support check earns its place on semantics and
per-tick churn, not on observable behaviour — and the test now SAYS that instead
of asserting something it cannot distinguish. (Second time today a poison caught
an arm that could not fail.) **(26)(27) CLANK — CONFIRMED AND SCOPED, DELIBERATELY NOT SHIPPED.** (26) the
sweep skips a pair when `ended.contains(a_owner) || ended.contains(b_owner)`, so
an EARLIER pair's outcome decides whether a LATER pair is considered at all: with
three equal attacks overlapping on one tick, A/B resolves first by stable id,
both end, A/C and B/C are skipped, and C survives BECAUSE OF `SimId` ORDER.
Deterministic, not simultaneous. ⇒ THE FIX IS ONE LINE — drop the two
`ended.contains` terms from the skip; `resolved` stays the dedup and `ended`
becomes what its own comment already says it is, a COMMIT ledger applied after.
✔ **(27) CLOSED — and this paragraph was stale; verified at HEAD 2026-08-26.**
The finding was that eligibility asked `BodyGroundState::on_ground` AT COLLISION
TIME, so a ground attack stopped clanking the moment its owner walked off a ledge
and an aerial started when its owner landed. It reads `MovePlayback::
started_grounded` now (`clank.rs:190`) — the same stance the SELECTOR used to
choose the variant, already in that system's hand, so it cost no new channel.
*"Grounded attack" is a CLASSIFICATION and it is settled when the swing comes
out.*

⭐⭐ **(26) CLOSED 2026-08-25 — THE FIXTURE GOT BUILT AND IT REPRODUCED THE
DEFECT EXACTLY.** Three fighters on one spot, one equal strike volume each, ids
stated so the sweep's order is the fixture's: **one of three survived**, which is
what the finding predicted. The fix is the one line — `resolved` is the dedup and
`ended` is a COMMIT LEDGER applied after the sweep, exactly as its own comment
says; reading it as an eligibility gate made it a third thing.

⛔ THE PREVIOUS ENTRY, kept because the discipline was right: *I MADE THE (26)
CHANGE AND REVERTED IT UNVERIFIED.* No test exercised `arbitrate_attack_clanks`
at all — the only clank arm covered the pure `clank_verdict` — and shipping a
behavioural change to arbitration without one is what this run refuses. Building
the fixture was the work; the line was never the work. ⚠ LATENT EITHER WAY: Smash
declares `clank_damage_window = 0`.

⭐⭐ **(27) CLOSED 2026-08-26 — AND NOT ON THE STRIKE VOLUME.** The finding said
*"latch it on the strike volume"*, and the first implementation did exactly that:
a `grounded` bool on every rectangle a move opens. ⛔ THE STANCE IS A FACT ABOUT
THE MOVE, and a move opens many volumes — so it belongs on `MovePlayback`
(`started_grounded`), populated with the SAME `grounded` the selector used to
choose the variant. ⭐ AND THE CLANK ALREADY HELD THE PLAYBACK: it queries
`&mut MovePlayback` to cancel the loser. No new channel, no per-rectangle
duplicate, one fewer query on `advance_move_playback`. **(28) pivot selection and move-facing DISAGREE** —
the gesture picks the forward move using the turnaround facing, `start_move` then
snapshots the OLD `kin.facing`, so the move is selected forward and its geometry
mirrors the old way; ⭐⭐ **(30) CLOSED 2026-08-26 — AND THE TYPE IT NEEDED ALREADY EXISTED.** The
roster carried EIGHT loose rule fields (`opens_suspended`,
`opening_countdown_ticks`, `time_limit_ticks`, `item_spawns`, `fighter_stocks`,
`fighter_abilities`, `fighter_body`, `fighter_health_pool`) whose ONLY consumer
was a transcription block in `prepare_match` copying them one by one into
`MatchRules` — two representations of one fact. ⇒ the roster carries
`rules: MatchRules` and the transcription is DELETED. A new rule is now one field
on `MatchRules`, not a field here plus a line there plus an initializer in every
roster literal in the tree.

⛔ THE ROSTER STILL OWNS THE QUESTION — the fields' own doc said *"the engine does
not get an opinion about a match's economy"*, and that is unchanged. What changed
is that it says so once.

⭐ REVIEWER CLEARED, after tracing, with no defect: crouch/body-mode scheduling,
Z-drop's grab edge, the recovery edge-cancel landing-lag writer, route-authored
defense presentation, charged-projectile preparation. ⚠ one authoring seam: the
ranged consumer has no body-local MUZZLE, so charge presentation uses an
anatomically wrong fallback and `gun_sword` already carries an ID special-case —
add the anchor with the next ranged feature rather than a second exception.

- ✔ **D239 — CLOSED 2026-08-26. THE DEEP REVIEW, CHECKPOINT 3: SUDDEN DEATH WAS
  A THIRD IMPLEMENTED.** All five findings — (31) the charge shot that fired
  nothing, (32) sudden death's missing stock, (33) the half-retired
  non-contenders, (34) the clock, (35) the fold — are closed, (31) under Jon's
  own relayed ruling and re-verified at HEAD. Nothing here is open.
  (opened 2026-08-25; closed 2026-08-26)**

⭐⭐ **CLOSED 2026-08-25 — (32) SUDDEN DEATH NEVER GAVE ANYONE ONE STOCK.** The
transition set the damage and nothing else; it did not query `FighterStocks` at
all. A genuine timed tie happens at whatever stock count the fighters are on —
this file's own tiebreak arms tie at TWO — so the first KO spent a stock,
eliminated nobody, the ordinary respawn reset the very damage the round had just
staged, and sudden death simply CONTINUED. The stated rule is *"both at 300%, one
stock, first hit decides"*. ⛔⛔ AND THE TEST COULD NOT SEE IT BECAUSE ITS FIXTURE
HAD NO `FighterStocks` COMPONENT AT ALL — the day's recurring shape, a fixture
omitting the state the bug lives in.

⭐ ALSO CLOSED — (33) NON-CONTENDERS WERE RETIRED WITH HALF A TRANSITION.
`spend_fighter_stocks` inserts `FighterEliminated` AND removes `ActiveCombatant`,
and documents why: a body stays standing until a ruleset removes it, so a marker
alone leaves a corpse holding attack state and a place on the anti-clump board.
The timeout path did only the marker — a second, weaker definition of leaving the
match, with command deferral meaning cleanup cannot cover the gap. Both halves
poisoned.

⭐⭐ **(34) PROVEN 2026-08-25 — AND THE CLOCK WAS THE WAY IN.** The recorded
blocker was that `PreparedMatch` has no constructor, so a unit fixture could only
build a system that early-returns. The answer was not a constructor: the stage's
limit is eight minutes and NOTHING about this card depends on how those minutes
were spent, so the harness states a match already fought nearly to its limit
(`LiveMatchTicks::from_snapshot`) and lets the real timeout, the real tiebreak and
the real announcer do the rest. ⛔ THE ASSERTION IS THE DURATION — one tick of the
right word is exactly what the bug looked like — and the poison shows 0 of 120
frames. The original entry follows.

◐ **(34) THE SUDDEN-DEATH CARD LASTED ONE TICK — FIXED, NOT PROVEN.**
`announce_the_opening_countdown` owns that HUD slot for any UNSETTLED match, and
sudden death is deliberately unsettled (the match CONTINUING, not a result), so
the announcer cleared the card every tick while `SuddenDeathBegan` fires once and
cannot rewrite it. The announcer now stands down on `SuddenDeathEntered`, the
canonical latch. ⚠ NO REGRESSION: `PreparedMatch` has private fields and no
constructor, so a unit fixture can only build a system that EARLY-RETURNS — a
check that cannot fail. **The follow-up is an integration harness that plays a
timed match to expiry**, which nothing in the tree does yet.

✔✔ **(31) CLOSED under Jon's relayed ruling — re-verified at HEAD 2026-08-26.**
The refire floor moved OUT of the projectile consumer and became explicit weapon
readiness checked BEFORE move acceptance (`weapon_ready`, `moveset/mod.rs:2042`,
consulted at both the trigger and the selection site), and an accepted authored
move carries `RangedCommitment::CommittedMove` so the consumer never asks a
second time — *"a committed move is not an attempt; its recharge was spent where
the move was ACCEPTED, a quarter of a second and one whole windup ago."* The
cadence is preserved deliberately rather than as a side effect: the floor is
authored per action (`RangedActionSpec::refire_s`) instead of one constant every
character in the game was silently balanced around. ⇒ neither (a) nor (b) below
as written; the ruling took (a)'s authority with (b)'s truthfulness.

~~▢ STILL OPEN: **(31) CHARGE SHOT can start, animate, play its release and fire
NOTHING — CONFIRMED AT HEAD 2026-08-25.**~~ The numbers are exact: `duration_s
0.58`, `CHARGE_FIRE_AT_S 0.26`, `RANGED_REFIRE_S 1.1`. A second Charge Shot at
the earliest legal opportunity (0.58s) reaches its authored fire frame at 0.84s —
0.58s after the first shot — and `try_fire_ranged` refuses it, so the move plays
its release and no projectile appears. ⛔ THE CONSUMER'S COMMENT ("simply spawns
nothing this tick") is right for a controller SPAMMING fire and wrong for an
authored move that has already been accepted, played startup, charged and reached
its firing frame.

⚠ **TWO COHERENT FIXES AND THEY ARE NOT EQUIVALENT — THIS IS A BALANCE DECISION,
so it is named rather than picked:**

```text
(a) gate the move's START on refire      keeps the weapon resource
    availability                          AUTHORITATIVE; no balance change;
                                          needs the ranged resource where moves
                                          are selected
(b) an accepted move's authored event    makes the authored timeline TRUTHFUL;
    is GUARANTEED                         Charge Shot can then fire every 0.58s
                                          instead of 1.1s — a real balance change
```

⭐ (b) IS THE DAY'S PATTERN EXACTLY: the fact *"this event came from an already
committed move"* is lost at the `ActorActionMessage` boundary, which carries only
`actor` and `request`. Blast radius measured: THIRTEEN construction sites, of
which the two in `moveset/mod.rs` (~2832, ~2889) are the committed ones. A
constructor pair (`requested` / `committed`) keeps the call sites honest.

⭐⭐ **JON PICKED (b), 2026-08-25, AND REFRAMED IT — CLOSED.** *"An accepted
authored move should guarantee its authored fire event. If 0.58s is too fast,
encode the desired cadence at move acceptance/authoring rather than vetoing the
projectile downstream."* ⛔ AND HIS KEY OBSERVATION WAS THAT `RANGED_REFIRE_S`
IS NOT PROJECTILE POLYGON BALANCE — it is a generic legacy floor for low-level
fire ATTEMPTS. Two authorities had come to overlap; the move is the right one,
because a 0.58s move with recovery IS a fire-rate limiter.

⇒ SHIPPED: `ActorActionMessage` carries an `ActionOrigin` (`Requested` /
`CommittedMove`); the floor binds attempts only; a committed shot still ARMS the
floor so a brain cannot fire free the tick after an authored one. Both poisons
redden — vetoing committed events, and deleting the floor.

⛔⛔ **AND MEASURING IT FIRST FOUND SOMETHING MUCH BIGGER THAN CHARGE SHOT.** In
the duel arena, **22 of 28 authored ranged events were being refused by that
floor.** The shared `simple_ranged` prefab authors 0.30–0.50s moves, so the rate
every ranged actor in the game has ever PLAYED at is 1.1s while its move claimed
0.3s. Arming the exemption without a content pass made them all fire ~3.7× faster
and the duel fighters stopped closing to melee entirely (measured: `melee 0`).

⇒ **SO THE STEP-4 CONTENT PASS WAS NOT A BALANCE JUDGEMENT — IT WAS THE SHIPPED
RATE WRITTEN DOWN.** Each style's windup+recover now sums to 1.10 (Rock
0.12+0.98, Bolt 0.18+0.92, Arrow 0.28+0.82): identical rate, and the commitment
moved to where a player can SEE it instead of an invisible weapon lock.

⭐⭐ **LANDED 2026-08-25 — AND NEITHER MEASURED COST WAS PAID. SEE D241.** Jon's
answer rejected the choice below: the two options were bundling three independent
variables (firing cadence, animation commitment, locomotion freedom). The floor
moved to move ACCEPTANCE as `RangedActionSpec::refire_s` and the moves kept their
short recovery. Measured on `duel_arena`: PCA melee 36, robot melee 23. The two
costs below are kept as the record of what was rejected and why:

```text
keep the authored 0.30s moves
  → every ranged actor fires ~3.7× faster than it ever has
  → measured: both duel fighters stop closing to melee entirely (melee 0)

stretch each style to sum 1.10 (Rock 0.12+0.98, …)
  → identical fire RATE, commitment made visible
  → ⛔ BUT the recovery is REAL: the body is motion-damped for ~1.1s per shot
     instead of ~0.3s. Measured: a possessed actor reads locomotion.x = 0,
     because it is now inside a damping move most of the time.
```

⇒ **"SAME RATE" IS NOT "SAME FEEL".** Under the veto a body finished its 0.3s
move and WALKED FREELY for 0.8s while an invisible cooldown ticked; authored
recovery takes that mobility away. That is a real change to every ranged actor
and it is Jon's to make — which is exactly the playtest step he sequenced.

⭐⭐⭐ **THE GENERAL HAZARD, worth more than the fix: A DOWNSTREAM VETO THAT
SILENTLY SWALLOWS A COMMITTED ACTION BECOMES LOAD-BEARING.** Content is authored
underneath it and tuned by playtest AGAINST the vetoed behaviour, so the authored
number was never a rate anyone experienced. Removing such a veto PROMOTES
whatever it was silently enforcing into something that must now be authored.
Same shape as the shield drop-lag fix the same day — except the affected surface
is CONTENT rather than a fixture. ⭐ **(35) CLOSED 2026-08-25** — the fold
QUERIED `Has<FighterEliminated>` AND DISCARDED IT (`_`), so an eliminated body
scored for its side for as long as it stayed RESIDENT, and two identical
histories ranked differently depending on whether the last stock went one tick
before the clock or ON it. Now "who is still standing", the one reading that does
not depend on residency. ⚠ THE OTHER READING — whole-team match HISTORY — is a
PRODUCT RULE, not a bug fix: it needs side-level scoring that outlives a body,
because components cannot store what a despawned fighter did. ⛔⛔ AND NO TEST RAN
THE FOLD AT ALL: every existing arm built the side map BY HAND and asserted the
RANKING, so none could see what the fold put in it — the third fixture today that
omitted the state its bug lived in; **(36) borrowed archetype movesets
use STRING SURGERY as identity** — `under_own_name` strips caller-supplied
prefixes and panics on a move that follows none, and shipped tables already use
two conventions, so every future move-id-referencing field is a new obligation.

- ✔ **D240 — THE GAP PASS: THREE MORE, AND A CORRECTION THAT MATTERS MORE THAN
  THEY DO. Opened 2026-08-25, CLOSED 2026-08-28.**
  ⚠ **its three items had ALL read CLOSED / COMPLETED / struck-through for days**,
  and its two remaining `▢` marks sat on lines whose own text said so. The one
  thing that was genuinely still open — the back-air diagnosis — turned out to be
  a SYMPTOM PARAGRAPH for D252, which closed on 2026-08-27 five rows further down
  this same file. ⇒ measured, not assumed: `moveset_takes` plays `air_back` for
  both characters tried.

⭐⭐⭐ **THE CORRECTION FIRST — IT CHANGES HOW 4/5 GETS BUILT.** *Do not wire
gameplay to the existing `BodyHitResolved`.* It is `#[cfg(feature = "causal")]`,
its writer is `Option`, and its own comments guarantee nothing in the simulation
reads it. Depending on it inverts the dependency: an OPTIONAL INSPECTOR becomes
REQUIRED GAMEPLAY AUTHORITY. ⇒ publish an unconditional resolved-hit fact that
simulation consumes, and let the inspector DERIVE the causal message from it.
Reuse the resolution VOCABULARY, not the channel. ⭐ I reached the same
conclusion independently while scoping the split and refused to reuse it — the
agreement is worth recording because it is the one design decision most likely to
be taken the wrong way round by a later session.

⭐⭐ **(38) CLOSED 2026-08-25.** The flag now asks each body
`Has<DrivingParticipant>`, and the singular `ControlledSubject` /
`PrimaryPlayer` params are DELETED rather than left bound. Renamed
`controlled` → `driven`, because the CONFLATION was the bug. The regression
contains no PRIMARY distinction at all — the moment one appears the old reading
is back. ⚠ ALSO FOUND ON THE WAY: `ambition_render` was RED on a hand-kept
west-drawn-sheet list that the new Author/Officer paperdolls extend (they borrow
Pointed Polygon's art and declare `authored_faces_left: true`). ⛔⛔ MY EARLIER
68-CRATE SWEEP CHECKED COMPILATION, NOT TEST PASSES — `cargo check --all-targets`
would never have caught it. **A crate's suite can be red while the gate and the
app suite are green.**

✔ ~~(38) THE NAMEPLATE POLICY IS PLURAL AND ITS PRODUCER IS SINGULAR.~~
`label_driven_bodies` is documented to apply uniformly to every body SOMEBODY IS
DRIVING, but `rebuild_nameplate_index` computes ONE `controlled_body` from
`ControlledSubject` (or a `PrimaryPlayer` fallback) and sets each row's flag by
`Some(entity) == controlled_body`. ⚠ MASKED IN SMASH, which labels driven
fighters; it breaks a multi-driver room with the opposite policy, where P1's body
is suppressed and P2's still gets a plate. ⭐ THE PLURAL AUTHORITY ALREADY EXISTS:
`DrivingParticipant`, and `ControlledBodiesView` already projects it correctly —
its own comment says *"a couch-versus match has two driven bodies and neither is
more protected than the other"*. ⇒ read `Has<DrivingParticipant>` per body, and
rename the flag `controlled` → `driven`: the bug IS the conflation of "the body
the camera focuses on" with "any body a participant drives".

✔ **(39) CLOSED — re-verified at HEAD 2026-08-26: `attack_intent_from_move_id`
has NO occurrence left in the tree and `MovePlayback::attack_intent` carries the
resolved gesture (`moveset/mod.rs:266`, with `with_attack_intent` as the seam).
Deletion, not a wrapper, exactly as the trace below asked.**

~~▢ **(39) DIRECTIONAL MELEE IS RECONSTRUCTED FROM MOVE-ID SPELLING.**~~
`attack_intent_from_move_id` matches a seven-entry canonical vocabulary
(`attack_up`, `attack_air_back`, …) and falls through to `Forward`. Pointed
authors `polygon_tilt_up`, Pugnacious `polygon_brawler_air_back`, and the
borrowed fighters add another prefix — so ALL of them synthesise `Forward`. ⚠ NOT
A HITBOX BUG (`MovePlayback` stays authoritative for geometry, and the pose path
prefers the authored clip), but `BodyMelee.swing.spec.intent` is canonical-looking
read-model state that animation, the HUD and gizmos consume. ⛔ THE COMMENT ABOVE
`synth_swing_from_move` claims *"the move's directional variant id carries the
swing direction"*, which is simply untrue for shipped content. ⇒ capture the
semantic `AttackIntent` where it is already known — gesture/action resolution —
and carry it on the accepted playback. DO NOT improve the string parser.

⭐ **TRACED 2026-08-25, so the next session starts scoped.** `synth_swing_from_
move(pb: &MovePlayback)` already takes the playback, so carrying the intent there
lets `attack_intent_from_move_id` be DELETED rather than wrapped — deletion is
available, which is what makes this worth doing properly.

```text
1  MovePlayback gains attack_intent: Option<AttackIntent>
2  trigger_moveset_moves sets it from the RESOLVED gesture
   (`intent.direction` + `intent.posture`), which it already holds
3  synth_swing_from_move reads it; the string match is deleted
4  ⛔ WIRE FORMAT: MovePlayback is snapshotted
   (`actor.move_playback  component-clone-resolved`), so this is a schema
   bump + baseline row, not a local change
5  ⚠ moves that start WITHOUT a gesture (pogo, held weapon, chain successors)
   carry `None` — decide whether that means Forward or "not directional",
   and say which in the type rather than in a fallback
```

⭐⭐ **CLOSED 2026-08-25, AND STEP 4 WAS WRONG.** `MovePlayback` carries
`attack_intent`, resolved by `attack_intent_of(dir, posture, running)` from the
same three facts the move was selected from; `attack_intent_from_move_id` is
DELETED. ⛔ NO WIRE BUMP: the registration is `component-clone-resolved`, so a
new field travels with the clone — the baseline test says so, and step 4's
prediction of a schema bump did not survive being measured.

⇒ **STEP 5 ANSWERED IN THE TYPE, not a fallback.** The field is
`AttackIntent`, not `Option<AttackIntent>`: a move no directional gesture
started (a chain successor, a held weapon's action) reports `Forward`, which is
exactly what the flat swing published for one. Saying it once at construction
keeps the fallback out of every consumer.

⛔⛔ **AND THE FIRST REGRESSION PINNED THE WRONG HALF.** It read the playback
field, so restoring the string parser in `synth_swing_from_move` left it GREEN —
the capture was proven and the read model was free to keep spelling out ids. It
runs `project_moveset_melee_to_body_melee` and asserts on
`BodyMelee.swing.spec.intent` now. Same shape as the day's other lesson: a
hand-listed chain pins the FUNCTION, not the WIRING.

✔ **(36) COMPLETED: it is a SCHEMA OWNERSHIP problem, not a broken fighter.** The
reviewer found no currently-broken Author/Officer cancel reference, so do not
invent one. The real hazard is that move IDs also live OUTSIDE `MovesetContract`
— `HurtboxDoc::moves` is keyed by move id — and `under_own_name` cannot discover
or rewrite them. ⇒ either make remapping a SCHEMA operation
(`MovesetContract::remap_move_ids`) so the crate owning every internal reference
owns the traversal, or better: keep archetype-local move keys and separate them
from CAUSAL IDENTITY, which the runtime already does (`SubjectKey::Sim(identity)`
+ `move_id`). ⚠ THE HELPER'S PREMISE IS QUESTIONABLE: there is no architectural
requirement that every fighter have globally distinct move keys.

◐ **(36) HALF CLOSED 2026-08-25 — THE TRAVERSAL MOVED TO THE SCHEMA.**
`MovesetContract::remap_move_ids` owns the walk now, beside the type that owns
the fields, so a future id-bearing field on a `MoveSpec` is an obligation the
compiler shows to whoever adds it rather than one on a CONTENT crate that would
never hear about it. `under_own_name` is the PREFIX POLICY and nothing else, and
`archetype_moveset.rs` no longer names `WindowTag`. Poisoned on the cancel-target
arm — missing one is not a red test, it is one dead button in a match.

✔ **THE REMAINING HALF IS THE PREMISE, and the ledger already doubts it —
MEASURED 2026-08-26 AND IT HAS NO CUSTOMER, so do not build a better renamer.**
~~The prefix rename still PANICS on an id carrying none of the archetype's
prefixes, and move ids also live OUTSIDE the contract (`HurtboxDoc::moves` is
keyed by one), which `remap_move_ids` cannot reach.~~

```text
under_own_name adopters        author_moveset.rs, officer_moveset.rs   (2)
do either author a HurtboxDoc? no — `grep -c hurtbox` is 0 on both
HurtboxDoc::moves populated in PRODUCTION at exactly one site:
  versus_fighters.rs:146 — the two duellists, who are their OWN characters
  and are not archetype-renamed at all
```

⇒ **the unreachable field and the renamed fighters are disjoint populations.**
⛔ and the PANIC is not a defect either — it is documented as the point:
*"a half-applied rename … surfaces as one dead button in a match rather than as
a red test. It fired the first time this ran, on exactly the two ids that break
the pattern."* Fail-loud is this project's pre-release stance. ⇒ what stays true
is the row's own conclusion: the endpoint is archetype-LOCAL keys, and until a
renamed fighter authors per-move hurtboxes there is nothing here to reach. ⚠ AND THERE IS NO ARCHITECTURAL
REQUIREMENT that every fighter have globally distinct move keys — the runtime
already separates causal identity from the key (`SubjectKey::Sim(identity)` +
`move_id`). ⇒ the endpoint is archetype-LOCAL keys, not a better renamer.

⭐ RECHECKED WITH NO DEFECT: HUD punch (the intermediate bug is fixed at HEAD —
do not report it), held-Special charge propagation, shield-tilt geometry vs
presentation, trade recoil, deterministic item spawning, the rest of the
`stocks_match` machine, fixed-knockback/transcendent-hit (inventory work, not
runtime), rollback registration of every new canonical field, camera reset,
crouch, Z-drop, recovery edge-cancel, route-authored defense, Pointed's autolink
frame. A targeted rescan for direct world-axis mutations and allocator identities
led back to the wavebounce, ledge-trump and bark findings rather than a third.

- ✔ **D254 — THE OVERNIGHT BRIEF: A BALANCE INSPECTOR AND FIVE SPECIALS.
  (2026-08-27, `overnight-agent3-moveset-and-specials`)**

`docs/planning/overnight-goal-agent3.md`, all six items. What landed, and what
each one is worth reading for:

**The moveset balance inspector** — `tools/ambition_moveset_inspector/`, plus two
binaries. `moveset_export` boots the composed host and reports what the SHIPPED
cast resolves to (46 fighters, 881 moves), not what an authoring file writes;
`moveset_takes` drives the nineteen repertoire presses through the real control
frame into a real seated match and records what the ENGINE did with them. Four
views: roster, one fighter's frame data with a timeline and a hitbox diagram, one
SLOT across the whole roster with median-absolute-deviation outlier flags, and
take playback. Durable feedback under `reviews/<character>/<move>.yaml`, keyed by
the stable id so *"the pirate's forward smash is too strong"* survives the tuning
pass it asks for; `--report` prints the standing notes for an agent told to
address them.

⭐⭐ AND IT IMMEDIATELY FOUND TWO THINGS NOTHING ELSE HAD: **D252** (the back air
is unreachable for the whole cast) and **D253** (`player_robot_v3` cannot throw
any of its five specials). ✔ **BOTH CLOSED 2026-08-27** — D252 at `1eb50e6e3`,
D253 at `22361bab3`; this line said *"both are open rows above"* for a day after
they were not.

**Pirate side-B** — `run_out_the_guns`. Draws the gun-sword (`MoveSpec::equips`,
whose timer is the MOVE'S OWN CLOCK), fires it (`MoveEventKind::Ranged` now
prefers the weapon a move drew), and bends the shot toward the nearest foe in the
commanded half-plane (`AimAssist`, a property of the WEAPON, applied at the one
place a shot's direction becomes world-space).

**Power ball** — `SmashChargeSpec::stores`. A full charge WAITS instead of firing
itself, and an interrupted one is banked for the next press. Decided in
`cancel_move_playback`, the one teardown path, from a fact the playback already
carries.

**Ponytail boomerang** — `ProjectileGameplay::accel`, one new primitive: a
constant world acceleration pointed back along the launch axis, which needs no
reference to the thrower and so stays inside the stepper's pure signature.

**Two teleports** — `smash.teleport` with a LEDGE ASSIST, the robot's and the
Author's, same technique and assist radius, different effect ids.

**Bomb down-B** — a `GroundItem` with a fuse, so the pick-up-and-throw half is
machinery the engine already installs. ⚠ its own module recorded a gap — only the
CONTROLLED SUBJECT could pick one up — ✔ **closed since, by R10 (`a901cdc2f`)**:
the pickup/throw/fire loop runs over `DrivenBodies`, the union of the controlled
subject and every `DrivingParticipant`, ordered by `SimId`. Four seats, four
hands.

⛔⛔ FOUR SCHEMA BUMPS IN ONE BRANCH, v122 → v126: `actor.move_brandished_item`,
`actor.stored_move_charge`, the projectile codec's `accel`, and
`smash.live_bomb`. Every one is declared in the baseline it belongs to, and the
codec-shape file was re-recorded — absorbing two files STALE ON MAIN that this
branch never touches, said out loud in that commit rather than laundered.

- ▣ **D253 — `player_robot_v3` HAS FIVE SPECIALS AND CANNOT THROW ANY OF THEM.
  CLOSED 2026-08-27 (`22361bab3`).**

✔✔ **AND THE DIAGNOSIS BELOW WAS WRONG — it is not flight posture.** GPT 5.6
found the real one and the take recorder confirms it: `sustain_bubble_shield`
(`avatar/starting_character.rs`) read the RAW `special_pressed` edge in
`PlayerInputSet::ControlGate` and forced `shield_held` because Robot's BODY KIT
names `bubble_shield` — before anything resolved WHICH directional special the
press meant. Grounded shield plus a direction is an evade; airborne on a Smash
body with no air guard arms the air dodge; and by the time `Combat` asked for the
special, the move was refused from the state that layer had just created. ONE
shared authority error reported as five broken moves.

⭐ IT ASKS NOW, WITH THE SAME TWO CALLS THE RESOLVER USES — `attack_dir_from_axis`
folds a facing into an aim, `move_for_directional_verb` picks the special.
Reimplementing either would have replaced one authority problem with two. A body
with NO repertoire still shields on any press: that is the ordinary body kit this
compatibility layer was written for.

```text
special           moves={"bubble_shield"}
special_forward   moves={"rocket_dash"}
special_up        moves={"phase_shift"}
special_down      moves={"stabilizer_slam"}
special_air_down  moves={"stabilizer_dive"}
```

Zero mismatches, against five that produced nothing. ⛔ THE ORIGINAL ROW AND ITS
FLIGHT-POSTURE SUSPICION FOLLOW, kept as the record of what the recorder saw:

Nineteen presses, driven through the real control frame into a real seated
match, one fresh match per take. Fourteen work. The five that do not are the
five specials, and they are ALL of them:

```text
attack .. smash_down .. attack_air_* .. grab .. taunt      all reach their move
special           intended=bubble_shield        got=None   grounded=true
special_forward   intended=rocket_dash          got=None   grounded=true
special_up        intended=phase_shift          got=None   grounded=false
special_down      intended=stabilizer_slam      got=None   grounded=true
special_air_down  intended=stabilizer_dive      got=None   grounded=false
```

⛔ NOT THE ENGINE, AND NOT THE PRESS. The same driver, the same tick budget and
the same re-seat get every special out of the other two fighters measured:

```text
npc_pirate_admiral   5 of 5 specials fire   (grapeshot, run_out_the_guns,
                                             call_the_shark, heave_to, heave_to)
author               5 of 5 specials fire   (…, author_revision)
player_robot_v3      0 of 5
```

⛔ AND FOUR OF THE FIVE ARE UNTOUCHED BY THE RUN THAT FOUND THEM. This branch
replaced the robot's `special_up` only; `bubble_shield`, `rocket_dash`,
`stabilizer_slam` and `stabilizer_dive` are as main has them, and they fail
identically. Whatever this is, it predates the change that surfaced it.

⭐ THE STANDING SUSPECT, unmeasured: this body can FLY, and a flying body is
never grounded by construction (`integration.rs`: *"a flying body is never
grounded — the collision sweep can still find support under a hovering
flyer"*). The take recorder had to be taught the same thing about its own
settle. A special resolves its posture at the press
(`SpecialGestureIntent`), and a body whose ground state is a permanent lie is
the obvious place to look first — but that is a hypothesis, and the four wrong
hypotheses D207 cost are the reason it is written down as one.

⭐ THE ACCEPTANCE TEST EXISTS: `moveset_takes --characters player_robot_v3`, and
the report line is `MISMATCH: drove <id> but the engine played {}`.

- ▣ **D252 — THE BACK AIR IS UNREACHABLE FOR THE WHOLE CAST. CLOSED 2026-08-27
  (`1eb50e6e3`).**

✔✔ **FIXED AT THE PRODUCER, NOT IN THE KERNEL — which is what this row asked
for.** `brain/player.rs` wrote a facing EVERY tick, grounded or not; it reaches
the movement kernel as `MotionStepContext::facing_intent` and is applied there
unconditionally, so an airborne fighter turned to face the back input before the
press was read. The human translator now steers facing only when the body may
actually turn (`actor_on_ground || actor_aerial`), which is the same carve-out
`movement/abilities.rs` already made with `ground.on_ground || flight.fly_enabled`
— the rule was written down one layer down and this contradicted it.

⛔ THE KERNEL IS UNTOUCHED, as the row required: `facing_intent` reaches
crawlers, bosses, surface-momentum riders and the recovery search whose own
comment deliberately says a body that cannot turn in the air still points where
it is trying to go.

Acceptance is this row's own instrument. Before:
`MISMATCH: drove air_back but the engine played {"air_forward"}`. After:
`moves={"air_back"}`, zero mismatches across all 19 takes.

⚠ GPT 5.6 reached the same diagnosis independently the same day. ⛔ THE ORIGINAL
ROW FOLLOWS:

Every fighter authors an `attack_air_back`. None of them can throw one.

`moveset_takes` drives the nineteen repertoire presses through the real control
frame into a real seated match and records what the engine did. Eighteen reach
the move they drove. The nineteenth reports:

```text
[take] npc_pirate_admiral  attack_air_back  moves={"air_forward"}
       MISMATCH: drove air_back but the engine played {"air_forward"}
       frame 0: facing=-1.0  gesture=Forward/Tilt/Airborne
```

⇒ the fighter TURNS TO FACE the back input before the press is read, so
`attack_dir_from_axis(axis, facing)` folds the reversal away and every back-air
press resolves as `Forward`. The move exists, is bound
(`attack_air_back -> air_back` in the verb table), has frame data, and is dead
content for all fourteen fighters.

⛔ TWO AUTHORITIES DISAGREE ABOUT AIR FACING, and the code says so out loud in
both places:

- `movement/abilities.rs` — `let can_turn = ground.on_ground || flight.fly_enabled;`
  i.e. an airborne body may NOT turn from the stick.
- `movement/recovery.rs:383` — *"The effort's own steering IS its facing intent:
  a body that cannot turn in the air still points where it is trying to go."*
  and `MotionStepContext::facing_intent` is applied unconditionally in
  `movement/kernel.rs:368`.

Both are defensible; they cannot both hold. The genre's answer is that an
airborne body keeps its facing and a back input is a BACK ATTACK — that is the
whole reason the slot exists — while the reverse aerial rush stays a GROUNDED
pivot resolved by the jump.

⛔ NOT FIXED IN THE RUN THAT FOUND IT. `facing_intent` is a shared movement
kernel input reaching every body in the game (crawlers, bosses, surface-momentum
riders, the recovery probe), and re-tuning air facing at 5am on an unattended run
would change how every character points while attacking. The finding is recorded
with its instrument: rerun `moveset_takes` after any change and the MISMATCH line
is the acceptance test.

⛔⛔ **AND THE INSTRUMENT SAYS IT DOES NOT REPRODUCE AT HEAD — measured
2026-08-28, on the acceptance test this row named.** Two characters, both
untouched:

```text
player_robot_v3      attack_air_back  moves={"air_back"}   no MISMATCH
npc_pirate_admiral   attack_air_back  moves={"air_back"}   no MISMATCH
```

and the engine's own log agrees — `move accepted: move=\`air_back\` grounded=false`.
⇒ **the back air is REACHABLE.** ⛔⛔ **AND THE ANSWER WAS FIVE ROWS BELOW THIS
ONE THE WHOLE TIME: D252 CLOSED 2026-08-27 (`1eb50e6e3`)** — fixed at the
PRODUCER, `brain/player.rs`, which wrote a facing every tick grounded or not. The
human translator steers facing only when the body may actually turn now. This
paragraph is the SYMPTOM REPORT that row was opened from, and nothing marked it
when the row closed.

⚠ **AND I ALMOST SHIPPED A GLOBAL RE-TUNE ON THE STRENGTH OF IT.** The two
authorities really do disagree in the source — `abilities.rs` computes
`can_turn = on_ground || fly_enabled` while `kernel.rs` applies `facing_intent`
unconditionally — so gating the kernel write on the same rule looked like the
obvious repair, and every suite stayed green with it applied (core 518, monolith
1141, combat 554, smash 39, app 499). **The CONTROL is what stopped it**:
disabling the gate and re-running the same take showed `air_back` playing either
way, so the change fixed nothing measurable while re-tuning how every body in the
game points in the air. ⇒ the contradiction is real and is worth resolving on its
own merits, WITH Jon, but it is not this defect's cause and green suites are not
evidence that a tuning change was needed. ⛔ **D252's own closure forbids exactly
the change I built**: *"THE KERNEL IS UNTOUCHED, as the row required"*, because
`facing_intent` reaches crawlers, bosses, surface-momentum riders and the
recovery search. ⇒ **a symptom paragraph and the row that closed it can live in
one file and never learn about each other** — when you close a row, mark the text
it was opened from.

- ◐ **D244 — THE LADDER'S CAP FORBIDS NOTHING; ITS REPERTOIRE DOES RISE, AND THE
  FIRST VERSION OF THIS ROW SAID OTHERWISE. (promoted from the intake 2026-08-26,
  corrected within the hour by the measurement that would falsify it)**

⛔⛔ **READ THIS FIRST: THE HEADLINE I OPENED THIS ROW WITH WAS AN ARTIFACT OF A
WINDOW LENGTH.** I measured `distinct_frames` flat across every rung in the
DECISION rig, concluded *"difficulty is a tempo knob and nothing else"*, and then
measured the half that would make it false — the same sweep in a REAL MATCH:

```text
                       900-tick window      2700-tick window
L1                      9 distinct moves     11
L9                      6 distinct moves     17  ← grabs, pummels, throws,
                                                   smashes L1 never touches
```

⇒ **at the short window the top rung looked NARROWER than the bottom**, and that
would have shipped as *"difficulty makes a CPU worse"*. A fast rung needs LONGER
to show its kit, because it spends more of a short window committed. ⭐ the whole
sweep at 2700, registered rungs only: **L1 11, L3 15, L5 17, L6 14, L9 17** — the
ladder broadens, roughly monotonically, with a dip at L6.

⇒ **and that also re-reads the decision rig's flat `distinct_frames`: it is a
property of a fixture that never steps a world** (a static `WorldView` and an
expected `Situation`), not a fact about the ladder.

⭐ **PROMOTED BECAUSE IT STOPPED BEING VAGUE.** `tracks.md`'s *"calibrate the
difficulty ladder through measured survival/damage/recovery evidence"* had no
definition of done. It has one now, measured at seed `0x5EED` over the FIVE
registered rungs (1, 3, 5, 6, 9 — `evaluation::report` walks 1..=9 and the other
four are generic fallbacks, which is now warned at the iteration site):

```text
L1  apm 12.7 / cap 120  = 10.6%    distinct frames 19
L3      21.3 /     195  = 10.9%                     19
L5      29.3 /     270  = 10.9%                     19
L6      34.0 /     308  = 11.0%                     21
L9      46.0 /     420  = 11.0%                     20
```

✔ **WHAT SURVIVES: the APM cap never binds.** Every registered rung runs at
10.6–11.0% of its own cap, uniformly, so the cap is a derived ~9x multiple of the
rate rather than a limit — moving one would change nothing. That is measured in
the decision rig and does not depend on any window.

⭐ **SO THE REMAINING QUESTION IS NARROWER THAN THE ROW OPENED WITH**: the ladder
already broadens, so *"calibrate"* is no longer *"make difficulty mean something"*
— it is whether the broadening is the RIGHT shape (L6 dips below L5 and L3), and
whether a broader rung actually WINS more, which only `ladder_rig` can answer.
⇒ the instrument for the first half is
`probe_repertoire_by_rung` (`the_repertoire_gets_used.rs`), which sweeps the five
registered rungs in real matches; ⛔ **its window is load-bearing and its doc says
so** — do not shorten it.

⚠ **AND THE L6 DIP IS NOT ESTABLISHED — read the SHAPE, not the steps.** The
sweep is ONE character over ONE run. The L1→L9 rise (11→17) is large enough to
survive noise; a single rung sitting three below its neighbour is not. ⛔ and a
second character cannot settle it in that harness: `build_demo_app` composes only
the demo's own cast, so `npc_pirate_admiral` fails the seat-count assertion
rather than reporting anything (measured). ⇒ **sweeping characters needs the
COMPOSED host** (`game/ambition_app`'s harness), which is where a follow-up
belongs.

⚠ **AND THE TWO RIGS ANSWER DIFFERENT QUESTIONS — do not conflate them.**
`brain::fighter::{evaluation, scenarios}` is a DECISION rig: a static `WorldView`
plus an expected `Situation`, no world stepping, so survival and damage are
unfillable there BY DESIGN. `ladder_rig.rs` is the OUTCOME rig: CPU-vs-CPU
matches reporting time to elimination, stocks remaining and engagement evidence
across deterministic seeds, with `--weight` for refitting the scorer. ⇒ the
repertoire claim is `evaluation`'s to measure; whether a broader rung actually
WINS more is `ladder_rig`'s.

⛔ this is a change to what a RUNG IS, not a number to retune, so price it as a
slice rather than a tuning pass.

- ✔ **D243 — A NON-DEFAULT FEATURE IS WHERE CODE GOES TO ROT, and one of them
  had stopped compiling. (opened AND closed 2026-08-26)**

⇒ **CLOSED the same day, because the audit turned out to be reachable.** `causal`
was broken and is fixed; the shell's fixture is fixed and 26 tests came back; the
`visible,desktop_platform` persona checks clean; the WEB persona checks clean on
`wasm32`; and our Android Rust turns out to be compiled and tested on the HOST
already. ⚠ **what is NOT closed is the habit** — the table below is the reason
this row exists, and it is the thing to re-read before editing a type that a
gated module consults.

⛔⛔ **`--features causal` DID NOT BUILD**, and had not for long enough to collect
four separate breaks: three references to `StocksMatchDecided.winner` after that
message became `outcome: MatchVerdict`, a `BodyReaction` construction missing the
`hitstun` field, two test fixtures on the old field, and an expectation spelling a
`HitSource` variant that no longer exists. ✔ FIXED — and the winner repair was
not mechanical: the instrument was still asking `winner: Option<String>` with
`None` meaning DRAW, which is exactly the conflation `MatchVerdict` exists to
remove, so an ABANDONED match had nowhere to go but to impersonate a draw.

⭐ **WHY NOTHING SAW IT: the standing gate builds one feature set.**
`cargo check -p ambition_app --all-targets` compiles what the APP enables, and a
per-crate `cargo test` compiles that crate's DEFAULTS. Anything behind a
non-default feature is compiled by neither.

```text
                    default / with features        note
ambition_input          56 / 125  (--all-features)  pass — VISIBILITY hole
ambition_dialog         30 /  42
ambition_items          27 /  39
ambition_encounter      34 /  41
ambition_sfx             8 /   8                    the only one with none
ambition_game_shell     45 /  72  (basic_presentation)  its 10 pause tests could
                                  not pass — fixture omitted a message the SHELL
                                  plugin owns; ✔ fixed, 26 tests came back
ambition_conversation   25 /  35  (ui)              a poisoned `Truth` arm came
                                                    back GREEN by default
ambition_combat        327 / 332  (causal)          ⛔ DID NOT COMPILE; ✔ fixed
monolith              1194 / 1202 (causal)          ⛔ same; ✔ fixed
ambition_demo_smash    --lib red for days           the gate runs the APP target
```

⛔⛔ **AND THE BIGGEST HOLE IS NOT A CARGO FEATURE AT ALL — IT IS THE FOUR PYTHON
SUITES. Swept 2026-08-26: ELEVEN RED TESTS, in three of the four.**

```text
tools/ambition_sprite2d_renderer   685 pass  15 skip   green (incl. the codegen
                                                       comparison D129 called
                                                       "left red on purpose")
tools/ambition_sfx_renderer         2 FAIL → 14 pass
tools/ambition_ldtk_tools           5 FAIL → 236 pass
tools/ambition_music_renderer       4 FAIL → 218 pass, 3 skip
```

⭐ **AND EVERY ONE WAS A TEST DESCRIBING A WORLD THAT HAD MOVED**, not a broken
tool:

```text
sfx     two cue tests asked for a bare `jump` / `death` after all 490 cues
        were namespaced; retargeted to `player.jump` / `player.death`, which
        also straddle the 300ms wav/ogg policy boundary
ldtk    the spec-brain guard pointed at `character_archetypes.ron`, DELETED by
        `74bd5e9ae Delete the enemy-archetype ontology`, so every test in the
        file died on `FileNotFoundError` rather than reporting. Retargeted to
        the catalog's `autonomous_profiles` + `brain_presets`; it passes for the
        RIGHT reason — 18 placements still name deleted archetypes and every one
        carries `character_id`, which is what its own rule permits
ldtk    the silent-default test used `LoadingZone.activation`, and the converter
        was FIXED to refuse; retargeted to `KinematicPath.mode`, one of nine
        fields that still declare `silent_default`
music   three tests patched `subprocess.run` after `_render_sfizz_cli` moved to
        `Popen` + a heartbeat, so they recorded ZERO calls and every argv
        assertion was unreachable; one compared an order that needed two files
        written microseconds apart to have distinguishable mtimes
```

⛔ **plus one defect in the SHIPPED contract, found by the ldtk guard the moment
it could run**: `LockWall.gated_by` carried `"kind"` and `"required"` — the only
field of 46 to do so, and the only one missing `presence`/`sample`. Rust's
`FieldContract` has neither key and no `deny_unknown_fields`, so serde dropped
them silently. Replaced with the vocabulary both readers know.

⇒ **nothing in `cargo` would have told anyone about any of it**: `python -m
pytest` in each of those four directories is its own gate, and it is not in
anybody's habit.

▢ **WHAT IS STILL UNAUDITED, and it is the expensive half**: the PERSONA
features — `android_platform`, `web_platform`, `visible_web`, `web_served` — need
their cross-compilation targets, so a host `cargo check` cannot answer for them.
⚠ they are also the ones a broken build hurts most, because nothing else builds
them either. ⇒ the honest next step is one host-buildable persona at a time
(`--no-default-features --features visible` first), not a workspace-wide
`--all-features` sweep, which enables mutually exclusive platform features and
fails for reasons that mean nothing.

⛔ **AND DO NOT SPEND A BUILD ON BARE `--no-default-features`: it reports 8
errors and they mean NOTHING.** Measured 2026-08-26 — every one is in
`input_systems.rs` naming `ActionState`, `Platformer2dInputActionMonolith`,
`SeatBurstTriggerState`, `input_suppressed_by_unfocus`,
`read_menu_control_frame`, all of which live behind `feature = "input"`. That is
the crate correctly refusing a configuration it never claimed to support:
`visible` lists `"input"`, `android = ["visible", …]`, and the Cargo comment says
Android builds pass `--no-default-features` WITH the `android` composite, never
bare. ⇒ the meaningful minimum is a PERSONA, and `visible,desktop_platform` is
the only one a host can compile.

✔ **AND THAT PERSONA IS CLEAN — measured 2026-08-26:
`cargo check -p ambition_platformer2d_actor_monolith --no-default-features
--features visible,desktop_platform` reports ZERO errors.** So the persona road
is healthy where a host can see it, and `causal` was the one that had rotted.
⇒ **the host-checkable half of this row is DONE.**

⭐⭐ **AND THE "EXPENSIVE HALF" IS REACHABLE AFTER ALL — BOTH CROSS TARGETS ARE
ALREADY INSTALLED.** `rustup target list --installed` reports
`aarch64-linux-android` and `wasm32-unknown-unknown` beside the host, so the
persona builds this row called unauditable are a `--target` away:

```text
cargo check -p ambition_platformer2d_actor_monolith \
  --target wasm32-unknown-unknown --no-default-features --features web
cargo check -p ambition_platformer2d_actor_monolith \
  --target aarch64-linux-android --no-default-features --features android
```

⚠ each is a COLD build of the whole dependency tree for a new target, so budget
for it — but *"nothing builds them on a normal run"* was the condition that let
`causal` rot, and these two commands are the whole answer to it.

✔ **THE WEB PERSONA IS CLEAN — run 2026-08-26, zero errors.**
`--target wasm32-unknown-unknown --no-default-features --features web` compiles.
⇒ so the browser build Jon ran by hand is not silently rotting between his runs.

⛔⛔ **AND THE ANDROID ONE IS NOT AUDITABLE HERE, WHICH CORRECTS THE PARAGRAPH
ABOVE: A RUSTUP TARGET IS NOT A TOOLCHAIN.** The `aarch64-linux-android` target
is installed and the build still fails — in `android-activity`'s BUILD SCRIPT,
not in our code:

```text
warning: Compiler family detection failed: ToolNotFound:
         failed to find tool "aarch64-linux-android-clang++"
error:   failed to run custom build command for `android-activity v0.6.1`
```

⚠ **and the environment LOOKS ready, which is the trap**: `ANDROID_NDK_HOME` is
set to `/home/agent/Android/Sdk/ndk/27.2.12479018` and **that directory does not
exist** (`ls` on it is empty). A set variable pointing at nothing reads as
configured. ⇒ `wasm32` needs nothing beyond the rustup target because it is pure
Rust; `aarch64-linux-android` needs the NDK's clang, and
`scripts/setup/android_prereqs.sh` is what installs it (a large download).
▢ **so the android persona stays unaudited, and the blocker is a PREREQ RUN
rather than a code question** — say that rather than reporting our own code as
broken on Android. ⇒ `scripts/setup/android_prereqs.sh` is the repo's own path
and it installs the NDK; ⚠ it is a large download, so it is a deliberate spend
rather than something to do in passing.

⭐ **AND THERE IS A CHEAPER ROUTE THAT NEEDS NO NDK AT ALL, already written down
in `dev/journals/android-what-an-agent-cannot-see-2026-08-08.md`**: to typecheck
Android-only Rust, retarget its `cfg` gates to the HOST and compile there — then
POISON the probe (inject a type error and confirm it surfaces) before trusting
the green, because a `cfg` that silences itself compiles nothing and looks
identical to a clean build. ⇒ that covers OUR code, which is the half this row
cares about; the NDK is only needed for the C++ dependency's build script.

✔✔ **AND MEASURING IT SHOWED THE RETARGET IS NOT EVEN NEEDED: OUR ANDROID CODE
IS ALREADY COMPILED AND TESTED ON THE HOST. 2026-08-26.**

```text
`cfg(target_os = "android")` sites        16, in 6 files
game/…/host/platform/mod.rs:10            `pub mod android;`  — NOT cfg-gated
game/…/host/platform/android.rs           451 lines; imports only bevy and our
                                          own crates — no `android_activity`,
                                          no `jni`, no `ndk`
its tests                                 4, and they RUN: `cargo test -p
                                          ambition_app --lib android` passes
```

⇒ **the 16 gates are small branch points; the substantive module is
unconditional**, so the standing gate typechecks it and its suspend/restore rules
are guarded on the host. ⛔ **what the NDK would buy is the C++ dependency's
build, which is not our code** — so this row's android half is CLOSED for the
question it was asking, and *"cargo check --target aarch64-linux-android dies"*
is a fact about `android-activity`, not about us.

⛔ **AND THE ANSWER IS NOT A CHECKER.** `AGENTS.md`'s *"avoid bullshit
guardrails"* is binding and a feature-parity test is exactly that. The answer is
that whoever edits a type consulted by a gated module has to know the module
exists — which is why the table above is written down rather than automated.

- ◐ **D242 — NINE PARTICIPANT/ACTION ARCHITECTURE ITEMS WERE REACHABLE FROM THE
  INTAKE AND FROM NO LEDGER ROW. (promoted 2026-08-26)**
  ⭐⭐ **THE ITEM `tracks.md` NAMES IS HALF DONE, 2026-08-28: a provider action now
  reaches a keypress and comes back.** Register an action the engine has never
  heard of, bind it, press the key, get `SemanticActionPressed` — no `Any`, no
  `TypeId`, no variant added to the 35-variant enum, checked end to end by
  `a_registered_action_bound_to_a_key_comes_back_as_a_seat_press`. Three pieces in
  `ambition_input`: `ProviderBindings` (the composition's map, kept separate from
  the registry because a capability DESCRIBES an action and the game it is
  installed into decides its key), `install_provider_bindings_on_seats` (a sync,
  so a capability installed after the seats exist still reaches them) and
  `publish_provider_action_edges` (`just_pressed`, sorted by seat id).
  ▢ **what remains is PRESENTABLE**, and re-measuring changed its shape:
  `ControlSlot` and `TouchActionButton` are DESCRIPTIONS OF HARDWARE, not
  arbitrary limits, so a provider action becomes presentable by being ASSIGNED a
  slot rather than by widening one — a design call about a finite pad or a finite
  screen. ⛔ do not price that as plumbing; the plan's *"presentation is the
  expensive road"* was measuring a mapping table that points the other way.

⭐ **PROMOTED, NOT WRITTEN.** [`engine/participant-action-system.md`](engine/participant-action-system.md)
carries nine `▢` items and is named by `tracks.md`'s *"Provider-defined actions
through the full physical/UI seam"* — so it was reachable from the RESERVOIR and
invisible to the EXECUTION AUTHORITY, which is the same shape as the seven
Engine 1.0 plans stranded on 2026-08-14. The design is already written; this row
is the pointer.

⛔⛔ **AND THE DOC'S OWN HEADER SAYS `cecd01ca` (2026-08-13), which is a fortnight
of HEAD ago. Re-grep every item before working it** — that is the rule this
ledger keeps paying for, and two of the nine are already suspect:

✔✔ **AND RE-READING CLOSED ONE OF THE NINE WITHOUT BUILDING ANYTHING
(2026-08-28).** *"Remove the seat-0 control split"* asked whether the split was
still a split or had become a stated design. It is neither: the split is GONE.
`drive_slot_frame` is the one delivery road — `input_drive.rs` says seat zero's
latch *"has since become row zero of the same table every other seat uses"* — and
`input_systems.rs` iterates `0..MAX_SLOTS`, its own comment recording that the
interact buffer *"was `slot_gestures.primary_mut` too"*. What still names
`PlayerSlot::PRIMARY` is a FALLBACK for a body no participant drives, and the
keyboard-belongs-to-seat-zero policy is the stated design the row's own test
(*"not merely for naming symmetry"*) says to leave alone. Four `▢` remain in the
plan.

✔ **AND TWO MORE MOVED 2026-08-28, both because the 2026-08-26 MEASUREMENT WAS
ITSELF STALE.** This is the row's own rule turned on the row's own notes.

*"Unify semantic menu activation"* was recorded as *"the seam is not missing, it is
UNADOPTED — one adopter, and `pause_menu` consumes raw `select` itself"*. Both
halves are false at HEAD: `shell_pause_menu_pointer` reads
`MenuActionActivated<PauseEntry>`, and `ambition_menu`'s pointer bridge already
runs on the SAME `ambition_ui_nav::PressArm` the one named adopter uses. What
actually diverged was the POLICY above that shared gesture, and it cost something
real: `MenuTapMode` ships defaulting to `SingleTapWithDestructiveGuard`, whose own
doc names *"a stray touch on Quit"* as its reason, and only the index-addressed
helper ever consulted it — so Abandon / Quit to Title / Quit to Desktop all fired
on the first release. ✔ ONE POLICY NOW: `resolve_press` is generic over an opaque
row identity (the `usize` was never an ordinate), and a menu declares its risky
rows once with `MenuDestructiveActions`. The non-pointer half stays open with the
feel question named — a controller cannot stray-tap the way a thumb can.

*"Finish context migration"*'s `VEHICLE` question is ANSWERED: no variant. The
verbs do differ, so the row's own test is met, but the prompt is not built from the
context — `rebuild_control_prompt` derives it from the subject's live authorities,
so a variant would have been a second answer to a question one authority already
owns. ⛔ the defect underneath was a four-button PROMPT LIE: `body_step` clears every
`MovementAction` for a body under `PoseOwnedExternally`, nothing touches
`AbilitySet`, and a rider on the shark was drawn Jump / Burst / Blink / Fly beside
the Attack that actually works from the saddle. Fixed by masking the authority
(`AbilitySet::while_pose_is_held`) in the prompt and NOT in the routing gate, where
a buffered press is input memory the player is owed. ⚠ the row's loading/retry half
inherited the same stale framing and has been re-scoped to the SCHEDULE seam it
names first.

```text
"Remove the seat-0 control split"   CLOSED by reading HEAD — see above.
"Per-seat pause ownership"          `SeatMenuFrames` shipped and the select
                                    screen consumes it per seat; what the doc
                                    calls missing is menu OWNERSHIP state, not
                                    the channel
```

✔✔ **ALL NINE SWEPT THE SAME DAY, and the doc now carries the measurement for
each — 2026-08-26.** Two were already SHIPPED (per-seat pause ownership, with a
guard at `pause_menu.rs:788`; pad-specific calibration filtering, on BOTH the
menu and gameplay roads). Two are partly done with the remainder named
(`ambition_ui_nav` — ✔ **the SHELL adopted it 2026-08-26**: the launcher had
hand-rolled `checked_sub(1).unwrap_or(len - 1)` and `(selected + 1) % len`, the
same wrap `ListCursor` implements and the pause menu had already disagreed with
by clamping. Two menus agreeing by coincidence is one edit from two menus
disagreeing. ⚠ what remains is the OTHER adopters; dialogue-per-seat — the explicit policy
shipped, and the OTHER half is inexpressible because `allows_gameplay` is a bare
`matches!` that never asks it). Three were sharpened from prose into counts (the
activation seam has ONE adopter; `ControlContextKind` has four variants and no
`VEHICLE`; provider actions are describable but unbindable behind a 35-variant
enum with 288 references in 21 files). One was re-framed (the seat-0 split is now
DEFENDED, so the question changed).

⇒ **what a reader should do first**: take one item, grep for the thing it says
is missing, and either update the doc or work it. ⛔ do not price the nine as a
campaign — the doc is a list of independent seams, and three of them (dialogue
per seat, `ambition_ui_nav`, context migration) each touch a different subsystem.

⚠ **the ninth item is the one `tracks.md` actually names** — *"a provider action
should be registerable, bindable, presentable, and consumable without editing
core action vocabulary"* — and it is the one with a measured blocker already in
the doc: the registry exists (`SemanticActionId`, `ActionRegistry`,
`InstalledActions`, `ModuleDraft::actions`, a tested external `grapple`), and the
physical input map / cue / touch path still bottoms out in the finite built-in
platformer action enum. That enum is the deletion gate.

⭐⭐ **AND IT NOW HAS A COMPILED CANDIDATE, which it never had. 2026-08-26.**
`InputMap<A: Actionlike>` is already generic, so a composition can install a
SECOND map keyed by a type a provider MINTS, and a registry-minted key satisfies
the whole `Actionlike` bound —
`a_registry_minted_key_satisfies_leafwing_without_erasure` binds one and reads it
back, and poisoning it reports *"a provider-minted key bound nothing"*. ⛔ no
`Any`, no `TypeId`, no service locator, and no edit to the 35-variant enum, which
is the combination every refused answer failed. ⚠ running it also found the price
the reasoning missed: neither the registry's `ActionControlKind` nor leafwing's
`InputControlKind` can be a field of a hashed, reflected key, so the kind gets a
small three-variant mirror. ⇒ **still a carve — two maps means two reader paths
and a rule for which wins — but the question is now "how" rather than
"whether".**

- ▣ **D255 — CHECKPOINT 5 (HEAD `a945c1de5`): EIGHTEEN ITEMS, AND THE TWO THAT
  MATTER MOST ARE PROOFS THAT ARE FALSELY CLOSED. CLOSED 2026-08-27, all
  eighteen.**

✔✔ **BOTH FALSE PROOFS WERE RETIRED BY MEASURING.** The shark-health census
scans `SmashRoster::assemble`'s 21 fighters — melee worst 36, ranged worst 14
at full charge, against a 40 HP pool — instead of a 13-row authoring table; the
mounted-launch arm asks for EQUIVALENCE and both roads produce
`(-1884.0382, -1884.0382)`, where reverting the deferral releases the mounted
one at exactly `(0, 0)`.

⭐⭐ **THE RECOVERY EPISODE WAS THE BLOCKER AND IT WAS EVERYBODY'S.** The
landing-class refresh asks `on_ground` every TICK, so a recovery spent while
still standing came back one frame later — for every fighter, not just the
pirate. `MotionStepContext::recovery_commitment_outstanding` is derived from the
`MovePlayback` each tick rather than stored, so no second copy can disagree with
the move it describes.

⭐ **AND D250 CAME WITH IT.** Recovery admits route KINDS now — burst, sustained
authority, teleport — because a planner that modelled one thrown velocity could
not see a summoned steerable shark or a teleport, both of which read
`lift_speed == 0.0`. `RecoveryLift` is deleted.

⚠ THE MOST USEFUL FINDINGS WERE NOT ON THE LIST. Three fixes came from tests
written to check something else: a buffered evade spent inside a saddle, three
projectile fixtures hand-listing what production spawns (now `#[require]`d), and
a body treated as a wall parking a dropped item 47px above its floor. ⛔ And one
went the other way: `PoseOwnedExternally`'s speculative harms DO NOT HAPPEN —
measured over 299 mounted ticks — so the kernel decomposition it seemed to call
for would have repaired a defect nobody has. The row list follows:

The reviewer re-read main after checkpoint 4's sixteen fixes landed and reported
what survived, plus what the fixes themselves opened. ⭐ THE ROWS ARE IN
[`triage/gpt-review-2026-08-27-remaining.md`](triage/gpt-review-2026-08-27-remaining.md)
with their acceptance criteria; this row is the ledger's handle on them and
closes when that file has no `▢` left.

⚠ THE HEADLINE IS NOT A FEATURE. Two tests are green about a subject that is not
what their names say — the shark-health census scans a hand-kept authoring table
while claiming the selectable cast (R3), and the assembled mounted-launch
assertion stays green with the deferred-launch fix REVERTED because another
source supplies lateral motion (R2). Those outrank every new mechanic in the
file, because a false proof is worse than no proof: it retires the question.

⭐ THE SHARK RECOVERY EPISODE (R1) IS THE BLOCKER. `call_the_shark` commits a
recovery charge and then the ordinary GROUNDED refresh hands it straight back,
because the move starts on the floor and the shark takes movement authority a
moment later. The intended rule — a flinch while mounted refreshes recovery —
therefore cannot be tested at all: the charge was never spent by the time the
hit lands. ⛔ NOT FIXABLE BY REMOVING GROUNDED REFRESH; a grounded Up-B that
stays on the floor still needs ordinary landing-class semantics. The episode
needs representing, which is the same seam D250 needs for sustained-authority
recovery ROUTES (R13) — one model, not two.


- ✔ **D241 — CHECKPOINT 4 (GPT 5.6, HEAD `42e894b`): TWO REGRESSIONS THE FIXES
  THEMSELVES INTRODUCED, AND ONE OWNERSHIP LESSON THEY SHARE. Opened 2026-08-25,
  CLOSED 2026-08-28 when the last owed item — recharge presentation — landed.**
  ⚠ the row header outlived its content by three days: everything under it read
  CLOSED or SHIPPED except one `▢`, and one line of a status table still said the
  wavebounce recogniser was open three days after it shipped in the paragraph
  below it. ⇒ a row's own body is where its status lives; the marker on the header
  is a summary, and summaries go stale first.

⭐⭐ **CLOSED — THE MATCH CLOCK'S FIX BROKE ITS PERIODIC READER (`43fe5ea`).**
Counting SCALED gameplay is right, but the projection back to 60 Hz
(`micros * 60 / 1_000_000`) then no longer advances every step: at half speed one
conceptual tick lands on two consecutive steps, so the item spawner's
`elapsed % every_ticks == 0` fired on both — two items, and two entities deriving
the SAME `SimId`. A determinism defect, not double loot. `LiveMatchTicks::crossed`
answers the question the spawner means, from BOTH ENDS of the step, and returns
the boundary's ORDINAL; the identity derives from that, because an ordinal counts
boundaries and cannot repeat however time is scaled. ⛔ THE OLD TEST MODELLED THE
RULE LOCALLY (`elapsed % every == 0`) and agreed with itself; driving the real
clock at half speed reports `[1, 1, 2, 2, 3, 3, …]`. ⚠ no shipped ruleset turns
items on (`roster.item_spawns = None`), so this was unreachable in play.

⭐⭐ **CLOSED — DOUBLE-JUMP CANCEL HAD THREE DEFECTS (`bd0a8a7`).**
`air_jump_rising` was a MAGNITUDE standing in for OWNERSHIP: *"an air jump was
spent at some point AND I am rising no faster than one could push me"*. The
resource half stays true for the whole airtime, so a fighter launched upward
below `double_jump_speed` read as riding its own jump and an aerial DELETED the
launch — measured with the old derivation restored, a launch at half jump speed
handed the body **222.5** of "owned" rise. It is an AMOUNT now
(`air_jump_rise_owned`), granted only by the SPEND and only ever shrunk after it,
so a launch cannot grow it back. Also fixed: the consumer shed `kin.vel.y` (the
rise axis only under screen-down gravity), and it mutated velocity BEFORE
`cancel_permits` — a rejected attack changed physics. Wire format v104.

⭐⭐ **CLOSED — CHARGE SHOT'S CADENCE, AND IT PAID NEITHER MEASURED COST.** Jon
ruled (via GPT) that the two options were bundling THREE independent variables —
firing cadence, animation commitment, locomotion freedom. The refire floor moved
OUT of the projectile consumer and became `RangedActionSpec::refire_s`, checked
in `moveset::weapon_ready` where the move is ACCEPTED and spent in `start_move`;
`ActionRequest::Ranged` carries a `RangedCommitment` so a controller ATTEMPT
still meets the floor and a `CommittedMove` is guaranteed its shot. Measured on
`duel_arena`: **PCA melee 36, robot melee 23, hp 60→34 / 60→51** — more melee
than either rejected option, because a refused move no longer costs a windup
(the gate runs before `proposer.spend`, so the ordinary buffer re-proposes).
✔ **PAID 2026-08-28: the presentation half of the ruling** — *"give recharge
enough presentation that an unavailable shot is legible"*. `PromptEntry::ready`
carries it and the touch overlay dims the button; the fact comes from
`BodyMelee::ranged_cooldown`, the body's OWN fire-rate floor, so a dimmed button
and a refused press cannot part company. ⛔ **A BOOL, NOT A FILL FRACTION**: the
prompt rebuild sits behind a change-detection cache built to skip quiet frames,
and a fraction changes EVERY frame while a weapon recharges — the ruling asks
that the shot be legible, not metered. ⛔ and it is written by
`project_prompt_readiness` AFTER the derive rather than inside it, for the same
reason. ⭐ **DIMMED, NEVER HIDDEN**: a button that vanished would take the key
with it. ⚠ the guard's second arm is the load-bearing one — `ranged_cooldown` is
orthogonal to melee (invariant I3), and *"the ranged button went grey"* and
*"every button went grey"* look identical from one assertion. Poison-verified
both ways, and the fixture needed a `ranged` verb before it measured anything at
all.

⛔⛔ **#20 IS NOT A THIRD MAGNITUDE BOUND, AND MEASURING IT NAMED THE BLOCKER.**
The dash floor (`along.abs() > want.abs()`) and the shield brake
(`along.abs() <= max_run_speed`) are the same magnitude-as-ownership shape the
double jump just shed. The representation they want ALREADY EXISTS —
`BodyFlightState::carried_run` is *"signed run-axis velocity carried from the
world"* and `hit_reaction` sets it on every launch — but `carried_hold` is set to
`hitstun_timer` and the floor is ZEROED the moment it expires, so by the time a
dash or a brake can be commanded the fact reads `0.0`. Reading it there would
therefore make things WORSE: a body at 1300px/s with a surrendered floor would
brake to a stop.

✔ **ROUTED 2026-08-26 — it is [decision 34](awaiting-maintainer-decision.md),
and it has THREE consumers, not two.** The same guess is made a third time by
`body_contact` (*"faster than one walk-tick is not walking"*), which D179 closed
as a feel question on the same day without either row noticing the other.

⛔⛔ **AND THE PARAGRAPH BELOW IS WRONG ABOUT TUMBLE — corrected 2026-08-26 by
running `match_report`.** `tumble_speed` is **500.0** on the Smash stage
(`ambition_demo_smash/src/lib.rs:275`) and a real match spends 121–401 ticks
tumbling. *"The only 500.0 in the tree is a unit-test fixture"* was true of
`ambition_platformer2d_core` and false of the game. ⇒ #20 is not blocked on
waking a dormant mechanic; it is blocked on whether the shared kernel may read a
fact one game authors and the other pins at zero.

⇒ **AND THE GENRE'S OWN ANSWER TO "you were launched hard" IS TUMBLE.**
~~WHICH NO SHIPPED BODY AUTHORS. `tumble_speed` is `0.0` in `DEFAULT_TUNING`; the
only `500.0` in the tree is a unit-test fixture, and neither `ambition_characters`
nor `ambition_content` names the field. So the two bounds are standing in for a
DORMANT mechanic.~~

⛔⛔ **STRUCK 2026-08-26 — THE GREP LOOKED IN THE WRONG PLACE.** It searched the
ENGINE crates for a value the GAME authors: `ambition_demo_smash/src/lib.rs:275`
sets `tumble_speed: 500.0` on the STAGE, and `match_report -- 30 --runs 3`
measures 121–203–401 tumbling ticks a run. `DEFAULT_TUNING` really is `0.0` and a
test really does pin it — for AMBITION, whose wandering enemies would all change.
⇒ tumble is LIVE in one game and pinned off in the other, and the bounds live in
the kernel both use. That is [decision 34](awaiting-maintainer-decision.md), and
it now carries a cheap third answer neither "dormant" nor "wake it" allowed.

⭐⭐ **CLOSED — #28, THE PIVOT MIRRORED THE MOVE THE OLD WAY.**
`resolve_attack_gestures` resolves the attack DIRECTION against `-kin.facing`
while the body is turning — that is what makes a pivot grab need no move of its
own — but the body still HOLDS the old facing and `start_move` snapshots it into
the playback, which is what every hit volume is mirrored by and every
`start_impulse` multiplied by. The right move came out pointing backwards. ⛔ THE
EXISTING ARM COULD NOT SEE IT: it asserts which move STARTED, which was always
right. Committed where the move starts, so a refused press turns nobody.

⭐⭐ **CLOSED — #19, A ROOTED MOVE HANDED BACK A FREE DASH.** The initial dash
remembers direction by comparing this tick's stick with last tick's; a
`motion_scale: 0.0` window scales the stick to zero, so a player who simply HELD
a direction through an attack was recorded as neutral for its whole duration and
the frame it ended read as *"pressed from nothing"* — the exact edge that arms a
full-speed dash. `InputState` carries `undamped_axes` now: the dash still ARMS on
what the body may act on (a rooted body cannot dash out of its own recovery) and
REMEMBERS what the player was holding. Recorded in `damped_by_move_motion`, the
one function that knows the value is about to be lost, so a third road that
forgets carries `None` — which is correct. Not on the wire.

⇒ **THE RULE THE LAST FOUR FIXES SHARE, and it is worth more than any of them:**

```text
proposing an action must not mutate simulation state
forbidding an action must not erase the state that reads the next input
```

◐ **STATUS ADJUSTMENTS GPT ASKED FOR, recorded rather than reopened:**

```text
148c158 special-turn   acceptance mutation CLOSED · gravity-relative CLOSED
                       real wavebounce RECOGNIZER shipped 2026-08-25 (below);
                       the composition was smaller than the ledger assumed
56fb9da settled item   wakes when unsupported CLOSED
                       rides a moving platform CLOSED — and it needed NO support
                       identity: `Block::velocity` IS the per-frame displacement
                       and the support probe already finds the block
42e894b windbox guard  behaviour fixed; `flinchless ⇒ bypasses shield` is a
                       PROVISIONAL conflation — carry an explicit guard policy
                       when #29b touches that channel
```

⭐⭐ **THE WAVEBOUNCE RECOGNISER — SHIPPED 2026-08-25, AND IT IS TWO TOGGLES,
NOT THREE TECHNIQUES.** Every previous framing listed three named inputs and
asked how to tell them apart. They are one rule composed:

```text
each qualifying input FLIPS THE FACING;
a post-press flick ALSO reverses the lateral drift.

back BEFORE the press              flip                    → turnaround-B
back flick AFTER the press         flip + reverse drift    → B-reverse
both                               flip twice (= no flip)
                                   + reverse drift          → WAVEBOUNCE
```

⇒ the fourth outcome falls out of the other two rather than needing its own
recognition, which is why this is small.

⛔⛔ **AND THE FLICK MUST BE READ OFF THE HELD STICK, WHICH THE FIRST VERSION
GOT WRONG.** `update.rs` publishes the POST-INTEGRATION frame back onto
`ActorControl`, so an actor's `locomotion` reads ZERO for the whole of a rooted
move — and `motion_scale: 0.0` is how this repository authors a commitment. A
B-reverse would have been impossible on exactly the specials that most want it.
⇒ `ActorControlFrame::steer_axis()`, the twin of `InputState::steer_axis()` added
the same day for the same reason on the other side of the seam. The fixtures
deliver a DAMPED frame now, so the arm cannot pass by accident.

⭐ WHAT IT COST, as designed: `special_turn_window` + `prev_lateral_sign` on
`AttackGestureState` (already per-body rollback state, already the gesture-history
home), armed where the move is ACCEPTED — a press that starts nothing turns
nobody — plus `apply_special_turn_flicks`, which flips facing and reverses the
lateral drift on the flick that spends the window. Wire v111. The window is the
ruleset's own `flick_window_ticks`, not a new knob: it is already how long a
flick and a press count as one intent.

⛔ AND `special_turn_reverses_drift` CHANGED MEANING, as recorded. It reversed on
every back-special — the B-reverse final state applied unconditionally, which is
why one gesture could not choose. It gates what a FLICK buys now. ⇒ THREE ARMS
ASSERTED THE OLD READING and were REPAIRED, not worked around: they were rule
combinations, and the four outcomes are input orders. The mislabelling GPT
flagged went with them — `(turn, drift)` was never "B-reverse vs wavebounce".

⛔ THE DRIFT REVERSAL IS STILL THE BODY'S OWN SIDE AXIS, and the rotated-gravity
arm now drives the FLICK, because the flick is the half that reaches for the
velocity.

⭐ ACCEPTED CLOSED by the reviewer with no further defect: `9d96948` (seat/device
authority — *"the strongest fix in the window"*), `0e766bd`+`6d0ed2b` (roll
lifecycle, all five separations), `ddd0417`, `f941700`, `4f1d885`, `23281e5`,
`e970dd4`, `cec4f95`, `ba481b3`, `fd223a0`.

## Standing continuation rule

**This file is a continuation LEDGER, not a terminal checklist.** There is no
"the queue is empty, therefore stop" state: an empty executable list is a signal
to re-measure HEAD and refill, never a completion condition.

When the executable rows above close, **do not stop**. Re-read HEAD and promote
the next highest-value verified card from [`tracks.md`](tracks.md), a new direct
maintainer direction, or a reproducible maintainer observation.

Prefer in order:

1. Ambition flagship needs that create reusable engine capability;
2. Engine-1.0 ownership/composition/authoring work;
3. serious secondary game/acceptance pressure such as Smash or TwinTrack; and
4. deferred/trigger-based work only when its trigger is present.

Do not add meta-work merely to keep the queue nonempty. The queue continues by
finding real product or architecture work, not by manufacturing process.
