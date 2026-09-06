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

✔ **THE PREMISE, VERIFIED RATHER THAN QUOTED (2026-09-05).** The whole repair
rests on "every actor wins the depth test against every pane", and the three
numbers behind it are now cited rather than repeated from the bug report:

```text
  PORTAL_WINDOW_Z            9.5   ambition_portal2d_presentation/src/lib.rs:88
  WORLD_Z_DUMMY             10.0   ambition_platformer2d_core/src/config.rs:26
  WORLD_Z_PLAYER            20.0   ambition_platformer2d_core/src/config.rs:27
  FeatureVisualKind::Actor  11.0   ambition_render/.../primitives.rs:171 (DUMMY + 1.0)
```

⚠ **`WORLD_Z_DUMMY + 1.0` appears nowhere else in production**, which is worth
saying because it reads like a constant and is not one — it is one row of the
`FeatureVisual` z table, and I had been repeating the number without a source.
⭐ That table also answers a question the row never asked: **Pickup 14, Chest 13,
Switch 12 are all above the pane too**, and all are `FeatureVisual`, so widening
the publisher to include `PlayerVisual` makes the covered population every
feature visual plus the player — not actors alone.

⛔⛔ **A REVIEW FOUND FOUR MORE DEFECTS AFTER THIS ROW SAID LANDED (2026-09-05),
and all four were real.** Recorded because the shape repeats: every one was in a
seam the crate-level tests do not cross, and every crate-level test was green.

| # | defect | why the tests missed it |
|---|---|---|
| 1 | the publisher's query was `With<FeatureVisual>`, so a `PlayerVisual` was NEVER a candidate | the player acceptance test CONSTRUCTS its own candidate |
| 2 | bounds were `translation ± custom_size/2`, ignoring the FEET anchor and scale | no test compared two anchors at one translation |
| 3 | publication had NO ordering edge to `animate_player` or `PortalPresentationSet` | nothing ran the publisher and the compositor in one app |
| 4 | the diagnostic derived `COMPOSITE_VIOLATION` from actor z | the compositor leaves z alone, so it flagged the REPAIR as the bug |

⭐ **Defect 4 is defect (a) of the compositor itself, repeated in the instrument
built to find it** — the dump also passed a hardcoded `transiting: false`, the
identical line, in the identical shape. Twice in one feature.

✔ **All four fixed**, each poison-verified to redden exactly one arm, plus two
tests that close the layers they hid in: the publisher and compositor chained in
ONE frame, and an assembled-host check that the shipped app registers both halves
(read off a built, never-run schedule graph — a crate test cannot see a
composition that never adds a system).

✔ **PARTLY PAID: the publisher and the compositor now meet in one frame, for
BOTH halves of the population.** A far-side NPC (`FeatureVisual` — Jon's actual
screenshot) and a far-side player (`PlayerVisual` — the half that was excluded)
each get their own end-to-end arm, and each fails on its own: poisoning the query
to `PlayerVisual` alone reddens the NPC arm and leaves the player arm green, and
the reverse poison does the reverse. One body-builder serves both, because they
differ ONLY by the marker they carry, which is the fact under test.

✔ **AND A SECOND REVIEW PASS FOUND TWO MORE PRODUCTION SEAMS, both now closed
(2026-09-05).** Both were invisible to the tests that existed, and for the same
reason: those tests removed the failure from the fixture.

1. **The bridge published LAST FRAME's pose.** It read `GlobalTransform` during
   `Update`, but transform propagation runs in `PostUpdate` — so it combined this
   frame's `Sprite` with the previous frame's pose, and the compositor subtracted
   a region the body had already left. It now reads the local `Transform` that
   `sync_visuals` wrote earlier in the same run, with `Without<ChildOf>` stating
   the condition that makes local == world rather than assuming it. ⚠ Every
   earlier bridge test seeded `Transform` and `GlobalTransform` IDENTICALLY,
   which deleted precisely the propagation gap; the new one makes them disagree
   by 200 units. Two further gaps closed with it: the publisher was ordered after
   `animate_player` ALONE while the population is `FeatureVisual` OR
   `PlayerVisual` (now after `sync_visuals` and all three animators), and
   `drawn_half` folded in scale but not ROTATION, so a rolled non-square sprite
   under-reported its world extent.
2. **Two systems wrote the source body's `Visibility` with no order between
   them.** `sync_portal_body_pieces` set `Inherited` UNCONDITIONALLY at the top
   of every run and `composite_far_side_bodies` set `Hidden` for a far-covered
   body ⇒ whichever ran later won. The reported case is the handoff frame
   (far-covered on N, `PortalTransit` on N+1, the whole sprite drawn back on top
   of its own slices), but the unconditional `Inherited` also clobbered the
   far-side hide on EVERY ordinary frame — and it reaches the player, because
   `tag_portal_scene_bodies` grants `PortalSceneBody` to every `PlayerVisual`.
   ⭐ Fixed by authority, not by an ordering edge: each system now states a
   REASON (`PortalFarSideHidden`, `PortalTransitHidden`) and
   `resolve_portal_source_visibility` is the only writer. A third reason later is
   a marker, not a renegotiation. It reverses only its own hides, because
   `PlayerVisual` bodies have other legitimate visibility writers.

⇒ Poison-verified: the resolver reverted to "far-side withdrawal wins" reddens
the new two-frame handoff test AND the pre-existing transit test; reverting to
the stale global reddens the pose test; dropping the rotation term reddens the
rotation test. ⚠ These live on the `portal_render` lane (258 green), which
`ambition_render`'s `default` does not include — a default-feature run compiles
none of them.

⚠ **Still owed:** live-room behaviour with a running session — transit, disjoint
and the two-pane case on a real portal room, driven by the sim rather than by a
hand-placed candidate. Registration is asserted (a schedule-graph check on the
built app) and the seam is asserted; a ROOM is not.
⛔ **AND THE OBVIOUS SHORTCUT DOES NOT EXIST — checked 2026-09-05, so the next
attempt does not start by looking for it.** `portal_translation_camera_continuity`
looks ideal: gated on `portal_render`, headless, and it enters the real room with
`StartRoomOverride("portal_lab")`. But it composes `MinimalPlugins` + Asset /
Image / Transform / States and **no portal presentation at all** — zero mentions
of `PortalPresentationPlugin`, the render plugin, or `composite_far_side_bodies`.
It is a SIM-and-camera harness, so the compositor never runs in it.
⇒ A live-room test needs a harness that composes the render presentation
alongside a session, which is a bigger fixture than any portal test currently
builds — not a few lines added to an existing one.

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
is what caught it. ⚠ **Those digits are the ledger as the instrument read it THEN**;
the same survey reports 416 since 2026-09-06, after two corrections that are
instrument rather than tests moving (see `status.md`'s feature-gated row). The
MOVEMENT of +2 is what caught the guard, and a movement survives a recalibration
that its endpoints do not. **A guard that stops a two-line wrong fix is worthless if it
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
   ⭐ **It serves a WIDER population than the transit road could.**
   `tag_portal_scene_bodies` bridges `PlayerVisual` only — its own doc says "the
   player's sprite" — so the clipped-piece machinery could only ever decompose
   the player, and Jon's screenshot is an NPC. This reads
   `PortalCompositingCandidate` instead, which the compositor consumes without
   naming any marker.
   ⛔⛔ **THIS PARAGRAPH SAID "population-agnostic" AND THAT WAS FALSE WHEN
   WRITTEN.** The COMPOSITOR names no marker, but the PUBLISHER did: its query
   was `With<FeatureVisual>`, and the exploration player is spawned
   `PlayerVisual` — so the player was excluded at the door, which is finding 1 in
   the table below. ⇒ Agnostic on the consuming side is not agnostic end to end,
   and writing the property of one half as a property of the seam is how it went
   unnoticed for a day.
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
   163.2px.**
   ⚠ **CORRECTION: the margin I first wrote was overstated, and by picking the
   wrong dimension.** It said "against a body about 32px wide — a five-fold
   margin". 32 is Mary-O's HEIGHT; her box is 21.33 x 32.00. The quantity that
   decides whether one body can overlap two apertures is its largest extent, and
   the widest thing in play is the DEFAULT player body at 30 x 48 (diagonal ~57).
   ⇒ **The honest margin is about 3x, not 5x** — 163.2 against ~48, or ~2.9x
   against the diagonal. Still comfortable, and still the reason the three-plane
   budget suffices, but a third smaller than claimed.
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

## ⭐⭐ The tether line is NOT the blocker for a tether RECOVERY — measured 2026-09-05

Recorded here because the ask arrived as *"`TetherVisual` needs to be able to draw
to a POINT, not only to a body"*, and that is not what the type says. Left as a
receipt so the next reader does not re-derive it.

`TetherVisual { body }` names the body doing the REACHING — the line's owner, used
to dedup and to despawn when the reach ends
(`crates/ambition_render/src/rendering/tether.rs:83`). The far end is ALREADY a
point: `crates/ambition_render/src/rendering/tether.rs:61` reads `pose.grab_reach`
as a `Vec2` and hands it to
`flyline::place_wire` as `to`. ⇒ A tether that catches the STAGE is already
expressible in this crate; a `Point` variant would be a sum type whose two arms
are both a point.

⛔ **THE REAL BLOCKER IS ONE EFFECT KEY, UPSTREAM.** `grab_reach` has exactly one
producer — `crates/ambition_sim_view/src/pose_view.rs:457` — and it is
`playback.and_then(|pb| pb.live_capture_reach())`.
`crates/ambition_combat/src/moveset/mod.rs:659` returns `Some` ONLY for a live
window whose `sustain_effect.key` is
`ambition_characters::smash_capture::CAPTURE_ATTEMPT` (`"smash.capture_attempt"`).
A ledge-assist recovery rides a different key —
`crates/ambition_characters/src/smash_teleport.rs:28` `TELEPORT = "smash.teleport"`,
with `ledge_assist` a field of `TeleportParams` (`:112`). It matches nothing, so
`grab_reach` is `None` and no line is ever spawned. The recovery itself works; it
publishes no reach for anything to draw.

⭐ **The (A) shape, so the fix is not just another arm.** "Where is this body
reaching this frame" is ONE fact, and its derivation is a hand-kept match on ONE
key living in the PRESENTATION read-model. Every new reaching verb — a recovery, a
command grab, a wire — must be remembered there or it silently draws nothing.
Adding a `TELEPORT` arm beside `CAPTURE_ATTEMPT` closes one case and leaves the
next verb the same trap.
⚠ Effect keys are STRINGS, so a type cannot make this exhaustive. The honest
shapes are: the reach becomes something a sustain effect STATES, or a guard that
every key placing a world volume is classified reaching-or-not in one place.

⇒ `moveset/mod.rs`, `smash_capture.rs` and `smash_teleport.rs` are the FIGHTER
lane's surface, so the choice is theirs and nothing here was touched. Either way
`tether.rs` needs no change: it would only consume a reach somebody publishes.

⚠ **AND THE REACH FORMULA IS ALREADY WRITTEN THREE TIMES** — found while
measuring the above, recorded rather than edited. ⛔ I first published this as
TWO; the fighter lane re-derived it as THREE and they are right (checked
2026-09-05). "How far does a capture reach" is `offset.x + half_extents.x` in:

| site | expression | consumer |
| --- | --- | --- |
| `crates/ambition_platformer2d_actor_monolith/src/features/ecs/actors/update.rs:1873` | `attempt.offset.0 + attempt.half_extents.0` | the brain's `AttackCandidate.reach` (a scalar) |
| `crates/ambition_sim_view/src/pose_view.rs:458` | `(offset.x + half.x) * side` | `BodyPoseView::grab_reach` — the PLAYER road |
| `crates/ambition_sim_view/src/view_index.rs:510` | `(offset.x + half.x) * side` | `FeatureView::grab_reach` — the ACTOR road |

⭐ The two read-model rows are the same both-roads split `tether.rs` documents at
its own seam, so the formula is stated once per road plus once for the brain.
⇒ Change how a capture's reach is derived and the LINE the player reads parts
company with the distance the AI aims at, with nothing to catch it.
⚠ **AND THAT IS WHY IT IS NOT A ONE-LINER.** The fix spans `ambition_characters`
(where a `reach_x()` on `CaptureAttemptParams` would live), the monolith and
`ambition_sim_view` — and doing two of the three is a dedup that still diverges.
The sites also legitimately differ in WHICH windows they scan (live-only versus
all) and whether they want a scalar or a world point; only the x-extent is
shared, and that difference must survive any unification.

## ⛔ D-MARYO-SPRITE: the BASELINE is eliminated, and the authored branch is where to look

**2026-09-05.** Recorded because I spent an afternoon on the wrong branch and
briefly landed a "fix" for it (`03304e31e`, retracted in `3f9f0558d`).

⭐ **MEASURED, and it closes a whole line of enquiry.** Her forms author
`BodySource::SpriteAuthored` (`game/ambition_demo_mary_o/src/lib.rs:1302`), so her
body carries `SpritePosedBody` and publishes `BodyPoseView::authored_render`. In
`sync_visuals` the `authored_render` arm is taken BEFORE the
`PlayerSpriteBaseline` arm and never reads the baseline. ⇒ **No baseline value —
guessed, stale, or otherwise — reaches her on screen.** Any explanation of her
positioning that runs through `PlayerSpriteBaseline` is wrong before it starts.
⚠ The staleness itself is real (a form swap in phase `FeatureInteraction` 9
rebinds against the box `WorldPrep` 2 gave the body earlier that frame) and is
pinned by a characterization test. It is simply not observable for her, and the
bodies that DO read the baseline are the dev body-profile experiment, where a
ratio other than one is the intended mechanism.

⇒ **Where the next look belongs:** `authored_render` and `authored_offset`,
published by `sync_sprite_posed_bodies`
(`crates/ambition_character_sprites/src/posed_body.rs:171-192`). That pass owns
both her quad's SIZE and its PLACEMENT, including the `stance_shift` term that
reverses `resize_feet_planted`'s centre slide. A positioning error against her
collider is a disagreement in that term or in `geometry.sprite_offset`.

### ⚠ A dead-looking fallback in that branch, carrying a derivation its own comment calls wrong

`sync_visuals`' authored arm has an `else if` for a body with `authored_render`
but no `authored_offset`, which recomputes a feet anchor via
`feet_anchor_for_render_size`. The comment beside it states that this is **a
SECOND derivation of one fact** and that it **disagreed with the first** — for the
robot v3, ~1 px vertically and ~2.5 px horizontally.

⛔⛔ **I REASONED IT WAS UNREACHABLE AND THAT WAS WRONG — MEASURED 2026-09-05,
and the restraint is the only reason no live code was deleted.** The reasoning
was: `ActorRenderSize` and `ActorSpriteOffset` are published by the SAME loop
iteration of `sync_sprite_posed_bodies` from one `PosedBodyGeometry`, so
`render.is_some() && offset.is_none()` has no producer. That is true of THAT pass
and false of the tree.

⇒ **`spawn_actors.rs` writes `ActorRenderSize` at FOUR sites (`:807`, `:1383`,
`:1803`, `:2056`) and mentions `ActorSpriteOffset` ZERO times.** A spawned actor
therefore holds the SIZE with no PLACEMENT until `sync_sprite_posed_bodies` runs
and publishes one. For a body that also carries `SpritePosedBody`,
`BodyPoseView::authored_render` is `Some` and `authored_offset` is `None` in that
window ⇒ **the fallback fires, on the frame a posed actor spawns.** It is
load-bearing, not dead.

⭐ **THE WINDOW IS ONE FRAME WIDE AND THE PHASE ORDER PROVES IT** (measured, not
assumed — this claim had already been wrong once):
`SpritePosedBody` is inserted by `project_prepared_character_definitions` in
`PlayerInputSet::CharacterProjection`, inside phase **PlayerInput (3)**;
`sync_sprite_posed_bodies` publishes the offset in `WorldPrepSet::BeforeIntegrate`,
inside phase **WorldPrep (2)**. ⇒ On the frame the marker lands, the offset pass
has ALREADY run and will not run again until the next frame — while presentation
runs late in that same frame and reads the pair. The body is therefore
size-without-placement for exactly one frame, every time a posed character is
projected.
⚠ Conditional on that spawn having set a render size at all
(`spawn_actors.rs:806`, `if let Some(size) = render_size`), so it is every posed
actor spawned WITH one, not literally every actor.

⚠ **And that makes it a defect worth naming rather than a wart to delete.** The
fallback derives placement by a DIFFERENT rule than the pass does — the comment
beside it records the two disagreeing by ~1px vertically and ~2.5px horizontally
for the robot v3. ⇒ A posed actor is placed by one rule on its spawn frame and by
another on every frame after, so its art can shift by a pixel or two the moment
the offset lands. That is a ONE-FRAME POP at spawn, small and real.
⭐ The (A) repair is therefore NOT "delete the second authority" but "give the
spawn path the same authority": have `spawn_actors` publish the offset beside the
size (both come from sheet metadata it already reads), after which the fallback
genuinely has no producer and can go. ⛔ Not done here — it is four spawn sites in
the monolith and wants its own change with the suite behind it.

## ⛔⛔ PORTAL COMPOSITION SEES THE BASE DRAWABLE, NOT THE BODY'S OTHER REPRESENTATIONS

**Raised by a GPT review 2026-09-05 and VERIFIED here rather than taken on
trust.** The publisher's population is `FeatureVisual OR PlayerVisual`, and a
character's visible pixels are not confined to that entity.

⭐ **MEASURED — the morph ball is the clearest case.**
`crates/ambition_render/src/rendering/morph_ball.rs:123` spawns a SEPARATE ROOT
entity carrying `Sprite`, `Transform`, `Visibility` and `MorphBallVisual`, and
**no `PlayerVisual` or `FeatureVisual`** (checked: zero occurrences in the spawn).
While morphed the base `PlayerVisual` is hidden and this is what draws the player
⇒ **the representation actually on screen is not a portal candidate at all.**
⚠ And it sits at `WORLD_Z_PLAYER + 0.05` = 20.05, far above the portal band
(pinned at or below `WORLD_Z_DUMMY` = 10), so it draws OVER a pane rather than
being clipped by one.

⚠ **The hit-flash silhouette is the same shape with an extra ordering twist.**
`attach_hit_flash_overlays` roots its own mesh and `sync_hit_flash_overlays`
suppresses the shader when the SOURCE is hidden — but that sync runs BEFORE
portal presentation. On a frame where an ordinary presentation writer has the
source visible, the flash enables itself; portal presentation then hides/clips
the base sprite, and the separately rooted overlay is not a candidate. A
far-side character can show its whole flash/parry/blink silhouette over the
window while its base art is correctly clipped.

⇒ **NOT a list of portal-specific patches.** `visuals.rs` already documents the
same gap for the held gun in TRANSIT, so this is the third instance of one
missing concept: **which drawables belong to which logical body.** The repair is
that seam — a body→drawable ownership relation the compositor can ask — and
adding flash, ball, gun and shield as special cases would make the next one
harder, not easier.
✔ **THE SEAM LANDED 2026-09-06, and the morph-ball half is closed.**
`PresentationOf(Entity)` (`shared_tangle`'s `lifecycle::markers`) is how a
drawable says whose body it draws. It lives there BY DEPENDENCY DIRECTION rather
than preference: `ambition_render` depends on `ambition_portal2d_presentation`,
so a component defined in the render crate is invisible to the compositor that
must read it — the same reason `PlayerVisual` sits there.

- All five body-drawing families carry it: flyline, trapdoor, tether, hit-flash
  overlay, slash. Each keeps its own field (a tether still needs its body to
  place a line); what is added is the ANSWER to *"whose body"*, askable by a
  consumer that knows nothing about tethers.
- The morph ball is stamped in its SYNC rather than at spawn, because the ball is
  built before any body exists — and idempotently, because that runs every frame.
- `publish_portal_compositing_candidates` now admits anything carrying the
  component, so **a morphed player is composited** where before only its hidden
  base sprite was visible to the publisher. ⭐ Not a special case for balls: the
  next drawable that declares an owner is composited without touching that file.
- A census guard (`scripts/check_body_drawables_declare_their_owner.py`) fails
  when a component starts naming a body and is not classified — the fallback
  where a type cannot say it, because a Bevy spawn is a tuple.

✔ **THE HIT FLASH IS CLOSED TOO, and it did not need the publisher.** The
overlay is a `Mesh2d`, not a `Sprite` with a `custom_size`, so the sprite-shaped
publisher could never describe it as a candidate — but it never had to.
`overlay_look` ALREADY blanks the silhouette when its source is `Hidden`; the
defect was that `sync_hit_flash_overlays` runs BEFORE portal presentation, so on
the frame the portal hides a far-side body the overlay had been computed from a
VISIBLE source.

⛔ **Ordering cannot fix that**: the publisher runs `.after(animate_feature_sprites)`,
which is itself after the hit-flash mirror in the render chain, so moving the
mirror later is a cycle. ⇒ `resolve_portal_source_visibility` settles the
DEPENDANTS in the same pass instead — a drawable carrying `PresentationOf` is
hidden while the body it names is portal-hidden. Neither side learns about the
other: the overlay does not know what a portal is, and the portal does not know
what a hit flash is.
⚠ Bevy refused the first version (B0001): excluding two of the three reason
markers left an entity both `&mut Visibility` queries could match, panicking seven
tests. All three are excluded now.

⛔ **AND MY NOTE ABOUT THE HELD GUN WAS FALSE — I named the wrong case.** The gun
in TRANSIT is not uncovered; it has been handled since before this work.
`sync_portal_mode_indicator` branches on `Option<&PortalTransit>` and draws one
clip-material quad PER CHART, and
`transiting_carrier_gun_decomposes_into_two_clipped_charts` pins it (verified
green 2026-09-06, and the docstring records why: the single sprite visibly
SNAPPED by the pair separation at the centroid crossing while the body slices
stayed continuous).

⚠ **The real gap is FAR SIDE, and it is narrower.** MEASURED from the system's
own signature: `sync_portal_mode_indicator` takes
`(&PortalBodyView, &PortalGun, Option<&PortalTransit>)` with
`With<PortalAffordanceBody>` — no `Visibility`, no hide marker. ⇒ the gun's draw
is independent of the carrier's visibility for EVERY reason except transit, which
it charts itself. So a far-side-composited carrier draws clipped body pieces AND
a whole gun at the authoritative pose.

⚠ **Reachability is a CONJUNCTION and nobody has hit it**: far-side compositing,
plus a carrier that is not the viewer (the affordance body is tagged from
`ControlledSubject`, so this is the POSSESSION case — see the `code_smells.md`
entry where the gun drew on the home avatar), plus a held portal gun. That is why
it is a row and not a fix.
⛔ **And the `PresentationOf` seam does NOT close it as-is**: the gun has no
ordering edge to the resolver, and it despawns/respawns its copies every frame, so
a stamp applied after the resolver ran settles nothing. The honest options are an
`.after(resolve_portal_source_visibility)` edge plus reading the carrier's SETTLED
visibility (one authority, the gun re-derives nothing), or charting the gun for a
far-side hide the way it already charts a transit. Those differ in what a far-side
character LOOKS like — gunless, or holding a clipped gun — which is a call, not a
wiring detail.

⛔ **A COUNT I PUBLISHED HERE WAS WRONG AND IS CORRECTED**: I first measured six
body-naming components, counting `SlashVisual { owner }` twice. One of those two
is a FUNCTION PARAMETER (`fn spawn_one(.., owner: Entity, ..)`). Five components
draw a body; the sixth name, `PortalCaptureParallaxLayerVisual`, is a portal rig's
parallax layer and is excluded after reading it.

## ◐ A NON-SPRITE portal dependant still takes a WHOLE-DRAWABLE hide — the deeper half of the 2026-09-06 HIGH (`82af28e25` closed the rest)

A GPT review found `resolve_portal_source_visibility`'s dependant pass forcing
`Visibility::Hidden` onto every `PresentationOf(body)` drawable with no claim and
no release. ✔ **Closed**: `PortalDependantHidden` is the claim and the release
RESTORES (the opposite of the body branch, and for the reason that branch gives —
a body always has a per-frame visibility owner, a dependant may have none). ✔ Also
closed: an unparented **sprite** dependant now skips the fallback entirely, because
`publish_portal_compositing_candidates` already admits `PresentationOf` drawables
and classifies them from their own bounds.

⛔ **WHAT REMAINS: the hit-flash `Mesh2d` is not a compositing candidate**, so when
its owner is far-side it takes a whole-drawable hide rather than losing only the
pixels behind the pane. The latch is gone; the clipping is still scalar.

⭐ **THE ROUTE IS ALREADY DESIGNED, and by this crate's own admission.**
`clip_material.rs`'s docstring says its "quad + atlas-frame UV mapping follows the
hit-flash overlay pattern (`ambition_render::rendering::hit_flash`), the
established way to draw 'the sprite's current frame' as a mesh." ⇒ The portal clip
material was MODELLED ON the hit flash. Both are `Material2d` quads over the
sprite's current atlas frame, so giving `HitFlashMaterial` the same world-space
clip half-plane uniform is the symmetric change, not a new mechanism.

⚠ **WHAT MAKES IT A REAL PIECE OF WORK RATHER THAN A UNIFORM**: the pane plane has
to reach the hit-flash overlay (it is published for candidates, not for arbitrary
meshes), and a body straddling a pair needs TWO flash pieces the way a transiting
body needs two sprite slices — so the overlay stops being "one sibling mesh per
character sprite", which is the sentence its module opens with.

⇒ Acceptance for whoever takes it, from the review: an active far-side hit flash
loses only its pane-overlapping pixels; the actor returns near-side and flashes
again with no manual visibility repair (**already guarded** —
`a_drawable_that_names_a_hidden_body_is_hidden_with_it`); a long flyline disjoint
from the pane stays visible while its body overlaps (**already guarded** —
`a_sprite_dependant_disjoint_from_the_pane_is_not_hidden_by_its_owner`); morph ball
and the two-visible-portals poison stay correct.

## ◐ THREE holds have no published presentation fact — not five (measured 2026-09-06)

The fighter lane raised it as *"a slept fighter stands identically to one in
shieldstun, landing lag, a recoil lock or a guard break, so in a 1v1 neither player
can tell which of five causes is holding them"*. ⇒ **The concern is real and the
count is not**, which changes what the fix has to cover.

✔ **TWO OF THE FIVE ARE ALREADY DISTINGUISHED, by published facts with presentation
readers:**
* **shieldstun** — the sim publishes its timer and
  `crates/ambition_render/src/rendering/bubble_shield.rs` normalises it into a
  flare. That file's own comment states the split: the SIM publishes the timer, the
  RENDERER owns how long the flare is spent.
* **guard break** — `crates/ambition_character_sprites/src/anim/mod.rs:247` carries
  a `GuardBreakBeat`.
* **launch/hitstun** is a third: `pose_view` publishes `LaunchedBodyFact` whenever
  `tumbling` or `hitstun_timer > 0`, and it deliberately ORs the two so a consumer
  does not drop the row the instant a launched body stops tumbling.

⛔ **WHAT THE POSE CANNOT SEE: `sleep_timer`, `landing_lag_timer` and
`recoil_lock_timer`** — and each is unreachable for a DIFFERENT reason, which is
what a fix has to respect:
* `sleep_timer` — **zero** hits across `sim_view`, `render` and
  `character_sprites`. Genuinely unpublished.
* `landing_lag_timer` — ⚠ **IS published**, as `landing_lag_s`
  (`crates/ambition_sim_view/src/combat_geometry_view.rs:240`). But that read-model
  opens by saying it "answers the two questions a combat debugger needs", and it
  feeds debug overlays rather than the pose. So the plumbing exists and the POSE is
  what is missing — a weaker and more useful claim than "nothing publishes it",
  which the next reader would falsify in one grep.
* `recoil_lock_timer` — reaches presentation only as
  `LaunchedBodyFact::launch_beat_secs`, a field OF the launch row, so it cannot
  answer for a body that is not launched.

⛔⛔ **CORRECTION, SAME DAY, BEFORE ANYONE ACTED ON IT: I FIRST WROTE THAT THE FIX
WAS A NEW `HoldReason` PUBLISHED BY THE SIM. That is wrong, and it would have built
a parallel mechanism beside one that already exists.**

⇒ **THE RANKING ALREADY HAS ONE AUTHORITY: `pick_body_anim(&BodyAnimView)`**
(`crates/ambition_character_sprites/src/anim/mod.rs:357`). It is a single ordered
ladder — `dead` → `held` → `knocked_down` → `getting_up` → `hit | tumbling |
guard_broken` → `dodge_roll` → `air_dodge` — and its comments already argue about
RANK for exactly the reason a new reason type would have had to re-argue ("the floor
game outranks the hit flash… reading `hit` first would draw the struck pose for the
whole prone beat"). The renderer draws `pose.anim` (`animation.rs:183`); it does no
ranking and must not start.

⇒ **So the sleep pose is ONE FLAG ON `BodyAnimView` PLUS ONE ARM AT THE RIGHT RANK**,
not a new published fact. ⚠ And the rank is the whole design question, not a
detail: sleep must outrank `hit` (a slept body is still inside hitstun, the same
argument the knockdown arm won) while losing to `dead` and `held`.

⚠ **THERE IS ALSO A SECOND LEVEL TO USE RATHER THAN DUPLICATE.** The `guard_broken`
arm returns `Hit` and says why: *"`body_state_clip` asks the sheet for `dizzy`
first, and a fighter sheet has that row. This arm is what a sheet without one lands
on."* ⇒ A sleep gets the same treatment — a sheet-specific clip when the sheet has
one, and a ladder arm as the floor — so a character without sleep art degrades
instead of drawing nothing.

⭐ **WHY I NEARLY GOT IT WRONG IS THE REUSABLE PART**: I measured which FACTS were
published, found none for sleep, and concluded a fact was missing. The fact was not
missing — the INPUT to an existing ranking was. Asking "what is published?" and
asking "who already decides this?" are different questions, and only the second
finds a ladder.
