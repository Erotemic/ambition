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
