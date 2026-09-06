# Endgame Sandbox Design Notes

This room exists before the game has a beginning.

It is a laboratory for the **ultimate endgame state**: the state where moving is rewarding even when there is no objective, no story, and no art. It should eventually become a movement instrument.

## Success condition

A player can spend five minutes in the room just moving, routing, recovering, and styling, and still want to continue.

## Current movement verbs

- Run
- Jump
- Variable jump height
- Wall jump
- Dash
- Pogo
- Slash/recoil
- Rebound pad

## Intended composition examples

```text
Dash o Pogo       => speed converted into height
Pogo o Dash       => height converted into routing
WallJump o Dash   => fast wall exit
Dash o WallJump   => wall bank / correction
Rebound o Dash    => preserve loop momentum
```

This is the first pass at the idea that movement is algebraic and non-commutative: order matters.

## Room requirements

The room should contain:

- A loop that returns to spawn.
- A fast route.
- A safe route.
- A flashy route.
- Recovery opportunities after mistakes.
- Debugging instrumentation.
- Enough geometry variety to make the same verbs compose differently.

## Do not add yet

- Story.
- Prerendered art.
- Imported music.
- Large procedural worlds.
- Enemies.

Those come after the motion is intrinsically fun.
