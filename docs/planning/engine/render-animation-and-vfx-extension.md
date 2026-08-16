# Render, animation and VFX extension — authored action FX and particle backends

**State:** TEMPORARY IMPLEMENTATION EXTENSION — OPEN.

**Parent roadmap:** [`render-animation-and-vfx.md`](render-animation-and-vfx.md).

**Integration owner:** the coordinator agent, not the implementation agents working
individual sessions below.

**Disposition:** this file is deliberately adjacent to the parent because the parent
roadmap is currently in flux. It is **not** a second permanent source of truth. The
coordinator must fold the surviving decisions, evidence, and remaining work back into
`render-animation-and-vfx.md` once that document is ready to absorb them, then delete
this extension in the same consolidation campaign.

> ⛔ **Implementation agents: do not skip this program because it looks broad.** The
> work is already decomposed into session-sized tasks below. Take one task (or one
> explicitly marked pair), re-measure HEAD, implement its exit criteria, and stop.
> If HEAD has moved, preserve the architectural intent and report the concrete rename
> or blocker rather than declaring the task “too large” or silently omitting it.
>
> ⛔ **Do not opportunistically rewrite the parent roadmap while executing these
> sessions.** It may be changing under another agent. The coordinator owns the final
> fold-back and conflict resolution.

---

## ⭐⭐ COORDINATOR DISPOSITION 2026-08-15 — the campaign SPLITS at the dependency line

⚠ **this program is two programs, and only one of them has a customer today.** Read
this before taking any task below; several are deferred and one candidate is dropped.

### ▶ RUNNING NOW — zero new dependencies

| Task | Why it goes first |
|---|---|
| AUTH-01 | ✔ **already landed** — `tools/ambition_sprite2d_renderer@59139e4` authored the action-FX sheet with frame timing and anchors in a generated sidecar |
| AUTH-03 | ▶ **in flight** — authored art replacing the placeholder impact, against two customers: Smash hit feel and the Mary-O coin pop |
| VFX-01 · VFX-08 · VFX-09 · VFX-11 | ⭐ **boundary work that is true whatever backend ever wins.** VFX-08 is the one with a real defect under it: `VfxMessage::Burst { count, speed, color, kind }` leaks renderer-shaped choices to simulation-side producers, and that is wrong today, with or without a particle crate |

### ⛔⛔ WITHDRAWN 2026-08-15 — the deferral below rested on a bad argument

⭐ **"no customer yet" is the wrong test for this project.** Jon: *every* game
wants good VFX, so we will need it at some point; and this is a hobby project
built with time and ambition, not one under shipping pressure. ⇒ a
YAGNI/nobody-wants-it-yet argument does not adjudicate anything here. The only
legitimate question is **ORDER**, never *whether*.

Of the three arguments below, exactly one survives on its own terms:

- ⛔ **argument 1 (repertoire, not capability) — WITHDRAWN as a reason to defer.**
  It is true and it explains why authored art paid off first; it does **not** show
  a particle runtime is unwanted.
- ⛔ **argument 3 (the spike ships evidence, not a game) — WITHDRAWN.** Evidence
  is a legitimate deliverable when the goal is an engine.
- ⚠ **argument 2 (capability footprint) — SURVIVES, but only as a COST to measure,
  not a veto.** It is a real number the adapter task must check, which the plan's
  own dependency section already requires.

⭐⭐ **and the reopening trigger stated further below was ALREADY MET at authoring
time** — the case for Enoki is its **RON-authored, hot-reloadable effect ASSETS**,
which is the same "content as data, not Rust" claim the LDtk lane just proved for
spatial content. That is a north-star capability, not a nice-to-have. ⇒ **Enoki is
scheduled, not deferred.**

⭐ **what survives is the ORDERING only:** VFX-01 · VFX-08 · VFX-09 · VFX-11
first, because VFX-08 repairs a defect that exists today and the adapter should
plug into a seam that is already correct rather than one being fixed underneath
it. ⛔ that is a sequencing claim; it is not permission to drop the spike.

⚠ **Hanabi stays dropped, and that ruling is untouched by any of this** — it rests
on a platform fact (WebGPU-only compute against an Android target), not on
appetite or timing.

---

⚠ **superseded reasoning below, kept because the flawed argument is worth being
able to recognise again.**

### ⛔ DEFERRED — VFX-02 … VFX-07, GATE G1, VFX-10, VFX-12, VFX-13

Not rejected. **Deferred for want of a customer**, and the deferral is cheap
*because* A1 and A5 already hold: the backend is presentation-only and its
dependencies point upward, so the swap seam can be built when something needs
swapping. ⭐ **that is the whole reason it is safe to wait** — a boundary that is
already correct does not need its far side chosen early.

Three measurements argue against spending the session budget here now:

1. ⭐⭐ **the combat vertical slice measured TWICE that the missing quality was
   authored REPERTOIRE, not simulation capability.** Timed self-motion was the one
   genuine engine gap in the whole slice. A particle runtime is a capability answer
   to what has repeatedly turned out to be a content question.
2. ⛔ **`capability-footprint-may-not-grow` currently reads 42 crates linked, 15 of
   them a movement-only game never asked for.** A GPU particle stack pushes exactly
   the number that contract exists to guard.
3. ⚠ **the spike's output is EVIDENCE, not a better game** — two adapter crates, two
   galleries, a lab host, dependency ratchets and a gate. It is the largest single
   block of work available right now and it ships nothing playable.

### ⛔ DROPPED — Hanabi, and drop it NOW rather than at G1

Hanabi is GPU/compute-shader oriented and its WASM compute path is **WebGPU, not
WebGL2**. This project ships to Android and treats `capture_scene` as the phone
proxy. ⇒ carrying a second candidate doubles the spike's cost for the candidate
whose platform story fights our actual target. ⭐ **G1 does not need two arms to be
a real gate**; it needs one candidate and a builtin baseline.

### ⭐ THE TRIGGER THAT REOPENS ENOKI — and it is NOT particle quality

⛔ **do not reopen this because an effect looks flat.** The honest argument for
Enoki is none of its particle math: it is **RON-authored, hot-reloadable effect
ASSETS**. An agent authoring a particle effect as *data* instead of Rust is this
project's whole thesis, and it is the same claim the LDtk lane is proving for
spatial content.

⇒ **the condition to state plainly:** when we want effects authored as data by
agents, and the built-in renderer cannot take an authored effect asset, adopt
Enoki for its **asset pipeline** — and say so in those words. ⚠ if instead we only
ever want a handful of hand-tuned effects, the built-in renderer plus the authored
sprite sheet is the cheaper permanent answer and this whole spike stays closed.

---

## ⭐⭐⭐ MEASURED 2026-08-16 — the authored effect vocabulary is COMPLETE, and unaddressable

Three facts, read off the shipped data rather than argued. They change what
VFX-08 is about and they touch the Enoki trigger directly.

### 1. The art and the audio already agree, one for one, across every sheet

**189 animation rows ↔ 189 `vfx.*` cues in `sfx.bank`, with no sheet off by one.**

| | sheets | rows |
|---|---|---|
| generic (`action`/`world`/`exotic`/`explosions`) | 4 | 65 |
| character (`george_booul` 21, `oiler` 23, `pirate_admiral` 14, `ninja_shadow_oni_leader` 14, `pca` 14, `patent_clerk` 14, `carl_stargan` 12, `noether` 12) | 8 | 124 |

⇒ **the unit of this vocabulary is the effect NAME.** It already addresses the
clip and its paired sound together, in the data, with no Rust in the middle. An
engine that needs three tables to reach it has invented a problem the content
does not have.

### 2. `ExplosionKind` is a transliteration of a naming the data carries twice

Five variants that ARE the five rows of `generic_explosions`, reconstructed by
three hand-kept tables: `move_vfx_kind` (name→enum), `explosion_anim`
(enum→`CharacterAnim` — i.e. effect rows addressed as *Idle/Walk/Run/Hit/Slash*),
`explosion_sfx` (enum→cue). ⭐ **the seam that deletes all three already exists**:
`SheetRecord::first_bound_row(chain)`, built 2026-08-11 as *"the seam that lets an
authored CLIP be drawn without an engine enum variant"* — for character clips,
for exactly this reason. Effects never adopted it.

⇒ this is VFX-08's defect with a second face. VFX-08 names
`VfxMessage::Burst { count, speed, color, kind }` leaking renderer-shaped choices
to simulation producers; `VfxMessage::Explosion { kind: ExplosionKind }` leaks a
closed renderer ENUM, which is why the vocabulary is capped at five while 189
effects ship. **Same seam, same fix, and the enum one has the deletion in it.**

### 3. ⛔⛔ the sheets are ABSENT from every demo, not merely limited

One table registers them — Ambition's intro
(`game/ambition_content/src/intro/sprites.rs`) — into
`GameAssets.characters.props`, *a map keyed by the LDtk `Prop.kind` field*. An FX
sheet is neither a character nor an LDtk prop; it is squatting. Smash, Sanic and
Mary-O register character sheets only, so `spawn_explosion` takes its no-asset
branch **every time** and every `Feel` class degrades to one particle burst.
⇒ **engine-level FX-sheet registration is the prerequisite.** Widening the
vocabulary buys nothing while there is no art to name.

⭐⭐ **and the reason it is absent is structural: THE ENGINE HAS NO WAY TO SHIP THE
ART IT ITSELF DRAWS.** Every registration site in the workspace is a *game*
system hand-rolling the same load — `game/ambition_content/src/intro/plugin.rs`
for Ambition's props, `game/ambition_demo_sanic/src/lib.rs` for one ring sheet —
so a sheet the ENGINE draws (`spawn_explosion` reaches for `generic_explosions`
by name, from `ambition_render`) exists only if some game happens to have
declared it. That is the same shape as an engine system reading a resource no
engine plugin inserts, and it will recur for every future built-in presentation
asset, not just effects.

### The one real design constraint

`MoveSpec::presentation_problems`' oracle is already INJECTED — `prefab_registry`
passes `|id| move_vfx_kind(id).is_some()` — but it runs at **roster install**: a
pure function, no Bevy world, no loaded assets. So a widened vocabulary cannot be
read off the sheets at validation time. Either a declared table in the `sfx_ids!`
shape (one declaration emitting constants *and* the name list) pinned by a test
that reads the shipped `_spritesheet.ron` rows and asserts set equality **both
ways**, or drop the refusal for SFX's own policy — open vocabulary, counted miss.
⚠ SFX chose the latter deliberately and it has not hurt.

### ⚠ this bears on the Enoki trigger, and it argues the OTHER way

The stated condition is *"when we want effects authored as data by agents, and
the built-in renderer cannot take an authored effect asset"*. The second half is
now measurably false: **11 of the 12 FX sheets ship a `*_authoring.yaml` sidecar**
carrying
per-row family, intent, loop/one-shot, orientation, mirror allowance,
`origin`/contact anchors, layer and attachment hints, tint policy, per-frame
phase/intensity and a `clear_frame` marker — authored effect data, generated,
already installed beside the art, and marked
`status: authoring_hints_not_yet_runtime_contract`. ⇒ **the built-in renderer's
missing piece is a READER for a sidecar it already ships, not a particle
backend.** Promoting that sidecar is the cheaper answer this plan's own trigger
says to prefer, and it should be attempted before the trigger is called met.

⭐ **and the one sheet WITHOUT a sidecar is `generic_explosions`** — the only one
the engine can currently draw, and the one `ExplosionKind` was built around. The
eleven carrying authored semantics are precisely the eleven it cannot reach. That
is the shape of the whole problem in one line.

### The slice, with its deletion gate

```
engine-level FX sheet registration (the four generic sheets, then character ones)
        ↓
an effect is a NAME: FxId (FNV-1a, exactly SfxId's shape)
        ↓
resolve name -> (sheet, row) via first_bound_row over registered FX sheets
        ↓
DELETE ExplosionKind, move_vfx_kind, explosion_anim, explosion_sfx
```

⛔ the deletion gate is those four. A slice that adds `VfxMessage::Effect` beside
`VfxMessage::Explosion` has wrapped the old model, not removed it.

## Why this extension exists

Ambition already has real VFX architecture, but it currently mixes three levels of
abstraction:

1. gameplay/presentation intent such as impacts, explosions, blink cues and fireworks;
2. a small built-in renderer that spawns sprite entities and entity-per-particle
   primitives;
3. authored procedural sprite effects such as the generic explosion sheet and robot
   slash art.

That is enough to ship effects, but it is not yet a clean long-term presentation
architecture. The current particle vocabulary is small and renderer-shaped, quality
budgets are not wired into particle spawning, effect system registration is partly
app-local, and there is no evidence yet for whether Ambition should own its particle
runtime or adapt a Bevy ecosystem particle engine.

This extension turns that uncertainty into a measured program. The goal is **not** to
select a fashionable particle crate up front. The goal is to establish the ownership
and rollback boundaries first, prototype Enoki and Hanabi in isolated presentation
crates, author a richer reusable 2D effect vocabulary, measure the candidates against
the same gallery, and only then extract the smallest permanent seam that the evidence
supports.

The likely end state is layered rather than monolithic:

```text
authoritative simulation / authored world facts
                    |
                    | semantic presentation intent
                    v
       confirmed external-effect boundary
                    |
                    v
          presentation-side resolver
        /            |             \
       v             v              v
 authored sprite   particles     trails/rings/etc.
    animation      provider       presentation
                  /       \
              Enoki      Hanabi
             ordinary   heavyweight
               2D       optional GPU
```

Neither Enoki nor Hanabi becomes gameplay authority. Neither is required to own all
VFX. A good final composition may use authored sprites for the readable core of an
effect, Enoki for ordinary particles, Hanabi only for effects that genuinely benefit
from GPU simulation, and a small built-in fallback for constrained or dependency-light
compositions.

---

## Measured repository state — 2026-08-15 snapshot

Re-measure these paths before changing them; the statements below describe the source
snapshot that produced this extension.

### Presentation-neutral vocabulary already exists

`crates/ambition_vfx/src/vfx.rs` owns a headless Bevy message vocabulary including:

- `VfxMessage`;
- `ParticleKind::{Spark, Dust, Shard}`;
- `HitBurst`;
- `ExplosionKind` / `ExplosionRequest`;
- `FireworksRequest`;
- `DebrisBurstMessage` / `PhysicsDebrisCue`;
- impact-material / hurt-feedback data used by combat feedback.

The crate deliberately has no renderer dependency. That direction is correct and must
remain true. However, `VfxMessage::Burst { count, speed, color, kind }` and `HitBurst`
currently expose low-level particle choices to simulation-side producers. Treat that
as compatibility surface to migrate incrementally, **not** as the model to expand for
all future VFX.

### The built-in renderer is already a real backend

`crates/ambition_render/src/fx.rs` currently owns:

- `process_explosion_requests` and the fireworks sequence fan-out;
- `vfx_spawn_messages`;
- sprite-backed explosion playback;
- the colored-square `ImpactVisual` placeholder;
- entity-per-particle `ParticleVisual` spawning and ticking;
- reset/blink/dust/coin-pop helpers;
- speech-bubble presentation.

`spawn_burst()` contains an explicit TODO to thread the resolved visual-quality particle
budget into the central spawn path. `update_particles()` also assumes a fixed world
down when applying `p.gravity`, so this work should eventually make reference-frame
semantics explicit rather than proliferating that assumption.

### VFX system ownership is not yet fully plugin-owned

`game/ambition_app/src/app/plugins.rs` still manually registers the render-owned VFX
systems. In particular, the app registers the fireworks/explosion fan-out,
`vfx_spawn_messages`, and the update chain for particles/explosions/impacts/speech
bubbles.

The repository already has the stronger precedent in
`ambition_render::PlatformerPresentationPlugin`: the domain that owns reusable
presentation behavior should normally own its plugin registration. The first code
slice in this program therefore consolidates existing VFX registration before adding
new backends.

### Rollback already has the correct external-effect seam

`crates/ambition_platformer2d_runtime/src/external_effects.rs` quarantines presentation-
facing effects by simulation frame and releases them only after the frame is
confirmed. The classified families already include:

- `VfxMessage`;
- `ExplosionRequest`;
- `FireworksRequest`;
- `DebrisBurstMessage`;
- `CameraShakeRequest`;
- owned SFX messages.

`game/ambition_app/tests/effect_quarantine.rs` pins exactly-once behavior across
rollback/resimulation.

**This is the seam third-party VFX providers must live downstream of.** Enoki/Hanabi
particle stores, emitter timers, RNG state, GPU buffers and adapter resources are
presentation state and must never be GGRS-registered or scheduled as authoritative
simulation.

### Visual quality already contains a particle policy

`crates/ambition_persistence/src/settings/video/quality.rs` defines
`ParticleBudget { max_particles, spawn_rate_scale }` inside `VisualQualityBudget`.
Current presets resolve to approximately:

| Profile | max particles | spawn-rate scale |
|---|---:|---:|
| Potato | 16 | 0.10 |
| Low | 128 | 0.50 |
| Medium | 256 | 0.75 |
| High / Custom baseline | 512 | 1.00 |
| Ultra | 1024 | 1.00 |

The budget exists; the VFX backend does not yet honor it consistently. This program
finishes that wiring on the presentation side rather than asking simulation to emit
fewer particles.

### Authored VFX already use the sprite renderer

The procedural sprite tool already contains:

- `targets/props/generic_explosions.py` — several authored explosion rows, using
  supersampled Pillow drawing, the alpha-safe `core.draw.overlay_draw`, `build_sheet`,
  and normal sheet/manifest publication;
- `targets/props/robot_slash.py` — authored melee art tied to the actual swing envelope
  rather than an arbitrary square.

The reusable authored-VFX work below extends this pipeline. It does **not** introduce a
second asset generator.

### Candidate particle crates verified against the pinned Bevy line

As of this extension's snapshot:

- Ambition pins Bevy `0.18.1`.
- Enoki documents Bevy `0.18` ↔ `bevy_enoki 0.6`. It is a 2D particle system with
  CPU particle calculation, GPU instancing, RON-authored/hot-reloadable effect assets,
  sprite/spritesheet particles, custom materials, and an explicit WASM/mobile focus.
- Hanabi documents Bevy `0.18` ↔ `bevy_hanabi 0.18`. It is GPU/compute-shader oriented,
  supports 2D cameras and effects such as trails/ribbons, and its WASM compute path is
  WebGPU rather than WebGL2.

Do not silently bump either dependency when implementing the spike. If Bevy has not
moved, use the compatible lines above first; if ecosystem versions have moved, record
why the compatible version changed and re-run the dependency/capability checks in the
relevant task.

Upstream references for the prototype agents:

- Enoki: <https://github.com/Lommix/bevy_enoki>
- Hanabi: <https://github.com/djeedai/bevy_hanabi>

---

## Architectural rulings for the whole campaign

These are not questions for each implementation agent to reopen independently.

### A1 — third-party particle engines are presentation only

Enoki/Hanabi components, effect assets, particle stores, emitter clocks, random state,
GPU state and spawned visual entities do **not** participate in rollback registration.
They run in ordinary presentation schedules downstream of confirmed external effects.

If an effect has gameplay consequences, simulate a deterministic gameplay primitive
separately and render particles as a visualization of it. Example: a damaging flame
field is an authoritative hazard/volume plus a disposable fire presentation; it is not
500 authoritative particles.

### A2 — one-shot and persistent effects have different handoff shapes

One-shot effects such as an impact, explosion, landing puff or teleport burst use the
existing confirmed-effect journal:

```text
simulation event -> quarantined message -> confirmed release -> presentation spawn
```

Persistent presentation such as “this machine is smoking”, “this body is burning”, or
“this room has embers” should **not** emit one message every tick. The authoritative or
read-model fact persists; presentation reconciles an emitter entity to that fact:

```text
authoritative/read-model fact -> presentation reconcile -> ensure/update/despawn emitter
```

The emitter remains disposable and non-rollback.

### A3 — do not grow `ParticleKind` into a content taxonomy

`ParticleKind::{Spark,Dust,Shard}` is existing compatibility surface. Do not answer new
content requests by adding `Smoke`, `Snow`, `Leaf`, `Magic`, `Blood`, `Bubble`, etc. to
a giant engine enum.

The long-term direction is semantic presentation intent plus presentation-owned
recipes. A gameplay producer should ideally say things like “metal impact at this
contact with this intensity” rather than prescribe 18 yellow squares at 320 px/s.
Migrate representative call sites only after the prototypes show what data the
resolver actually needs.

### A4 — do not design a universal `VfxBackend` trait before the spikes

Enoki, Hanabi, authored sprites, ribbons, afterimages, rings and the built-in fallback
are not guaranteed to be interchangeable implementations of one interface. Prototype
both particle engines behind ordinary Bevy plugins first. After the same gallery has
been implemented twice, extract only the common seam that is visible in real code.

A narrow particle-provider abstraction may emerge. A monolithic “backend that renders
every VFX concept” should not be assumed.

### A5 — backend dependencies point upward, never into simulation

The intended dependency direction is:

```text
ambition_vfx (headless vocabulary / semantic intent)
        ^
        |
ambition_render (built-in presentation)
ambition_vfx_enoki (optional presentation adapter)
ambition_vfx_hanabi (optional presentation adapter)
        ^
        |
app/demo composition chooses providers
```

The optional adapters must not become dependencies of simulation/core crates. They
must not depend on `bevy_ggrs`. Prefer the existing workspace-policy framework to pin
that boundary once the crates exist.

### A6 — optional means optional in the compile graph

The default Ambition game, headless simulation, minimal fixtures, and WebGL-oriented
browser paths must not acquire Enoki/Hanabi merely because the prototype crates exist.

For the prototype crates, use an opt-in Cargo feature for the external backend if
needed to keep ordinary `cargo check --workspace` from compiling heavyweight optional
closure. The VFX lab app should select the backend explicitly with mutually exclusive
features such as `builtin`, `enoki`, and `hanabi`; a normal game should not link all
three just to choose one at runtime.

The exact feature spelling may adapt to repository conventions, but the compile-graph
property is the requirement.

### A7 — quality is presentation policy

Semantic intensity is stable; visual fidelity is allowed to change with the resolved
quality budget. A hit remains the same hit on Potato and Ultra. The presenter may vary
particle count, rate, expensive materials, secondary layers and backend choice.

Do not put visual-quality profile branches into authoritative simulation code.

### A8 — reference frames are explicit

Particle motion must not silently assume screen-down or one privileged participant
frame. Effects that care about orientation should resolve against an explicit frame or
basis, e.g. world, source-local, or gravity-local. Dust rising away from a surface and
debris falling with gravity must continue to make sense under nonstandard gravity and
gravity-relative view policies.

The exact enum/type belongs to the implementation task that first needs it; the
behavioral invariant is fixed here.

### A9 — authored sprites and procedural particles compose

Do not force readable 2D action effects into a particle engine merely because one is
available. The preferred style for important actions is often:

```text
hand-authored animated impact / puff / flash
                    +
      procedural particles and motes
                    +
 optional camera/audio/debris companions
```

The sprite renderer remains the authority for the authored layer. Particle engines may
consume small generated sprite textures, but they do not replace the sprite pipeline.

### A10 — prototype evidence precedes production migration

The Enoki/Hanabi crates and VFX gallery are evidence-producing code. Do not convert all
production effects during the spike. Production migration begins only after the
comparison gate below selects a direction.

---

## Target topology for the spike

The following shape is intentionally concrete enough that agents can build it without
inventing a different experiment in each branch.

```text
crates/
    ambition_vfx/                  # existing headless vocabulary
    ambition_render/               # existing built-in renderer
    ambition_vfx_enoki/            # new optional Enoki adapter/proof crate
    ambition_vfx_hanabi/           # new optional Hanabi adapter/proof crate

game/
    ambition_demo_vfx/             # new backend-neutral gallery content/fixture
    ambition_demo_vfx_app/         # new visible host selecting exactly one backend

assets/vfx/                        # provider assets/recipes as appropriate
    enoki/
    hanabi/
```

If HEAD establishes a newer canonical location for demo content or provider assets,
follow that convention rather than creating a parallel one. Preserve the separation:
the gallery content must not import either third-party backend; the app/provider layer
may.

### Common gallery contract

`ambition_demo_vfx` should define a tiny presentation-neutral **prototype-only** cue
vocabulary so the two spikes are judged against the same scene without prematurely
changing the production `VfxMessage` API. Name it clearly as lab/demo vocabulary, for
example:

```rust
pub enum VfxPrototypeCue {
    RadialSparks,
    DirectionalSparks { direction: Vec2 },
    LandingDust { surface_normal: Vec2 },
    AmbientEmbers,
    ChargeInward,
    HeavyShowcase,
}
```

The exact Rust spelling may vary, but keep those semantic cases. `HeavyShowcase` exists
so Hanabi is tested on something that actually exercises GPU-oriented strengths; it is
allowed to be a conservative fallback in the built-in/Enoki presentations.

The lab must place each effect at stable world coordinates with a visible label or
other unambiguous identity, and provide a repeat/restart input so captures can be
compared from the same state. Do not use random wall-clock placement as the test.

---

## Execution map

The numbered tasks below are the unit of delegation. **One task is intended to fit one
focused agent session.** A task may land alone. Agents should not absorb later tasks
because nearby code is convenient.

```text
VFX-01  existing VFX plugin ownership
   |
VFX-02A gallery contract/content
   |
VFX-02B visible lab + builtin baseline
   |---------------------------|
   v                           v
VFX-03 Enoki shell        VFX-05 Hanabi shell
   |                           |
VFX-04A/B/C Enoki         VFX-06A/B/C Hanabi
   |---------------------------|
               v
        GATE G1 comparison
               |
               v
VFX-07 selected-provider production seam
   |
VFX-08 semantic impact migration
   |
VFX-09 quality-budget enforcement
   |
VFX-10 persistent emitters
   |
VFX-11 reference-frame correctness
   |
VFX-12 consolidation / fallback policy
```

AUTH-01A/AUTH-01B and AUTH-02 may run in parallel after VFX-02B.
AUTH-03 follows AUTH-01B and may land before or after G1.
VFX-13A/VFX-13B are independent after VFX-07 and should not block particles.

**Letters are assignment boundaries.** `VFX-04A`, `VFX-04B`, and `VFX-04C`, for
example, are three separate agent sessions/commits, not one task an agent is expected
to swallow whole. The same rule applies to the other lettered tasks below.

### Delegation table

The coordinator can hand these rows directly to agents. A row's **stop condition** is
where that agent stops even if later work is obvious.

| Unit | Depends on | One-session deliverable / stop condition |
|---|---|---|
| VFX-01 | — | Built-in VFX systems are plugin-owned; no visual/backend rewrite. |
| VFX-02A | VFX-01 | Backend-neutral cue/gallery content + deterministic station schedule; no visible host. |
| VFX-02B | VFX-02A | Visible lab + builtin baseline + capture invocation; no third-party crate linked. |
| VFX-03 | VFX-02B | Enoki adapter shell, minimal emitter, feature/dependency ratchets; no full gallery. |
| VFX-04A | VFX-03 | Enoki radial + directional one-shots only. |
| VFX-04B | VFX-04A | Enoki landing dust + ambient embers and persistent lifecycle only. |
| VFX-04C | VFX-04B | Enoki charge-inward + capture/evidence; no production migration. |
| VFX-05 | VFX-02B | Hanabi adapter shell, minimal 2D effect, capability/dependency ratchets. |
| VFX-06A | VFX-05 | Hanabi radial + directional + landing one-shots only. |
| VFX-06B | VFX-06A | Hanabi embers + charge-inward and lifecycle only. |
| VFX-06C | VFX-06B | Hanabi heavyweight showcase + capture/evidence; no production migration. |
| AUTH-01A | VFX-02B | New action-FX target + four impact rows + preview. |
| AUTH-01B | AUTH-01A | Five utility/locomotion rows + refreshed preview; no Rust integration. |
| AUTH-02 | VFX-02B | Generic particle-texture target + preview only. |
| AUTH-03 | AUTH-01B | Built-in production impact uses authored art with fallback. |
| G1 | VFX-04C + VFX-06C | **Coordinator only:** provider/platform decision recorded. |
| VFX-07 | G1 | Selected provider consumes one real production effect downstream of quarantine. |
| VFX-08 | VFX-07 | One combat hit path migrates from particle recipe to semantic cue. |
| VFX-09 | VFX-07 | Existing particle quality budget enforced with tests. |
| VFX-10 | VFX-07 | One persistent emitter reconciled from state/read model. |
| VFX-11 | VFX-07 | Landing/gravity particle semantics work in an explicit non-default frame. |
| VFX-12A | VFX-08..11 as applicable | Measured remaining-path classification + at most one last coherent migration. |
| VFX-12B | VFX-12A | Superseded particle/runtime spike code removed; fallback roles explicit. |
| VFX-13A | VFX-07 | Generic afterimages + one real use only. |
| VFX-13B | VFX-07 | Generic ribbon/polyline + one real use only. |
| FOLD | relevant completed/open rows | **Coordinator only:** merge surviving content into parent and delete this extension. |

Agents should report a blocker against the specific row they were assigned. They should
not self-promote to a later row to work around it, and should not mark sibling rows done
because their implementation happens to contain similar code.

---

# Session tasks

## VFX-01 — consolidate the existing built-in VFX presentation plugin

**Purpose:** establish honest ownership before third-party backends arrive. This is a
behavior-preserving refactor, not the particle rewrite.

**Primary touch points:**

- `crates/ambition_render/src/fx.rs` (or a new `fx/plugin.rs` if that is cleaner at
  current HEAD);
- `crates/ambition_render/src/lib.rs`;
- `game/ambition_app/src/app/plugins.rs`;
- focused render/app tests if required to pin registration and ordering.

**Implement:**

1. Add a reusable `BuiltinVfxPresentationPlugin` (name may be adjusted only to match a
   clearly established local naming convention).
2. Move registration of render-owned VFX systems out of `ambition_app` and into that
   plugin:
   - fireworks request fan-out / sequence ticking;
   - explosion request fan-out;
   - `vfx_spawn_messages`;
   - particle/explosion/impact/speech-bubble update systems that are owned by the VFX
     renderer.
3. Introduce presentation-owned system-set labels if needed to preserve ordering
   without making `ambition_render` depend on app-local systems. A useful shape is
   `Resolve -> Spawn -> Update`; use the smallest set vocabulary that makes real order
   constraints explicit.
4. Preserve the existing session-world guards and effect ordering.
5. Keep app-specific debug/HUD ordering in the app. Do **not** make
   `ambition_render` import `game/ambition_app` just to preserve an incidental
   `.after(debug_overlay::...)` edge. If that edge expresses a real requirement, order
   the app system against the new public VFX set instead.
6. Install the plugin from the Ambition app and any existing generic composition that
   should receive the built-in VFX renderer. Do not automatically add it to headless
   compositions.

**Acceptance:**

- existing Ambition VFX behavior is unchanged;
- the app no longer enumerates the built-in VFX system list;
- `ambition_render` owns the reusable plugin and public schedule labels it needs;
- no Enoki/Hanabi dependency exists yet;
- existing external-effect quarantine tests still pass;
- a focused plugin/composition test proves installing the plugin actually installs its
  consumer/update chain rather than relying on the Ambition app.

**Do not do in this task:** change `VfxMessage`, replace particles, add new art, or
introduce a backend trait.

---

## VFX-02 — build the backend-neutral VFX laboratory

**Purpose:** create one reproducible scene against which built-in, Enoki and Hanabi can
be judged. Without this, each prototype will optimize for a different effect and the
comparison will be mostly aesthetic anecdotes.

**This is two assignable sessions:**

### VFX-02A — gallery contract + backend-neutral content crate

One agent creates `game/ambition_demo_vfx`, the common `VfxPrototypeCue` vocabulary,
stable station coordinates/labels, and deterministic/restartable emission schedule.
It stops when the content crate has unit tests for the station list/timing and has **no**
provider dependency. It does not create the visible host.

### VFX-02B — visible lab host + builtin baseline

A second agent creates `game/ambition_demo_vfx_app`, composes the camera/reference
geometry, adds the mutually exclusive provider features, implements only the builtin
adapter, and establishes the capture/review invocation. It stops when the builtin lab
runs and its dependency tree contains neither Enoki nor Hanabi.

**Primary touch points:**

- new `game/ambition_demo_vfx/` content crate;
- new `game/ambition_demo_vfx_app/` visible app crate;
- root workspace manifest;
- small lab-only tests.

**Implement:**

1. Create a backend-neutral demo content crate that defines the common
   `VfxPrototypeCue` cases listed above and a stable gallery layout.
2. Create a visible demo app with an ordinary 2D camera and enough world reference
   geometry to judge scale and motion.
3. Give the app mutually exclusive compile-time provider features:
   - `builtin` (default is acceptable for the lab);
   - `enoki`;
   - `hanabi`.
   It must be possible to build the builtin lab without linking either third-party
   particle crate.
4. Implement the `builtin` gallery adapter using the current Ambition VFX primitives.
   It is a baseline, not a demand for feature parity.
5. Place stable labels/markers for:
   - radial sparks;
   - directional sparks;
   - landing dust;
   - ambient embers;
   - charge-inward;
   - heavyweight showcase location.
6. Provide a deterministic restart/replay trigger. A fixed timer sequence is fine;
   don't make comparison depend on uncontrolled randomness or manually timing five
   keypresses.
7. Keep the gallery presentation-only. It does not need GGRS just to demonstrate
   particles.

**Acceptance:**

- `builtin` runs with no Enoki/Hanabi dependency in its normal dependency tree;
- the six gallery stations are visually distinguishable;
- restarting reproduces the same emission schedule;
- the content crate itself has no dependency on `ambition_render`, Enoki, or Hanabi if
  a lower presentation-neutral dependency suffices;
- there is a simple capture/review command or documented invocation that an agent can
  use to produce a screenshot under `tmp/` for comparison.

**Do not do in this task:** redesign production `VfxMessage` or move production effects
to the gallery vocabulary.

---

## VFX-03 — add the isolated Enoki adapter shell and dependency ratchets

**Purpose:** prove Enoki can live where the architecture says it lives before spending
time authoring effects.

**Primary touch points:**

- new `crates/ambition_vfx_enoki/`;
- root workspace manifest;
- `game/ambition_demo_vfx_app/Cargo.toml` feature wiring;
- `tests/ambition_workspace_policy/policies/engine.toml` (or the current canonical
  policy file).

**Dependency target:** start from `bevy_enoki 0.6` for Bevy 0.18.x.

**Implement:**

1. Create `ambition_vfx_enoki` as a presentation-only adapter crate.
2. Keep the external dependency opt-in if necessary to prevent ordinary workspace
   builds from compiling it. A clean pattern is an empty default feature set plus a
   feature that activates `dep:bevy_enoki`; follow current workspace conventions if a
   better established pattern exists at HEAD.
3. Under the backend feature, expose an `EnokiVfxPlugin` that installs `EnokiPlugin`
   and the minimal Ambition-side adapter systems/resources needed by the lab.
4. Do **not** depend on:
   - `bevy_ggrs`;
   - `ambition_platformer2d_runtime` merely to learn about rollback;
   - actor-monolith/game-content crates;
   - `ambition_app`.
5. Add workspace-policy ratchets that pin the important dependency direction. At a
   minimum, the adapter must be forbidden from depending on `bevy_ggrs`,
   `ambition_platformer2d_runtime`, `ambition_platformer2d_actor_monolith`, and
   `ambition_app`; simulation/core manifests must not acquire an
   `ambition_vfx_enoki` dependency.
6. Wire the demo app's `enoki` feature to the adapter without affecting its builtin
   feature closure.
7. Add one minimal one-shot emitter only to prove the plugin and asset path work. Do
   not implement the full gallery yet.

**Evidence to record in the task/commit notes:**

- `cargo tree` for the builtin lab (must not contain `bevy_enoki`);
- `cargo tree` for the Enoki lab feature;
- native `cargo check` of the adapter with backend enabled;
- WASM `cargo check` if the target is available in the agent environment; if it is
  unavailable, say exactly that rather than claiming browser proof.

**Acceptance:** the Enoki feature compiles in isolation, builtin remains dependency-
clean, policy ratchets pass, and no Enoki type appears in simulation or rollback code.

---

## VFX-04 — implement the Enoki gallery

**Purpose:** evaluate Enoki as Ambition's everyday 2D particle provider.

**This is three assignable sessions:**

### VFX-04A — one-shot Enoki motion primitives

Implement only `radial_sparks` and `directional_sparks`, including one-shot cleanup and
lab reset. Stop after both stations are correct and their RON/assets are hot-reloadable
when the backend supports it.

### VFX-04B — Enoki surface/persistent primitives

Implement only `landing_dust` and `ambient_embers`. This session owns surface-relative
motion plus persistent-emitter lifecycle and proves restart does not duplicate the
continuous emitter.

### VFX-04C — Enoki inward motion + evidence pass

Implement `charge_inward`, then make the common gallery capture, record dependency/
runtime observations, and fix only issues that prevent a fair Enoki comparison. Do not
begin production integration in this session.

**Primary touch points:**

- `crates/ambition_vfx_enoki/src/...`;
- `assets/vfx/enoki/` (or the current provider-asset location);
- VFX lab adapter wiring.

**Implement the same five ordinary effects:**

1. `radial_sparks` — short one-shot burst, clear velocity variation, fast fade;
2. `directional_sparks` — cone around an authored direction/contact normal;
3. `landing_dust` — low, surface-relative spread with upward/away motion rather than
   a radial firework;
4. `ambient_embers` — continuous low-rate emitter, long enough lifetime to judge
   steady-state cost;
5. `charge_inward` — particles begin on a ring/region and move inward so the spike
   exercises something other than outward bursts.

Use Enoki's RON-authored effect configuration where it is a good fit. Keep material or
shader customizations modest in this task; the point is authoring workflow, 2D quality,
and integration, not winning the comparison with one bespoke shader week.

**One-shot vs persistent behavior:**

- one-shot stations should use Enoki's one-shot lifecycle rather than leaving inert
  emitter entities forever;
- ambient embers should be a persistent emitter controlled by presentation state;
- restarting the gallery must remove/recreate or reset provider state cleanly with no
  doubled emitters.

**Acceptance:**

- the five ordinary stations are implemented through Enoki;
- authored effect files hot-reload if Enoki's normal asset path provides that in the
  current build, or the task records why it is unavailable;
- no simulation message is emitted every frame to keep ambient embers alive;
- a captured gallery image and a short qualitative note identify what Enoki does
  better/worse than the builtin baseline;
- particle entities/resources are session/demo scoped so replay/restart leaves no
  stale emitter.

**Do not do in this task:** production migration, Hanabi comparison, or a universal
provider trait.

---

## VFX-05 — add the isolated Hanabi adapter shell and capability boundary

**Purpose:** prove Hanabi can be an optional high-end provider without becoming a
baseline platform requirement.

**Primary touch points:**

- new `crates/ambition_vfx_hanabi/`;
- root workspace manifest;
- VFX lab feature wiring;
- workspace dependency policies.

**Dependency target:** start from `bevy_hanabi 0.18` for Bevy 0.18.x, with default
features disabled and only the 2D capability enabled unless current 0.18 metadata
proves a smaller required set.

A representative manifest shape is:

```toml
bevy_hanabi = {
    version = "0.18",
    default-features = false,
    features = ["2d"],
    optional = true,
}
```

Adjust syntax to the real manifest layout, but preserve the narrow feature closure.

**Implement:**

1. Create `ambition_vfx_hanabi` as a presentation-only adapter crate.
2. Keep the external dependency opt-in for the same compile-graph reason as Enoki.
3. Under the backend feature, expose `HanabiVfxPlugin` and one minimal 2D effect.
4. Apply the same no-GGRS / no-runtime / no-actor / no-app dependency ratchets as the
   Enoki adapter.
5. Wire the lab's `hanabi` feature without linking Enoki or the builtin particle
   provider by accident.
6. Treat WebGPU compute as an explicit capability requirement. Do not “solve” Hanabi
   WASM by silently making Ambition's WebGL2 path require WebGPU.

**Evidence to record:**

- builtin lab tree contains no `bevy_hanabi`;
- Hanabi feature tree contains only the intended Hanabi/Bevy closure;
- native check succeeds;
- WASM compile result is recorded separately from browser runtime support;
- no claim of WebGL2 support is made for Hanabi's compute path.

**Acceptance:** isolated 2D Hanabi effect compiles/runs on the supported native path,
normal/default compositions stay clean, and the capability limitation is explicit.

---

## VFX-06 — implement the Hanabi comparison gallery plus one heavyweight showcase

**Purpose:** evaluate Hanabi on both the same everyday workload and the class of effect
for which a GPU particle engine could actually justify its dependency/capability cost.

**This is three assignable sessions:**

### VFX-06A — ordinary Hanabi one-shots

Implement `radial_sparks`, `directional_sparks`, and `landing_dust` only. Match the
common gallery semantics; do not add the heavyweight effect yet.

### VFX-06B — persistent/inward Hanabi effects

Implement `ambient_embers` and `charge_inward`, including clean restart/despawn of
provider state. Stop when the five ordinary stations are comparable to Enoki.

### VFX-06C — heavyweight showcase + evidence pass

Implement `HeavyShowcase`, capture the same gallery framing, and record approximate
particle counts plus available Bevy CPU/GPU/frame diagnostics. This session is where
Hanabi earns (or fails to earn) its high-end capability role.

**Implement:**

1. Reproduce the same five ordinary gallery effects from VFX-04 as closely as Hanabi's
   idioms reasonably allow.
2. Add `HeavyShowcase` as a sixth effect that would be unreasonable to use as the
   baseline CPU/entity-per-particle implementation. Good candidates include:
   - a high-count shockwave/energy storm;
   - a ribbon/trail-heavy burst;
   - a dense portal/magic effect with thousands of short-lived particles.
3. Keep the effect 2D and relevant to Ambition rather than demonstrating unrelated 3D
   features.
4. Preserve deterministic gallery timing even though the visual simulation itself is
   presentation-only and need not be deterministic across machines.
5. Ensure gallery reset destroys Hanabi effect entities/state cleanly.

**Acceptance:**

- all six stations render through Hanabi;
- a capture exists from the same camera/layout used for Enoki;
- the task records approximate particle counts and any obvious CPU/GPU/frame-time
  observations available from Bevy diagnostics;
- the note explicitly distinguishes “looks better because we authored it better” from
  a capability Hanabi uniquely or materially simplifies;
- no production gameplay path depends on Hanabi yet.

---

## GATE G1 — coordinator chooses the prototype outcome

This is a **decision gate**, not an invitation for the next implementation agent to
pick its favorite crate. The coordinator reviews VFX-03 through VFX-06 evidence and
records one of the following outcomes before VFX-07 starts:

1. **Enoki for ordinary 2D particles; builtin fallback; Hanabi reserved for optional
   heavyweight effects.** This is the expected shape if the prototypes validate the
   current hypothesis.
2. **Enoki only** because Hanabi does not justify its platform/dependency cost yet.
3. **Builtin + Hanabi** because Enoki does not materially improve authoring/runtime.
4. **Builtin only for now** because neither dependency earns a permanent place.
5. Another evidence-backed split, stated explicitly.

The decision must answer:

- authoring ergonomics (especially Enoki RON/hot reload);
- visual quality for ordinary 2D effects;
- ease of using generated sprite textures;
- native build/dependency cost;
- WASM/WebGL2/WebGPU implications;
- mobile implications if measured;
- cleanup/session lifecycle behavior;
- quality-budget control points;
- whether persistent emitters are straightforward;
- whether Hanabi's heavyweight result demonstrates a real use case rather than a tech
  demo.

⛔ Do not begin production migration while this gate is unresolved.

---

## AUTH-01 — author the generic action-FX sprite sheet

**May run in parallel with the backend spikes after VFX-02B. This is two assignable art
sessions so one sprite agent is never handed nine rows at once.**

### AUTH-01A — contact-impact rows

Create the target/publishing scaffold and author only `impact_soft`, `impact_hard`,
`impact_metal`, and `impact_energy`. Produce the normal contact-sheet preview and stop.

### AUTH-01B — locomotion/utility rows

On top of AUTH-01A, author `landing_puff`, `skid_puff`, `poof`, `glint`, and
`release_flash`; refresh the preview/parity products and stop. Do not integrate the art
into Rust in this session.

**Purpose:** improve the readable core of common actions independent of particle
provider choice.

**Primary touch points:**

- new
  `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/props/generic_action_fx.py`;
- renderer registry/discovery wiring required to publish the target;
- generated target tests/parity registration following `generic_explosions.py`.

**Author these rows:**

| Row | Intended read |
|---|---|
| `impact_soft` | compact asymmetric contact flower/flash |
| `impact_hard` | larger jagged strike with bright core |
| `impact_metal` | crossed sharp glints / ringing contact |
| `impact_energy` | round energetic flash with broken rim |
| `landing_puff` | low cartoon dust cloud expanding along surface |
| `skid_puff` | directional dust/smoke dragged opposite motion |
| `poof` | generic disappearance/spawn/transformation puff |
| `glint` | small four-point/star twinkle |
| `release_flash` | muzzle/ability-release flash with clear forward read |

**Authoring constraints:**

- mirror the established procedural target shape rather than creating a new tool;
- use Pillow + stdlib and the canonical alpha-safe `core.draw.overlay_draw`/
  compositing helpers;
- supersample (the explosion/slash targets use 4x) and downsample cleanly;
- transparent background, no baked world context;
- publish normal sheet + YAML compatibility sidecar + RON/actor metadata through the
  existing publishing path;
- keep the center/origin metadata explicit;
- make frame timing fast enough that an impact reads as punctuation, not as an actor
  animation; roughly 120–250 ms total per contact effect is a sensible starting range;
- prefer silhouettes that remain legible when scaled down; do not rely on dozens of
  1-pixel details;
- use authored asymmetry where it helps directional motion, but keep rows transformable
  by presentation rather than drawing separate left/right content without need.

A reasonable starting frame is ~96x96 for impacts/puffs, but follow the target's actual
containment needs rather than making that number a permanent API.

**Acceptance:**

- all nine rows publish through the existing sprite-renderer pipeline;
- renderer tests/freshness/parity mechanisms know about the target;
- a contact-sheet/gallery preview exists for visual review;
- no runtime code is changed in this task.

---

## AUTH-02 — author a tiny reusable particle-texture sheet

**May run in parallel with AUTH-01 and the backend spikes.**

**Purpose:** give Enoki/Hanabi/builtin particle layers Ambition-shaped source art rather
than forcing every particle to be a flat colored square.

**Create a separate procedural prop/VFX target** (for example
`generic_particle_textures.py`) with small transparent cells suitable for instancing.

At minimum publish:

- `soft_dot` — feathered/soft round mote;
- `spark` — compact pointed spark;
- `streak` — narrow velocity-oriented slash/streak;
- `smoke` — small soft irregular puff, optionally a few animation frames;
- `ember` — warm irregular fleck;
- `star` — tiny glint/star;
- `dust_chip` — matte irregular chip/mote.

Keep these deliberately generic. They are texture primitives; `metal_spark` versus
`magic_spark` belongs in recipe/material/color/motion unless the art really differs.

**Acceptance:**

- target publishes through the existing renderer contract;
- cells have transparent backgrounds and sensible origins;
- at least Enoki can consume the generated texture/spritesheet without a bespoke image
  conversion step once VFX-04/AUTH-02 are both present;
- preview shows each primitive at native scale and enlarged nearest-neighbor scale for
  review.

---

## AUTH-03 — replace the built-in placeholder impact with authored action art

**Depends on AUTH-01; independent of the particle-backend decision.**

**Purpose:** prove the authored-sprite + particle composition pattern in production.

**Primary touch point:** `crates/ambition_render/src/fx.rs` and normal asset lookup.

**Implement:**

1. Replace the current expanding yellow square used by `ImpactVisual` with the relevant
   `generic_action_fx` animation.
2. Preserve a no-assets/headless fallback so missing visual assets never become
   gameplay failure.
3. Keep the animation/session lifecycle entirely presentation-side.
4. Do not remove secondary particles; the point is to layer readable authored contact
   art with procedural motes.
5. If the current `VfxMessage::Impact` lacks material/intensity information, use the
   generic/default impact row for this task. Material-aware semantic migration belongs
   to VFX-08.

**Acceptance:** an ordinary impact visibly uses authored animation in the normal game,
no-assets paths still work, and no simulation dependency changes.

---

## VFX-07 — integrate the selected particle provider behind a production presentation seam

**Blocked on G1.**

**Purpose:** turn the selected spike into production-capable presentation without
pretending every VFX layer is one backend.

**Implement:**

1. Keep the provider crate separate. Do not move Enoki/Hanabi types into
   `ambition_vfx`.
2. Define the smallest production-facing adapter shape proven by the gallery. It may be
   a plugin plus provider-owned systems/resources; it does **not** need to be a trait.
3. Make the host/composition choose which particle provider is installed. The built-in
   provider remains available until consolidation proves it can be removed.
4. Consume confirmed production VFX only **after** the external-effect quarantine.
   Never create a second rollback journal inside the provider.
5. Ensure session reset/world replacement despawns provider-owned emitters and
   particles through the existing session-scoping mechanism or an equivalent
   presentation lifecycle.
6. Add a structural/policy test ensuring adapter crates remain absent from sim/core
   dependency graphs.
7. Keep Hanabi capability-gated if selected only for heavyweight effects. A WebGL2
   composition must have a valid path that does not link or require it.

**Acceptance:** one real production effect can be presented by the selected provider
without changing simulation semantics, and the default/headless dependency closure is
still clean.

---

## VFX-08 — migrate one representative low-level burst to semantic impact intent

**Depends on VFX-07.**

**Purpose:** prove the direction away from renderer-shaped simulation messages before
attempting a broad message rewrite.

Use **combat hurt/impact feedback** as the representative slice because current
`HitBurst` carries exactly the leakage this task is meant to remove: particle count,
speed, color and `ParticleKind` are chosen upstream.

**Implement:**

1. Introduce a semantic presentation cue for a resolved contact. It should carry only
   information gameplay/read models genuinely know, such as:
   - world/contact position;
   - contact normal or direction when available;
   - `ImpactMaterial` / victim material family;
   - a small semantic intensity/scale value if gameplay has a real basis for it.
2. Do **not** put particle count, backend type, Enoki asset path, Hanabi effect handle,
   color palette, or spawn rate into this cue.
3. Have presentation resolve the cue into:
   - authored impact row (`impact_soft`, `impact_metal`, etc.);
   - provider particle recipe;
   - optional existing debris/audio/camera companions where already owned elsewhere.
4. Migrate one complete hit-feedback path to the semantic cue and delete its now-unused
   low-level `HitBurst` data if the migration leaves that data unreferenced.
5. Leave unrelated legacy `VfxMessage::Burst` call sites alone. They form an explicit
   migration backlog rather than a reason to sweep the whole repository in one task.
6. Extend `effect_quarantine` classification/tests if the new message family is a new
   presentation-facing message rather than a variant of an already quarantined family.

**Acceptance:** the chosen combat hit no longer encodes a particle recipe in
simulation-facing data, visual behavior remains rich, and rollback still releases the
presentation intent exactly once.

---

## VFX-09 — enforce `ParticleBudget` in the presentation layer

**Depends on VFX-07; may follow VFX-08 or run in parallel if file overlap is managed.**

**Purpose:** make the existing quality policy real and backend-independent in meaning.

**Implement:**

1. Resolve `VisualQualityBudget.particles` at the particle-provider boundary.
2. Apply `spawn_rate_scale` to burst counts and continuous emitter rates in
   presentation. Simulation must emit the same semantic event at every quality level.
3. Enforce `max_particles` as an actual active-particle ceiling for the provider's
   presentation world/session. If the backend provides its own capacity control, adapt
   to it; otherwise maintain a small provider-owned budget/accounting resource.
4. When budget is exhausted, degrade gracefully:
   - preserve the authored/core sprite effect first;
   - reduce or omit secondary motes;
   - never block gameplay or queue unbounded deferred particles.
5. Apply budget changes live if `ResolvedVisualQuality` changes during a session.
6. Add focused tests for the extreme profiles (Potato and Ultra) that assert counts or
   provider capacity, not screenshots alone.

**Acceptance:** the same semantic effect produces lower/higher particle fidelity under
Potato/Ultra, active counts respect the cap, and no authoritative state differs between
profiles.

---

## VFX-10 — add a persistent-emitter reconciliation path

**Depends on VFX-07.**

**Purpose:** support smoke, embers, auras, weather and status visuals without per-tick
message spam.

**Implement one representative persistent effect**, preferably ambient embers in the
VFX lab first and then one real room/body use if there is an obvious current producer.

The presentation shape should be approximately:

```text
read-model / authored presentation fact
        |
        v
reconcile persistent VFX anchors
        |
        +-- spawn emitter if missing
        +-- update transform/parameters if changed
        +-- despawn when fact/session disappears
```

**Requirements:**

- provider emitter entity/state is presentation-only;
- stable source identity prevents duplicate emitters;
- session/world changes clean up automatically;
- pausing or disabling an effect does not require destroying/recreating it every frame
  if the backend has a cheap activation state;
- no one-shot quarantine message is emitted every tick merely to maintain it;
- a rollback correction that changes the authoritative/read-model fact is reflected by
  ordinary reconciliation on the resulting presentation state.

**Acceptance:** a persistent effect survives normally, follows/removes its source, and
never multiplies on restart/rollback/session replacement.

---

## VFX-11 — make particle orientation/reference-frame semantics explicit

**Depends on a production particle provider; can be done before or after VFX-10.**

**Purpose:** remove the fixed-world-down assumption from new VFX before it becomes a
large compatibility surface.

**Implement:**

1. Introduce the smallest presentation-side representation needed to distinguish
   effects that live in:
   - world frame;
   - source/body local frame;
   - resolved gravity/surface frame.
2. Do not infer a privileged human-player or screen frame.
3. Migrate landing dust and at least one gravity-affected particle recipe so:
   - dust spreads along the contacted surface and rises away from it;
   - debris/particles configured to “fall” follow the relevant resolved gravity rather
     than hard-coded positive world Y.
4. Ensure camera rotation/gravity-relative camera policy does not alter simulation
   semantics; it is presentation transformation only.
5. Add value-level tests for basis/velocity transformation independent of the particle
   backend where practical.

**Acceptance:** the representative effects remain semantically correct under a rotated
or non-default gravity frame without screen-space special cases.

---

## VFX-12 — consolidate providers and retire superseded particle code

**Depends on G1 plus enough production use to know what is actually redundant. This is
two assignable sessions.**

### VFX-12A — measured retirement plan + last ordinary migration

Re-measure every production caller of `ParticleVisual`/`spawn_burst` and the selected
provider. Classify each use as selected-provider, required builtin fallback, or dead/
transitional. Migrate at most one coherent remaining ordinary family needed to make the
classification actionable. Commit the measured classification in code comments/tests
or the coordinator's evidence note; do not perform a giant deletion yet.

### VFX-12B — delete superseded implementation and spike scaffolding

Using VFX-12A's classification, remove only code with no supported consumer, tighten
feature/dependency ratchets, and remove/archive lab-only glue that has been replaced by
durable tests/workbench code. Stop once there is one clear ordinary-particle path and
any remaining fallback has an explicitly named supported composition.

**Purpose:** avoid ending with three permanent implementations of every ordinary
particle effect.

**Implement:**

1. Re-measure real production usage of `ParticleVisual`, `spawn_burst`, provider
   adapters, and fallback paths.
2. Keep a built-in fallback only where it has a concrete supported composition or
   platform role. If it has no remaining user, delete it rather than maintaining a
   parallel engine indefinitely.
3. If Enoki is the ordinary provider, move ordinary smoke/dust/sparks to it and keep
   authored action sprites separate.
4. If Hanabi is optional-high-end only, give every Hanabi-backed semantic effect a
   graceful non-Hanabi presentation. The fallback may be simpler; gameplay behavior
   must be identical.
5. Remove temporary lab-only adapter code that has been replaced by production paths.
6. Keep the VFX lab itself if it remains useful as a visual regression/workbench; if it
   is only spike scaffolding, archive/remove it after captures and tests have moved to a
   durable home.

**Acceptance:** one clear ordinary-particle path exists, optional high-end capability
is genuinely optional, and transitional code is not retained merely because it once
helped the spike.

---

## VFX-13 — extract generic trails/afterimages as a separate presentation facility

**Independent after VFX-07; do not block particle adoption on it. This is explicitly
two sessions.**

### VFX-13A — afterimage presentation

Implement only the bounded presentation-owned pose/sprite history and one real dash or
blink use. Stop once lifecycle, quality cap, and session cleanup are proven.

### VFX-13B — ribbon/polyline presentation

Implement only the generic ribbon/polyline visual primitive plus one real use, with a
continuity-break mechanism so teleports/portals cannot draw false connecting segments.
Stop before attempting to convert the gameplay/topological `PlayerTrail` mechanic.

**Purpose:** cover motion effects that are structurally better represented by retained
geometry/poses than by particle clouds.

Ambition already has gameplay-specific emitted trail machinery in
`avatar/trail.rs`. Do **not** blindly reuse its authoritative/topological `PlayerTrail`
state as the generic VFX abstraction; that trail is a gameplay mechanic with its own
meaning. Instead, inspect its render/sample utilities for reusable presentation ideas.

**Implement in two small sub-slices if needed:**

1. **Afterimage presentation**
   - presentation-owned snapshots of a subject's presented sprite/pose/transform;
   - bounded count/lifetime;
   - useful for dash/blink/fast movement;
   - disposable and non-rollback.
2. **Ribbon/polyline presentation primitive**
   - width/lifetime/alpha (and texture only if evidence warrants it);
   - explicit world/source frame;
   - discontinuity/break support so teleports/portals do not draw a false segment.

If Hanabi has been selected for some ribbon-heavy effects, the generic semantic trail
request may map to Hanabi there; do not make the generic request type itself a Hanabi
type.

**Acceptance:** at least one afterimage and one ribbon use the generic presentation
facility, and neither takes gameplay authority from existing mechanics.

---

# Additional implementation rules

## Confirmed-effect quarantine: what must and must not change

The provider adapters should normally require **zero changes** to the journal algorithm.
The journal holds Ambition's message intent; providers consume the released messages.

A change to `external_effects.rs` is justified only when a new presentation-facing
message family is introduced. In that case:

1. add it to `quarantine_presentation_effects`;
2. add the abandoned-branch discard installation;
3. extend `effect_quarantine.rs` to prove exactly-once behavior;
4. preserve the invariant that the journal itself is host bookkeeping and is not
   rollback state.

Do not quarantine provider-internal emitter messages merely because they are called
“effects”. Provider-internal presentation systems run downstream of the boundary.

## Presentation reproducibility is optional and separate from network determinism

Particles do not need bit-identical positions across peers. Quality tiers may render
different particle counts. GPU particle integration may vary slightly across hardware.
That is legal because the particles do not feed simulation.

If visual regression/capture tooling benefits from stable randomness, derive an
optional presentation seed from a stable effect/event identity. Treat that as a capture
and debugging feature, not a GGRS requirement.

## Multiview policy

Effects exist in world presentation unless explicitly view-local. A shared world impact
should not be simulated once per local camera just because two views are active. The
renderer may draw the same effect into multiple views, but emitter ownership should not
accidentally multiply with camera count.

View-local effects (screen flash, per-view distortion, camera-local HUD particles) need
an explicit view target and belong with the view-local presentation policy in the main
render roadmap.

## Assets and hot reload

Provider recipes and particle textures are presentation assets. Use normal Ambition
asset paths/handles and session lifetime. Do not put third-party provider handles into
simulation state or authored gameplay schemas.

For Enoki, prefer its RON recipe format during the spike because authorability/hot
reload is part of what is being evaluated. For Hanabi, follow the crate's idiomatic
effect-asset construction; do not build a parallel serialization format solely to make
its spike resemble Enoki.

## Dependency and platform checks

Each adapter task should inspect **actual** feature closure. A dependency being
presentation-only in our source does not prevent it from enabling large default Bevy
features transitively.

At minimum check:

- adapter's direct manifest feature set;
- `cargo tree -e features` for the selected demo feature;
- absence of the provider from builtin/headless dependency trees;
- WASM compile where available;
- Hanabi's WebGPU-only compute runtime constraint separately from “Rust compiles for
  wasm32”.

Do not add Node/NPM tooling to this campaign. Enoki's optional editor is a Rust/Cargo
tool if an agent wants to evaluate it; using the editor is not required for the first
integration.

---

# Visual targets: what “more VFX variety” means

This program is successful only if the engine work produces a visibly broader game,
not merely cleaner crates. The reusable effect vocabulary should eventually cover at
least these presentation families:

### Contact and combat

- soft/hard/material-aware impacts;
- authored melee slash/core art;
- directional contact sparks;
- energy hit flashes;
- breakable shards/debris companions;
- shield/parry rings or arcs where the mechanic asks for them.

### Locomotion

- landing dust;
- skid/start dust;
- wall-kick/contact puff;
- dash/blink afterimages;
- short movement streaks/ribbons;
- water/snow/sand contact variants when those surfaces exist.

### Spawn/despawn/transform

- poof;
- glint/twinkle;
- inward charge;
- outward release;
- teleport departure/arrival layering;
- transformation aura or persistent emitter.

### Projectiles and abilities

- muzzle/release flash;
- projectile trail/streak;
- impact/endcap;
- charge particles;
- beam/energy motes;
- ring/arc primitives when a mesh is cleaner than particles.

### Ambient room identity

- embers/ash;
- dust motes/pollen;
- snow/rain splashes;
- bubbles;
- leaves;
- magical/math motes;
- smoke/steam from props.

The list is a vocabulary target, **not** an instruction to add a gameplay enum variant
for every noun.

---

# Whole-program definition of done

The coordinator should not mark this extension absorbed/complete until the parent
roadmap can state all of the following truthfully (or explicitly record an evidence-
backed decision not to pursue a candidate):

1. Reusable built-in VFX registration is plugin-owned rather than an app-local system
   census.
2. Presentation-facing one-shot effects still cross the confirmed-frame quarantine
   exactly once under rollback.
3. Third-party particle-engine state is never rollback-registered and no provider
   dependency leaks into simulation/core crates.
4. Enoki has been evaluated in an isolated Bevy-0.18-compatible adapter against a
   repeatable Ambition VFX gallery.
5. Hanabi has been evaluated in an isolated 2D adapter against the same gallery plus a
   workload that actually exercises GPU-heavy strengths.
6. A coordinator decision records which provider(s), if any, are permanent and on
   which platforms/capability tiers.
7. The default/headless/WebGL-oriented dependency closures do not accidentally acquire
   optional particle engines.
8. `VisualQualityBudget.particles` meaningfully controls production particle fidelity
   and active-particle capacity.
9. At least one renderer-shaped simulation particle recipe has been migrated to a
   semantic presentation cue, proving the long-term direction without demanding a
   repository-wide flag day.
10. Persistent emitters are reconciled from state/read models rather than produced by
    per-tick message spam.
11. New gravity-sensitive VFX have explicit reference-frame semantics and do not assume
    a privileged screen/player frame.
12. Authored generic action FX (impacts/puffs/poof/glint/release flash) exist in the
    established Python sprite-renderer pipeline.
13. Tiny reusable particle art exists if the selected provider benefits from it.
14. At least one production impact composes authored sprite art with procedural
    secondary particles.
15. Transitional particle implementations have been consolidated so ordinary effects
    do not have multiple permanent backends without a real supported reason.
16. The parent `render-animation-and-vfx.md` contains the surviving architecture,
    provider decision, remaining backlog and open questions.
17. **This extension file is deleted after that fold-back.** Keeping both documents
    indefinitely is a planning defect, not “extra documentation”.

---

# Coordinator fold-back instructions

When the parent roadmap is ready, the coordinator should integrate this extension by
**semantic section**, not by pasting it wholesale at the bottom.

Recommended fold-back map:

- parent **Goal / Program areas** ← authored action FX + layered VFX thesis;
- parent **Candidate crate / Bevy shape** ← plugin ownership, optional adapter crates,
  dependency direction, provider/capability decision;
- parent **simulation/presentation authority warning** ← explicit no-GGRS provider
  state + confirmed-effect handoff;
- parent **quality policy** ← `ParticleBudget` behavior and platform tiers;
- parent **open design questions** ← only questions that remain after G1 and semantic
  migration evidence;
- parent **execution phases** ← unfinished VFX-N/AUTH-N tasks, renumbered to the
  parent's current scheme;
- parent **status/evidence** ← completed spike measurements and selected providers.

The coordinator must resolve conflicts against the **newer parent text** rather than
blindly preferring this extension. Preserve concrete measured evidence and architectural
invariants; discard obsolete filenames/task numbers when HEAD has superseded them.

After the fold-back:

1. verify every still-open task has a home in the parent or another explicitly owning
   roadmap;
2. verify the provider decision and rollback boundary are stated once, not twice;
3. update cross-links that pointed here;
4. delete `render-animation-and-vfx-extension.md` in the same consolidation change.

That deletion is the final acceptance signal that this temporary extension has served
its purpose.
