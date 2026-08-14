# Secondary games and acceptance customers — doctrine

Ambition is the flagship game and primary product driver. The games in this
directory are serious secondary customers used to force reusable engine
capabilities into honest shapes.

They are not disposable fixtures, and they are not the project hierarchy. A
secondary game may eventually graduate into a **first-class game** if its product
quality warrants that investment. Super Smash Siblings is a plausible candidate.
Ambition remains the main game.

Current customers include Sanic, Super Mary-O, Super Smash Siblings, Hollow Lite
and TwinTrack.

## Ownership rule

Each game separates:

- **engine capability it consumes** — reusable mechanics, simulation, world,
  participant, presentation, authoring and service seams;
- **game policy it owns** — named rules, content, tuning, UI flow and fiction.

If a secondary game needs a capability that should also be usable by Ambition or
another game, implement it in the reusable engine with a game-independent API.
Do not hide missing engine capability in provider-local infrastructure. Equally,
do not promote named game policy into core merely because it is sophisticated.

## World authoring

LDtk is the preferred spatial authoring surface for Ambition and for games where
it fits. Programmatic world IR remains useful for focused tests, generated
content and small experimental games, but it is not a reason to leave obvious
LDtk authoring gaps unfixed.

If a real level needs a reusable world concept LDtk cannot express pleasantly,
improve [`../engine/ldtk-authoring-and-world-tools.md`](../engine/ldtk-authoring-and-world-tools.md)
and the backend-neutral world IR rather than building a second game-specific
world channel.

## Composition shape

A game should compose ordinary engine capabilities plus its provider/content and
rules. Hosted and standalone forms should use the same simulation/rules systems
with explicit scope chosen by the composition root.

Conceptually:

```text
engine capabilities
      +
game/provider content
      +
game rules / presentation policy
      |
      +--> standalone host
      +--> Ambition-hosted area when product-appropriate
```

Hosting a secondary game inside Ambition is a strong composability test, not a
requirement that every future first-class game remain a tiny embedded mode.

## Participant and view doctrine

Do not assume one human, one controller, one body or one camera merely because a
particular acceptance game currently has them.

- participants/control authority belong to the shared input/session model;
- local versus network transport is not body identity;
- shared/split/adaptive presentation belongs to the multi-view engine;
- one game may require party cohesion while another allows different rooms;
- TwinTrack may derive a distinct reference-frame presentation per local view;
- Smash usually uses a shared arena view but must not force the engine back to a
  singleton-camera ontology.

See [`../engine/multiplayer-and-multiview.md`](../engine/multiplayer-and-multiview.md).

## Headless acceptance

A serious customer should prove its rules through the real headless simulation
before relying on visual feel. The test should exercise the capability the game
actually claims to need rather than building a parallel test-only simulator.

Visual/game-feel evaluation remains human/product work. A passing headless test
cannot establish that a fight, camera transition or level is fun.

## Graduation to first-class game

Graduation is a product decision, not an architecture loophole. A graduated game
may gain deeper content, packaging, UX and long-term support, but:

- its ordinary mechanics still use shared engine capabilities;
- its named rules remain game-owned;
- its needs can create new engine programs when they are genuinely reusable; and
- Ambition remains the flagship unless the maintainer explicitly changes that
  product decision.
