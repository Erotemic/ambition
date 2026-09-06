# Bevy 0.19.1 leverage campaign

## Status — ✔ COMPLETE 2026-08-31 (reviewed and repaired the same day)

⚠ **A REVIEW OF THE FINISHED CAMPAIGN FOUND FIVE THINGS, AND THEY ARE FIXED.**
Recorded here rather than in each section, because the pattern is the lesson:
four of the five were in the code this campaign ADDED, not in what it removed,
and the fifth was a section marked done on the strength of a fact that was true
but insufficient.

- **A5 was closed without doing it** — the render-pass paths exist upstream, so
  the section was called a selection problem and then no selection was made. See
  A5 below; reopened and closed properly.
- **The F1 panel spawned three overlay windows exactly on top of each other**,
  because Bevy gives every `DiagnosticsOverlay` the same initial offset and
  nothing staggers them. One window now.
- **Every population read as a smoothed float** (`7.3842 bodies`), because
  `DiagnosticPath::into()` picks an EMA at four decimals. The publisher's own
  test could not see it: it checks `Diagnostic::value()`, and the panel rendered
  `smoothed()`.
- **A terminal render error logged every frame forever.** `StopRendering` is
  documented to re-poll the handler each frame; the pure policy test could not
  see it, because a pure function called twice is called once.
- **The ECS census walked the world on every visible frame**, F1 open or not.
  Now `Query::count()` (archetypal) on a 250ms cadence.

Every section is closed. A (A1–A5), B, C, D, E and G were implemented; F was
SPIKED AND DECLINED on its own decision gate, which is a completed section and
not a skipped one. H was optional throughout and stays optional — H2 (`Rem`
scaling) is now cheap to reach because A1 put the typography on upstream's
config, and H1 (`render_debug`) is unlocked because `bevy_dev_tools` is in the
build; neither is a campaign obligation.

The campaign said it should "leave behind fewer custom mechanisms than it starts
with". It did:

```text
gone   MenuTextHeightFraction + its per-frame conversion, installer and marker
gone   FpsOverlayState, the FPS text spawn, its formatter, its visibility system,
       and the hand-rolled min/mean/max window statistics
gone   DebugOverlayLabel and the per-frame Text2d spawn/despawn of debug labels
gone   the fullscreen post-process pass on every default frame
new    ScreenEffectCamera (a marker), EcsPopulation (a system param),
       host::render_recovery (a policy), dev::diagnostics_panel (a view)
```

⛔ TWO OF THIS DOCUMENT'S OWN CLAIMS WERE WRONG, and both were caught by checking
rather than believing: gizmo text is WORLD-space, not screen-space (section B),
and Bevy's FPS overlay cannot satisfy this campaign's own web requirement
unaided (section A1). A campaign brief is a hypothesis, not a specification.

## Original status

Ambition has completed the Bevy 0.19.1 migration.

This campaign is a short follow-up intended to spend roughly one or two focused development days converting newly available Bevy 0.19.1 capabilities into concrete improvements.

The campaign should favor three kinds of work:

1. replace Ambition-specific mechanisms with stronger upstream mechanisms;
2. improve runtime observability without adding a second interpretation of gameplay;
3. remove work from ordinary frames when a feature is disabled.

This is not another engine migration and should not become a broad rewrite.

---

# Goals

By the end of the campaign:

* FPS is visible everywhere in the visible application whenever the global FPS setting is enabled;
* the FPS setting defaults to enabled and remains independent of the visual-quality profile;
* F1 exposes useful numeric runtime diagnostics in addition to world-space debug geometry;
* F1 world labels use Bevy's text gizmos instead of spawning `Text2d` entities;
* existing runtime measurements begin feeding Bevy's `DiagnosticsStore` rather than being useful only as log output;
* menu viewport-height text sizing uses `FontSize::Vh` directly;
* entity diagnostics correctly account for Bevy 0.19 resource entities;
* Ambition has an explicit render-device-loss policy;
* Bevy's `SettingsPlugin` has been evaluated against Ambition's actual persistence requirements, with migration performed only if the fit is clearly better.

The campaign should leave behind fewer custom mechanisms than it starts with.

---

# A. Make diagnostics host-global

This is the highest-priority work.

The current diagnostics architecture is partly session-scoped even though several of the facts being displayed are process/display facts.

That distinction should become explicit:

```text
host diagnostics
    FPS
    frame time
    process/system information
    render information
    asset residency
    camera/render-target census
    presentation workload
        |
        +---- available from launcher/title/loading/gameplay/pause

session diagnostics
    body state
    hitboxes
    contacts
    room geometry
    rollback/session state
    portal state
        |
        +---- available only when the corresponding authority exists
```

The host-level diagnostic UI must not disappear because no gameplay session currently exists.

---

# A1. Replace the custom FPS UI with Bevy's FPS overlay  ✔ DONE 2026-08-31

Landed on `bevy::dev_tools::fps_overlay`. Deleted: `FpsOverlayState`, the spawn, <!-- cite-ok: the row records what the port DELETED -->
the text formatter, the visibility system, and the hand-rolled min/mean/max
window statistics. Kept the two things that are Ambition's rather than Bevy's —
which setting decides it (`UserSettings::video.show_fps`, still the only
authority, still persisted, still independent of `VisualQualityProfile`) and
WHERE it sits (under the touch Menu/Back row, read from
`ResolvedGameplayPresentation`; the counter used to spend every phone session
underneath the action cluster).

⛔⛔ AND UPSTREAM CANNOT MEET THIS SECTION'S OWN WEB REQUIREMENT ON ITS OWN. Bevy
spawns the `FrameTimeGraph` node under
`#[cfg(not(all(target_arch = "wasm32", not(feature = "webgpu"))))]`; Ambition's
web persona is `bevy/webgl2`; and `toggle_display` takes
`Single<&mut Node, With<FrameTimeGraph>>`, which returns `skipped` when nothing
matches. So on the shipping browser build `FpsOverlayConfig.enabled` would be a
setting that does nothing. Ambition therefore adopts the overlay AND owns
visibility on the root node, which covers every platform and wins over
upstream's per-text toggle rather than fighting it.

⭐ `bevy_dev_tools` COSTS NO NEW SUBGRAPH, measured rather than feared: it
depends on `bevy_pbr`, but `bevy_pbr` is already in all three personas via
`bevy_gizmos_render`. One leaf crate, not a renderer — and the same feature
carries `diagnostics_overlay`, which A3 needs.

⭐ THE ACCEPTANCE TEST ASKS BEVY'S OWN ANSWER. `the_fps_counter_draws_in_front_of_the_launcher`
reads `UiStack.uinodes` — the computed back-to-front draw order — so "last entry"
IS "frontmost" rather than a z-index number re-derived in the test.
Poison-verified: pointing it at the back of the stack fails it.

## Original plan

Current implementation:

`game/ambition_app/src/dev/fps_overlay.rs`

This currently:

* installs `FrameTimeDiagnosticsPlugin`;
* spawns and owns its own UI `Text`;
* calculates its own recent min/mean/max;
* owns `FpsOverlayState`; <!-- cite-ok: under '## Original plan' — describes the pre-port code -->
* mirrors `UserSettings::video.show_fps` into that resource;
* manually positions the UI;
* manually controls visibility.

Bevy 0.19.1 already has:

`bevy::dev_tools::fps_overlay::FpsOverlayPlugin`

and:

`FpsOverlayConfig`.

Use the upstream implementation unless testing identifies a product requirement it cannot satisfy.

## Required semantics

The authoritative setting remains:

```text
UserSettings::video.show_fps
```

It is a global presentation preference.

It must:

* default to `true`;
* remain persisted;
* apply on desktop, web and Android;
* apply independently of `VisualQualityProfile`;
* work with no gameplay session;
* remain in effect while switching between Ambition, Smash and other shell-hosted games.

Do not make `High` or `Ultra` quality imply FPS visibility. `show_fps` is the explicit authority.

Sync:

```text
UserSettings::video.show_fps
        ↓
FpsOverlayConfig.enabled
```

directly.

Delete `FpsOverlayState` if it becomes only a mirror of that boolean. <!-- cite-ok: the original plan's own proposal, kept as the record -->

## Visibility contract

When enabled, FPS must be visible in at least:

* initial loading;
* launcher/title;
* game selection;
* Ambition gameplay;
* Smash gameplay;
* normal pause menu;
* kaleidoscope pause menu;
* dialogue/cutscene UI;
* room-transition/loading presentation;
* any state where no session world exists.

The current host already owns a persistent `FrontHudCamera` marked `IsDefaultUiCamera`.

Treat that camera as the host-global UI presentation seam.

Do not bind FPS lifetime to gameplay entities, a room, `GameMode`, or `session_world_exists`.

## Layering

The current custom FPS node has no explicit `GlobalZIndex`.

Bevy's FPS overlay deliberately renders at a very high `GlobalZIndex`.

Use that upstream layering rather than rebuilding another ordering convention.

Add an acceptance test involving an opaque fullscreen menu/loading UI so the test proves the FPS surface stays above it, rather than merely proving that an FPS entity exists.

## Frame-time graph

The upstream FPS overlay can also display a frame-time graph.

Recommended policy:

* ordinary `show_fps=true`: compact FPS/frame-time text;
* F1 debug mode: enable the graph.

That keeps the default display small while making frame pacing immediately inspectable during tuning.

If the graph is too intrusive in practice, keep it F1-only.

---

# A2. Make F1 a host-level diagnostics toggle  ✔ DONE 2026-08-31

`handle_debug_hotkeys` is out of the session gate. It reads only
`DeveloperRuntimeState` and `DeveloperTools` — two HOST resources — but sat in a
`.chain()` under `session_world_exists` beside three systems that genuinely need
a session (LDtk reload, the trace hotkey, the map menu). It keeps an explicit
`.before(handle_ldtk_hot_reload)` edge, because both write
`DeveloperRuntimeState` and the chain used to decide that between them.

⛔⛔ THE PRODUCER WAS NEVER GATED, WHICH IS WHY THE FAILURE WAS SILENT.
`emit_developer_actions` runs in `PreUpdate` unconditionally, so the message was
always being written and only the consumer refused to hear it: no log, no
warning, F1 simply did nothing on the launcher, the title screen or a loading
screen.

⭐ The regression test states its premise first, and that premise EARNED itself:
the first draft booted the direct/test bypass, whose own guard caught that it
DOES carry a gameplay session — a state in which the test would have passed
against the gate it exists to pin.

## Original plan

Current F1 handling eventually reaches:

`game/ambition_app/src/app/dev_runtime.rs::handle_debug_hotkeys`

but its installation in `app/plugins.rs` currently places it in a chain gated by:

```text
session_world_exists
```

That means the semantic F1 action cannot update the main debug state from launcher/title/loading states.

Separate the actions.

Host-global debug actions such as:

```text
ToggleDebugOverlay
ToggleInspector
ToggleWorldInspector
```

should be consumed by host presentation regardless of session existence.

Operations that genuinely require a session, such as LDtk reload or room-specific operations, remain session-gated.

The desired shape is:

```text
DeveloperAction
        |
        +---- host debug state             always
        |
        +---- session-specific operations  only when session exists
```

F1 should therefore work before entering a game and continue to refer to the same debug mode after entering or leaving one.

---

# A3. Use `DiagnosticsOverlayPlugin` for the F1 numeric panel  ✔ DONE 2026-08-31

`dev/diagnostics_panel.rs` installs `DiagnosticsOverlayPlugin` and spawns three
windows while `DeveloperRuntimeState.debug` is on: **Frame** (FPS, frame time),
**Ambition** (the ECS and render populations below) and, on desktop, **Host**
(process CPU and memory).

⭐ NOTHING IN THE PANEL COUNTS ANYTHING. Every value is a `DiagnosticPath`
published by the subsystem that already knew the fact, so the panel and the
periodic `[census]` row cannot become two answers with one name.

⭐ TWO ENTITY NUMBERS, NAMED — `ambition/ecs/scene_entities` and
`ambition/ecs/resource_entities` — exactly as this section demands, because one
number called "entities" would carry Bevy 0.19's resources-are-entities
ambiguity into every note taken from the panel.

⛔ SYSTEM INFORMATION IS DESKTOP-ONLY AND SAYS SO. `SystemInformationDiagnosticsPlugin`
rides `bevy/sysinfo_plugin`, which `default_platform` carries and which
Ambition's `android_platform` and `web_platform` sets EXCLUDE deliberately. The
overlay renders an unregistered path as `Missing`, which is the honest answer;
no substitute is synthesized.

## Original plan

Add Bevy's new:

```text
DiagnosticsOverlayPlugin
```

as the screen-space numeric diagnostics surface.

When F1 is enabled, display an Ambition diagnostics window.

The initial useful set should include existing Bevy facts rather than building equivalent measurements:

### Frame

* FPS;
* frame time;
* frame count where useful.

### Host/process

Evaluate:

* `SystemInformationDiagnosticsPlugin`;
* CPU usage;
* memory usage.

These are useful for distinguishing:

```text
the game got slower
```

from:

```text
the host is under CPU/memory pressure
```

without opening an external profiler.

### ECS

Do not blindly display Bevy's raw entity count as "game entities".

Bevy 0.19 resources occupy singleton entities.

Publish two clearly named values if both are useful:

```text
ecs/scene_entities
ecs/resource_entities
```

or:

```text
ecs/entities_total
ecs/entities_non_resource
```

The semantic names must make the distinction clear.

---

# A4. Turn Ambition's existing censuses into diagnostics  ✔ DONE 2026-08-31 (the selected subset)

Published, from the crate that owns each fact:

```text
ambition/ecs/scene_entities        ambition_dev_tools::runtime_census (EcsPopulation)
ambition/ecs/resource_entities     ambition_dev_tools::runtime_census (EcsPopulation)
ambition/ecs/bodies                ambition_dev_tools::runtime_census (EcsPopulation)
ambition/render/cameras            ambition_render::runtime_census
ambition/render/world_draws        ambition_render::runtime_census
ambition/render/offscreen_targets  ambition_render::runtime_census
```

⭐ THE ECS PUBLISHER SHARES `EcsPopulation` WITH THE CENSUS PRINTER, and a test
pins that they agree — it samples the param and the store in the same frame, then
MOVES the population and re-checks, so a publisher wired to a constant fails.

⭐ THE RENDER PUBLISHER SHARES THE RULE, NOT THE LOOP. `report_view_census`'s
iteration is inseparable from the per-camera row it prints, so the publisher
walks the cameras itself — but through the same `classify_camera`, so the two can
never disagree about what "renders the world" means. A second copy of the RULE
would be the defect; a second `for` over four cameras is not.

⛔ NEITHER PUBLISHER IS GATED ON `AMBITION_PROFILE_CENSUS`, and that is the
point: that variable gates a stderr printer on a clock nobody asked for, while F1
is something a developer turns on WITHOUT restarting the game with an environment
variable set.

⛔ THE REST OF THE CENSUS WAS NOT CONVERTED, per this section's own instruction.
Portal, asset-decode and rollback paths remain census rows; convert them when a
question needs them, not because they exist.

## Original plan

Ambition already measures considerably more than the FPS overlay exposes.

Relevant code includes:

```text
crates/ambition_render/src/runtime_census.rs
ambition_dev_tools runtime-census machinery
game/ambition_app/src/dev/rollback_observatory.rs
```

The render census already knows about:

* cameras;
* camera roles;
* repeated world rendering;
* offscreen targets;
* portal capture rigs;
* active capture budget;
* resident images;
* cumulative decoded image work;
* retained HUD image hits/loads;
* Bevy render-pass CPU timings;
* Bevy render-pass GPU timings when supported;
* pipeline statistics when supported.

Today much of this exists primarily as periodic text output.

Start publishing the high-value numeric facts into Bevy diagnostics.

A useful first vocabulary would be approximately:

```text
ambition/ecs/entities_non_resource

ambition/render/cameras
ambition/render/world_draws
ambition/render/offscreen_targets

ambition/assets/images_resident
ambition/assets/decoded_megapixels
ambition/assets/decoded_bytes

ambition/portal/capture_rigs
ambition/portal/active_captures

ambition/rollback/current_frame
ambition/rollback/confirmed_frame
ambition/rollback/resimulated_frames
```

Only publish a metric where the owning subsystem already knows the fact.

Do not move calculation into the overlay.

The intended architecture is:

```text
runtime authority / measurement
            ↓
       DiagnosticPath
            ↓
      DiagnosticsStore
        ↙          ↘
 F1 overlay         logging/reporting
```

There should be one measurement and multiple consumers.

This also creates a path to eventually simplify the periodic `[census]` printer around common diagnostic values.

Do not try to convert every census field during this campaign. Select the values that answer common performance/debugging questions.

---

# A5. Surface Bevy render-pass diagnostics  ✔ DONE 2026-08-31 (⚠ REOPENED AND
# CLOSED PROPERLY THE SAME DAY)

⛔⛔ THE FIRST CLOSE WAS WRONG, AND A REVIEW CAUGHT IT. This section was marked
"ANSWERED — nothing to publish" on the reasoning below: the measurements exist
upstream, so A5 is a SELECTION problem rather than a publishing one. Both halves
of that are true. The mistake was stopping there — **the selection was never
made.** `diagnostics_panel.rs` named no `render/*` path at all, so ordinary F1
answered neither of A5's two questions. "The paths exist" and "F1 shows them" are
different claims, and only the first was ever checked.

⭐ WHAT IT DOES NOW. `render_pass_rows` reads `DiagnosticsStore` at the moment
the panel opens and takes every registered `render/**/elapsed_cpu` and
`render/**/elapsed_gpu`, sorted. On a 2026-08-31 desktop bundle that is four
passes — `main_transparent_pass_2d`, `ui`, `msaa_writeback`, `upscaling` — which
is the answer to "which pass is expensive", not a wall.

⛔ IT IS DISCOVERED, NOT HAND-LISTED, AND THAT IS A DEPARTURE FROM THE ORIGINAL
PLAN BELOW. The plan asked for named paths so that adding one is a deliberate
act. But the names are Bevy render-graph node names: a hand-kept list of them
goes stale the moment a pass is renamed or added, and it goes stale SILENTLY, as
a row that reads `Missing` forever and that nobody re-reads. Filtering the store
to the two timing suffixes keeps the "not a wall" property — pipeline statistics
like `vertex_shader_invocations` are excluded — without a list to maintain.

⛔ AN ORDINARY RUN STILL SHOWS NONE OF THEM, AND THAT IS THE HONEST ANSWER,
NOT A GAP. `RenderDiagnosticsPlugin` is what registers those paths, and Ambition
installs it only under `AMBITION_PROFILE_CENSUS`, because it adds GPU-timestamp
and pipeline-statistics queries to every pass. Making it unconditional to fill in
a dashboard would charge every session for a number almost nobody is reading —
and its cost has not been measured. So render-pass timing remains a
PROFILING-MODE capability; F1 surfaces it in that mode, and
`render_diagnostics.csv` in a profile bundle remains the fuller answer (16 paths
with a time series, against F1's instantaneous 8).

⚠ GPU timestamps are backend-dependent. An unregistered path renders as
`Missing`, which is the required "measurement unavailable" and never a zero.

## Original plan

`PresentationCensusPlugin` already conditionally installs:

```text
bevy::render::diagnostic::RenderDiagnosticsPlugin
```

and already knows how to enumerate:

```text
render/.../elapsed_cpu
render/.../elapsed_gpu
```

when those measurements exist.

Investigate showing a small useful subset in the F1 diagnostic panel.

Do not create a giant dynamically changing wall of every render diagnostic.

Start with facts that answer:

```text
Is the frame CPU-bound or GPU-bound?
Which major render pass is expensive?
Are portal/offscreen cameras causing repeated rendering?
```

GPU measurement availability must remain explicit.

No GPU timestamp support means "measurement unavailable", not zero GPU cost.

---

# B. Replace F1 label entities with text gizmos  ✔ DONE 2026-08-31

`render_debug_overlay_labels` draws `Gizmos::text_2d` instead of despawning and
respawning a `Text2d` entity per label per frame. Gone with the entities: the
spawn churn, the despawn sweep, the `DebugOverlayLabel` marker, the lifetime <!-- cite-ok: the row records what the port DELETED -->
bookkeeping, and — the one that mattered — the dependency of F1 world labels on
the PRODUCT font stack. A developer overlay should not be able to fail because a
typeface has not finished loading. The one-frame `DebugOverlayLabels` scratch
buffer stays, which this section explicitly permits.

⛔ THIS SECTION'S LEGIBILITY CLAIM IS WRONG, AND CHECKING IT SAVED A REGRESSION.
It says gizmo text size is expressed in screen-space pixels. It is not:
`text_sections_2d` scales the glyph outline by `font_size / SIMPLEX_CAP_HEIGHT`
and then puts every point through `isometry * point` — a WORLD-space isometry —
so `font_size` is world units exactly as `Text2d`'s was.

⭐ MEASURED, not just read, because a coherent source reading is not a
measurement. The same scene captured at two camera zooms (960x540 both,
`--combat-overlay`, point focus vs `--fit-room`):

| | `player` label | player collision box |
|---|---|---|
| near | 45 x 13 px | 50 x 52 px |
| fit-room | 10 x 1 px | 8 x 11 px |
| shrink | **4.5x** | **4.7x** |

The label scaled with the world by the same factor as the geometry. Screen-space
sizing would have left it 45x13 while the box shrank to 8x11.
`DEBUG_LABEL_FONT_PX = 7.0` therefore carries over unchanged. Had the claim been
believed, the constant would have been "converted" and every label resized.

⚠ THE GRAIN OF TRUTH: gizmo LINE WIDTH *is* screen-space pixels, which is why
the far label measures 10px wide but only 1px tall — the glyph geometry shrank
and the stroke thickness did not. At extreme zoom-out the labels degrade to a
smear, exactly as the `Text2d` ones did. Upstream invites the confusion too: the
doc comment on the 3D `text_sections` says "the size of the text in pixels",
while the 2D one says only "the size of the text".

## Original plan

Bevy 0.19.1 added world-space debug text through `Gizmos::text` / `text_2d` using a built-in stroke font.

This fits Ambition's F1 diagnostics directly.

Current implementation:

```text
game/ambition_app/src/dev/debug_overlay.rs
game/ambition_app/src/dev/debug_overlay/prims.rs
```

currently:

1. queues `DebugLabel`s into `DebugOverlayLabels`;
2. despawns the previous frame's `Text2d` labels;
3. spawns new `Text2d` entities;
4. relies on normal Bevy text/font presentation.

Replace this with text gizmos.

## Benefits

This should remove:

* per-frame debug-label entity spawning;
* per-frame debug-label despawning;
* `DebugOverlayLabel`; <!-- cite-ok: under 'This should remove:' — the plan's intent, not a live name -->
* product font resolution from F1 world labels;
* font atlas/glyph asset dependencies from these labels;
* label lifetime bookkeeping.

The text-gizmo font is deliberately ASCII-only.

That is appropriate here.

Keep user-facing text on the real typography stack.

## Implementation options

Prefer direct immediate-mode rendering if the call structure stays readable:

```text
draw box
draw label
```

If changing every debug helper to accept text-gizmo state creates excessive churn, keeping the one-frame `DebugOverlayLabels` scratch buffer is acceptable:

```text
geometry systems
    ↓
DebugOverlayLabels scratch
    ↓
one system emits gizmo text
```

The important removal is the retained/spawned `Text2d` entity layer.

## Legibility

Text gizmo size is expressed in screen-space pixels.

That is likely better for debug labels than the current world-space text whose readability changes with camera zoom.

Test:

* normal room;
* boss zoom;
* Smash camera;
* split/local view if supported;
* portal-debug geometry.

Labels should remain associated with their boxes and remain readable.

---

# C. Delete `MenuTextHeightFraction`  ✔ DONE 2026-08-31 <!-- cite-ok: the heading names the symbol this section deleted -->

Landed. `MenuNode::Text`/`DynamicText`'s `size` is spawned as `FontSize::Vh`
directly, and the unit is now documented on the FIELD — the place the original
five-pixel bug came from having no documented unit at all. Deleted:
`MenuTextHeightFraction`, `MENU_REFERENCE_VIEWPORT_HEIGHT`, <!-- cite-ok: the deletion inventory for this section -->
`resolve_menu_text_size`, `install_bevy_ui_menu_text_scaling`, its <!-- cite-ok: the deletion inventory for this section -->
`MenuTextScalingInstalled` marker, the `before(UiSystems::Content)` constraint <!-- cite-ok: the deletion inventory for this section -->
that existed only for it, and the three unit tests of the conversion arithmetic.

⭐ THE ENGINE RESOLVES AGAINST THE UI RENDER TARGET, NOT THE PRIMARY WINDOW, and
for Ambition those are the same thing: menu UI resolves to the order-9
`FrontHudCamera` (`IsDefaultUiCamera`), which carries no viewport override.
Split-screen viewports are set on the GAMEPLAY cameras only. Verified by reading
`host_presentation_scaffold` and `gameplay_presentation`.

⛔ ONE BEHAVIOUR DID CHANGE, and it only ever affected tests: with no window the
old system fell back to a 1080 reference height, so a windowless composition drew
legible text. `Vh` against a 0x0 target is 0px, which Bevy warns about and skips.
`VisibleRenderMode::NoWindow` composes no window AND no render app, so nothing
was being presented there anyway — measured, every UI target in that harness
reads 0x0. The launcher legibility test now states its own reference viewport
and resolves through `FontSize::eval`, which is the call Bevy's text pipeline
makes.

The acceptance test (`menu_text_is_sized_as_a_percentage_of_the_live_viewport`)
compares TWO viewport heights and runs no system of Ambition's own. Poisoning the
spawn back to `Px` fails it; poisoning it to the wrong axis (`Vw`) fails it on
the value, because the role filter asks for "not `Px`" rather than "is `Vh`".

---

## Original plan

This is the cleanest non-diagnostic 0.19 cleanup.

Current code owns a unit that Bevy now owns:

```text
MenuTextHeightFraction
```

and a system converts that viewport-height fraction into `TextFont` pixel values.

Bevy 0.19.1 provides:

```text
FontSize::Vh
```

directly.

Replace menu-authored viewport-height text sizes with `FontSize::Vh`.

Delete:

* `MenuTextHeightFraction`; <!-- cite-ok: under 'Delete:' — the plan's intent, not a live name -->
* the per-frame window-height conversion system;
* scheduling required only for that conversion;
* resize handling that exists only to recalculate font pixels;
* tests whose sole purpose is the custom conversion.

Keep tests that verify actual authored sizing semantics.

Acceptance should compare at multiple window sizes and prove the menu scales as before.

---

# D. Harden resources-as-components semantics  ✔ DONE 2026-08-31 (the census; the campaign's broader sweep stands)

The `runtime_census` defect is fixed. The four populations moved behind one
`EcsPopulation` system param so the `Without<IsResource>` cannot be forgotten by
the next reader who wants an entity count, and `resources=` is reported BESIDE
`live=` rather than folded into it. The regression the section asks for exists
and has both arms — adding a resource must move `resources` and not `live`, and
spawning an entity must do the opposite — and is poison-verified.

The sweep found no second production defect: the four other `Query<()>` and the
one `Query<Entity>` in the tree are all `get(known_entity)` existence checks,
where a resource entity can never be the argument. The one `entities().len()`
use in a test is a before/after delta, which a constant offset cancels.

⛔ NO GREP PROHIBITION WAS ADDED, per this section's own last line: the five
legitimate uses would each need a waiver, and the discriminator that actually
matters is not the query TYPE but whether it is ITERATED. `EcsPopulation` is the
semantic pin instead.

---

## Original plan

Bevy 0.19 makes resources components stored on singleton resource entities.

This has already produced one port regression and there is another remaining measurement issue:

`runtime_census` uses a broad:

```text
Query<()>
```

to mean "live game entities".

That now includes resource entities.

Fix it using the Bevy 0.19 resource marker, e.g. the appropriate:

```text
Without<IsResource>
```

filter.

Then sweep broad population queries whose meaning is:

```text
all gameplay / scene entities
```

Pay particular attention to:

```text
Query<()>
Query<Entity>
EntityRef / EntityMut iteration
world.iter_entities*
raw world entity counts
```

Do not change targeted queries that happen to include resources intentionally.

Add at least one regression proving that inserting another Resource does not change the reported scene-entity count.

A large grep-based prohibition is probably too coarse. Pin semantic consumers instead.

---

# E. Add explicit render recovery  ✔ DONE 2026-08-31

`host::render_recovery` installs an explicit `RenderErrorHandler` on the WINDOWED
host only — `NoWindow` has no render app to lose, and `OffscreenGpu` is a capture
tool whose right answer to a dead device is to fail the run loudly rather than
quietly rebuild one and hand back a picture of something else.

| category | response |
|---|---|
| `DeviceLost` | recover, bounded at 2 attempts per run, then stop |
| `OutOfMemory` | stop rendering — recovering would re-allocate exactly what did not fit |
| `Validation` | quit; wgpu says the engine used it wrongly, and that will not fix itself |
| `Internal` | quit loudly |

⛔⛔ THE DEFAULT WAS "QUIT ON EVERYTHING", INCLUDING `DeviceLost` — routinely a
driver reset or a laptop waking from sleep, and the one category a game can
survive. Upstream's own comment calls it "overzealous". Inheriting it was a
decision nobody had made.

⛔ `Ignore` APPEARS NOWHERE. It is the one policy that re-runs the frame that
just failed, which is the shape upstream warns can produce hazardous rapid
flashing. Recovery is COUNTED instead, in a main-world resource — the render
world is torn down by the very recovery that would reset a tally living there.

⭐ THE POLICY IS A PURE FUNCTION, so its test needs no GPU: `response_for` takes
the error category and the recoveries-so-far. Forcing a real device loss needs a
cooperative backend, and a policy nobody can check is a policy nobody checks.

⭐ NO CATCH-ALL ARM, AND THE COMPILER SETTLED IT. The first draft had
`_ => Quit` arguing that a wgpu upgrade could add a category; the arm was
unreachable, because `ErrorType` is not `#[non_exhaustive]`. Exhaustiveness is
the stronger guard: a new wgpu category now breaks the build, which is a person
reading it and deciding, instead of a silent default.

## Original plan

Bevy 0.19.1 exposes typed render failures through:

```text
RenderErrorHandler
RenderErrorPolicy
ErrorType
```

Ambition should install an explicit policy rather than inherit whatever Bevy's default happens to be.

This belongs in the visible host/platform composition.

Start conservatively.

Recommended initial policy:

```text
DeviceLost
    → log structured diagnostic
    → attempt renderer recovery

OutOfMemory
    → log prominently
    → stop rendering or exit through a deliberate product policy

Validation
    → preserve Bevy's sensible handling unless Ambition has stronger evidence

Internal
    → fail loudly
```

Do not endlessly recover from repeated device-loss/OOM loops.

If practical, keep a small host-side recovery counter so repeated failures within one run escalate rather than producing repeated flashing.

This is presentation/platform state and must not enter rollback simulation.

A pure policy test should pin the response chosen for each Bevy error category.

An actual forced device-loss test is optional if the backend makes it impractical.

---

# F. Evaluate `SettingsPlugin`, but do not force a migration  ✔ SPIKED 2026-08-31 — DO NOT MIGRATE

Spiked against `bevy-settings 0.19.1` (1053 + 137 + 90 lines, read in full). The
decision gate says migrate only if the Bevy mechanism preserves Ambition's
important semantics AND deletes a meaningful amount of code. It does neither.

| requirement | bevy-settings | verdict |
|---|---|---|
| existing RON compatibility | **TOML** (`settings.toml`) | ⛔ every existing `settings.ron` is orphaned |
| schema evolution / defaulting | reflection + `ReflectDefault` | ⛔ cannot express "legacy `crt: bool` → `crt_strength: f32`", which `ScreenShaderSettings` does today during clamping |
| `clamp_all()` normalization | none; reflected values applied directly | ⛔ needs an adapter pass |
| test-specific `PersistenceRoot` | **none** — `SettingsStore::new` derives `base_path` from `preferences_dir()` and takes only `app_name` | ⛔⛔ FATAL, see below |
| atomic / crash-resistant save | temp file + rename | ✔ same as Ambition's |
| deferred / debounced writes | `SaveSettingsDeferred` | ✔ |
| synchronous save on exit | `SaveSettingsSync::IfChanged` | ✔ |
| desktop / Android paths | platform preferences dir | ✔ |
| **web persistence** | `store_wasm.rs`, browser local storage | ✔ the one thing Ambition lacks |

⛔⛔ THE TEST-ISOLATION ROW IS THE ONE THAT ENDS IT. `PersistenceRoot::isolated()`
exists because every `app_it` test shared three mutable files with every other
test, every worktree and every concurrent session on the machine — a headless
acceptance run could overwrite a real save. bevy-settings offers no root
override at all: the base path IS the user's preferences directory, and the only
input is `app_name`. Smuggling a unique name in still writes under the player's
config dir. Adopting it would delete a capability this repository added
deliberately, to fix a defect it had already been bitten by.

⛔ AND IT WOULD NOT DELETE CODE, IT WOULD ADD ADAPTERS: a RON→TOML importer, a
post-load `clamp_all` pass, a legacy-field migration the reflection path cannot
express, and `Reflect` on the whole `UserSettings` tree (which is serde-shaped
today). That is precisely the "pile of adapters" the gate names.

⭐ THE PRIZE IS SEPARABLE, AND THAT IS THE FOLLOW-UP. Browser persistence is
`store_wasm.rs` — **90 lines** over `window().local_storage()`, keyed
`{app_name}-{filename}`. Ambition can add a wasm arm to its OWN persistence
behind the existing `PersistenceRoot` seam and get web settings without adopting
TOML, reflection, or losing test isolation. That is a small, self-contained
follow-up and it is where the value in this section actually is.

## Original plan

Bevy 0.19.1 now has first-party persistent app settings.

Ambition already has a mature settings implementation.

The correct campaign task is therefore a compatibility spike, not an assumption that upstream must replace it.

Compare Bevy `SettingsPlugin` with the current:

```text
crates/ambition_persistence/src/settings/
```

against these requirements:

* existing RON compatibility;
* schema evolution/defaulting;
* `clamp_all()` normalization;
* test-specific `PersistenceRoot`;
* atomic/crash-resistant save behavior;
* deferred/debounced writes;
* synchronous save where shutdown requires it;
* desktop paths;
* Android paths;
* web persistence;
* preservation of existing user settings.

Web support is particularly interesting because Ambition's current persistence code explicitly leaves browser persistence for later, while Bevy's settings framework was designed to abstract persistent application settings across platforms.

## Decision gate

Only migrate if the Bevy mechanism can preserve Ambition's important semantics while deleting a meaningful amount of code.

If doing so requires replacing a clean tested persistence abstraction with a pile of adapters, stop after the spike and record why.

A successful spike can become its own follow-up campaign.

Do not let settings migration block the diagnostics work.

---

# G. Small post-processing audit

## ✔ The migrated pass is MEASURED WORKING on a real GPU (2026-08-31)

The port rewrote `screen_effects.rs` from a hand-maintained render-graph node onto
`FullscreenMaterialPlugin`, and nothing tested it — a clean compile cannot catch a
shader that fails to load, wrong bindings or a pass that never executes.

Three arms of `capture_scene hall_of_characters player 640x360 --warmup 60`, on
`llvmpipe`, `AMBITION_QUALITY_PROFILE=ultra`:

| arm | md5 |
|---|---|
| no effect | `30372c4b715e7f699aae56a3617407e0` |
| `--screen-effect crt,vignette` | `e7e2ec88957e1a4d1ff7a02b0ea991a3` |
| the same again | `e7e2ec88957e1a4d1ff7a02b0ea991a3` |

100% of pixels changed, mean absolute difference 46/255, and the corner-to-centre
luminance ratio fell 0.609 → 0.053 — which is the vignette's OWN signature, not
merely "different". The picture shows scanlines, the RGB mask, barrel curvature
and chroma split. The third arm is the determinism control: a repeat is
byte-identical, so arm 2's difference is the effect and not capture noise.

⛔⛔ THE FIRST TWO ATTEMPTS MEASURED NOTHING, and both looked like a finding.
Screen effects are `UserSettings.video.shaders` state and a windowless host
inserts `PersistenceRoot::isolated()`, so a hand-written `settings.ron` is never
read; and the quality budget scales screen shaders to ZERO on the Potato tier,
which is what `llvmpipe` gets seeded to. `capture_scene --screen-effect` exists
because of that, and it prints a refusal when the tier would zero the shaders
rather than returning a plausible frame.

⛔ NO GPU TEST WAS ADDED. Nothing in this repository's suite touches a GPU —
every `OffscreenGpu` user is a tool binary — and a first one would bring adapter
availability and ~40s of runtime into the gates. The check is a documented,
repeatable tool invocation instead: see `docs/recipes/headless-room-verification.md`.

## ✔ The no-op pass no longer runs (2026-08-31)

Answering this section's one question — *can any Ambition custom code disappear
without adding another fullscreen pass?* — the answer is **no**, and the combined
shader is left alone. Bevy's built-in vignette and lens distortion would each be
another pass; splitting one combined pass into three to use newer APIs would buy
API modernity with GPU work.

The other half of the section is done. `FullscreenMaterialPlugin` uses component
PRESENCE as the enable: without `ScreenEffectSettings`,
`prepare_fullscreen_material_pipelines` drops the pipeline id, the bind-group
preparation drops the bind group, and `fullscreen_material_system`'s view query
does not match — so the pass and the `post_process_write` ping-pong it forces do
not happen at all. Eligibility moved to a `ScreenEffectCamera` marker (which
nothing removes), and `sync_screen_effect_settings_from_video_settings` now adds
and removes the settings component.

⛔⛔ THE DEFAULT CONFIGURATION WAS PAYING FOR A COPY. `default_shader_strength()`
is **0.0**, so the shader's very first statement — `if global_strength <= 0.001 {
return sample_screen(in.uv); }` — was the branch every shipped frame took. A
full-screen read and a full-screen write to return the pixel it read.

⭐ PROVED PIXEL-IDENTICAL, which is the only claim that matters for a change that
removes a pass. Same two captures as above, before and after:

| arm | before | after |
|---|---|---|
| no effect | `30372c4b715e7f699aae56a3617407e0` | **same** |
| `--screen-effect crt,vignette` | `e7e2ec88957e1a4d1ff7a02b0ea991a3` | **same** |

⛔ NO FRAME-TIME CLAIM IS MADE. What is established is the MECHANISM (from Bevy's
own source, and it is Bevy's documented enable) and that the picture is
unchanged. The MAGNITUDE is unmeasured: the only GPU here is `llvmpipe`, where a
saved fullscreen pass is worth far more than on the hardware a player has, so a
number taken here would flatter the change. Measure it on real hardware before
quoting one.

Three unit tests carry the behaviour: enrol/retract across off → on → off, a
marked-camera-only guard, and a Potato-tier arm. Each of the last two carries a
PREMISE GUARD, and they earned it — `strength` is a second gate whose default is
0.0, so the first drafts asserted an absence the settings alone produced.

## Original plan

Bevy 0.19 now provides built-in vignette and lens-distortion effects.

Ambition currently has one combined fullscreen shader covering:

* CRT;
* film grain;
* vignette;
* robot death;
* underwater;
* deep dream.

Do not split that single combined pass into several Bevy passes just to use newer APIs.

That could increase GPU work.

Instead answer one question:

> Can any Ambition custom code disappear without adding another fullscreen pass?

If the answer is no, leave the combined effect alone.

One improvement is still worth implementing if it is straightforward:

when every screen effect is disabled, do not keep a no-op fullscreen post-process active.

Prefer component presence to mean that the camera actually needs the effect.

Measure or inspect the resulting render path before claiming a performance improvement.

---

# H. Optional 0.19.1 extras if the core campaign finishes early

These are candidates, not required work.

## H1. Render debug overlay

Bevy dev tools can visualize renderer data such as depth, normals, motion vectors and deferred buffers.

This may be useful for:

* kaleidoscope/3D UI rendering;
* custom material investigation;
* capture/render bugs.

It is less valuable for normal 2D gameplay than Ambition's semantic combat/world overlays.

If added, put it behind a separate developer action rather than overloading normal F1 immediately.

## H2. `Rem` UI scaling

Bevy's:

```text
FontSize::Rem
RemSize
```

provide a clean global text-scale authority.

This becomes worth implementing when Ambition adds a user-facing UI/accessibility scale setting or needs a dedicated Steam Deck readability profile.

Do not mass-convert existing UI sizes during this campaign without that product requirement.

## H3. Observer run conditions

Observers can now have `run_if` conditions.

When touching an observer that currently begins by manually checking:

```text
if debug disabled { return; }
```

or another clean resource condition, consider moving that gate onto the observer.

Do not sweep the entire repository merely to use the API.

## H4. Contiguous query access

Bevy 0.19 can expose dense table component data through contiguous query access for better vectorization.

Ambition should not launch a speculative CPU optimization sweep.

Only try this on a measured hot loop where:

* the query is dense;
* profiling identifies it as meaningful;
* semantics around change detection are understood.

The standing rule remains measurement first.

## H5. Asset saving / transform gizmos

These may eventually help authoring tools.

They are not part of this campaign unless an existing Ambition tool already needs the exact capability.

Do not grow an in-engine level editor simply because Bevy gained editor primitives.

---

# Work order

## Checkpoint 1 — global observability

Complete together:

* upstream FPS overlay;
* global `show_fps` behavior;
* global F1 hotkey handling;
* initial `DiagnosticsOverlay`;
* FPS/frame-time/system diagnostics.

At this checkpoint a developer should be able to start the game and see useful performance information before entering a gameplay session.

## Checkpoint 2 — Ambition diagnostics

Complete:

* selected runtime-census values published as diagnostics;
* resource-aware entity count;
* render diagnostics exposed where supported;
* optional rollback numeric diagnostics;
* F1 panel consuming these values.

At this checkpoint the screen overlay and command-line profiling tools should increasingly consume the same measurements.

## Checkpoint 3 — F1 visual cleanup

Complete:

* debug labels converted to text gizmos;
* `Text2d` label lifecycle removed;
* font dependency removed from world-space F1 labels.

## Checkpoint 4 — small Bevy-native deletions

Complete:

* ✔ `MenuTextHeightFraction` → `FontSize::Vh` (2026-08-31); <!-- cite-ok: a ✔ migration row: old name → new, the old one is meant to be gone -->
* ✔ no-op screen-effects pass avoided (2026-08-31);
* ✔ render recovery installed (2026-08-31).

## Checkpoint 5 — persistence decision

Perform the `SettingsPlugin` spike.

Either:

* adopt it because the migration is clearly smaller/stronger;

or:

* document the mismatched requirements and retain Ambition persistence.

Both are valid outcomes.

---

# Validation matrix

The diagnostics work is incomplete until it is tested through actual application routes.

At minimum exercise:

| State             | FPS with `show_fps=true` | F1 numeric panel | F1 world gizmos                                                  |
| ----------------- | ------------------------ | ---------------- | ---------------------------------------------------------------- |
| Startup/loading   | visible                  | available        | no session facts                                                 |
| Launcher/title    | visible                  | available        | no session facts                                                 |
| Ambition gameplay | visible                  | available        | available                                                        |
| Smash gameplay    | visible                  | available        | available where supported                                        |
| Grid pause menu   | visible above menu       | available        | underlying world may remain visible according to existing policy |
| Kaleidoscope menu | visible above cube UI    | available        | existing policy                                                  |
| Dialogue/cutscene | visible                  | available        | existing policy                                                  |
| Room transition   | visible                  | available        | only facts whose authority exists                                |
| No active session | visible                  | available        | absent                                                           |

Also test:

```text
show_fps default = true
show_fps false -> hidden everywhere
F6 toggles show_fps
F6 persistence survives restart
F1 works without a session
F1 does not mutate gameplay simulation
additional Resource insertion does not inflate scene entity census
text-gizmo labels require no font asset
```

Where web and Android toolchains are available, compile/test those paths too.

---

# Architectural constraints

Do not:

* make diagnostics another source of gameplay truth;
* rederive hit geometry, contacts, room state or rollback facts in UI code;
* make FPS visibility depend on session lifetime;
* make FPS visibility depend on visual quality;
* use text gizmos for product/UI/dialogue text;
* expose unsupported GPU measurements as zero;
* put diagnostic/presentation state into rollback snapshots;
* add heavyweight per-frame measurement merely because the F1 panel can display it;
* replace the combined screen-effects shader with multiple passes without evidence;
* use Bevy delayed commands for rollback-sensitive gameplay timing;
* turn the settings spike into a forced persistence rewrite.

The intended direction is:

```text
real runtime facts / measurements
              ↓
       shared diagnostic paths
              ↓
         DiagnosticsStore
          ↙          ↘
    F1 display      logs/tools
```

and separately:

```text
semantic debug geometry
              ↓
          Bevy Gizmos
       shapes + ASCII labels
```

The campaign succeeds when diagnostics become easier to see and there is less Ambition-specific presentation machinery maintaining them.

---

# Expected cleanup targets

Successful completion should make some or all of these removable:

```text
game/ambition_app/src/dev/fps_overlay.rs
    custom FpsOverlayState
    custom FPS text entity
    custom text refresh formatting
    custom visibility system
    custom positioning system

game/ambition_app/src/dev/debug_overlay.rs
    DebugOverlayLabel entities
    per-frame Text2d label despawn/spawn

game/ambition_app/src/dev/debug_overlay/prims.rs
    font-dependent debug-label presentation

ambition_menu                                   ✔ REMOVED 2026-08-31
    MenuTextHeightFraction
    viewport-height -> pixel conversion system
```

The exact module files do not have to disappear if thin integration code remains.

What should disappear are the custom mechanisms Bevy 0.19.1 now supplies directly.
