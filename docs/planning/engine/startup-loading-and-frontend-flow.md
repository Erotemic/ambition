# Loading, shell, and frontend integration

> **Status:** the incoming baseline has tested load/shell cores and headless Sanic/Mary-O provider lifecycles. The current worktree adds captured spawn-time session ownership, a shared shell-to-session bridge, broad simulation/presentation scoping, and shared visible/headless demo composition. These additions remain **OPEN pending Rust compilation and runtime tests**. The next architectural chain is: canonical active-session world -> App-local catalogs -> real provider load plans -> Ambition provider/launcher -> cross-experience proof.

## Target experience

```text
process entry -> configured shell route -> optional startup sequence -> host launcher
              -> provider load plan -> activation authorization -> gameplay session
              -> QuitToHome -> exact session retirement -> host launcher
```

Ambition's launcher exposes Ambition, Sanic, Mary-O, and Exit. Standalone Sanic and Mary-O use the same providers under private minimal hosts: gameplay may be the initial route while the demo-only launcher is the home route. Completion means all three games share one provider, session, load, shell, catalog, minimal-presentation, teardown, and relaunch architecture.

## Maintainer intent

1. Fast preparation shows no loading screen; unavoidable waits show honest facts and optional estimates.
2. Activation-critical, streamable, and speculative work share one load model and stable work identity.
3. Arbitrary loading activities are isolated first-class experiences; engaged activities may continue after readiness until universal Continue.
4. Minimal launcher/loading presentation is complete for demos and early Ambition; polish replaces visuals rather than authority.
5. Boot is route configuration. Vanity segments may be arbitrary Bevy programs, with text/static/image-sequence helpers and optional video adapters.
6. Credits and top-level cutscenes are shell experiences; ordinary in-session cutscenes remain under `ambition_cutscene`.

## Binding crate carve

```text
crates/ambition_load
crates/ambition_game_shell
crates/ambition_load_presentation
```

### `ambition_load`

Owns headless preparation truth: stable plan/request/work/barrier IDs; exact work state; discovery accounting; activation-critical, streamable, and speculative roles; priority; promotion without restart; cancellation/supersession; failure/retry facts; barrier readiness; and one-shot activation authorization. Asset, save, world, and content systems perform work and report through this protocol. The crate remains renderer-, menu-, and game-content-free.

### `ambition_game_shell`

Owns renderer-independent top-level lifecycle: `initial_route`; `home_route`; provider and route registration; launcher projection; activation/replacement/completion/failure/return/exit; semantic `QuitToHome`; top-level focus transfer; shell/gameplay activation identity; neutral presentation sequences; and minimal launcher behavior through `ambition_menu`. Gameplay owns rooms, combat, inventory, dialogue, pause, bosses, and in-session cutscenes. The shell bridge maps shell activation to an engine-neutral gameplay-session scope.

### `ambition_load_presentation`

Owns hidden grace, exact stage/step presentation, optional estimated percentage, indeterminate/failure/retry/return views, arbitrary activity registration, activity input/engagement/result, ready-hold, Continue, cleanup, and the basic no-art implementation. It consumes load and shell facts and never manufactures readiness.

## Constitutional dependency shape

```text
AssetServer / save / world / content contributors
                     |
                     v
               ambition_load
          work facts, barriers, commit
              /               \
             v                 v
 ambition_game_shell       headless clients
             ^
             |
 ambition_load_presentation
             ^
             |
 providers, activities, styling, app hosts

ambition_platformer_primitives::lifecycle::session
             ^
             |
 ambition_game_shell session bridge
             ^
             |
      gameplay providers
```

Rules: providers self-register and launcher entries derive from the registry; hosts select providers/routes/presentation; standalone apps depend on provider crates rather than `ambition_app`; `ambition_app` links provider crates rather than demo apps; session identity is shell-free and captured when spawn work is requested; load evidence alone controls readiness; one active gameplay session owns current gameplay-world authority; authored catalogs are App-local and composable.

## Provider, host, and session contracts

A provider owns registration, immutable catalog fragments, load contribution, preparation, activation-specific construction, session scope, teardown, and semantic shell commands. A host owns linked providers, initial/home routes, startup sequence, platform/render/audio selection, launcher projection, and process exit.

```text
Ambition:        initial = ambition_startup   home = ambition_launcher
Standalone Sanic: initial = sanic_gameplay    home = sanic_launcher
Standalone Mary-O: initial = mary_o_gameplay  home = mary_o_launcher
```

Every gameplay activation receives a fresh `SessionScopeId`. `SessionSpawnScope` captures ownership at spawn-request time, so nested/deferred work cannot be reassigned by a later route change. Session ownership covers actors, authored features, enemies/bosses/hazards/pickups/rewards, abilities/projectiles/debris, room visuals/parallax, overlays/health/effects/gameplay UI, and eventually gameplay camera/audio/input.

The shell bridge owns `ShellActivationId <-> SessionScopeId`. Retirement removes active-session authority, revokes ambient spawn authority immediately, emits provider and exact-scope retirement facts, cleans only that scope, and preserves a newer scope during same-frame replacement.

The canonical active gameplay session must own `RoomGeometry`, `RoomSet`, `ActiveRoomMetadata`, `StartingCharacter`, and related scene/runtime state. At launchers, credits, or other non-gameplay routes there is no active gameplay session and gameplay schedules sleep safely.

## Load, evidence, and activities

A plan groups preparation for one route request; a barrier decides activation. Background streaming may continue afterward.

```rust
pub enum ActivationRequirement { RequiredFor(LoadBarrierId), Degradable, Speculative }
pub enum LoadPriority { Immediate, High, Normal, Low }
```

Required examples: save header, collision/world data, player definition, required sprites/entities. Degradable examples: distant art, ambience, high-resolution variants. Speculative examples: likely next route/room. Promotion preserves work identity and progress. Cancelled/superseded transactions cannot authorize activation; commit is one-shot for the current request.

Player-facing work steps are semantic units such as catalog assembly, required asset request, room/save decode, content validation, staged world preparation, or required pipeline warmup. Snapshots separate exact completed/active/known-remaining work, discovery openness, stage labels, optional undiscovered-work forecast, optional effort/confidence/provenance, failures/retryability, and exact readiness. Presentation may show stages, counts, estimate, both, or indeterminate; 100% appears only after readiness.

A waiting foreground attaches to one unresolved barrier after hidden grace. An activity has stable identity, scoped state/input, explicit engagement, optional result, and no destination authority. Policies are `AutoAdvance`, `AwaitConfirmation`, and `AutoUnlessEngaged`; engaged ready-hold continues until Continue cleans the activity and commits exactly once.

## Startup, vanity, credits, and cutscenes

A startup route is an ordinary shell sequence of text, static/image-sequence media, optional video adapters, arbitrary registered Bevy segments, notices, acknowledgements, and route transitions. Minimal Ambition flow: `Powered by Ambition -> Ambition title -> ambition_launcher`. Credits are initially game-owned shell experiences. Top-level cutscene previews adapt `ambition_cutscene`; in-session cutscenes remain inside gameplay.

## Current implementation state

### Verified incoming baseline

Passing tests established the core load/shell/presentation contracts, provider-derived launcher registration, host-relative `QuitToHome`, exact synthetic scope cleanup, headless Sanic and Mary-O launch/return/relaunch, the initial shell-free session primitive, and dependency policy.

### Current worktree awaiting verification

Implemented but still `OPEN` until compiled and exercised: request-time captured `SessionSpawnScope`; immediate spawn-authority revocation plus exact retirement; shared `GameplaySessionBridgePlugin`; Sanic/Mary-O provider migration to that bridge; shared visible/headless provider composition; route-aware room presentation; broad session ownership across actor/feature/encounter/projectile/platform/world/transient-render spawns; and visible lifecycle tests for both demos.

## Remaining-work ledger

| ID | Status | Required result |
|---|---|---|
| V0 | OPEN | Compile modified dependency chain; run narrow, policy, headless, and visible tests |
| V1 | OPEN | Poison captured-scope, immediate-revocation, bridge, and visible-cleanup tests |
| W0 | OPEN | Canonical App-local active session owns current world state |
| W1 | OPEN | Gameplay sleeps with no session; build-time placeholder worlds disappear |
| W2 | OPEN | Camera, HUD, dialog, map, cutscene UI, input, and audio gain explicit ownership |
| C0 | OPEN | Character/music/SFX authority becomes App-local provider fragments plus deterministic catalogs |
| C1 | OPEN | Three-provider coexistence, order independence, duplicate diagnostics, multi-App isolation |
| L0 | OPEN | Sanic/Mary-O report real preparation through `ambition_load` |
| L1 | OPEN | Relaunch/retry/cancel/stream/promotion use correct fresh transaction authority |
| A0 | OPEN after W0/C0 | Main Ambition game becomes a provider on the shared lifecycle |
| A1 | OPEN after A0 | Ambition launcher derives Ambition + Sanic + Mary-O + Exit |
| X0 | OPEN after A1 | Headless cross-experience cycle is leak-free |
| X1 | OPEN after X0 | No-window rendered cycle proves presentation/camera/UI ownership |
| B0 | OPEN | Startup sequence hands off to launcher; direct route entry remains available |
| B1 | OPEN | Arbitrary loading activity proves engagement, ready-hold, Continue, cleanup |
| F0 | LATER | Game-owned credits route and top-level cutscene adapter |

## Ordered implementation plan

### Step 0 — Verify this worktree

1. Run formatter, metadata, and generated-doc checks.
2. Compile from platformer primitives through actors/render/shell/providers/apps.
3. Run session, shell, headless lifecycle, and visible lifecycle tests.
4. Fix feature gates, signatures, and schedule ordering from compiler/runtime evidence.
5. Poison the captured-scope and immediate-retirement invariants; then update statuses.

### Step 1 — Canonical active-session world

1. Introduce one App-local active gameplay-session representation containing scope, provider/activation identity, room/world state, and staged session data.
2. Move current-world pointers into it or explicit handles; gate gameplay schedules on its presence.
3. Publish a prepared world atomically on activation and clear authority before frontend execution on retirement.
4. Remove standalone build-time placeholder worlds.
5. Prove safe launcher frames, fresh relaunch, and exact Sanic-to-Mary-O world replacement.

### Step 2 — Complete runtime ownership

1. Audit every gameplay spawn against captured scope and add a content-heavy fixture covering authored, nested, deferred, and dynamic spawns.
2. Assign camera, gameplay input, HUD, map, dialog, cutscene UI, music, ambience, and looped SFX to host/session/experience owners.
3. Prove each owner retires once while frontend camera/menu/input survive at home.

### Step 3 — App-local catalogs

1. Define character/music/SFX fragment types and generic provider registration.
2. Assemble deterministically by provider/item ID; validate references and emit stable duplicate diagnostics.
3. Publish App-local resources and migrate systems/pure helpers to explicit resources/parameters.
4. Remove process-global lookup from integrated runtime authority.
5. Prove standalone-only, Ambition-only, three-provider, registration-order, duplicate, and multiple-App cases.

### Step 4 — Real provider load plans

1. Create a fresh transaction per route request/relaunch.
2. Report catalog validation, required assets, room/session preparation, staged immutable data, and classified streamable/speculative work.
3. Consume one-shot authorization before activation.
4. Prove hidden grace, slow reveal, exact facts, optional estimate, failure/Retry/Return Home, cancellation, streaming, promotion, and fresh relaunch authorization.

### Step 5 — Ambition provider and launcher

1. Extract main-game lifecycle from `ambition_app` startup authority and register it through the shared gameplay provider contract.
2. Use shared session scope, active world, catalogs, load plan, ownership, and `QuitToHome`.
3. Configure `ambition_startup`/`ambition_launcher`, link reusable Sanic/Mary-O providers, and derive entries from registrations.
4. Prove each entry launches, returns, and relaunches.

### Step 6 — Cross-experience proof

Exercise `launcher -> Sanic -> launcher -> Mary-O -> launcher -> Ambition -> launcher -> Sanic -> launcher`. At every boundary assert one shell experience, zero/one gameplay session as appropriate, fresh IDs, correct world/catalog/input/camera/UI/audio, no stale load transaction, and no previous-provider state. Repeat in no-window rendered composition.

### Step 7 — Startup and activity

1. Configure minimal programmatic/text startup sequence without using vanity timing as load concealment.
2. Preserve direct routes for tests/development.
3. Add one deterministic arbitrary activity and prove unengaged auto-advance, engaged ready-hold, Continue, exact cleanup, and one activation commit.

### Step 8 — Credits and cutscene adapters

Register game-owned credits with postgame/home routing and a top-level cutscene adapter for previews/openings/endings; preserve ordinary in-session cutscenes.

## Required acceptance tests

| Area | Proofs |
|---|---|
| Session | request-time ownership; nested/deferred inheritance; ambient-change immunity; immediate retirement revocation; A retirement preserves B; complete representative ownership; zero session state at home; visible/headless same lifecycle |
| Catalog | provider-only catalogs; all three coexist; order independence; deterministic duplicates; multiple Apps; simulation/presentation same authority |
| Load | hidden fast load; slow honest evidence; estimates non-authoritative; streaming activation; promotion reuse; cancelled/superseded inert; fresh retry/relaunch; one-shot commit |
| Hosts | Ambition catalog exactly once each; direct standalone entry/private home; embedded host-relative return; repeated/cross-provider leak-free cycles; startup handoff; direct gameplay/credits/cutscene entry |
| Activity | two unrelated registrations; unengaged auto-advance; engaged ready-hold; Continue cleanup/commit; destination isolation |

Every invariant supporting `DONE` receives poison evidence.

## Validation commands

```bash
cargo test -p ambition_platformer_primitives
cargo test -p ambition_game_shell
cargo test -p ambition_load
cargo test -p ambition_load_presentation
cargo test -p ambition_actors
cargo test -p ambition_render
cargo test -p ambition_demo_sanic
cargo test -p ambition_demo_sanic_app
cargo test -p ambition_demo_smb1
cargo test -p ambition_demo_smb1_app
cargo test -p ambition_workspace_policy
cargo fmt --all -- --check
python3 scripts/modules_md.py
python3 scripts/generate_agent_index.py
python3 scripts/check_agent_kb.py
python3 scripts/check_doc_links.py
```

Run visible/no-window tests with their required features and add main Ambition packages once its provider lands. Report only executed commands.

## Completion reporting

Track executable slices as `DONE`, `OPEN`, or `BLOCKED`. `DONE` cites a passing test or machine-derived invariant; source inspection remains labeled. This live plan records current architecture and remaining work. Commit history or an archive holds detailed execution history.
