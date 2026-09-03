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

✔ **ANSWERED by Jon, 2026-09-02, verbatim:** *"Weapons can have recoil and they
will kick whoever is firing it. The kick might depend on the mass property of
the actor doing the firing."*

⇒ **The first sentence settles this entry: delete the zeroing.** One weapon, one
authored number, and it kicks its wielder whoever that is. `fire_held_ranged_system`
stops special-casing hand-fired items and the guard retargets to expect recoil.

▢ **The second sentence opens a NEW question and must not be smuggled into this
one.** "Might" is a direction, not a ruling. Recording what it would have to
attach to, because the input already exists and is not vestigial:
`ActorDefinition.mass` is `Option<f32>`, read at spawn as
`definition.vitals.mass.unwrap_or(1.0)`, merged by
`character_runtime/physical_baseline.rs`, and rollback-registered under the
stable name `mount.mass`. So a mass-scaled recoil would read a live,
rollback-safe value rather than needing a new authored field. ⛔ What is NOT
decided: the curve (linear in mass? inverse? clamped?), whether an unauthored
mass of 1.0 means "average" or "unscaled", and whether this generalises to all
knockback or only to discharge recoil. Do not pick one by inference — a shipped
feel change to every weapon is a bigger ruling than the one asked here.

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

✔ **ANSWERED by Jon, 2026-09-02, verbatim:** *"They get hit by the spikes (as
long as they are not immune - e.g. they might have iframes from a ledge grab).
Spikes may or may not insta-kill, they could just do damage."*

⇒ **"Dies" was the wrong framing of the second option and the ruling corrects
it.** The body is HIT; whether that kills it is the hazard's authored damage,
not a property of hanging. Both halves are already expressible and need no new
vocabulary: `HazardSpec` carries `damage: i32`, `knockback`, `kind`, `team`,
`hitstop_seconds` and `respawn`, so a spike that merely hurts is authored, not
built.

⛔ **AND THE EXEMPTION MOVES.** It is no longer "hanging is a committed state" —
a hanging body is judged like any other, and if it survives that is because it
is IMMUNE, through the ordinary invulnerability road. Jon names a ledge grab
granting iframes as an example, not as a fact about today's code.

▢ **So what remains is one question this entry did not ask:** does a ledge grab
grant iframes, and if so for how long? ⭐ It has a home already — invulnerability
is a REASON SET, not a flag (`features/empowerment.rs` delegates
`Empowerment::UNTOUCHABLE` to the body's invulnerability-reason set, beside
`Invulnerability::EMPOWERED`), so "hanging on a ledge" would be another reason
rather than a new mechanism. That is a separate authoring decision
from the gate ordering, and it is the one that decides whether the visible
behaviour actually changes. Until it is answered, moving the gate makes hanging
bodies take spike damage — which is now the intended behaviour, so the
constraint that held the gate back is lifted.

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

✔ **RE-MEASURED 2026-09-02 evening and the claim reproduces exactly**:
`pirate_admiral_vfx` 14 rows / 0 named, `george_booul_vfx` 21 / 1 named — 34 of
35, unchanged. ⓘ Wider context the row did not have: across all 13 fx sheets it
is **196 rows, 120 named, 76 named by nothing**, and `pirate_admiral_vfx` is the
only sheet with NO row named by anything. So the two named here are the extreme
of a spread, not isolated cases — which strengthens "wire them" and weakens
"they were superseded", because eleven other sheets are partly wired.

⇒ **Wire them, or were they superseded?** The row→move pairing is unambiguous
from the names, but WHEN in a timeline each fires and at what scale is a feel
ruling, which is why it is here rather than done. Residency's interest was the
size (9.4 MP resident in every room); since 2026-09-02 the two sheets are never
decoded, because no realized character's moveset names a row of them.

### The LDtk editor-preview tileset is 7.6 MP the runtime never draws

**FIVE** world files declare `sprite_player_robot_v3 = ../sprites/player_robot_v3_
spritesheet.png` so the editor can draw entity previews, and `bevy_ecs_ldtk`
decodes every tileset of a project when the project loads — on every boot and
every world load, at FULL tier, beside whatever tier the game actually realizes.

⭐ **MEASURED 2026-09-02 (`scripts/measure_ldtk_tileset_usage.py`), and the two
things that were assumed are now checked.** NO LEVEL LAYER in any of the five
uses the tileset (`layerInstances[].__tilesetDefUid`, across 1/11/60/1/1
levels); its only consumer is one entity definition per world, `PlayerStart`,
cropping the top-left tile. So "editor previews only" is measured — which
matters, because a layer using it would have made a cheaper tier a QUALITY
decision instead of a free one. And it is five worlds, not four: the four
`ambition_content` ones plus `ambition_demo_sanic/worlds/sanic_speedway.ldtk`.
`mary_o` does not reference the sheet.

⛔⛔ **AND IT IS NOT "one line per world".** `tileRect`, `uiTileRect`,
`tileGridSize`, `pxWid` and `pxHei` are all TILESET PIXEL coordinates. Change
only the `relPath` and the 256×256 crop that framed one animation frame spans a
third of an 832-pixel image — the preview breaks while the JSON still looks
plausible, and nothing in the game reports it. The tiers are also not exact
fractions and the x and y factors differ (`sprites_0_25x` of 3072×2468 is
832×653, not 768×617), so there is no constant to scale by.

⇒ **A prepared patch is waiting: `dev/patches/ldtk-player-tileset-retarget-20260902.patch`**
(`patch -p1 <` or `git apply`, both verified; regenerate with
`scripts/propose_ldtk_tileset_retarget.py --tier <tier>`). It recomputes every
pixel field from the real PNG header and preserves each crop as a FRACTION of
the image. Boot decode for this tileset goes 7.6 MP → ~0.54 MP.

ⓘ It also fixes two stale declarations found by reading the real header: the
four content worlds declare `pxHei 2484` against a 2468-pixel file, and
`sanic_speedway` declares `1681×1728, tileGridSize 224` for that same file. ⚠ The
patch preserves sanic's framing as a fraction, i.e. whatever that stale
declaration was already showing.

⛔ **Jon's submodule, so it waits** — applying it needs a commit in
`game/ambition_map_assets` plus a pointer bump. ▢ And it is untested against the
LDtk editor itself, which needs Jon opening a world.

### The shared sprite pack is 442.6 MB and one prop reads it (raised 2026-09-02)

⭐ **MEASURED** (`scripts/measure_pack_reachability.py`).
`build_prop_sprite_asset_packed` is the ultrapack's ONLY production consumer; it
has ONE call site, the intro prop loop; and it runs only for
`intro_prop_sprite_rows()` entries whose 4th tuple element is `Some(target)`.
**Exactly one row is: `intro_cart`.** Characters have no pack road at all —
`load_character_sprites_in` takes the per-target `*_spritesheet.ron` every time.
All four tiers pack the same 197 targets. On one machine: **442.6 MB of pack
pages, 5.2 MB on a page any consumer can reach — 98.8% unreachable.**

⚠ **NOT A DEFECT REPORT.** Packing every target is what a packer should do; the
finding is that adoption never followed. Reachability is a SOURCE fact and reads
the same on any checkout; the megabytes are generated, gitignored, per-machine.

⇒ **Three answers are all reasonable and none is an agent's call:** adopt the
pack for characters (it was built for that, and `project_ultrapack` design intent
says the two roads should converge); narrow the generator to pack only what a
consumer opts into; or leave it, on the grounds that a packer that packs
everything is correct and the cost is disk nobody is paying attention to. ⛔ What
is NOT reasonable is dropping the per-target PNGs to "save" the duplication —
they are every character's only road.

### Portraits are generated at four tiers and only full resolution is readable (raised 2026-09-02)

`bake_portrait_manifests` collects portrait manifests from `assets/sprites` ONLY
and says why: *"Portraits are presentation products and currently have no
quality-tier variants"*. The generator emits the PNGs at all four tiers anyway —
**487 files, 14.2 MB, with no road**
(`scripts/measure_orphan_shipped_pages.py`).

ⓘ The missing `.ron`s are POLICY, not a bug —
`check_quality_variants_are_fresh.py` records that portraits are *"published
SELECTIVELY"*. ⛔ But the 9 that ARE published per reduced tier cannot be read
either: `PortraitSheetRegistry` is built `from_baked_table(BAKED_PORTRAIT_RONS)`
and `build.rs` bakes from `assets/sprites`. A deliberate selective publication
produces files no build can load.

⭐⭐ **AND THE MEASUREMENT NARROWS THE ANSWER — 2026-09-02,
`scripts/measure_portrait_tier_headroom.py`.** Portrait draw size is chosen by
VIEWPORT, never by quality tier: `DialogLayoutProfile::for_viewport` picks
**56×62** (phone landscape), **82×94** (phone portrait / small tablet) or
**104×120** (everything else), consulting no quality setting. So no quality tier
can select a portrait resolution — the window size does. Against `alice`:

```text
tier              frame     @1x display          @2x display
sprites          256x320    covers every box     covers every box
sprites_0_5x     128x160    covers every box     smallest box only
sprites_0_25x      64x80    smallest box only    UPSCALES ALWAYS
sprites_potato     16x20    UPSCALES ALWAYS      UPSCALES ALWAYS
```

⇒ **Nothing wants a Potato portrait**: at 16×20 it is under even the 56×62 box
at 1×. `sprites_0_25x` is defensible only on a phone-landscape viewport at 1×
display scale — the least likely combination, since phones are high-DPI. Only
`sprites_0_5x` has a real case, and only at 1×. ⚠ A tier under the box it is
drawn into is not a cheaper portrait; it is a blurrier one, which is the
failure Jon's standing rule forbids.

ⓘ Residency is already bounded independently: `RetainedHudImages` holds one
entry per portrait ACTUALLY SHOWN (~1.3–2.0 MP each), not the 163 baked
manifests — so the tiers would save package size, not runtime memory.

⭐ **AND THEY ARE STILL BEING PRODUCED, not left over.** Age signal
(`measure_orphan_shipped_pages.py`): 439 of 475 comparable portrait files were
written in the same run as their full-resolution twin or after it, median +3.07
days — against the stranded sheet pages, where 44 of 44 predate their manifest.
⇒ **A clean regen on another machine will reproduce these**, so the "wait for
yardrat" answer that covers the stranded pages does not cover this row.

⇒ **Stop generating them, start baking them, or leave them?** The measurement
says at most one tier (`0_5x`) could ever be wanted and two certainly cannot.
The comment says portraits have no tier variants *currently*, which reads as
intent that may change — and that is the part an agent cannot know.

### 44. Should `SmashChargeSpec` keep a game-mode name for a general mechanism?

Jon raised the general shape of this on 2026-08-28, about a different type
(*"it might be a good idea to rename the actor given its conflation with a very
core concept in the architecture. But we can do that in a different pass"*). The
`Actor` half was done — it is `Performer` now. This half was never put to him and
should not be decided by an agent, because a rename is Jon's vocabulary call.

⭐ THE CONFLATION IS MEASURED, not assumed. `SmashChargeSpec` is named for one
game mode and its own doc comment describes something general: *"How a chargeable
move HOLDS: where on its own timeline the charge waits, and how long it may wait
before it fires itself."* It carries `roots`, `sustain`
(`WhileHeld` / `UntilPressedAgain`) and two seconds-valued clocks in the owner's
proper time — none of which is Smash-specific — and the Trap (the Performer's
down-B, an Ambition move, not a Smash one) uses it for a three-second
subterranean beat, which is what made the name visible.

⚠ THE SIZE, so the answer can be costed: **36 references** across `crates/` and
`game/`. A rename is mechanical and touches authored content, so it wants to
happen in one pass or not at all.

⇒ Three answers are all reasonable and the choice is not an agent's: keep the
name (the mechanism was authored for Smash and the association is useful), rename
to something like `TimelineHoldSpec` / `ChargeHoldSpec`, or keep it and let a
future Smash-specific type take the name back. ⛔ No engineering is blocked
either way — this is recorded so it stops being asked and forgotten.

### Interact dialogue for the characters the Hall's authoring did not cover (raised 2026-09-02)

`triage/character-dialogue-from-suggestions.md` re-measured: 149 catalog rows,
124 with a `hall_dialogue_id`, 131 authored `hall_*` Yarn nodes. The Hall was
solved by hand-authoring, the escape the 2026-07-26 decision left open, so a
generator built to that decision would generate over 124 characters that
already have nodes. What remains is ~25 rows with no hall id, and every room
that is not the Hall, where a character with a real `fallback_dialogue` voice
still opens `generic_npc` on interact.

Needed: keep authoring by hand (then the triage closes as superseded), or
build the generator for the remainder only (a per-character node synthesized
from `fallback_dialogue`, overridden by any authored node of the same title).
Content call; the engine side is unchanged either way.

### 45. Is a unique capability item an ENTITLEMENT or an OCCURRENCE? (2026-09-02)

`item-custody-and-accounting.md` I3 asks special pickup roads to "converge
toward the same occurrence/custody model as ordinary held items **when that
model can express their semantics**". Measured, it cannot express the portal
gun's, and the difference is a product decision rather than an engineering one.

An ordinary held item is an OCCURRENCE: a `GroundItem` with a `SimId` that
persists through pickup and drop, its location remembered by custody and the
whereabouts ledger. The portal gun is an ENTITLEMENT: picking it up despawns the
world token, grants `PortalGun` on the body and `Item::PortalGun` in
`OwnedItems`, and **dropping never revokes the grant** — it unequips and spawns
a fresh, room-scoped token. The menu re-equips straight from `OwnedItems`. The
code states the intent where it is decided: *"The gun is a single item: it
doesn't exist until you pick it up — picking up the one world item IS getting
the portal gun."*

✔ Nothing is broken: `OwnedItems::grant` clamps a unique item to 1, so the two
roads cannot inflate a count, and the measured behaviour is self-consistent.

⇒ **The question is what you want a unique capability item to MEAN**, and the
two readings differ observably in exactly one place: **can dropping the portal
gun and walking away ever lose it?**

- **Entitlement** (what ships today): no. Once acquired it is yours; the world
  token is a convenience for re-equipping in place. Zelda's hookshot.
- **Occurrence**: yes. The gun is a thing that exists somewhere, can be left in
  a room, stolen, or lost down a pit, and the durable record is where it IS.

⛔ Not an agent's call — it decides whether a whole category of future item is
losable. Recorded rather than implemented; I3 stays open behind it.

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

### ~~Which character owns each per-fighter FX sheet~~ (raised 2026-09-02, withdrawn 2026-09-02 — no decision was needed)

The question assumed the demand seam needed a sheet → CHARACTER ID table. It
does not: a realized character carries its own prepared moveset
(`PreparedCharacterDefinition.kit.projectable_moveset()`), and the moveset's
`Vfx` events name the rows. `character_sprites::demand_character_fx_sheets`
asks that moveset the frame the character realizes and decodes whichever
character-owned sheets its rows live on (`fx::owned_fx_sheets_named_by`) —
ownership is read off the content that fires the effect, not off a name.
Landed in `asset-preparation-and-residency.md` §2. The two never-wired sheets
below are unaffected: a sheet no moveset names is now never decoded, which is
the correct residency for art nothing can request, and the wiring question
stays the owner's.

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
