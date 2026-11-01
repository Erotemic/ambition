# Ambition on the engine — flagship customer

Ambition is the primary product driver and deepest customer of the reusable
engine. It is not a thin demo that waits for a "finished engine" before game
production begins. The game and engine co-evolve: real Ambition world pressure
reveals missing reusable capability, and the resulting capability returns to the
game through supported engine surfaces.

Story/pillars: [`vision.md`](vision.md). Open-world build order:
[`open-world-roadmap.md`](open-world-roadmap.md). Systemic progression:
[`systemic-progression.md`](systemic-progression.md). Multiplayer:
[`multiplayer.md`](multiplayer.md).

## Structural rule

Named Ambition content and product policy live in game/provider crates. Reusable
actor, world, persistence, navigation, item, view, authoring and service semantics
belong in engine domains when another game could plausibly consume them with the
same meaning.

The same rule applies to acceptance-game wings and content integrations: they may
share the engine and even appear inside Ambition, but they do not define a second
engine ontology.

## Current flagship pressure

The most important Ambition-driven engine work is:

1. finish the controlled-character actor kernel;
2. make LDtk/kinematic world mechanics first-class;
3. persistent world residency and instance/item accounting (several rooms can be
   live at once through `LiveRooms`; see
   [`../engine/open-world-runtime-and-residency.md`](../engine/open-world-runtime-and-residency.md));
4. make capability/item/world-state progression queryable;
5. add platformer reachability/navigation;
6. support persistent/spawned actor populations;
7. multiplayer and multiview: two local players in two live rooms with a view
   each is built, and so is the adaptive split in one room; online and mixed
   parties are not (see
   [`multiplayer.md`](multiplayer.md));
8. layer reactive character intelligence/dialogue over authoritative world facts.

The current story arcs remain desired product work, but the world-first roadmap
outranks the old assumption that a linear narrative slice is the implementation
spine.

## Open: every Ambition NPC can defend itself and eligible allies join fights

**Current gap (2026-11-01 source review):** Retaliation and danger-room ally
rally already exist. `body/aggression.rs` provokes a struck NPC; `body/rally.rs`
joins a fight only for a non-driven, live NPC with `RetaliatesWhenHit`, an
available provoked brain profile **and** a usable `ActionSet` melee/ranged
attack or a nonempty `ActorMoveset`. The ranger fixtures prove the happy path,
not completeness of the entire authored cast. Many rows still select
`default_action_set: "peaceful"`. This is *not* a request to make every NPC
aggressive on sight or to replace the existing faction/room logic.

### NPC-KIT-1 — inventory real prepared combat coverage (independent, first)

- **Owner:** Ambition's prepared cast and content, not the generic rally
  engine. Start at `game/ambition_content/assets/data/character_catalog.ron`,
  `game/ambition_content/assets/data/movesets/`,
  `crates/ambition_platformer2d_actor_spawn/src/actor_spawn/brain_builders.rs`
  (`authored_provoked_policy`), and
  `game/ambition_app/tests/npcs_fight_beside_you.rs`.
- Build a focused test/census from *prepared* character definitions for
  placeable, interactable Ambition NPCs (include NPCs placed in dangerous
  rooms, not just fighter roster members). For each report the character ID,
  room(s), effective faction/disposition, retaliation policy, provoked
  profile, and actual executable attacks **on the built body**. Do not count
  mere move names, animations or a peaceful profile as a working attack.
  Distinguish props/immobile scripted actors explicitly instead of silently
  skipping all characters with missing data. Keep the census executable, not
  a dated table in planning.
- **Acceptance:** The test deterministically names every eligible NPC without
  a usable offensive kit/policy. It must fail on at least one real missing
  coverage case before content is repaired. Existing ranger tests still pass.

### NPC-KIT-2 — repair content in bounded batches (after KIT-1)

- Take **one area or roughly 5-10 uncovered characters per change** from the
  KIT-1 report. Author or reuse character-appropriate melee/ranged attacks
  and provoked profiles through the existing `ActionSet`/`ActorMoveset` and
  provider preparation paths. Do not wait for every area to finish before
  landing a verified batch. Preserve peaceful dialogue/idle behavior until
  actual provocation/rally; do not introduce a universal attacking brain,
  character-ID engine branches, or attacks for inanimate props. The Ambition
  provider already installs the cast's fighter specials; do not reimplement
  that bridge.
- Extend the real room-simulation fixture to exercise (1) retaliation when
  struck even in a safe room; (2) an aligned NPC helping when a foe engages
  the player in a room marked `dangerous`; (3) an aligned NPC **not** joining
  in a safe room or without an actual fight; (4) the rally ending after the
  foe is gone; and (5) an NPC without the player's faction not joining.
  Assert at least one attack from **that NPC's prepared kit** connects/starts,
  not just a hostile disposition. Keep target/faction and same-room checks.
- **Acceptance per batch:** A prepared/built NPC from each repaired kit
  actually attacks when provoked; KIT-1's uncovered-ID list shrinks. The
  existing `npcs_fight_beside_you` suite stays green. **Campaign close:** no
  eligible NPC remains without a usable fight kit. Do not regenerate sprites
  for gameplay-only action-set changes.

**Focused check:** `cargo test -p ambition_app --test app_it npcs_fight_beside_you`
(default desktop features include `rl_sim`). When complete, delete this
implementation section instead of leaving a finished campaign in planning.
