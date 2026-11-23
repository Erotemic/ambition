# Asset authoring and prepared media

**State:** PLANNED after source review, 2026-11-21. Production baseline
`3ed50e318392b9c2aedc7e900a3358f75f716474`. Sprite source
`015613c0d7afdd1d3f371afcbfb3100e2d57526f`; music source
`186a13f8115752a4bbdb4955fb9e46483562a2d4`; SFX source
`cf1742d452cf4b7787ce5cc5d520a4b3cc4f76a6`.
This review did not render media, listen to audio, or measure execution cost.
Recheck these paths before implementation; other agents can complete a slice.

**Execution:** [ASSET-AUTHORING](../queue.md#asset-authoring--edit-owned-sources-and-prepare-the-meaning-once).

## Goal and boundary

An author should change a fact at its source, see its prepared meaning, and
review the same meaning that publication will use. An editor should identify
which source it changes and which other characters or cues use that source.
A rebuild must not silently decide which authored changes to keep.

The submodules already have useful domain models. Keep them:

- sprite authoring has several families, an SVG/motion binding pilot, and
  part-first publication;
- music has `CompiledScore`, an exact score clock, instrument and processing
  plans, playback exports, and render-dependency evidence;
- SFX has recipes, backend adapters, ordered effects, and packed-bank output;
- LDtk has semantic operations, room ownership, identity carry, and diffs;
- scripts have command discovery, an editor shell, and scoped asset preflight.

The missing boundary differs in each domain. Do not introduce one universal
asset source format, DSP graph, editor runtime, or plugin service to fix them.

| Packet | Fact with a defined owner | First customer | Existing owner to extend |
| --- | --- | --- | --- |
| SA1 | Editable sprite source versus generated projection | An existing motion-bound character; robot source blocks; scientist rebuild | Sprite authoring and the existing motion binding |
| SA2 | Motion units versus rig units versus raster scale | `director` and `director_boss`, which share a motion library | `CharacterMotionBinding` and motion preparation |
| MA1 | Musical clock and playback products versus game state policy | Hobford sections, then Dinosaur grouped layers | Music compiler/playback exporter and game cue preparation |
| FX1 | Layer time and common processing versus backend synthesis | `dialogue.quest.flag_set`, then `enemy.slime.alert` | SFX source preparation and layer adapters |
| A5.2 | Selected authoring context and producer execution result | Editor preflight for a non-default pack | [Authoring and tools](authoring-and-tools.md#a52---carry-context-and-execution-results-across-clients) |
| L4.1 | Reviewed candidate versus installed map source | Entity move, level clone | [LDtk authoring](ldtk-authoring-and-world-tools.md#l41--prepare-the-complete-edit-before-writing-source) |

[Asset lifecycle](asset-library-editor-and-delivery.md) owns validity,
accepted-bundle selection, and delivery. This page owns source editing and
the media meaning that a consumer must receive. These are separate contracts:
a current render can still be paired with incorrect game timing; a valid
published rig can still have an unclear editable source.

## Additional presentation consumer

**Additional selected consumer, 2026-11-21:** the
[2.5D cart pilot](presentation-order-and-25d-rendering.md#authoring-and-installation)
uses current part textures plus optional mesh/material products. Extend the
existing cart source and provider look with named composition places and a
small authored mesh binding; do not require all sprite authors to change
formats. Preserve source identity, canonical geometry units, quality and
immutable publication. A view or other demanding consumer selects residency;
installing a second renderer alone does not load its richer products. The
2D authoring and preview tools remain supported. Visual depth/thickness is
authored meaning, not a reinterpretation of existing numeric z bands.

## SA1 — editable sources and disposable sprite projections

### Evidence

The sprite [motion IR](../../../tools/ambition_sprite2d_renderer/docs/authoring/motion_ir_v1.md)
already separates SVG rest geometry from named motion poses/clips and compiles
them through `CharacterMotionBinding` into a renderer projection. It is a real
pilot, not a missing format. Keep its authored pose identity and sparse,
rest-relative transforms.

Other authoring paths still edit generated rig JSON. The root sprite README
describes SVG-to-rig rebuild, GUI editing, and sheet generation as separate
steps. Family rebuilds have different rules for keeping edits:

- `scripts/build_scientist_fighter_rigs.py::build_one` replaces generated
  clips; `authoring/canonical_scientist_rig.py::ensure_scientist_rig` can
  trigger that rebuild during pose audit.
- `scripts/build_player_robot_v3_svg.py::_preserved_authoring_blocks` keeps
  only `gameplay_geometry` and `animation_constraints` during a rebuild.
- `targets/characters/rigged/pca_rig_extract.py::merge_doc` keeps old clips,
  but the later `pca_combat_authoring.author_pca_combat_clips` replaces
  selected channels.
- `gui/app.py::save` writes the open `RigDocument` without resolving these
  different ownership rules.

These paths are relative to the sprite submodule. Use the exact target route
rather than treating every rig JSON file as the same kind of source.

This is an authority problem before it is a GUI problem. An editable clip
must not survive only because a family-specific builder happens to copy it.
Nor can a derived side/back view become a second editable copy of the same
motion without a declared override.

### Contract

For a selected target, its authoring owner exposes a small description of:

1. the authored files and the facts each file owns;
2. the preparation step and generated files it may replace;
3. the editable pose/clip identities and the bindings that use them;
4. the view/facing/unit conversion used by the preview;
5. the source revisions and source patch represented by a draft.

Extend the existing target and motion binding. This description is an owner
projection, not a second checked-in asset catalog. A procedural target may
point to Python or configuration. It need not acquire a skeleton or a motion
library. The [plural authoring rule](sprite-renderer.md#principles) stands.

For a migrated rig family:

- SVG owns static artwork, topology, pivots, markers, and view geometry.
- Motion source owns poses, clip keys/timing, and declared motion parameters.
- A binding owns source selection and explicit coordinate conversion.
- The generated renderer rig is disposable. Frontends can display it, but
  save the corresponding source edit through the owner.
- A generated view is disposable unless the author explicitly creates a
  view-specific source override. An arbitrary edit to a lossy projection
  cannot silently be written back to a shared source.

A draft review names the target, pose/clip, source file, affected bindings,
and source revision. The frontend must make a shared-library edit visible
as shared. An author can edit the shared motion or create an explicit local
pose/clip source; the tool must not fork a library as a hidden save step.
Reuse A3's stale-source refusal for applying the draft. Preview is local work;
saving source and accepting a publication remain separate operations.

Inspection reports a stale projection; it does not rebuild a tracked file as
a side effect. Explicit preparation uses scratch or the existing publication
snapshot and observes the actual builder inputs. Scientist freshness currently
uses `EXPECTED_BUILDER_VERSION`; changing clip-builder code need not change
that manual value. A recorded read of the old generated rig cannot prove that
the omitted upstream derivation is current. Feed the actual derivation into
the existing owner evidence, without another freshness database.

Keep rig-only preview as a useful, labelled scope. A target-result preview
uses the selected family's composition. For example,
`targets/characters/_authored_swing_fighter.py` needs earlier frames in the
clip to form a later streak. A pose alone is not always a complete picture.
The preview request can select a clip/time and its required history; the GUI
must not implement a second effect renderer.

### First implementation and deletion

1. Give one existing motion-bound character a target-aware CLI/frontend
   open/inspect/save route. A save writes its authored motion; preparation
   derives its rig. Preserve its current art, timing, and parts output.
2. Give the robot's two already-preserved source blocks an explicit source
   home and resolve them through its family preparer. Delete that builder's
   copy-old-fields/merge-back route once the new owner supplies those facts.
   Keep its procedural clip program; this step does not convert it to JSON.
3. Use a scientist rig as the stale-derivation witness: inspecting it does
   not overwrite it, and explicit preparation observes a clip-builder edit.
   An edit to generated motion changes the owning program or creates an
   explicit authored override. Sampling a procedural clip into pose keys is
   lossy source promotion, not an invertible save.
4. Remove editor writes to disposable projections for the migrated routes.
   Retain ordinary standalone editable rig documents. Migrate PCA's merge
   rule only with a target edit that names generated channels and authored
   overrides. Do not move the entire corpus at once.

**Acceptance:** edit a contact pose through a frontend or the headless owner,
rebuild the target from its SVG, and show the edit in the prepared preview and
part publication. A repeat rebuild changes no authored motion. A changed
shared pose names both affected bindings. An edit to a derived-only view is
refused or creates a declared source override. Inspect does not rewrite source;
a builder edit invalidates its derived rig. A review identifies rig-only or
full target composition. No baked reference sheet or full-library render is
required for this proof.

## SA2 — make motion-space conversion explicit

### The current invariant confuses two units

`load_svg_rig_definition` in
[`motion_ir.py`](../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/authoring/motion_ir.py)
resolves transformed markers into root SVG user units. Pose root translations
and bone-local translation deltas use those units. `to_rig_document` multiplies
rest geometry and motion translations by `render.frame_px_per_rig_unit` to
obtain the renderer's pixel coordinates. Facing reflection is already explicit
and invertible.

The motion IR guide's section **A shared library fixes its readers' unit scale**
and `test_every_binding_on_one_library_shares_that_library_unit_scale` in
[`test_motion_ir.py`](../../../tools/ambition_sprite2d_renderer/tests/test_motion_ir.py)
require equal raster conversion for bindings that share a library. Equal
pixels per rig unit do not establish equal motion units or body proportions.
Different raster conversions can also be correct for the same motion.

Replace that assertion with a coordinate contract. Do not preserve the
current test by forcing authors to reshape SVG art to satisfy a raster rule.

### Contract and bounded mathematical proof

Distinguish three values:

- `motion_to_rig`: a positive uniform unit conversion, resolved by the binding;
- the existing motion/source-facing conversion;
- `frame_px_per_rig_unit`: presentation conversion after pose evaluation.

Names are illustrative; extend the existing serialized binding and library
space declaration. Do not add a separate runtime transform stack. A library
has one declared source space. A binding states how that space maps to its
rig, even when the conversion is identity. Preparation refuses unresolved or
unsupported space relations instead of inferring them from output pixels.

Apply the unit conversion once in prepared motion, before direct sampling,
Godot, and the `RigDocument` adapter consume it. Patching only the sheet
converter would leave those readers in different spaces. Include root and
bone-local linear deltas and dimensioned position tracks. Rotations and
dimensionless scalar tracks retain their meanings. Rest geometry remains
in the rig's own space. A length-valued parameter needs an owner-declared unit;
do not multiply every scalar track. Facing conversion keeps the existing
reflection of horizontal deltas and rotation signs. A frontend's source
export applies the inverse conversion exactly once.

For a uniform re-expression of a rig by a positive factor `q`, use:

```text
rig rest geometry and art coordinates: q times the previous coordinates
motion_to_rig: q times the previous conversion
frame_px_per_rig_unit: previous conversion divided by q
```

With an unchanged frame anchor, compare the evaluated positions after mapping
both results into the same frame/reference space. Raw rig-space positions
correctly become `q` times larger; frame positions and trajectories remain
equivalent within the renderer's stated numeric tolerance. The assertion
covers actual evaluated positions, not only equal metadata.
It is a unit-change proof, not a requirement for bit-identical SVG raster
antialiasing on every library version.

This conversion does not retarget body proportions, joint topology, contacts,
or rest poses. Rig-profile compatibility and the existing reviewed anatomical
retarget operation remain separate. A matching pixel height proves none of
those properties. Gameplay height, collision, and attack timing are outside
this presentation-unit change.

This packet does not resume the held cast-wide
[texel-density rollout](character-texel-density.md) or select density, quality
tier, or supersampling policy. `frame_px_per_rig_unit`, `render_scale`, and
`supersample` remain distinct controls. Use one bounded binding-space proof.

### First implementation and deletion

Use `director` and `director_boss`, which share `director_pen_v1`. Their
declared motion parameters are opacity/visibility values, so the first proof
does not need a general physical-units system for arbitrary parameters.

Add the binding conversion, prepare/export inverse, and direct unit-change
proof in the motion owner. Recheck facing reflection with a translated pose.
Replace the equal-raster-scale test and amend the submodule guide in the same
change. Preserve the current look by writing the current identity conversion
for migrated bindings; further family adoption is separate work.

**Acceptance:** changing only raster density changes presentation resolution,
not the motion-to-rig interpretation. Uniformly re-expressed rig coordinates
with compensating conversions preserve geometry and motion in the common
frame space, and preserve published mechanical measurements. Keep the earlier Director dash regression
caught by the new proof. A saved pose round-trips through both facing choices.
Unsupported proportions or unknown length-valued controls do not receive a
guessed conversion.

## MA1 — export playback facts; link game music policy

**Refinement, 2026-11-22:** MA1 owns compiler/export facts.
[Music programs and owned playback](music-programs-and-playback.md) owns
candidate selection, native occurrence lifetime, transition execution, and
cue recipes. Its reported handoff defect can be corrected before the full
MA1 migration. Keep the boundaries separate and use one prepared link.

### Evidence

The music compiler owns the score clock and form. Its
[`playback_layer_plan`](../../../tools/ambition_music_renderer/ambition_music_renderer/render/playback_layers.py)
already resolves section spans, loops, layer groups, lead-ins, and tails.
`_cut` folds a loop's ring tail onto its head and its lead-in onto its end;
a one-shot can append a tail. The file span and the musical transition point
are thus distinct facts.

The game repeats BPM, beats per bar, duration in beats, loops, and product
paths in
[`music_cues.ron`](../../../game/ambition_content/assets/audio/music_cues.ron).
[`MusicCueSpec`](../../../crates/ambition_audio/src/cue_spec.rs) computes section
duration from one cue-wide BPM. The director's
[`timing.rs`](../../../crates/ambition_audio/src/music/director/timing.rs)
reconstructs its bar/phrase grid from that same constant. Music publication
copies an adaptive manifest as a debugging aid; the game does not load it as
its playback clock (`ambition_music_renderer/cli.py`,
`publish_adaptive_full_sections`; also copied by `publish_playback_layers`).

Hobford already needs a one-shot `break_in`, its ring tail, and the game-owned
continuation to `raid`. Its current source and game section lengths/loop flags
agree: `break_in` has a 16-second musical body and a 4-second tail. Treating
its 20-second file span as the transition boundary would regress the current
32-beat game declaration. Dinosaur needs six grouped playback layers and game
intensity gains. Those are two current customers for the ownership split.

A representational counterexample explains the clock gap: four quarter notes
at 120 BPM and four at 60 BPM take six seconds. An eight-beat, 120-BPM runtime
entry takes four. An average BPM that matches the total still places internal
bars incorrectly. This is not a claim that a shipped cue currently has that
tempo map or that its observed playback is wrong.

### Contract

The music owner exports a small, versioned playback description from the same
compiled score and product plan that render the audio. It contains:

- cue, section, and layer identities and logical product roles;
- the map from score time to the exported file's local time;
- musical end, loop period, and audible tail as separate values;
- bar boundaries and the information needed by supported phrase/transition
  queries, including section origins and loop wrap;
- frame/sample rounding needed to align the produced file;
- the gain basis: a mastered full mix or grouped pre-master layers, including
  native versus shared runtime-stem attenuation already applied by export.

The current plan is a useful starting point, not proof that every field is
already exact. Its lead-in calculation uses the preceding form section's
average seconds per beat. For a variable clock, resolve the actual last-beat
boundary through the compiled score clock. Preserve the current constant
case. Do not add a second clock inside the export operation.

Keep constant-tempo/meter sections as a compact arithmetic representation.
Use a compact clock map or ordered boundaries for varying clocks, and query
that prepared representation. Do not walk notes or parse source in the game
loop. The source-only inspection form can use score time; product acceptance
adds the final render/sample facts. Clearly label which form is available.

Game policy retains states, state-to-section bindings, intensity gains,
continuations, room/encounter/boss associations, relative mix level, and
transition preferences. Preparation links these references to the selected
playback description. Keep the current layer-channel budget and allocation
order for the first migration.

The logical end can start the next state while a tail remains audible. The
decoded file duration is not a substitute for that logical end. A mastered
full mix is not interchangeable with the sum of independently faded grouped
layers. Do not apply export attenuation twice or feed a mastered full mix
through legacy per-layer balance correction. The soundtrack/radio product
and the adaptive playback product can
share source while retaining different roles.

The playback description and its media belong to the same accepted product
selection. Retaining old audio also retains its old playback facts. Linking
new source timing to old selected audio under the same cue ID is invalid.
Use the existing bundle and content-generation owners for this association.

### Frontend clocks, product capabilities, and strict linking

The later static audit at `00bf92c22142` found that V1 already renders
tempo maps but sets `CompiledScore.exact_metadata` to `None`. V2/V3 retain
exact clock metadata. Expose each compiler's clock through one projection;
do not infer the V1 clock from initial/average BPM. Keep V3 generator-clip
clock limits visible beside the richer exact-clip support.

Retain typed entry/exit roles separately from display chapters. The current
playback plan drops exit markers, and V3's public form validation rejects
`valid_exit_local_bars` although lowering can copy it. Resolve supported
markers into the actual product's frame origin using the same cut and
rounding as the render. Publish musical facts and final media facts as
distinct inspection levels.

A tail folded into a loop head is not an independently playable release
tail. A folded lead-in repairs the repeat seam; it is not a scheduled
destination pickup. State which material each product retains. Add a
separate entry/exit product only when a requested operation needs it.
Current ring validation checks adjacency without compiled event ownership;
validate the declared note-free region against its actual events. Source
validation cannot prove acoustic decay quality.

Use one pure native preparer for content compilation, direct registration,
and reload. Reject duplicate IDs/slots before map collection, empty playable
sections, missing required sources, incompatible layer/product facts,
invalid continuations, and unsupported synchronization requests. Resolve
game references and gains to prepared IDs and total arrays. A simple
whole-asset loop or AssetEnd one-shot needs no musical clock; richer
operations add only the facts they require.

The product identity must include playback/mix settings, not just compiled
notes. Reuse the existing render dependencies and accepted-bundle identity.
Decoded readiness and matching layer origins do not prove a synchronized
native start. MUS1/MUS3 own that distinct admission/transport evidence.
Keep compiler-side clock/marker/ring checks separate from native linking;
the latter does not need the score's note events. Preserve V1/V2's existing
mastering/regeneration policy and selected audio. New recipes use composition
mastering; changing an older product's mastering is a separate reviewed edit.
Full source paths and the bounded recipes are in the
[music owner](music-programs-and-playback.md#contract-2--a-prepared-program-describes-possible-playback).

### First implementation and deletion

1. Expose a music-owned inspect/export operation using `CompiledScore` and
   the existing playback-plan builder. It needs no synthesis, device, or
   installed sample library to report score structure. Resource realization
   obligations remain visible and separate.
2. Link Hobford's sections and continuation, then Dinosaur's grouped layers.
   Remove their duplicated numeric clock, loop, and generated-path values
   from game source. Preserve game policy and current selected products.
3. Make director timing query prepared playback facts. Keep the cheap
   constant-grid path and native music ownership. MUS1–MUS3 replace the
   displaced playback/transition authority; MA1 does not add a sample-accurate
   device scheduler.
4. Have the owner export cue inventory and one-shot/product roles. Remove the
   corresponding line-based YAML interpretation from
   `scripts/regen/regen_music_registry.py` when its caller uses that export.
   Preserve score-level one-shot policy separately from adaptive section
   loops. Display names, radio inclusion, and retirement policy stay game-owned.

**Acceptance:** an authored tempo/form edit changes the prepared transition
schedule without a numeric RON edit. A one-shot advances at musical end while
its tail follows the declared policy. A layer-gain change keeps all layers at
one loop position. A changed source cannot silently retime retained old
audio. Unknown section/layer references fail before activation. Test the
clock/plan without rendering. MUS1's recording consumer must process the
same command decisions as the device adapter for transition/ownership
acceptance; existing mode assertions alone cannot prove command execution.
Device timing requires separate evidence. Hobford and Dinosaur remain the
first customers. Galvanic/Tritone add the reported handoff; `fable` and
`billions_and_billions` add source-only V1/V3 variable-clock inspection.

## FX1 — prepare SFX layer meaning before backend execution

### Evidence

The SFX schema validates a raw recipe; the renderer interprets common fields
while it executes. [`layers.py`](../../../tools/ambition_sfx_renderer/ambition_sfx_renderer/layers.py)
first renders a source, then crops/pads it to `duration_ms`, then applies
envelope, gain, pan, and ordered effects.

Synth and noise backends already use `duration_ms` for source generation.
The [Faust adapter](../../../tools/ambition_sfx_renderer/ambition_sfx_renderer_gpl/dawdreamer_backend.py)
instead uses `duration_seconds` or the whole cue duration for its render and
parameter ramp. The common layer crop then occurs after that ramp was built.

The shipped
[`dialogue.quest.flag_set`](../../../tools/ambition_sfx_renderer/sounds/active/dialogue.quest.flag_set.sfx.yaml)
recipe has a 260 ms cue and 160 ms / 120 ms Faust layers. Its parameter ramps
are prepared over 260 ms and then cropped. `enemy.slime.alert` combines a
70 ms noise layer and a 150 ms Faust layer in a 220 ms cue. The same common
duration field has different control-time meaning by backend. This conclusion
comes from source; no rendered waveform or listening result is claimed.

The renderer also chooses safety DSP/peak defaults from cue-ID substrings
(`render.py`, `default_final_peak_db` and `default_tone_safety_effects`).
A rename can therefore change sound even
when the recipe's sound parameters are unchanged.

### Contract

Add a small SFX-owned prepared layer invocation. Resolve source aliases and
common defaults once, before optional backend imports. It distinguishes:

- placement on the render timeline, including preroll, and the separate
  playable loop window;
- source/control interval;
- retained output interval and explicit crop/loop operation;
- sample rate, channels, seed, envelope, gain, and pan;
- ordered layer effects;
- backend-specific parameters and unresolved realization requirements.

Backends receive the resolved source/control interval. They do not choose
between parent and layer durations. A sample crop, intentional truncation of
a longer source, preroll, loop crossfade, or effect tail is valid when named
explicitly. One layer duration must not silently mean a control span for one
backend and truncation of a different span for another.

The enclosing cue program owns mixing, cue postprocessing/safety, and loop
folding once, after layer execution. A looped cue's `start_ms` is measured on
the whole render timeline, including its preroll; do not shift that origin to
the playable loop start. The exact temporal order of synthesis, crop,
envelope, effects, and cue mixing is part of the prepared program. Preserve it
unless a particular correction
changes it deliberately. Do not reorder a core effect across a Pedalboard
effect because both fit a shared API.

Use this result for source-only validate/inspect and for rendering. A missing
plugin can be an unresolved realization requirement while ordinary source
structure is inspectable. A source-only operation cannot certify an arbitrary
plugin's controls without plugin metadata. Never substitute another sound or
claim full render readiness because source validation passed.

Make the selected safety/sound profile explicit for migrated recipes. The
current `render.safety_profile: game_sfx_v2` label still branches on cue ID;
setting that existing string alone does not close this boundary. Store the
resolved numeric policy or select a new ID-independent preset. Retain the
tuned values, rather than retuning the corpus. With explicit profile and seed,
changing an identifier alone must
not change the prepared sound program except for provenance and identity.

### First implementation and deletion

1. Normalize the synth/noise/Faust common invocation. Use the two named recipes
   to expose actual source/control/output intervals without loading a native
   renderer. Keep common interval validation in this owner.
2. Make these adapters consume the prepared interval and common defaults.
   Remove their independent duration precedence. Add explicit long-source
   truncation if a recipe intends the old sound.
3. Review the affected pitch ramps with audio when the backend is available.
   Reaching the authored end frequency at the local layer end can change the
   old cropped sound. Record it as a sonic correction, not refactor parity.
4. Freeze explicit sound policy for the first migrated recipes, then remove
   ID-substring inference for them. Further recipes migrate with a concrete
   edit or render customer, not as a prerequisite for the first proof.

**Acceptance:** inspect the same local control span through each admitted
backend; a generic ramp has the declared endpoint at that span's end. An
explicit longer-source crop retains its separate meaning. Ordered effects
stay ordered. A rename with the same explicit profile/seed preserves the
prepared sound program. Source inspection succeeds without native synthesis
while reporting the remaining realization requirements. Check actual PCM
under the intended backend before claiming audible equivalence.

## Efficiency and scope limits

These packets remove repeated interpretation before expensive work. They do
not add generic dispatch to gameplay:

- motion spaces resolve during preparation; part playback stays on its
  existing prepared draw path;
- music uses a compact prepared clock and selected file descriptions;
- SFX inspects a prepared program without synthesizing audio;
- draft sprite preview targets a selected clip/view; publication keeps its
  existing parts and owner-validity rules;
- script context and map candidates let repeated authoring steps share the
  same inputs and finalization work.

Measure one changed target/cue/edit when its implementation lands. Report
source preparation separately from rasterization, native synthesis, packing,
and publication. There is no compile-time or runtime speedup measurement in
this review. New engine-facing playback types should remain small and should
not pull Python tools, GUI packages, or synthesis dependencies into Rust.

Do not merge MusicIR and SFXIR, or move similarly named DSP functions into a
utility package without proving equivalent units, order, and numerical
behavior. Music loop-tail folding and SFX texture crossfade have different
semantics. Reusable low-level primitives need a demonstrated common contract.

Do not reopen completed tool discovery, editor-shell consolidation,
part-first output, or accepted-bundle publication. Their established boundaries
are inputs to this work, not new projects in this plan.
