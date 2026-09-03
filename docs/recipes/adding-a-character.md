---
status: current
last_verified: 2026-08-13
related_docs:
  - docs/systems/actors-brains-and-character-content.md
  - docs/concepts/content-and-provider-boundaries.md
---

# Add a character

Adding a character should extend provider-owned authored content and the shared
character-preparation path. It should not require a new actor species, enemy
archetype or player-only construction branch.

## 1. Localize the current character sources

Ambition uses more than one **authoring source**, but preparation produces one
runtime character value. Start by locating the provider row/definition and a
nearby character with similar needs:

```bash
python scripts/agent_query.py "CharacterDefinition character catalog prepared character"
python scripts/agent_query.py tests "try_register_character PreparedCharacterDefinition"
```

Important distinction:

- `character_catalog.ron` still carries broad provider-owned cast metadata,
  presentation/writing/default data and tool-facing rows;
- `CharacterDefinition` carries/registers reusable character composition and
  intrinsic body/kit facts;
- preparation resolves/folds those inputs into `PreparedCharacterDefinition`,
  which is what runtime body construction consumes.

The deleted enemy `ArchetypeSpec`/`CharacterRoster` body system is not part of <!-- cite-ok: names the deleted archetype system to say it is not the workflow -->
the workflow.

## 2. Start from sprite/authoring metadata when appropriate

For Ambition characters whose sprite target carries `ACTOR_METADATA`, the
character-notes tool can seed the provider catalog row and writing metadata:

```bash
PYTHONPATH=tools/ambition_ldtk_tools:tools/ambition_sprite2d_renderer \
  python3 -m ambition_ldtk_tools.character_notes --splice --target <target>
```

Treat the generated row as a starting point, not as the entire character body.
Keep the required authoring metadata with the sprite target where that workflow
owns it:

- `authoring_description` — parody/source inspiration and transformation notes;
- `gameplay_description` — intended role/mechanics;
- suggested bark pools and `fallback_dialogue`.

Do not hand-copy stale catalog snippets from old planning documents.

## 3. Author/register the character composition

If the character can instantiate as a body, ensure the provider registers a
`CharacterDefinition` through the current character-registration seam. Copy a
nearby current definition from `game/ambition_content` rather than recreating an
old archetype shape.

Author intrinsic facts on the character or referenced typed documents where
appropriate:

- body source/size and hurt geometry;
- vitals/death traits;
- locomotion and body abilities;
- moveset/action repertoire;
- autonomous profile where the character itself owns one;
- contact behavior or other intrinsic capability;
- sheet/presentation references and voice floor.

Placement hostility/disposition, session seat, participant assignment, spawn
location, encounter role and ruleset are contextual and should stay outside the
character identity.

A character may intentionally delegate some authoring defaults to the provider
catalog/source during preparation. The important invariant is that **runtime
construction receives one resolved `PreparedCharacterDefinition` rather than
choosing among parallel body authorities**.

## 4. Generate and publish presentation

Use the registered sprite target's supported generation/publish path:

```bash
cd tools/ambition_sprite2d_renderer
python -m ambition_sprite2d_renderer list
python -m ambition_sprite2d_renderer canonical <target>
python -m ambition_sprite2d_renderer sheet <target>
python -m ambition_sprite2d_renderer publish <target>
```

The generator/target source is authoritative for generated art. Choose the
character-authoring family that best fits the design; do not introduce a rig or
shared generator merely for uniformity.

## 5. Place the character through LDtk/provider content

Use LDtk/editor tooling for spatial placement and author the stable character ID.
Do not hand-edit `.ldtk` JSON. Per-instance placement/controller overrides should
be explicit fields or rules, never inferred from geometry or from an obsolete
archetype name.

For generated provider rooms such as the Hall, use their current generator rather
than editing derived output by hand.

## 6. Validate the preparation and construction path

Use focused tests located from the current source, then exercise the real
provider/headless path. Verify:

- the stable character identifier resolves during preparation;
- the prepared definition is complete for the intended body;
- placement/session/controller facts remain contextual;
- human, AI and possession paths do not change the character's intrinsic kit;
- art/dialogue/prompt consumers derive from resolved character/provider data
  rather than becoming alternate identity authorities;
- reset/room transition/restore can reconstruct the body; and
- no reusable engine crate learns the character's proper name.

For a genuinely new reusable behavior primitive, extend the shared engine/action
vocabulary instead of hiding it in the character registration.
