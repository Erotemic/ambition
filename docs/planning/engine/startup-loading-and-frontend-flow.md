# Loading, shell, and frontend integration

> **Purpose:** define the desired reusable engine architecture first, then track the shortest dependency-ordered path from the current repository to that state.
>
> **Current status (2026-07-13):** the multi-game host EXISTS and is proven headlessly. `./run_game.sh` composes the shell-routed host by default: the Ambition launcher (title screen) derives Ambition + Sanic + Mary-O + built-in Exit from provider registrations; each activates a fresh session-scoped gameplay session; `QuitToHome` (F10 in-session) retires the exact session; Exit leaves the process; `--direct` / `--start-room` preserve direct development entry. The App-local character/hostile/boss authority migration is verified `DONE` (compile + full focused suites + policy ratchet). The gameplay simulation carries ONE session gate (`GameplaySimulationRoot` + `simulation_authorized`): session-routed hosts freeze the whole sim — tick timeline included — at frontend routes. Active session audio authority (`ActiveAudioSelection`) is selected per activation by the shell bridge and retired at home. The X0 headless cross-experience cycle (launcher → Sanic → Mary-O → Ambition → Sanic → Exit) passes with zero-state and fresh-identity contracts at every boundary. Remaining: real provider load plans (routes currently activate without barriers), provider-authoring dedup (P0), the rendered/no-window ownership proof (X1), the startup/vanity sequence (B0), and the loading activity (B1).

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
- deterministic App-local playable-character, hostile-archetype, boss, and audio fragment registries;
- real Ambition/Sanic/Mary-O character/audio composition plus provider-owned hostile and boss content;
- removal of the old process-global music/SFX registry APIs;
- a source-policy ratchet against reintroducing those audio globals;
- a character-authority completion candidate with private validated fragments, fallible parsing, and transactional registration-boundary revalidation;
- explicit App-local `CharacterCatalog` flow through player wear/re-wear, actor and NPC construction, sprite and collision resolution, asset manifests, room lowering, reset/hot reload, snapshot reconstruction, encounter and summon spawns, interaction, dialogue, barks, and authored attack geometry;
- a deterministic App-local hostile `CharacterRoster` assembled from provider fragments, including provider-relative defaults that can coexist without selecting one process-wide winner;
- one App-local `BossCatalog` assembling provider-owned behavior profiles, encounter specifications, sheet geometry, sprite filenames, special-animation vocabulary, and provider-relative defaults;
- App-local `AuthoredAttackVolumeResolver`, hostile roster, and boss catalog resources threaded through simulation, presentation, snapshots, encounters, projectiles, and content validation, with separate-App isolation coverage;
- required catalog resources at production ECS boundaries: playable-character authority no longer degrades through optional resources or implicit empty-catalog constructors; reusable boss-free/hostile-free Apps carry explicit content-free resources, while provider activation validation remains part of W0;
- deterministic validation of reverse display-name authoring joins, followed by stable character IDs carried in runtime components;
- deletion of production character/hostile/boss install caches, demo installers, global attack-volume and boss lookup seams, engine-owned provider asset enumeration, and implicit sprite lookup wrappers;
- one complete source-policy ratchet protecting the retired process-global character, hostile-archetype, and boss authority across production crates and games.

### Important limitations in the current slice

Remaining honest gaps after the 2026-07-13 host campaign:

- no provider registers real load-plan work yet: gameplay routes carry no
  barrier, so activation is immediate (the load/authorization stack itself is
  live and one-shot-tested);
- the combined audio surface does not yet prove deterministic
  provider-relative SFX authority (music selection is proven; SFX rides the
  selection but has no cross-provider consumer test);
- world-POINTER resources (`RoomGeometry`/`RoomSet`/metadata/indices) stay
  process-resident between sessions as inert prepared data — republished per
  activation, unread while the sim sleeps (the accepted session-scope
  pattern), but not literally absent at the title;
- persistent host chrome (map menu root, kaleidoscope/dev overlays,
  `SceneEntities` placeholder) survives at the title; gameplay HUD/quest
  widgets are session-scoped;
- provider authoring still duplicates activation/host boilerplate across the
  three customers (P0);
- no rendered/no-window Ambition-host ownership cycle (X1) and no startup
  vanity sequence (B0) / loading activity (B1);
- the windowed lifecycle was exercised through the headless X0 acceptance
  test and the demos' OV1 rendered suites; this dev VM has no display server,
  so the literal `./run_game.sh` window pass remains for a machine with one.

## Evidence-backed ledger

`DONE` means a passing test or machine-derived invariant supports the exact claim. `OPEN` means implementation is absent or incomplete. `BLOCKED` means the dependency that must land first is named.

| ID | Status | Required result |
|---|---|---|
| C0 | DONE | Deterministic App-local character/audio fragment registries; real Ambition/Sanic/Mary-O coexistence; registration-order and separate-App isolation coverage; stable duplicate ownership diagnostics; candidate-before-commit App updates. |
| C0H | DONE (2026-07-13: workspace compiles; `ambition_characters` 375, `ambition_actors` 775, `ambition_audio`, `ambition_combat`, boss/roster/audio registry unit suites all green) | Playable-character, hostile-roster, boss, and audio fragments are immutable after validation; malformed RON is a structured error; registration revalidates and assembles a candidate before App mutation; duplicate identity ownership fails deterministically. |
| C1-char | DONE (2026-07-13: full focused suites + `app_local_catalog_composition` + reachability/app suites green after repairing the untested candidate — see commit "repair + verify the inherited App-local authority patch") | Every production playable-character, hostile-archetype, and boss consumer uses explicit App-local authority: wear/re-wear, construction, brain/action/movement/body resolution, boss behavior/encounters/art/special rows, sprites, collision, manifests, room lowering, reset/hot reload, snapshots, projectiles, encounters/summons, interaction, dialogue, barks, and attack volumes. Playable-character consumers fail visibly when composition is absent; content-free hostile/boss resources remain explicit for reusable frontend/demo Apps, and W0 must reject activation when a selected provider's required fragments are absent. Separate Apps prove isolation and provider defaults coexist without a global winner. |
| C2-char | DONE (2026-07-13: `ambition_workspace_policy` 33 green including `engine.character-authority-is-app-local`; the ratchet caught and forced the fix of two violations in the candidate itself) | Production playable-character, hostile-roster, and boss install/override globals; the global attack-volume function pointer; engine-owned provider boss-asset lists; demo installers; and implicit sprite wrappers are removed. One workspace-policy ratchet rejects their return and rejects optional authority resources in production. Pure test fixtures may construct explicit values without becoming runtime authority. |
| C1-audio-registry | DONE | Process-global music/SFX registry APIs are removed; App-local provider fragments are registered and read explicitly by current bootstrap paths. |
| C1-audio-session | DONE (2026-07-13: `ActiveAudioSelection` + shell-bridge selection; `session_activation_owns_audio_authority_and_home_retires_it` (shell) + provider/audio asserts at every X0 boundary; Sanic's hand-rolled music driver replaced by the provider-agnostic `drive_selected_session_music`) | Active gameplay session selects provider-relative music/SFX authority; activation replaces prior authority; home retirement clears playback ownership; Sanic -> Mary-O switching is proven. |
| C2-audio | DONE | `install_music_registry`, `install_sfx_registry`, `authored_music_registry`, `authored_sfx_registry`, `MUSIC_REGISTRY_OVERRIDE`, and `SFX_REGISTRY_OVERRIDE` are deleted and guarded by policy. |
| W0 | DONE (2026-07-13: `ActiveGameplaySession` strengthened — provider identity, activation id, session scope, captured load-barrier identity (`session_instance_carries_its_load_barrier_identity`), per-experience `GameplaySessionProfile`; world authority republished per activation from immutable prepared data) | One canonical App-local active gameplay-session representation owns current world/provider/session/load authority. |
| W1 | DONE (2026-07-13: `GameplaySimulationRoot` + `simulation_authorized` — one session gate over the whole sim incl. the tick timeline; `simulation_sleeps_at_the_launcher_and_wakes_per_session` (sanic_app) + frozen-timeline asserts at every X0 home visit. Deliberate residual: world-POINTER resources stay process-resident as inert prepared data and are republished per activation — the accepted session-scope-campaign pattern) | Frontend routes safely have no gameplay session; gameplay schedules sleep. |
| W2 | OPEN (host mode: HUD/quest text, room visuals, parallax, moving platforms, LDtk spine roots, player, and audio authority are session-scoped; cameras + GameAssets + audio library are host-owned caches. Remaining: map/kaleidoscope menu roots and dev overlays are still host-resident chrome; a rendered X1 proof is absent) | Camera, HUD, dialog, map, cutscene UI, input, and audio receive explicit host/session ownership. |
| L0 | OPEN (routes currently register no load barrier: activation is immediate; the load/authorization stack is live and one-shot-tested but no provider reports real preparation work yet) | Sanic and Mary-O contribute real preparation through `ambition_load` and produce immutable prepared sessions. |
| L1 | BLOCKED on L0 | Retry, cancellation, supersession, streaming, promotion, and relaunch use fresh transaction authority. |
| P0 | OPEN (the provider contract is uniform — registration + Providers-set activation + scope teardown — and now has THREE customers, but Sanic/Mary-O/Ambition still hand-roll near-identical activation/host boilerplate; extract the shared shape) | Provider authoring surface is compact, documented by an example/test provider, and supplies reusable standalone/load/session defaults. |
| A0 | DONE (2026-07-13: `AmbitionExperiencePlugin` — registration + session-scoped construction from immutable `AmbitionPreparedWorld` (real LDtk data); teardown is the generic scope sweep; direct entry preserved as host configuration (`--direct`/`--start-room`)) | Main Ambition game becomes a provider using the shared lifecycle. |
| A1 | DONE (2026-07-13: `compose_ambition_shell_host` links the three providers; launcher entries derive from registrations (asserted in X0); Exit is a built-in launcher row emitting semantic `ExitProcess`, mapped to `AppExit` by the HOST) | Ambition host derives Ambition + Sanic + Mary-O + Exit from registrations. |
| X0 | DONE (2026-07-13: `shell_host_lifecycle::the_full_multi_game_lifecycle_is_leak_free` — launcher → Sanic → Mary-O → Ambition → fresh Sanic → Exit; zero-state contract at every home (no session/scope/entities/players/audio, frozen timeline), identity contract in every game (provider, one player, worn character, room authority, audio provider, never-reused scope)) | Headless cross-experience cycle proves exact replacement and no stale authority. |
| X1 | OPEN (the demo OV1 suites prove per-session presentation retirement for Sanic/Mary-O; the Ambition host has no no-window rendered cycle yet, and this VM has no display server for a literal windowed pass) | No-window rendered cycle proves camera/UI/input/audio ownership. |
| B0 | OPEN | Startup sequence hands off to launcher while direct route entry remains available. |
| B1 | OPEN | One deterministic loading activity proves engagement, ready-hold, Continue, cleanup, and destination isolation. |
| F0 | LATER | Game-owned credits route and top-level cutscene adapter. |

## Dependency-ordered roadmap

### Phase 0 — Verify and land complete App-local character authority

1. Run `ambition_characters`, `ambition_audio`, `ambition_combat`, `ambition_actors`, `ambition_runtime`, `ambition_content`, `ambition_app`, both demos in visible/headless modes, and `ambition_workspace_policy`.
2. Confirm malformed playable-character, hostile-roster, and boss fragments return stable structured errors; duplicate identities report both providers; and failed registration preserves the previous valid App assembly.
3. Confirm Ambition, Sanic, Mary-O, combined, and separate-App compositions select the expected playable-character, hostile, and boss definitions across spawn, sprite, bark, dialogue, collision, attack geometry, encounter behavior, projectiles, and asset indexing.
4. Confirm multiple provider defaults coexist without one linked game becoming the implicit fallback for another; active-session provider selection remains the later W0 responsibility.
5. Poison missing `CharacterCatalog`; optional-resource fallback; reintroduced install/override globals; implicit sprite lookup; engine-owned provider boss asset lists; and process-global attack resolution. Also prove that explicit content-free `CharacterRoster`/`BossCatalog` resources are safe outside gameplay and that W0 rejects activation when the selected provider's required fragments are absent. Every poison must fail for the intended reason.
6. Run `cargo fmt --all -- --check`, module/doc/agent checks, and resolve every failure before changing C0H/C1-char/C2-char to `DONE`.

### Phase 1 — Finish active session-level audio authority

1. Add explicit active-provider audio selection to prepared/live gameplay-session state.
2. Build deterministic provider-relative music and SFX indexing as required by runtime consumers.
3. Prove Ambition, Sanic, and Mary-O defaults; two-App isolation; home cleanup; and Sanic-to-Mary-O replacement.

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
