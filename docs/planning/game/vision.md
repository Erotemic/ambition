# Game vision — Ambition

Ambition is the flagship game: a 2D platformer aiming for the systemic depth,
world persistence and character reactivity usually associated with much larger
RPGs.

## North star

> Every upgrade a theorem, every boss a failed objective function, every biome a
> mathematical world model.

The controlled body begins as a small embodied AI without a clear purpose. The
world's pressure is uncertainty rather than a tidy moral binary.

## World-first development direction

The previous planning emphasis on building a linear intro/story spine first is
retired as an **implementation order**. Those story beats remain desired content,
but the near-term game goal is:

> Put the robot into a substantial persistent world with the real movement,
> possession, item, ability and interaction vocabulary. Make exploration and
> world state feel coherent before relying on story gates to carry progression.

Use [`open-world-roadmap.md`](open-world-roadmap.md) and
[`systemic-progression.md`](systemic-progression.md).

### World state should explain progression

Prefer:

- body capability/property;
- theorem ability;
- held/equipped/physical item;
- repaired/powered/opened world mechanism;
- environmental danger;
- social cooperation or knowledge;

before an invisible "quest stage N permits this door" check.

Explicit story gates remain valid where authored sequence is genuinely important.

## Story layers we still want

The basement/lab escape, Kernel, Fia arc, Alice/Bob cryptography material,
Eve/Mallory, theorem upgrades, factions and boss arcs remain product intent. They
should increasingly **react to and explain a world that already has systemic
truth**, rather than being the only thing that gives the world state.

The simulation decides what exists, where actors/items are and what changed.
Dialogue/AI may interpret those facts and may be incomplete or mistaken.

See [`reactive-characters-and-dialogue.md`](reactive-characters-and-dialogue.md).

## Multiplayer direction

Ambition should support solo, local couch co-op, online co-op and mixed
local+remote parties through one body/control architecture. Presentation may be
shared, fixed split or adaptively split/rejoined. When the rules allow independent
exploration, participants may occupy different rooms/regions.

See [`multiplayer.md`](multiplayer.md).

## Strong content ideas kept alive

- theorem-as-ability progression;
- failed-objective-function bosses;
- mathematical-world-model biomes;
- cryptography-as-traversal ideas such as challenge/response, public/private key
  doors and commit/reveal mechanisms;
- stable gates versus AI-exploited ripples;
- courier/message-integrity situations;
- killable/persistent characters and world consequences;
- the Hall/cast as ordinary world/content rather than engine-special ontology.

These are product ideas, not mandatory engine abstractions.

## Open game-design questions — deliberately unresolved

- How large should the first systemic region be before we layer serious story?
- Which theorem abilities are the best early progression vocabulary?
- How much sequence breaking and capability asymmetry is desirable?
- Which characters need persistent off-room existence early?
- How much authored dialogue versus reactive/agentic dialogue produces the right
  tone?
- How harsh should persistent item/actor consequences be?

Do not answer these by encoding engine-level quest gates prematurely.
