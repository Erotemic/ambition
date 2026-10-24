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
  `dialogue_id` `perfect_cellular_automaton`, `brain_override: stand_still`). It <!-- cite-ok: a dialogue and character id in the content data, not a Rust item -->
  is also placed in `hall_of_characters`.

Remaining PCA work is encounter and narrative polish, not kit.

`imperfect_cellular_automaton` is a separate catalog character (a hall NPC). It
wears the PCA's move table. Whether it is a boss is an open design call.

Its art (2026-10-13) is an SVG rig whose bones have the PCA's names, so it
plays the PCA's clips, hunched. Its two halves differ (green keeps the rule,
purple is a fault), so each row is published from both sides and the game
draws the `~mirrored` row when it faces left. Open: its `standing_height` is
50 and the PCA's is 67.8, so it is drawn at about 80% of the PCA's scale; its
sheet states no attack volumes of its own; its tail goes under the ground in
the rows where it lies down.

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
- The call (`trex_call`, phase 2 on): a pair of one kind of his flock, the
  next kind each call (`FLOCK` in his conductor, twelve kinds): parrots,
  chickens, owls, kiwis, emus, European swallows, ostriches, African
  swallows, cassowaries, shoebills, vultures, penguins. The flyers come down from the
  high corners and the runners come in along the floor. A kind whose two
  sexes do not look alike comes as a cock and a hen (the chicken, the
  ostrich). Enraged, raptors along the floor first; at most four alive
  (`BossConduct::minions`). A new kind of bird is one row of `FLOCK` and one
  buildable row of the character catalog
  (`trex_fight::each_call_brings_the_next_kind_of_his_flock`).
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

## The Hunny Horror — the hug (2026-10-14, Jon)

Jon: "turn this character into a proper boss battle and replace one of the
filler boss battles in the hall of bosses". Ruled the same day: it replaces
the **Exploding Gradient** (hall-only, no art of its own, borrowed music, a
generic kit), and the Exploding Gradient is deleted, not kept unplaced
(its `gradient_nova` move with it). Its hall door and life switch lead to
`hunny_horror_arena` (`hall.hunny_horror`).

The art (renderer `hunny_horror_boss`): a stuffed bear the size of a door,
**sweet at rest and horrific when it attacks or is angry**. No red shirt (it
is not in the public-domain bear) and no scars (Jon).

**The rule of the fight: his face is the tell.** Sweet, he is harmless: his
body does not hurt to touch while he walks or sits. Every attack turns him
into the monster first (fur up, glowing eyes, teeth), and only then does
anything hurt.

**Distinct from the T-rex** (Jon, the same day: "make sure that this boss
feels distinct from the trex. Maybe incorporate a red balloon?", then "He
could use a red balloon to gain height and then pounce", and a honey throw
"that lands in a glob on the ground and causes a slow debuff to actors that
came in contact with it"). The first version echoed the T-rex three times (a
charge into the wall and a daze, a belly flop with a floor shock, a roar that
brought things down from the ceiling); those are gone.

| move (pattern key) | what it is | the dodge | the punish |
|---|---|---|---|
| swipe (`hunny_swipe`), maul from phase 2 (`hunny_maul`) | claws in front, the maul twice | back off / jump | after the follow-through |
| hug (`hunny_hug`) | arms open wide and a giggle; caught, squeezed and thrown | do not stand in the open arms; mash out | |
| honey (`hunny_spit`) | globs lobbed in arcs; each lands in a puddle that SLOWS any actor in it (it does not hurt) | leave the landing spots; jump the puddles | |
| red balloon (`hunny_balloon`) | a red balloon in his paw; he rises on it over the room, lets go and pounces at where you are; the balloon floats on and pops on the ceiling | move off your spot as he lets go | he sits dazed where he lands, bees round his head: the long window |

Each cycle ends with the balloon; enraged, it comes twice a cycle and the
honey five globs at a time.

The slow is an engine capability, not a bear one: `BodyCombat::slow` (a
timer and a speed fraction on every body, decayed with the other reaction
timers and in rollback state), applied in `step_body` (the one movement
seam of every body) through `MovementTuning::slowed`, and offered to modules
as `ambition.combat.slow_bodies` (`SlowBodiesPort`) in `boss_conduct`.

Architecture: the T-rex's road. The pattern is `boss_profiles.ron`; the
conducted module `ambition_content_modules::hunny` performs it; he wears his
character's sheet (`boss_sheets.ron`, `parts: npc_hunny_horror_boss`) and is
hit through his body box (no body rig). The balloon is its own picture
(renderer `red_balloon`), drawn as a projectile that rises with him. Receipts:
`game/ambition_app/tests/hunny_fight.rs`.

Open:
- His voice and his sounds are borrowed cues (the bear mauler, a laugh, the
  slime, the mockingbird's growl). Wanted: auditioned candidates first.
- Music: `crooked_ascent_boss` as a fallback. Wanted: a music-box lullaby
  that goes wrong.
- The room wears the `boss` biome. A nursery or a wood at dusk would fit him.
- A slowed body shows nothing on itself: the puddle's bubbling is the only
  sign. A honey drip on a slowed body is wanted.
- The balloon cannot be popped by the player: it is a projectile.
- No signature gauntlet: the Exploding Gradient's `sentry` gauntlet now drops
  from no boss.
- Tuning from `fight_discovery` and Jon's playtest.

## Wolfenstein — the leash (proposed 2026-10-17, Jon: "really make it something special")

Jon: "really polish the Wolfenstein boss fight ... beef up the boss
animations, and really plan an excellent metroidvania / hollow knight style
patterned boss." This section is the plan.

**Built (2026-10-18), Jon: "the boss to be scaled bigger ... an intro
presentation, more health, and a phase 3 where it injects itself with a
serum and becomes even bigger ... a better death sequence":**

- Body 84 x 140 (was 60 x 100). The conductor measures every volume on his
  art through `Body` (world units per frame pixel from his built box), so
  every reach, step and speed grows with him.
- 110 HP; phase 2 at 66%, phase 3 (the encounter's enrage) at 33%.
- Intro (8.28 s, the length of his cue's `awaken`; Jon: "an awesome intro
  into this fight"). The throne and coils stand by the door. He sleeps in the
  dark (`wake_radius` 380: an encounter sleeps until a player comes near),
  eyes faintly lit, a coil crackling, to `oppressive_crush`. Then: dark; the
  coils flash and show him on the throne; dark; more lightning; dark; the
  lights flicker on, the coils strike his bolts, he rises (`throne_wake`);
  the lightning finds him and he roars under his name card; he leaps back
  from you (`jump`, `fall`, `land_hard`). The dark, the flashes and the card
  are an engine capability, `ambition.boss.stage` (`rendering::boss_stage`).
  The first time, a cutscene (`boss_intro_wolfenstein_boss`) holds input.
- Phase 3, the serum: `serum_tell`, `serum_inject`, `serum_surge`; his guard
  is up for the beat, and the surge grows him to 1.3x
  (`ambition.boss.scale`, an engine capability: body box, quad and hurt
  volumes rebuilt at the new scale, feet planted). After it: a green aura,
  25% faster.
- Death: his `death` row (28 frames); the bolts sputter out, a broken howl,
  the fall, the last spark. Every boss's defeat holds the world 0.4 s, pulls
  the camera in, shakes, plays the defeat cue and chord, and bursts. His
  corpse stays; his chest drops (`galvanic_bolts`).

**Built (2026-10-17):** slices 1 and 2. Each pattern step plays at its own
authored lengths, for every boss (`073f40c5f`). The conductor
(`ambition_content_modules::wolfenstein`) performs every move below on his
current rows, the pattern is `Select` arms on where you stand, and the
validator has no finding for him. Receipts:
`game/ambition_app/tests/wolfenstein_fight.rs`. Since then: his own 36 rows
(a tell, strike and window row for each move, the flatline-and-awaken intro,
the arc lash from his neck bolts), and the Hall of Bosses' own copy of his
throne room, `hall_wolfenstein_arena` (placement `hall.wolfenstein`, a life
switch by its door). His sheet turns off the warm placeholder tint
(`attack_tint: false`): his tells are drawn. Not yet: the coils' part in the
enrage, effect sheets and sounds (borrowed). His music cue is built:
`galvanic_march` (score `tools/ambition_music_renderer/scores/active/galvanic_march.music.yaml`), bound
to his phases in `music_cues.ron` `boss_bindings` (Intro awaken, Phase1
march, Transition unleash, Phase2 and Stagger howl, Enrage storm).

**Jon's rulings (2026-10-17):** first "I want this to be a bipedal boss";
then, after GPT concept sheets: "we can get rid of the no-all-4's rule, some
of these all 4s concepts are good". He fights upright and goes down on all
fours to stalk, to gather a pounce and when he is spent. He wakes on the throne. He gets his own music cue: a stiff march
in phase 1 that detunes into a howling theremin theme in phase 2, both in
enrage. Playable first: the engine fixes and the conductor on his current
rows come before the rig and art passes.

**Today (read 2026-10-17):** a `Scripted` pattern of three generic strikes
(`side_sweep`, `floor_slam`, `full_body_pulse`), no conductor, one sheet row
per strike. Known defects:

- A `Scripted` pattern has no `intro` or `transition`: the pattern ticks only
  in Phase1, Phase2 and Enrage, and an authored `intro` is refused when the
  profile is read (BOSS-DRAMATIC-PATTERNS). His conductor plays his wake-up
  and his transition.
- A move kept the telegraph and strike lengths of its FIRST occurrence, so
  the faster phase 2 and enrage timings did nothing, for every boss. Fixed:
  one move per authored beat (`BossPatternCfg::beats`).
- The windup and the strike draw the same row. The 0.72 s `slash` row plays
  out in the 0.95 s windup, so the swing shows before the hit.
- `attack_side` and `attack_up` never show: three rows name `SideSweep`, and
  the first wins. Walk and run do not show.
- The generic volumes do not fit his art: `floor_slam` is below his feet,
  `side_sweep` reaches 15 px past his body.
- WOLFENSTEIN-FAIRNESS (`queue.md`): the enrage sweep has no punish window and
  no move has a telegraph identity.

**The look (Jon, the same day):** the Frankenstein half must read: "make
the whole body a lot more muscular, think lurch from The Addams family,
probably bigger though ... big green hands"; "a full coat ... I'm okay with
the coat being open or ripped to show chest and abs"; "he needs to have the
armband"; the head bigger, in proportion; a hand pose for each attack; the
slash trails drawn from the claw tips. The lightning plays off the bolts in
his neck. Art is hand authored; particles and shaders may add light and pop,
cheaply, and the fight must work with them off.

**Concept sheets (GPT, sent by Jon):** red-white trails for physical blows,
blue for current. Move ideas not yet built: a radial burst, a finger-gun
shot (the arc lash is its first form), summoned stitched hounds (the Attack
Hound exists), a storm of falling sparks (enrage), an electric ground ring
off the hammer, a fake death in the last phase, a reanimation surge, a
final-phase lunge chain.

### The fight's shape

**His failed objective: obey.** The regime sewed a wolf into a soldier and
gave it orders. The soldier obeys; the wolf cannot. Each phase, the leash is
looser. He runs on electricity (the bolts in his neck), and each big move
spends charge. He over-commits to the order and to the hunt. When the charge
is spent he goes flat: that is the long punish window, at the end of each
cycle.

**The arena is his laboratory throne room** (`castle_godwin_throne`, 1920 x
896): the iron throne, two Tesla coils, two side galleries and a high gallery.
The coils are part of the fight.

| phase (HP) | name | how he moves | what changes |
|---|---|---|---|
| intro | "It's alive" | sits dead on the throne | the coils arc into his bolts; he jolts, stands, howls |
| 1 (100-60%) | The Order | upright, a heavy lurch at you | drilled and slow; one move at a time; long tells |
| transition | The leash breaks | staggers to the coils | lightning hits him, his stitches flare, the armband tears, a howl |
| 2 (60-25%) | The Wolf | upright, hunched low, fast | lunges in chains, the wall bound, the pack |
| enrage (<25%) | Insanity | upright, twitching, faster | chains of 3; the coils fire across the floor |
| death | | | the charge dies out; a short howl, then "Sorry." |

### The moves (each is a `Special` that the `wolfenstein` conductor performs)

| key | what it is | the tell | the dodge | the punish |
|---|---|---|---|---|
| `wolf_rake` | claw rake, a step on each blow; 2 blows (phase 1), 3 with an overhead last (phase 2 on) | the arm cocks back, a snarl | back off, or jump over him after the first blow | after the follow-through |
| `wolf_hammer` | two fists high, slam; a shock runs along the floor both ways (2 in enrage) | he rises on his toes, fists up (long) | jump the shock | his fists stick in the cracked floor; he pulls them out |
| `wolf_lunge` | low, then a bite lunge across most of the room; he skids and turns | he drops low, hackles up, jaws open | jump over him, or go up to a gallery | the skid and the turn |
| `wolf_bound` (signature, phase 2 on) | a lunge into the wall, he kicks off it and lunges back | the same drop, and a look at the wall | jump the first lunge, then jump again | he lands in a crouch, out of breath |
| `wolf_howl` | head back, a ring that pushes you off him; phase 2 calls two Failed Prototypes once; enrage calls lightning | a deep breath, the chest heaves | walk out | the howl itself (he stands still) |
| `wolf_galvanize` | hands on his bolts; sparks mark three spots (one under you, galleries too); lightning strikes them | the sparks on the floor | leave the marks | his arms shake after it |
| `wolf_flatline` | the charge is spent: he sags, his head drops, the bolts sputter; then a coil arc jolts him back up | the bolts flicker out | none needed | the long window (about 2 s, shorter later, never under the floor) |

A cycle is chosen, not fixed: each phase is `Select` arms on where you are
(near: rake or hammer; far: lunge; above: galvanize; behind: howl), and each
cycle ends in `wolf_flatline`. Body contact hurts only while a move is live.

**Distinct from the T-rex and the Hunny Horror:** no charge into a wall with a
daze (T-rex), no pounce from above with a daze (the bear). His window is a
scheduled one that the tempo of the cycle builds to, not a stun from a
mistake. The wall bound uses the wall. It does not crash into it.

**Seed back to the library:** `wall_bound` (a dash through that rebounds off
the arena wall) or `flatline` (a scheduled exhaustion window).

### Animation: what "beef up" means

He is a Python person-rig character (renderer
`targets/characters/wolfenstein.py`, `_person_rig.py`). Every attack becomes
four rows: tell, strike, hold or recover, so each phase of a move has its own
picture and the strike frames play in the strike window. The rows:

- Body: `idle` (12 frames: a heaving breath, a head twitch, bolt sparks),
  `walk` (a heavy lurch, arms forward), `prowl` (phase 2: hunched low, fast),
  `turn`, `hit` (a heavy stagger), `death` (16 frames).
- Story: `throne_dead`, `awaken` (the jolt), `shock` (the lightning in the
  transition), `flatline` (a loop), `recharge`.
- Each move: `<move>_tell`, `<move>_strike`, `<move>_hold`, `<move>_recover`.

Rig work first, so that all person-rig characters gain from it: per-key easing
(in, out, snap, overshoot) and a squash and stretch control, as the robot v3
strikes have. The key poses go at the move's live-window times, with one snap
frame, a held contact frame and a follow-through.

The claw trails come out of the baked frames and become his own effect sheet
(`wolfenstein_claw`: three claw lines that travel and follow through). The
floor shock, the lightning strike, the spark marks and the coil arcs are
authored effect sheets too. No shader draws any of them.

The hit volumes come from his art, in sheet pixels, as the Hunny Horror's do.

### Slices, in order

1. ✔ **Engine:** each step's own telegraph and strike length holds, for
   every boss. (A pattern has no intro or transition: BOSS-DRAMATIC-PATTERNS.)
2. ✔ **Conductor** (`ambition_content_modules::wolfenstein`) with the phase 1
   moves on the rows he has now, and `game/ambition_app/tests/wolfenstein_fight.rs`.
   The fight is playable from here.
3. **Rig:** per-key easing and squash and stretch in the person rig.
4. **Art, phase 1:** idle, walk, hit, flatline and the four rows of rake,
   hammer, lunge and howl. Volumes measured from the art.
5. **Phase 2:** the prowl, the bound, the pack, the three-blow rake.
6. **Intro, transition, enrage:** the throne, the coils, the lightning.
7. **Effects and sound:** the effect sheets; auditioned sound candidates (the
   howl, the crackle, the sputter, the slam); his own music cue.
8. **Acceptance:** the validator has no Wolfenstein finding (then lower the
   roster pin and close WOLFENSTEIN-FAIRNESS); `fight_discovery` tuning;
   Jon's playtest.

## The Inugami — the wolf unsewn (Jon, 2026-10-20)

Jon: "add an additional phase to Wolfenstein where the head resurrects as an
inugami and you fight the floating spectral head as an additional boss".
Then: "I think a red spectral glow would be better".
Then: "maybe the spirits around the head should be blue. and a shader may be
in order here", and "the head should also get bigger in the fight. so it's a
giant head".

**The story.** When Wolfenstein dies, the lights come back on his body. Then
the seam round his neck gives: the stitches pop in red ghost fire, and his
head tears off. The fire catches in it, it grows, rises and howls under its
own name card. The wolf is free of the body that obeyed. When it dies, the
fire goes out of it, the spirit leaves as a wisp, and his own head drops to
the floor beside his headless body.

**Built (untested in play):**

- A second boss placement, `castle_godwin.wolfenstein_inugami` (the Hall of
  Bosses copy: `hall.wolfenstein_inugami`), with its own health (48), phases
  (transition at 50%, enrage at 20%), name card ("Inugami", "The Wolf
  Unsewn"), save record and chest (his `galvanic_bolts` chest moved to it).
- Engine: a `BossSpawn` can name the `BossSpawn` it wakes after
  (`wakes_after` EntityRef → `RoomSpec::boss_successions` →
  `BossConfig::wakes_after`). The successor sleeps until the save records the
  predecessor `Cleared`. Its conductor sees the predecessor's body
  (`ambition.boss.conduct` v9, `predecessor`). The health bar shows only an
  awake boss. A life switch sets the life of the whole succession. A fallen
  boss with no music state does not stop the cue that its successor plays.
- Art: renderer `wolfenstein_inugami`, his own head pieces
  (`wolfenstein.WolfHead`) in a red spirit ink (`SPIRIT`), with red ghost
  fire from the neck. His sheet adds `unsewn` and `headless`.
- A giant head: built 176 x 212 (he stands 140). It rises at the size his
  head was drawn on his body and swells to its build in the rise; it swells
  again to 1.15x from the transition and 1.3x at the enrage
  (`ambition.boss.scale`). When the spirit leaves it, it shrinks back to his
  head. Its sheet publishes at 1.68 texels a world unit (830 px frames, the
  median character's density), not the game's 3.4 (about 1700 px frames): so
  its frame fits the largest impostor cell, which its hit flash needs.
- The onibi round it are blue (sheet wisps and `inugami_onibi`, the cold
  fire's art); the head and its fire are red.
- Shader (polish): `ambition_render::spirit_field`, a whole-screen pass: near
  a `SpiritAura` the picture ripples like air over a fire and takes the
  aura's tint. `ambition_content::presentation::inugami_field` gives the
  risen head its red aura. With screen shaders off nothing else changes.
- Conductor: `ambition_content_modules::inugami`.

| move | the tell | the dodge | the punish |
|---|---|---|---|
| `inu_bite` | it draws back, jaws wide | step out of its line | it tumbles past and shakes its head |
| `inu_dive` | it flies up over you, nose down | leave the spot | its teeth in the floor (1.5 s, and 0.4 s to pull free; 1.2 s in all at the enrage) |
| `inu_onibi` | fire balls kindle round it | dodge the balls | none |
| `inu_howl` | head back | walk out of the ring | the howl (it stays still) |
| `inu_vanish` | it fades out | turn round: it comes back behind you | (a bite follows) |
| `inu_gutter` | its fire gutters | none needed | the long window on the floor |

**Blue spirits (2026-10-21).** Jon: "I want the Japanese ghost spirits
around him to be blue". Four blue hitodama turn round the head on a tilted
ring, behind it and in front of it. They go while it throws onibi, and while
it vanishes or appears.

**Kagero comes to your aid (2026-10-21).** Jon: "I also want a ninja to come
to your aid in the fight, appear on screen and do a few hits on the boss and
then fly off the screen (using flyline maybe). It should not be the Shadow
Oni Leader, a different ninja. We can have some story play in with that."

- Who: Kagero (`npc_ninja_kagero`), a kunoichi who hunts the spirits the
  regime makes. She followed the inugami into the castle. She is black with
  red eyes in a dark slit, as the ninjas are; the white of her scarf and her
  paper talismans says she is not of the dojo. Art: renderer `ninja_kagero`
  (first draft).
- When: once, at the first window the head opens (its teeth in the floor,
  or its fire spent) after you have been in the final hall with it for 3 s:
  her cuts land while it cannot guard or vanish. The conductor summons her
  on its far side from you
  (`ambition.boss.summon` v3: `side: Player`, `holds`).
- What: her own module performs her visit (`ambition_content_modules::kagero`,
  status `ambition.kagero_visit`, ticked in `technique_execution`). She comes
  out of her smoke and speaks, goes at the head, cuts three times (2 + 2 + 3
  damage, real hits), then her line drops from the sky, she catches it and is
  reeled up and away. She is dismissed when she is high or after 1.6 s.
- Her lines: "Noisy one. Even the dead heard you." and "It was a dog once.
  Set it free, ripple-foot." (the ninjas name the robot by its noise:
  `docs/brainstorms/llm_brainstorm.md`).
- Engine: `technique_execution` now also offers `ambition.presentation.drawn_row`,
  `ambition.feedback.speech_bubble`, `ambition.feedback.burst` and
  `ambition.feedback.body_sound`, so a module can draw, voice and sound a
  body it ticks.
- The line is the engine's body line (`ambition.feedback.body_line`), not
  Smash's flyline move: that move is a winch for a body on the axis-swept
  model, and her visit is a short scripted beat.

**Open:** the onibi use the fireball art (`inugami_onibi` in
`projectile_visuals.ron`); its own sounds (it borrows cues); its own music
state (it plays `galvanic_march` "howl" and "storm").

### Rework (Jon, 2026-10-22) — slices 1-3 built

Jon, after playing it: "there is a spot that the head cant actually pass on
the way to the final arena, the camera zones snap the camera in really weird
ways. Its just a very unbalanced fight. It's got a lot of really good bones on
it, but it needs to be executed correctly." Then: "You shouldn't be able to
move into a chase zone until you damage him enough in phase 1 of the fight,
maybe when he enrages he crashes into a wall so you can do the run and
chase", "if you get to the final arena before he does he just teleports
there, that is no good", "I did like the theramin to cue the player into
something being up with the head, but we should also probably start iron
toccata a bit sooner". On balance: "Its unpolished ... the spirit fires are
hard to reason about, but it just doesn't have a strong boss identity or
attack patterns. And it probably shouldn't get tired so much." And: "add
more animations and poses to the inugami boss. I.e. better art. Can also add
more art / animations / posing to wolfenstein too to make the boss move
smoother."

**Measured (2026-10-22, `inugami_fight.rs`):**

- At its transition the final hall's bounds put the head 7428 px down the
  castle for one tick (fixed: `arrived`, 5886379b5).
- The head (176 x 212) was held 11 s on a 48 px step at the throne room's
  door, and 10 s under the battlements' floor at the top of the tower: its
  run keeps it 40 px over your centre, so it drags along the floor.
- Out of the throne room the camera went from still to 38 px a tick; off the
  tower's top it made up 430 px on its own in under a second (fixed: zones
  mix across their edges, 0e4bf34ba).

**The fight's new shape (rulings; to build):**

1. **Phase 1 in the throne room.** The arena's walls stay up after
   Wolfenstein falls; the Inugami's phase 1 is fought in his arena.
2. **The crash.** At its transition the head goes back, flares, and crashes
   through the gallery wall: the wall breaks (a gated lock wall the conductor
   opens, with debris, shake and a slam), and the chase starts through the
   gap.
3. **The chase hunts you there.** It flies after you through the whole route,
   high enough that its jaws are at your height and its bulk is over you
   (not on the floor), and the route is widened where its 176 x 212 does not
   fit (Jon: "Widen the route"). If you are in the final hall before it, it
   comes in after you, at its own speed: no sweep, no teleport.
4. **Music.** Silence while it waits in his body. When the seam pops, the
   theremin (Galvanic March's `unleash`) cues that something is wrong with the
   head; Iron Toccata comes in under the rest of the rise (Jon: "When it tears
   off").
5. **Tired less.** It lies spent on the floor (the gutter) once a phase, at
   the end of its longest string, not at the end of every cycle.

**Identity — Jon: "Yes, build it".** The Inugami is a hunting dog of the regime, set
free, and its fight is the hunt and the leash it broke. Every move names a
hunting dog's verb and reads from its silhouette and its fire:

| move | tell | what it does | window |
|---|---|---|---|
| **stalk** (replaces the float) | low, ears flat, circles at your height | it keeps a hunting distance and paces you | — |
| **lunge** (the bite, kept) | it gathers back, jaws open, eyes flare | a straight dart with a fire trail; two snaps | it skids past, shakes its head |
| **pounce** (the dive, kept) | it climbs over you and points its nose down | drops jaws first; a shock along the floor both ways | its teeth in the floor |
| **broken leash** (NEW, signature) | the stitches at its neck burn and a ghost chain unrolls | the chain whips across the room in one arc: jump it or duck it | the chain falls slack, it pants |
| **the pack** (replaces the onibi) | the four blue hitodama drop to the floor and become wolf wisps | they run along the floor at you one at a time, in a visible line: jump each | — |
| **howl** (kept, readable) | head back, the ring is drawn on the floor first | a ring pushes you off; from phase 2 it calls the pack back | it stays still for the howl |
| vanish | — | REMOVED: a teleport, and Jon rejected the teleports | — |

Built (`inugami.rs`, `boss_profiles.ron`): each phase is three picks from a
table, then the gutter (once a cycle of three moves). Phase 1: lunge, pack
(3 wisps), leash, pounce, howl. Phase 2: a two-lunge chain, a pack of four,
and the howl sends a wisp each way. The enrage chains a lunge into the
leash, and the pack into the pounce. The chase: on your heels, every 2.4 s a
lunge at your back (gather, dart, skid: the skid is unguarded) or a wisp
along your floor, in turn. The leash hurts only at its tip, which crosses
the floor under it from behind to in front over the strike; its slack (1.2 s)
is a window, and Kagero comes in at it too. Kagero stays as built.

**Art — Jon: "Procedural, more rows".** Built: the Inugami got `hunt`,
`crash`, `pack_tell`, `pack` (one bark a wisp), `leash_tell`, `leash_whip`,
`leash_slack` (the chain is drawn from its neck to its frame's foot; its tip
is the game's sparks), and more `idle` and `howl` frames; `onibi`, `vanish`
and `appear` are gone. Wolfenstein got `turn` (drawn for 0.24 s when he turns
between moves) and more frames on `walk`, `prowl`, `run`, the three tells and
the four recoveries. Not built yet from the list below: the Inugami's
`turn`, `hurt_flinch` and squash on the darts; Wolfenstein's in-betweens
from a tell into its strike. The proposal was:

- Inugami: new rows `stalk` (a pacing loop), `lunge_gather`, `lunge_skid`,
  `pounce_turn`, `leash_tell`, `leash_whip`, `leash_slack`, `pack_call`,
  `crash` (into the wall), `hurt_flinch` (two directions), `turn` (it turns
  to face you over 4 frames, not a flip), `pant`; more frames on `idle`
  (breath and ears), `bite` and `howl`; and squash and stretch on every dart.
- Wolfenstein: in-between frames where his moves now cut (`walk` to `prowl`
  to `run`, each tell to its strike); a `turn` row; settle frames after
  `hammer_pull`, `lunge_skid` and `rake_recover`; more frames on `walk` and
  `run`.
- Whether the art stays procedural or becomes an authored SVG rig (as the
  robot's) is Jon's call.

**Slices, in order:** (1) the shape, music and route (rulings, no taste
call); (2) the identity, after Jon's yes; (3) the art, after Jon's yes.

## The Mockingbird — air chase (proposed 2026-10-06, Jon; built 2026-10-06)

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
   rig). The sheet loops its row (the shark's `idle`) and faces the way the
   platform flies through the air: the sharks drift toward the Mockingbird
   slower than the sky scrolls past, so they face away from it
   (`the_mockingbirds_sharks_flee`).
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

**Built (2026-10-06).** The four room features and the boss, as proposed:

- **Rooms.** `mockingbird_sky` (1280x720 at camera zoom 1.6, placement
  `cove.mockingbird`, 40 HP) sits above `mockingbird_arena`, which is now the
  shore: no boss, a hole in its ceiling, and a lift shark
  (`MovingPlatform.lift_dy`) that waits for a rider and carries them up into
  the sky. Falling out of the sky's bottom after the kill lands you on the
  shore. The hall has its own copies (`hall_mockingbird_sky`/`_arena`,
  `hall.mockingbird`, practice).
- **Room features.** `sky_scroll` (px/s) wraps the parallax in 4 mirrored
  panels (`ParallaxWrap`), and the new `open_sky` backdrop is a cloud sea with
  no sun or vignette. Platforms gain `loop_dx`/`loop_min_x`/`bob`
  (`HorizontalLoop`), `lift_dy` (`Lift`), `one_way` and `visual` (drawn from a
  sheet via `PlatformLooks`). `fall_rescue` (a carrier sheet) spawns a ferry
  under a falling body while a boss in the room lives. `boss_reward_drop`
  makes a room host that boss's chest, which falls from the top centre.
- **The boss** is conducted (`ambition_content_modules::mockingbird`) and owns
  its own position. It is guarded (hits clang, `BossGuard`) except while
  diving, winded, flying home or on the strafe's low pass. Phase 1: missile,
  near snap or far fireballs, dive-bite. Phase 2 opens with a screech: missile
  salvo, double dive, fireballs. Enrage: a strafe run that drops bombs, and the
  5-ball fire fan. Shot down, it falls trailing smoke. New art rows: missile,
  dive, chomp, stunned and screech; a missile prop sheet; 12
  `boss.mockingbird.*` SFX.
- **Tests:** `game/ambition_app/tests/mockingbird_fight.rs` (12).

**Asked (Jon, 2026-10-09), built the same day: anger, cold fire, space and
the moon.** "After enough damage the mockingbird needs to light itself on fire
(not because it is damaged, but because it is angry). That is phase 2. Then
after more damage it triggers phase 3 where the red fire turns into blue fire
(cold fire) and the mockingbird gets even angrier. The mockingbird also needs
to shoot lightsabers from its mouth, which are also on fire. And it turns out
that the only way to kill a mockingbird is by hitting it with the moon."

- **Phase 2 (60% HP): it lights itself on fire.** Red flames come off its own
  drawn body (`presentation/mockingbird_sky.rs`, `mockingbird_fire.wgsl`: a
  quad over its sprite that asks, for each pixel, whether there is body below
  it and downwind). New move `mockingbird_lightsabers`: three burning
  lightsabers spat from its mouth in a fan. They spin, fly out and come back
  (`ProjectileSpawn::boomerang_return_s`). Prop sheet
  `mockingbird_lightsaber`, rows `fly` and `fly_cold`. The spin showed an
  engine defect, now fixed: an animated projectile drew each trimmed frame in
  its first frame's box. Each frame is now drawn at its own size and place
  (`projectile_visuals::place_frames`).
- **Phase 3 (25% HP): cold fire, and the fight leaves the sky.** The fire
  turns blue and burns higher; its fireballs and lightsabers are cold. Its
  pattern gains a single dive and the lightsabers. Its hull now turns each
  blow, also while it dives and while it hangs winded: no blow of yours hurts
  it again. The sky climbs into space in `ASCENT_S` (7 s): the dark comes
  down from the top, the stars come out and the planet's limb falls to the
  bottom of the room (`mockingbird_space.wgsl`). The sharks stay: the footing
  does not change.
- **The moon.** In space the moon crosses the room from right to left on a
  lane (a 5 s crossing each 10 s; the lane is drawn as a dashed arc). It hurts
  a body it rolls over. It kills the Mockingbird if it reaches it while it
  dives or hangs winded after a dive. So the finish is a lure: stand where its
  dive at you ends on the lane while the moon is coming. A dive that ends off
  the lane changes nothing, and it does not go to the lane by itself: when
  the moon comes at its side of the sky, it climbs over the moon and holds
  there until the moon is past.
- **The death is the usual one.** The moon's blow is a real hit, an
  environmental one (`HitSource::Hazard`, which no guard turns), sent by
  `strike_it_with_the_moon`. So its bounty and gauntlet fall as for each boss,
  the sky comes back down in 5 s, and you fall to the shore where the chest
  lands.
- **State.** The phase, the climb (`ascent`), the moon's clock and position
  and `moonstruck` are fields of the conductor's record, which is rollback
  state. Presentation reads the record and decides nothing.
- **Tests:** three more in `mockingbird_fight.rs` (15): the lightsabers fly out
  and come back; cold fire, the climb and the turned blows; and the moon
  (control: two crossings with the player under the lane leave it alive;
  lured, it dies).

Limits, stated:

- "Only the moon kills it" holds in play, not as an engine rule. One blow
  larger than a quarter of its health, landed in phase 2, would kill it with
  no moon. No weapon the player has does that. The hall's kill switch and a
  test's 9,999 blow still kill it, as they must.
- Gravity does not change in space. A lower gravity needs a room-rule port
  that the conductor does not have.
- No new sound: the lightsabers use the fan's charge and spit cues, and the
  moon's strike uses the chomp cue and then the defeat cue. New cues wait for
  an audition.
- The looks are checked with `capture_scene`. A window on a GPU was not seen.

Open:

- The SFX are first picks and have not been auditioned.
- A rescue costs no HP. A chip of damage per catch would make falling matter.
- Tuning lives in `mockingbird.rs` (speeds, stun lengths) and
  `boss_profiles.ron` (tells, rests, the pattern per phase).

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

Hall-only bosses (T-rex, FSM, the Hunny Horror, Overflow, GNU-ton,
Cut-the-Rope) stay real until each is placed in the main game (Jon,
2026-10-06).

So a hall placement does not fire these archetype-keyed consequences:

- `QuestAdvanceEvent::BossDefeated(archetype)`;
- the signature gauntlet drop;
- the reward contents (`pirate_hoard`);
- Yarn `boss_cleared("cove.mockingbird")` is keyed by placement, so it is
  unaffected.

## GNU-ton — a giant with arms, and portals (proposed 2026-10-11, Jon)

The giant's new look is in the game, with the fight of today (see
`art-and-ambiance.md`, "The giant gnu, redrawn"). The fight below is NOT
built. Concept stills and animations are in `untracked/giant-gnu-candidate/`
(not in git).

**What the new giant changed in the fight of today (2026-10-11).** The
giant is larger, so its numbers moved: the scholar's seat is the right
shoulder, 464 above the floor (it was 217); the `back` the conductor reads
is the line of the two shoulders; the fists rest 400 to each side of its
centre (270). The choreography's own heights did not move: `HOVER`,
`CEILING`, the pivots, and the orbit about the scholar are numbers of the
room, and with the scholar higher the orbit is higher over the floor than
it was. The fight was not tuned again for this: that is the rework.

**The body.** A giant like a gnu, almost a centaur: a beast on four long
legs, a chest that stands up at its front, two arms to two fists, a gnu's
horns. Its back is a slope from a low rump to the left shoulder, not a flat
floor. It has two stances: on all fours, and reared up (the slope steeper,
the shoulders higher). Moss shows where an actor can stand (the slope, the
level part of each horn); each shoulder has an epaulet.

**The fight Jon proposed.**

- Phase 1 (short). The fists are on the arms, so they do not reach far. The
  scholar opens a REAL portal pair for a time: one flat line by the giant's
  shoulder, one flat line over the player's head. The giant lifts a fist
  and punches DOWN into the near line, and the fist comes down out of the
  far one. A punch down needs no turn of the giant's chest. While the pair
  is open the player can jump up into the far line (a double jump: for now
  the fight is made for the full set of abilities) and come up out of the
  near one, by the shoulders, to strike the scholar. A punch up through a
  line under a player who jumps, or a line at an angle, were thought of and
  not chosen.
- The portals are NOT in the sprite. The game draws them with the engine's
  portal effect: a thin line of light, two places of the room made one.
- The blow is ONE smooth motion, and the arm goes into the line square to
  it (Jon, 2026-10-11). The arm is one picture at each moment. It is drawn
  on each side of the pair and cut at the line, so what has gone into the
  near line is what comes out of the far one, at the same angle. An arm of
  two stiff bones goes in on a slant, and where it crosses the line moves
  along it. So the arm WARPS: it is one curve from the shoulder to the
  wrist, its end held square to the line, and it is longer in the blow than
  at rest. Concepts as animations: `untracked/giant-gnu-candidate/`
  (`punch_down.gif`: down into a flat line; `punch_side.gif`: across into
  an upright one; `punch.gif`: the stiff arm, for comparison). With the
  warp the lines stay flat or upright, which the engine does exactly: a
  line at an angle is not needed for this.
- After a few hits the scholar is angry (Jon, 2026-10-11, the second
  telling). He calls two portals and the arms go through them. Then he does
  a new thing: the portals CLAMP DOWN about the arms until they are tight
  to them. From then they are not real portals: they are the portal LOOK
  only, the arm fills them, and the player can not use them. The far ends
  move about the room with the fore arms and the fists, in the patterns
  that today's two fists have.

**The order of the work (Jon, 2026-10-11).** First the new giant goes into
the game, with the fight of today. Then the fight is made again, in these
phases. A portal at an angle may look better and Jon wants angled portals
in time; they are not a part of this.

**What the engine has for this (read from the code, 2026-10-11; not run).**

- A timed portal pair in free air exists: a move of the smash demo opens
  one and closes it on a clock (`game/ambition_demo_smash/src/portal.rs`).
  Each body with kinematics transits it, the player also.
- A portal with no partner is drawn and does nothing. That is the portal
  look of phase 2.
- The look of a portal is a line of light from a shader
  (`ambition_portal2d_presentation`), not a ring. The ovals in the stills
  stand in for it.

**What it does not have.**

- The conductor is an extension module and has no port that opens a portal.
- A fist is posed at an absolute place each tick. A fist that transits is
  put back on the entry side by the next tick's pose.
- A fist's damage box is a box in the world at the choreography's place.
  It is not mapped through a portal: a fist half through hurts nobody at
  the exit.
- A fist (about 136 x 114) is larger than the default opening (92 long).
  Both ends of a pair take the smaller size.
- A portal can not ride the giant. A free portal that moves 1 px or more is
  taken as a closed plane, and bodies in it are pushed out.
- No body can refuse a portal: the scholar, a thrown item, a shot use the
  pair as the player does.
- A boss's pair needs its own end: a clock in rollback state, an identity,
  a despawn on defeat and on a reset of the arena.

**The shape that needs least (a proposal, not decided).** In phase 1 the
pair is real for the player, and the fist does not transit it: the
choreography poses the fist on the exit side (it is a body of its own, as
today) and the arm is drawn into the entry. Then the fist's place and its
damage box are where the choreography says, as they are today. Open: the
size of the opening against the size of the fist, and what the pair does
when the giant bucks or rears.

**Open, for Jon.** Whether the player can walk the slope (about 36 degrees
on all fours, 46 reared; not checked against the engine). Which shoulder
the scholar stands on. Where the two fists rest.

**Built 2026-10-11 (first cut; seen in `capture_scene`, not in a window on
a GPU). What is not elegant in it: `gnu-ton-fight-debts.md`.**

- Phase 1: the giant's arms are on it (rows `armed_*`, `punch_*`). The
  pattern is one move, `portal_punch`. A real pair opens: the near line by
  the right shoulder, the far line over where the player was when the
  telegraph began. The fist's body is put under the far line (rows
  `through_*` of the fist's sheet) and hurts there. A player that was put on
  the near line came out of the far line (seen). The way UP was not played.
- Phase 1 ends at 64% of his health (`phase1_to_transition_hp`): Jon wants
  it a full phase, not a short one. Its pattern is a slow punch and a quick
  one; it needs more moves (the left arm, a punch to the side).
- The clamp: the encounter's lock between the phases (2 s). Two wide lines
  open and the arms go out into them (row `clamp`); the fists come out as
  bodies of their own; the lines close on the arms until they are as wide
  as the arm (`arms::tight`); the giant flinches (row `hit`). Then the
  fight of before goes on (its phase 1 and its phase 2 are now ONE phase 2).
- He says four lines, once each (`Bark` in the conductor): at the first
  punch, when the player is on the giant's back, at the clamp, and when
  the lines are tight. First words; Jon has not read them.
- The lines at the shoulders and behind the wrists are a look
  (`PortalLightLine` of the portal presentation crate), not portals: a free
  portal that moves pushes each body on its plane.
- Where: the module `ambition_content_modules::gnu_ton` (`choreography::arms`,
  `conductor`), `ambition_content::bosses::gnu_ton::portals`, the rows of
  `giant_art.py`, `boss_profiles.ron`.

**How the fight could play better (2026-10-11; Jon: "rework and improve
how this fight plays out", phase 2 does not need to be the fight of
before; and of this list: "Your proposals make sense").** Each line is a
change that can be made alone. BUILT 2026-10-11: 1 (three blows, the one
and the two; not "faster on the floor"), 2 (the lock; the one-two is two
quick punches 0.5 s apart), 3 (he goes to the other shoulder after a
blow), 4, and of 5 the lines that come open when he is dazed. NOT built:
the left arm and the punch along the floor (art), barks at each move, the
bright trail, and 6 (it needs rows with one arm on and one in its line).

1. *Phase 1 has one question: "where is the way up?"* The far line is the
   threat and the way. Make that plain: the first punch is slow and he says
   so; a player on the giant's back gets two or three blows before the buck
   (today the first blow starts it); a player who stays on the floor gets
   punches that come faster.
2. *More punches from the same arm.* A punch that follows the player until
   a lock (as the demonstration does): BUILT 2026-10-11, the far line
   follows at 360 through the tell and stays for its last 0.3 s. Two punches with one opening: the
   fist goes back up and comes down again where the player went. Later,
   with art: the left arm, and a punch along the floor out of an upright
   line (a thing to jump over).
3. *The scholar is in the fight.* On the back with him, he is not a target
   that waits: he steps to the other shoulder, or he calls the buck. In
   phase 2 he is who you watch: each move starts with his row and a bark,
   so the fists are read from him.
4. *Phase 2 is shorter loops that answer the player.* Today it is one list
   of 35 seconds. Three short lists, chosen by where the player is: near
   the giant (cradle, stomp), far (demonstration, pendulum), on a perch
   (fluxions). The apple rain and the eureka come at a part of his health,
   not at the end of a list.
5. *The lines stay part of phase 2.* The trail from a shoulder's line to its
   fist is his spell (Jon). A fist that is stuck pulls its line tight and
   the trail goes bright: the sign of the punish window. When he is dazed
   (eureka) the lines flicker and go wide: the arms nearly come out.
6. *The enrage is the pair again.* Below the last part of his health the
   clamp breaks: one arm comes back onto the giant and punches through a
   real pair while the other fist is still a body of its own. The way up is
   open again for the last blows.

**Known, not done.**

- The tests of the fight were brought up to it (the conductor's parity
  test, the arena's tests, the two-room apple rain). No test plays phase 1.
- The limb trail (wisps from the giant to each fist) is still drawn: to the
  far fist of the punch, and beside the arms in phase 2. It needs a way for
  a body to say it has no trail.
- The places of the lines are typed in `choreography::arms` from the
  drawing (debt 5 of `engine/the-art-says-where-a-body-stands.md`).
- When the giant bucks, the lines stay at the place of the giant at rest.
- The hall of characters shows the giant with its arms cut at two lines
  that are not drawn (no conductor is there).
- The way up is PLAYED by a test
  (`the_punchs_pair_is_the_way_up_to_the_giants_back`): from the floor, two
  jumps go up into the far line (145 over the floor), the player comes up
  out of the near line, and goes to the side onto the giant's back. For
  that the near line is OVER the right shoulder (68 over the back): by the
  shoulder it was too low, and the player came out with no rise left.
- Not tuned: the times, the damage, how many blows end phase 1.

**Aimed blows, a kick, and their weight (built 2026-10-24; Jon, 2026-10-23:
"it should track where the player is ... down diagonal, horizontal, or up ...
sometimes its a kick with a leg ... it should feel dangerous").** Where:
`choreography::blows` (the module), the conductor, the rows `punch_*`,
`jab_*`, `upper_*`, `kick_*`, `armed_kick_*` (`giant_art.BLOWS`), and the
pattern in `boss_profiles.ron`.

- Each blow goes into a near line that is fixed to the giant, square to it,
  and comes out of a far line by the player. Four limbs: the hammer (down
  from over the right shoulder), the jab (straight out), the uppercut (up in
  front of the chest), the kick (the near fore leg, forward and low). The
  far line is at an angle when the blow needs it: the pair turns the limb.
- The conductor chooses the exit by where the player is: down or at 45
  degrees onto a player on the floor (at a slant near the giant's flank),
  across or up at a player off the floor, a sweep along the floor or a hoof
  down from over the head for a kick. No blow is the one before it. The first
  punch is straight down: it shows the way up.
- `portal_kick` is a move of its own, in phase 1 and in phase 2: with the
  arms in their lines, the hoof keeps a real pair in the fight, and the way
  up to the back. The hoof is a limb body (`hoof_right`, from
  `CharacterHands::more`) that the giant's picture draws.
- Tell: the pair eases open over the first 0.4 s of the tell and closed at
  the end of the strike; the windup row plays over the tell and ends cocked
  and trembling (the lock); a dotted line shows the way the limb will come.
  Each tell has its own length in the pattern.
- Weight: the strike that lands (`ambition.boss.mount` v3 `struck`) shakes the
  camera hard, holds the limb and the giant still for 0.11 s, and throws a
  flash, sparks and shards; the launch is 2.2 to 2.6 of the standard one. A
  blow that misses hits the floor (dust, a boom, a shake) or the air. Sounds
  (`boss.gnu_ton.blow_*`, `hoof_kick`, `portal_open`) are generated; the
  candidates are in `untracked/sfx-candidates/gnu_ton_blows/`.
- Not done: a recoil row of its own after a land (the hold row jolts); the
  giant's own picture through the far line is cut at the frame's edge if a
  limb goes farther than the jab does.

**Placed in the main game (2026-10-23).** Jon asked for a gravity and
portal puzzle level with GNU-ton at its end. The level is the GNU-ton
gravity handle, the gravity works of the Science dimension (`science.ldtk`;
Jon, 2026-10-24: GNU-ton lives in the Science dimension, with its orbital station):
four puzzle rooms (up fields; side fields; portals; a room that turns over)
and his arena `gnu_ton_shoulders`. The arena is the hall's arena as it is,
with his placement `gnu_ton_handle.gnu_ton`. A `LockWall` gated on
`boss.cleared gnu_ton_handle.gnu_ton` opens the door home. Room by room:
`tools/ambition_ldtk_tools/specs/gnu_ton_gravity/README.md`. Proof:
`game/ambition_app/tests/the_gnu_ton_handle.rs`. Open: the hall's arena is
not yet `practice` (the rule above for a hall boss that has a main-game
placement; T-rex and the FSM are not either).

## The Smirking Behemoth — its eye beam (2026-10-13, Jon)

Jon: "his eyes glow, but they should shoot some sort of laser beam or
projectile at the player ... now the eyes just glow and nothing happens."

MEASURED: the simulation did fire (five shots each strike). Three things made
that invisible, and each is changed:

- It shot only once it had arrived (`suppress_attacks_while_moving`), so each
  beam was fired at a player that touched it, across its own body. A player
  that kept away was never shot. It now shoots as it walks.
- The shots left from the top of the body. They now leave from the eye
  (`projectile_origin_offset`, from the sheet's eye point).
- The shots had no look, so the game drew the engine's plain orange boxes. The
  `eye_beam` projectile look draws a line of bolts of light (renderer target
  `behemoth_eye_bolt`) that reads as one beam.

Witness: `game/ambition_app/tests/the_behemoth_shoots_from_its_eyes.rs`.

Open: a shader effect on the beam (Jon: "maybe some cool shader animation");
a projectile is a plain sprite today, so this needs a material road for
projectile looks. The telegraph shows only the eye's glow: there is no aim
line. The pillar by the door is cover from the beam, by the map.

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
