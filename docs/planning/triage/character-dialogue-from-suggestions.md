# Interact dialogue should read the same IR the barks already do

**Status:** SHELVED 2026-07-26, design settled, nothing built. Opened after
wiring nine generated characters into the Hall and finding two channels reading
two different sources.

> **RE-MEASURED 2026-09-17 (and against `7ca4f1df6` on 2026-09-02 before that).
> ⭐ STILL UNBUILT AS DESIGNED — AND THE RESIDUAL IS TWO ORDERS OF MAGNITUDE
> SMALLER THAN THIS BANNER USED TO SAY.**
>
> - The generation is **not** built. `fallback_dialogue` still feeds only barks
>   — its only consumers are `CharacterCatalogEntry::bark`'s fallthrough and the
>   catalog accessor beside it — and `npc_dialogue_request`
>   (`features/npcs.rs`) still reads only the LDtk `Interactable`'s
>   `dialogue_id`, falling through to `"generic_npc"` when it is absent, blank
>   or whitespace. It never consults the catalog, although the same
>   `InteractionKind::Npc` carries the `character_id` that would let it. The
>   two-channel table below is an accurate description of that code path today.
> - ⭐ **The Hall was covered by authoring instead.** The catalog gained a
>   per-character `hall_dialogue_id`, and `known_dialogue_ids`
>   (`ambition_content/src/dialogue/yarn.rs`) folds those ids into the
>   validator's accepted set so authored `hall_<id>` nodes need no second
>   hand-maintained list. That is exactly the escape the 2026-07-26 decision
>   left open — *"a hand-authored node of the same title overrides it by
>   existing … so writing real dialogue is never blocked."* Somebody wrote the
>   dialogue.
>
> ⛔⛤ **AND THE POPULATION WAS WRONG, WHICH IS WHY THE RESIDUAL LOOKED BIG.**
> This banner said *"149 catalog rows, 124 declaring a `hall_dialogue_id` (8
> explicitly `None`)"* and concluded that *"~25 catalog rows"* had none. 149 is
> every `"key": (` in `character_catalog.ron`, which counts the 17 brain
> presets, action-set presets and controller profiles authored **before**
> `characters: {`. Inside that block there are **132 character rows, and every
> one of them declares the field** — 124 `Some`, 8 `None`. ⇒ Residual (a) is
> **8 rows**, not ~25. Re-derived by taking the `characters:` block by brace
> depth and matching `^        "<id>": ($` inside it, rather than grepping the
> file.
>
> ⭐⭐ **AND RESIDUAL (b) — "every room that is NOT the Hall" — IS TWO
> PLACEMENTS.** Measured over the six distinct `.ldtk` projects (the two demo
> `assets/worlds/` paths are symlinks into `ambition_map_assets`, so resolve
> before counting); three of the six author an `NpcSpawn` at all —
> `hall_of_characters` 129, `sandbox` 26, `intro` 8. **163 placements, 161 with a
> non-blank `dialogue_id` and 2 without** — `npc_puppy_slug` in `gravity_lab` and `npc_viking_warrior` in
> `sanic_sandbox`. Both of those characters already have an authored Hall node
> (`hall_npc_puppy_slug`, `hall_npc_viking_warrior`); the id is simply not
> carried into the LDtk placement. Neither has a `fallback_dialogue` either —
> only **24 of the 132 rows** declare one at all, so *"a character with a real
> `fallback_dialogue` voice"* describes 24 rows and not the cast.
>
> ⚠ **THE HALL'S FOUR NUMBERS RECONCILE EXACTLY, WHICH IS WHY THEY LOOK
> DIFFERENT.** 129 Hall placements, 124 catalog rows declaring an id, 131
> authored `title: hall_*` Yarn nodes: 6 of the 129 placements name a
> `character_id` that is not a catalog row at all (`sanic`, `super_sanic`,
> `mary_o`, `mary_o_tall`, and the two snakes-on-a-plane exhibits), leaving 123
> placed catalog characters; `sandbag_infinite` declares an id with no
> placement, giving 124; and the 131 nodes are those 124 plus those 6 plus
> `hall_player__self`. **Zero** placements disagree with their row, and **zero**
> declared ids lack a node.
>
> ⇒ **SO THE GENERATOR HAS ALMOST NO REMAINING CONSUMER IN SHIPPED CONTENT.**
> What is left is 8 catalog rows with `hall_dialogue_id: None` and 2 LDtk
> placements missing an id — each of which is one line of authoring, not a
> generation pipeline. ⛔ Re-scope against those two numbers before implementing
> anything here; a generator written to the original framing would be generating
> over 161 placements that already resolve.
>
> ⚠ **THE TWO PLACEMENTS ARE NOT CLOSED HERE BECAUSE THEY ARE NOT IN THIS
> REPO.** `game/ambition_map_assets` is a submodule
> (`github.com/Erotemic/ambition_map_assets`, at `cb7062a95`), so setting
> `dialogue_id` on the `gravity_lab` and `sanic_sandbox` exhibits is a commit in
> that repository. Both target nodes already exist and read fine outside a
> pedestal — `hall_npc_puppy_slug` is four lines of blorbing with no reference
> to the Hall.

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
