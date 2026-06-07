# Side-view walk-cycle baseline

This note records the current "good" baseline for the side-view biped lanes
(`robot_side`, `goblin_side`, `toon_side`, and `ninja_side`) so future tweaks do
not regress back to the old opposed-stick feeling.

## Why this cycle reads better

The older side-view walks mostly rotated upper and lower leg segments with two
sinusoids. That was cheap, but it made several rigs look like:

- two metronome sticks hinged at the hips,
- feet that slid or floated because the foot position was only an afterthought,
- silhouettes that swapped depth ambiguously because limb ownership and draw
  order were not explicit.

The current baseline fixes those issues by treating the walk as an authored
8-pose contact/down/passing/up cycle instead of a pair of raw angle oscillators.

## Core idea

For walk and run rows, each rig now uses authored **ankle targets** per frame.
Those targets encode the things viewers notice first:

- how far each planted foot reaches in front and behind,
- when the body drops into the down pose,
- how high the passing foot lifts,
- how the shoe tilts and shifts while rolling through contact.

The upper and lower leg still keep their exact authored lengths. We solve the
knee with a simple two-bone IK pass toward the ankle target, which gives us:

- consistent knee bends,
- planted feet with much less sliding,
- cleaner silhouettes across different body proportions.

## Practical recipe

When authoring or extending a side-view biped, follow this order:

1. Start from an 8-frame loop: contact, down, passing, up, contact, down,
   passing, up.
2. Author ankle target arrays first. Use separate X/Y arrays for the far leg
   and near leg.
3. Add small per-frame foot-roll offsets and foot-angle tilts.
4. Clamp the ankle target to the reachable range of the upper+lower leg.
5. Solve the knee with two-bone IK.
6. Keep explicit semantic limb labels and draw order:
   - far/back arm
   - far/back leg
   - pelvis / torso
   - near/front leg
   - near/front arm
   - head / front accessories

That order matters almost as much as the ankle targets; it keeps the side-view
read stable instead of letting limbs pop in front of each other frame to frame.

## What is shared vs. target-specific

Shared ideas:

- 8-frame contact/down/passing/up structure
- ankle-target-driven feet
- reachable-target clamp
- two-bone IK for knees
- explicit near/far limb semantics

Target-specific tuning:

- stride width and lift height
- foot roll amount
- hip spread
- bounce / torso lean from the target's existing pose code
- shoe/foot drawing style

This is intentional. The baseline should keep the motion language coherent
without flattening the personality of each renderer.

## Files using this baseline

- `targets/characters/robot_side.py`
- `targets/characters/goblin_side.py`
- `targets/characters/toon_side.py`
- `targets/characters/ninja_side.py`

If a future side-view humanoid is added, start by copying one of those walk/run
implementations instead of reviving the older direct-angle-only pattern.
