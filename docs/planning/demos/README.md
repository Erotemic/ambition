# Secondary games and acceptance customers — doctrine

Ambition is the flagship game and primary product driver. The games in this
directory are serious secondary customers used to force reusable engine
capabilities into honest shapes.

They are not disposable fixtures, and they are not the project hierarchy. A
secondary game may eventually graduate into a **first-class game** if its product
quality warrants that investment. Super Smash Siblings is a plausible candidate.
Ambition remains the main game.

Current customers, each with its own plan — **this list is the index, and it is
load-bearing**:

- [Sanic](sanic.md) — momentum acceptance;
- [Super Mary-O](super-mary-o.md) — classic axis-swept AABB platforming;
- [Super Smash Siblings](super-smash-siblings.md) — platform-fighter product
  charter and index to its current inventory/campaign; plausible graduation
  candidate;
- [Hollow Lite](hollow-lite.md) — encounters and boss authoring;
- [TwinTrack](twintrack.md) — independent observers and reference-frame
  presentation.

⛔ **`ambition_demo_pocket` IS NOT A MISSING ENTRY — it is not a customer.** It
sits in `game/` beside the five above and is named like a sixth, which is why it
keeps being investigated; its own manifest settles it: *"Tiny fourth-provider
acceptance fixture for Ambition's provider authoring surface."* A fixture, one
source file, depended on by `ambition_app` to prove the provider surface admits a
fourth author. It has no product charter and wants no plan. ⚠ The confusion is
structural rather than anybody's mistake — the repository's other fixtures live
in `fixtures/` (`minimal_game`, `external_consumer`) while this one lives in
`game/` under a `demo_` prefix. Re-checked 2026-09-02; if it ever grows a product
charter it graduates into the list above, and until then this paragraph is here
to stop the next session spending ten minutes deciding whether the index is
incomplete.

⚠ **the links are why this paragraph exists.** Until 2026-08-14 it named these
games in prose and linked none of them, and `sanic.md` and `super-mary-o.md` were
consequently referenced by NO document anywhere in the repository. Super Mary-O's
was the only planning file in the tree that carried open items and was reachable
from nothing — its two `▢`s, including the still-unanswered *"no way to get the
fire flower"* report, were invisible to every session that worked the ledger. A
plan nothing links to is not a plan; it is a file.

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
