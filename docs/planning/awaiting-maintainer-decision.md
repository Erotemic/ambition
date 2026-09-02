# Awaiting a maintainer decision

Only questions whose next step is **Jon's product/authoring judgement** belong
here. Engineering work goes to [`queue.md`](queue.md) or [`tracks.md`](tracks.md).
Answered rulings belong in [`maintainer-decisions.md`](maintainer-decisions.md);
the investigation that led to an answered question remains available in git
history.

This file intentionally does not retain answered decision transcripts.

## Open decisions

### 37. Should the F9 rollback proof pulse survive a gameplay-session change?

`LocalSessionPolicy::check_distance` is raised by the F9 proof pulse and returns
to normal only when that pulse finishes. If the player quits to title during the
pulse and launches another game, the elevated verification distance currently
survives.

The recent session-ownership fix deliberately did **not** decide this because the
value is developer tuning rather than gameplay authority.

Choose one policy:

- **session-scoped:** a new gameplay session always starts with the ordinary
  check distance; or
- **process-scoped developer intent:** the proof pulse deliberately spans a
  relaunch until it completes/cancels.

This is primarily a developer-iteration/expectation decision. The gameplay
rollback authority itself is already session-owned by ADR 0027.

### 36. What are the authored standing heights of the puppy slug, stochastic parrot and burning flying shark?

These are the remaining characters whose old size derivation cannot be replaced
by preserving one existing placement size because their authored spawn boxes
disagree substantially across rooms.

Representative placement variation:

```text
npc_puppy_slug            (48,22), (32,48), (64,32), (48,32),
                          (64,16), (52,66), (42,42), (28,44)
stochastic_parrot         three different boxes
npc_burning_flying_shark  mostly (108,96), also (32,48)
```

The needed value is one character-authored `standing_height` in world units for
each. Do not choose by majority box size: the box was editor/layout data, not a
stature authority.

Decision 32 still applies: there is no standard adult/humanoid height and no
bulk normalization. Character stature is authored individually.

Related visual followups that should be judged by playtest rather than another
population average: the cove pirates relative to Robot v3, slop size, and the
Mary-O snake's post-rescale size.

### 35. What should own fighter reach during move startup?

The current fighter brain uses one global `REACH_TOLERANCE = 2.0`, effectively
allowing a move to remain viable out to roughly three times its authored reach.
The bug that exposed this proxy is fixed; the constant is not currently known to
cause a product defect.

The design choices are:

1. keep the proxy until platform-fighter option ranking has its own capability
   boundary;
2. add a per-move tolerance field;
3. derive reachable distance from move startup plus the body's movement
   capability, which requires threading capability/top-speed information into
   perception;
4. for moves with authored startup impulse/travel, derive that part directly
   from the move and keep a fallback for ordinary movement.

**Default if no change is desired:** leave it. Do not widen generic actor data for
a proxy that is not currently hurting play merely to close a planning row.

### 34. Should external/launch-owned motion become an explicit cross-game fact?

Three shared movement decisions have historically inferred "this velocity belongs
to a launch rather than locomotion" from speed magnitude: initial-dash settling,
shield braking and body-contact resistance. The approximation fails when a launch
has decayed below ordinary run speed.

Smash already has a live tumble mechanic and therefore a genre-specific fact that
can represent external/launch-owned motion. Ambition does not necessarily want
Smash tumble semantics for ordinary bodies.

The decision is whether to:

- keep the current thresholds until a visible defect requires more;
- introduce a generic carried/external-motion ownership fact with Smash tumble as
  one producer; or
- let the platform-fighter capability own the richer rule while the shared kernel
  keeps the simpler behavior for other games.

Do not solve this by simply reading Smash's `tumble_speed` from the generic
kernel; the question is exactly whether that game-specific semantic is shared.

### 33. How should a recharging ranged weapon communicate that it is unavailable?

The firing cadence is implemented: `BodyMelee::ranged_cooldown` follows the
weapon's authored refire interval, and an early press is refused before spending
the proposer so ordinary combat buffering can retry when the weapon becomes
ready. The unresolved part is presentation.

Choose the product channel when this becomes important in play:

- character/muzzle VFX driven by recharge fraction;
- a presentation treatment on the firing limb/body; or
- a HUD indicator.

The mechanic does not need to block architecture work while the unavailable
state is merely invisible rather than incorrect. Prefer character-local
presentation if it reads clearly; do not add another gameplay authority to show
the cooldown.

### 38. Does an actor released in a foreign room stay there?

Today an actor moved away from its authored home and then left in another room is
retired when that room unloads and is authored again at home when encountered
later. The current construction road honestly refuses to claim the actor was
persistently relocated.

Two valid product policies:

- **go home:** authored home placement is restored when the actor is no longer
  live/resident;
- **stay where left:** persist a `Placed`/relocation occurrence for actors as is
  already done for relevant item occurrences, and teach reconstruction to honor
  it.

If choosing “stay,” the producer and reconstruction consumer must land together;
recording a moved placement that construction refuses would only add warnings and
still teleport the actor home.

This decision feeds
[`engine/construction-and-reconstitution.md`](engine/construction-and-reconstitution.md)
and [`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md).

### 39. Which authored move, if any, should adopt the dormant windbox/armor vocabulary?

The windbox mechanic is implemented and can express outward gust or inward
suction. `WindowTag::Armor` also exists but has no shipped authored customer.
There is no engine defect merely because the vocabulary is currently unused.

If one should become product-visible, name a fighter/move. Otherwise leave the
mechanism dormant until a character design asks for it. Do not invent a customer
to make an adoption count nonzero.

### 40. Should a held gun-sword kick the player the way it kicks the pirate?

The K2 fold puts the player's held gun-sword and fireball on the ONE projectile
road, and that road applies the weapon's authored `Discharge`.
`gun_sword_discharge()` authors **380 px/s of recoil**, written for the pirate
who carries it.

The held-shot path that was deleted applied **no recoil at all** when the player
fired. That difference was a property of the second code path, not an authored
decision — nobody chose it. The fold preserved the old feel by zeroing recoil
for hand-fired held items, so today the same weapon kicks its NPC wielder and
not its player wielder.

Choose one:

- **the weapon kicks whoever fires it:** delete the zeroing; a player firing the
  gun-sword takes the authored 380 px/s. One weapon, one authored number.
- **recoil is a wielder property:** keep the zeroing, and say so in the weapon
  vocabulary rather than as a special case — an NPC braces, a player does not.
- **the player number is simply different:** author a separate player recoil
  value; name it.

⚠ This is a FEEL ruling on a shipped weapon, not an engineering question. The
engineering is done either way; the fold currently encodes "no kick for the
player" only because that is what the deleted path happened to do. The zeroing
is `fire_held_ranged_system` in `items/pickup/mod.rs`; the guard is
`a_hand_fired_gun_sword_bolt_flies_the_one_projectile_road`
(`game/ambition_app/tests/hand_fired_held_shot.rs`) — retarget it with the
ruling.

### 41. Where should a hand-fired fireball leave the body?

The deleted held-shot path spawned the fireball from a side muzzle at
`(size.x / 2 + 8, -0.12 * size.y)` — offset forward and slightly toward the
head. The projectile road's default is `Muzzle::BodyOrigin`, a few pixels away
from that point.

The difference is small and entirely cosmetic, but it is visible on a wide body
and it changes where the shot clears the player's own silhouette.

Choose one:

- **body origin:** accept the road's default and retire the old offset; one
  spawn rule for every projectile.
- **keep the authored muzzle:** register the old offset as an authored
  `Muzzle` on the fireball so the fold preserves the shipped look exactly.
- **a muzzle is a per-weapon authored fact:** every hand-fired weapon names its
  own, and the fireball's is the old offset.

The fireball's spec (`held_item_by_id("fireball")`, `action_set/mod.rs`) authors
`Muzzle::default()` today.

### 42. Should the gauntlet fireball keep its own sprite, or become the catalog's energy ball?

The deleted path drew the fireball as the 30 px `gauntlet_fireball.png` sprite.
The projectile catalog's `"fireball"` is a tinted energy ball — a different
look, shared with every other fireball in the game.

To avoid changing a shipped visual inside a refactor, the fold registers a
`"gauntlet_fireball"` Image visual that reproduces the old sprite.

Choose one:

- **keep the gauntlet's own sprite:** the registered Image visual stays, and the
  gauntlet reads as its own weapon.
- **adopt the catalog energy ball:** delete the registration; one fireball look
  across the game, and the gauntlet loses its distinct art.
- **the sprite is right but the catalog should own it:** promote
  `gauntlet_fireball` into the shared catalog as a first-class projectile visual
  other weapons may also use.

⭐ **THE SIZE/ANCHOR COMPARISON IS DONE, IN CLOSED FORM, 2026-09-02 — and it
found the difference somewhere else.** A capture was queued for this; reading the
two paths answers it more exactly than a screenshot could.

- **SIZE: exact parity, at every quality tier.** The deleted renderer drew
  `Vec2::splat(30.0)`. The new registration is
  `ProjectileRenderSize::FixedWidth(30.0)`, which resolves as
  `Vec2::new(w, w / frame_aspect)`. `gauntlet_fireball.png` ships at **64 x 64**
  (base), 32 x 32, 16 x 16 and 8 x 8 (`sprites_potato`) — **all square**, so the
  aspect is 1.0 and the height is 30.0 whichever tier the quality budget loads.
  Identical, not merely close.
  ⚠ An earlier revision of this note said the sprite was 16 x 16. That was the
  `sprites_0_25x` copy — one of four, read without checking the others. The
  conclusion was right and is now stronger, but the number was a sample quoted as
  the population.
- **ANCHOR: exact parity.** The old sprite took `..default()` (centre); the new
  Image path passes `anchor: None`, which is the same centre.
- ⛔ **DEPTH CHANGED, and nobody asked about depth.** The old fireball drew at
  `z = 9.5` — below `WORLD_Z_DUMMY` (10.0) and well below `WORLD_Z_PLAYER`
  (20.0), so it passed BEHIND the player and behind enemies. The projectile road
  draws every shot at `projectile_z()` = `WORLD_Z_PLAYER + 2.0` = **22.0**, in
  front of the player. `world_to_bevy` passes `z` straight through, so these are
  the same scale and directly comparable.

⇒ The fold moved the fireball from behind the cast to in front of it. That reads
like a repair rather than a regression — a thrown fireball vanishing behind a
body is hard to defend — but it IS a visible change, it was not authored as part
of the fold, and it is not what decision 42 asks about. Say whether the new
layering is what you want; if it is, nothing to do.

⚠ **What a capture would still add, and only this:** that the asset RESOLVES at
runtime rather than drawing the magenta placeholder. The geometry above needs no
picture.

The registration is `GAUNTLET_FIREBALL_VISUAL` in
`game/ambition_content/src/projectiles.rs`.

All three (40–42) were opened by the K2 fold on 2026-09-02; the fold itself is
landed and none of them blocks it.

### 43. Does a body hanging on a ledge inside a hazard volume die?

Spikes under a ledge lip is an authored shape, and a hanging body's box can
overlap one. Today it does not die: an active ledge grab consumes the simulation
frame before the hazard/OOB gate runs, so the gate never judges it.

That was not authored — it is a consequence of where the gate sits. It surfaced
2026-09-02 when the gate moved to fix an ordering bug and would have started
judging three populations it never had; the move was constrained back to its
original population rather than deciding this by accident. See
[`engine/collision-and-ccd.md`](engine/collision-and-ccd.md) §1.

- **stays immune:** hanging is a committed state, and a body that cannot act
  cannot be asked to escape; or
- **dies:** the hazard is a volume and being inside it is being inside it,
  whatever the body is doing.

⚠ Recorded, not recommended. Whichever way it goes it is a FEEL ruling and wants
authoring deliberately, not acquiring from a refactor.

### Two fighters' bespoke effect art is never requested (2026-09-02)

`npc_pirate_admiral` has a 14-row effect sheet and `smash_george_booul` a 21-row
one, and **nothing in the repository names 34 of those 35 rows** — measured by
`scripts/measure_fx_row_reachability.py`, which asks which fx row names any
tracked `.rs`/`.ron`/`.yarn`/`.json` mentions (an effect is drawn by name, so a
row nothing names cannot be requested).

⛔ IT IS NOT DEAD ART, which is why this is a question and not a slice. The rows
were drawn FOR those kits: `grapeshot_cloud`, `heave_to_anchor`,
`heave_to_brake`, `cutlass_wake`, `boarding_wake`, `captains_mark` sit beside an
admiral moveset whose moves are named `grapeshot`, `heave_to`, `gun_sword` — and
that moveset asks for `muzzle_flash` and `air_slice` from the GENERIC sheets
instead. George is the same: `bivalence_weak`/`bivalence_strong`,
`excluded_middle_windup/launch/ascent/gate/tail`, `modus_ponens_*`, `reductio_*`,
beside moves called `bivalence`, `excluded_middle`, `commitment`.

⇒ **Wire them, or were they superseded?** The row→move pairing is unambiguous
from the names, but WHEN in a timeline each fires and at what scale is a feel
ruling, which is why it is here rather than done. Residency's interest is only
the size: the FX set is 9.6 MP resident in every room and 76 of its 196 rows are
currently unrequestable.

### A view-scoped sprite tier: is the pop acceptable? (2026-09-02)

Fourteen of the hall's 138 resident cast pages are ever drawn — 90% of the cast's
pixels are decoded for characters nobody can see. The room-level tier cap cannot
reach that, because every character in a room gets the room's tier whether it is
on screen or three screens away. Scoped in
[`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md);
the engineering is small (the per-token tier floor already exists on the demand,
and only rooms use it today).

⛔ TWO OF THE THREE OPEN QUESTIONS ARE FEEL, which is why it is not built:

- **Pop.** A character that walks on screen at a low tier and converges upward
  re-tiers in front of the player. The room cap has no equivalent moment — it
  changes only at a room boundary, under a cover. Is that acceptable, does it
  want its own cover, or must the convergence be fast enough to hide?
- **Hysteresis.** A character loitering at the edge of a view would re-tier on
  every crossing, which is the re-decode this project already measures at 44 MP
  for a second hall entry. A margin or a dwell time is required and its size is a
  feel number.

⚠ The third is engineering and answerable here: with split-screen and the N-view
work, "a live view" is plural, and a character visible to seat 2 must not be
tiered for seat 1's camera.

### The LDtk editor-preview tileset is 7.6 MP the runtime never draws

Every world file declares `sprite_player_robot_v3 = ../sprites/player_robot_v3_
spritesheet.png` (3072×2484) so the editor can draw entity previews, and
`bevy_ecs_ldtk` decodes every tileset of a project when the project loads — on
every boot and every world load, at FULL tier, beside whatever tier the game
actually realizes. The runtime never draws it. Fix is one line per world in the
map submodule (point it at the `sprites_0_25x` copy, 0.5 MP; the editor preview
survives at a quarter resolution). ⛔ Jon's file, so it waits.

## Waiting on maintainer measurement, not a decision

### The residency limit open work 4 needs

A budget policy that keeps the last room's cast resident needs a ceiling, and the
ceiling is a host number: `resident_mb` at Full on the 3090 after a
hub→hall→hub walk. Everything else in that section is measured; this is the one
input that cannot be taken on a software rasteriser.

### D-RASTER-3's remaining half

Splitting the weak-GPU 2.54× between framebuffer scale and MSAA needs an
interleaved A/B on real weak-GPU hardware with the independent
`AMBITION_MAX_SCALE_FACTOR` and `AMBITION_MSAA` knobs, multiple reps per arm,
build/features/profile held constant. ⛔ Explicitly not lavapipe: the row says so
and the substitution is what made the original result unattributable.

### Switch Pro outer stick range

The remaining cross-machine controller question needs the actual hardware
measurement: run the existing `Shift+F6` axis probe on both machines, push the
Switch Pro to each extreme/corner, and compare reported peak magnitude.

The proposed shared outer-saturation fix should be judged only after that number
exists. This is tracked in the execution queue as an external measurement, not a
maintainer design decision.

### Which character owns each per-fighter FX sheet (raised 2026-09-02)

Seven of the thirteen FX sheets are named by exactly one moveset file, so their
demand could move from "resident in every room" to the room's cast — 9.6 MP is
31% of the hall's resident megapixels and this is the largest owner left after
the boss sheets. `scripts/measure_fx_row_reachability.py --owners` measures the
sheet → MOVESET FILE half. The demand seam needs a sheet → CHARACTER ID map, and
that last hop cannot be derived: `noether_vfx` belongs to `npc_emmy_noether`,
`carl_stargan_vfx` sits beside a sheet target `carl_runga`, and three sheets use
bare ids with no `npc_` prefix. A wrong guess loads nothing and the effects fall
back to particles SILENTLY.

Needed: the character id for each of `oiler_vfx`, `pca_vfx` (named by
`cellular_automaton_moveset.rs` — "pca" is not a fighter), `patent_clerk_vfx`,
`carl_stargan_vfx`, `noether_vfx`, `ninja_shadow_oni_leader_vfx`,
`projectile_polygon_vfx`.

⚠ And a separate content question in the same measurement, NOT blocking the
above: `pirate_admiral_vfx` (14 rows) is named by nothing at all, and
`george_booul_vfx` (21 rows) only by a test and the engine's own `fx.rs`. Their
rows sit beside movesets whose moves have matching names, so this reads as
missing wiring rather than dead art — but whether to wire them or drop them is
the owner's call. Both stay resident until it is made.

Engine side is unblocked the moment the id column exists: load only the four
shared sheets at boot, ensure the owned ones at room-manifest time exactly as
`ensure_boss_sheets_loaded` already does for bosses.

## ✔ WITHDRAWN 2026-09-02 — "is 8% of the floor crate worth an encoder split?"

Raised here the same day and withdrawn the same day: it is an ENGINEERING
question, not a product one, so it is answered in
[`engine/control-authority-and-ai-policy.md`](engine/control-authority-and-ai-policy.md)
instead. Short answer: do not split. Of the three available shapes, two are
refused by rules the repository already holds (no service locator; the orphan
rule again one crate up) and the third fails the plan's own acceptance criterion
4, because `ambition_mount` holds a `Brain` by value and does not depend on
`ambition_combat` — so moving `Brain` into a combat crate makes a movement-only
game link one. Recorded here only so the question is not raised a third time.
