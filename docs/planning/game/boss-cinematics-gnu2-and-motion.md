# Boss cinematics, GNU2's finale and the giant's motion

**Owner of:** [BOSS-CINEMATIC-COVERAGE](../queue.md#boss-cinematic-coverage--give-every-boss-a-wolfenstein-class-name-intro), [GNUTON-GNU2-FINALE](../queue.md#gnuton-gnu2-finale--summon-a-second-gnu-through-a-real-portal), [GNU-MOTION-PARTS](../queue.md#gnu-motion-parts--diagnose-and-fix-non-smooth-giant-motion).
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

**Not started.** The finale summons GNU2 through a real live portal in the
same active area. Its portal staging belongs with the portals work owned by
the portals agent; the parts that are not portal work are: publishing GNU2's
character and combat geometry (its art is an unpublished prototype), a
two-boss victory policy (either defeat order; doors gated on both; no
duplicate spawn on retries; both cleaned on teardown), and readable counters
for two giants. Keep phases 1 and 2 as they are until the replacement passes
replay, reset and exit gating.

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
