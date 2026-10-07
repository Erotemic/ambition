# Bosses (game content)

The boss *system* is engine work
([`../engine/boss-system.md`](../engine/boss-system.md)). This page is the
**design language** and the specific bosses. Generic machinery (fighter-brain
verbs, the glider projectile primitive, `CharacterAnim::Special`, the
dialogue→provoke Yarn command) belongs to the reusable actor, projectile,
animation and interaction owners, not to a common `core`. A boss's stats,
tuning, placement and dialogue are game content in `ambition_content`.

## The design language

> Every boss is a failed objective function.

A boss is a character whose flawed optimization the player reads, exploits and
out-learns. Its defeat is the player demonstrating a better policy than the
boss's.

## The Perfect Cell-ular Automaton (the exemplar)

The PCA proves the unified actor pipeline: it is not a special-case boss. It
starts as a talking NPC and becomes a melee boss only if the player chooses
"Challenge" in dialogue. It is the same body and the same `Brain` +
`ActorControlFrame` seam from peaceful to hostile (and, one day, possessed).

- **Concept:** a cellular-automaton entity. Its ranged zoning tool is a Conway
  Game-of-Life glider.
- **Brain:** the fighter brain. Brain output is abstract intent; the per-actor
  `ActionSet` resolves it to concrete verbs. Difficulty is data
  (`reaction_delay_s`, `commit_probability`, `accuracy`), and the brain perceives
  a lagged opponent.
- **Kit (built):** melee, jump, fly-reposition, the glider, blink-evade, the
  aerial dive/perch game, and the `cellular_pulse` signature move
  (`game/ambition_content/src/cellular_automaton_moveset.rs`). Glider, blink and
  fly are body capabilities, so a possessing player inherits them.
- **Encounter:** dormant NPC → Yarn dialogue (a Challenge branch and peaceful
  exits) → combat → win or loss. The dialogue→provoke bridge flips the brain and
  disposition and arms the hostile volumes.
- **Placement:** the design name "Noether Chamber" is the LDtk level
  `symmetry_room`. The PCA is an `NpcSpawn` there (`character_id` and
  `dialogue_id` `perfect_cellular_automaton`, `brain_override: stand_still`). It
  is also placed in `hall_of_characters`.

Remaining PCA work is encounter and narrative polish, not kit.

`imperfect_cellular_automaton` is a separate catalog character (a hall NPC). It
wears the PCA's move table. Whether it is a boss is an open design call.

## Roster (story bosses)

- **Perfect Cell-ular Automaton** — the dialogue-gated fighter above.
- **Mockingbird** — mimics and steals the player's moves.
- **Clockwork Warden** — reads the player's patterns; beating it means breaking
  pattern.

Each is authored as content on the engine boss system. None needs a bespoke
simulation path.

## The Tyrant King (T-rex) — rework (2026-10-05, Jon)

Jon: "his collision is all messed up and he stands on the ground … let's make a
good trex boss fight", after GNU-ton. The old fight is a data-only charger:
generic strike boxes scaled by a body box that disagrees with the art, an
aerial body pinned at its spawn height, and no fight test.

**The fight: a pattern boss, metroidvania style** (Jon, the same day: "I don't
like the stand under his feet gimmick … make it a polished excellent 2d
metroidvania pattern based boss fight"). No safe spot: his body hurts to
touch, and every place you can stand is answered by some move. Each move is
learnable: a distinct tell (pose, sound, dust or shine), one honest dodge, and a
recovery that is the punish window. Damage comes from reading him, not from
hiding.

- **Phase 1 — The Hunt.** *Snap bite* (you are in front: the head rears, the
  jaw glints, he lunges one stride; dodge back or jump the lunge, then hit the
  lowered head). *Charge* (you are far: a roar and a foot scrape, then he runs
  the hall and cannot stop; get on a ledge or jump the lowered head. He hits the
  wall and is stunned with stars, the long punish window). *Tail whip* (you are
  behind: the tail coils, then sweeps low; jump it, punish the turn).
- **Phase 2 — The Quake.** *Stomp* (he rears onto one leg and stamps, and a
  shockwave runs the floor both ways; jump it). *Rock fall* (a crash or a stomp
  shakes rocks from the ceiling, with dust and shadow tells). *Ledge snap*:
  standing on a ledge is no refuge, because he snaps up at it. The bite becomes
  a double.
- **Enrage — Extinction.** A roar that blows you back and shakes the ceiling,
  charges that turn once and come back, and a leap that lands where you stood
  with a quake.

**Body.** One world box, sized from the art pose by pose: the GNU's "the art
is the body". His feet are on the floor and he walks it under gravity. World
collision stays a single box, the engine's contract. What the player HITS is
per-frame convex hulls for head, body, tail and legs, published by the renderer
the way the FSM's bell is. What HITS the player comes from the art too: the
mouth, the tail tip and the stamping foot, frame by frame from his part tracks.

**Architecture.** The FSM's road: the pattern in `boss_profiles.ron` picks the
move and when (`Special` keys, telegraph `(pose, cue)` identities, `Select`
arms on where you stand). A conducted module, `ambition_content_modules::trex`,
does the rest: pose, volumes, drawn row, rocks. Receipts are a headless
`trex_fight.rs` in the GNU-ton test's style, a `fight_discovery` run, and
captures.

**Where it stands (2026-10-05, evening).** Slices 1-4 are in, and the art
part of 6. He stands on his floor (the placement law, guarded for every boss
sheet), stalks you, and has the full kit:

| phase | moves (pattern key) | the dodge | the punish |
|---|---|---|---|
| Hunt | bite (`trex_bite`), tail whip, snap up at a ledge, charge into the wall | back off / jump the tail / leave the ledge edge / ledge or jump | the head after a bite; the stun after the charge |
| Quake | double bite (one clack of teeth, two snaps), stomp (shocks both ways + rocks), the rest of the Hunt | jump the shock; leave the dust | the stomp's recovery |
| Extinction | roar (blast + rocks), leap at where you stood (quake + rocks), the charge turns once and comes back | out of the roar's mouth; move off your spot; read the turn | the stun |

The sheet gained `stunned` (slumped, stars circling), `snap_up` and `leap`,
and a stomp that rears higher (`scripts/build_trex_enemy_rig.py`); rocks are a
`trex_rock` prop. A module can shake the camera now
(`ambition.feedback.camera_shake`, lowered to the quarantined
`CameraShakeRequest`). Thirteen headless fight tests
(`game/ambition_app/tests/trex_fight.rs`), each poisoned once.

**2026-10-06 (Jon: "its hurtbox is crazy big … grab the player with its
mouth, shake its head around … and throw the player … spawning some enemy
stochastic parrots, raptors").**
- He is hit through his PARTS (renderer `trex_enemy_body_rig.ron`: eleven
  hurt parts measured from his art), posed from his pinned row
  (`resolve_body_rig_poses` reads `PinnedRow`), placed by `RigFeetOffset`.
- The jaw grab (`trex_jaw_grab`, phase 2 on): `grab_reach` / `grab_shake` /
  `grab_throw` rows; the engine's capture relation offered to modules as
  `ambition.combat.body_hold` (seize, carry, pummel, throw, release); mash to
  break free. The held body rides the `jaw` attachment of his body rig
  (review 2026-10-06, P2): his art states the point on his jaw joint, his
  hold names it (`hold_at`), and the capture relation places it from the pose
  of the tick. His conductor keeps no number for it and makes no thrash of
  its own: the body moves as `grab_shake` moves his jaw
  (`trex_fight::the_body_in_his_jaws_is_where_his_rig_says_his_jaw_is`).
- The call (`trex_call`, phase 2 on): stochastic parrots from the high
  corners; enraged, raptors along the floor first; at most four alive
  (`BossConduct::minions`).
- Roar candidates for Jon's ear: `untracked/sfx-candidates/trex_roar/`
  (a new source-filter `roar` mode in the SFX renderer).
- He is DRAWN from his parts too: `BossSheetSpec::parts` names
  `npc_trex_enemy`, whose sheet the renderer demands; the cell
  `animate_bosses` draws poses that character's animator by row name, and the
  rigged driver draws him (`the_trex_is_drawn_from_his_parts.rs`).
  `animate_bosses` now runs before the character animators and the driver.

- His VOICE (Jon's picks of five audition rounds, 2026-10-06; round 1 "sounds
  like a lawnmower"): one throat, the SFX renderer's new `creature` mode (the
  audition code itself, `backends/creature_voice.py`): `boss.trex.roar`, the
  phase-2 `boss.trex.scream`, seven growls in two takes each
  (`boss.trex.growl_*`) and the death wail `boss.trex.death`. Each tell's
  growl is its telegraph cue (which also keeps the moves readable); the
  conductor screams him into phase 2 (rearing through the encounter's
  transition lock, `BossConduct::between_phases`), roars the call, growls as
  he seizes you, snarls as he flings you, yelps on the crash, huffs while he
  stalks, and wails once dying. `every_boss_telegraph_cue_has_a_recipe` holds
  every boss's telegraph cues to a recipe (a missing cue plays silence).

- His tail and his crash have their own sounds (`boss.trex.tail_whip`, a whip
  crack; `boss.trex.crash`, a quake boom; Jon: "its your pick"). His raptors
  hunt (their catalog row states a profile: with none, a body is lowered
  with a 0 aggro radius and stands where the call put it). Walking back in on
  him dead is silent (he wails only for a death he was seen alive before).

Still open: `fight_discovery` tuning and Jon's playtest.

Slices: (1) the body: one placement law for every boss drawn from a sheet,
with the T-rex grounded, the hulls and the art's volumes; (2) the conductor,
with bite, tail and charge-into-stun; (3) the pattern and arena (width for the
charge, ledges, ceiling); (4) phase 2 and enrage; (5) sound and effects (tells,
impacts, stun stars, shockwave, rocks), auditioned before they are ported;
(6) art: rows the moves need that the sheet lacks (stunned, rear, leap); (7)
tuning from `fight_discovery` and Jon's playtest.

## The Mockingbird — air chase (proposed 2026-10-06, Jon)

**Art: shipped.** The boss wears `mockingbird_boss_v2`, an SVG-rigged
redesign: a big mecha jet engine with a hooded, skull-like face and a lipless
grin, rigid jet wings with missiles, two rotors, a thruster and two grappling
claws. The first design (`mockingbird_boss/`) still publishes as its lineage.

**The fight Jon wants:** a fast air chase. The Mockingbird holds one side of
the screen and chases burning flying sharks that flee across a sky that never
stops moving. The sharks are the player's footing: moving platforms to dodge
and attack from. It fires missiles and fireballs, and dives into the stage to
chase and bite, which opens it to damage. A player who falls off the bottom is
caught by a shark and carried back up, so the fight keeps going. When it dies,
its treasure falls into a "ground" room below, which joins the chase to the
normal LDtk levels. The fight must not assume the player can fly (or has the
fuel to).

What exists (measured 2026-10-06):

- **Parallax:** 4 fixed layers offset by camera position
  (`ambition_render/src/rendering/parallax.rs`). No time-based scroll, no
  tiling, no wraparound, no autoscroll camera.
- **Moving platforms:** `Sweep`, `Path` and `VerticalLoop`, which wraps like an
  elevator (`ambition_platformer2d_world/src/platforms/mod.rs`). No horizontal
  wrap. The burning flying shark is a mount sheet, not a platform.
- **The current fight:** 28 HP in `mockingbird_arena` (960x768, between the
  cove and the dojo, placement `cove.mockingbird`). It uses `AirSwoop` and
  cycles `wing_sweep`, `dive_lane`, `broadside` and `echo_fan`.

**Proposal: no "chase level" type. Four room features, each useful on its
own; the chase room composes them.** The stage is static and only looks like
it moves:

1. **Autoscrolling parallax:** a room states a scroll velocity, and its
   layers tile and wrap horizontally at their own factor (a Hanna-Barbera
   loop). Presentation only.
2. **A horizontal wrapping motion** for moving platforms, the sideways
   sibling of `VerticalLoop`. Sharks leave one edge and re-enter at the
   other, weaving on a bob. A platform can wear a character sheet (the shark
   rig).
3. **A fall rescue:** a room rule that, instead of the kill floor, sends a
   carrier platform up under a falling player and lifts them back into play.
4. **A reward anchor in another room:** the chest is already keyed by
   placement id (`BossRewardAnchor.placement_id`). The chase's anchor sits
   at the top of the ground room, so the treasure falls in from the sky
   there.

The Mockingbird itself is boss content: it is anchored to one side, with
volleys (missiles from the wingtip, fireballs), and a dive-in bite that ends
in a recovery window.

**Ruled (Jon, 2026-10-06):** the player gets up into the chase from the
ground room on a shark that swoops down and carries them up through the
ceiling.

## The hall of bosses: its own instances, and life switches (proposed 2026-10-06, Jon)

**The problem.** Two of the hall's ten doors lead into main-game rooms
instead of rooms of their own:

- the Mockingbird: `hall_to_mockingbird_portal` → `mockingbird_arena`, whose
  exit leads to the cove;
- the Clockwork Warden: `hall_to_warden_portal` → `basement_boss`, whose exit
  leads to the hub.

So you walk through a door and come out somewhere else. The other eight
lead to dedicated arenas that return to the hall.

**Wanted.** The hall has its own instance of every boss: separate rooms and
separate placements, so the hall's progress and the main game's progress are
independent. This already works: progress is keyed by authored placement id
(Q57). The world already has archetypes placed twice (Mode Collapse, the
Warden).

**Life switches.** A switch outside each hall door shows that hall boss's
state: green when it is alive, red when it is dead.

- Flipping red to green revives it. `retract_defeat_records` already moves a
  placement back to `Untouched` and clears its looted flag.
- Green to red, if allowed, kills it:
  - when the boss is loaded, through its death;
  - when it is not loaded, by recording it `Cleared`, so the next visit finds
    a corpse.

The switch shows the boss's state; it stores no state of its own. Today's
`Switch` toggles its own save entry, so this is a new switch action, plus
the "may this be flipped" rule. There is no interactability condition today
(`features/ecs/interact.rs`).

**Ruled (Jon, 2026-10-06):**

- **Hall kills are practice.** Killing a hall boss leaves the main game's
  boss untouched: "They are completely separate." A hall kill may drop a
  reward chest of something small (health, money), but no story reward.
- **The kill switch is allowed and kills properly.** Flipping a green switch
  kills the boss on the spot if it is loaded (anyone fighting it sees it
  die), or marks it dead if it is not. No lock.

**Built (2026-10-06):** the hall's Mockingbird and Warden doors lead to
`hall_mockingbird_arena` and `hall_warden_arena`, copies of the story rooms
whose doors return to the hall. Their bosses are `hall.mockingbird` and
`hall.clockwork_warden`. Both rooms carry the level field `practice`
(`RoomMetadata::practice` → `BossConfig::practice`). A practice boss's
death:

- drops the coin and health bounty, and a chest holding a purse
  (`PRACTICE_CHEST_PURSE`);
- drops no ability and no gauntlet, and moves no quest.

Pinned by `boss_lifecycle::a_practice_copy_dies_without_the_story_consequences`
and the `the_hall_of_bosses_has_its_own_bosses` tests.

**Built (2026-10-06): the life switches.** There is one by each of the nine
boss doors (`SwitchAction::BossLife`, `target_encounter` = the hall boss's
placement). Each is green while that boss is alive, and stores nothing of
its own: it shows the boss's record, so a boss killed in a fight turns it
red. A press on a green switch kills the boss: zero health and `Death` at
once if loaded, otherwise just recorded dead, and its reward reads looted,
so a switch kill pays nothing. A press on a red switch revives it: the
defeat records are retracted and a loaded body is re-seeded at its spawn.
The hall-only bosses now have placement ids (`hall.trex`, `hall.gnu_ton`,
…), so existing saves see them alive once. Mode Collapse's hall arena is a
practice room, because Mode Collapse has a main-game placement (the overfit
annex).

Hall-only bosses (T-rex, FSM, Exploding
Gradient, Overflow, GNU-ton, Cut-the-Rope) stay real until each is placed in
the main game (Jon, 2026-10-06).

So a hall placement does not fire these archetype-keyed consequences:

- `QuestAdvanceEvent::BossDefeated(archetype)`;
- the signature gauntlet drop;
- the reward contents (`pirate_hoard`);
- Yarn `boss_cleared("cove.mockingbird")` is keyed by placement, so it is
  unaffected.

## Mode Collapse music (Q148)

Mode Collapse (`boss_encounters/mode_collapse_boss.ron`, a summoner that floods
the arena with identical copies) fights to `crooked_ascent_boss` in all four
phases. Ruling Q148 (2026-10-04): the final product wants a bespoke
"Mode Collapse" track, a loop that degenerates. `crooked_ascent_boss` is an
acceptable temporary authored fallback until it exists.

- Current: `crooked_ascent_boss`.
- Wanted: a bespoke degenerating loop.
- Status: art/content follow-up. It blocks no engine or gameplay work. When the
  track exists, register it and set the four `music_*` fields
  ([room music recipe](../../recipes/room-music.md)).
