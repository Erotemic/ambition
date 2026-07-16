# Track M — Super Mary-O (classic platformer acceptance demo)

Parody-original classic tile-platforming: teach-by-play opening, pipe secrets,
powerups, enemies, and a flag sequence without copied art or layout.

**Purpose:** prove that the conventional axis-swept AABB platformer is a simple
customer of the same engine that supports momentum and relativistic mechanics.
Classic behavior must be authored through engine seams rather than a privileged
"normal game" path.

## Current state

Landed:

- provider/demo shells and authored world path;
- equipment rows and equipment-to-action/parameter machinery;
- forward-only camera scroll policy as authored camera data;
- provider-owned flag sequence and scoring geometry;
- the initial grow/ranged powerup definitions.

Remaining acceptance work:

- world pickup entities and equip-on-touch flow;
- live body-size consumption by collision and presentation;
- complete enemies, shell-prop behavior, HUD, results, and level content;
- deterministic headless completion of the first level, including the secret pipe.

## Consumes

- runtime, provider lifecycle, and windowed host composition;
- the shared body/control path using the axis-swept motion model;
- item/equipment and canonical action/moveset execution;
- combat/contact vocabulary for stomps and sliding hazards;
- world IR + LDtk rooms/loading zones;
- the cutscene domain for presentation sequencing where appropriate;
- `SimView` for HUD and programmatic observation.

## Owns

`ambition_demo_mary_o` owns its levels, rules, lives/score/coins/timer, equipment
rows, enemy/content rows, shell prop, flag sequence, HUD, title, and results. A
need discovered while authoring becomes engine work only when it is a reusable
missing seam.

## V1 design

- **World:** three levels sharing one authored world: an opening grammar, an underground variant, and a moving-platform level.
- **Powerups:** a grow/armor equipment row and a ranged-action grant. Numeric effects fold through equipment parameters; behavior grants compose through action data.
- **Enemies:** ordinary actor rows for walkers plus a brainless sliding shell prop; this exercises the actors-versus-props distinction.
- **Camera:** forward-only scroll is an authored camera policy, not Mary-O-specific engine code.
- **Flag:** provider-owned gameplay state captures contact height and drives the body deterministically; presentation/results may use the cutscene domain without turning cutscenes into encounter logic.
- **Death:** level restart is authored session/game policy rather than a universal engine default.

## Acceptance

A scripted headless run completes the first level, reaches the pipe secret, uses
a powerup through the real pickup/equipment path, and never touches the
surface-momentum implementation. The visible app uses the same provider and body
state, including size and equipment presentation.

The demo app remains an explicit composition root, not a second implementation of
input, session lifecycle, sprite binding, or platformer simulation.
