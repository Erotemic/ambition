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

**Open.**

- **Multiview.** The card is drawn for the boss in the *primary* live room
  (`PrimaryLiveRoom`). A second local view in another boss's room shows no
  card. A per-view card needs the card node to belong to a view, as the HUD
  does.
- **Skip.** Boss intros have no skip input; the card has nothing to skip.
  If an intro skip is added, it should advance the phase state, and the card
  follows.
- **A rendered witness.** The app_it witness reads the read model the
  renderer reads, not pixels. `boss_stage::name_card_tests` cover the node.

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

**Not started.** The giant looks like it moves unevenly. A hypothesis only:
the conductor pins the mount's animation rows apart from its `MountWalk`
motion, its rider and its fists. First measure, frame by frame, the mount's
root position, each row clock, the rendered part transforms and the rider and
fist poses, and find which one steps. Fix the pose authority or the shared
render adapter that is at fault, not one boss's offsets.
