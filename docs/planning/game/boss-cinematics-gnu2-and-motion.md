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
- **Boss UI that is not per view (seen 2026-11-10 in the two-view captures).**
  - The T-Rex's health bar is drawn top-centre across both views, over the
    second player's HUD.
  - A "GRADIENT SENTINEL" title banner is drawn the same way.
  - The Sentinel's dialogue box is centred on the whole screen, not in the
    view of the room it speaks in.

  The name card is per view; these are not. They belong with the per-view
  HUD work, not this row.

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
  portal blow's strike starts and its recover ends: the fist going into and
  coming back out of the portal pair. Portal transit belongs to the portals
  work.
- **Turning mid-walk** happens once in the run (it turns to face the player).

**Not measured.** The rendered part transforms (the headless harness draws
nothing), render-rate interpolation, phase 2 and 3 (arms in their lines, the
space phase), and the rider's own rows. On the measuring machine the giant's sheet
was not baked, so the shoulder and hump tests in `gnu_ton_fight` failed there
for that reason, before this change and after it alike.
