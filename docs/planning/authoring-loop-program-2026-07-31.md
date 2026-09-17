# Authoring loop - remaining work

**Status:** residual work only, re-verified against HEAD on 2026-08-13.

> **Re-checked against `4768eb13d` (2026-09-02) and again 2026-09-17: the
> residual work is STILL ACCURATE.** Spot-checked each candidate under "Remaining
> work": `boss_sheets.ron` and `ambition_sprite_sheet::boss::BossSheetSpec` are
> both still live, so the duplicate boss-sheet authority is unresolved; and the
> action seam's three types (`SemanticActionId`, `ActionRegistry`,
> `InstalledActions`) are all present, in
> `crates/ambition_input/src/semantic.rs`. ⚠ `content_validation.rs` is **655**
> lines, not the 622 this line recorded — the file grew 5% while the row
> describing it as a consolidation candidate stood still, which is the direction
> that makes the candidate more worth doing, not less.
>
> ✔ **THE AGED EXAMPLE IS REPLACED, 2026-09-17 — this row asked for it "when
> this is picked up", and it is picked up.** §2 used to offer *"a provider-owned
> action such as `grapple`"*, and grapple is an ENGINE traversal ability
> (`crates/ambition_abilities/src/traversal/grapple.rs`, alongside blink, dive and
> mark/recall), not anything registered through the provider seam. ⚠ The path
> moved in the abilities carve (D33, 2026-09-03) and the COMPANY it keeps changed
> with it: `possession` and `flyline` did NOT move — they are runtime-registered
> control authority that only shared the old directory name — so the illustration
> was four siblings, not six. An illustration that names something built the
> other way is exactly the sentence a later session takes as evidence that the
> work was done.
>
> ⇒ **The replacement is a REAL one, found rather than invented:**
> `SemanticActionId("pulse")` in `examples/capability_demo/src/lib.rs`, which the
> SDK's own doc already uses as its example (`module.actions(&[capability_demo::PULSE_ACTION])`).
> ⛔ And the census that found it is worth carrying: across the whole workspace,
> `SemanticActionId` is named in **seven files**, and `capability_demo` is the
> ONLY non-engine one. So the provider seam has exactly one provider-owned action
> in the tree, and it lives in an example — which is the honest evidence for §2's
> claim that the seam is present and the physical-input half is what is missing.

The original 2026-07-31 program connected three goals: compiled content packs,
participant-scoped semantic actions, and causal inspection. Most of that program
has landed. The full execution record is archived at
`docs/archive/planning-superseded/2026-08-13/authoring-loop-program-2026-07-31.md` — <!-- cite-ok: removed from the checkout 2026-09-05; naming the path is the point -->
removed from the checkout 2026-09-05, still in git history.

Do not recreate the completed phases below. This file owns only the remaining
authoring-loop gaps that still have a concrete implementation payoff.

## Verified landed

- `ambition_content_pack` provides `ContentPackDraft`, `SchemaRegistry`,
  `ContentSchemaHandler`, prepared/lowered content, diagnostics and fingerprints.
- `ambition_platformer2d::content::engine_schemas()` is the engine schema
  composition used by Ambition's pack.
- Ambition consumes prepared pack output for the migrated families instead of
  validating one parse and running from another.
- Character, item, fighter-brain, boss-profile, boss-seed, boss-band,
  boss-encounter, encounter-wave, music-registry and SFX-registry content have
  compiler/schema ownership.
- Participant input contexts are per-seat through `SeatInputContexts`; bindings,
  rebinding and semantic action registration have their own focused continuation
  in [`engine/participant-action-system.md`](engine/participant-action-system.md).
- The causal inspector is public through `ambition_platformer2d::causal`, accepts
  game-owned facts, and is exercised against the real Ambition app composition.

## Remaining work

### 1. Remove the remaining real duplicate content authorities

Do this family by family. The goal is not to force every authored backend through
one compiler. The goal is to remove cases where the same authored fact has two
independent readers or two independent runtime authorities.

Current concrete candidates:

- **Boss sheet metadata.** Ambition still passes
  `assets/data/boss_sheets.ron` directly into the boss catalog while
  `ambition_sprite_sheet::boss` also carries built-in `BossSheetSpec` fallback
  definitions. Determine which values are true fallback policy and which are a
  second copy of provider-authored sheet geometry. Make the provider-authored
  path authoritative and keep only intentional engine fallbacks.
- **Boss animation/sprite maps authored in Rust.** Where a provider fact is still
  represented by hand-maintained maps beside generated/published sheet metadata,
  collapse it onto the published provider data rather than adding another
  registry.
- **Dialogue.** Ambition's Yarn files are still embedded and parsed through the
  dialogue path, while cross-content validation separately reasons about
  dialogue references. If a content-pack schema can replace a duplicate reader,
  add that handler and delete the displaced validation. Do not wrap Yarn in a
  schema merely for uniformity if no authority is removed.
- **World/LDtk cross-reference validation.** `WorldManifest`, `LdtkVocabulary`
  and the LDtk lowering pipeline are legitimate backend-specific authoring seams.
  Do not migrate worlds into `ContentPack` simply to make the list uniform.
  Instead, move any duplicated cross-reference rule out of
  `game/ambition_content/src/content_validation.rs` when a canonical LDtk/world
  owner already validates the same invariant.

For each slice, the acceptance condition is simple: one authoritative read of the
fact remains, diagnostics still name the authored source, and the old reader is
deleted.

### 2. Finish provider-defined semantic actions through physical input

The open action identity and module-contribution seam are already present:
`SemanticActionId`, `ActionRegistry`, `InstalledActions`, and
`ModuleDraft::actions` can register a provider-owned action such as
`capability_demo`'s `pulse` — the one provider-owned action in the tree, and the
one the SDK's own doc uses.

The remaining limitation is physical input: the Leafwing `InputMap` still bottoms
out in the engine's finite `Platformer2dInputActionMonolith`. A provider-defined
semantic action therefore cannot yet own an ordinary device binding without a
parallel/private route.

This work is owned in detail by
[`engine/participant-action-system.md`](engine/participant-action-system.md).
Do it there rather than creating a second action architecture in this program.

### 3. Prove one external capability uses the three public seams together

Once the physical-action limitation above is removed, exercise one capability
outside the actor monolith that contributes:

- a provider-owned schema or prepared content facet;
- a provider-owned semantic action with a real device binding; and
- a provider-owned causal fact visible through the public facade.

The capability should not need a new central enum variant, a private content
reader, or an internal-shaped facade import. This is an integration proof of the
existing seams, not a request for another general framework.

## Completion condition

Archive this residual plan when:

1. every remaining duplicated content reader named above has either been removed
   or explicitly shown to be an intentional distinct authority;
2. provider-defined semantic actions can participate in the normal physical
   binding/cue path; and
3. one external capability demonstrates schema + action + causal composition
   through public engine APIs.

## Updated acceptance dependency

The [architecture review](engine/architecture-reassessment.md) keeps the external
provider/authoring loop as a real engine customer. Its next acceptance surface is
not a missing flow interpreter: that interpreter and a `read_and_seize` customer
already exist. A11/A12 in the [frontier](engine/actor-monolith-work-frontier.md)
close installed-technique validation and flow bounds before new content relies on
them.

The external witness must discover support, author valid and invalid content,
receive source-local diagnostics, prepare a revision, drive semantic actions
through the same physical-input acceptance road, and step its resulting behavior
without flagship-only imports. A manifest-only fixture and a screenshot do not
prove that loop. Keep field-authority cleanup bounded to actual duplicate values.

## Runtime-loaded authoring continuation

This residual plan still owns its named duplicate-reader/action/causal seams.
The [extension model](engine/extension-model.md) now owns a separate missing cost
boundary: build content or procedural code independently, then load it without
relinking the host. Reuse this program's existing preparation and diagnostic
surfaces. Do not call compiled in-process packs a completed runtime-loaded format.

The first acceptance is a real move edit through a small builder and the current
installed-technique/character preparation road. Later procedural code uses the
same state/port contract as its native reference. LDtk, sprites and audio retain
their current domain tools; this continuation does not unify every asset compiler.
Priority and completion are tracked only in [the queue](queue.md).
