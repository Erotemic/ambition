# Bosses (game content)

The boss *system* is engine work
([`../engine/boss-system.md`](../engine/boss-system.md)). This page is the
**design language** and the specific bosses. Generic machinery (fighter-brain
verbs, the glider projectile primitive, `CharacterAnim::Special`, the
dialogue→provoke Yarn command) belongs to the reusable actor, projectile,
animation and interaction owners, not to a common `core`. A boss's stats,
tuning, placement and dialogue are game content in `ambition_content`.

## Current state and outstanding acceptance (2026-11-06 audit)

This is the **content/design owner**, not a second execution queue. Built systems
and moves are described below so the intended encounters remain recoverable;
remaining acceptance work is recorded here until it is selected in
[`../queue.md`](../queue.md). Unselected work is indexed in
[`../tracks.md`](../tracks.md). Dated measurements describe their recorded
build, not necessarily current HEAD; revalidate before reopening a fix.

| Encounter | Remaining acceptance / follow-up |
|---|---|
| PCA / Imperfect CA | Encounter and narrative playtest; decide whether Imperfect CA is a boss; review its scale, unowned attack volumes and tail clipping |
| T-rex | `fight_discovery` balance run and human playtest |
| Hunny Horror | Audition voice/SFX/music; consider biome, slowed-body feedback, poppable balloon, orphaned sentry gauntlet; run fight discovery/playtest |
| Wolfenstein | Verify post-rework move/pose/effect coverage and current roster fairness findings; finish missing art/cues/coil involvement and playtest, not a default short-reach rewrite |
| Inugami Wolfenstein | `inu_storm` seed/telegraph and slam-band review; investigate four dated `content_it` findings; audition cues/music and playtest chase, ward and camera; validate baked fallback/hurt expressions |
| Mockingbird | SFX audition, rescue cost decision, fight tuning and GPU look review; optional space-gravity and alternate-art follow-ups |
| Hall of Bosses | Verify whether main-game boss copies need conversion to `practice` without changing hall-only bosses' story semantics |
| GNU-ton | Review remaining presentation, line/limb, arena/practice, tests, tuning and GPU issues against [`gnu-ton-fight-debts.md`](gnu-ton-fight-debts.md) and current phase-3 implementation |
| Smirking Behemoth / Mode Collapse | Optional eye-beam material/aim tell; bespoke degenerating Mode Collapse music |

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

**Built encounter.** The T-rex is a grounded, art-sized, part-rigged boss;
body, head, tail and legs expose authored hurt geometry and strike poses.
`ambition_content_modules::trex` conducts the phase pattern and performs
its moves, while the art-owned jaw attachment positions a captured body.
The engine's `ambition.combat.body_hold` supplies seize, shake, throw and
mash-out, rather than a T-rex-specific capture path. The phases use these
moves and cues:

| phase | attack vocabulary | opening / escape |
|---|---|---|
| Hunt | `trex_bite`, tail whip, ledge snap, charge into wall | dodge bite/tail, leave ledge, punish the lowered head or stunned wall crash |
| Quake | double bite, stomp with shockwaves and falling rocks, jaw grab, bird call | jump shock, move from shadow, mash free of the jaw; punish recovery |
| Extinction | blowback roar, aimed leap/quake, returning charge and raptor call | evade marked landing and charge reversal; punish the crash |

The sheet contains `stunned`, `snap_up`, `leap`, `grab_reach`, `grab_shake`
and `grab_throw` rows. The flock cycles through twelve authored birds
(parrots, chickens, owls, kiwis, emus, European swallows, ostriches,
African swallows, cassowaries, shoebills, vultures, penguins), alternating
kinds on successive calls and entering from the appropriate floor/sky
edges; raptors precede the enraged call and at most four minions live.
Some birds arrive as a cock/hen pair. Each tell has its own creature-voice
recipe (`boss.trex.*`), including phase scream, growls, tail whip, crash,
roar and death; the latter plays only on an observed live death. The
renderer is `npc_trex_enemy` / `trex_enemy_body_rig.ron` and the authored
rig includes the held-jaw landmark. Tests: `game/ambition_app/tests/trex_fight.rs`
and `game/ambition_app/tests/the_trex_is_drawn_from_his_parts.rs`.

Still open: `fight_discovery` tuning and Jon's playtest.

Acceptance: run `fight_discovery`, validate that readable tells precede
strikes and that every move has an honest escape and punish, and playtest
with the actual character capabilities. Candidate roar auditions are in
`untracked/sfx-candidates/trex_roar/` when available.

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

**Built state.** The conductor (`ambition_content_modules::wolfenstein`)
performs the authored `Select` patterns at phase-specific telegraph/strike
lengths (`game/ambition_app/tests/wolfenstein_fight.rs`). The body was
scaled to 84 x 140, 110 HP, with thresholds currently recorded at 66% and
33%. The 8.28 s throne-and-coil intro uses `ambition.boss.stage`; the
serum enrage scales him by 1.3x through `ambition.boss.scale`, grants a
green aura and raises speed. The death sequence leaves his corpse and
`galvanic_bolts` chest. The Hall owns a separate `hall_wolfenstein_arena`.
The character has attack-specific rows, bolted-neck lightning, and the
`galvanic_march` music phases. Some conductor effects and sounds still
use borrowed sources, and the coil's enrage role is not fully realized.
The older `Scripted`-only, identical-length, single-row attack defects
were repaired during this implementation, not outstanding prerequisites.

**Jon's rulings (2026-10-17):** first "I want this to be a bipedal boss";
then, after GPT concept sheets: "we can get rid of the no-all-4's rule, some
of these all 4s concepts are good". He fights upright and goes down on all
fours to stalk, to gather a pounce and when he is spent. He wakes on the throne. He gets his own music cue: a stiff march
in phase 1 that detunes into a howling theremin theme in phase 2, both in
enrage. Playable first: the engine fixes and the conductor on his current
rows come before the rig and art passes.

**Acceptance still owed.** Recheck the current roster validator before
claiming a Wolfenstein fairness regression; its historical telegraph and
punish findings predate the authored conductor and later boss fixes. Do not
reintroduce them as already-proven defects. Finish omitted effect sheets,
audit bite/reach/volume against the visible pose, audition borrowed sound,
and tune the fight with `fight_discovery` and a real playtest.

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
| 1 (100-66%) | The Order | upright, a heavy lurch at you | drilled and slow; one move at a time; long tells |
| transition | The leash breaks | staggers to the coils | lightning hits him, his stitches flare, the armband tears, a howl |
| 2 (66-33%) | The Wolf | upright, hunched low, fast | lunges in chains, the wall bound, the pack |
| enrage (<33%) | Insanity | upright, twitching, faster | chains of 3; the coils fire across the floor |
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

### Remaining polish and acceptance

The staged implementation established its own attacks, intro, phase 3,
cutscene, music and defeat. Revalidate rather than automatically reopen
historical slices 1–7. Open work is the remaining art/easing/pose coverage,
per-move sound and particle treatment, explicit coil behavior at enrage,
any *current* roster punish/telegraph findings, and `fight_discovery` plus
human acceptance. The scene's stated phase percentages above describe the
original dramatic plan; actual recorded thresholds are 66%/33% until changed
intentionally. Suggested but **not established** attacks (such as summoned
hounds, storm of sparks, fake death, final lunge chain) remain optional
content ideas, not missing shipped mechanics.

## The Inugami — the wolf unsewn (Jon, 2026-10-20)

**Current identity.** The Inugami is Wolfenstein's detached spectral head,
freed from the obedient body. When Wolfenstein falls, the neck stitches
rupture in red spirit fire, the head grows and rises with its own title,
**Inugami Wolfenstein**. When it dies, the spirit leaves as a wisp and
his head falls beside the body. Its own placement and health belong to
the successor boss (`castle_godwin.wolfenstein_inugami`, plus the separate
`hall.wolfenstein_inugami`); `BossSpawn::wakes_after` / boss succession
ensures it remains asleep until Wolfenstein is cleared. The Inugami owns
its save record, phase progression, health bar and final chest holding
`galvanic_bolts`.

**Visual direction.** The head and severed-neck fire glow red. Four blue
hitodama orbit its head **only while warded**, not as its ordinary aura;
`ward_ring_kindle`, `ward_ring` and `ward_ring_dispel` are separate drawn
layers attached to the head's `skull` track, including correct behind/in-front
ordering and mirrored turns. A `SpiritAura` may add a screen-space ripple
but the encounter must work with shaders disabled. The head grows to
176 x 212, then 1.15x and 1.3x at later phases, and shrinks as the spirit
leaves; published texel density is intentionally lower for the large
impostor cell. Kagero (`npc_ninja_kagero`) is a separate red-eyed ninja
with a white scarf and paper talismans, **not** the Shadow Oni Leader.
Her authored lines include "Noisy one. Even the dead heard you." and
"It was a dog once. Set it free, ripple-foot." Preserve dialogue
continuity if her combat role changes.

**Encounter sequence (built through 2026-10-26).**

1. **Throne room duel.** The successor rises only after Wolfenstein's
   death. The health bar begins after the rise/intro. Its phase 1 is a
   regular fight in the throne room, without its protective wisps.
2. **Ward and escape.** At transition the hitodama kindle and make the
   head invulnerable. It crashes through the east gate (crumble/debris),
   then withdraws overhead to let the player through.
3. **The chase.** A continuous route goes right, **down** a shaft and **left**
   beneath the castle, returning near the gatehouse. The head pursues
   the player at a distance derived from measured target speed and
   traveled route length. `FrameAlongside` lets the camera frame both when
   the gap permits; it must not use the camera to drive motion, snap across
   segments or teleport into the final arena. While warded, alternating
   ghost-fire lunges and marked lightning drive the player **forward**
   along the escape path, not backward; body contact alone is not damage.
4. **Kagero's spell.** In the final hall, when the wolf has followed within
   range, Kagero appears and casts talismans (`cast` / `ward_bound` /
   `dispel`) to extinguish its blue ward. She does not physically cut the
   head. She departs on her line; her older three-hit attack proposal is
   superseded by this spell.
5. **The hall fight.** The unwarded head fights through phase 2 and enrage
   and leaves the Wolfenstein head as a corpse on defeat.
6. **Return route.** A caged drain lever available after boss defeat
   crumbles cracked wall/floor/niche looks and opens a passage back to the
   castle gatehouse. Lock walls with a `break` row crumble; other looks
   slide into their lintels. The crack and its hidden breach do not appear
   open before the correct state change.

**Attack language.** The hunting dog stalks and gathers before its lunge,
pounces jaws-first onto a marked floor, lashes the **tip** of a broken
ghost leash across the room (its slack is a punish window), howls with an
outlined ring, and recharges after a longer string rather than lying tired
after every move. `inu_storm` uses three floor marks and lightning from
above. The earlier teleport (`inu_vanish`) and wisp-pack attacks were
removed; the storm replaces the pack. It remains recognizable as a freed
hunting wolf, not a generic fireball boss. Lightning still recurs during
the chase, warded stage and fight. The latest chase attack spacing was
recorded as 2.4 s when in range; use authored profiles, not this prose,
for runtime timing truth.

**Movement, presentation and test contracts.** Camera-zone transitions
should mix without snapping. The body remains above the floor on the route;
the pursuer does not jump across an obstacle via teleportation. The
conductor `ambition_content_modules::inugami`, boss profile and part-based
visuals own state; chase distance, ward, attack volumes, route and
presentation are tested by `game/ambition_app/tests/inugami_fight.rs`.
In a part-rendered boss, `RiggedSpriteAsset::face_as` overlays the `Hit`
face without freezing the current action row. Player/NPC expression
handling through the character animator is a separate unfinished
capability where face tracks are present; baked heads require face tracks
before that technique works. A baked Inugami currently lacks the
part-only ward layer and therefore needs its fallback evaluated, not
assumed identical. The fixed chase uses a live-camera view only for
captures, never movement authority.

**Remaining acceptance (revalidate against HEAD before selection).**

- Register `inu_storm` in the seed catalog; review the `slam` duration
  band and current roster punish/telegraph warnings. A dated run reported
  four failing `content_it` checks; reproduce before treating them as
  current regressions.
- Audition dedicated sound and music cues; current Wolfenstein music
  (`galvanic_march`'s howl/storm) and other borrowed cues are placeholders.
  The planned transition cues the rising head with theremin, then starts
  Iron Toccata during the rise; verify the implemented music routing.
- Review fight/chase balance, route-camera framing and pursuit visibility
  in a real playtest. Check the chase's attack directions, collision margins,
  transition bar timing and return-route presentation on a GPU.
- Inspect remaining authored `hurt_flinch`, dart squash, move-interpolation
  and turn/pose quality **against the later Oct 24–25 art passes** before
  assigning work; `turn` and Wolfenstein in-betweens were implemented after
  the older request, so the old list is not an open task inventory.
- Ensure a warded part-based body still shows its ring through all pose
  and mirror changes; evaluate the baked fallback and actor-generic hit-face
  expression seam rather than duplicating the boss-only path.

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

**Room model now implemented.** This fight uses a sky room above its shore
instead of a bespoke chase-level type. Reusable features are autoscrolling,
tiled parallax, horizontally wrapping/bobbing moving shark platforms, a
fall-rescue carrier and a chest anchored in the shore room. The boss owns
its fireball, missile and dive attack patterns; it must work for players
without flight or spare fuel. The sky appears to move while the authored
rooms remain ordinary rooms. The four features are still valuable generic
room capabilities rather than Mockingbird-only simulation.

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
  bottom of the room (authored parts, see below). The sharks stay: the footing
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


**Asked (Jon, 2026-10-24), built the same day: space as part-based art.**
"I think currently in the mockingbird battle, the background and things like
the moon and earth is done via shaders a lot, and that is likely expensive,
when perhaps we should be using part-based spritework there."

- **Done.** `mockingbird_space.wgsl` (one quad over the room that made noise,
  stars, the planet and the moon for each pixel) is deleted. Space is nine
  authored pictures (`backgrounds/sky_parts.py` in the art submodule,
  published to `backgrounds/sky_parts/` by `scripts/regen/backgrounds.sh`):
  the dark with its dust band and a soft lower edge, two tiled star layers
  and an edge strip, a glint, the planet's limb, the moon, its halo and a lane
  dash. `presentation/mockingbird_sky.rs` poses them from the record: the
  dark and its stars come down from above the room as the fight climbs, the
  planet falls away and curves more (the shader's radius and top), the moon is
  where the record puts it, and the dashes walk its lane. A few glints twinkle
  by alpha. No shader is left in it, so space looks the same with shaders off.
- **Cost.** See `docs/planning/engine/performance-and-composition-findings.md`
  (llvmpipe rasterizer CPU a frame, -6% at `high`, -10% at `low`).
- **Open (Jon's eye).** Look A ("night", today's palette) is built; looks B
  (outlined, cel moon, sparkles) and C (violet, sunrise on the limb) are one
  line each (`sky_parts.PUBLISHED`). `python -m
  ambition_sprite2d_renderer.backgrounds parts-preview OUT.png` draws all
  three. What differs from the shader: the stars now come down with the dark
  instead of fading in place; the planet's surface does not drift; the stars
  are a little denser; the dust band is fainter.
## The hall of bosses: its own instances, and life switches (proposed 2026-10-06, Jon)

**Built:** the hall's Mockingbird and Clockwork Warden use their own
`hall_mockingbird_arena` and `hall_warden_arena` copies, with separate
placement IDs. Hall progress never clears a story placement. A practice
boss awards coin/health and a small purse, not a story gauntlet, ability
or quest advancement. The Hall's nine external `SwitchAction::BossLife`
switches display the corresponding encounter's **live** saved state
without saving a second switch state. A green-to-red press kills or marks
it dead without a reward; red-to-green retracts defeat and reseeds a
loaded boss. Tests include `the_hall_of_bosses_has_its_own_bosses` and
`boss_lifecycle::a_practice_copy_dies_without_the_story_consequences`.

**Remaining classification check.** The Hall's GNU-ton copy was recorded
as not yet `practice` after the Science-world placement was added;
T-rex and the FSM were in the same category. Verify current placement and
progression rules before changing `practice` metadata. Hall-only bosses
remain genuine encounters until they acquire story placements. The
practice policy should be applied per placement, not per archetype.

## GNU-ton — a giant with arms, and portals (proposed 2026-10-11, Jon)

**Current shape.** The GNU-ton fight has received several reworks through
2026-10-24: an on-foot scholar duel, a mobile giant with grounded mount
surfaces, portal punches, a clamp phase with separate fists, and a
space-launch phase built around Newton's laws. The old 2026-10-11
"fight below is NOT built" note and its port-missing audit were superseded
by these implementations. This section keeps the gameplay contract; the
technical debts remain in [`gnu-ton-fight-debts.md`](gnu-ton-fight-debts.md),
and placement in the Science gravity works is specified in
`tools/ambition_ldtk_tools/specs/gnu_ton_gravity/README.md`.

**The body.** A giant like a gnu, almost a centaur: a beast on four long
legs, a chest that stands up at its front, two arms to two fists, a gnu's
horns. Its back is a low hump from the rump to the foot of an upright,
centaur-like torso; the torso leans over the hump and the hump does not lean
with it (Jon, 2026-10-25). The hump ends under the left shoulder, about 93 under
the shoulders' top: a double jump goes up through the one-way shoulders
(`gnu_ton_fight::a_body_walks_up_the_giants_hump_and_jumps_to_his_shoulders`).
It has two stances: on all fours, and reared up (the hump steeper, the
shoulders higher). A trim (a gold rail with studs over a crimson band with
gilt points) shows where an actor can stand: the hump, the level part of each
horn, and the top of each epaulet. The surfaces are a fact of the body: they
are ground ridden or not, so the Hall's giant (`npc_giant_gnu_whole`, no
mount) has them too.

**Encounter loop.** Phase 1 teaches a useful portal pair: the scholar
opens an entry beside the giant and an exit over the targeted player,
then punches down through it. The exit is both a telegraphed threat and
a route for a player to reach the giant's shoulders and strike the scholar.
The arm and the punch are one physical event on both sides of the pair.
After enough damage, the scholar clamps the portals onto the giant's arms;
those become look-only lines around autonomous fists during phase 2,
while authored kicks can still use real portals. At the enrage, the fight
**does not** use the old proposed one-arm-return sequence: the built
encounter launches the room into space (Round 3 below). The portal
surfaces remain ordinary engine portals and are never painted into the
sprite. Avoid accidental collision/clip at the pair, but keep the arm
**rigid**: the initial stretchy-arm concept was rejected in the Oct 24
rework. Player access to the hump, epaulets and horns must be physical,
not a stand-under-feet exploit.

**Built architecture, superseding the first-cut proposal.** The conductor
`ambition_content_modules::gnu_ton` requests actual held portal pairs
(`ambition.portal.held_pair`) and look-only portal lines
(`ambition.portal.light_line`), so the engine owns their lifetimes. A
phase-1 fist is a posed limb/body that **really** transits a portal; its
hitbox emerges at the far aperture and the arm visual is clipped by that
same pair (`PortalHostReach`), rather than rendering two imitation fists.
The near-line and arm/limb geometry are intended to be art-driven, although
some line placements remain manually encoded (see the debt owner).
The portal punch tests cover the route from floor through the far line to
GNU-ton's back. On clamp the real pair closes about the arms, after which
look-only lines accompany independently acting fists. The later
2026-10-24 implementation expanded this to both arms and kicks, with
aim/telegraphs determined by player position and staged punish windows.

**Remaining debts from the early versions (validate on HEAD).** The original
plan called for repeated blows, active scholar reactions, authored barks,
short location-responsive phase-2 cycles, and a clear reward for taking
the portal route onto the back. Later work added both arms and the third
space phase, so the earlier proposed arm-return enrage should **not** be
implemented over the built launch-to-space transition. Check the
current fight for gaps rather than restoring the first-cut pattern:

- The limb trail must disappear when its fist is on the arm and follow the
  proper line when separated. The old hall-of-characters representation
  could show cut arms with invisible lines.
- Limb/portal line placement must follow the actual animated rig during
  bucking and rearing; the hand-enter/exit bounds and clipped part visuals
  require GPU-speed inspection. The dedicated debt owner records this.
- Verify phase-1 route tests, fight timing/damage tuning and the scholar's
  barks/reactions across the full encounter. The 2026-10-11 test report
  did not itself play phase 1, but later tests may now cover that.
- Check the real portal's clear opening against fist size and giant stance,
  whether the player can walk the hump and shoulders as intended, and whether
  the scholar's current shoulder and the fists' rest poses agree with the
  art. These were open visual/gameplay decisions in the 2026-10-11 plan.

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

**Two arms, a heavier hammer, a fight in three parts, and a giant drawn from
pieces (built 2026-10-24; Jon: "the size of the sheet should be able to be
mitigated by part based rendering ... is it always with the canvas-right arm?
... the hammer isn't visually forceful enough").** What changed and why:

- *The giant is drawn from pieces.* The giant's drawing (`giant_art.scene_for`,
  `_pieces.py`) is now pieces, each painted once: the bones of the legs and
  the arms (stretched along the bone), the barrel of the beast (it turns about
  its rump to the stance), the chest, the head (one face for each
  expression, turned by its tilt), the ears, the tail. The arm of a blow, one
  curve, is a chain of eight bones; the ink of each is drawn before the fill
  of any, so the chain reads as one tube. The giant publishes a part
  flipbook (`giant_gnu_parts.ron`) and the game draws it from that and loads
  no baked page. MEASURED in the arena (`capture_scene`, the `[image]`
  census): the giant's texture was 89.1 MP (10558 x 8443, 356 MB) at `high`
  and 23.1 MP (two pages, 92 MB) at `low`. After: one page of pieces,
  1.0 MP (1020 x 983, 4 MB) at `high` and 0.26 MP (512 x 501, 1 MB) at
  `low` (MEASURED, the same census). A row costs no texels now, so the rows
  are dense. The giant is read as one image (through a portal, in a hit
  flash) from the largest impostor cell (1152 px); near its rest length the
  barrel is painted as drawn, so that its quad stays in that cell.
- *Both arms throw.* Each arm has a blow down (the HAMMER), one across (the
  right JAB, the left HOOK out behind, over the slope of the back) and one up
  (the UPPERCUT). The arm on the player's side of the giant throws; a blow
  whose tell starts less than 0.5 s after another ended is the next of a
  combination and the OTHER arm throws it (the one-two). Each blow has its own
  windup and body (the hammer rears up with the fist over the head and slams
  with the whole body; the jab is cocked by the jaw and lunges; the hook is
  wound across the chest and twists out; the uppercut crouches with the fist
  by the knee and rises), its own follow-through (the other arm braces or
  swings back) and its own recovery (the hammer drags its arm out of the
  floor; the jab snaps back). So the tell says which blow comes, and the side
  of the arm says from where.
- *The hitch.* Each windup is set on its line and trembles, then draws back
  the last of the way (`giant_art.SET_AT`, `HITCH_AT`) with a star at the far
  line (`gnu_blow_glint`): the blow goes NOW. The face flares with it.
- *The hammer lands.* Its strike is 0.24 s (was 0.35) and the fist is out of
  the far line in its last 0.07 s; lines of speed come out with it
  (`gnu_blow_streak`, turned to the line's normal). Into the floor it quakes
  (`gnu_hammer_quake`: cracks, a flat ring of dust, a plume, stones), shakes
  the camera 22 (was 11) and holds the giant still 0.07 s. On a body it
  bursts (`gnu_blow_impact`), shakes 32 and holds 0.16 s, and deals 3. The
  effects are an authored sheet (`gnu_ton_blow_fx`) that the giant OWNS (its
  record names `npc_giant_gnu`): it is decoded with the giant. The effect port
  turns an effect (`ambition.feedback.effect` v2, `angle`) and is offered in
  `boss_conduct`.
- *A punish window on the floor.* A hammer's fist in the floor is an opening
  while it is held there: its first blow passes to him, as a stuck fist's does
  in phase 2. So phase 1 has two answers: strike the fist from the floor (safe,
  one blow), or go up through the pair to fight him on the back (more blows,
  more danger).
- *Phase 1 in three parts* (`boss_profiles.ron`). Fresh: one blow at a time
  with a long tell, after the slow first hammer (its fist stays about 1.5 s
  in the floor: the lesson). Under 86%: the one-two, a kick then an arm, and each
  hammer into the floor sends a shock along it each way (jump it). Under 74%:
  three blows in a row, the arms in turn, the last one slow to come out of
  the floor.
- Tests: the arms, their blows and their lines; a blow comes through its line
  once (module); a player on the left is struck by the left arm and both arms
  throw once he is hurt; a hammer in the floor is an opening that passes one
  blow (app). Art: renderer (the pieces, the rows, the effects).

The left-arm hook is implemented in the both-arms pass above (not an open
left-jab proposal). Remaining presentation review: the eight-bone chain has
visible joints instead of a continuously smooth arm, and the new effect
rows require observation at GPU speed.

**Placed in the main game (2026-10-23).** Jon asked for a gravity and
portal puzzle level with GNU-ton at its end. The level is the GNU-ton
gravity handle, the gravity works of the Science dimension (`science.ldtk`;
Jon, 2026-10-24: GNU-ton lives in the Science dimension, with its orbital station):
four puzzle rooms (up fields; side fields; portals; a room that turns over)
and his arena `gnu_ton_shoulders`. The arena is the hall's arena as it is,
with his placement `gnu_ton_handle.gnu_ton`. A `LockWall` gated on
`boss.cleared gnu_ton_handle.gnu_ton` opens the door home. Room by room:
`tools/ambition_ldtk_tools/specs/gnu_ton_gravity/README.md`. Proof:
`game/ambition_app/tests/the_gnu_ton_handle.rs`. Recorded follow-up: verify whether the Hall copy is now `practice`; the
2026-10-23 pass reported that GNU-ton, T-rex and FSM were not. Do not infer
current state from that dated observation.

### Round 3: the whole encounter (built 2026-10-24)

Jon (2026-10-24): *"the mount should be able to move around the room. We
should avoid too many platforms in the room, because the mechanism to get
onto the boss should be the shoulders, back and portals. The mount should
feel like a character in its own right, and if you defeat the scholar, it
needs to have an appropriate reaction. The fight sort of ends
anticlimactically ... a phase 3 where everyone blasts off into space ...
I want the fight to feel like it goes through newtons laws."* And later the
same day: *"I really don't like the bendy arm. We can place the entrance
portals wherever we need to to make the fight work, and such that a punch
moves through the portal cleanly without clipping in the upper part of the
giant's arms."*

**The script of the beats.**

1. **Prelude, the duel.** In the room before the hall (`gnu_ton_approach`
   in the sandbox, `gnu_ton_works_approach` in the Science world) GNU-ton
   waits on his own feet: the fighter `npc_gnu_ton` (ten hearts), with his
   own moves and his laws. Beaten down to three hearts he cannot be hurt,
   says *"Enough! A body in motion... stays in MOTION!"*, and runs out by
   the door to his hall (`bosses/gnu_ton/prelude.rs`).
2. **The chase and the arrival.** The player follows him in by the hall's
   top door. As the hall wakes he is on the ledge by that door (*"You'll not
   catch me! GNU, ARISE!"*); he leaps down onto his giant's shoulders, calls
   his power (`gnu_power_up`), and the giant roars (`choreography::arrival`,
   in the encounter's intro).
3. **Phase 1.** The giant's arms are on it. It walks the hall between its
   blows (a mark a stride from the player, never from under a player on
   its back) and looks round at a player on its back. Its blows go through
   a real portal pair. The hall has only its two low steps by the walls:
   the ways onto the giant are its back, its shoulders and the pair.
4. **The clamp and phase 2,** as before: two lines take the arms, and the
   fists are bodies of their own.
5. **The launch: the well.** At the enrage the encounter holds a beat
   (`enrage_seconds`, 3.6). He powers up (*"Every body pulls on every
   body..."*). A huge pair of portals eases open: one in the floor of the
   hall, wider than the giant, and one high over it. A well of gravity at
   the floor's portal (`ambition.world.held_force` v2, `well`: the same
   acceleration on every mass) pulls every body into the floor (*"...and
   MINE pulls hardest!"*). The giant, the scholar on it and the player fall
   through the floor and out of the portal over it, still falling, as the
   sky goes to space (authored art, `gnu_ton_space`;
   `presentation/gnu_ton_sky.rs`). The pair eases shut and the well lets
   go; a body keeps the fall it came out with and lands light on the floor
   of the hall in space. The scholar and the giant put on space helmets.
   It is one fall through, not a loop: each fall through the pair is faster
   than the one before (MEASURED: 1000, 2300, then 3000 px/s, and at 3000
   the giant went through the floor and out of the hall). Test:
   `gnu_ton_fight::the_well_pulls_the_hall_through_its_portals_into_space`.
6. **Phase 3, in space.** Each body has two fifths of its weight. Every ten
   seconds a tide comes, told first (*"The tide turns!"*, *"Gravity... is
   optional!"*): a push to the right, then to the left, then up. The pair
   and the fists go on as in phase 2.
7. **The end.** Beaten, he falls; the sky and the weight come back as he
   falls, and the giant gathers him in its arms. It grieves, then roars in
   rage and the hall shakes. A golden apple falls on his head
   (*"...Eureka?"*), the helmets come off, and he wakes on its shoulder:
   *"I yield. A body at rest stays at rest."*

**Newton's laws, as mechanics.**

- *First law.* In space a body keeps the most part of its speed: a launch
  goes far. The field is `ambition.world.held_force` (`lift` 0.6).
- *Second law.* A tide is a FORCE: a body of mass `m` takes `force / m`. The
  player (mass 1) is pushed eight times as hard as the giant (mass 8). A
  heavier blow (the hammer) throws the player farther in a clash.
- *Third law.* A blow of the player's that meets the giant's fist as it
  comes out of its far line is a clash (*"Equal! And opposite!"*): each
  body is thrown back by its mass. In space every blow that lands on him
  pushes the player back.

**The arms are rigid, and the near lines come from them.** Each arm is an
upper arm and a forearm, joined at the shoulder, the elbow and the wrist,
with their lengths fixed. Only the forearm and the fist go through a near
line, square to it; the upper arm and the hair at the elbow stay on the
giant's side of its plane. Each near line is derived from the arm of its
blow (`giant_art._derive_lines`), and the game types the same places
(`choreography::blows::limb`). The pair opens in the tell only once the
arm is out of the place of its opening (`open_from`), and is closed before
the arm goes home through it (`close_at`). `tests/test_gnu_ton_giant_blows.py`
in the renderer checks this on the posed parts of every frame of every
blow. A limb in a pair takes only its own parts through
(`rigged::LimbTwin`), so the back slope does not show at the far line.

**Gear.** The helmets are the first worn gear (`docs/systems/worn-gear.md`).

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
