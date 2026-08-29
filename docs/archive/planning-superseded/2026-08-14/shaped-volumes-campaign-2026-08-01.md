> ⚠ **ARCHIVED 2026-08-14 — EVIDENCE, NOT AUTHORITY.** This campaign is complete
> (zero open markers, untouched since 2026-08-02) and was moved out of
> `docs/planning/` under that directory's own rule: *completed campaigns and
> migration evidence belong in `docs/archive`*. ⛔ **do not reconstruct a deleted
> representation because this file names it**, and do not treat any instruction
> here as live tasking. Live direction is `docs/planning/queue-72h-2026-08-08.md`.

# Shaped volumes — one authored swing, every consumer reads its real shape

**Armed:** 2026-08-01, on Jon's instruction. Surveyed against source, not docs.

**The objective, in Jon's words:**

> I want it to be easy for an artist to know what hitpoly they are writing a
> sprite for. Conversely I want a really good effect to have an artist be able
> to draw a hitpoly to match it.

Everything below serves that sentence. The correctness work (fronts A and B) is
not a separate campaign — it is the substrate that has to be true before an
artist can trust either direction.

---

## The thesis

A swing is authored **once**, as a shape descriptor. The hit polygon, the
drawn effect, and the runtime cue are all *derived* from that one object, so
they cannot disagree. Today there is no such object: the poly, the box, and the
art are three independent authorings of the same swing, and the runtime
discards the shape before presentation ever sees it.

---

## What is true today, with evidence

### 1. The manifest carries two independently-authored geometries per attack

`tools/ambition_sprite2d_renderer/.../targets/characters/robot_side.py:214`:

```python
"attack_side": shaped(
    box (cx + w*0.26, h*0.12, w*0.60, h*0.72),
    cone(cx - w*0.06, body_cy, 1.0, 0.0, w*1.34, h*0.22, h*0.62),
),
```

Both reproduce the shipped manifest exactly (`w = h = 128`, `cx = 64`,
`body_cy = 60.16`). Nothing derives one from the other, and nothing checks even
a containment relation:

| | near x | far x | y span |
|---|---|---|---|
| `bbox` | 97.3 | 174.1 | 15.4 … 107.5 (92 tall) |
| `poly` | 56.3 | 258.7 | −19.2 … 139.5 (159 tall) |

The poly begins *behind* the box's near edge (at the body, where the swing
originates), reaches ~1.8× further out, and is ~1.7× taller. That is by
construction — `cone()`'s docstring says the hull is deliberately unclamped —
but nothing makes it a *deliberate* relationship rather than an accident.

Minor corroboration of drift: `box()` also emits `active_frames: [0, 1, 2]`,
which `core/manifest_ron.py:50-69` silently drops. It has never reached a
manifest.

### 2. One authored attack has two geometries, depending on who reads it

| consumer | reads | file |
|---|---|---|
| actor / player melee | `poly` first, `bbox` as fallback | `character_sprites/attack_hitbox.rs:100` |
| **boss** | `parts` / `bbox` / `frames` only — never `poly` | `boss_encounter/attack_geometry/frame.rs:128` |

The boss path's signature is `-> Vec<ae::Aabb>`; it structurally cannot express
a hull. So a boss swinging the identical manifest row gets a volume ~45%
shorter-reaching and ~40% shorter than the player's, silently. This is the same
failure shape as "the two fighters in a Smash match are built by two different
paths".

### 3. The VFX seam destroys the shape in three lossy steps

1. **Hull → AABB** — `moveset/mod.rs:682`: `let b = hb.world_volume(kin.pos).bounds();`
2. **AABB → one scalar** — `combat/src/util.rs:216`:
   `((half_size * 2.0).max_element() * 2.0).max(24.0)`. Aspect ratio is discarded.
3. **Scalar → square** — `slash_visuals.rs:202`: `custom_size = Some(BVec2::splat(size))`.

The message type is the whole contract:
`VfxMessage::Slash { center: Vec2, size: f32, kind, pose, dir: Vec2 }`.
One point, one number, one direction.

For `attack_side` at a 30×48 body (scale ≈ 0.506): the hull's bounds are
≈ 102 × 80 world units, so `size` = 204.8, and a **205 × 205 square** is drawn
centered on the bounds centroid. The hull — a cone 98 long, 28 tall at the body
flaring to 80 at the tip — is about **13% of the drawn quad's area**. The
observed "the vfx has low overlap with the hit polygon" is arithmetic, not
taste.

§7.2's stated invariant ("one box drives damage AND presentation, so they can
never point different ways") is real but covers **direction only**. Extent and
shape were never carried.

Under non-screen-down gravity `.bounds()` of a rotated hull inflates further,
so the mismatch is worse sideways.

### 4. The art and the poly live in different worlds

| | hit poly | slash art |
|---|---|---|
| where | `targets/characters/robot_side.py::attack_hitboxes` | `targets/props/robot_slash.py` |
| frame | 128×128, anchored at `feet_pixel` | 160×160, centered, no anchor |
| scope | per character, per animation | **one sheet for the entire game** |

`SLASH_SHEET = "robot_slash"` is a hardcoded const at `slash_visuals.rs:30`.
Four rows (`side`/`up`/`down`/`poke`) serve ~8 authored attack polys. There is
no channel by which the art author could know which poly they are drawing for,
because they are drawing for all of them at once.

`robot_slash.py`'s own docstring says "The game sizes this effect to the
resolved melee hitbox" — true only in the `max_element × 2` sense above. That
sentence is why an agent asked to fix the fit would believe the fit was already
handled.

Contrast that makes the gap sharp: the protagonist's slash **sound** already
retargets per character (`apply_player_robot_slash_sfx` — dry air swing,
material selector, own rebound cue), while its slash **art** cannot. It is not
the robot's blade; it is the engine's blade.

### 5. Hurtboxes are rect-only, and the original boss's are wrong

`strike_reaches_victim` (`hitbox/mod.rs:85`) tests
`world_volume.intersects_aabb(*part)` — every published hurtbox part is an
`Aabb`. `AnimationBox.poly` exists (`ambition_sprite_sheet/src/lib.rs:278`) but
no hurtbox authoring or consumer uses it.

Multi-*rect* hurtboxes already exist and are already wrong on the original boss.
`assets/sprites_0_5x/boss_spritesheet.ron` (clockwork_warden / Gradient
Sentinel, `sprite_target: "boss"`):

- `body_pixel_bbox` spans x **8 … 114**
- `floor_slam` / `dash_echo` hurtbox parts: `head` x 46…82, `body` x 42…86

The published silhouette covers roughly the middle 40% of the visible body.
Several animations (`death`) fall back to a single coarse bbox. Jon's call:
this boss is the proving ground — its hurtboxes need redoing anyway, so redo
them **as parts with polys**, once.

### 6. Two orphans found on the way

- `PLAYER_ATTACK_HITBOX_SCALE = 1.3` (`attack_hitbox.rs:185`) — Jon's 2026-07-12
  blind fix for making dair/up-tilt easier to pogo — lives on the
  `sprite_character_id == None` arm. Both live callers pass `Some(id)`
  (`moveset/mod.rs:584`, `ecs/attack.rs:300`), so it has been inert since the
  lookup became character-id-keyed. The module doc at `attack_hitbox.rs:10`
  still claims the player path uses `player_placeholder_render_size`; that arm
  is unreachable. Decide explicitly whether to restore the 1.3 on the live path
  or delete it — do not leave it as decoration.
- The same divergence means the hitbox is derived at `sprite_render_size`
  (×1.0) while the player sprite is *drawn* at `player_placeholder_render_size`
  (×1.16, `ambition_render/.../actors/mod.rs:117,759`) — the box is built at
  ~86% of the drawn sprite's scale.

---

## Decisions taken (Jon, 2026-08-01)

1. **Unify on `CombatVolume`; keep `Aabb` as a variant.** The AABB fast path is
   already *inside* the type — `intersects` (`combat_volume.rs:130`) is
   bounds-reject → box-vs-box exact → Parry. Unifying the API costs nothing.
   What costs is making everything *actually* a hull: `convex_shape`
   (`:206`) allocates a `Vec<Vector>` and calls `ConvexPolygon::from_convex_hull`
   **per test, per side, per tick**, on points that are already a convex hull.
   So: unify the type, never force a rect authoring into a hull.
2. **Hurtboxes go multi-part, in this campaign.** A single convex hull cannot
   represent disjoint pieces — one hull over head + torso + outstretched arm
   fills the armpit and every gap, which is *less* accurate than today's
   multi-rect. Per part it is. Clockwork Warden is the proving ground.
3. **Attack effects are per character**, with several characters allowed to
   reference the same effect by id.
4. **The runtime cue carries the descriptor, not a hull and not a rect.**
   `{ origin, dir, length, near_half, far_half }` — five floats, `Copy`, same
   message size as an oriented rect and strictly more information. The renderer
   builds today's quad from it with no new render tech; a later mesh path builds
   a conforming cone from the same five numbers with no message change. The
   oriented rect is the degenerate case (`near_half == far_half`).

The descriptor is the point. `cone(ox, oy, dx, dy, length, near_w, far_w, tip)`
at `robot_side.py:157` already *is* these numbers. Promoting it to a
first-class authored object makes one thing flow
**authoring → hit poly → runtime cue → drawn art**, which is exactly Jon's two
directions collapsing into one edit.

---

## The four fronts

Ordered by dependency. C and D are the visible slice and should land together;
A and B are the substrate and can proceed in parallel with them.

### Front C — the runtime cue carries the shape

- Replace `VfxMessage::Slash { size: f32 }` with the descriptor
  `{ origin, dir, length, near_half, far_half }` (plus existing `kind`, `pose`).
- `emit_melee_slash` derives the descriptor from the resolved `CombatVolume`
  instead of `.bounds()` + `max_element`. For a `Convex` hull: near edge
  midpoint → tip along the swing axis; near/far half-heights perpendicular.
- `spawn_one` builds an oriented quad from the descriptor rather than
  `splat(size)`. Rotation continues to come from the swing axis.
- Delete `slash_effect_size`; its `SLASH_EFFECT_SCALE = 2.0` becomes an explicit
  presentation margin on the descriptor, not a shape-destroying multiplier.

**Done when:** the drawn quad's footprint and the hit poly's footprint agree to
within the authored margin, verified by a capture with `show_combat_preview`
on — the box you see and the art you see are the same swing.

### Front D — the swing descriptor as the shared authoring object

- Promote `cone` / `poke` / `ring` from local closures in `attack_hitboxes` to a
  named, exported descriptor type in the generator.
- Descriptor → hit poly (what it already does).
- Descriptor → slash art: the crescent's inner/outer arc, length and flare drawn
  from the *same* numbers, per character.
- Per-character slash sheets, resolved by id the way the SFX family already is,
  with sharing by reference so several characters can name one effect. Retire
  the `SLASH_SHEET` const.
- **The preview.** `authoring/frame_debug.py:79` already overlays
  `attack_hitboxes` onto character frames. What does not exist is a composite
  that draws **character frame + hit poly + slash art at the size and placement
  the game will use**. That artifact is what makes "know what you are drawing
  for" true. Both halves already render; this is compositing, not new art code.

**Done when:** an artist can run one command, see the poly and the effect
superimposed at game scale, edit five numbers, and see both move together.

**Open question — `ring`.** The aerial-neutral spin (`ring()`, a hexagonal hull
around the body) does not fit `{origin, dir, length, near_half, far_half}`. It
needs either its own descriptor variant or a radial degenerate form
(`length = 0`, `near_half = far_half = radius`). Decide before implementing
front C's message type, since it fixes the enum shape.

### Front A — the boss path reads shaped volumes

- `attack_geometry` returns `Vec<CombatVolume>` instead of `Vec<Aabb>`;
  `active_attack_volumes`, damage resolution, and the debug overlay follow.
- Then **delete `bbox` from attack rows in the generator**. With one shaped
  consumer there is no second authoring to drift.
- Interim safety if A lands after D: emit `bbox = bounds(poly)` from the
  generator so the box is at least a true fallback that *contains* the real
  volume, instead of a differently-shaped rectangle.

### Front B — hurtboxes go shaped and multi-part

- `DamageableVolumes` carries `Vec<CombatVolume>` instead of `Vec<Aabb>`.
- `strike_reaches_victim` uses `intersects` rather than `intersects_aabb`.
- Author the Gradient Sentinel's hurtboxes as parts-with-polys and fix the
  coverage gap in §5 while doing it.
- Simple bodies keep authoring one part; one hull is already strictly better
  than one rect for a humanoid. Allow N, author 1.

**Performance notes to apply when B lands** (not before — today's volume counts
do not justify them):

- `ConvexPolygon::from_convex_polyline` instead of `from_convex_hull`; the
  points are already hulls.
- Cache the Parry shape on the `Hitbox` rather than rebuilding per test.
  Hitboxes × bodies × parts tests per tick, each building two hulls, is pure
  repeated work on static geometry.

---

## Non-goals

- Runtime hull-clipped or mesh-based VFX. The descriptor keeps that door open;
  this campaign does not walk through it.
- Hurtbox polys for every character. Multi-part shaped hurtboxes must be
  *possible* and must be right on the proving-ground boss; a rect stays the
  honest authoring for a body that is a rect.
- Changing what any attack does. This is geometry and authoring plumbing —
  damage, knockback, and timing are untouched. Any felt combat change is a
  regression, not a result.

---

## How we will know it worked

1. The generator preview shows poly and effect superimposed, at game scale, for
   the protagonist's `attack_side`.
2. Editing the descriptor's `far_half` visibly moves *both* in the next preview.
3. The Gradient Sentinel's published hurtbox covers its visible body.
4. A boss and the player swinging the same authored row produce the same volume.
5. No attack row in any manifest carries both a `poly` and an independently
   authored `bbox`.

---

# Fronts E and F — attack VFX ownership, and a swing that travels

**Added 2026-08-02**, after the first slash landed and Jon looked at it. Fronts
A–D above are about *shape*; these two are about *whose* effect it is and
*where* it lives while it plays.

## Front E — a character either authors its attack VFX, or gets the red polygon

Two halves of one rule.

**E1 — the sheet is per character.** `SLASH_SHEET = "robot_slash"` is a `const`
at `slash_visuals.rs:30`: one sheet, four rows, every body in the game. The
protagonist's blade is currently the engine's blade, and the geometry fix made
that worse rather than better — the art's envelope is now sampled off *v3's*
polygon (`robot_slash.py::_STATIONS`), so any other character swinging it wears
a silhouette cut for someone else's hitbox.

The precedent already exists one field over: `apply_player_robot_slash_sfx`
retargets the protagonist's slash **sound** per character. Sound resolves per
character; art cannot. Same seam, same shape of fix — a catalog field naming the
sheet, resolved through the presentation-source machinery rather than a const,
with several characters free to name the same sheet.

**E2 — unauthored means VISIBLE, not silent.** Jon's call, and the better half
of the idea: a character with no authored attack VFX should draw *a translucent
red polygon exactly over its hit volume*. Not a placeholder swoosh — the volume
itself, so an unauthored attack is legible in play instead of invisible or
wearing a stranger's crescent.

The cheap route is already three-quarters built. `Hitbox` entities exist for
exactly the active window, carry their `shape`, and the debug overlay already
walks them (`gizmos.rs:747`, `draw_hitbox_volume`). A product-facing pass over
the same query, filtered to owners with no authored sheet, is the whole feature
— **no message change at all**, and it cannot drift from the hitbox because it
*is* the hitbox.

That also settles a question front C deliberately left open. I argued then for
shipping the descriptor rather than the hull, because a quad is all a sprite
needs. An exact-fit fallback needs the exact shape — but it can read it off the
live volume instead of the cue, so the descriptor stays the right payload and
the fallback gets the truth. Both, without widening the message.

**Done when:** the protagonist swings its own sheet; a character with none draws
a red volume that matches the debug overlay's outline exactly; and no character
silently borrows another's art.

## Front F — the swing travels with the body

A melee effect is attached to a person. Ours is not: `spawn_one` builds a
world-positioned entity with a fixed `Transform` and `animate_slash` only
advances its frames. The hitbox does the opposite — `HitboxAnchor::FollowOwner`
re-resolves from the owner's position every tick.

So during the 100 ms the swing is live, a moving attacker's damage box tracks
the body and the drawn blade stays where the body *was*. Attacking while running
is the common case, which makes this a per-swing drift, not an edge case.

**The design is the anchor rule the hitbox already uses**, applied to the
presentation entity: the cue carries the owner and a BODY-LOCAL shape, and the
slash's transform is re-resolved each render frame from the owner's pose. One
anchoring rule, two clocks.

Three things that will bite, named now:

- **The clock.** The visual must sample the owner's *presented* pose
  (`PresentedPose`), not its sim pose, or the blade shudders against a body that
  looks perfectly stable — the same failure the debug overlay's box already hit
  and fixed by sampling `draw_pos`.
- **Facing and gravity mid-swing.** The hitbox commits its aim at the Active
  edge and rotates through the owner's frame. The visual must resolve identically
  or the two disagree exactly when a player turns during a swing.
- **Ownership at death.** A body that despawns mid-swing must not strand a
  visual anchored to a dead entity — the same class of bug
  `retire_orphaned_strike_volumes` exists for on the damage side.

**Done when:** a capture of a running attack shows the blade on the body, and
walking backward through a swing does not slide the art off the volume.

## Sequencing

E2 first: it is small, it is self-contained, and until it exists every character
without art is either invisible or lying. E1 next, which is what lets the
protagonist's tuned art stop being everyone's. F last — it is the only one that
changes a message and a spawn lifetime, and it wants E's ownership settled so
there is one path to fix rather than two.

---

# What is NOT done — status as of 2026-08-02

Written at the end of the campaign's first long session, so the next person does
not have to reconstruct it from commit archaeology. **Everything under "Open"
below is genuinely unfinished, not tidying.**

## Landed on `main`

Fronts A, B, C, D, F. Shaped volumes read by every consumer; the cue carries a
`SwingShape` instead of a scalar; the slash is generated from a shared envelope,
matches the polygon's curve, is trimmed to the damage window, and travels with
the body swinging it. Plus `docs/recipes/headless-room-verification.md`'s
picture-making section, `scripts/mirror_assets_for_worktree.py`, and
`scripts/regen/sprites.sh` taking repeated `--target` (it used to silently render only
the last one).

## Landed on branch `vfx-ownership`, NOT merged

Front E — a character names its own `attack_vfx` or draws its live hit volume as
a translucent red mesh — plus `NamedPixelRect::poly` (a shape per part), the
Gradient Sentinel's shaped silhouette, and the `SwingDescriptor`.

**⛔ Do not merge `main` into this branch. Jon's call, 2026-08-02: `main` is not
stable right now.** The branch is being held stable on its own base so that the
merge, when it happens, is a merge and not a rescue — and it goes in the other
direction, branch → `main`, once `main` settles.

A merge of `main` was made on 2026-08-02 (`f6b7b617a`, bringing `88f442ada` and
`ea17a4839`) and has been undone. The branch is now seven of its own commits
replayed onto `015c1cbe6` (front F), and `main` is no longer an ancestor of it
past that point. The pre-rewrite tip `f6e24962b` is kept at
`backup/vfx-ownership-2026-08-02`; the diff between it and the current tip is
exactly `main`'s eight files backed out, nothing of the branch's own.

⚠ One thing the rewrite deliberately did NOT undo: the renderer **submodule**
pointer, `4dffae8`. `tools/ambition_sprite2d_renderer` is its own repository with
its own linear history, and the branch's renderer work (the boss silhouette, the
`SwingDescriptor`) is authored on top of `f0270a0`, front D's preview. That is
not "`main` merged into the branch" — it is one repo's history, and rewriting it
to unpick a superproject mistake would be the actual damage.

Verified on the branch after the rewrite, cold target dir:
`cargo check --workspace --all-targets` — 0 errors, 0 warnings. Tests for the
five crates the branch touches (`ambition_geometry`, `ambition_sprite_sheet`,
`ambition_characters`, `ambition_render`,
`ambition_platformer2d_actor_monolith`) — 1774 passed, 0 failed. That is a
COMPILE-AND-UNIT result and nothing more: open 4 below still stands, because
none of this branch's visible behaviour has been looked at.

## ⛔ Open 1 — two containment harnesses disagree, so no percentage is trusted

**Read this before quoting any number from this campaign, including the ones in
its own commit messages.**

Two instruments measure "what fraction of the drawn effect lands outside its hit
polygon". The scratchpad one reads the PUBLISHED sheets and manifest; the
committed `swing_preview` reads the GENERATOR. They disagree by 11–13 points on
the same commit: checking out `e185a98`, whose message reports 0.00% for every
attack, and running the preview against it gives 12.97%.

Two candidates, neither eliminated: they place the attacker differently (the
manifest's `feet_pixel` versus the rig's `center_x`/`ground_y`, about 2px apart),
and one samples a published frame while the other samples `_draw_frame` output.

Reconciling them is the highest-value next task in the campaign, because every
other open item is judged by this measurement.

## ⛔ Open 2 — the swing's axis is the attacker's, so a raised swing tilts

`swing_shape` takes its direction from `attacker → volume centroid`. A swing
authored above the body — a slash across the chest rather than the navel, which
the drawn character wants, being half again its collision box with its feet at
the bottom — therefore tilts the drawn quad up to meet it. Measured at 8 degrees
for a rise of 0.10 body-heights. `SLASH_RISE` is 0 because of this, and Jon has
reported the swing reading low at that value.

⚠ Inferring the axis from the volume does NOT work, and three ways were tried
and reverted (see the doc comment on `swing_shape`). Nearest-to-farthest puts
both ends on corners arbitrarily; slice-averaging recovers the midpoints only if
the provisional axis is already level, which is what the tilt prevents.

The fix is to stop inferring: `cone()` and `half_disc()` are authored with a
cardinal direction and the volume drops it. Carry the swing direction on the
`Hitbox` beside its shape. That is a change to the strike-spawn path.

## ⛔ Open 3 — the art's constants are not derived from the descriptor

`SwingDescriptor` is the polygon's single source. The effect's frame constants
(`REACH`, `PEAK_HALF`, the end insets) are still hand-tuned numbers in
`robot_slash.py`. Every derivation attempted measured worse than what it
replaced — against the instrument in Open 1, which is why this is blocked behind
it rather than merely unfinished.

Until it lands, "several characters share an effect" still means sharing the
PIXELS, and a second character naming this sheet wears a silhouette cut for the
protagonist's polygon.

## ⛔ Open 4 — front E has never been seen

The red-volume path compiles and has no runtime test. Nothing has looked at it,
and it needs a body with no `attack_vfx` swinging — which today is every
character except `player_robot_v3`, so any enemy melee should show it.

The Gradient Sentinel's shaped hurtbox is in the same position: authored,
compiling, never rendered.

## ⛔ Open 5 — smaller, still real

- **`PLAYER_ATTACK_HITBOX_SCALE = 1.3`** (`attack_hitbox.rs`) is still dead. It
  was Jon's 2026-07-12 pogo-feel fix and has been unreachable since the volume
  lookup became character-id-keyed. Restore it on the live path or delete it;
  leaving it as decoration was already flagged once in this document.
- **`air_neutral` is authored and unbound.** No move names that row, so its ring
  geometry is unreachable.
- **The authored hurtbox TIMELINE is still rect-only.** `DamageableVolumes` can
  carry hulls and the sprite-metadata producer emits them; the entity-catalog
  hurtbox contracts still author rectangles.
- **`capture_scene` does not finish on the dev VM** (measured: forty minutes, no
  frame). Documented in the recipe with what to try; not fixed.
