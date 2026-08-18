# Character authoring package — one character home, federated game facets

**State:** OPEN DIRECTION / ACTIONABLE MIGRATION PLAN. **PROMOTED to the live
ledger 2026-08-17 as D165, with canonical height as its first slice.**

This plan introduces a character-authoring boundary without requiring a flag-day
rewrite of the roster.

⚠ **it used to say it deliberately did not add itself to `tracks.md` or the live
queue, and that is SUPERSEDED** — Jon promoted it on 2026-08-17 because current
work reached it: the ruling that every character scale must multiply one SHARED
unit needs a home, and this package is it. ⛔ **promotion did not schedule the
other eight milestones.** A slice becomes work when something asks for it; the
ledger row is [`../queue.md`](../queue.md) D165 and it names which slice is live.

Related current doctrine and plans:

- [`../../adr/0032-authoring-is-declarative.md`](../../adr/0032-authoring-is-declarative.md)
- [`authoring-and-tools.md`](authoring-and-tools.md)
- [`sprite-renderer.md`](sprite-renderer.md)
- [`svg-component-character-migration.md`](svg-component-character-migration.md)
- [`combat-model.md`](combat-model.md)
- [`character-actions.md`](character-actions.md)

## Why this plan exists

Ambition increasingly reuses the same authored character in multiple substantial
2D games. That is useful product pressure, but it makes a formerly simple
boundary fuzzy:

> If George is one character, where do George's weight, animations, attacks,
> hit/hurt geometry, projectiles, VFX and SFX belong when Smash, Ambition and
> another platformer may interpret different subsets of those facts?

For a single game, the intuitive answer is strong: **author the character as a
coherent whole, balance the cast, and let the game import the cast.** The game
should not become a giant table that reaches into George and rewrites him after
registration.

The multi-game case should preserve that authoring experience rather than split
one character across unrelated repositories and runtime reach-ins.

The direction of this plan is therefore:

> **Character-specific authored facts live together. Rulesets/capabilities own
> the schemas and runtime interpretation of the facts they understand. A game
> imports a projection of the character package rather than mutating the
> character definition.**

This is not a new universal character ontology. It is an ownership and authoring
boundary over capabilities the engine already has.

---

# Settled direction

The following decisions are strong enough to implement against now.

## 1. The existing sprite submodule becomes the character-authoring home

Do **not** create another character-content submodule now.

`tools/ambition_sprite2d_renderer` has already evolved beyond a narrow renderer.
It contains or directly coordinates substantial character-specific authoring:

- bespoke Python character construction;
- SVG/paper-doll parts and composition;
- anatomy, silhouettes and palettes;
- poses and motion vocabulary;
- character-specific VFX authoring;
- portraits and presentation products;
- review tooling and authoring metadata.

Those things co-change when somebody works on a character. Splitting the
character-specific Python and SVG source away merely to preserve the old
"renderer" name would create cross-repository ceremony without a demonstrated
benefit.

Treat the existing submodule as the **emerging character-authoring repository**.
Its current name may remain during migration. The submodule README should be
updated early in implementation to state this expanded responsibility and to
make clear that a later rename is expected if the boundary proves itself.

A future conceptual organization may resemble:

```text
ambition_sprite2d_renderer/          # name retained initially
    characters/
        george_booul/
            identity / authoring context
            presentation/
                Python renderer
                SVG / paper-doll parts
                poses / motion
                VFX source
            smash/
                fighter facet source
            ambition/
                Ambition-specific authored facet source
            audio/
                character-specific SFX refs/recipes where appropriate
        alice/
        oiler/
        ...

    sprite2d/                        # reusable machinery, still internal for now
    motion/
    review/
    validation/
    publishing/
```

The exact directory layout is **not** settled by this plan. Ownership is.

### Authoring source is not restricted by file type

The boundary is **character-specific versus reusable**, not "data versus code."

Character-owned source may be:

- Python;
- SVG;
- RON;
- YAML where an existing authoring family already uses it;
- generated inert values;
- other formats justified by a concrete authoring workflow.

A bespoke Python renderer that defines Oiler's anatomy belongs with Oiler even
though it is executable code. Generic SVG composition, rasterization, motion
interpolation and sheet publishing remain reusable tooling.

Do not force expressive bespoke characters through one rigid declarative rig in
the name of package consistency.

## 2. The main Rust repository owns semantic protocols and runtime meaning

Moving authored facts into the character-authoring submodule does **not** make
the Python tooling authoritative for gameplay semantics.

The main repository/capability crates continue to own:

- `CharacterId` and stable runtime/content identity;
- content/facet validation and preparation;
- generic body, movement, combat and projectile semantics;
- the schema and lowering for game/ruleset-specific facets;
- deterministic runtime representation;
- rollback/persistence/runtime authority.

The character-authoring repository authors **instances** of those capabilities.
It does not redefine their meaning.

Conceptually:

```text
main repo / capability
    defines SmashFighterFacet semantics
              ^
              |
character authoring repository
    George authors a SmashFighterFacet instance
              |
              v
main preparation
    validates + resolves + lowers
              |
              v
PreparedSmashFighter / ordinary runtime components
```

This follows ADR 0032: authored input is inert; the engine validates, prepares
and installs it. Runtime simulation must never call Python to decide what a
character means.

## 3. Character authoring and ruleset specificity are orthogonal

A fact may be **authored with a character** while still being meaningful only to
one ruleset.

For example, George's Smash weight should be authored with George rather than in
a game-owned `George -> 1.35` table, while the Smash ruleset owns the function
that turns that authored weight into launch resistance.

```text
George authoring:
    smash_weight = 1.35

Smash rules:
    actual_launch = f(
        authored hit response,
        victim damage,
        victim smash_weight,
        DI,
        other match rules,
    )
```

Ambition may ignore `smash_weight` entirely.

Do not force a Smash balance number into a universal physical-mass field merely
because both are called "weight." If a genuinely shared physical mass concept
later earns multiple consumers, it can become a shared body fact then.

The same rule applies to other authored data:

- Smash move hitboxes are character-authored, Smash-owned semantics;
- Smash hurtbox timelines are character-authored, Smash-owned semantics;
- a platformer locomotion hull may belong to a different body/platformer facet;
- a character-specific projectile technique is authored with the character but
  lowers through generic projectile capabilities;
- a presentation clip is authored with the character but interpreted by the
  presentation capability.

## 4. A game imports/consumes character facets; it does not rewrite characters

The desired composition is:

```text
Character package
    identity
    presentation
    common/shared body offers where proven
    Smash fighter facet
    Ambition actor facet
    other future facets
           |
           v
Experience selects the facets it understands/wants
           |
           v
validate / resolve / prepare
           |
           v
immutable prepared content
```

A game should become closer to:

```text
install ruleset
select stage/world
select cast
run
```

and farther from:

```text
for every selected character:
    reach into CharacterDefinition
    rewrite vitals
    rewrite abilities
    rewrite moves
```

Registration-time mutation of shared character definitions is the most
important representation to eliminate early because it makes composition order
part of character identity and makes later restitching expensive.

Where the final schema decision is still unresolved, route game-specific
composition through **one pure named preparation/projection seam**. Restitching
one function later is acceptable; scattered reach-ins are not.

## 5. Existing authoring remains usable during migration

This plan does not require rewriting the current roster before useful work can
continue.

Existing sources remain valid behind adapters while ownership migrates:

```text
current Rust MoveSpec builders --------+
current CharacterDefinition values ----+--> preparation seam --> runtime
current Python/SVG presentation --------+
```

Then one source at a time may move into the character-authoring package:

```text
character package facet ---------------+--> same preparation seam --> runtime
```

The richer/general path should eventually become the only path, but the
migration is **consumer-driven and character-driven**, not a flag day.

Do not duplicate runtime authority during the transition. Compatibility adapters
may translate old authored source into the new preparation input; they must not
create a second live gameplay representation.

## 6. Combat response uses multiple authored regions, not a general vector field

A fully sampled or arbitrary vector field is more data and machinery than the
current platform-fighter customer needs.

The useful primitive is already close to today's `HitVolume`: an attack can
contain **multiple local-space hit regions**, and each region can carry its own
response direction/magnitude semantics.

Conceptually:

```text
Attack active phase
    hit region A
        shape
        damage
        launch/impulse response: constant direction + authored strength/growth

    hit region B
        shape
        damage
        launch/impulse response: different constant direction + strength/growth

    hit region C
        ...
```

That collection is sufficient for attacks such as:

- a base region that pushes mostly forward;
- an upper region that launches upward for juggling;
- a tip region that sends diagonally;
- a spike region that sends downward.

Directions are authored in the controlled body's/body-local reference frame and
must transform correctly with facing and resolved body/gravity frame.

### Radial response is a justified second primitive

Some attacks naturally want the direction to vary smoothly from a center. For
those, a radial response is a compact semantic primitive:

```text
Radial {
    origin: body-local point,
    strength/growth: ...
}

contact point -> direction away from origin
```

This does not require storing a field. The runtime derives the direction from
the contact point.

Do **not** add arbitrary sampled grids, spline fields or general mathematical
vector-field authoring until a real attack demonstrates that multiple constant
regions plus radial response cannot express the desired behavior cleanly.

### Preserve today's hit authoring through lowering

Current scalar knockback plus optional `launch_dir` should lower naturally into
the richer response representation during migration. The current roster should
not need to be rewritten merely to introduce the semantic seam.

Ruleset interpretation remains separate from authored response. The character
states the attack's intended response; Smash decides how percent, weight, DI and
other match rules transform it into final velocity.

## 7. Hurtbox and attack geometry are not automatically universal body geometry

Damageable hurtboxes and attack hitboxes are balance/gameplay authoring, not
necessarily intrinsic measurements of rendered anatomy.

A Smash fighter may intentionally have:

- crouching hurtboxes;
- attack-specific hurtbox changes;
- tumble geometry;
- ledge-hang geometry;
- temporary intangibility on a limb;
- oversized or stylized attack regions.

Those belong naturally in the Smash fighter facet until evidence proves some
subset is shared across rulesets.

Presentation/body landmarks such as hand, head, feet or attachment sockets may
be reusable across games and renderers, but promote them only when multiple
consumers actually benefit.

## 8. Character-specific audiovisual authoring may live with the character

A character package should be allowed to refer to character-specific VFX/SFX
without making the package itself an audio/render runtime.

Settled for now:

- character-specific VFX source may live with the character;
- character-specific SFX identities/references may live with the character;
- a character facet may reference an SFX recipe or published cue;
- authoring/review tooling may invoke external render tools to produce or review
  those assets;
- runtime consumes validated/published products and semantic IDs, not authoring
  Python.

The generic SFX synthesis/render engine remains separate **for now** because it
also serves non-character material such as UI, machinery, environment, pickups
and world mechanisms.

Whether character-specific SFX recipe source itself should move into the
character-authoring repository is intentionally left open below.

## 9. Preserve plural visual authoring

The existing sprite plan's "plural authoring, one validated published-asset
contract" remains correct.

The character package must not turn every character into the same paper doll or
same Python class hierarchy. It should organize ownership and published
semantics while preserving:

- bespoke procedural Python;
- SVG/paper-doll families;
- rigs where useful;
- family-specific helpers;
- procedural overlays;
- pose-specific corrections;
- future authoring styles that still satisfy the published contract.

Fighter uniqueness is a product requirement, not debt to normalize away.

---

# Target conceptual model

This is a design sketch, not a required Rust/file API.

```text
CharacterAuthoringPackage: george_booul

identity / authoring context
    stable CharacterId
    display/authorship metadata
    parody/inspiration notes
    gameplay intent
    bark/fallback-dialogue suggestions

presentation.sprite2d
    character-specific Python/SVG source
    published animation/clip vocabulary
    motion/pose source
    attachment landmarks where authored
    portraits
    character VFX source/references

body/platformer facts
    only facts proven to be meaningfully shared

smash.fighter
    Smash balance weight
    semantic repertoire bindings
    move timelines
    hit regions
    per-region constant/radial response
    hurtbox timelines
    projectile techniques
    Smash-specific presentation bindings

audio
    character-specific cue identities/references
    possibly character-specific recipe source later

other ruleset/capability facets
    authored only where George actually participates
```

Not every package needs every facet.

A character package containing a Smash facet must not force a non-Smash
experience to install Smash merely to use the character. The experience's
content import/projection decides which offered facets are admitted to that
prepared experience.

Once admitted, ADR 0032's rule applies: an authored facet must resolve to a
compatible installed handler or fail clearly; silently carrying a supposedly
active facet that nobody consumes is not acceptable.

---

# How this improves on a conventional Godot-style character scene

A conventional Godot project can make a character pleasant to author by placing
presentation, collision shapes, animation tracks and custom resources under one
reusable scene. That is a useful UX target: a character feels like one coherent
thing and spatial data can be manipulated visually.

The awkwardness appears when the same identity participates in several games or
rulesets. Inherited scenes and per-game resource overrides can make it difficult
to answer which mechanical facts belong to the character versus a particular
experience.

Ambition should preserve the best part — **one coherent character-authoring
home** — while improving the multi-game boundary:

```text
one authored character package
        +
capability-owned facet schemas
        +
experience-specific facet projection
        +
deterministic preparation
```

The eventual editor/workbench should therefore feel Godot-like in directness
without making a mutable scene tree the simulation authority.

For combat, an author should eventually be able to scrub a move and inspect:

```text
sprite / paper-doll pose
hurt regions
active hit regions
launch/impulse arrows per region
radial origin where applicable
self-motion
projectile spawn points
VFX/SFX events
cancel / landing windows
```

The deterministic move timeline remains simulation authority. Visual tooling is
a frontend over those semantics.

---

# Actionable migration program

These phases intentionally start with boundaries and one falsifier rather than a
roster migration.

## M0 — tell the truth about the sprite submodule

**Goal:** establish the repository responsibility before moving data.

Actions:

1. Update the sprite submodule README to state that it is evolving into the
   **Ambition character-authoring workspace**, not merely a sprite renderer.
2. Document the ownership rule there:
   - character-specific source belongs with the character;
   - reusable rendering/authoring machinery remains shared inside the repo;
   - main Rust capabilities own runtime/facet semantics;
   - a future repository rename is expected but deliberately deferred.
3. Inventory where one representative fighter's character-specific facts live
   today: presentation Python/SVG, metadata, moveset, weight, hit/hurt geometry,
   VFX, SFX references, projectile techniques.

Acceptance:

- a cold agent can open the submodule README and understand that adding or
  redesigning a character is in scope there;
- no source movement is required merely to close M0.

## M1 — one pure character preparation/projection seam

**Goal:** make later ownership movement cheap and eliminate registration-time
reach-ins.

Actions:

1. Identify every current place where a game adjusts a selected character by
   mutating a shared `CharacterDefinition` or equivalent source after
   registration.
2. Route those adjustments through one pure preparation/projection boundary.
3. Preserve existing authored sources behind adapters so runtime behavior need
   not change yet.
4. Make the boundary explicit enough that a future character facet can replace
   the old source without changing downstream runtime consumers.

Conceptually:

```text
current authored character facts
        +
ruleset preparation policy
        -> PreparedCharacterForContext
```

rather than:

```text
install game
    -> find shared character definition
    -> mutate fields in place
```

Acceptance:

- Smash character weight no longer requires a scattered registration-time
  mutation of generic character definitions;
- current games still prepare equivalent characters;
- changing where the weight is sourced later is one localized edit.

## M2 — package inventory and per-character discovery

**Goal:** let the character-authoring repo discover a character as one authored
unit without first converting every source format.

Actions:

1. Introduce a per-character package/index concept in the submodule.
2. For one character, enumerate the sources/products that already exist:
   - bespoke visual source;
   - portraits / clips;
   - authoring metadata;
   - motion/pose source;
   - VFX/SFX references/products;
   - currently external gameplay facet sources still awaiting migration.
3. Make character-specific presentation target discovery package-local rather
   than requiring edits to one ever-growing global roster where practical.
4. Add package-level validation/reporting that can say which pieces are present,
   missing, external or not yet migrated.

The package/index may initially point at legacy source locations. That is a
migration tool, not the end state.

Acceptance:

- an agent can ask for "George's package" and receive one coherent inventory;
- adding a new presentation target does not require unrelated renderer-global
  character logic;
- the report distinguishes authoritative source from generated/published
  products.

## M3 — define the first real game-specific facet: Smash fighter authoring

**Goal:** prove that character-owned, ruleset-specific gameplay authoring can
move beside presentation without moving runtime semantics into Python.

The main Rust Smash capability owns the facet schema and lowering. The
character-authoring package authors its values.

Start with the pieces already proven by current fighters:

- Smash balance weight;
- semantic repertoire bindings;
- move timelines / authored move values;
- hit regions;
- hurtbox timelines;
- projectile technique references/parameters;
- presentation event references.

Do not require the first facet to serialize every current bespoke move on day
one. Use the M1 preparation seam to permit mixed legacy/package-backed source
while the first complete fighter is migrated.

Acceptance:

- Ambition can use the same character identity without consuming the Smash
  facet;
- Smash can consume the facet without mutating the generic character source;
- a missing required Smash fact produces an authoring/preparation diagnostic,
  not a silent fallback;
- generic engine crates do not learn a closed list of Smash move names.

## M4 — make hit response a compact spatial authoring primitive

**Goal:** support juggling and expressive launch behavior without an arbitrary
vector-field format.

Actions:

1. Evolve the current hit-volume response into a representation where each hit
   region can express its own constant body-local launch/impulse direction and
   authored magnitude/growth semantics.
2. Preserve current scalar knockback + optional `launch_dir` through lowering
   during migration.
3. Add an optional radial response mode whose direction is derived from contact
   point relative to an authored body-local origin.
4. Keep victim damage, character weight, DI and match tuning in ruleset-owned
   final-response calculation.
5. Verify facing and arbitrary gravity/reference-frame transformation.

Do not add arbitrary vector fields.

Acceptance:

- one attack uses at least two regions with meaningfully different launch
  directions and produces the intended juggle/spacing behavior;
- one radial customer, if/when authored, needs no sampled field data;
- legacy attacks retain their existing behavior through compatibility lowering;
- headless tests prove local directions conjugate correctly with facing and
  gravity.

## M5 — character move review artifact

**Goal:** make gameplay authoring inspectable without reading runtime Rust.

Extend the existing character/sprite review tooling to render a compact move
review product for a selected character and move/time range.

Show, where available:

- character pose/presentation;
- hurtboxes;
- active hit regions;
- arrows for constant region responses;
- radial origins/directions;
- self-motion impulses;
- projectile spawn anchors;
- VFX event positions;
- SFX event/cue labels;
- startup/active/recovery/cancel/landing annotations.

Also emit a concise machine-readable semantic summary suitable for an agent.

This review surface should consume the same authored/prepared semantics the game
uses; do not create debug-only duplicate geometry.

Acceptance:

- an agent can explain why a move launches/juggles the way it does from the
  review product and semantic summary;
- a human can see obvious sprite/hitbox/response disagreement before launching
  the game.

## M6 — migrate one complete fighter as the falsifier

**Goal:** prove the repository and facet boundary with one substantial real
character.

Choose one fighter with enough existing complexity to exercise the design
(George Booul, Pirate Admiral, or another similarly rich current fighter).

Move/converge that character's relevant authored facts into its package while
keeping generic schemas/runtime in the main repo.

The proof should include:

- character authoring context;
- presentation source;
- motion/pose vocabulary;
- Smash weight;
- full enough repertoire to exercise normals/aerials/specials/recovery;
- hit/hurt geometry;
- the new per-region response representation;
- projectile technique where the fighter has one;
- VFX and SFX references;
- review artifact generation.

Acceptance:

```text
edit fighter package
    -> package validation
    -> render/review
    -> main content preparation
    -> Smash runtime
```

without editing an unrelated global character registry or mutating a shared
character definition after registration.

The fighter must remain mechanically bespoke. Success is lower authoring/change
amplification, not fewer unique moves.

## M7 — migrate opportunistically, not by flag day

After one complete fighter succeeds:

- migrate another character when substantial work already touches it;
- delete the corresponding legacy authority as each character crosses;
- promote genuinely repeated helpers into reusable character-authoring
  primitives only after multiple customers prove them;
- keep bespoke character code bespoke when abstraction would reduce clarity or
  personality.

Do not stop feature work merely to relocate the whole roster.

## M8 — decide whether the repository should be renamed or split

Only after non-presentation character facets have lived successfully in the
submodule should we decide the physical repository name/boundary.

Likely outcomes include:

- rename the current repository to something like `ambition_character_authoring`
  while keeping generic sprite machinery inside it; or
- extract a truly reusable renderer library later if an independent consumer
  actually wants it without Ambition's character corpus.

Do not extract a generic renderer repository merely because a subset of the code
could theoretically be reusable.

---

# Agent-native authoring requirements

This program should make character work easier for autonomous agents, not merely
move files.

A mature package surface should answer questions like:

```text
what facets does George offer?
which experience consumes each facet?
what moves does George have in Smash?
what does forward-air do at 0.12 s?
where are its hit and hurt regions?
which direction does each region launch?
which animation/pose/VFX/SFX does it reference?
what files/products own those facts?
what is missing for George to participate in Smash?
what will break if I rename this move/cue/clip?
```

Validation should point at character-owned source and explain missing capability
handlers, references or required fields without making an agent reverse-engineer
Rust registration topology.

The ideal authoring loop is:

```text
inspect character package
    -> edit character-specific source
    -> validate semantics/references
    -> generate compact review products
    -> prepare/run relevant game
    -> observe behavior
    -> iterate
```

A new use of an existing engine mechanic should normally be a character-package
edit, not a new Rust feature.

---

# Things we deliberately should not pre-generalize

The following are explicitly **not** requirements of the first implementation:

- one universal character file format;
- one universal character rig;
- one universal animation model;
- a general sampled vector-field format for attacks;
- a universal hurtbox that every game must consume;
- a universal physical `mass` standing in for every game's balance weight;
- a new character submodule separate from the existing sprite/authoring repo;
- a runtime Python dependency;
- one repository per character;
- an editor that competes with Godot before the underlying semantics are
  coherent;
- migration of the complete roster before current game work continues.

---

# Open design questions — deliberately unresolved

Implementation should collect evidence for these rather than silently deciding
them from this plan.

## O1 — What is the exact package/document format?

The package needs stable identity, source discovery, facet association,
validation and provenance, but that does not imply one monolithic file.

Questions:

- one manifest plus plural source files?
- generated package inventory from Python modules?
- RON for cross-language gameplay facets?
- how much is explicit versus convention/discovery?

Select the smallest format that the first complete fighter can validate and
prepare without duplicating authority.

## O2 — What is the exact main-repo type/API seam?

Names such as `CharacterPackage`, `CharacterFacet`, `CharacterProjection` and
`PreparedCharacterForContext` are conceptual in this plan.

The implementation should reuse ADR 0032's existing content/facet preparation
machinery where it fits instead of growing a parallel character-only compiler.

## O3 — Which body facts are truly shared across games?

✔✔ **PARTLY ANSWERED 2026-08-17 — `canonical height` got its evidence and its
ruling, and the rest of the list stands open.**

- **canonical height — SHARED, and it is the first slice.** The evidence this
  question asked for arrived as three separate maintainer reports that turned out
  to be one defect: the snake and AI slop too big, Sanic too small in his own
  game, the cove pirates mis-sized against a chibi robot. Cause: every
  `collision_scale` multiplies its OWN sheet's frame size, so heavies 1.95,
  pirates 1.60 and `robot` 2.10 are not comparable numbers — the robot's is the
  largest and he reads chibi.
- **the unit is ONE BASE-GRID PIXEL**, 16 to a tile (`defaultGridSize: 16` across
  the shipped worlds), which is what the collision AABBs already effectively use.
  ⚠ a quality tier scales the ART, never the declared height.
- **height is a CONTRACT**: art scales to it, and a tight tolerance **warns** when
  the scale drifts. ⛔ warns, does not refuse.
- **landmarks are OPTIONAL SLOTS** — authored where an author has something to
  say, and a consumer must work without them. ⛔ never make one required.
  ⚠ Jon: *"we may eventually have skeletons available in game"*, and a skeleton
  subsumes hand-authored landmarks rather than extending them.

Still requiring evidence:

- physical mass;
- locomotion hull;
- default movement tuning;
- intrinsic capabilities.

Keep ruleset-specific facts in ruleset facets until multiple consumers prove a
shared semantic meaning.

## O4 — Where should character-specific SFX recipe source live?

Strong candidates:

1. character package contains only semantic SFX references; recipes remain in
   `ambition_sfx_renderer`;
2. character-specific recipe source moves beside the character while generic
   SFX synthesis machinery stays in the SFX renderer;
3. character authoring invokes the SFX renderer as an external authoring engine
   and publishes/reviews the result.

Any solution should let an agent working on one character see its audiovisual
identity without duplicating generic audio DSP machinery.

## O5 — Should the SFX and music render engines eventually merge?

Both are true audio-authoring/render engines in a way the current sprite repo is
increasingly not: they transform audio authoring descriptions into deterministic
published audio and diagnostics.

There may be value in a shared audio-rendering repository or common lower-level
library, especially for mastering, loudness, synthesis primitives, reports and
publishing.

But the music tool also authors reactive/stemmed musical structure, while SFX
has different recipe and review needs. Do not merge them merely for symmetry.
Measure shared machinery and independent change cadence first.

This question is outside the initial character-package migration.

## O6 — How should presentation clips and gameplay moves relate?

A move needs presentation without making animation frame progression the
simulation authority.

Open questions include:

- semantic clip IDs versus per-move direct clip references;
- whether presentation mappings live in the Smash facet or a presentation
  binding facet;
- how grounded/airborne/contextual move variants share or override clips;
- how one presentation source serves several games.

The invariant is settled: gameplay timeline is authoritative; presentation
samples/binds to it.

## O7 — Where do character-specific VFX recipes/source stop and generic VFX begin?

The character package should own the character's visual identity, but generic
VFX primitives and runtime intent semantics belong below any one character.

Promote a helper only after repeated customers demonstrate that it is genuinely
reusable.

## O8 — How are package facet versions related to save/rollback/content identity?

ADR 0032 already calls out this obligation for versioned facets. The first
serialized gameplay facet must deliberately define how its schema/version enters
content fingerprints and compatibility decisions.

Do this before minting a stable public `@1`, not after a migration exists.

## O9 — What happens to generated sprite/audio products?

The authoring package owns source, but generated binary products need not become
canonical history in the same repository.

Preserve the existing principle that runtime consumes published products while
source and generated binary churn may have different storage/distribution
boundaries.

## O10 — When is a graphical character editor worth building?

The likely eventual UX is a Godot-like character/move workbench with direct
manipulation of geometry and timeline scrubbing.

Do not build it before M5 proves a coherent inspectable semantic model. The
first useful frontend may simply extend the existing Python review tooling.

## O11 — How much narrative/personality content is a character fact?

Character-owned authoring metadata, suggested barks, fallback dialogue,
personality and gameplay intent naturally belong with the character.

Scene-specific dialogue, quest state and world relationships may remain in
world/narrative authoring even when they reference the character. Do not move
all dialogue into the character package merely because a character speaks it.

## O12 — How should an experience select offered facets?

A package may contain Smash and Ambition facets while one experience wants only
one of them.

The content/import API needs a clear projection rule so:

- unused facets do not force irrelevant capabilities into a game;
- admitted facets cannot silently go unhandled;
- selection is deterministic and inspectable;
- authoring diagnostics can explain why a facet is or is not active.

Use existing content/capability preparation rather than inventing a second
module system.

---

# Falsifiers

Reconsider the design if implementation produces any of these outcomes.

## The game becomes the character database

If Smash accumulates tables such as:

```text
George -> weight / hurtbox / attacks
Alice  -> weight / hurtbox / attacks
```

then character-owned data has leaked back into game composition.

## The character-authoring repository owns runtime semantics

If the Python sprite/character tooling must define what `SmashWeight` means or
runtime simulation calls into Python, dependency direction is wrong.

## Using George requires installing every facet George has ever authored

If Ambition cannot import George without installing Smash, package projection is
wrong.

## Adding an ordinary move requires unrelated engine registry edits

Once a mechanic exists, authoring another use should normally touch the
character package, not a closed central move census.

## Package migration duplicates live authority

During migration, adapters may translate legacy authored source, but there must
not be two independent runtime truths for the same weight/move/hurtbox.

## "Simplification" makes fighters homogeneous

If the package system reduces expressive bespoke move/mechanism design in order
to share more code, the abstraction is aimed at the wrong layer.

The intended simplification is **authoring ceremony and ownership**, not fighter
identity.

---

# Success criteria

This program has reached a credible first plateau when all of the following are
true:

1. the sprite submodule explicitly presents itself as the character-authoring
   workspace even if the repository name is unchanged;
2. character-specific presentation source remains there and is discoverable as
   one package;
3. one complete fighter authors a real ruleset-specific gameplay facet there;
4. the main repo owns facet semantics, validation/preparation and runtime
   lowering;
5. game composition no longer mutates shared character definitions to balance
   that fighter;
6. the same character identity can be imported by another game without
   consuming irrelevant facets;
7. the fighter's hit/hurt geometry and per-region launch response are authored
   with the fighter and produce deterministic runtime behavior;
8. multiple constant-response hit regions support juggling/spacing, with radial
   response available when a real attack needs it and no arbitrary vector-field
   representation;
9. one compact review artifact shows the move's presentation, geometry and
   response semantics together;
10. VFX/SFX references are discoverable and validated from the character
    package;
11. editing an existing mechanic on the migrated fighter does not require
    editing unrelated engine registration code;
12. no runtime Python dependency or duplicate content authority was introduced.

At that point, use the evidence to decide repository renaming, wider roster
migration, audio-repository relationships and richer editing surfaces.
