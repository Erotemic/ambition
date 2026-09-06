# Adding and validating content without rebuilding Rust

Operational recipe. You should not need to open an engine source file to follow
it.

## TL;DR

```bash
# build the validator ONCE (~seconds; it links no game and no Bevy renderer)
cargo build -p ambition_content_cli --bin ambition_content

# thereafter, every edit costs milliseconds
./target/debug/ambition_content game/ambition_content/assets \
    --asset-root crates/ambition_platformer2d_actor_monolith/assets
```

⚠ **pass that `--asset-root`.** Ambition's binary payloads live in the engine's
asset tree, not beside `pack.ron`, so the pack-root-only default reports every
sprite sheet and all 72 music tracks as missing. That is the asset check working
— it is looking where you told it to — but it is not the invocation you want.

Measured on the shipped pack (17 sources, 322 content identities — 140
characters, 72 music tracks, 24 items, 24 enemy archetypes, 15 SFX cues, 9
bosses, 9 boss encounters, 9 boss seeds, 17 presets — 342 assets, 298 resolved
references): **~17 ms**, and it passes STRICT with zero missing assets. The rebuild it replaces is ~10 minutes. Exit status is 1 on refusal and
0 on pass, so this is usable as a CI gate and not only as a report.

## Add a character

1. Open `game/ambition_content/assets/data/character_catalog.ron`.
2. Pick a `brain_presets` key and an `action_set_presets` key that already
   exist — presets are shared on purpose (`peaceful`, `striker_swipe`), and one
   preset per character is the thing to avoid.
3. Add a row under `characters` with the sprite paths and tier.
4. Run the validator.

That is the whole loop. **No Rust changes, no rebuild.**

## Re-author an item

Edit its row in `game/ambition_content/assets/data/items.ron` and run the
validator. Same loop, no rebuild.

⛔ **but do not delete a row.** The file is POSITIONAL — the slot index is what
binds a row to its item, and there is no key. Removing one shifts every later
row up a slot and re-authors the wrong items, with the short tail falling back
to built-in defaults so the grid still looks full. To blank a slot, keep the row
and empty its fields. The validator refuses a grid that is not exactly 24 rows
and says this; before the `item_catalog` schema existed, nothing did.

## What it checks, and what each refusal means

| code | meaning | typical fix |
|---|---|---|
| `unknown-schema` | a source declares a schema no installed capability owns | typo, or the capability is not installed — the refusal lists the installed ones |
| `schema-version-mismatch` | the source declares a version the handler does not read | migrate the source; a handler reading another version reads different fields |
| `missing-capability` | the pack (or a row) needs a capability this composition lacks | install it, or drop the content that needs it |
| `unknown-preset` | a `default_brain` / `default_action_set` names nothing | the refusal suggests the nearest existing preset |
| `unresolved-reference` | a character/role/world/action reference finds no target | define it, or point the field elsewhere |
| `missing-asset` | an authored path resolves to no file | see **assets** below |
| `duplicate-identity` | two sources define the same canonical id | both sides are named; rename or delete one |
| `unknown-field` | an authored field nothing consumes | **not a warning.** A typo'd field means a mechanic that silently never fires |
| `conflicting-module-contribution` | e.g. two characters share one display name | give one its own |
| `conflicting-source-alias` | two manifest lines are the same file under different schemas | one file, one meaning |

Every refusal also prints **the checks that did not run**. The stages depend on
each other — reference resolution needs every definition to exist — so a list
that stops early says so instead of looking complete.

## Assets

Binary payloads are git-ignored but present on disk (`docs/recipes/adding-an-asset.md`).
On a checkout whose art was never generated they are legitimately absent, so:

* **`--advisory-assets`** — a missing asset is a warning; the pack still
  compiles. Use this while authoring.
* **default (strict)** — a missing asset refuses. Use this for packaging and
  release.
* **`--no-asset-check`** — do not look. The prepared pack records `<unchecked>`
  provenance, so it is visibly not claiming anything about its assets.

✔ **The three art-less characters are FIXED** (re-measured 2026-08-03).
`npc_giant_gnu_hands`, `npc_hypatia_prime` and `npc_le_beast` were reported by
this tool on its first run as having no art; their sheets and manifests are all
present now, and the whole pack passes the STRICT asset path with zero missing
assets. Recorded because the old note read as a standing defect and is no longer
one — and because the tool finding them is what got them made.

## Add a content family (a new schema)

A capability owns its schemas; there is no central content enum to edit.

```rust
// in the capability's own crate
use ambition_content_pack::{
    ContentSchemaHandler, FacetOutcome, FacetSource, SchemaRegistration, /* … */
};

struct MySchema;

impl ContentSchemaHandler for MySchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let file: MyFile = match ron::from_str(facet.text) { /* … */ };
        for row in file.rows {
            let id = facet.content_id(&row.name);
            out.define(id.clone(), canonical(&row));            // an identity
            out.refer(PendingRef::new(/* … */));                // a reference
            out.need_asset(AssetRequirement::new(/* … */));     // an asset
            out.require(CapabilityId::new("combat"));           // a dependency
            out.report(facet.diagnostic(/* … */));              // a problem
        }
    }
}

pub fn my_schema() -> SchemaRegistration { /* id, version, capability, doc, handler */ }
```

Then register it in **both** compositions — they are two sites and nothing in
the compiler makes them agree:

* `ambition_platformer2d::content::engine_schemas()` — what the GAME compiles
  with. Gate the line on the capability's own feature (`#[cfg(feature =
  "ambition_items")]`) so a composition without the capability does not claim to
  own its schema.
* `ambition_content_cli::default_registry()` — what the TOOL validates with, plus
  a dependency on the owning capability crate with its `content_pack` feature.

⚠ Miss the engine side and the game panics at startup on its own content; miss
the CLI side and `ambition_content` reports a pack the game runs fine as
`unknown-schema`. `content_pack_registry` in `game/ambition_content/tests/`
fails on either, and it reads `pack.ron` off disk so it sees what you edited.

Two more rules:

* **Use `#[serde(deny_unknown_fields)]`** on the authored types, and map ron's
  `NoSuchStructField` to `DiagnosticCode::UnknownField`. Match the variant, not
  the message text.
* **Make `canonical` semantic**, not the raw bytes. Reflowing a comment must not
  move the fingerprint; changing a value must. Round-tripping through
  `ron::ser::to_string` is usually right.

## Add a source to a pack

`pack.ron` holds an explicit source list, not a glob — a glob would make the
pack's contents depend on filesystem enumeration order.

```ron
sources: [
    (path: "data/character_catalog.ron", schema: "character_catalog", version: 1),
]
```

## What you get back

`PreparedContentPack` is a value, not a loader: pack identity and version,
namespace, source manifest with per-source fingerprints, required capabilities,
schema versions, **canonically ordered** content, asset provenance, resolved
references, diagnostics, and a stable 64-bit content fingerprint (FNV-1a over
canonical bytes — identical on every platform, unlike `DefaultHasher`).

`--fingerprint` prints just the fingerprint, for a cache key or a build script.

## Where this is enforced

`ambition_content_pack::compile` is the **only** validator. The CLI, the
standard tests, CI and packaging all call it. There is deliberately no "quick
check the CLI does" — two implementations disagree, and the disagreement is
worse than either being wrong.
