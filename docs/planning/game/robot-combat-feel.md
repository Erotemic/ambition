# The robot's blade in Ambition: Hollow Knight feel (started 2026-10-12)

Jon, 2026-10-12: "The base moveset of the robot should feel like hollow
knight." The Smash game keeps a platform fighter's moves. This is the state
of that work and what is open.

## The ruling

- A normal attack does not change the robot's speed. It walks, jumps and
  dashes through a swing. Ambition has no dash attack.
- The jab and the forward attack are the same attack: a 1-2 of an up swing
  and a down swing when the presses are close. The reach of the two is the
  same; the difference is the picture. After a pause, a press is the up swing
  again.
- Held forward (a walk): the string is the two swings, again and again.
- At rest, with no direction: a third strike follows, with more power.
- The air has the same attacks: neutral air is the jab, forward air is the
  forward attack, up air is the up attack.
- The down attack on the ground stays (Hollow Knight has none).
- A jump backward does not turn the robot (as in Smash). The back air is its
  own move, and as Marth's it turns the robot about: the same stick is then
  forward, and the next attack is the forward air.
- The smash attacks do not change.

## What is built

Moves (`game/ambition_content/assets/data/movesets/player_robot.ron`; each is
for the ground and the air unless it says not):

| Move | When | Timing (s) | Health |
|---|---|---|---|
| `blade_rise` | attack, forward attack | live 0.05 to 0.12; next from 0.24 | 1 |
| `blade_fall` | the second of the string | the same | 1 |
| `blade_thrust` | the third, at rest only | live 0.065 to 0.17; 0.5 long | 2 |
| `blade_overhead` | up attack | live 0.055 to 0.125 | 1 |
| `blade_low` | down attack, ground only | the frames of `tilt_down` | 1 |
| `blade_behind` | back attack, air only | live 0.088 to 0.15; turns at 0.31 | 1 |
| `air_down` | down attack, air | not changed (the pogo; landing lag) | 1 |

- **The time to press again is 0.5 s** from the press of a swing. The
  swing's picture is over at 0.28 s; from then the robot is drawn as it
  stands or walks, and a press is still the next of the string
  (`WindowTag::Settled`). The quickest string is one swing each 0.24 s.
- A HELD attack with no direction goes through the string by itself (the
  engine's rule for a string, as for each fighter's jab).
- The two strings are one cancel table. The two verbs name `blade_rise`.
  From `blade_fall`, a press with no direction is the thrust, and a forward
  press asks for the rise.
- The pictures are rows the robot had: `attack_side` (low to high), `jab`
  (high to low), `dash_attack` (the thrust), `attack_up`, `air_back`. The
  slash effect has two new rows, `rise` and `fall` (`robot_slash`).
- The Smash game: the robot's fighter facet names the fighter's moves for
  its buttons (`verbs` in `data/fighters/player_robot_v3.ron`): the jab, the
  tilts, the dash attack and the five aerials, as before. One move table for
  the two games; each game says which move a button plays.

Engine parts this needed (each is general):

- `SmashFighterFacet::verbs` and `PreparedCharacterDefinition::moveset_in`:
  a game's own verbs over the one table, as its own damage is.
- A press that asks for the move that plays continues its string (it was a
  press with no direction only).
- `WindowTag::Settled`: the picture of a move is over and its string is not.
- `MoveEventKind::TurnAround`: the owner faces the other way from that beat.
- `slash_rise` / `slash_fall`: the art of a swing says which way it went.
- `moveset_render --home [--repress TICKS]`: photographs a fighter with the
  verbs of its own game, and a string.

Measured in the game (`game/ambition_app/tests/robot_blade_at_home.rs`): a
swing in a run keeps 97% or more of the run's speed; at rest the string is
rise, fall, thrust; in a walk it is rise, fall, rise, fall; a second press
0.45 s after the first is still the fall; presses 0.6 s apart are each the
rise; in the air the string is the same, and the strike behind turns him.

## The second pass (2026-10-12)

Jon, after he played it: the thrust reaches as far as the two swings; the
robot walks and runs while it swings; the three swings sound different; the
swing art is better; Ambition charges much less landing lag.

- **Reach.** `blade_thrust` reaches 90 px (the rise 94, the fall 86); its
  height did not change.
- **Legs under a swing.** `WindowTag::UpperBody` on a move says its row is
  the body above the hips. While it is live and the body walks, runs, jumps
  or falls, the legs are drawn from that row and the swing's row gives the
  rest, moved by the difference of the two hips
  (`RiggedSpriteAsset::legs_under`). The legs keep their own time, and the
  walk continues from there when the swing ends. Which tracks are legs is by
  the track's name (`is_lower_body_track`). The four blade moves that do not
  root have the window. Only the player's road asks for it; an NPC's does
  not yet.
- **Sound.** Three cues: `player.robot.blade.rise` (the air goes up and
  stays bright), `.fall` (comes down, more body), `.thrust` (one fast narrow
  push, a longer wake). Each has its loudest 10 ms about 60 ms in; the cue
  before had it at 88 ms. `blade_overhead` plays the rise; `blade_low` and
  `blade_behind` play the fall. The audition is
  `untracked/sfx-candidates/robot_blade/` (`make.py`; reels of the cue
  before, of these, and of two more families: a thin ring of the edge, and a
  low push of the arm).
- **Art.** The slash rows `rise` and `fall` are a blade that turns about the
  hand with a crescent behind it that goes out from its start; `poke` is its
  own row, a lance. The robot's face in a swing is open, not a glare.
- **No kick back on a press.** Ambition's `slash_recoil` was 110 px/s: each
  press of attack pushed the robot back along its facing (a run of 270 went
  to 247 for one frame). It is 0 now (`platformer_defaults.ron`). The test
  measures from the frame of the press, through two swings.
- **Landing lag.** `LandingLagRules` (`ambition_combat::landing_lag`), a
  rule each game declares: Ambition charges 0.25 of each move's landing lag
  (`landing_lag_share` in `room_rules.ron`). At home only `air_down` (0.28 s, now
  0.07 s) and `stabilizer_dive` (0.3 s) state one. The Smash game declares
  none and charges each move's own.

## The third pass: the slash art and the smash attacks (2026-10-12)

Jon's rules for a slash, in his words:

- "I want more of the upward or downward motion and follow through visible
  in the slashes." "The symmetric parts at the end are what I don't like."
- "parts of the slash should not be visible until the slash has 'traveled
  through' that area."
- "any alpha parts you add fade to complete transparency before the sheet
  ends, so there isn't any discontinuity by overlaying on something."
- A sheet of very good slashes is his reference for quality, and it is not
  to be copied.

What is built (`robot_slash`, in the sprite renderer):

- A swing with a direction (`rise`, `fall`) is a comet that goes along the
  outline of the hit region: from one horn, across the front, to the other
  horn, and then back over the body (the follow through). Its head is the
  blade; its tail comes after. The last frames are the curl at the end of
  the swing. Nothing is drawn where the blade has not been.
- The comet is a solid white band with a sharp inner edge, thin fibres
  behind it, and a little light on the air behind those. The rows with no
  direction (`side`, `up`, `down`) have the same parts.
- No frame has alpha at a side of its cell (a test in the renderer).
- A move says its direction with its volume's `vfx`: `slash_rise` or
  `slash_fall`, in each pose. For an up strike, a rise goes from the body's
  front, over it, to its back. Stated for: the 1-2 string, the up attack
  (`blade_overhead`), the forward smash (a fall: it is a chop), the up
  smash, and the Smash game's jab and two tilts. Not stated yet: the back
  air, the down attack, the airs of the Smash game, the down smash (it is
  on the two sides at one time).
- The smash attacks have sheet polygons, as the tilts have (they used the
  moveset's boxes, and the art is fitted to the volume): forward 110 x 106
  (the tilt is 96 x 94), up 100 x 98 (the tilt is 87 x 84), down 155 x 39
  on the two sides. World units.

## Open

1. **Play it.** Each number is a first cut. Seen in software renders of the
   Smash stage (`--home`) only; not in a room of Ambition, and not on a GPU.
2. The legs under a swing are by the names of the tracks, and for the
   player only. A body of an NPC that swings in a walk slides.
3. The up swing and the down swing differ in the body's row and in which
   end of the slash has the light. If that is too quiet, the slash rows can
   differ more.
4. The other arcs have no direction yet (see the third pass), and the
   thrust's lance did not get the solid band and fibres.
5. `blade_low` roots no one: the robot can walk in a low poke. If the low
   poke is to stop it, that is one flag.
6. Enemies were tuned against a jab that rooted. The fastest string is the
   old jab's pace (one each 0.24 s; the jab was 0.25 s), and it now moves.
7. A walk that holds attack does not go through the string by itself (a
   held attack at rest does).

## The fireball (2026-10-24)

Jon: "my fireballs aren't doing much damage", "somewhat difficult to have
work", "it doesn't really work with the hand pose".

Measured in the Mockingbird's sky with rollback on, the robot beside the
winded Mockingbird (`game/ambition_app/tests/the_robots_fireball_against_the_mockingbird.rs`,
`measure_…`, ignored):

| Way of fighting | Before | After |
|---|---|---|
| Blade, a press each 0.25 s | 9 damage, 4.15/s | the same |
| Fireball tap each 0.10 s | 22 presses, 6 shots, 2.33/s | 7 shots, 3.23/s |
| Fireball tap each 0.20 s | 11 presses, 6 shots, 2.33/s | 7 shots, 3.23/s |
| Fireball tap each 0.33 s | 7 shots, 3.23/s | the same |
| Fireball held 0.4 s (tier 1) | 4 a shot, 8.7/s | the same |

- **A press in the cooldown was lost.** The fireball fires on the release,
  and a release in the 0.30 s cooldown was refused. Now it waits
  (`PlayerProjectileState::buffered`, up to `FIRE_BUFFER_S`, 0.35 s) and
  fires when the cooldown ends. A press and a release in one tick is a shot
  (it left a charge that never fired). A Hadouken gesture the meter cannot
  pay for is a fireball, not a lost press.
- **The pose.** The `shoot` row hung the arm at the side and held the orb at
  the knee, and the shot was born at the hand of the row's middle frame. The
  shot is fired on the tick the row starts, so it is now born at the hand of
  the FIRST frame (`SHOOT_RELEASE_PHASE`). The robot's `shoot` row is a push
  of both palms at chest height from frame 0, 0.18 s long (the time the game
  shows it); `aim` (the held charge) is the wind-up, both hands back at the hip
  with the orb. No shot is drawn in the art: the shot is the game's.

Open (Jon):

1. A tap deals 1, as one swing does, and the 0.30 s cooldown holds taps to
   3.3/s against the blade's 4.15/s. A charged shot deals 4 (0.35 s) or 16
   (0.85 s). The tap's damage and the cooldown are Ambition's numbers on
   `ProjectileKind`, and the Smash stage reads the same ones.
2. Out at its side the Mockingbird's guard turns every blow, a fireball too,
   and a tap flies about 430 px (360 px/s for 1.2 s) in a sky 1280 wide. So
   a fireball hurts it only when it is close and open.
3. The fireball fires on the release. A shot on the press (and a charged
   shot on the release) is the Mega Man way.
4. A shot aimed up or on a diagonal (the right stick) leaves the forward
   push's hand: there is one `shoot` row.

Closed: "under GGRS resimulation only the first strike of a window damages
the winded Mockingbird" was the probe's fixture, not the game. The probe held
the player with a `world_mut` write on each tick. A rewind erases such a
write, so under resimulation the player drifted out of reach. Put through the
harness (a rollback rebase), with only input in the window, every hit lands
at check distance 1 and 4, on the boss and on a plain enemy. Guards:
`every_hit_in_the_winded_window_lands_under_rollback`,
`every_hit_on_a_sandbag_lands_under_rollback`.
