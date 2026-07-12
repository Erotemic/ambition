# Loading and game-shell architecture
> **Status:** core-tools checkpoint. All executable slices remain **OPEN** until
> their acceptance tests pass. No game/demo/subsystem integration has landed.
## Maintainer intent
- Loading screens should be nonexistent when preparation finishes quickly.
- When waiting is unavoidable, the engine should honestly report what is done,
  what is active, how much known work remains, and what it estimates is still undiscovered.
- Presentation may show an estimated percentage, exact step counts, both, or an
  indeterminate view. Presentation never becomes readiness authority.
- Arbitrary playable loading activities are first-class. If the player engages
  with one, a ready destination may wait for explicit confirmation while the activity continues.
- Minimal, unpolished implementations must be complete enough for Sanic,
  Mary-O, new games, tests, and Ambition itself. Polish replaces presentation; it must not create a second
  loading or shell path.
- Boot is configuration, not a special application architecture. A process may
  enter a vanity sequence, a menu, gameplay, credits, or a prefab cutscene.
Priority order:
1. avoid waits through retention and prefetching;
2. keep required preparation asynchronous;
3. stream optional work after activation where safe;
4. reveal a waiting experience only after the relevant barrier misses its
   latency budget;
5. never use an attractive minigame to excuse avoidable stalls.
## Binding crate carve
Add exactly these engine crates for the initial implementation:
```text
crates/ambition_load
crates/ambition_game_shell
crates/ambition_load_presentation
```
Do **not** initially add `ambition_loader`, `ambition_load_backend`, `ambition_load_frontend`,
`ambition_boot_sequence`, `ambition_vanity_card`, `ambition_load_activity`, or
`ambition_presentation_sequence`.
### `ambition_load`
> Owns headless asynchronous load coordination, work evidence, activation
> barriers, streaming/prefetch roles, cancellation, supersession, failure, and
> readiness. It does not perform subsystem-specific work or render anything.
It is named `load`, not `loader`, because Bevy uses “loader” for concrete asset format/source loaders.
This crate coordinates assets, save decoding, world construction, procedural work, required pipeline
warmup, and other contributors. It must remain usable by headless binaries and tests. It must not depend
on `ambition_render`, `ambition_menu`, game content, or `ambition_app`.
### `ambition_game_shell`
> Owns renderer-independent selection and lifecycle of top-level player-facing
> experiences, including configurable process entrypoints and routing among
> menus, presentation sequences, gameplay sessions, credits, top-level
> cutscenes, and recovery states.
The shell is not a universal gameplay state machine. It does not own room portals, combat modes,
inventories, dialogue, pause overlays, ordinary in-session cutscenes, or boss phases. It may depend on
`ambition_load`, `ambition_menu`, input/navigation foundations, and the minimum machinery required for
scoped experience lifecycles. It must not know Ambition-specific route IDs, branding, menu content, or
cutscene IDs. Its core plugin and semantic model remain render-free. The same crate may expose an
optional `basic_presentation` module/feature containing `BasicShellPresentationPlugin`; headless users
do not enable that feature.
### `ambition_load_presentation`
> Provides a replaceable shell-integrated waiting experience for unresolved load
> barriers, including a complete minimal loading presentation, optional
> arbitrary foreground activities, engagement, ready-hold, Continue, failure
> actions, and deterministic cleanup.
It depends on `ambition_load` and `ambition_game_shell`; the shell does not depend on it. It may depend
on presentation/input/audio crates because it is a presentation-layer package. Headless systems use
`ambition_load` without it. The activity-host module must be intentionally extractable. Do not create an
activity crate until loading activities, practice modes, or title-screen toys prove substantial shared
mini-session machinery.
## Constitutional dependency shape
```text
Bevy AssetServer / bevy_asset_loader / save / world / content contributors
                              |
                              v
                       ambition_load
                  work facts and barriers
                       /           \
                      v             v
          ambition_game_shell   headless clients
                      ^
                      |
          ambition_load_presentation
                      ^
                      |
       game policy, styling, activities, content
                      |
                      v
                    app
```
Rules:
1. Bevy loads assets; `ambition_load` explains what the game is waiting for.
2. Contributors depend on the load-reporting protocol. The load crate does not
depend upward on every contributor.
3. The shell owns top-level routing, not the internals of registered experiences.
4. Load presentation consumes evidence; it never manufactures readiness.
5. `ambition_app` composes plugins but owns no reusable semantics.
6. Standalone demos must use the same crates without depending on
`ambition_app`.
## Core model: load plans, work, and barriers
A **load plan** groups related work. A **barrier** answers whether a particular activation is currently
safe. Background streaming may continue after a barrier opens. “Blocking” means **blocks activation**,
never “blocks the main thread.” Work has at least two independent policy axes:
```rust
pub enum ActivationRequirement {
    RequiredFor(LoadBarrierId),
    Degradable,
    Speculative,
}
pub enum LoadPriority {
    Immediate,
    High,
    Normal,
    Low,
}
```
Examples:
| Work | Requirement | Typical priority |
|---|---|---|
| save header, collision geometry, player definition | required | immediate |
| required sprite fallback and room entities | required | high |
| distant art, ambient audio, high-resolution variants | degradable | normal/low |
| likely next room or title-menu Continue target | speculative | high/normal |
A speculative or degradable item may be promoted when the player chooses a route that requires it.
Promotion reuses stable work identity and existing progress; it does not restart or duplicate the load.
Possible barriers include `BootRenderable`, `FrontendReady`, `SessionActivatable`, and game-owned
region/room barriers. These are examples, not one engine-owned exhaustive enum.
### Transaction safety
Every plan/request has stable identity. Cancellation and supersession must make late results harmless. A
replaced request can never activate after its replacement. A route provider owns the actual
destination/session activation. The load crate owns evidence that the named barrier is open and that the
request is still current. Activation is one explicit, idempotence-tested commit on a clean frame. Commit
must not leave two active sessions, a half-active destination, or an activity that still owns input. A
failure remains explicit and recoverable.
## Honest work accounting
The engine publishes **facts**, **estimates**, and **uncertainty** separately. A work step is
player-meaningful, not one future, file, asset handle, or ECS command. Examples include resolving an
asset profile, decoding region geometry, constructing required entities, and warming required pipelines.
```rust
pub enum WorkStepState {
    Planned,
    Running,
    Complete,
    Failed,
    Cancelled,
    Skipped,
}
pub enum WorkMetric {
    Discrete { completed: u64, total: u64 },
    Bytes { completed: u64, total: Option<u64> },
    Items { completed: u64, total: Option<u64> },
    Fraction { completed: f32 },
    Indeterminate,
}
```
Bytes, items, fractions, and step counts are local evidence; they are not naively additive. `Fraction`
requires a defensible contributor estimate. The barrier snapshot exposes at least:
- completed, active, and known-remaining required steps;
- whether discovery is still open;
- current stage and active labels;
- optional forecast of additional undiscovered steps;
- optional remaining-effort range, confidence, and provenance;
- failures and retryability;
- exact barrier readiness.
A useful discovery-open report is:
```text
12 complete · 2 active · 5 known remaining
approximately 2–6 additional steps may still be discovered
```
After discovery closes, known remaining is exact for that plan. One step may outweigh ten others, so
estimated effort remains distinct from step count. Estimator inputs may include exact bounded work,
authored phase weights, contributor estimates, and later historical telemetry. Raw estimates may move
backward when work is discovered. They never control readiness.
## Progress presentation
`ambition_load` publishes a semantic snapshot. Presentation chooses among:
- no UI;
- current stage only;
- exact step counts;
- estimated percentage;
- percentage plus steps;
- detailed/debug evidence;
- indeterminate progress.
A percentage is permitted and useful, but its uncertainty remains representable. Presentation may smooth
or keep a displayed estimate monotonic, while debug views retain the raw estimate. Binding rules:
1. never display 100% before the barrier is open;
2. reserve uncertainty while discovery remains open;
3. mark low-confidence values as estimated (`about 68%`, not `68.00%`);
4. prefer stages/steps when a percentage would be mostly invented;
5. show active work when an estimate appears stalled;
6. never call degradable/streaming work activation-blocking;
7. failure replaces progress rather than hiding behind endless animation;
8. presentation output never feeds back into readiness.
## Shell routes, entrypoints, and experiences
A process starts from a configurable shell entrypoint:
```text
development build -> gameplay
normal release    -> startup sequence -> initial menu
credits preview   -> credits -> title menu
cutscene tool     -> prefab cutscene -> exit
benchmark         -> prepared scenario
```
The engine owns stable route/experience IDs, lifecycle, focus, completion, failure, and scoped cleanup.
Games own route names, parameters, and policy. A shell host configures **separate initial and home
routes**:
```rust
pub struct ShellHostSpec {
    pub initial_route: ShellRouteId,
    pub home_route: ShellRouteId,
}
pub enum ShellCommand {
    GoTo(ShellRouteRequest),
    ReplaceWith(ShellRouteRequest),
    Return,
    QuitToHome,
    ExitProcess,
}
```
`initial_route` answers what this binary enters first. `home_route` answers where an active game/demo
returns when it emits the host-independent `QuitToHome` command. They may differ: standalone Sanic may
enter gameplay directly yet return to its barebones Sanic launcher; Ambition may enter its startup
sequence and use the Ambition launcher as home.

Use ECS messages/components/plugins rather than assuming Rust traits. An experience owns a scoped
entity/resource/input lifetime and reports semantic completion/failure/navigation. It may request
`QuitToHome`, but it must not know or name its host's menu route. The host resolves the command, revokes
gameplay input, retires the active session, cancels session-scoped load/stream work, restores shell
focus, and activates `home_route` exactly once.

The shell hosts one primary top-level experience and may host a shell-managed foreground such as a
loading presentation. A foreground need not destroy the current gameplay experience; this permits
in-session waits without routing every room transition through the application shell.
## Standard shell sequence
Boot is a configured shell route implemented by a reusable ordered sequence, not a separate crate. The
initial neutral sequence runner lives in `ambition_game_shell::sequence` and is designed for later
extraction only after a second substantial consumer proves the same abstraction. A segment has a
semantic role, an implementation ID, and policy. The role does not dictate media format.
```rust
pub enum ShellSegmentRole {
    Vanity,
    Notice,
    TitleReveal,
    CreditsSection,
    Custom(ShellRoleId),
}
```
A vanity segment may be a text card, static image, image sequence, video adapter, shader scene, 3D
scene, or arbitrary registered Bevy program with entities, systems, audio, and local input. The shell
owns ordering, skip/cancel policy, completion, and cleanup. The segment owns its scoped implementation
and reports ready-to-skip, completed, or failed. It never chooses the next route or sequence index.
Provide low-cost standard helpers for:
- text card;
- static image;
- image sequence;
- timed hold/fade;
- explicit acknowledgement;
- load-barrier wait;
- registered programmatic segment.
Video remains an optional adapter so minimal demos do not inherit decoding and platform dependencies.
### Vanity/startup route
A normal startup route may sequence branding, notices, and a title reveal while `FrontendReady` work
proceeds underneath. Never prolong a vanity segment solely to hide loading. When the sequence ends,
route to the menu, show a load foreground if frontend readiness is unresolved, or show recovery on
failure.
### End credits
Credits are a shell experience triggered by game policy after a semantic ending event. Gameplay reports
the ending; it does not spawn credits UI or route the shell directly. Initially credits remain
game-owned and may use the standard sequence runner or a custom experience. On completion/skip they
route to a configured postgame or title state. Add an `ambition_credits` crate only if reusable credits
layout, localization, attribution, controls, and multiple games justify it.
### Cutscenes
`ambition_cutscene` retains actor/camera/dialogue/world choreography.
- **In-session cutscene:** gameplay remains active; no shell route is required.
- **Shell-level cutscene:** opening cinematic, ending cinematic, preview tool, or
  direct process entry; a shell adapter activates `ambition_cutscene` and maps completion back to a shell
  result. Do not give generic shell segments gameplay-world powers merely to unify names. A future
  `ambition_presentation_sequence` extraction requires at least two real hosts sharing substantial neutral
  runner machinery.
## Home menu, launcher catalog, and embedded demos
The home menu is a stable shell experience backed by `ambition_menu`. Game policy supplies Continue/New
Game/profile/settings/accessibility/quit actions, visuals, and a catalog of launchable top-level
experiences. It may prefetch likely routes, but selection alone may promote work and commit.

Plugins are installed at app construction; the runtime catalog contains only experiences compiled into
that host. Each catalog entry supplies stable route ID, label/description, availability, and a route
request—not a second app or runtime plugin loader.

The Ambition desktop host registers the Ambition game plus every bundled demo (Sanic, Mary-O, and future
demos). It depends on demo **content/session crates**, never their standalone `*_app` crates. Selecting
an entry loads and activates the same experience provider used by that demo's standalone binary.

Each standalone demo app composes the same generic shell and minimal presentation but registers only its
own demo route and barebones home menu. Its `initial_route` may be the demo itself; `QuitToHome` still
reaches the demo-only launcher. Thus the same Sanic session returns to Ambition's launcher when hosted
by Ambition and to Sanic's launcher when hosted by `sanic_demo`.

The generic shell provides a plain no-art catalog/menu. Games may replace its presentation without
replacing catalog, navigation, host-home, or routing semantics.
## Load presentation and arbitrary activities
`ambition_load_presentation` attaches a foreground to a specific unresolved barrier after a configurable
hidden grace period. Fast loads produce no loading UI and no activity. There is no artificial
minimum-visible delay.
```rust
pub enum ReadyTransitionPolicy {
    AutoAdvance,
    AwaitConfirmation,
    AutoUnlessEngaged,
}
```
`AutoUnlessEngaged` is the recommended activity default:
1. reveal the foreground only after grace expires;
2. start the configured activity;
3. incidental input does not count as engagement;
4. the activity deliberately reports meaningful engagement;
5. if readiness arrives before engagement, advance automatically;
6. if engaged, enter ready-hold and show a universal Continue action;
7. let the activity continue until explicit confirmation;
8. stop activity input, capture outcome, clean up, then commit.
Activities may be movement practice, score attack, puzzles, rhythm games, lore, visual toys, or
arbitrary game-installed Bevy programs. The coordinator contains no match over game-specific activity
IDs. Every activity declares its frontend-resident assets, input context, engagement rule, cleanup
scope, platform/memory limits, ready-hold support, and optional outcome. It cannot mutate destination,
inventory, save, quest, or progression state. Version one may return score/completion/telemetry; rewards
pass through game-owned policy after commit. Same-world scoped roots/schedules are acceptable only with
isolation tests. The public protocol must permit a later isolated world/sub-app mini-session host.
## Minimal plugins and customization
The agreed crates must ship plain, correct reference implementations, not only protocols. A demo should
be able to compose approximately:
```rust
app.add_plugins((
    AmbitionLoadPlugin,
    AmbitionGameShellPlugin,
    BasicShellPresentationPlugin,
    BasicLoadPresentationPlugin,
));
```
Convenience groups such as `MinimalLoadPlugins` and `MinimalShellPlugins` may bundle these without
hiding ownership. The minimal implementation must provide:
- direct route entry;
- plain text/static-image sequence helpers;
- a simple initial menu;
- hidden grace and a basic loading screen;
- exact steps and optional estimated percentage;
- indeterminate and failure views;
- Retry/Return/Continue actions;
- no-activity behavior and the activity lifecycle;
- asset-free text fallbacks;
- keyboard/controller navigation and accessible readable defaults.
Sanic, Mary-O, and Ambition initially use this same path. Ambition may later install custom route
content, sequence segments, menu skin, load renderer, and activities. Custom plugins consume the same
semantic models and emit the same commands; they do not replace readiness, routing, or cleanup
semantics.
## Asset residency
1. **Boot:** enough to render input/error/text fallback.
2. **Shell-resident:** menu, sequence, loading UI, fonts, accessibility UI, and
   selected activities.
3. **Activation-critical:** required for the chosen route/barrier.
4. **Streamable:** arrives after activation with explicit fallback.
[`ambition_asset_manager`](../../../crates/ambition_asset_manager/) owns asset catalog/profile/residency
vocabulary and translates Bevy load state into semantic work. It does not own saves, world construction,
the shell, or the full load plan.
## Failure and recovery
A failed required step records its step/stage, safe player message, developer detail, retryability,
cleanup need, and configured fallback route. The shell may offer Retry, Return, or Exit. An activity
cannot hide failure. Retry may reuse verified immutable results but creates a new request identity.
Cancellation/supersession rejects late completion. Route failure must not strand input, camera, audio
focus, entities, or an inactive prepared session.
## Core-tools checkpoint at HEAD
Initial source exists in all three crates: load plans/barriers/evidence and commit
authorization; shell routing/holds/launcher/scopes/sequences; and load foreground,
progress/failure, activities, engagement, ready-hold, retry requests, and plain
optional UI. `MinimalLoadShellPlugins` composes them without game content.
No real contributor or game is integrated. Rust acceptance tests, real Bevy
adapters, two activity customers, isolation poison tests, and host-relative
Ambition/Sanic/Mary-O behavior remain OPEN.
## Step-by-step implementation plan
The executor must implement in this dependency order. Each step remains OPEN until every listed
acceptance test passes; do not close a broad parent with a caveat.
| Step | Status | Required result |
|---|---|---|
| L0 crate skeleton and policy guards | OPEN | three crates, legal dependency edges, MODULES/docs |
| L1 load IDs, states, contributor protocol | OPEN | headless work registration/update/removal |
| L2 barriers and exact accounting | OPEN | exact done/active/known-left + discovery state |
| L3 cancellation, supersession, failure | OPEN | stale completion cannot alter current request |
| L4 streaming/prefetch promotion | OPEN | work reuses identity/progress when promoted |
| L5 estimates and evidence snapshot | OPEN | ranges/confidence/provenance separate from facts |
| S0 shell route core and direct entry | OPEN | menu/gameplay/credits IDs route headlessly |
| S1 host initial/home routes and exit commands | OPEN | QuitToHome resolves through host policy |
| S2 scoped experience lifecycle | OPEN | activation/result/cleanup/focus are deterministic |
| S3 standard sequence runner | OPEN | text + programmatic segments share one runner |
| S4 minimal shell presentation/catalog | OPEN | no-art launcher lists registered experiences |
| P0 load foreground and hidden grace | OPEN | fast barrier shows nothing; slow barrier reveals |
| P1 progress/failure presentation | OPEN | steps, estimate, indeterminate, retry share evidence |
| P2 activity host and engagement | OPEN | two unrelated activities need no engine branch |
| P3 ready-hold and cleanup | OPEN | engaged activity continues; Continue cleans/commits |
| I0 asset/save/world contributors | OPEN | at least three contributor kinds share protocol |
| I1 reusable game/demo experience providers | OPEN | content crates expose host-independent sessions |
| I2 Ambition launcher integration | OPEN | Ambition game + every bundled demo are selectable |
| I3 standalone demo hosts | OPEN | each demo has a minimal private launcher/home route |
| I4 quit-to-home and repeated relaunch | OPEN | host-specific return, teardown, and relaunch work |
| I5 startup/menu/gameplay route | OPEN | sequence -> launcher -> prepared session end to end |
| I6 credits and cutscene adapters | OPEN | direct credits and top-level cutscene entry work |
| I7 architecture/poison policy | OPEN | app-crate deps, hard-coded home routes, leaks fail |
### L0 — skeleton and dependency proof
1. Create all three crates and add them to the workspace.
2. Declare one concern per `MODULES.md` and crate-level doc.
3. Add workspace-policy tests enforcing the dependency graph above.
4. Add empty plugins that compile in headless and minimal app compositions.
5. Do not add game-specific route/activity/media IDs.
### L1–L5 — load core
1. Land stable request, work, stage, and barrier IDs.
2. Land contributor messages/API and a deterministic in-memory test contributor.
3. Derive barrier snapshots and exact counts mechanically.
4. Add open/closed discovery and dynamic child-work tests.
5. Add cancellation/supersession/failure and poison late results.
6. Add activation requirement and independent priority.
7. Prove prefetch-to-required promotion preserves completed work.
8. Add estimate ranges/confidence/provenance without affecting readiness.
9. Add Bevy AssetServer/`bevy_asset_loader` adapters; do not reimplement either.
10. Keep save/world/content adapters outside the load core.
### S0–S4 — shell core, host policy, and minimal shell
1. Land game-owned route/experience IDs plus configurable `initial_route` and
`home_route`; prove they may differ.
2. Prove direct entry to three fake experiences without rendering.
3. Land `QuitToHome`, `Return`, restart, and process-exit as distinct semantic
commands; experiences never name the host's home route.
4. Land scoped experience ownership, commands/results, cleanup, and focus.
5. Add primary experience plus optional shell-managed foreground.
6. Land neutral sequence runner with text and custom programmatic fixtures.
7. Add skip, acknowledgement, cancel, failure, and next-route policies.
8. Add a generic launch catalog and basic text/static-image menu through
`ambition_menu`; do not create a second navigation model.
9. Add cutscene and game-owned credits adapters; keep in-session cutscenes
unchanged.
### P0–P3 — load presentation
1. Register load foreground as a shell extension; do not modify shell for game IDs.
2. Add hidden grace and prove fast loads instantiate no foreground/activity.
3. Render exact step/stage evidence before adding percentage smoothing.
4. Add estimated/indeterminate/failure policies and assert no pre-ready 100%.
5. Land generic activity registration, scope, input, engagement, and outcome.
6. Prove two unrelated fixtures run without coordinator branching.
7. Add `AutoAdvance`, `AwaitConfirmation`, and `AutoUnlessEngaged`.
8. Prove ready-hold, universal Continue, cleanup, and one activation commit.
9. Poison destination mutation and leaked activity entities/resources.
### I0–I7 — real integration and host-relative return
1. Add asset, save/profile, and world-construction contributors; classify
shell-resident, activation-critical, and streamable assets.
2. Refactor each game/demo into a host-independent experience provider in its
content/session crate. The provider owns session setup/teardown and emits semantic shell commands; it
does not own a process, menu, or return route.
3. Keep `ambition_demo_sanic_app` and `ambition_demo_smb1_app` as thin hosts:
generic minimal plugins + one demo provider + one barebones catalog/home menu.
4. Make `ambition_app` register the Ambition provider and all bundled demo
providers directly. It may depend on demo content crates, never demo app crates.
5. Populate Ambition's launcher catalog from registered providers. On desktop,
prove Ambition, Sanic, and Mary-O are visible and launchable; feature-limited hosts show only
compiled/available entries.
6. Prove host-relative return: quit each embedded session to Ambition's launcher;
quit standalone Sanic/Mary-O to their own launchers. No demo code branches on host identity or
hard-codes an Ambition route.
7. On return, revoke session input, cancel/retire session-scoped work, clean all
session-owned entities/resources/cameras/audio, retain shell-resident assets, and activate home once.
Re-launch repeatedly to expose leaks.
8. Add startup route: programmatic vanity fixture -> launcher -> prepared session;
add direct entry for gameplay, credits, and prefab cutscene.
9. Remove replaced app-local startup/loading/session authorities and add policy
checks against illegal app-crate dependencies or duplicate routing.
10. Update live planning from passing tests only; archive execution diaries.
## Required acceptance tests
1. Headless load accounting works without render/assets.
2. Exact completed/active/known-left counts survive dynamic discovery.
3. Readiness ignores presentation percentages and optional streaming work.
4. Promoted prefetch reuses work rather than duplicating it.
5. Cancelled/superseded requests cannot activate through late results.
6. Fast readiness creates no loading UI or activity.
7. Slow readiness reveals after grace with honest evidence.
8. Estimated percentage and step-only renderers consume the same snapshot.
9. No display reaches 100% before barrier readiness.
10. Shell enters gameplay, menu, credits, and cutscene routes directly.
11. Arbitrary programmatic vanity segment runs and cleans up.
12. Text/static/image-sequence helpers require no custom game renderer.
13. In-session cutscene behavior remains outside shell routing.
14. Two unrelated activities require no coordinator branch.
15. Unengaged activity auto-advances; engaged activity waits for Continue.
16. Activity cannot mutate destination and leaves no owned state after commit.
17. Initial menu remains responsive while speculative prefetch runs.
18. Sanic, Mary-O, and Ambition share minimal plugins and semantic contracts.
19. Ambition's launcher enumerates and launches the Ambition game and every
    bundled demo without depending on a demo `*_app` crate.
20. `QuitToHome` from embedded Ambition/Sanic/Mary-O returns to Ambition's home;
    the same command in standalone demos returns to their private home.
21. A direct-entry standalone demo can quit to a home menu that was not its
    initial route.
22. Repeated launch -> quit-to-home -> relaunch cycles leak no session-owned
    entities/resources/input/camera/audio or stale load completion.
23. Custom presentation can replace visuals without replacing load/shell truth.
24. Failure/retry/return restore input, camera, audio, and entity ownership.
Every new invariant requires poison evidence before it supports a DONE claim.
## Explicit non-goals for the first campaign
- polished Ambition branding or final menu art;
- video decoding as a mandatory dependency;
- a universal cutscene/sequence framework;
- a separate credits crate;
- a separate activity/mini-session crate;
- routing ordinary gameplay state through the shell;
- rewards from loading activities;
- historical telemetry before deterministic estimates work;
- forcing every room transition to show a loading experience.
## Current maintainer decisions
- Crates are `ambition_load`, `ambition_game_shell`, and
  `ambition_load_presentation`.
- Boot is a configurable shell entry route, not a separate crate.
- Vanity segments may be arbitrary programmatic Bevy experiences; static image,
  image-sequence, and text forms are helpers. Video is optional.
- Credits are initially a game-owned shell experience.
- `ambition_cutscene` retains in-session narrative/world authority; shell-level
  cutscenes use an adapter.
- Load presentation supports arbitrary minigames and a complete minimal default.
- Sanic, Mary-O, and Ambition first consume the same minimal path; polish comes
  later through replaceable game plugins.
- `initial_route` and `home_route` are separate host policy. Experiences emit
  `QuitToHome`; they never hard-code the menu that launched them.
- Ambition's home launcher registers the Ambition game and all bundled demo
  providers. Standalone demo apps register the same provider under a private, minimal demo-only launcher
  and never depend on `ambition_app`.
- Streaming, prefetching, and activation-blocking work share one load system and
  differ through barrier/priority policy.
