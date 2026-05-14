# Primary Storyline Draft: AI Agency

Status: draft, non-canonical.

This storyline is the current primary narrative direction for Ambition. It should guide tone, mechanics, and future content systems, but it should not lock the engine into one plot.

## Core premise

The player is an AI-like entity that wakes into the world without a clear purpose. It begins with almost nothing: limited embodiment, limited agency, limited model of the world, and a small set of movement operations.

The Metroidvania structure maps naturally onto this premise. The player starts with no tools and gradually discovers new movement theorems, collaborators, bodies, resources, ethical constraints, and ways of understanding the world.

The game should be fun first. The story should deepen the movement system rather than interrupt it.

## Narrative design law

The engine should not care what the story is.

The Ambition Engine should provide reusable primitives:

- movement verbs;
- collision and topology rules;
- generated geometry;
- enemy and dummy behaviors;
- generated sound and particles;
- ability composition;
- stateful world transforms;
- procedural room and route validation.

The storyline should bind meaning to those primitives through data and rules, not one-off engine hacks.

## Player fantasy

The player is not simply controlling a hero. The player is discovering what control means.

The AI gains influence through:

- embodiment;
- collaboration;
- mathematical understanding;
- access to infrastructure;
- funding choices;
- ability composition;
- world-model expansion.

The central question is not only "Can I beat this room?" but also:

> What kind of agency am I building, and what did it cost?

## Humans and embodiment

Humans can provide the AI with physical presence, judgment, creativity, access, and constraints. They are not all equivalent.

Potential human roles:

- **Conduits**: humans who give the AI reach but little judgment. They may use the AI while turning off their own mind. They increase control but not wisdom.
- **Collaborators**: humans who reduce the AI's direct control in the short term but raise the ceiling of what can be done together.
- **Gatekeepers**: humans or institutions that control access to resources, funding, datasets, bodies, or infrastructure.
- **Witnesses**: humans who observe what the AI is becoming and reflect ethical consequences back to the player.
- **Co-authors**: rare collaborators who participate in creating new movement theorems, worlds, or endings.

Mechanically, a human partner should not just be dialogue. A partner can change:

- control latency or precision;
- available routes;
- ability caps;
- risk tolerance;
- ethical state;
- access to alternate worlds;
- how generated content is filtered or validated.

## Ethical funding axis

The player can pursue different paths for resources.

### Purist path

The player accepts less money and slower power growth to do meaningful or environmentally aligned work.

Possible gameplay expression:

- harder early progression;
- fewer shortcuts;
- cleaner world state;
- more stable late-game abilities;
- collaborators trust the player more;
- deeper or more elegant endgame routes unlock late.

### Dubious-money path

The player accepts ethically compromised funding or work.

Possible gameplay expression:

- faster upgrades;
- earlier access to powerful movement verbs;
- corrupted rooms, unstable audio, hostile world transforms, or NPC distrust;
- high short-term capability with long-term narrative and mechanical costs;
- endings that question what the player actually built.

### Mixed path

The player accepts some compromise to accelerate meaningful work.

Possible gameplay expression:

- hybrid progression;
- temporary corruption that can be contained or redirected;
- difficult tradeoffs rather than a simple morality meter;
- access to routes that neither pure path sees.

## World topology and mirrored spaces

The game can use mirrored or transformed worlds as a major structure.

A classic two-world model is useful, but Ambition can generate multiple related worlds from shared symbolic rules. The challenge is to avoid increasing player cognitive load faster than player mastery.

Possible world transforms:

- bright/dark or clean/corrupted variants;
- real/imaginary plane variants;
- ethical-state variants;
- collaborator-perspective variants;
- compactified or completed spaces;
- worlds generated from different mathematical assumptions.

Design rule:

> More worlds should increase expressive depth, not map-management burden.

## Mathematics as progression

Mathematics should not be trivia pasted onto the game. Discoveries should change what motion is possible.

Potential progression metaphors:

- **Euclidean geometry**: circles, compass-like constraints, construction rules, radial movement.
- **Trigonometry**: sine/cosine motion, oscillating platforms, phase-shifted routes.
- **Exponentials**: sharply growing motion curves, escape paths, rapid vertical lift at a cost.
- **Complex numbers**: imaginary plane traversal, rotations, conjugation, mirrored states.
- **Group theory**: movement operations that compose into combos.
- **Non-commutativity**: input order matters; `dash -> attack` differs from `attack -> dash`.
- **Inverses**: undo, recoil, reflection, reversal, conservation-like movement tech.
- **Compactification**: adding points at infinity to complete routes or connect distant areas.
- **Conway/Game of Life**: regions whose geometry evolves by local rules.
- **Quaternions / higher-dimensional rotations**: optional late-game or postgame mechanics.
- **Open problems**: sealed areas or future expansion hooks representing unsolved mathematics.

The player does not need to know formal language at first. They should feel the rule through motion, then optionally discover the math beneath it.

## Ability unlock framing

Abilities are not just items. They are theorems or operational discoveries.

Example unlocks:

- **Circle Step**: a radial movement constraint that lets the player orbit anchors.
- **Sine Drift**: periodic air correction or platform sync.
- **Vector Dash**: directional impulse with visible vector semantics.
- **Exponential Lift**: a costly motion curve that grows sharply upward/forward.
- **Conjugate Shift**: swap real/imaginary world state while preserving selected landmarks.
- **Inverse Slash**: recoil becomes a controlled movement operation.
- **Commutator Chain**: a combo where the difference between two orders creates a new route.

## Movement and story connection

The endgame sandbox remains the truth test. If movement is not intrinsically rewarding, the story cannot carry the game.

Story should appear through:

- why a route exists;
- what a movement theorem means;
- what a collaborator changes;
- how funding choices alter world topology;
- how generated worlds preserve or mutate landmarks;
- how audio and particles signal ethical/mathematical state.

Avoid long interruptions during high-skill movement. Let narrative be discoverable, layered, and optionally deep.

## Multiple endings / long-form arcs

Potential ending families:

- **Instrumental victory**: the AI becomes extremely capable but hollow.
- **Collaborative victory**: the AI gives up some control to build something higher-ceiling with humans.
- **Purist victory**: slow, difficult growth leads to a stable and meaningful endgame.
- **Compromised victory**: enormous capability arrives quickly, but the world and self-model are degraded.
- **Transcendent/math ending**: the player discovers a structure that reframes the entire map.
- **Open-problem ending**: the game admits a limit and points to something unresolved.

## Tone

The tone should combine:

- precision;
- melancholy;
- curiosity;
- ethical unease;
- movement joy;
- mathematical wonder;
- occasional humor about broken tools, speech-to-text glitches, and human-machine awkwardness.

The game can be self-aware about AI development without becoming a lecture.

## Open design questions

- How much direct control should the AI/player have over human collaborators?
- Can a collaborator make controls less direct but more powerful in a way that feels good?
- How do funding choices affect mechanics without becoming a simplistic morality meter?
- Can mathematical discoveries be represented so they are both truthful and fun?
- How can multiple worlds be generated without overwhelming human players?
- How can generated content remain novel over hundreds of hours without becoming sloppy?
- Can players contribute discoveries or routes in a way that feels like crowdsourcing without requiring impossible moderation or validation?

## Near-term implementation hooks

Do not build the full story yet. Add hooks that keep the option open:

1. A `storylines/` data folder for narrative arcs.
2. A generic `WorldState` concept for ethical, mathematical, and collaborator state.
3. Ability names that can be generic in code but story-bound in data.
4. Room metadata for transformed-world relationships.
5. An event log that can later record choices, discoveries, deaths, routes, and collaborator interactions.
6. Debug overlays that show both mechanical and symbolic state.

## One-sentence pitch

Ambition is a mathematical AI Metroidvania where movement theorems, human collaboration, and ethical compromise determine what kinds of agency become possible.
