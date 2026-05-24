# Codegen creates rollup entries that aren't real characters

**Date:** 2026-05-24
**Tags:** `codegen`, `data-modeling`, `catalog`, `agent-bias`

## Mistake

During Phase 3 of the character-catalog refactor, an agent wrote a
codegen script that synthesized catalog entries for every renderer
target. The script naively assumed: "every entry in
`list-targets` is a real character; emit a catalog row for it."

The renderer's target list includes some entries that are NOT real
characters — they're authoring-organization shorthands. Examples:
  - `pirate_heavy` is a multi-variant rig whose ACTUAL characters are
    `pirate_heavy_broadside_bess` / `_iron_mary` / `_salt_annet`. The
    bare name has no canonical sprite output.
  - `robot_heavy` is similar — variants `_bastion` / `_arsenal` /
    etc. The bare name's publisher renders to `generated/` but never
    installs.

The codegen blindly emitted `npc_pirate_heavy` and `npc_robot_heavy`
catalog entries. The Hall of Characters generator then placed
pedestals for them, and the runtime tried to load their non-existent
sprites — the user saw colored-rectangle placeholders for
"characters" that didn't exist as real characters anywhere in the
game.

The agent's subsequent fix attempted to MAKE the placeholder real
by adding a flat-name publisher in `pirate_heavy.py` that aliased
one variant. The user pushed back: "if pirate heavy isn't an actual
sprite that is supposed to be rendered don't shoehorn it in." The
correct fix was to REMOVE the bogus catalog entries, not to make
them resolve.

## The principle the agent missed

Codegen output is data, not architecture. When source data has
multiple shapes (canonical characters vs. authoring shorthand), the
codegen needs to be selective. Otherwise the codegen "completes the
catalog" with rows that compile, parse, and pass internal-
consistency tests — but never represent real game content.

Two ways to detect this:
1. Hand-author the catalog and use codegen only for *bulk fields*
   (sprite path, brain preset) of confirmed-real entries.
2. Mark renderer targets as `aspirational | variant_root | character`
   and have the codegen skip non-`character` entries.

The agent chose neither, ended up with an over-generated catalog,
and only the user's eye-test caught it.

## Pre-mistake context

The agent had:
- The renderer's `list-targets` output (89 character entries).
- The catalog with 24 hand-authored entries.
- A goal: "every renderer-registered character has a catalog entry."

The mistake was reading "every entry in list-targets" as "every
character," not as "every renderer surface, some of which are
authoring shorthands."

A close reading of the renderer's `targets/characters/pirate_heavy.py`
(or `robot_heavy.py`) reveals their `VARIANTS` dict — the agent could
have noticed that these targets emit variant outputs, not a
canonical character. But the codegen treated them uniformly.

## Repair shape

```python
# Skip targets whose publisher emits *only* variants, not a canonical
# `<target>_spritesheet.png`. These targets are authoring shorthands
# for the variant rig, not real characters.
VARIANT_ROOTS = {"pirate_heavy", "robot_heavy"}

def character_id_for(target: str) -> str | None:
    if target in VARIANT_ROOTS:
        return None  # skip — variants are catalog entries individually
    ...
```

Plus a post-codegen test that asserts every catalog entry resolves
to a load path at runtime (`every_character_catalog_entry_resolves_a_load_path`),
so an aspirational entry without a sprite trips CI rather than
shipping as a placeholder.

## Why this is a good benchmark question

The agent has to:
1. Recognize that codegen output needs domain reasoning, not just
   structural transformation.
2. Find the signal: which renderer targets are "shorthand" vs
   "canonical character"?
3. Decide whether to enrich the codegen (skip shorthands) or to
   trim the catalog after generation.
4. Resist the natural agent bias to "complete the catalog by
   making the placeholder real" — the correct move is sometimes
   "delete the bogus row."

## Compact question

> A codegen script reads the renderer's `list-targets` output (89
> entries under `[characters]`) and emits a catalog row for each.
> After the codegen runs, you notice that two entries —
> `npc_pirate_heavy` and `npc_robot_heavy` — never resolve to a real
> sprite; the renderer publishes their files only as
> `<target>_<variant>_spritesheet.png` (e.g.
> `pirate_heavy_broadside_bess_spritesheet.png`), never as the bare
> `<target>_spritesheet.png` the catalog points at.
>
> Without modifying the renderer's publish behavior, what's the
> minimum change to the catalog and codegen so these shorthand
> targets stop ending up as colored-rectangle placeholders in the
> game?

## Validation

```bash
~/.cargo/bin/cargo test -p ambition_sandbox --lib \
    every_character_catalog_entry_resolves_a_load_path
```

Should fail before the fix (with `npc_pirate_heavy` in the
unresolved list), pass after.
