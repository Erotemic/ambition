# Render, animation and VFX — Engine 1.0 program

**State:** OPEN — built-in semantic VFX path is real; third-party particle
provider work remains demand/trigger driven.

The old temporary extension has been consolidated into this authority; its execution history remains in git.

## Goal

Keep simulation responsible for **what happened** and presentation responsible
for **how that fact looks/sounds on this host/view/quality tier**.

The engine should support:

- authored sprite effects;
- lightweight procedural/built-in particles;
- optional richer particle providers when a real effect requires them;
- persistent emitters when a semantic source actually persists;
- explicit reference-frame/orientation semantics;
- presentation quality/budgets;
- multiview-safe presentation;
- confirmed-effect handling where an external/non-rewindable side effect cannot
  simply be replayed.

Do not make animation/VFX another gameplay timeline.

## Current boundary

The reusable effect vocabulary lives outside game-specific presentation. The
visible host installs the VFX presentation consumer, while simulation/domain
code emits semantic requests/facts.

Current built-in rendering already supports authored effect clips and fallback
particles. The `generic_action_fx` sheet is consumed for ordinary hit markers,
and `ParticleBudget` is part of the quality policy surface.

A product may omit the visible VFX consumer without changing simulation outcome.
Backend dependencies therefore point upward into presentation/host composition,
not down into gameplay.

## Settled architecture

### Presentation providers are presentation only

A third-party particle engine may consume semantic VFX requests. Gameplay must
not depend on its particle entities/components or use its state as simulation
authority.

### One-shot and persistent effects have different lifetimes

A one-shot can be emitted as an event/request carrying all facts needed to draw
it. A persistent emitter needs a stable semantic source and reconciliation so
presentation can create/update/remove its visual representation without becoming
its authority.

Do not force both through an ever-growing `ParticleKind` taxonomy.

### No universal VFX backend trait in advance

The built-in renderer and any future provider should first prove what they need
in common. Compose plugins/adapters over semantic requests; introduce a trait
only if two real providers need the same runtime interface.

### Optional means optional in the compile graph

A richer provider should be behind a presentation capability/feature and must not
be required by headless simulation or minimal consumers.

### Quality is presentation policy

Particle counts, trail density, expensive shaders and other purely visual work
follow the active quality/raster budget. Lower quality must preserve semantic
readability rather than delete gameplay information.

> ⛔⛔ **MEASURED `3593ccb9f` (2026-09-02): THE FX ART DOES NOT FOLLOW IT.** The rule
> above is true of particle counts and shader scale and false of the sheets
> themselves. Three of the eleven asset loaders never receive the quality budget,
> and two of them are the FX pair:
>
> - `load_fx_sheets` (`crates/ambition_platformer2d_actor_monolith/src/character_sprites/assets.rs:740`) — the boot core;
> - `ensure_fx_sheet_loaded` (`crates/ambition_platformer2d_actor_monolith/src/character_sprites/assets.rs:772`) — the per-character owned road;
> - ✔ `load_prop_sheet_for_target` (`crates/ambition_platformer2d_actor_monolith/src/character_sprites/assets.rs:838`) — **NOT a gap, checked separately.** It passes `TextureResolutionScale::Full` explicitly and says why in place: *"this path never consults a quality budget, so nothing was asked for beyond `Full` and nothing but the authored PNG was loaded"*. Its docstring also scopes it to a demo that registers one animated prop outside the asset catalog. A documented decision on a narrow road is not an oversight, and an audit that counts it as one is inflating itself.
>
> `load_fx_sheets` builds its set `with_sprite_folder(…)`, a FIXED folder, so it
> cannot select a variant even though the variants are authored and on disk: 12
> fx PNGs in `sprites/` (1.3 MB), 12 in `sprites_0_25x/` (964 KB), 12 in
> `sprites_potato/` (**68 KB**). Measured in the hall: `fx-sheet` is **7.7 MP at
> Potato, High and Ultra alike**.
>
> ⭐ **AND IT IS INVISIBLE WHERE THE MEASUREMENTS ARE TAKEN.** At Ultra,
> full-resolution FX art is exactly right, so a 3090 sees no defect. It costs
> only the configurations that ASKED for less — Steam Deck, mobile, web,
> weak-GPU desktop — which is the target class this engine's vision names.
>
> ⛔ A ROUTING defect, not a proposal to draw fewer pixels: the fix gives a
> Potato user the 68 KB sheets they chose and changes nothing at Ultra. Owner for
> the residency half:
> [`asset-preparation-and-residency.md`](asset-preparation-and-residency.md).

### Reference frames are explicit

An effect that is gravity-relative, surface-relative, attacker-facing or world
fixed should say so through semantic pose/reference-frame data. Do not infer
orientation from a victim/world axis after the authoritative producer has lost
the relevant frame.

### Authored sprites and procedural particles compose

Use authored sheets where they carry identity/readability and procedural
particles where motion/volume is the useful part. Do not require one provider to
replace the other.

### Prototype evidence precedes dependency adoption

A new particle dependency must first demonstrate an effect the current built-in
path cannot express adequately, on the pinned Bevy/platform targets. Do not add a
provider because "particles" exist as a category.

## Current open work

### ✔ Portal scene compositing — a global z cannot express portal optics (2026-09-05, LANDED)

✔ **ALL SIX ACCEPTANCE CASES JON NAMED ARE GREEN**, and both roads he ruled out
stayed ruled out: `PORTAL_WINDOW_Z` is untouched, and no actor's z is mutated —
asserted, not merely intended, by
`the_player_band_and_the_actor_band_composite_identically`, which demands
IDENTICAL output at z 11 and z 20. This repair never reads z at all.

⚠ **Open, and small:** a split-screen session. One body is near for one player
and far for the other — the two-pane problem again — and it would need per-view
pieces on per-view layers. The seam cannot express it today: `PortalViewer` is a
`Resource`, so there is exactly one eye by construction.

**Reported by Jon with a screenshot**: a far-side Perfect Cellular Automaton's
sprite draws OVER a seamless portal window it should be hidden behind.

⭐ **MEASURED, and the premise is exact.** `PORTAL_WINDOW_Z = 9.5`
(`crates/ambition_portal2d_presentation/src/lib.rs:68`); a generic actor draws at
`WORLD_Z_DUMMY + 1.0 = 11.0` and the player at `WORLD_Z_PLAYER = 20.0`
(`crates/ambition_platformer2d_core/src/config.rs:26`, `:27`). **Every actor wins
the depth test against every pane.**

⛔ **The constant's own doc names an intent it can only half-serve** — *"below
actors so a near-side actor still occludes it"*. Near-side occlusion works;
far-side covering cannot, ever. ⚠ The half that works is the visible one, which
is why this survived.

⛔ **DO NOT FIX BY RAISING THE PANE ABOVE ACTORS.** That inverts the bug: a
near-side actor would vanish behind the aperture. And do NOT mutate an actor's
single z per frame — **one body can be NEAR one pane and FAR of another in the
same frame**, and one entity z cannot say both.

✔ **BOTH WRONG FIXES ARE NOW GUARDED, IN THE DEFAULT LANE, and the guards landed
before the repair they constrain.** Raising `PORTAL_WINDOW_Z` fails
`the_portal_band_stays_at_or_below_the_shared_world_datum`, naming the inversion
and pointing at `pane_relation`; the two-pane relation test fails any
implementation that stores one ordering per actor.

⛔⛔ **AND THE FIRST VERSION OF THE BAND GUARD DID NOT RUN.** I put it in
`ambition_render` — the only crate that can see both bands — behind
`#[cfg(feature = "portal_render")]`, and that crate declares `default = []`, so
it executed only under `--run-everything`. The gated-test ledger going 447 → 449
is what caught it. **A guard that stops a two-line wrong fix is worthless if it
does not run in the plan the person making that fix will run.**

⭐ **Fixed by splitting the claim on a shared term rather than adding a feature.**
Both crates already depend on `ambition_platformer2d_core`, so portal
presentation pins `portal band <= WORLD_Z_DUMMY` and render pins
`WORLD_Z_DUMMY < actor draw z`; together the same claim, with no optional feature
on either side. ⇒ It only LOOKED cross-crate because I phrased it in terms of the
actor z rather than the datum underneath both.

✔ **DONE: the shared authority.** `compositing::pane_relation(pane, viewer,
drawable, transiting)` → `Disjoint | NearOccluder | FarCovered | Transiting`,
plus `current_z_policy_is_correct_for(relation)` so the defect is countable.
It reuses the portal domain's `pieces::front_distance` rather than a private
`.dot(normal)`. ⭐ **The two-pane poison Jon named is already in the tree** — one
body, two facing apertures, opposite relations — so the cheap wrong fix cannot
pass later. Nothing consumes it for drawing yet, deliberately: the diagnostics and
the compositor must read the SAME answer.

✔ **DONE: the Shift+F8 dump reports ordering, not only geometry.** Each selected
pane now prints its z, the viewer, the candidate count, the viewer's signed side,
and per overlapping drawable: drawn bounds, render z, its signed side, the
expected `PaneRelation`, the ACTUAL ordering, and `COMPOSITE_VIOLATION`, plus a
per-pane violation total.

⛔⛔ **AND IT NEEDED A NEW HOST SEAM, WHICH IS ITSELF A FINDING.** The presentation
crate could not SEE the bug it has: its only body seams are `PortalSceneBody`
(ONE entity, whose sprite is decomposed at the seam) and `PortalAffordanceBody`
(whoever operates the portals). An ordinary NPC behind an aperture is neither.
`PortalCompositingCandidate` widens the POPULATION, not the vocabulary.

⚠ **It carries DRAWN bounds, not `PortalBodyView`** — that seam's `size` is the
COLLISION box ("crouch / morph compaction included") and the question is which
PIXELS a pane covers. A sprite overhangs its box, and the overhang is the part of
the screenshot that punches through. Reusing the existing seam was the tempting
shortcut and would have made the report miss the finding.

⇒ **Remaining, in the order Jon asked for:**

✔ **DONE: the host publishes.** `ambition_render` tags every drawn actor sprite
behind its optional `portal_render` feature, in ENGINE coordinates, using
`Sprite::custom_size` (the DRAWN rect) and the `bevy_size_to_world` inverse added
beside its forward rather than a second y-flip. ⚠ Gated on a portal existing, so
a portal-free room does no work; a sprite with no `custom_size` is SKIPPED rather
than guessed at. ⭐ It needed no new dependency edge: the presentation crate
re-exports `PlacedPortal` (which its own public systems already query) rather than
`ambition_render -> ambition_portal2d` being added just to spell a `run_if` —
`critical_path_crates` is a live compile-ratchet finding.

⇒ **Remaining:**

1. ✔ **DONE: `RenderLayers` and the camera stack** in the dump. It needed NO host
   seam after all — cameras are entities, so the debug system queries every one
   of them (not just the portal rigs, which see only the capture cameras), prints
   the stack in `order`, and per drawable names the ACTIVE camera whose mask
   renders both the actor's layers and the pane's: `compared_by: main`, or
   `compared_by: NONE — no active camera renders both masks`, which makes a
   `depth_says` verdict meaningless rather than merely uncertain.
   ⭐ The pane's own mask is printed too. The old report told the reader to
   "compare render_layers above" while printing only the ACTOR's half, and
   `portal_window_render_layers(channel)` is now the one authority for it —
   previously spelled inline at the window spawn site, where a diagnostic copy
   could have disagreed with the renderer about the fact it exists to explain.
   ⛔⛔ **The predicate is about a CAMERA, not about the two masks intersecting
   each other, and I shipped the wrong one first.** `RenderLayers` gates which
   camera SEES an entity; it does not group depth. An actor on layer 0 and a pane
   that is not ARE depth-compared, provided one camera renders both — so
   "disjoint masks ⇒ never compared" was false, and two existing tests caught it
   before it reached anyone.
✔ **DONE: the overlay colours the relation.** `debug_portal_view_zones` draws one
outline per (pane, candidate) pair — green near-occluder, blue far-covered,
yellow transiting, RED where the classification and today's ordering disagree.
⚠ ONE OUTLINE PER PAIR, not per candidate: a body is near one aperture and far of
another in the same frame, and collapsing that to a single colour would
reintroduce, in the diagnostic, the one-answer-per-actor assumption the bug is
made of. ⚠ Nothing is drawn without a `PortalViewer` — near and far are relative
to a viewpoint.
3. ✔ **DONE: the compositor itself** (`far_side::composite_far_side_bodies`,
   flag `far_side_compositing`, default ON). A far-covered body's whole-sprite
   draw is withdrawn and the uncovered remainder is drawn as clipped quads on the
   road the transit path already uses. It needed NO new render machinery.
   ⭐ **It serves the RIGHT population.** `tag_portal_scene_bodies` bridges
   `PlayerVisual` only — its own doc says "the player's sprite" — so the
   clipped-piece machinery could only ever decompose the player, and Jon's
   screenshot is an NPC. This reads `PortalCompositingCandidate`, which is
   population-agnostic, so an ordinary actor is served without naming its kind.
   ⛔⛔ **Three defects were found AFTER the first version was pushed, all by
   asking what the code assumed rather than by a failing test:**
   - it hardcoded `transiting: false`, so a straddling body was `FarCovered` and
     gained a THIRD copy beside its two transit slices, with two systems writing
     its `Visibility`. `PaneRelation::Transiting` existed the whole time. **With
     the defect restored, 10 of 11 arms still pass** — every other arm is
     satisfied by a body that is also mid-transit, because none put a
     `PortalTransit` on anything;
   - it wrote `Inherited` onto EVERY candidate each frame, which would overrule
     every other reason a body can be hidden (death, culling, a cutscene).
     `PortalFarSideHidden` records what this system did so it reverses that and
     nothing else; and
   - it read the eye as a COMPONENT while the host publishes a `Resource`. That
     happens to work — resources live on a singleton entity — so the tests were
     green against a world shape the game never produces.
   ⚠ **And registering it would have CRASHED every default host at startup**:
   `add_plugins(Material2dPlugin<..>)` panics on a duplicate, and the transit
   pieces and the compositor both need that material with both flags ON by
   default. The adder is idempotent now — invisible to a headless test, which
   returns early before reaching the duplicate.

   ⓘ Remaining on this row: a split-screen session, which would need per-view
   pieces on per-view layers —
   one body is near for one player and far for the other, the two-pane problem
   again. The seam cannot express it today (one eye, one resource).

   The original sizing, kept because it was right: it needs NO NEW RENDER
   MACHINERY, which is the cheapest thing anyone can know before starting it.
   ⭐ **`PortalClipMaterial` already does exactly this job for transit pieces.**
   It is a `Material2d` carrying THREE world-space clip half-planes
   (`clip0/1/2`, each `(point.xy, normal.xy)`, `CLIP_PLANE_OFF` to disable), it
   clips in the FRAGMENT SHADER against final render-world positions — so it is
   exact for any anchor, trim rect, flip, roll or scale — and its own doc says the
   through slice is clipped "to the front of the exit plane **plus the exit
   aperture span**". ⇒ Clipping a sprite to a BOUNDED aperture is shipped and in
   use; what step 3 owes is the piece decomposition for the FarCovered case, not
   a new pass.
   ⛔⛔ **AND THE DECOMPOSITION IS NOT OPTIONAL — MEASURED IN THE SHADER.**
   `portal_clip.wgsl` discards where `dot(p - point, normal) < 0` for ANY active
   plane, so the material KEEPS THE INTERSECTION of up to three half-planes: a
   CONVEX region. The far-side case needs the opposite — *discard the part inside
   the aperture, keep everything else* — and the complement of a convex region is
   a UNION, which an intersection cannot express in one draw. ⇒ A far-side sprite
   needs up to FOUR clipped quads (above / below / left / right of the aperture).
   The transit path already draws a body as multiple clipped pieces, so this is
   the same shape one step wider.

   ✔ **DONE: the decomposition, as `compositing::uncovered_remainder`.** It
   returns the part of a drawable the pane does NOT hide, as up to four disjoint
   axis-aligned pieces, and is total on both ends (no overlap → one whole piece,
   full cover → none). ⭐⭐ **The covered region is never handed to the renderer,
   so there is no ordering left to get wrong** — asserted as
   `no_piece_ever_overlaps_the_cover` over a 17×17 grid of offsets rather than a
   hand-picked case. That is the (B) test instead of a guard that catches a bad z
   after the fact.

   ⛔⛔ **CORRECTION, measured 2026-09-05: "each one a single half-plane keep" was
   WRONG, and it mattered.** `clip_piece_transform` scales the quad to the WHOLE
   sprite, so a piece is not a sub-rect and every cut is a plane. The two BANDS
   cost one plane each; the two MIDDLE pieces cost THREE (one lateral, two
   bounding the band). ⇒ Worst case is exactly three — the material's budget
   exactly, with nothing spare. It is pinned by
   `no_piece_needs_more_than_the_materials_three_clip_planes`, which asserts the
   worst case IS 3 — without that equality the bound would pass just as well on a
   decomposition that never exercised the budget.
   ⚠ **This is why the bands are cut FULL-WIDTH FIRST.** Four half-planes meeting
   at the corners would need four on some piece and would not fit, besides drawing
   the corners twice. The shape was chosen to avoid double-drawing; it turns out
   to be the only shape that fits the budget, and a future re-cut is far more
   likely to remember the first reason than the second.

   ✔ **AND THE BUDGET IS ENOUGH FOR THE SHIPPED WORLDS — MEASURED, not assumed.**
   Subtracting TWO apertures from one body would exceed three planes, so the road
   is complete only if no body can be far-covered by two panes at once. All 14
   portals sit in ONE area (`sandbox:portal_lab`), 91 pairs. The closest pair is
   32px — and is NOT a counter-example: same link, opposing normals, i.e. a
   back-to-back thin-wall doorway where "far of both" is the band BETWEEN them,
   the inside of the wall slab, which `compute_cone`'s doorway clamp already
   treats specially. **The closest pair a body could really be far of BOTH is
   163.2px, against a body about 32px wide — a five-fold margin.**
   ⚠ That is a fact about CONTENT with no structural guarantee behind it, so
   `scripts/portal_pane_separation.py` PRINTS the margin rather than asserting a
   threshold: the number is the thing to watch, and authoring that halves it is
   the signal to revisit, not a build to break.
   ⚠ **The alternative — drawing a second copy of the PANE above the actor band —
   does not work without a mask**: it would cover near-side actors too, which is
   the inverted bug. That road wants a stencil, i.e. new machinery; the piece
   road wants none.
   ⛔ **AND THIS IS WHY THE CHEAP FIX IS STILL FORBIDDEN.** With ONE pane, giving
   a far-side drawable a z below `PORTAL_WINDOW_Z` would work and is a two-line
   change. It cannot generalise: a body between two facing apertures is near one
   and far of the other, and one entity z cannot say both. The two-pane test is
   already in the tree for exactly that temptation.

⚠ **Acceptance Jon specified**: far-side NPC partially clipped by the pane; the
same NPC near-side occluding it; both again for a player sprite (different z
band); a transiting body keeping its split pieces and not gaining a third copy;
a disjoint actor unchanged; and the two-pane case.

### Persistent emitter reconciliation

Add only when a real persistent semantic source needs it. The simulation/source
owns existence and parameters; presentation reconciles the provider-specific
entity. Rollback/replay may recreate presentation from current semantic state.

### Rich particle provider trigger

The prior Enoki/Hanabi comparison campaign is closed. Hanabi is not a standing
planned dependency. Reopen a provider spike only when a concrete desired effect
cannot be expressed reasonably by the authored-sprite + built-in particle path.

The acceptance question is capability, platform fit and ownership—not whether a
third-party screenshot looks more elaborate.

### Trails and afterimages

Extract a separate reusable trail/afterimage facility only when multiple current
consumers need shared lifetime/geometry semantics. Do not hide it inside the
particle taxonomy.

### Multiview

Effects are derived presentation. A local view decides visibility/culling and
quality while consuming the same semantic event/state. Never let one camera's
presentation entity become simulation authority for another view.

### Confirmed external effects

Pure visual entities may be discarded/rebuilt on rewind. Effects that cross an
external irreversible boundary must follow the netcode/confirmed-frame contract.
Do not solve this by making ordinary VFX rollback state.

## Assets and authoring

Effect identities should resolve through the same provider/catalog preparation
principles as other assets. Authored effect sheets should publish stable semantic
clip names/metadata; runtime presentation consumes the published products rather
than maintaining a second hand-written copy of sheet geometry.

## Acceptance

This program is in a healthy Engine 1.0 state when:

1. gameplay emits semantic effect facts without importing presentation backends;
2. built-in sprite/particle presentation covers common gameplay feedback;
3. quality/reference-frame/multiview policy is explicit;
4. persistent emitters, if introduced, reconcile from semantic source state;
5. richer providers remain optional and are added only for demonstrated
   expressibility needs;
6. headless/minimal consumers do not depend on particle/render packages.
