# Loading, shell, and frontend integration

> **Purpose:** define the desired reusable engine architecture first, then track the shortest dependency-ordered path from the current repository to that state.
>
> **Current status (2026-07-13):** the multi-game host EXISTS and is proven headlessly. `./run_game.sh` composes the shell-routed host by default: the Ambition launcher (title screen) derives Ambition + Sanic + Mary-O + built-in Exit from provider registrations; each activates a fresh session-scoped gameplay session; `QuitToHome` (F10 in-session) retires the exact session; Exit leaves the process; `--direct` / `--start-room` preserve direct development entry. The App-local character/hostile/boss authority migration is verified `DONE` (compile + full focused suites + policy ratchet). The gameplay simulation carries ONE session gate (`GameplaySimulationRoot` + `simulation_authorized`): session-routed hosts freeze the whole sim — tick timeline included — at frontend routes. Active session audio authority (`ActiveAudioSelection`) is selected per activation by the shell bridge and retired at home. **Review-driven repair (2026-07-14):** audio is now genuinely provider-relative — the host builds its music library from every linked provider's tracks (`combined_music_registry`, shared assets deduped by resolved path), so a host-launched Sanic/Mary-O session plays its OWN music instead of inheriting Ambition's; the sprite catalog is likewise rebuilt from the fully-merged character catalog so host-launched providers draw their real sprites; audio retirement is identity-safe (a stale session cannot silence a newer one); the title route enforces silence or the host's `a_possible_morning` theme; duplicate provider identities fail deterministically; and the standalone Mary-O demo shell is now controllable (input feature). **Second repair pass (2026-07-14):** music authority is now genuinely provider-relative and ENFORCED — a `MusicAuthority` derived from the active selection rides on `MusicIntent` and the director filters candidates through it, so a track that merely exists in the combined library but is foreign to the active provider can never play, and a music-less provider (Mary-O) is deliberate silence (STOP) rather than retaining the previous track; this is proven at the PLAYBACK layer (`MusicPlaybackState.active_track` over the full title→Ambition→Sanic→Mary-O sequence), not merely by inspecting the selection. Session-audio composition now requires the catalog registry (absent provider on activation panics; silence is an explicit empty fragment), and duplicate ROUTE ids (not just experience ids) are rejected transactionally with order-independent diagnostics. Both X0 (headless full cycle) and X1 (no-window rendered, full sequence + exact launcher uniqueness + playback-layer music) pass. Remaining OPEN: real provider load plans (L0/L1 — routes activate without barriers), promoting world authority into the canonical session (Issue 4 tail), extracting `AmbitionExperiencePlugin` to its own provider crate (Issue 5), provider-authoring dedup (P0), the headless-host entrypoint alignment (Issue 11), byte-level cross-provider SFX bank merging (needs a Rust `.sfxbank` encoder — SFX authority + id-level combined index are DONE, see `C1-sfx-session`), controller-input acceptance, the startup/vanity sequence (B0), and the loading activity (B1). **Third repair pass (2026-07-14):** SFX is now provider-relative and ENFORCED at the playback consumer — a `SfxAuthority` derived from the active selection gates `audio_play_sfx_messages` so an id the active provider did not author (its cues ∪ its bank ids) is dropped before it can resolve against the resident bank/synth; Mary-O is silent, Sanic authors its own cues, and dedup/conflict over provider-contributed bank ids is deterministic.

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

Remaining honest gaps after the 2026-07-14 review-driven repair pass:

- **Audio/sprite composition (FIXED 2026-07-14):** the host previously
  resolved music against a resident Ambition-only `AudioLibrary` and built its
  sprite catalog from an Ambition-only snapshot taken before providers
  registered, so a host-launched Sanic/Mary-O session inherited Ambition's
  music and drew colored-rectangle placeholders. The host now builds the music
  library from `combined_music_registry` (every provider's tracks resolvable;
  shared assets deduped by resolved path; genuine id/asset conflicts error) and
  rebuilds the sandbox asset catalog from the fully-merged character catalog.
  Audio retirement is identity-safe (`clear_if_owner`, poison-tested).
- **provider-relative music authority + deliberate silence (2026-07-14, second
  repair pass):** the combined library is now STORAGE, not permission. A
  `MusicAuthority` (Ungoverned | Governed{authorized ids}) derived from
  `ActiveAudioSelection` rides on `MusicIntent`; the director filters candidates
  through it, so a track present in the combined library but foreign to the
  active provider can NEVER play, and a music-less provider (Mary-O) is
  deliberate silence (STOP the base channel) rather than "retain the previous
  track." Proven at the PLAYBACK layer (not just the selection): X1's real
  visible composition asserts `MusicPlaybackState.active_track` — title plays
  `a_possible_morning`, Ambition plays a gameplay track, Quit restores the
  theme, Sanic plays `you_are_too_slow`, Mary-O is silent (`active_track == ""`).
  Session-audio composition now REQUIRES the catalog registry (never Option): a
  gameplay provider must register a fragment (empty for deliberate silence), and
  an absent provider on activation panics — a missing audio system can no longer
  be mistaken for "everyone is silent."
- SFX is now provider-relative and ENFORCED at playback (2026-07-14, third
  pass): a `SfxAuthority` (`Ungoverned | Governed{authorized ids}`) is derived
  from `ActiveAudioSelection` — the ids of the provider's authored procedural
  cues (`SfxRegistry::authorized_cue_ids`) plus the bank ids it contributes
  (`SfxBankRegistry`). `audio_play_sfx_messages` drops any emission whose id the
  active provider did not authorize, BEFORE it resolves against the resident
  bank / synth handles — so the typed cue shortcuts (jump, dash, …) no longer
  bypass authority via the resident Ambition table. A provider that authored no
  SFX (Mary-O) is deliberate silence (permits nothing); Sanic now AUTHORS its
  Dash/Jump cues instead of inheriting Ambition's. The resident bank is
  storage; the `SfxBankRegistry` records which provider contributes each id
  (with a content fingerprint) so combined indexing dedups identical entries and
  rejects the same id resolving to incompatible assets transactionally. Bank ids
  are published into the registry once the resident bank lands
  (`publish_resident_sfx_bank_authority`). Proven by unit poison tests
  (`an_sfx_provider_only_authorizes_its_own_cues_and_bank_ids`,
  `a_provider_with_no_sfx_is_deliberate_silence`, the `authority_gate_tests`
  module: Ambition-only SFX cannot resolve after switching to Sanic, Mary-O is
  silent, a delayed emission from session A is judged by B's authority, dedup +
  conflict in both registration orders). Byte-level bank-payload composition
  (merging distinct provider banks into one resident bank) still awaits a Rust
  bank encoder — only the Python packer writes `.sfxbank` today; the authority
  and id-level combined index are complete.
- no provider registers real load-plan work yet: gameplay routes carry no
  barrier, so activation is immediate (the load/authorization stack itself is
  live and one-shot-tested) — L0/L1 OPEN;
- **canonical world authority (Issue 4, partial):** `ActiveGameplaySession`
  owns provider/activation/scope/load-barrier identity, but the current WORLD
  (`RoomGeometry`/`RoomSet`/metadata/indices) lives in process-resident
  resources republished per activation, not held by the session struct. They
  are inert prepared data, unread while the sim sleeps (the accepted
  session-scope pattern), but NOT literally absent at the title. Promoting
  world authority into the session (or a session-owned handle) remains OPEN.
- `AmbitionExperiencePlugin` lives in the `ambition_app` crate and leans on
  app-local startup (`AmbitionPreparedWorld`, `init_sandbox_resources`); it is
  a provider peer in behavior but not yet extracted to its own provider crate
  (Issue 5) — OPEN;
- persistent host chrome (map menu root, kaleidoscope/dev overlays,
  `SceneEntities` placeholder) survives at the title; gameplay HUD/quest
  widgets are session-scoped (W2 tail);
- provider authoring still duplicates activation/host boilerplate across the
  three customers (P0);
- input acceptance covers raw keyboard launcher nav/confirm (unit-tested) and
  the F10 Quit-to-Home binding; a controller-mapping acceptance test is OPEN;
- the shipping headless fallback (`--headless-ticks`) still runs the direct
  sandbox path, not the composed title host (Issue 11) — OPEN;
- no startup vanity sequence (B0) / loading activity (B1);
- the windowed lifecycle is exercised through the headless X0 acceptance test
  AND the full no-window rendered X1 cycle (Ambition → Sanic → Mary-O →
  relaunch, exact launcher uniqueness) over the real visible composition; this
  dev VM has no display server, so the literal `./run_game.sh` window pass
  (and audio-device playback, incl. the `a_possible_morning` title theme)
  remains for a machine with one — those ship as blind fixes.

## Evidence-backed ledger

`DONE` means a passing test or machine-derived invariant supports the exact claim. `OPEN` means implementation is absent or incomplete. `BLOCKED` means the dependency that must land first is named.

| ID | Status | Required result |
|---|---|---|
| C0 | DONE | Deterministic App-local character/audio fragment registries; real Ambition/Sanic/Mary-O coexistence; registration-order and separate-App isolation coverage; stable duplicate ownership diagnostics; candidate-before-commit App updates. |
| C0H | DONE (2026-07-13: workspace compiles; `ambition_characters` 375, `ambition_actors` 775, `ambition_audio`, `ambition_combat`, boss/roster/audio registry unit suites all green) | Playable-character, hostile-roster, boss, and audio fragments are immutable after validation; malformed RON is a structured error; registration revalidates and assembles a candidate before App mutation; duplicate identity ownership fails deterministically. |
| C1-char | DONE (2026-07-13: full focused suites + `app_local_catalog_composition` + reachability/app suites green after repairing the untested candidate — see commit "repair + verify the inherited App-local authority patch") | Every production playable-character, hostile-archetype, and boss consumer uses explicit App-local authority: wear/re-wear, construction, brain/action/movement/body resolution, boss behavior/encounters/art/special rows, sprites, collision, manifests, room lowering, reset/hot reload, snapshots, projectiles, encounters/summons, interaction, dialogue, barks, and attack volumes. Playable-character consumers fail visibly when composition is absent; content-free hostile/boss resources remain explicit for reusable frontend/demo Apps, and W0 must reject activation when a selected provider's required fragments are absent. Separate Apps prove isolation and provider defaults coexist without a global winner. |
| C2-char | DONE (2026-07-13: `ambition_workspace_policy` 33 green including `engine.character-authority-is-app-local`; the ratchet caught and forced the fix of two violations in the candidate itself) | Production playable-character, hostile-roster, and boss install/override globals; the global attack-volume function pointer; engine-owned provider boss-asset lists; demo installers; and implicit sprite wrappers are removed. One workspace-policy ratchet rejects their return and rejects optional authority resources in production. Pure test fixtures may construct explicit values without becoming runtime authority. |
| C1-audio-registry | DONE | Process-global music/SFX registry APIs are removed; App-local provider fragments are registered and read explicitly by current bootstrap paths. |
| C1-audio-session | DONE for MUSIC, ENFORCED (2026-07-14 second pass: the shell bridge selects the provider's real `MusicRegistry` into `ActiveAudioSelection`; a `MusicAuthority` derived from it rides on `MusicIntent` and the director FILTERS candidates through it, so a combined-library track foreign to the active provider cannot play; identity-safe retirement `stale_retirement_does_not_clear_a_newer_selection`; shared-asset dedup; frontend silence/title-theme via `FrontendMusicPolicy`. Proven at the PLAYBACK layer by `shell_host_rendered::provider_relative_music_drives_the_base_channel` — title `a_possible_morning`, Ambition gameplay track, Sanic `you_are_too_slow`, Mary-O `active_track==""`; X0 asserts the Issue-1 poison that a Sanic session does not authorize Ambition's resident default track). SFX is now provider-relative and enforced too — see `C1-sfx-session`. | Active gameplay session selects provider-relative music/SFX authority; activation replaces prior authority; home retirement clears playback ownership; Sanic -> Mary-O switching is proven. |
| MUSIC-SILENCE | DONE (2026-07-14: `MusicAuthority::Governed{empty}` = deliberate silence; the director stops the base + adaptive channels once rather than retaining the prior track; unit poison tests `a_silent_provider_authorizes_nothing`, `a_provider_with_no_music_is_deliberate_silence`; playback proof Mary-O `active_track==""`) | An empty provider music registry means STOP, not "leave the previous track playing." |
| AUDIO-REQUIRED | DONE (2026-07-14: `select_session_audio_authority` takes `Res<AudioCatalogRegistry>` (never Option) and the bridge inits it; a gameplay provider absent from the registry panics on activation; Mary-O now registers an explicit empty fragment; `activating_a_provider_with_no_audio_fragment_panics`) | A host missing the audio registry is a composition error, not inferred silence; silence is an explicit empty registration. |
| DUP-ROUTE | DONE (2026-07-14: `register_experience` validates the whole candidate against BOTH the experience registry and route catalog before mutating either — a second experience claiming a registered route id panics instead of silently clobbering via `BTreeMap::insert`; diagnostics canonicalize both owners so the message is byte-identical in either order; `duplicate_route_id_is_rejected_in_both_orders_with_one_message`, `duplicate_experience_id_diagnostic_is_order_independent`, `preexisting_route_blocks_a_later_experience_claiming_it`) | Duplicate route ids fail deterministically and transactionally; diagnostics are registration-order-independent. |
| C1-audio-composition | DONE (2026-07-14: host music library built from `combined_music_registry`; per-track `asset_path` fallback resolves demo tracks; duplicate-by-resolved-path deduped, id/asset conflict errors; sprite catalog rebuilt from merged character catalog — X0/X1 green) | The shared host resolves EVERY linked provider's music (and character sprites) without an Ambition-only library/catalog and without a per-provider host match. |
| C1-adaptive-music | DONE for authority (2026-07-14: `MusicAuthority::Governed` now carries `authorized_cues` alongside track ids; `compute_music_intent` folds in the active provider's authored cue ids from a new provider-relative `AdaptiveCueRegistry` (Ambition registers its cues where it inserts the `MusicCueCatalog`); the director downgrades an unauthorized adaptive `Play` to `None` via `authorized_adaptive`, so a Sanic/Mary-O session cannot start an Ambition cue that merely exists in the process-wide catalog, and a stale directive is shut down rather than played. `is_deliberate_silence` now requires BOTH empty tracks AND empty cues. Proven by `director::adaptive_authority_tests::{a_foreign_adaptive_cue_is_downgraded_to_stop, the_authoring_provider_keeps_its_cue, ungoverned_and_stop_pass_through}`, `selection::tests::music_authority_governs_adaptive_cues_separately_from_tracks`, `catalog::tests::adaptive_cue_registry_indexes_cues_per_provider`) | A provider cannot activate an adaptive cue merely because it exists in a process-wide catalog; adaptive layers stop on retirement; foreign/stale cues cannot start. |
| C1-audio-reset | PARTIAL (2026-07-14: cross-provider request-state leakage is structurally prevented — the music/SFX authority filters drop any track/cue/id the active provider did not author, so stale `RoomMusicRequest`/`RadioStationState`/`EncounterMusicRequest`/adaptive state from session A cannot play in a session B of a DIFFERENT provider. Returning home resets `MusicDirectorState`+`MusicIntent` via `apply_frontend_music_policy` and `EncounterMusicRequest` via the sandbox reset. OPEN: fully session-OWNED request resources with identity-tagged reset on Activated for same-provider relaunch — deferred because `ambition_actors` cannot read `GameplaySessionEvent` (no dep on `ambition_game_shell`) and the sandbox-reset system is already at Bevy's 16-param limit; needs a session-owned resource bundle or a reset hook in a crate that sees both layers.) | Room/radio/encounter/adaptive request state is activation-local; same-provider relaunch gets fresh state. |
| C1-sfx-session | DONE for authority + enforcement (2026-07-14 third pass: `SfxAuthority` derived from `ActiveAudioSelection` = authored cue ids (`SfxRegistry::authorized_cue_ids`) ∪ contributed bank ids (`SfxBankRegistry`); `audio_play_sfx_messages` drops any id the active provider did not authorize BEFORE resolving against the resident bank/synth — typed cues no longer bypass authority; Mary-O silent, Sanic authors its own Dash/Jump; `publish_resident_sfx_bank_authority` publishes the resident bank's ids under Ambition. Proven by `selection::tests::an_sfx_provider_only_authorizes_its_own_cues_and_bank_ids`, `a_provider_with_no_sfx_is_deliberate_silence`, `bank_asset::authority_gate_tests::*` (Ambition-only SFX cannot resolve in Sanic; Mary-O silent; stale-A judged by B), `catalog::tests::{shared_sfx_entry_across_providers_is_deduped_not_a_conflict, conflicting_sfx_entry_is_rejected_transactionally_in_both_orders, separate_apps_hold_independent_sfx_bank_registries}`). OPEN: byte-level bank-payload merge (distinct provider banks → one resident bank) needs a Rust `.sfxbank` encoder (only the Python packer writes banks today). | Every `SfxMessage` resolves only against the active provider's authorized SFX set; typed cues obey it; Mary-O inherits no SFX; dedup/conflict are deterministic. |
| C2-audio | DONE | `install_music_registry`, `install_sfx_registry`, `authored_music_registry`, `authored_sfx_registry`, `MUSIC_REGISTRY_OVERRIDE`, and `SFX_REGISTRY_OVERRIDE` are deleted and guarded by policy. |
| W0 | PARTIAL (2026-07-13: `ActiveGameplaySession` owns provider identity, activation id, session scope, captured load-barrier identity (`session_instance_carries_its_load_barrier_identity`), per-experience `GameplaySessionProfile`). HONEST CAVEAT (Issue 4): the current WORLD (`RoomGeometry`/`RoomSet`/metadata) is process-resident, republished per activation — NOT held by the session struct. Identity authority is canonical; world authority is not yet session-owned. OPEN: promote world authority into the session or a session-owned handle. | One canonical App-local active gameplay-session representation owns current world/provider/session/load authority. |
| DUP-PROVIDER | DONE (2026-07-14: `register_experience` rejects a conflicting duplicate experience id before mutating any catalog, naming both owners order-independently; identical re-registration idempotent — `conflicting_duplicate_experience_id_panics`, `identical_re_registration_is_idempotent`, `launcher_entries_stay_unique_and_ordered`) | Duplicate provider identities fail deterministically; launcher entries stay unique and ordered. |
| RAW-INPUT | DONE for keyboard (2026-07-14: `basic_shell_keyboard` raw `ButtonInput<KeyCode>` → launcher nav/confirm — `arrow_keys_move_the_launcher_cursor`, `enter_and_space_confirm_the_selection`, `keyboard_is_inert_when_launcher_is_not_active`; F10 Quit-to-Home binding is `quit_to_home_on_key`). Controller-mapping acceptance OPEN. | The title launcher responds to actual input resources, not only injected commands. |
| MARY-O-INPUT | DONE (2026-07-14: `ambition_demo_smb1_app` `visible` now folds in the `input` feature, mirroring Sanic — the standalone Mary-O window is controllable; builds green) | The standalone Mary-O demo shell is playable. |
| W1 | DONE (2026-07-13: `GameplaySimulationRoot` + `simulation_authorized` — one session gate over the whole sim incl. the tick timeline; `simulation_sleeps_at_the_launcher_and_wakes_per_session` (sanic_app) + frozen-timeline asserts at every X0 home visit. Deliberate residual: world-POINTER resources stay process-resident as inert prepared data and are republished per activation — the accepted session-scope-campaign pattern) | Frontend routes safely have no gameplay session; gameplay schedules sleep. |
| W2 | OPEN (host mode: HUD/quest text, room visuals, parallax, moving platforms, LDtk spine roots, player, and audio authority are session-scoped; cameras + GameAssets + audio library are host-owned caches. Remaining: map/kaleidoscope menu roots and dev overlays are still host-resident chrome; a rendered X1 proof is absent) | Camera, HUD, dialog, map, cutscene UI, input, and audio receive explicit host/session ownership. |
| L0 | OPEN (routes currently register no load barrier: activation is immediate; the load/authorization stack is live and one-shot-tested but no provider reports real preparation work yet) | Sanic and Mary-O contribute real preparation through `ambition_load` and produce immutable prepared sessions. |
| L1 | BLOCKED on L0 | Retry, cancellation, supersession, streaming, promotion, and relaunch use fresh transaction authority. |
| P0 | OPEN (the provider contract is uniform — registration + Providers-set activation + scope teardown — and now has THREE customers, but Sanic/Mary-O/Ambition still hand-roll near-identical activation/host boilerplate; extract the shared shape) | Provider authoring surface is compact, documented by an example/test provider, and supplies reusable standalone/load/session defaults. |
| A0 | DONE (2026-07-13: `AmbitionExperiencePlugin` — registration + session-scoped construction from immutable `AmbitionPreparedWorld` (real LDtk data); teardown is the generic scope sweep; direct entry preserved as host configuration (`--direct`/`--start-room`)) | Main Ambition game becomes a provider using the shared lifecycle. |
| A1 | DONE (2026-07-13: `compose_ambition_shell_host` links the three providers; launcher entries derive from registrations (asserted in X0); Exit is a built-in launcher row emitting semantic `ExitProcess`, mapped to `AppExit` by the HOST) | Ambition host derives Ambition + Sanic + Mary-O + Exit from registrations. |
| X0 | DONE (2026-07-13: `shell_host_lifecycle::the_full_multi_game_lifecycle_is_leak_free` — launcher → Sanic → Mary-O → Ambition → fresh Sanic → Exit; zero-state contract at every home (no session/scope/entities/players/audio, frozen timeline), identity contract in every game (provider, one player, worn character, room authority, audio provider, never-reused scope)) | Headless cross-experience cycle proves exact replacement and no stale authority. |
| X1 | DONE (2026-07-14: `shell_host_rendered::rendered_ownership_across_the_title_and_two_games` drives `build_visible_app(NoWindow)` — the real windowed composition minus window/backend — through title → Ambition → title → Sanic → title → Mary-O → title → relaunch, asserting host-camera constancy, EXACTLY ONE launcher UI root at every title stop, and ZERO gameplay presentation (room visuals, HUD, session entities) at every title stop. Found+fixed a real clobber: the grid pause menu despawned every `BevyUiMenuRoot` incl. the launcher's; both producers now arbitrate by identity markers. Residual: no-window mode skips the LDtk painted-tile spine (bevy_ecs_tilemap needs a RenderApp); a literal windowed pass needs a machine with a display server) | No-window rendered cycle proves camera/UI/input/audio ownership. |
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
