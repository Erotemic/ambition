# Boss cinematics, GNU2's finale and the giant's motion

**Owner of:** [BOSS-CINEMATIC-COVERAGE](../queue.md#boss-cinematic-coverage--give-every-boss-a-wolfenstein-class-name-intro---done-2026-11-17), [GNUTON-GNU2-FINALE](../queue.md#gnuton-gnu2-finale--summon-a-second-gnu-through-a-real-portal---done-2026-11-19), [GNU-MOTION-PARTS](../queue.md#gnu-motion-parts--diagnose-and-fix-non-smooth-giant-motion).
**Related:** [bosses](bosses.md) (the fights' design), [GNU-ton's debts](gnu-ton-fight-debts.md), [semantic-part rendering](../engine/semantic-part-rendering-and-ragdolls.md).

## Name cards (BOSS-CINEMATIC-COVERAGE)

**Census, 2026-11-10.** Twelve authored boss encounters
(`game/ambition_content/assets/data/boss_encounters/*.ron`). The name card
(`ambition_render::rendering::boss_stage::show_boss_name_card`) draws a boss's
`BossStage::card`, and before this change only a conductor could set it:
Wolfenstein's (`wolfenstein::stage_at`, with his roar) and the Inugami's
(`inugami::rise_stage_at`, at the end of its rise). The other ten bosses never
showed their names. Two bosses author a title (`BossBehaviorProfile::title`);
the card shows the name alone when there is none.

**Done, 2026-11-10: the encounter owns a default card.**
`BossEncounterSpec::name_card` is a `BossNameCardCue`:

- `At(seconds)` (the default, `At(0.4)`): the encounter shows the card that
  long after the boss wakes. It comes in over 0.35 s, holds 2.2 s and goes
  over 0.5 s. The count is `ActorPhaseState::seconds_since_wake`, which runs
  on from a short intro into the first phase, so a one-second intro does not
  cut the card.
- `Conducted`: the conductor stages the card and the encounter shows none, so
  a bespoke intro is never doubled. Wolfenstein and the Inugami declare it.

The read model (`ambition_sim_view::rebuild_boss_render_index`) shows the
larger of the conductor's card and the encounter's. The card is a function of
the boss's phase state, so a rewind, a replay and a retry each show it on the
same frames, and a boss put back to sleep (a reset, a new attempt) shows it
again when it wakes.

**Witnesses.** `boss_encounter::phase_mechanism_tests::the_encounter_shows_a_name_card_after_the_boss_wakes`
and `a_conducted_card_is_the_conductors_alone` (unit);
`every_boss_shows_its_name::each_boss_shows_its_name_card_after_it_wakes_and_puts_it_away`
(app_it: the Clockwork Warden, the T-rex and the Flying Spaghetti Monster
woken in their own rooms, read from `BossRenderIndex`); Wolfenstein's
conducted card stays `wolfenstein_fight`'s subject.

**Done, 2026-11-10: a card per view.** `show_boss_name_card` draws one card
over each local view, for the boss in that view's live room (the room of its
`ResolvedViewSubject`), placed in the view's rectangle (`ViewPlacement` over
the gameplay rectangle, as the HUD is). A composition with no local views
draws the primary live room's card across the whole gameplay area. Witness:
`boss_stage::name_card_tests::each_view_shows_the_card_of_the_boss_in_its_room`
(two views in two rooms; a view whose boss shows no card shows none; poison:
every view reads the primary's room, red).

**Open.**

- **Skip.** Boss intros have no skip input; the card has nothing to skip.
  If an intro skip is added, it should advance the phase state, and the card
  follows.
- **Boss UI that is not per view (seen 2026-11-10 in the two-view captures;
  fixed 2026-11-14).**
  - The boss health bar is drawn at the top of each local view, for the boss
    fought in that view's room (`sync_boss_health_bar_overlay`; witness
    `each_view_draws_the_bar_of_the_boss_fought_in_its_room`).
  - The "GRADIENT SENTINEL" banner was the Clockwork Warden's room intro
    still naming the boss he replaced, and it doubled his name card: the
    intro (`boss_intro_gradient_sentinel`, id kept for saves) has no banner
    now, and its line is the Warden's.
  - The intro's dialogue box draws in the view of the player it holds (the
    2026-11-14 capture shows it in the second view).

**Done, 2026-11-10: a rendered witness.**
`a_boss_name_card_is_drawn::a_woken_bosss_name_card_is_drawn_in_its_rectangle`
(app_it, GPU, `--include-ignored`) starts the shipped host in `trex_arena`
and in `basement_boss`, captures a frame before the boss wakes, wakes it, and
captures again once its card is all the way up. It counts the pixels of the
name's colour inside the name's own drawn rectangle: 2,981 for the T-rex and
11,994 for the Clockwork Warden with the card, 0 in each before it. The
captures (`/tmp/ambition-boss-card-<room>.png`) show "T-REX" and "CLOCKWORK
WARDEN" with the rule beneath.

**Done, 2026-11-10: a rendered witness of two views.**
`a_boss_name_card_is_drawn::two_views_in_two_boss_rooms_each_draw_their_bosss_card`
(app_it, GPU, `--include-ignored`) starts the shipped host in `trex_arena`
with two pads in a fixed split. It sends the second seat to `basement_boss`
and waits until both bosses' cards are all the way up. Then it counts each
drawn name's pixels in its own rectangle:
- the T-rex in the left half: 2,980;
- the Clockwork Warden in the right half: 12,003.

The control is the same rectangles once both cards are put away: 0 and 8.
The bosses wake while the split is set up, so no frame exists before the
wake. Captures: `/tmp/ambition-boss-card-two-views-up.png` (cards up) and
`/tmp/ambition-boss-card-two-views.png` (put away).

## GNU2's finale (GNUTON-GNU2-FINALE)

**Step 1 done 2026-11-18: GNU2 is published and catalogued.** The renderer's
`gnu_ton_boss` target publishes `giant_gnu2` beside the giant: the giant's own
drawing, pose and rows in the psychic containment harness (`gnu2_art`), with
the giant's seams, standing surfaces and body box per frame, its part
flipbook, actor contract and portrait. The catalog entry `npc_giant_gnu2` has
the body of `npc_giant_gnu_whole` (same rows, scale and mount surfaces), so a
body rides it as it rides the giant.

The finale summons GNU2 through a real live portal in the same active area.
Its portal staging belongs with the portals work; the parts that are not
portal work are: a two-boss victory policy (either defeat order; doors gated on both; no
duplicate spawn on retries; both cleaned on teardown), and readable counters
for two giants. Keep phases 1 and 2 as they are until the replacement passes
replay, reset and exit gating.

### The plan (decided 2026-11-19)

Measured first: the arena (both the hall's, `sandbox.ldtk`, and the handle's,
`generators/gnu_ton_gravity`) has no off-camera space, its camera zone is the
whole room, and the boss summon (`ambition.boss.summon`) makes a *minion*,
which is not a boss: it has no bar, no save record and is not a member of
the arena's gate. The boss bar draws one boss a view.

Decisions, each the shape that makes the acceptance follow from what exists
rather than from new bookkeeping:

- **GNU2 is a boss placement, not a summon.** It is authored in the arena
  room, in a sealed **containment cell** past the arena's wall, outside the
  camera zone, held (no conduct, no bar, no harm) until it is called. One
  placement gives the rest for free: a retry or a re-entry builds it once
  (no duplicate), the room's teardown takes it, its defeat is recorded
  `Cleared` under its own id, and the gate already counts every GNU-ton-kind
  placement of the arena (debt 13): GNU2 joins that set by its brain.
- **The call is a real pair.** In the beat that now starts the launch, the
  scholar opens a held pair (`ambition.portal.held_pair`): one end upright in
  the cell's inner wall, the other upright in the arena's wall, each wider
  than the giant. GNU2 walks into the cell's end and out of the arena's: it
  is never placed, only walked, and the pair closes behind it.
- **Its own conductor.** A `gnu2` conduct module: it walks the hall at the
  giant's stride, stomps (the giant's floor shocks), bucks a body off its
  back, and is hurt on its head and back. Its health is its own.
- **Two bars.** The boss bar draws every boss being fought in the view's
  room, stacked, each with its name.
- **Victory.** Either order. The rider yields when he is beaten (his finale
  plays, the giant he rode stands down) while GNU2 fights on; the gate opens
  when both are `Cleared`.
- **Then retire space.** The well, the sky's ascent and the tides go when
  the replacement passes replay, reset (a death mid-finale) and exit gating.

Steps: (2) the cell and GNU2's held placement in both arenas; (3) the call
through the pair; (4) GNU2's conductor; (5) two bars; (6) the victory and
the rider's yield; (7) retire space.

**Steps 2-6 done 2026-11-19.** What was built, and where it differs from
the plan:
- **The cell.** Both arenas (LDtk-owned: `gnu_ton_arena` in `sandbox.ldtk`,
  `gnu_ton_shoulders` in `science.ldtk`) are 2048 high. The top 1280 is the
  hall, framed by a `zone_bounds` camera zone. Under the floor (solid to
  1344) is a sealed cell, x 96-960 and y 1344-1920. GNU2's `BossSpawn`
  (`hall.gnu2` / `gnu_ton_handle.gnu2`, brain `PhaseScript:gnu2`) is at the
  cell's east end, with `wakes_after` naming GNU-ton. Its rider encounter
  says `successors_wake: Enraged`, so GNU2 wakes at his enrage rather than
  at his defeat.
- **GNU2 opens the pair, not the scholar.** A held pair belongs to the body
  that submits it, and GNU2 is the one that walks through, so its conductor
  holds the pair. The scholar's beat calls it ("Every great work has a
  sequel. GNU2!"). Both ends are upright with normal +x: one in the cell's
  west wall, one in the hall's west wall. The conductor learns the hall from
  its predecessor (`BossConduct.predecessor.hall`, port v11). When GNU-ton
  is cleared and not rebuilt, the predecessor is read from his authored
  placement, so a retry after his defeat still has a hall.
- **Walked, not placed.** A new request, `ambition.boss.walk`, steers a boss
  along its side axis by `CommandedMove`. Three things stopped the walk,
  each measured:
  1. *A float-exact touch.* GNU2 flies (it does not fall). Its feet were
     exactly on the cell's floor, and the hole cut that floor at the near
     plane, so the floor piece's new face stood at the plane level with its
     feet. A body that is not standing is stopped by a face its feet only
     touch; the grounded "a seam is not a wall" rule does not cover it. The
     far end reaching into the hall's floor did the same through the pair.
     GNU2's placement now holds it 2 px over the cell's floor, and it comes
     out of the far end 2 px over the hall's (`EXIT_LIFT`).
  2. *The transit's map.* Two ends that face the same way map by a half
     turn, so walking west into the near end comes out walking east.
  3. *The conductor's chart.* The conductor reads the body before the
     transit takes it. Told "walk west" until it had arrived, GNU2 crossed,
     walked back into the far end, and was carried back each tick. Now, once
     its middle is past the near end, it is told "walk east": the next step
     it takes is on the far side. It has arrived when it is wholly out of the
     far end, and then the pair closes.
- **Two bars:** the boss bar draws every fought boss of the view's room,
  sorted by id, stacked 66 px apart.
- **Victory:** the arena's gate members are every GNU-ton-kind placement,
  GNU2 among them. The handle's way out has two grilles, one per boss.

Witnessed in `app_it`: `at_his_enrage_gnu2_walks_into_the_hall_through_its_pair`,
`defeating_him_opens_the_arena_and_his_body_going_does_not_close_it` (the gate holds after GNU-ton alone and
opens when GNU2 is cleared too), and the handle's two-grille tests.

**Step 7 done 2026-11-19: space is retired.** The conductor no longer has
`space()`, the well's pair and field, the tides, the helmets or the ascent;
`ch::space` and the space sky (`presentation/gnu_ton_sky.rs`, `ascent_of`)
are deleted. Its record is v2: `call_t` (the call's clock) replaces the
launch's four fields. The space helmet stays a worn-gear row.

What step 2 needed first, measured 2026-11-19: no boss takes its body from a
character entry (`BossSpawn` has no `character_id`; `BossProfile` names
none), so GNU2 as a boss needs a boss sheet (`boss_sheets.ron`, its rows as
`BossAnim`s: `Rest` from `armed_rest`, a walk, `Hit`, `Death`) drawn from
`npc_giant_gnu2`'s parts the way PF2b draws nine boss sheets from their
characters' (`BossSheetSpec::parts`), published by the renderer's
`gnu_ton_boss` target beside `giant_gnu2`. Step 5 is small: the bar's
`find_map` over `BossRenderIndex` becomes every fought boss of the room,
sorted by id (the index is a hash map) and stacked one panel height apart;
it waits for a two-boss fight to witness it.

## The giant's motion (GNU-MOTION-PARTS)

**Measured 2026-11-10** with `gnu_ton_fight::how_the_giant_moves_frame_by_frame`
(ignored; a measurement): twenty seconds of phase 1 with the player stood far,
under, far on the other side and under again, so the giant walks to its marks
(`choreography::stride::mark` walks only for a player within 300 or beyond 640
world units). Each frame records the giant's body, the row it is drawn in and
that row's clock, the scholar on its back and its fists, and the test prints
every jump.

**What stepped, and what was fixed.**

1. **The hooves slid.** The walk row's clock was the walk's elapsed time
   (`walk_t`), but the body takes about 13 frames (0.21 s) to reach
   `stride::SPEED`, and slides about 13 more to a stop after a walk. Its art
   is drawn so a hoof on the floor stands still at `SPEED`, so the feet slid
   by the difference: the walks covered 113–129 world units per second of row
   clock against 135.4. Every walk in the run was short (36–120 frames; a blow
   ends a walk), so most of each walk was speeding up or slowing down.
   **Fixed:** the row's clock is `Conductor::stride`, the distance walked over
   `SPEED`, from the start of the walk; the walk row stays drawn while the body
   slides to a stop (`stride::STANDING`). Measured after: 135 per row second
   on every walk.
2. **Rest started mid-breath.** With its arms on, the giant's rest row was
   drawn at the conductor's global clock (`clock % REST_ROW`), so after every
   blow it came back to rest 0.08–0.78 s into the row instead of at the frame
   the blow's recover row ends on. **Fixed:** `Conductor::rest_t` counts the
   seconds at rest; measured after, each return to rest starts at 0.00 s.

3. **The giant's walk stepped between frames (rendered parts, measured
   2026-11-14).** The giant is drawn from its part flipbook
   (`giant_gnu_parts.ron`), and none of its 56 clips tweened: its publisher
   built the flipbook with no tween rows, where every other character's
   walk tweens (`part_flipbook.LOCOMOTION_LOOPS`). Its walk is 12 frames of
   0.08 s, and each frame moved its legs 22-26 sheet pixels at once (median
   22.4, at the 840 x 680 frame) while the body slid smoothly at 135 world
   units per second: at the giant's size that is the visible step. Its rest
   loops move about 2 px a frame and read as still. **Fixed in the source**
   (renderer `81b31ac`): `GIANT_TWEENED_ROWS` (walk, walk_back, armed_walk,
   armed_walk_back) publish as tweened clips; built in-process, 276 of the
   walk's 348 draws pair with their next frame and move between frames, the
   rest change picture and hold. **Republished 2026-11-17:** the published
   flipbook carries the four tweened clips (`tween: Linear`). Render-rate
   interpolation itself is the shared rigged path's (`frame_phase` of the
   way to the next frame), so a tweened clip is interpolated at any render
   rate; the root's own interpolation is `PresentedPose`, measured smooth
   in the 2026-11-10 probe.

4. **The tweened walk still snapped its hooves (measured 2026-11-18).** The
   published walk was sampled at 60 and 144 Hz through the runtime's own
   `tween_into` (a temporary probe over the published `giant_gnu`
   flipbook): the worst move of any draw between two samples was 26.2 sheet
   pixels at both rates, tweened or not. Every such jump was a hoof: its
   track draws another raster of the same hoof in the next frame (63 x 50
   and 62 x 51 px, pivots in the middle, drawn at one angle and scale), and
   the tween rule moved a track only to the same part, so the hoof held and
   then jumped its whole step every 0.08 s. **Fixed (rule):** a track whose
   next draw is the same picture redrawn (a part within a pixel of its size,
   at its angle and scale, a step no longer than the picture) moves to it,
   keeping its own part until the frame changes
   (`rigged::same_picture_redrawn`, with the renderer's oracle
   `part_flipbook.tween_draws` stating it the same way). A part change that
   turns or flips (a hand drawn mirrored) holds, and so does a longer step,
   which is a cut: an 8 px swim bubble that starts again 38 px below, a ring
   node going round behind. After: the walk's worst move is 5.5 px a sample
   at 60 Hz and 5.1 at 144 Hz (10.2 in `armed_walk_back`). Across the cast
   the rule pairs 6410 more draws in 411 flipbooks, none further than its
   own picture (the most is 29.6 px, a 41 px part); a census before the size
   and step conditions showed why each is needed (the architect's mirrored
   hands, Scout's swim bubbles, Vera Ruin's ring nodes).

**Measured and not changed.**

- **Windups play slower than real time** (their clock steps 0.010–0.014 a
  frame): the conductor stretches the 0.72 s windup row over the move's
  telegraph. Uniform, so smooth; it is the move's timing.
- **Holds freeze** (clock step 0): authored holds.
- **A blow cuts a walk.** The windup row starts while the body still slides
  for about 13 frames. Holding the blow until the giant stands would change
  the fight's timing; left as it is.
- **The fist jumps 230–460 units relative to the giant** on the frames a
  portal blow's strike starts and its recover ends. Measured 2026-11-12: it
  is not a crossing and nothing drawn moves. With its arms on, the giant's
  sheet draws the arms and each fist is drawn as `ch::arms::NO_FIST`; the
  fist body is only the blow's hit volume, parked at the giant's middle
  between blows and held through its near line (`near + inward * depth`, in
  its own chart) while the blow lasts. Its first strike frame is already
  130 units into the line, so the volume moves there in one frame.
- **Turning mid-walk** happens once in the run (it turns to face the player).

**Phases 2 and 3 and the rider, measured 2026-11-11** with the same probe,
now three runs of 20 s: phase 1; phase 2 (`free_fists`); phase 3 (the
scholar at a fifth of his health, so the well opens and the fight goes on
in space). It also records the scholar's own drawn row and its clock, and
checks each fist's move against its own velocity.

- **The walk is right in every phase:** 135 world units per second of row
  clock, phases 1, 2 and 3 alike.
- **The rider's rows are smooth:** `point`, `conduct`, `invoke` and `brace`
  start at 0.00 s when a move starts and step by exactly one frame each
  frame; between moves he is unpinned (his own rows). No clock went back or
  skipped.
- **The stomp plays at half speed** (its clock steps 0.008 a frame), as the
  windups do: stretched over its telegraph, uniform.
- **In phase 1 the fists ride the rig:** placed each frame with velocity
  zero (417 frames move a fist 2.3 units with velocity zero). Not a jump; a
  renderer that extrapolated from velocity would lag them.
- **⛔ A body that straddles a portal moves unevenly (portal work).** Two
  cases, both in portal transit, which the portals work owns:
  1. **The giant falling out of the well's exit portal** (phase 3, frames
     77–89 of the run): falling 15–16 units a frame, it does not move at all
     on every second or third frame (77, 80, 82, 84, 87, 89) while its
     velocity keeps growing, then moves every frame once it is clear of the
     aperture (about frame 90). A hitch at about 20 Hz for a quarter second.
     **Diagnosed 2026-11-12** (temporary probes, deleted): the giant collides
     with its own back. Its slope (`ambition_mount::SLOPE_LINE`) is room
     collision that moves with it. While the giant straddles the exit, the
     cut planes put that cast into the curved search. The slope enters the
     stay cell across the cut already inside the giant's material. The envelope
     keeps a constant rounding band before that instant, and the exact region
     inside the band is empty, so the search refuses `Precision`. All nine
     refusals in phase 3 (frames 78–132, every 7 or 8 frames) name
     `mount_slope` segments. Since `811150d6f` a refused straddler halts, so
     the giant now restarts its fall from rest at each refusal. Skipping the
     floor's empty instants is no answer: the band is about 73,000 ulps and
     the search refuses `WorkLimit` instead. **Fixed 2026-11-12:** a body
     does not collide with the lines it carries. `SurfaceChain::carrier`
     names the body by its entity's bits; the mount sets it on its slope and
     drawn ledges, and every material cast of a body's own step carries its
     key (`MaterialCast::carried_by`, from `BodyClustersMut::carrier`). The
     giant now falls out of the exit accelerating every frame, with no
     refusal; the 92 px snap at frame 134 went with the hitch. Falling
     freely, it reaches the floor portal again while the well closes, and the
     well closes by shrinking its opening. That shrank onto the straddling
     giant and pushed it through the floor. A held pair now keeps its opening
     while a body straddles it, and closes once it is empty (see the portal
     plan, "A body's own lines, and a pair that does not close on a body").
  2. **A free fist in a portal pair** (`demonstrate_pair`, phase 2): the
     probe read each fist's position jumping about 280 units between the
     pair's two ends every frame, and each fist's velocity as zero on every
     other frame. **Not a defect (2026-11-12):** the probe named the fists
     by `Scene::fists`, which is ordered by x, and the two fists stood at
     one x a pair apart, so they swapped names whenever their x order
     flipped. Following each fist by its hand (`fists_by_hand`), phase 2 has
     no jump and no fist whose velocity disagrees with its motion.

**Not measured.** The rendered part transforms (the headless harness draws
nothing) and render-rate interpolation. On the measuring machine the giant's sheet
was not baked, so the shoulder and hump tests in `gnu_ton_fight` failed there
for that reason, before this change and after it alike.
