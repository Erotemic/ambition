# The character definition — one authored unit

**Status:** DESIGN SETTLED, nothing built. Opened 2026-07-26 after two review
rounds with GPT-5.6 and Jon. §§4–6 are decided; §7 is the work queue; §8 records
what is deliberately deferred.

**Goal (Jon's words):** *"plop Mary-O or Sanic into Ambition and have them behave
similarly to how they do in their standalone game"* — author movesets, VFX/SFX
triggers, hitboxes and hurtboxes at the **character level**, then connect the
result to the engine.

**Forcing function:** a Smash-style versus mode, soon. Several characters from
several providers, live in one session, all player-drivable.

> **Reading this cold?** §2 is the diagnosis, §3 is every verified code fact with
> a location (do not re-derive these), §4–6 are the settled decisions, §7 is what
> to build and in what order, and each §7 item says whether it can run in
> parallel with the others. §0 is the current state — read it first.

---

## 0. Message to the next agent

> **This section is a baton, not a record. Overwrite it when you hand off.**
> Everything below it is the durable design; this is only "where things stand
> and what I would tell you in person." Rewrite it wholesale — do not append.

*Written 2026-07-26 by Claude Opus 5, after landing the binding-boundary work
and settling this design across two GPT-5.6 review rounds. Suite was 18/18 green
at commit `984d88062`, confirmed independently by Jon.*

**Nothing here is built.** The design is settled; the code is untouched.

### Two things that are not obvious from the rest of this document

**1. §7.2 is the point, and it is the one you will skip.**

Every bug that opened this investigation is one defect class: *an engine step
written in an application crate, so every other app silently loses it.* Three
instances, all found in a single session — sprites (§2), music (§3.6), movement
inheritance (§3.4). §7.1 fixes the sprite instance and makes Mary-O playable
again, which will feel like the job is done. **It is not.** After §7.1 nothing
is visibly broken, so §7.2 — the engine-owned readiness invariant — reads like
polish and gets deferred. It is the only item that stops the class from
recurring, and the class has a 3-for-3 record. Jon's framing: *"the only thing
that should change is what quit to title goes to."*

**2. §7.3 is free right now, and that expires.**

Making `inherits` strict and provider-local is a no-op migration *because zero
`.ron` files author `inherits` today* (§3.4 — grepped, confirmed). The moment
anyone authors a template — a Smash "heavy body" archetype is the obvious first
one — the strict policy stops being free and starts being a content migration.
Do it while it costs an afternoon and nothing else.

### How to start

- **Solo, want the game playable:** §7.1, then §7.2. Do not stop between them.
- **Handing work to a second agent (GPT patch or otherwise):** §7.3, §7.4, and
  §7.5 are parallel-safe — disjoint files, no dependency on the character work,
  each independently valuable. §7.5 (the music fork) is the one Jon has already
  asked for by name and has not received.
- **Before writing any code:** read §3. Those facts cost real time to establish
  and every one of them was checked against the source. Several contradict what
  the surrounding comments claim.

### What a reviewer got wrong, so you weigh the next one correctly

GPT-5.6 was right about most of this and corrected me twice (see §4.3's
correction note). But it twice reasoned confidently from an assumption the code
disproved — the audio retrofit is far smaller than it argued, because
`ProviderSfxHandleCache` is already source-qualified (§3.5). **Verify before
conceding or rejecting.** Its file/line claims were reliable; its inferences
about scale were not.

---

## 1. What already exists (more than it feels like)

The move model is not the gap. `MoveSpec`
(`crates/ambition_entity_catalog/src/lib.rs:386`) is already the professional
shape — close to 1:1 with Unreal's `AnimNotify` / `AnimNotifyState` +
GameplayCue split, and with fighting-game move scripts (Street Fighter's
BAC/BCM, ArcSys per-frame data):

| Ambition | Industry equivalent |
|---|---|
| `duration_s` in the owner's **proper time** | move clock, not animation-driven |
| `windows: Vec<MoveWindow>` + `WindowTag` | startup / active / recovery frame data |
| `MoveWindow.volumes: Vec<HitVolume>` | `AnimNotifyState` gating a collider |
| `MoveEvent { at_s, Sfx{cue} \| Vfx{effect} }` | `AnimNotify` / GameplayCue |
| `motion_scale` per window | committed-move motion lock |
| `gates`, `start_impulse`, `smash_charge_mult` | cancels, lunge, charge payoff |
| `MoveSpec::presentation_problems()` | *ahead of the baseline* — Unity/Unreal let a mistyped notify fail silently forever |

Two decisions were already made correctly, both of which teams commonly get
wrong: the **sim clock is authoritative** rather than the animation, and
presentation cues pass the **external-effect quarantine** so rollback cannot
re-fire them.

## 2. The diagnosis: a character has no name, and no owner

A character is declared through **six seams, keyed three different ways**, and
none of them is the character:

| Seam | Keyed by | Carries |
|---|---|---|
| `register_character_catalog_fragment` | `character_id` | identity, sheet spec, variant tuning |
| `register_character_roster_fragment` | **`brain_id`** | `CharacterArchetypeSpec` — health, speeds, mass, movement patch |
| `register_audio_catalog_fragment` | **`provider_id`** | which cues are authorized |
| `register_world_item_art` | sprite id | pickup art |
| `register_projectile_visual` | projectile id | shot art |
| *(no registration at all)* | — | **art materialization**, hand-written in the host app |

That last row is a live bug: **Mary-O renders as a colored rectangle in her own
standalone app** and correctly in the multi-game host, because the step that
turns a declared character into a loaded sheet lives in
`game/ambition_app/src/app/world_flow/room_transition_assets.rs` — an
*application* crate. `ambition_demo_mary_o_app` never runs it;
`ambition_demo_sanic_app` hand-rolled a duplicate.

**This is a class, not an incident.** The same shape was found three times:

1. **Sprites** — materialization in the host app; one demo lost it, one copied it.
2. **Music** — two entirely different implementations (§3.6). Standalone can
   never play victory or death music; the host can, but loops a one-shot sting.
3. **Movement inheritance** — resolved across the *merged* roster, so a
   character can inherit a parent that exists only when another provider is
   loaded, and silently degrade to baseline when it isn't (§3.4).

Nothing fails when two applications compose the engine differently. That is the
root cause behind all three, and §7.2 is the fix.

Unity and Godot split their data the same way we do — prefab ≠ animation ≠
ability asset. What they have that we do not is **one aggregation point** the
author edits as a single thing. That is the whole ask.

## 3. Verified code facts — do not re-derive these

Every claim below was checked against the source on 2026-07-26.

### 3.1 The privileged four
- `EAGER_CHARACTER_IDS = ["player", "robot", "goblin", "sandbag"]` —
  `crates/ambition_actors/src/character_sprites/assets.rs:277`.
- `CharacterSpriteAssets` has hardcoded `player` / `robot` / `goblin` /
  `sandbag` fields and a `deferred_npcs` map —
  `crates/ambition_sprite_sheet/src/character/assets.rs:12-33`.
- `asset_for_character_id` is a hardcoded `match` on those four names, falling
  through to `npcs.get(id)` — same file, `:71-79`. **A deferred character
  returns `None` here, indistinguishable from "no such character."**
- **Blast radius is small:** 5 external uses of the privileged slots, two of
  them test fixtures (`crates/ambition_render/src/rendering/actors/worn_binder_tests.rs:35,36,88`,
  `game/ambition_app/src/app/player_clone.rs:154`,
  `game/ambition_app/src/app/world_flow/room_transition_assets.rs:356`).
- `actor_fallback_asset(is_sandbag, fighting)` has **exactly one consumer** —
  `crates/ambition_render/src/rendering/actors/mod.rs:556`. It borrows the
  **goblin** sheet for any fighting actor with no sheet of its own.
- The other fallback is the marked colored-rectangle placeholder, which is what
  a worn character with no sheet already gets.
- Sanic's hand-rolled materializer: `game/ambition_demo_sanic_app/src/lib.rs:479`.
- `deferred_npcs` is consumed by **nothing else in the workspace**.

### 3.2 Attack intent gaps (block Smash)
- `AttackDir` is `Neutral | Up | Down | Back` — **no `Forward`** —
  `crates/ambition_entity_catalog/src/lib.rs:681`.
- `attack_dir_from_axis` sends *forward* input to the same `else` branch as *no*
  input → `Neutral` — `crates/ambition_combat/src/moveset/mod.rs:600-616`. So
  **jab ≡ ftilt and nair ≡ fair are the same value**, not merely unresolved.
- `ControlFrame` has `attack_pressed` and **no held/released** —
  `crates/ambition_engine_core/src/control_frame.rs:64`.
- Special is non-directional: `move_for_verb("special")` —
  `crates/ambition_actors/src/action_scheme.rs:235`.
- `directional_verb_chain` resolves **most-specific-first with fallback to the
  base verb**, so adding `Forward` is purely additive: existing movesets keep
  resolving to `attack`, and `attack_forward` is a new optional verb id.
- `ControlSlot` already has Jump / Attack / Special / Projectile / Dash / Blink /
  Interact / Utility / QuickAction / Modifier, with `ActionGate` binding each to
  a movement action, a technique, or a moveset verb —
  `crates/ambition_entity_catalog/src/action_scheme.rs:38`. **This is
  structurally Smash's model already** (`A + direction` is one slot plus
  resolution, not sixteen slots). The vocabulary is right; the resolver is not.

### 3.3 Volume shapes already exist
- `VolumeShape { Rect { offset, half_extents } | Circle { offset, radius } }` —
  `crates/ambition_entity_catalog/src/lib.rs:217-225`.
- `ambition_engine_core::VolumeShape { Box, Obb }` —
  `crates/ambition_engine_core/src/volume_shape.rs:32`.
- So the "rectangles only, or circles too?" question is already answered:
  **reuse `VolumeShape` for hurtboxes.** Rectangle-only would be an asymmetry
  between two volume kinds that get tested against each other, and Sanic in ball
  form wants a circle today.

### 3.4 Movement inheritance is silently composition-dependent
- `resolve_movement_inheritance` — `crates/ambition_actors/src/features/enemies/mod.rs:600`.
  Its own doc: *"a missing parent or a cycle falls back to the baseline rather
  than panicking (a malformed `inherits` is a data smell, not a crash)."*
- It runs **after all provider fragments are merged** (`:796-804`), so
  inheritance is an unqualified lookup across the global roster. A Mary-O
  archetype can inherit an Ambition parent in the host and lose it standalone.
- An `owners: brain_id → provider_id` map **already exists** at `:797`, so
  enforcing a policy is a filter, not new bookkeeping.
- **Zero content files author `inherits`.** Grepped `.ron` across `crates/` and
  `game/`: no hits. The strict policy is free — no migration.
- The `BASELINE ← patch` fold *is* live and used; only the parent-chain half is
  unexercised.

### 3.5 Audio provenance
- `ProviderSfxHandleCache` is keyed `(provider_id, SfxId)` — the resolution
  table is **already source-qualified** — `crates/ambition_audio/src/render.rs`.
- The single point of loss: `audio_play_sfx_messages` takes `provider_id` from
  `selection.provider_id()` (the one active provider), not from the request.
- `OwnedSfxMessage { owner, request }` carries session ownership + cue + position
  and **drops the emitter** — `crates/ambition_sfx/src/message.rs`.
- So the retrofit is: put the emitting source on the message, resolve the
  provider from it. Much cheaper than it first appeared.
- `PreparedContentIdentity { fingerprint_schema, fingerprint, snapshot_schema,
  epoch }` — `crates/ambition_runtime/src/content_identity.rs:345`. Both
  fingerprint and epoch already exist; the character layer composes with this
  rather than inventing a parallel generation counter.

### 3.6 The music fork (unfixed, separate from characters)
- Standalone demos run `drive_selected_session_music`
  (`crates/ambition_audio/src/music/mod.rs:84`): plays `music.default_track`,
  hardcoded `.looped()`, only when `selection.is_changed()`. It **never reads
  `EncounterMusicRequest`**, so the priority tier does not exist in that build —
  victory and death music can never play. The standalone app also never inserts
  `MusicPlaybackState`.
- The host runs the real director via `game/ambition_app/src/app/scene_setup.rs:101-117`.
- `music_registry.ron:59` declares `mary_o_flag_victory` with no `one_shot`
  field → `#[serde(default)]` → `false` → `.looped()`. The fanfare repeats.
- `scripts/regen_music_registry.py` docstring: *"A registry entry is
  intentionally trivial: just `id` + `display_name`"* — that decision is what
  drops the flag.
- The score YAML already knows: `mary_o_flag_victory.music.yaml:76` has
  `loopable: false`. ⚠ It is **per-section**, not per-track (51 scores use it,
  long themes have mixed values), so deriving "track is a sting" from it is
  inference. Prefer an explicit top-level field on the two sting scores.
- The victory claim is held from flag-grab through slide → walk-off → `Tallied`
  → the 2s `LEVEL_CYCLE_DWELL`, released exactly when the level replays. So
  "silence for the rest of the level" is already correct — only the looping is
  wrong.

## 4. Settled decisions

### 4.1 One registration, decomposable data
`register_character(definition)` is the single seam. The data is **not** one
monolithic blob: sheets, portraits, movesets, and gameplay numbers have different
load times, headless requirements, replacement frequencies, and sizes.

Any substantial section may be inline **or** an explicit reference to another
typed document. **Includes resolve to complete typed values.** No generic deep
merging, no partial overlay, no implicit list-replacement or precedence rules —
the same principle as the binding boundary and for the same reason: implicit
precedence is a silent failure waiting to happen.

Two shapes, one pipeline:

```
authored manifest  (decomposable, human-readable, may reference)
        |  prepare_character(...)   -- validates and flattens
        v
PreparedCharacterDefinition  (immutable, no inheritance, no string search
                              in authoritative gameplay paths)
```

⛔ **`register_character` must not become six registries behind one function.**
A compatibility bridge that populates the existing catalogs is fine during
migration; the end state is one prepared authority with subsystem read models
derived from it. Otherwise this improves ergonomics and keeps the consistency
problem.

### 4.2 Layout: a directory per character
```
characters/
    mary_o/
        character.ron
        moveset.ron
        hurtboxes.ron
        presentation.ron
        sprites/...
```
The provider keeps compiling these in (`include_str!` or ordinary
registration). Define the serialized manifest and relative-reference rules now;
change the loader only when there is a concrete modding or hot-reload
requirement. Generated output is committed text (like `music_registry.ron`), and
regen must work on a fresh clone.

### 4.3 Crossover characters are separate, generated products
`mary_o` and `mary_o_smash` are two independent, fully-resolved definitions with
distinct stable ids, emitted by one generator from shared source. The engine
**never learns what a mode is** — no patch layer, no override precedence.

Keep non-authoritative lineage metadata for reproducibility (`derived_from`,
generator revision, source fingerprint). The engine must not interpret it as a
balance layer. A derived character may share its sibling's sheet by reference; it
needs its own only when its frames or collision differ.

⚠ **Correction:** an earlier draft claimed runtime inheritance is
rollback-hostile. It is not — `resolve_movement_inheritance` runs once at roster
build and writes a flat value. The real invariant is **"the session consumes
resolved values"**, not "sharing must live in a generator." Generation-time
derivation is right for crossover variants because they are separate *products*.

### 4.4 Malformed inheritance refuses to publish — Jon's ruling
Unknown parent → an unresolved-reference failure through the binding boundary
(namespace, id, declarer, available, did-you-mean). Cycle → report the full
chain, not disguised as an unresolved reference:

```
movement inheritance cycle:
  mary_o_fast -> mary_o_light -> mary_o_fast
```

Fatal to *publication of the candidate content*, not to the process: on reload
the last known-good prepared content stays active.

**Inheritance is provider-local**, with cross-provider allowed only through an
explicit qualified name (`ambition::heavy`) if at all. Unqualified global lookup
is exactly what makes content non-portable.

### 4.5 Presentation identity is stable, not a dense index
An emitted request carries `(PresentationSourceId, SfxId)`. Not a
session-local integer: logs and replays stay readable, table ordering stops
mattering, and staleness cannot silently target the wrong table. The per-request
lookup is negligible beside actual audio playback.

`PresentationSourceId` rather than `CharacterId` because stages, world objects,
rulesets, announcers, and the shell also emit cues.

`AudioContextOwner` (which session owns this) and presentation source (which
content package supplies this) stay **separate fields**. Do not overload one
into the other.

A prepared source may map its cues to dense entries internally — that is an
implementation detail behind the stable pair.

Note: in ordinary rollback netcode SFX messages should **not** cross the network
at all; each peer deterministically re-emits and releases through the
confirmed-frame quarantine.

### 4.6 Cue vocabulary is DERIVED, not hand-listed
A hand-maintained cue list beside the moves it describes will drift. Derive the
dependency inventory from `MoveEvent::Sfx`, `MoveEvent::Vfx`, hit-volume strike
sounds, movement/ability definitions, and techniques. Keep a small explicit list
only for cues emitted by code that cannot be discovered from data.

A session's authorized set is **not** merely the union over its cast: it also
includes stage, ruleset, announcer, world-object, UI and shell dependencies.
The authority is session-level, assembled from all of them.

### 4.7 The brain is a session binding, not identity
A character definition describes the **body**: physical limits, vitals, moves,
abilities, presentation, hurt behaviour. It does not carry `default_brain`.

Control assignment lives on a session participant / spawn spec:
```
CharacterDefinitionId
ControllerBinding          // human | CPU | replay | RL policy
BrainProfileId?            // AI only
Participant / Team
SpawnContext
```

Locomotion crosses the seam as **normalized effort**, never world-space speed:
```
brain internal state:  Pursue          (tactical, never crosses the seam)
brain profile:         Pursue -> 0.85  (normalized effort)
body:                  effort + direction -> its own accel, cap, traction
```
Today's `patrol_speed` / `chase_speed` / `aggro_radius` / `attack_range` on
`CharacterArchetypeSpec` are the inconsistency: they are brain or encounter
policy that knows absolute world speeds.

A heavy at `0.9` and a light at `0.35` sometimes having the same absolute speed
is **not** wrong — effort is relative exertion, not a cross-character ranking.
Navigation that must reach a point by a deadline is a separate concern and may
legitimately use world-space constraints.

### 4.8 Loading keys on a projection, not a universal object
Rooms, matches, and direct startup are semantically different and keep their own
schemas. They share exactly one projection:

```
RoomSpawnPlan ---------------\
MatchParticipantRoster -------> CharacterLoadDemand { ids... }
DirectStartupSpec -----------/
```
The engine-owned materializer consumes `CharacterLoadDemand`. Do **not** build a
rich universal `StagedCast` gameplay object. This also handles transformations,
summons, assists, alternate forms, and post-reveal bosses naturally.

### 4.9 Composition parity is a readiness invariant
Not a test comparing two apps' resource sets — that tests implementation
details. The invariant:

> **Every staged character reaches `Ready` or a named terminal `Failed` state
> before the reveal barrier opens.**

The engine character-runtime plugin owns load-demand consumption,
materialization, load-state tracking, failure reporting, and reveal-barrier
participation. Applications register definitions and submit demand; **they never
install materialization systems.**

Make omission impossible by construction: `register_character` ensures the
runtime plugin, or the top-level engine plugin installs it unconditionally. A
startup capability audit is the backstop for unusual compositions:

```
character `mary_o` was staged, but the CharacterMaterialization
engine service is not installed
```

Behaviour test shape: minimal Bevy app → engine plugin → one representative
provider → enter through **each** route (direct startup, room staging, match
roster) → run the loading/reveal lifecycle → assert the same character reaches
the same materialized result. **No host-application module in the fixture.**
Plus one negative test that omits the capability and asserts the audit names it.

### 4.10 Fallback art: rectangle, and nobody declares — Jon's ruling
Delete the fallback-*sheet* concept entirely. Any actor whose sheet does not
resolve draws the marked rectangle, everywhere, plus a binding report naming the
id. No `CharacterFallbackSheets` registration; **no character names in engine
source**.

Consequence, accepted deliberately: Ambition enemies that currently borrow the
goblin sheet will visibly regress until each gets its own art. That makes
missing art *visible work* instead of a goblin in disguise.

### 4.11 Hurtboxes: simulation-time timelines, two sources
Hitboxes belong to the move (already true — `MoveWindow.volumes`). Hurtboxes do
not, and **must never be derived from the rendered animation frame**: not from
whether an image loaded, not from the renderer's clock, not from frame
interpolation, and they must exist in headless runs.

Two authored sources, one format:
- **body-state / pose profiles** — idle, run, crouch, shield, hitstun, tumble,
  airborne, ledge hang;
- **move-time overrides** — sampled from the authoritative move clock.

Selection: `active move/status override → current body-pose profile → default
body shapes`.

Each profile supports **multiple volumes** and **time ranges/keyframes** from
day one, even if early content authors one rectangle. Hitstun is **not** a fake
move — moves initiate actions with cancels and motion locks; hitstun, tumble,
shield, and locomotion are body states.

The clock comes from the active simulation state: move elapsed for moves,
hitstun timer for hitstun, tumble timer for tumble, a deterministic pose phase
for locomotion, static for idle. One format, several authoritative clocks.

Sprite-derived bbox remains the **default** when unauthored, never the authority.

### 4.12 Attack intent resolution
```
device adapter            -> keys, axes, C-stick, touch, edges, timestamps
per-player interpreter    -> deadzones, flick timing, motion history,
                             accessibility assists, tilt/smash classification
ControlFrame              -> deterministic semantic input the sim consumes
prepared moveset           -> concrete MoveSpec
```
The interpreter is **reusable and per-player**, not per-character: no moveset
reimplements stick-flick recognition, and no device adapter emits
`"forward_smash"` (that would make replay and rollback depend on device-specific
interpretation).

Gesture history is authoritative state — an `AttackGestureState` component per
controlled body, **included in rollback snapshots**. It produces:

```
AttackIntent {
    direction: Neutral | Forward | Back | Up | Down,
    strength:  Tilt | Smash,
    posture:   Grounded | Airborne,
    phase:     Press | Hold | Release,
}
```

A C-stick or dedicated smash key supplies a device-independent `Strong` **hint**
on the control frame; the accumulated history does not live there. Per-player
settings tune flick windows and assists; rulesets provide defaults.
**Characters never define gesture thresholds.** The moveset owns only the final
mapping (`Strong + Forward + Grounded → this character's fsmash`).

### 4.13 `Bound<N>` is preparation-local
`Bound<N>` proves an id existed at a slot in *some* resolver of that namespace —
not which authority minted it, nor which content activation owns it. Acceptable
as a preparation-local result or when consumed immediately through the minting
authority. **Not** a durable session-global capability token across tables.

A prepared character stores flattened values, or table-local indices behind a
registry that owns both the indices and their interpretation. Brand at the
character level, not per cue/move/row:

```
PreparedCharacterHandle { epoch, slot }
```
scoped to the existing `PreparedContentIdentity` (§3.5). **Fingerprint** answers
"do two peers have the same content"; **epoch** answers "does this local handle
belong to this activation". Saved, replayed, and networked state keeps the
stable `CharacterDefinitionId`.

## 5. The unit

```rust
/// One registration. Sections may be inline or an explicit typed reference.
pub struct CharacterDefinition {
    // Identity
    pub id: CharacterDefinitionId,
    pub display_name: String,
    pub provider: ProviderId,          // attribution + asset roots, NOT authority
    pub lineage: Option<Lineage>,      // derived_from, generator rev, fingerprint

    // Art
    pub sheet: Ref<SheetDoc>,
    pub variant_tuning: Option<VariantTuning>,
    pub portrait: Option<AssetRef>,    // select screen; loads without the sheet

    // Body
    pub body: BodySource,              // SpriteAuthored { .. } | Explicit { .. }
    pub hurtboxes: Ref<HurtboxDoc>,    // pose profiles + move overrides (§4.11)
    pub vitals: Vitals,                // max_health, mass, physical limits

    // What it can do
    pub abilities: AbilitySet,
    pub moveset: Ref<MovesetDoc>,      // MoveSpecs: windows, volumes, cues

    // NOT here: default_brain (§4.7), cue vocabulary (§4.6 — derived)
}
```

The engine **derives** rather than the game registering separately: the
character-catalog entry, the roster entry, the cue dependency inventory, the
art-load requirement, and a binding report over every id above. Every string is
a cross-layer reference and resolves through the
[binding resolution boundary](binding-resolution-boundary.md) at preparation, so
a misspelled cue, move, or sheet row is named at load instead of going silent in
a playtest.

## 6. Lifecycle

```
declare      app build      register_character(def)      one call per character
prepare      registration   validate + flatten           -> PreparedCharacterDefinition
enumerate    select screen  identity + portrait only     -> no sheet decode
stage        session start  CharacterLoadDemand { ids }   <- room | match | startup
materialize  reveal barrier ENGINE loads the demand       -> every app, no exceptions
play         --             authorized cues = session set (cast + stage + ruleset + shell)
```

`materialize` replaces the four eager ids, the host app's
`room_transition_assets` step, and Sanic's hand-rolled copy.

## 7. Work queue

`P` = safe to run in parallel with other `P` items (disjoint files).
Estimates assume familiarity with the facts in §3.

| # | Slice | Parallel | Est | Depends on |
|---|---|---|---|---|
| 7.1 | Engine-owned materialization + unprivilege the four | — | ~3h | — |
| 7.2 | Composition-parity readiness invariant + capability audit | after 7.1 | ~1.5h | 7.1 |
| 7.3 | Strict provider-local `inherits` | **P** | ~1h | — |
| 7.4 | `AttackDir::Forward` + attack held/released + directional Special | **P** | ~2h | — |
| 7.5 | Music: `one_shot` survives generation; delete the second implementation | **P** | ~2h | — |
| 7.6 | Character manifest + prepared registry | — | ~5h | 7.1 |
| 7.7 | Source-qualified presentation emission | — | ~3h | 7.6 |
| 7.8 | Match participants + `CharacterLoadDemand` | — | ~3h | 7.6 |
| 7.9 | `AttackGestureState` + tilt/smash classification | — | ~3h | 7.4 |
| 7.10 | Two characters actually fight, coarse hurtboxes | — | ~4h | 7.7, 7.8 |
| 7.11 | Pose/status/move hurtbox timelines | — | ~4h | 7.10 |

**7.3, 7.4, and 7.5 are the parallel-safe starters** — disjoint files, no
dependency on the character work, each independently valuable. Good candidates
to hand to a second agent.

### 7.1 detail — engine-owned materialization + unprivilege
- Move the materializer out of `game/ambition_app/.../room_transition_assets.rs`
  into an engine plugin that consumes `CharacterLoadDemand` and participates in
  the reveal barrier.
- Delete Sanic's copy (`demo_sanic_app/src/lib.rs:479`) and Mary-O's
  startup expectation (her loader currently `warn!`s that the sheet is unbound
  and has been doing so since the perf campaign).
- `CharacterSpriteAssets`: delete the four `Option<..>` fields, one map keyed by
  character id, `asset_for_character_id` becomes one lookup.
- Delete `actor_fallback_asset` and its single consumer's goblin branch (§4.10).
- Delete `EAGER_CHARACTER_IDS` — eagerness is whatever the demand says.
- Make "declared but not materialized" a **different answer** from "no such
  character" so a typo and a pending decode stop looking identical.
- Also chase why the standalone background differs; likely the same
  `load_game_assets` path, not verified.

### 7.5 detail — the music fork
Two independent defects (§3.6). Fix both:
1. Delete `drive_selected_session_music` and install the real music runtime
   (`AudioLibrary` + `MusicPlaybackState` + intent/director) from **one shared
   helper** used by the host and both standalone demos. This is the same
   duplicate-authority disease as 7.1.
2. Teach `regen_music_registry.py` to carry a one-shot flag, correct its
   docstring, regen. Prefer an explicit top-level field on the two sting scores
   over inferring from per-section `loopable`.

## 8. Deliberately deferred

Not over-engineering to skip these; each waits for demonstrated demand:
runtime mode-patch machinery · dynamic third-party folder discovery / mods ·
a generalized inheritance language for every component · skeletal limb-attached
hurtbox tooling · a universal `StagedCast` gameplay object · an editor ·
authority-branding `Bound<AnimRow>` (one namespace escapes its resolver today,
and a release `assert!` catches misuse) · every Smash input nuance before two
characters can fight.

## 9. Prior art referenced

- `docs/planning/engine/binding-resolution-boundary.md` — the resolve-once
  boundary this composes with.
- `docs/planning/triage/declared-id-resolution-checks.md` — the silent-id triage
  that opened this thread.
- ADR 0023 (determinism), ADR 0021 (backend-agnostic world IR).
- `dev/journals/code_smells.md` — the spark blossom entry, corrected.
