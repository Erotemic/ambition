# Loading, shell, and frontend integration

> **Purpose:** define the desired reusable engine architecture first, then track the shortest dependency-ordered path from the current repository to that state.
>
> **Current status:** the load/shell cores, captured session ownership, shared shell-to-session bridge, standalone Sanic/Mary-O lifecycle, and App-local catalog registries exist. Process-global music/SFX registries have been removed. Character runtime authority is only partly migrated, active-provider audio selection is not yet complete, and the canonical active gameplay-session model remains the next major architectural seam. The launcher, real provider load plans, main Ambition provider, cross-experience proof, startup sequence, and loading activity remain open.

## Desired end state

Ambition is an engine in which an independently authored game can be added by:

1. defining its authored content;
2. contributing immutable App-local catalog fragments;
3. implementing one reusable experience provider;
4. describing real preparation work;
5. constructing a fresh gameplay session after load authorization;
6. composing that provider into a standalone host or a multi-game host;
7. receiving correct loading, routing, ownership, teardown, return-to-home, and relaunch behavior from shared engine infrastructure.

The same provider implementation serves every host. A standalone host may start directly in gameplay and return to a private launcher. The Ambition host starts through its configured startup route and returns every embedded game to the Ambition launcher.

```text
Standalone Sanic process:
    initial route -> Sanic gameplay
    QuitToHome -> Sanic-only launcher
    launcher -> Sanic gameplay or Exit

Standalone Mary-O process:
    initial route -> Mary-O gameplay
    QuitToHome -> Mary-O-only launcher
    launcher -> Mary-O gameplay or Exit

Ambition host process:
    startup sequence -> Ambition launcher
    launcher -> Ambition | Sanic | Mary-O | future providers | Exit
    every embedded gameplay session -> QuitToHome -> Ambition launcher
```

The architecture is complete only when adding a fourth provider does not require a new host match arm, a new lifecycle path, or knowledge of shell internals.

## Architectural authorities

Five authorities define the design.

```text
Host
    Owns process entry, linked providers, initial route, home route, launcher,
    startup sequence, presentation policy, platform integration, and exit.

Provider
    Owns one experience's identity, metadata, authored fragments, preparation,
    prepared-session construction, activation, gameplay construction, and teardown.

Session
    Owns everything created for one gameplay activation, including current world
    authority and all activation-local simulation and presentation state.

App
    Owns the immutable authored catalogs assembled from its linked providers.

Load
    Owns readiness facts, progress evidence, failure/retry state, cancellation,
    supersession, and one-shot activation authorization.
```

No authority should be duplicated. In particular:

- the host does not know provider-specific gameplay construction;
- the provider does not know which host linked it or where that host's home route is;
- the launcher does not retain gameplay-world authority;
- catalogs are not process-global runtime state;
- presentation never manufactures readiness;
- route changes do not redefine ownership of already-requested deferred spawns.

## Canonical lifecycle

```text
host requests a provider route
-> provider creates a preparation plan
-> load contributors perform real work and report evidence
-> the required activation barrier becomes ready
-> shell consumes one-shot activation authorization
-> provider publishes a prepared gameplay session
-> provider constructs a fresh live gameplay session
-> the session owns all activation-local state
-> gameplay emits semantic QuitToHome
-> exact session authority is revoked and retired
-> the host's configured home route resumes
```

Cancellation, supersession, retry, and relaunch always create fresh transaction or activation identity. Stale load completions, deferred commands, retirement requests, or route messages cannot activate or damage a newer session.

At a frontend route there is no active gameplay session. That absence is normal and explicit: gameplay schedules sleep, no placeholder room is authoritative, and only frontend input/camera/UI authority remains.

## Provider authoring experience

The common path for a new game should look like normal Bevy composition:

```rust
app.add_plugins(MyGameExperiencePlugin);
```

The provider should contribute concepts equivalent to:

```text
ExperienceRegistration
AuthoredCatalogFragments
LoadPlan
PreparedGameplaySession
GameplaySessionBuilder
SessionTeardown
```

A declarative surface may resemble:

```rust
GameExperiencePlugin::new(MY_GAME)
    .with_route(MY_GAMEPLAY_ROUTE)
    .with_catalogs(MyGameCatalogFragments::default())
    .with_load_plan(build_my_game_load_plan)
    .with_session_builder(build_my_game_session);
```

The exact API should follow repository and Bevy conventions, but these properties are required:

- provider registration is deterministic and compositional;
- provider-local identities and presets are intuitive;
- simple providers receive complete defaults;
- advanced providers can extend preparation and activation without replacing lifecycle truth;
- providers depend on reusable engine crates, never on host app crates;
- standalone hosts reuse the provider unchanged;
- headless tests can exercise the provider without rendering dependencies;
- incorrect ownership or missing authority is difficult to express silently.

Excessive boilerplate is an architectural defect. The design should make the correct path shorter than bespoke wiring.

## Session and ownership model

Every gameplay activation receives fresh typed identities equivalent to:

```text
ShellActivationId
GameplaySessionId
SessionScopeId
LoadTransactionId
```

One canonical App-local active gameplay-session representation owns or references:

- provider identity;
- shell activation identity;
- session scope;
- prepared world data;
- `RoomGeometry`;
- `RoomSet`;
- `ActiveRoomMetadata`;
- starting character;
- selected character/audio provider authority;
- load transaction identity;
- provider-local session resources.

`SessionSpawnScope` captures ownership when spawn work is requested. Nested helpers, deferred commands, authored features, and dynamic effects inherit that captured scope rather than consulting whichever route happens to be active later.

Session ownership covers at least:

- players, NPCs, enemies, bosses, and encounters;
- portals, room features, moving platforms, hazards, pickups, and rewards;
- abilities, projectiles, attack volumes, debris, and transient effects;
- gameplay sprites, parallax, overlays, HUD, dialog, map, and cutscene UI;
- gameplay cameras and input contexts;
- music, ambience, looped SFX, and other playback ownership;
- load activity state and provider-local session resources.

At the host home route there must be:

```text
zero session entities
zero gameplay cameras
zero gameplay input owners
zero gameplay UI roots
zero gameplay audio owners
zero active gameplay session
zero session load transactions
exactly one frontend authority
```

## App-local authored catalogs

Linked providers contribute immutable authored fragments to each Bevy `App`. Assembly must be:

- deterministic;
- independent of provider registration order;
- transactional;
- isolated between separate Apps in one process;
- explicit about ownership of duplicate identities;
- stable and actionable in diagnostics;
- immutable after successful assembly where practical.

Runtime consumers use:

- `Res<CharacterCatalog>` in ECS systems;
- `&CharacterCatalog` in pure helpers;
- explicit character IDs at the authored-to-runtime boundary;
- focused system parameters when several catalog resources travel together;
- App-local audio registries plus explicit active-provider/session selection.

Display names are presentation, not identity. Missing required catalogs in production composition should not silently degrade to empty data. Minimal test worlds may insert explicit empty fixtures.

Shared asset caches may outlive a gameplay session, but playback authority does not. Activating a provider selects its audio authority; returning home retires gameplay playback ownership; switching providers cannot observe stale defaults or active loops from the prior session.

## Load and presentation model

`ambition_load` owns preparation truth. A plan groups work for one route request; a barrier authorizes activation.

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

Provider preparation reports semantic work such as:

- catalog assembly and validation;
- required asset requests;
- sprite/character readiness;
- save or session decoding;
- room/world-data preparation;
- immutable prepared-session construction;
- provider audio readiness.

Exact facts distinguish completed, active, known remaining, discovery openness, failure, retryability, and barrier readiness. Estimates are optional and never authoritative. Fast required work remains inside hidden grace. Slow work reveals minimal honest presentation. Streamable work may continue after activation. Speculative work may be promoted without restarting.

A loading activity is an isolated shell experience with scoped input/state, explicit engagement, optional result, ready-hold, universal Continue, exact cleanup, and no ability to mutate destination state. Engaged activities may continue after readiness until the player confirms.

## Host and frontend model

The shell owns top-level routing and host-relative return semantics.

```text
Ambition host:
    initial = ambition_startup
    home = ambition_launcher

Standalone Sanic host:
    initial = sanic_gameplay
    home = sanic_launcher

Standalone Mary-O host:
    initial = mary_o_gameplay
    home = mary_o_launcher
```

The Ambition launcher derives entries from linked provider registrations and contains Ambition, Sanic, Mary-O, and Exit. Providers remain unaware of the host that linked them.

The minimal Ambition startup route is:

```text
Powered by Ambition
-> Ambition title
-> Ambition launcher
```

Startup segments may be arbitrary registered Bevy programs as well as reusable text, static-image, image-sequence, and optional video adapters. Direct route entry remains available for tests and development. Credits and top-level cutscene previews are shell experiences; ordinary in-session cutscenes remain gameplay-session concerns.

## Compile-time architecture

Compile performance is an engine feature. Preserve these dependency rules:

- small headless core crates remain independent of rendering;
- generic session/catalog/load types contain no game-specific enums;
- reusable engine crates never depend on game content or host apps;
- provider crates never depend on standalone app crates;
- hosts depend on providers, not the reverse;
- `ambition_app` composes provider crates rather than demo app crates;
- presentation remains feature-gated where practical;
- one provider can be built and tested without compiling unrelated games;
- public types live at the lowest correct layer;
- adding a provider does not broaden low-level dependency fanout.

Workspace policy tests should protect critical dependency direction and retired global-authority seams.

## Completion criteria

The campaign is complete when all of these are true:

1. Ambition, Sanic, and Mary-O use one provider/session/load/shell/catalog lifecycle.
2. Standalone and embedded hosts use identical provider implementations.
3. Every activation creates fresh session and load identities.
4. Returning home leaves no gameplay authority or activation-local ownership behind.
5. Cross-provider transitions cannot observe stale world, catalog, input, camera, UI, audio, or load state.
6. Catalog assembly is deterministic, transactional, App-local, and order independent.
7. Runtime code has no hidden process-global character or audio authority.
8. Real provider preparation controls activation through one-shot authorization.
9. Minimal launcher/loading/startup presentation is complete without owning lifecycle truth.
10. Adding a fourth provider requires provider composition, not engine or host-specific lifecycle edits.
11. Narrow headless tests and compile paths remain available.
12. The architecture is simpler to explain after implementation than before it.

## Current implementation state

### Established foundation

The repository currently contains:

- headless `ambition_load`, `ambition_game_shell`, and `ambition_load_presentation` cores;
- provider-derived launcher registration and host-relative `QuitToHome` behavior;
- engine-neutral session scope and request-time captured spawn ownership;
- a shared shell-to-gameplay-session bridge;
- broad session ownership across existing simulation and presentation paths;
- Sanic and Mary-O provider/standalone lifecycle customers;
- deterministic App-local character and audio fragment registries;
- real Ambition/Sanic/Mary-O fragment composition;
- removal of the old process-global music/SFX registry APIs;
- a source-policy ratchet against reintroducing those audio globals.

### Important limitations in the current slice

The current source must not be described as the completed desired authority model:

- character migration is partial;
- the combat-bark slice currently permits an absent `CharacterCatalog` and falls back to an empty catalog;
- bark lookup reverses a display name instead of carrying a stable character ID;
- fragment structs still expose mutable validated state and need stronger registration-boundary validation;
- audio fragments are App-local, but active provider/session audio selection and cross-experience replacement are not implemented;
- the combined audio surface does not yet prove deterministic provider-relative SFX authority;
- the launcher can still have gameplay-world authority through historically global resources;
- real provider preparation plans do not yet govern activation;
- the main Ambition game is not yet a provider;
- no full cross-experience lifecycle test proves exact cleanup and replacement.

## Evidence-backed ledger

`DONE` means a passing test or machine-derived invariant supports the exact claim. `OPEN` means implementation is absent or incomplete. `BLOCKED` means the dependency that must land first is named.

| ID | Status | Required result |
|---|---|---|
| C0 | DONE | Deterministic App-local character/audio fragment registries; real Ambition/Sanic/Mary-O coexistence; registration-order and separate-App isolation coverage; stable duplicate ownership diagnostics; candidate-before-commit App updates. |
| C0H | OPEN | Harden fragment APIs: private validated state, fallible parsing, registration-boundary revalidation, and no panic-based reliance on externally mutable invariants. |
| C1-char | OPEN (partial) | Every meaningful character consumer uses explicit App-local authority. Replace display-name reverse lookup with stable character IDs; require the resource in production systems; migrate sprite/spawn/brain/dialogue/asset-manifest consumers; add direct authority and poison tests. |
| C2-char | BLOCKED on C1-char | Delete `install_character_catalog`, `catalog()`, `catalog_ron()`, `default_character_id`, and `character_roster_plugin`; migrate fixtures; add a source-policy ratchet. |
| C1-audio-registry | DONE | Process-global music/SFX registry APIs are removed; App-local provider fragments are registered and read explicitly by current bootstrap paths. |
| C1-audio-session | OPEN | Active gameplay session selects provider-relative music/SFX authority; activation replaces prior authority; home retirement clears playback ownership; Sanic -> Mary-O switching is proven. |
| C2-audio | DONE | `install_music_registry`, `install_sfx_registry`, `authored_music_registry`, `authored_sfx_registry`, `MUSIC_REGISTRY_OVERRIDE`, and `SFX_REGISTRY_OVERRIDE` are deleted and guarded by policy. |
| W0 | OPEN | One canonical App-local active gameplay-session representation owns current world/provider/session/load authority. |
| W1 | BLOCKED on W0 | Frontend routes safely have no gameplay session; gameplay schedules sleep; placeholder worlds disappear. |
| W2 | BLOCKED on W0 | Camera, HUD, dialog, map, cutscene UI, input, and audio receive explicit host/session ownership. |
| L0 | BLOCKED on C2-char/W0 | Sanic and Mary-O contribute real preparation through `ambition_load` and produce immutable prepared sessions. |
| L1 | BLOCKED on L0 | Retry, cancellation, supersession, streaming, promotion, and relaunch use fresh transaction authority. |
| P0 | OPEN | Provider authoring surface is compact, documented by an example/test provider, and supplies reusable standalone/load/session defaults. |
| A0 | BLOCKED on C2-char/W0/P0 | Main Ambition game becomes a provider using the shared lifecycle. |
| A1 | BLOCKED on A0 | Ambition host derives Ambition + Sanic + Mary-O + Exit from registrations. |
| X0 | BLOCKED on A1/L1/W2 | Headless cross-experience cycle proves exact replacement and no stale authority. |
| X1 | BLOCKED on X0 | No-window rendered cycle proves camera/UI/input/audio ownership. |
| B0 | OPEN | Startup sequence hands off to launcher while direct route entry remains available. |
| B1 | OPEN | One deterministic loading activity proves engagement, ready-hold, Continue, cleanup, and destination isolation. |
| F0 | LATER | Game-owned credits route and top-level cutscene adapter. |

## Dependency-ordered roadmap

### Phase 0 — Harden the catalog foundation

1. Make character/audio fragments immutable after validated construction.
2. Make parsing and registration failures structured and non-panicking.
3. Revalidate at the transactional registration boundary.
4. Replace bark display-name reversal with explicit runtime character identity.
5. Require catalog authority in production systems; use explicit empty fixtures only in minimal tests.
6. Add direct tests proving different Apps/catalogs produce different runtime behavior.
7. Correct stale comments and evidence claims before expanding the architecture.

### Phase 1 — Finish explicit character and audio authority

1. Thread `&CharacterCatalog` or `Res<CharacterCatalog>` through NPC/enemy construction, actor seeds, brains, actions, movement/body selection, sprite manifests/tuning/collision/render dimensions, attack volumes, dialogue validation, wear/re-wear, and content fixtures.
2. Remove every remaining process-global character lookup and add the character policy ratchet.
3. Add explicit active-provider audio selection to prepared/live gameplay-session state.
4. Build deterministic provider-relative music and SFX indexing as required by runtime consumers.
5. Prove Ambition, Sanic, and Mary-O defaults; two-App isolation; home cleanup; and Sanic-to-Mary-O replacement.

### Phase 2 — Canonical active gameplay session

1. Introduce one App-local active gameplay-session representation containing scope, provider/activation identity, world state, selected catalog/audio authority, and load transaction identity.
2. Publish prepared world data atomically during activation.
3. Revoke gameplay authority before frontend execution during retirement.
4. Gate gameplay schedules on active-session presence.
5. Remove build-time placeholder worlds and historically global current-world pointers.
6. Prove fresh relaunch and that retirement of activation A cannot damage activation B.

### Phase 3 — Complete exact ownership

1. Audit every gameplay spawn against captured session scope.
2. Add a content-heavy fixture covering authored, nested, deferred, and dynamic spawns.
3. Assign gameplay cameras, input, HUD, dialog, map, cutscene UI, music, ambience, loops, and provider-local resources to exact owners.
4. Prove the home-route zero-state contract while frontend authority survives.

### Phase 4 — Add real provider load plans

1. Create a fresh load transaction for every route request and relaunch.
2. Report catalog validation, required assets, sprite readiness, room/session decoding, immutable prepared-session construction, and audio readiness.
3. Classify activation-critical, streamable, and speculative work.
4. Consume one-shot authorization before live gameplay construction.
5. Prove hidden grace, slow honest presentation, optional estimates, retry/return-home, cancellation, streaming, promotion, and fresh relaunch authorization.

### Phase 5 — Make providers exceptionally easy to author

1. Consolidate registration, catalog contribution, load planning, prepared-session construction, activation, and teardown behind one obvious provider plugin.
2. Supply reusable defaults for standalone hosts, direct gameplay entry, demo launchers, host-relative home, minimal loading presentation, and headless lifecycle tests.
3. Add a small example/test provider demonstrating the intended authoring experience.
4. Measure dependency fanout and preserve narrow provider/headless compile paths.

### Phase 6 — Convert Ambition and build the host

1. Convert the main Ambition game to the shared provider contract.
2. Move main-game session state, ownership, catalogs, load plan, activation, teardown, and `QuitToHome` behind that provider.
3. Configure `ambition_startup` and `ambition_launcher` in the host.
4. Link reusable Ambition, Sanic, and Mary-O providers.
5. Derive launcher entries from provider registrations.
6. Preserve direct development entry through host configuration.

### Phase 7 — Cross-experience acceptance

Exercise:

```text
Ambition launcher
-> Sanic
-> Ambition launcher
-> Mary-O
-> Ambition launcher
-> Ambition
-> Ambition launcher
-> Sanic
-> Ambition launcher
```

At every boundary assert:

- one active shell experience;
- zero or one gameplay session as appropriate;
- fresh activation/session identities;
- correct provider and world authority;
- correct character/audio selection;
- correct input, camera, and UI ownership;
- zero previous-provider entities or session resources;
- zero stale load transactions;
- exactly one launcher authority at home.

Provide both a headless lifecycle test and a no-window rendered ownership test. Poison stale catalog selection, stale room authority, skipped scope retirement, stale camera/input/audio, reused load authorization, hard-coded return routes, and registration-order dependence.

### Phase 8 — Startup sequence and loading activity

1. Configure the minimal startup route without using vanity timing to conceal load.
2. Preserve direct route entry for tests and development.
3. Add one deterministic activity proving scoped input/state, engagement, ready-hold, universal Continue, optional result, exact cleanup, and destination isolation.
4. Leave credits and top-level cutscene adapters as later shell-experience extensions of the same architecture.

## Required acceptance tests

| Area | Proofs |
|---|---|
| Catalog | Provider-only fragments; all three coexist; registration-order independence; deterministic duplicate diagnostics; failed registration preserves prior valid state; separate Apps; immutable validated fragments; simulation/presentation agree on one authority; no hidden process globals. |
| Session | Request-time captured ownership; nested/deferred inheritance; ambient-route immunity; immediate retirement revocation; A retirement preserves B; frontend-safe absence; fresh relaunch; zero session authority at home. |
| Audio | Ambition/Sanic/Mary-O provider selection; combined deterministic indexing; clean activation replacement; home playback retirement; separate-App isolation; no central host match. |
| Load | Hidden fast load; slow honest evidence; optional estimates; streaming activation; promotion reuse; cancelled/superseded inert; retry/relaunch freshness; one-shot commit. |
| Hosts | Registration-derived launcher; direct standalone entry/private home; embedded host-relative return; identical provider implementation; repeated and cross-provider leak-free cycles; startup handoff. |
| Activity | Unengaged auto-advance; engaged ready-hold; Continue cleanup/commit; scoped input/state; destination isolation. |
| Compile | Headless cores avoid render dependencies; providers avoid app crates; engine avoids game content; one provider tests without unrelated games; dependency policies remain green. |

Every invariant supporting `DONE` receives direct or poison evidence.

## Validation commands

```bash
cargo test -p ambition_audio
cargo test -p ambition_characters
cargo test -p ambition_platformer_primitives
cargo test -p ambition_game_shell
cargo test -p ambition_load
cargo test -p ambition_load_presentation
cargo test -p ambition_actors
cargo test -p ambition_render
cargo test -p ambition_demo_sanic
cargo test -p ambition_demo_sanic_app
cargo test -p ambition_demo_sanic_app --features visible
cargo test -p ambition_demo_smb1
cargo test -p ambition_demo_smb1_app
cargo test -p ambition_demo_smb1_app --features visible
cargo test -p ambition_app
cargo test -p ambition_workspace_policy
cargo fmt --all -- --check
python3 scripts/modules_md.py
python3 scripts/generate_agent_index.py
python3 scripts/check_agent_kb.py
python3 scripts/check_doc_links.py
```

Run visible/no-window tests with their required features. Report only commands actually executed, their exact result, and any source-inspection-only claims separately.

## Completion reporting

Track executable slices as `DONE`, `OPEN`, or `BLOCKED`. A `DONE` row cites a passing test or machine-derived invariant supporting the exact wording. Do not call App-local storage complete runtime authority when active provider/session selection remains absent. Keep implementation history in commits or archives; keep this live document focused on the desired architecture, present constraints, dependency order, and remaining work.
