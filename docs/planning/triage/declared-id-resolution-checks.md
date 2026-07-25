# Declared-id resolution checks — catch the silent binding, not at boot

> **State:** TRIAGE — LOW PRIORITY, opened 2026-07-25. Shape agreed, not
> scheduled. Jon ruled out the obvious answer (a boot-time validation pass) on
> startup cost; the options below are the alternatives that cost the shipped
> binary nothing.

## The failure this is about

Content declares a thing by STRING ID and something else resolves that id to an
asset. When the id names nothing, the resolution returns `None`, the caller
treats it the way it treats "this build has no assets", and the content
**simulates perfectly while producing nothing at all**. No warning, no log line,
no failing test.

Five instances surfaced in two days, all in the same demo:

| What was declared | What resolved it | Symptom |
| --- | --- | --- |
| `WorldItemArt` → `sprites/props/super_mary_o_spark_blossom.png` | render item visuals | a fire-flower you collect and never see |
| a runtime `spawn_pickup` (Sanic's scattered rings) | the dynamic-visual families | rings that magnetize and credit, invisibly |
| the Solid Snake's collision box | nothing — hand-authored beside the art | a box with no relation to the sprite |
| the `dead` sheet row | `CharacterAnim::from_name("death")` | a death animation that could never play |
| `mary_o_you_died` (had it been bound early) | the provider audio fragment | a cue requested successfully, into silence |

The common cause is not carelessness. It is that `Option` is doing two jobs at
once: *"this build legitimately has no assets"* and *"this content named
something that does not exist."* They are indistinguishable at the call site, so
the second silently inherits the first's tolerance.

## Why not a boot-time pass

Rejected by Jon on 2026-07-25: it puts the cost on every launch forever to catch
a class of mistake that is made at authoring time. Startup budget is not the
place to pay for author errors.

## Options

### 1. Make the miss loud, at the miss site — smallest, immediate

Roughly six sites resolve an art/audio id and discard the `None` with a bare
`?`. Replace those with `error_once!`.

Cost is **zero on the success path** — the new branch only runs where a lookup
has already failed. It is the pattern already in use in this tree
(`scatter_rings_on_hit`'s spend guard, the room-transition guards), so it needs
no new vocabulary.

Catches: anything, including ids composed at RUNTIME, but only once the content
is actually exercised.

### 2. Extend the `every_*` tests — the gate, and already the house pattern

This repo already asserts whole-registry resolution in tests:

- `every_live_music_track_resolves_under_web_served_assets`
- `every_renderer_target_has_catalog_entry_or_explicit_exclusion`
- `every_entity_sprite_has_a_unique_asset_id_in_sprite_entity_namespace`

The sprite one checks **target → catalog**. Every bug above was the **other
direction**: a declared id → a generated target that does not exist. So this is
a gap-fill in an established pattern rather than a new mechanism. It runs in CI
and costs the shipped binary nothing.

Should cover, in the shipped app registries: `WorldItemArtManifest` entries,
authored `PickupSpec.sprite` ids, provider audio-fragment track ids, and catalog
`manifest` targets.

Catches: everything statically declared — four of the five above. Cannot see
ids assembled at runtime.

### 3. Generated ids as symbols — kills the class

Have the sprite generator emit its target names as Rust consts, and have content
reference the symbol instead of a string literal. A missing target becomes a
**compile error** and neither of the above is needed for the cases it covers.

Right time is whenever the sprite pipeline is next open; not worth opening it
for this alone. Note the interaction with
[`stable-identifier-centralization.md`](stable-identifier-centralization.md):
this is the same "an id should not be a bare string" question, arrived at from
the asset side.

## Recommendation

**2 as the gate, 1 alongside.** The test catches the statically declared
majority; `error_once!` covers the runtime-composed remainder that no static
check can see. **3** is the real fix and should be folded into the next sprite
pipeline change rather than scheduled on its own.

## Note on scope

`max_health`, `WorldItemArt`, prop-sheet ids and audio track ids are each
resolved by DIFFERENT machinery, so this is deliberately not proposing one
unified resolver. The claim is narrower: whatever resolves a declared id should
be able to tell "absent by design" from "absent by mistake", and today it
cannot.

## Provenance

Opened out of the 2026-07-25 Mary-O playtest round (commits `7b2ee66a6`
through `e94107cda`). Every row in the table above was found by a player
noticing something missing, not by a test.
