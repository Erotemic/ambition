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

## Open

1. **Play it.** Each number is a first cut. Seen in software renders of the
   Smash stage (`--home`) only; not in a room of Ambition, and not on a GPU.
2. The robot in the air draws the ground rows (standing legs). Rows of its
   own for the swings in the air are art work.
3. The up swing and the down swing differ in the body's row and in which
   end of the slash has the light. If that is too quiet, the slash rows can
   differ more.
4. `air_down` keeps a fighter's landing lag (0.28 s). Hollow Knight's pogo
   has none.
5. `blade_low` roots no one: the robot can walk in a low poke. If the low
   poke is to stop it, that is one flag.
6. Enemies were tuned against a jab that rooted. The fastest string is the
   old jab's pace (one each 0.24 s; the jab was 0.25 s), and it now moves.
7. A walk that holds attack does not go through the string by itself (a
   held attack at rest does).
