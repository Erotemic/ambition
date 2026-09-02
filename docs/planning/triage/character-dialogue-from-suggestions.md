# Interact dialogue should read the same IR the barks already do

**Status:** SHELVED 2026-07-26, design settled, nothing built. Opened after
wiring nine generated characters into the Hall and finding two channels reading
two different sources.

> **RE-MEASURED against `7ca4f1df6` (2026-09-02). ⭐ STILL UNBUILT AS DESIGNED — AND
> LARGELY UNNECESSARY, because the problem was solved the other way the decision
> allowed: by hand-authoring.**
>
> - The generation is **not** built. `fallback_dialogue` still feeds only barks
>   (`CharacterCatalog::fallback_dialogue`), and `npc_dialogue_request`
>   (`features/npcs.rs`) still reads only the LDtk `Interactable`'s
>   `dialogue_id`, falling through to `"generic_npc"`. The two-channel table
>   below is an accurate description of that code path today.
> - ⭐ **But the Hall was covered by authoring instead.** The catalog gained a
>   per-character `hall_dialogue_id`, and `known_dialogue_ids`
>   (`ambition_content/src/dialogue/yarn.rs`) folds those ids into the
>   validator's accepted set so authored `hall_<id>` nodes need no second
>   hand-maintained list. Counted at HEAD: **149 catalog rows, 124 declaring a
>   `hall_dialogue_id`** (8 explicitly `None`), against **131 authored
>   `title: hall_*` Yarn nodes**.
> - That is exactly the escape the 2026-07-26 decision left open — *"a
>   hand-authored node of the same title overrides it by existing … so writing
>   real dialogue is never blocked."* Somebody wrote the dialogue.
>
> ⇒ **So the value of building the generator has dropped, and the question it
> should be re-opened against has changed.** It is no longer "generated
> characters say a placeholder when you talk to them" — in the Hall, 124 of them
> do not. What remains is (a) the ~25 catalog rows with no `hall_dialogue_id`,
> and (b) every room that is NOT the Hall, where a character with a real
> `fallback_dialogue` voice still reaches `generic_npc` on interact. ⛔ Re-scope
> to that before implementing; a generator written to the original framing would
> now be generating over 124 characters that already have authored nodes.

## The state today

A character arriving from the sprite pipeline declares
`dialogue_hints.suggested_barks` / `fallback_dialogue` in its target's
`ACTOR_METADATA`. `character_notes.py` carries those into the catalog row's
`fallback_dialogue`, and `CharacterCatalogEntry::bark` falls through to that pool
whenever a situation has no authored one. So a generated character **mutters in
its own voice** on its pedestal, when struck, when provoked, and while idling.

Pressing *interact* on that same character reaches a different channel:

| channel | reads | result |
|---|---|---|
| ambient bark | catalog `fallback_dialogue` | the character's own line |
| interact conversation | a Yarn node named by `dialogue_id` | `generic_npc` placeholder |

`generic_npc` is a real authored node whose text is *"This NPC has no named Yarn
node yet."* — so the fallback is honest, just not the character.

**Fixed already (do not re-fix):** LDtk stores an unset string field as `""`, so
a spawn with no conversation used to arrive as `Some("")` and get forwarded
verbatim, producing `start(""): Yarn node not found` and an NPC that opened
nothing at all. `npc_dialogue_request` now treats blank as absent. That is why
these characters reach `generic_npc` instead of failing — the placeholder is the
current *correct* behaviour, not a bug.

## The decision (Jon, 2026-07-26)

Generate a conversation per character from `fallback_dialogue`; **a
hand-authored node of the same title overrides it by existing** — the same rule
the bark fallback already follows, so writing real dialogue is never blocked.

Not a second, non-Yarn dialogue path for "characters without scenes". That means
two conversation runtimes and two sets of bugs.

## Shape

Generated Yarn is committed text, like `music_registry.ron` — a regen script, not
runtime compilation:

```
title: character_npc_marie_curry
---
Marie Curry: Careful, it is still reactive.
-> Close.
===
```

- Emit one node per catalog character into a single generated
  `assets/dialogue/sandbox/generated_characters.yarn`, **skipping any title the
  authored `.yarn` set already defines** — that is the override.
- Emit a node for EVERY character, using the generic line when the row has no
  suggested dialogue, so `character_<id>` always resolves and the runtime needs
  no "does it exist" branch.
- `npc_dialogue_request` routes a blank/absent `dialogue_id` to
  `character_<character_id>` rather than `generic_npc`.
- Regen must work on a fresh clone (project invariant).

Sources are declared in `game/ambition_content/src/dialogue/yarn.rs`
(`YarnSpinnerPlugin::with_yarn_sources`, `YarnFileSource::InMemory`), so a
generated file joins the set the same way the authored ones do.

## What a generated conversation IS

One line (rotating over the same pool the barks use) and a Close. Deliberately
not choices, branches, or state: that is what a hand-written node is for, and a
richer generated node would become something authors have to fight rather than
replace.

## Prior art

- `docs/recipes/adding-a-character.md` §0 — the three-command hookup this
  completes.
- `tools/ambition_ldtk_tools/ambition_ldtk_tools/character_notes.py` — the
  existing target→catalog join; the dialogue generator is its sibling and should
  read the same normalized `CharacterNotes`.
- `crates/ambition_platformer2d_actor_monolith/src/features/npcs.rs::npc_dialogue_request` — the one
  routing decision to change.
