---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/systems/actors-brains-and-character-content.md
  - docs/concepts/content-and-provider-boundaries.md
---

# Add a character

The common case is provider data plus generated/published presentation. Adding a
character must not require editing core movement/combat code or adding a new
actor species.

## 0. The short path

A character whose sprite target already declares `ACTOR_METADATA` is three
commands away from standing on a Hall pedestal and speaking in its own voice:

```bash
# 1. catalog row, including the target's prose and its suggested lines
PYTHONPATH=tools/ambition_ldtk_tools:tools/ambition_sprite2d_renderer \
  python3 -m ambition_ldtk_tools.character_notes --splice --target <target>

# 2. a pedestal in the Hall
PYTHONPATH=tools/ambition_ldtk_tools \
  python3 -m ambition_ldtk_tools.generate_hall_of_characters

# 3. the sheet itself (generated; gitignored)
PYTHONPATH=$PWD/tools/ambition_sprite2d_renderer PYTHON=python3 \
  ./regen_sprites.sh --target <target>
```

Step 1 is idempotent and never rewrites a row that already exists, so re-running
it after hand-tuning a row is safe. Run it with no `--target` to REPORT which
targets are missing which notes; it refuses to splice a whole-renderer scan,
because not every render target is a character (variant rigs like
`npc_pirate_heavy` are intentionally row-less).

The generated row is a starting posture — `patrol_peaceful` + `striker_swipe`,
`tier: MainHall`. Retune it afterwards; getting the character into the game is
the part worth automating.

The rest of this recipe is the long form: what those fields mean, and what to do
when the short path does not apply.

## 1. Localize the current contracts

```bash
python scripts/agent_query.py "character catalog provider sprite brain action set"
python scripts/agent_query.py tests "character catalog sprite"
```

The Ambition provider's catalog currently lives at:

```text
game/ambition_content/assets/data/character_catalog.ron
```

The reusable schema, validation, binding, brain, and action-scheme machinery
lives in focused engine/domain crates. Named characters and defaults remain in
the provider.

## 2. Add provider data

Create a stable provider-owned character ID and select existing data where
possible:

- display/presentation identity;
- body/archetype/capability composition;
- default brain preset;
- default action-set/action-scheme inputs;
- sprite target/manifest;
- dialogue/roster/tags as applicable;
- an `authoring_description` recording the parody/source figure, name joke,
  visual and thematic references, and behind-the-scenes design intent;
- a `gameplay_description` translating that concept into a suggested role and
  mechanics without replacing the live brain/action-set authorities;
- character-specific bark pools plus `fallback_dialogue` lines for contexts
  that do not yet have bespoke scene/Yarn writing.

The authoring and gameplay descriptions are guidance, not necessarily canonical
lore. Barks and fallback dialogue are usable defaults that later scene writing
may expand, replace, or ignore. New characters should author all of them even
though the fields default empty while legacy rows are migrated.

Author these on the SPRITE TARGET's `ACTOR_METADATA` (under
`authoring_description`, `gameplay_description`, and
`dialogue_hints.suggested_barks` / `dialogue_hints.fallback_dialogue`) rather
than typing them into the catalog by hand — §0 carries them across, and the
character's writing then travels with its art.

`fallback_dialogue` is not decoration: `CharacterCatalogEntry::bark` reaches for
it whenever a situation has no authored pool, so a character with suggested
lines and no bark pools still speaks in its own voice when struck, when
provoked, while idling, and on its pedestal. Writing a real pool for a situation
takes that situation back and leaves the others on the fallback.

Copy a nearby current entry rather than a snippet from an old document. Run the
catalog tests immediately; the schema changes faster than this recipe should.

## 3. Create and publish presentation

List registered sprite targets and use the renderer's explicit publish step:

```bash
cd tools/ambition_sprite2d_renderer
python -m ambition_sprite2d_renderer list
python -m ambition_sprite2d_renderer canonical <target>
python -m ambition_sprite2d_renderer sheet <target>
python -m ambition_sprite2d_renderer publish <target>
```

The registered generator/target source is authoritative. Choose the authoring
family that best serves the character: direct procedural Python, a shared
procedural family, a config-driven generator, a rig or SVG-part workflow, a
scene graph, or a specialized hybrid. A rig is optional and should not be
introduced merely for consistency.

Review the canonical pose, sheet, idle row, anchors, actor metadata, and debug
hitbox views before publishing. Runtime files belong in the provider asset flow
selected by the current target contract. The game consumes the published sheet
and metadata, not the target's internal pose or drawing representation.

## 4. Place or register the character

Use LDtk tooling/editor to add the relevant spawn entity and set the stable
character ID. Do not hand-edit LDtk JSON. Per-instance brain overrides should be
explicit authored fields; geometry or hostility must not silently infer a brain.

Regenerate provider-owned derived rooms such as the Hall only through their
current generator:

```bash
PYTHONPATH=tools/ambition_ldtk_tools \
  python -m ambition_ldtk_tools generate hall-of-characters --help
```

## 5. Validate

```bash
./run_tests.sh -p ambition_content -k character
./run_tests.sh -p ambition_content -k sprite
./run_tests.sh -p ambition_characters -k catalog
./run_tests.sh -k hall
```

Then load the authored room through the real headless/provider path. Confirm:

- the stable ID resolves exactly once;
- brain and action scheme are derived from the selected live authorities;
- sprite/prompt/dialogue are derived consumers, not alternate identity stores;
- cleanup/reset/restore reconstruct the actor;
- no reusable crate learned the character's name.

For a genuinely new behavior primitive, follow
[`extending-brains-and-action-sets.md`](extending-brains-and-action-sets.md).
