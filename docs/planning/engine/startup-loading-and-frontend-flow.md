# Loading, shell, and frontend integration

> **Purpose:** define the desired reusable engine architecture first, then track the shortest dependency-ordered path from the current repository to that state.
>
> **Current status (2026-07-13):** the previously stabilized audio/session overlay now compiles and its targeted suites pass in the developer environment. This next overlay is an unverified candidate for the remaining title/loading/shell sprint: no-window composition selects a recording backend that omits the physical Kira output; provider preparation reports staged catalog, world, sprite, music, SFX, adaptive, defaults, prepared-session, packed-bank, and speculative evidence; loading presentation names current/completed/remaining/streamable/speculative work and Retry creates a fresh provider transaction; frontend cameras, startup/launcher/loading roots, input ownership, and cross-cutting presentation classes are explicit; Pocket selects the reusable deterministic loading activity through provider authoring; startup input and timed advance are covered; and the shipping host has an executable Ambition/Sanic/Mary-O/relaunch/Exit acceptance cycle. These new claims remain `OPEN` until Cargo formatting, focused tests, workspace tests, Clippy, and the executable cycle pass. Literal visible and audible behavior must still be reported separately from automated state evidence.

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
    launcher -> Ambition | Sanic | Mary-O | Pocket | future providers | Exit
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

### Current integration candidate and remaining limits

The current candidate applies one authority rule to every shell experience:

```text
current shell activation
-> exact audio owner
-> explicit frontend or gameplay audio profile
-> provider-relative music, adaptive-cue, procedural-SFX, and bank-SFX sources
-> playback only while that exact owner remains current
```

A frontend route is not an audio-free special case. Startup, launcher, loading,
credits, and other frontend experiences may author music and SFX through their
own exact activation context. What is forbidden is stale gameplay work playing
under a later frontend or gameplay activation.

The candidate introduces these implementation seams, all awaiting Rust
verification:

- `AudioContextOwner` distinguishes frontend, gameplay, and explicit direct-app
  ownership. `SfxWriter` captures that owner when work is emitted, so delayed
  work can be rejected even across a same-provider relaunch.
- `ActiveAudioSelection` selects an explicit frontend or provider profile;
  absence of a gameplay session no longer means permissive, ungoverned SFX.
- provider-authored procedural SFX resolve from the active provider's actual
  `SfxSpec`, while provider-qualified packed banks are stored independently and
  may become ready after activation without granting authority to another
  provider.
- the real playback decision records provider, owner, logical ID, source kind,
  and stable source identity so tests can distinguish authorization metadata
  from the sound source actually selected.
- complete adaptive cue definitions are composed per provider rather than
  keeping one replaceable process-wide cue catalog.
- audio request/director/playback resources reset when the exact shell audio
  context changes, using a focused `SystemParam` bundle to stay below Bevy's
  system-parameter limit.
- the no-display `--headless-ticks` path is redirected through the shared host
  unless explicit direct-development mode is requested.
- generated agent metadata omits wall-clock and self-invalidating HEAD fields so
  repeated generation can be byte-stable.

The current candidate implements the remaining shell-facing tranche without claiming compiler evidence:

- no-window hosts install audio assets and inert channels without the physical Kira output or command-drain systems;
- exact frontend authority and entity ownership distinguish host cameras, startup, launcher, loading, kaleidoscope, developer, and debug presentation from gameplay-session presentation;
- the shared provider preparation `SystemParam` emits stage-by-stage evidence and publishes one exact immutable prepared-session identity;
- Retry is generic for provider-authored plans and creates a fresh superseding transaction, while externally owned static barriers retain the event-only retry seam;
- the loader reports named current, completed, required-next, streamable, and speculative work;
- Pocket selects a deterministic neutral-input loading activity through provider registration with no host branch;
- startup auto-advance and keyboard/controller acknowledgement use the same shell action vocabulary;
- the shipping composition exposes a no-window multi-provider acceptance cycle;
- one exact `SessionRoot` owns the live `RoomSet`, `RoomGeometry`, active-room metadata, starting character, runtime-room index, provider selections, and audio request components; the former process-resource projection and its two-way synchronization bridge are deleted;
- focused `SessionWorldRef` / `SessionWorldMut` parameters make gameplay-world access structurally unavailable at frontend routes, while direct hosts construct the same root bundle once;
- a workspace-policy ratchet rejects the deleted bridge names and rejects any canonical world component regaining a `Resource` derive.

Literal windowed and audible observation still requires a display/audio host.


## Evidence-backed ledger

`DONE` means a passing test or machine-derived invariant supports the exact claim. `OPEN` means implementation is absent or incomplete. `BLOCKED` means the dependency that must land first is named.

| ID | Status | Required result |
|---|---|---|
| W0-live-world | OPEN | Implementation owns one exact mutable `SessionRoot` component bundle, deletes all resident world mirrors and synchronization code, migrates gameplay consumers to focused root queries, and adds room-change, stale-publication, stale-retirement, same-provider freshness, frontend-unavailability, and policy-ratchet evidence. Awaiting Rust verification on this tranche. |
| P0-provider-api | OPEN | Candidate provides common registration, loading spec, staged preparation, prepared store, session builder, and teardown surfaces; Pocket proves standalone/shared authoring without host match arms. Awaiting Rust verification. |
| L0-fresh-load | OPEN | Candidate mints fresh per-launch transactions, publishes exact prepared identity, consumes one-shot authorization, retires exact authority, and adds retry/relaunch stale-work tests. Awaiting Rust verification. |
| INPUT0 | OPEN | Candidate unifies startup, launcher, loading, Continue, back, Quit-to-Home, keyboard, D-pad, face-button, Start-button, and analog-edge actions. Awaiting focused input suites. |
| LOAD-ACTIVITY0 | OPEN | Candidate provider-selectable activity consumes real keyboard/controller shell actions, records an optional result, holds readiness when engaged, cleans its load scope, and leaves destination state unchanged. Awaiting feature-suite execution. |
| C0 | DONE | Deterministic App-local character/audio fragment registries; real Ambition/Sanic/Mary-O coexistence; registration-order and separate-App isolation coverage; stable duplicate ownership diagnostics; candidate-before-commit App updates. |
| C0H | DONE | Playable-character, hostile-roster, boss, and audio fragments are immutable after validation; malformed RON is a structured error; registration revalidates and assembles a candidate before App mutation; duplicate identity ownership fails deterministically. Evidence status note: 2026-07-13: workspace compiles; `ambition_characters` 375, `ambition_actors` 775, `ambition_audio`, `ambition_combat`, boss/roster/audio registry unit suites all green. |
| C1-char | DONE | Every production playable-character, hostile-archetype, and boss consumer uses explicit App-local authority: wear/re-wear, construction, brain/action/movement/body resolution, boss behavior/encounters/art/special rows, sprites, collision, manifests, room lowering, reset/hot reload, snapshots, projectiles, encounters/summons, interaction, dialogue, barks, and attack volumes. Playable-character consumers fail visibly when composition is absent; content-free hostile/boss resources remain explicit for reusable frontend/demo Apps, and W0 must reject activation when a selected provider's required fragments are absent. Separate Apps prove isolation and provider defaults coexist without a global winner. Evidence status note: 2026-07-13: full focused suites + `app_local_catalog_composition` + reachability/app suites green after repairing the untested candidate — see commit "repair + verify the inherited App-local authority patch". |
| C2-char | DONE | Production playable-character, hostile-roster, and boss install/override globals; the global attack-volume function pointer; engine-owned provider boss-asset lists; demo installers; and implicit sprite wrappers are removed. One workspace-policy ratchet rejects their return and rejects optional authority resources in production. Pure test fixtures may construct explicit values without becoming runtime authority. Evidence status note: 2026-07-13: `ambition_workspace_policy` 33 green including `engine.character-authority-is-app-local`; the ratchet caught and forced the fix of two violations in the candidate itself. |
| C1-audio-registry | DONE | Process-global music/SFX registry APIs are removed; App-local provider fragments are registered and read explicitly by current bootstrap paths. |
| C1-audio-session | OPEN | Every shell activation owns an explicit audio context; frontend and gameplay music/SFX resolve through the exact current owner; stale retirement or queued work cannot affect a newer activation. Evidence status note: exact frontend/gameplay audio-context implementation candidate awaiting Rust verification. |
| MUSIC-SILENCE | OPEN | A provider or frontend profile that deliberately authors no music stops prior playback rather than retaining it. Evidence status note: explicit governed-empty playback behavior is implemented but must be reverified after the unified context rewrite. |
| AUDIO-REQUIRED | OPEN | Missing audio composition is an error; deliberate silence is a valid explicit profile. Evidence status note: required registry plus explicit empty fragments retained in the candidate; focused tests must pass. |
| DUP-ROUTE | DONE | Duplicate route ids fail deterministically and transactionally; diagnostics are registration-order-independent. Evidence status note: 2026-07-14: `register_experience` validates the whole candidate against BOTH the experience registry and route catalog before mutating either — a second experience claiming a registered route id panics instead of silently clobbering via `BTreeMap::insert`; diagnostics canonicalize both owners so the message is byte-identical in either order; `duplicate_route_id_is_rejected_in_both_orders_with_one_message`, `duplicate_experience_id_diagnostic_is_order_independent`, `preexisting_route_blocks_a_later_experience_claiming_it`. |
| C1-audio-composition | DONE | The shared host builds its combined music and character-sprite caches from all linked provider fragments without provider-specific host matches. |
| C1-adaptive-music | OPEN | Actual adaptive cue definitions, encounter bindings, loaded layers, and playback authority are provider-relative and exact-owner scoped. Evidence status note: complete provider-composable cue catalogs and provider-qualified loaded layers are implemented as a Rust-verification candidate. |
| C1-audio-reset | OPEN | Room, radio, encounter, adaptive, director, playback, and SFX request state is activation-local; relaunch starts fresh without damaging a newer owner. Evidence status note: focused context-change reset bundle implemented; same-provider and stale-retirement tests must compile and pass. |
| C1-sfx-session | OPEN | Frontend and gameplay SFX resolve from the exact active profile and provider source; delayed work, silent providers, late banks, and same-provider relaunch are identity-safe. Evidence status note: actual provider procedural/bank source resolution, exact-owner messages, late-bank refresh, frontend SFX, and playback records are implemented as a Rust-verification candidate. |
| C2-audio | DONE | `install_music_registry`, `install_sfx_registry`, `authored_music_registry`, `authored_sfx_registry`, `MUSIC_REGISTRY_OVERRIDE`, and `SFX_REGISTRY_OVERRIDE` are deleted and guarded by policy. |
| W0 | OPEN | One canonical live session-owned root contains room set, geometry, metadata, starting character, runtime indices, provider selections, load authorization, and request state. All gameplay consumers read root components directly; the old process-resource projection is deleted and structurally forbidden. Awaiting Rust verification before `DONE`. |
| DUP-PROVIDER | DONE | Duplicate provider identities fail deterministically; launcher entries stay unique and ordered. Evidence status note: 2026-07-14: `register_experience` rejects a conflicting duplicate experience id before mutating any catalog, naming both owners order-independently; identical re-registration idempotent — `conflicting_duplicate_experience_id_panics`, `identical_re_registration_is_idempotent`, `launcher_entries_stay_unique_and_ordered`. |
| RAW-INPUT | DONE | The title launcher responds to actual keyboard AND controller input resources, not only injected commands. Evidence status note: for keyboard AND controller (2026-07-14: `basic_shell_keyboard` unifies raw `ButtonInput<KeyCode>` and `Query<&Gamepad>` through one neutral `menu_nav_edges` → launcher nav/confirm; D-pad mirrors arrows, South mirrors Enter/Space — `arrow_keys_move_the_launcher_cursor`, `enter_and_space_confirm_the_selection`, `keyboard_is_inert_when_launcher_is_not_active`, `controller_dpad_and_south_drive_the_same_launcher_commands_as_the_keyboard` (spawns a real `Gamepad`, presses via `digital_mut`), `controller_is_inert_when_launcher_is_not_active`. Quit-to-Home binding `quit_to_home_on_key` accepts F10 OR the controller Start button. X0's launcher walk derives its row count from `ShellLaunchCatalog.entries` + the Exit action, no literal.. |
| MARY-O-INPUT | DONE | The standalone Mary-O demo shell is playable. Evidence status note: 2026-07-14: `ambition_demo_smb1_app` `visible` now folds in the `input` feature, mirroring Sanic — the standalone Mary-O window is controllable; builds green. |
| W1 | DONE | Frontend routes safely have no gameplay session; gameplay schedules sleep. Evidence status note: 2026-07-13: `GameplaySimulationRoot` + `simulation_authorized` — one session gate over the whole sim incl. the tick timeline; `simulation_sleeps_at_the_launcher_and_wakes_per_session` (sanic_app) + frozen-timeline asserts at every X0 home visit. Prepared provider values remain immutable load output only; live gameplay world authority exists solely on the exact session root. |
| W2 | OPEN | Candidate marks host/frontend/session ownership explicitly and expands X1 to assert actual player, camera, input, HUD, dialog, map, room, moving-platform, load, world, and audio authority at every title stop. Awaiting rendered-suite execution. |
| L0 | OPEN | Candidate providers execute shared staged preparation over real catalog/world/sprite/audio/default evidence, then publish an immutable prepared session behind a fresh required barrier. Awaiting focused provider/load tests. |
| L1 | OPEN | Candidate implements fresh Retry, cancellation, supersession, streamable/speculative work, stale-completion rejection, one-shot authorization, and fresh same-provider relaunch. Awaiting test execution. |
| P0 | OPEN | Candidate uses one compact Bevy-native authoring path across Ambition, Sanic, Mary-O, and Pocket, including provider-owned loading policy. Awaiting compilation and proof suites. |
| A0 | OPEN | Candidate AmbitionExperiencePlugin lives in ambition_content and an alternate-host test composes it without hidden ambition_app initialization. Awaiting test execution. |
| A1 | DONE | Ambition host derives Ambition + Sanic + Mary-O + Exit from registrations. Evidence status note: 2026-07-13: `compose_ambition_shell_host` links the three providers; launcher entries derive from registrations (asserted in X0); Exit is a built-in launcher row emitting semantic `ExitProcess`, mapped to `AppExit` by the HOST. |
| X0 | OPEN | Candidate lifecycle proof covers fresh identities, canonical live-room mutation, same-provider fresh world, provider audio, exact title zero state, and Exit. Awaiting execution. |
| X1 | OPEN | Candidate no-window shipping composition uses a device-free recording backend and checks exact frontend/gameplay presentation and audio ownership through all linked games. Awaiting execution. |
| B0 | OPEN | Candidate tests natural startup timing plus keyboard and controller acknowledgement, exact frontend authority/audio, no gameplay authority, cleanup, and one launcher handoff. Awaiting execution. |
| B1 | OPEN | Candidate reusable deterministic activity proves neutral input, engagement, optional result, ready-hold, Continue, load-scope cleanup, and destination isolation. Awaiting execution. |
| F0 | OPEN | Game-owned credits route and top-level cutscene adapter. |

## Dependency-ordered roadmap

### Phase 0 — Compile and repair the exact shell-audio context candidate

1. Run formatting and the independent `ambition_audio --features kira` check before broader tests.
2. Repair every API migration failure from `SfxMessage` to exact-owner `OwnedSfxMessage`/`SfxWriter` without adding oversized Bevy systems.
3. Prove frontend title/menu SFX, provider-authored procedural SFX, provider-qualified bank SFX, late-bank readiness, deliberate silence, stale same-provider rejection, and actual playback-source identity.
4. Prove complete provider adaptive cue definitions and loaded layers remain isolated.
5. Prove context changes reset room/radio/encounter/director/playback/SFX state without stale retirement damaging the current owner.
6. Run the real shared-host headless entrypoint and the full X0/X1 no-window sequence.
7. Keep these rows `OPEN` until the focused and workspace gates pass.

### Phase 1 — Canonical live gameplay session and exact ownership

1. Replace the activation-time type-erased world snapshot with one live session-owned world bundle/entity.
2. Move `RoomSet`, `RoomGeometry`, active-room metadata, starting character, runtime indices, provider selections, load authorization, and request state into that authority.
3. Migrate consumers behind focused `SystemParam` bundles, beginning with collision/world-flow choke points.
4. Make attach, replacement, transition, hot reload, reset, rollback, and retirement exact-identity operations.
5. Complete frontend ownership for cameras, input, HUD/dialog/map/cutscene roots, developer chrome, music, adaptive layers, SFX queues/loops, and load state.
6. Prove no gameplay-world or gameplay-presentation authority exists at frontend routes.


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

### Workspace-context caveat (2026-07-13)

`cargo test --workspace` compiles every crate under UNIFIED features and is
part of this campaign's gate (it exposed and fixed a latent
feature-unification panic: `PortalObservationPlugin` now owns its
`PortalWorldFrame` seam resource). `--all-features` is NOT a supported
matrix: the `static_core_assets`/web feature family embeds generated sprite
and asset artifacts that are deliberately not in git (regen scripts own
them), so an `--all-features` build fails on a fresh clone by design. The
supported matrix is: per-crate defaults, the demo apps' `--features visible`,
and the feature-specific suites named below.

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
