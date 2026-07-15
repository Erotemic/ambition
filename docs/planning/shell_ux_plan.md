# Shell UX Plan — pause primitive, exit-to-title, presentation timing, control hints

- **Author:** Fable (claude-fable-5)
- **Date:** 2026-07-15
- **Status:** PLANNED (not started)
- **Scope:** the multi-game host's player-facing shell UX. Four interrelated
  features + one design-question sketch. Everything lands in engine crates and
  flows to Ambition AND the demos through the `ambition` umbrella — no demo may
  need a lower `ambition_*` crate (E9 oracle), and nothing new may live only in
  `game/ambition_app`.

## Read this first — the existing seams

The host is already the right shape; this plan adds four small organs to it,
not a new skeleton.

| Seam | Where |
|---|---|
| Host composition (title screen, providers, QuitToHome key) | `game/ambition_app/src/app/shell_host.rs:63` (`compose_ambition_shell_host`), `:131` (`compose_ambition_startup_sequence`), `:268` (`quit_to_home_on_key`) |
| Shell routing + semantic commands | `crates/ambition_game_shell/src/router.rs:187` (`ShellCommand`), `:275` (`QuitToHome` → home route) |
| Session lifecycle bridge (audio authority, activation/retire) | `crates/ambition_game_shell/src/session.rs:311-320` (bridge systems), `:360` (`select_shell_audio_context`) |
| Vanity/startup sequence data + runtime | `crates/ambition_game_shell/src/sequence.rs:45-58` (`ShellSegmentPolicy`, default 2 s auto-advance), `:205` (`tick`) |
| Basic shell presentation (launcher menu + sequence cards) | `crates/ambition_game_shell/src/basic_presentation.rs:117` (`render_basic_shell`), `:220` (`spawn_launcher_menu`) |
| Neutral shell input edges (kb + pad unified) | `crates/ambition_game_shell/src/input.rs:10` (`ShellActionEdges`) |
| Renderer-independent menu model + flat Bevy-UI renderer | `crates/ambition_menu/src/lib.rs:249` (`MenuPageModel`), `ambition_menu::render::bevy_ui::spawn_bevy_ui_menu_with_assets` (used at `basic_presentation.rs:8`) |
| Frontend audio profile + authority | `crates/ambition_audio/src/selection.rs:122` (`FrontendAudioProfile`), `crates/ambition_actors/src/audio/plugin.rs:280` (`apply_frontend_music_policy`) |
| Pause vocabulary (engine-level, demos get it free) | `crates/ambition_platformer_primitives/src/schedule.rs:310` (`GameMode`), `:298` (`gameplay_suspended`); zeroing wired in `crates/ambition_runtime/src/player_schedule.rs:53-77` via `apply_suspended_time_scale_system` (`crates/ambition_actors/src/time/time_control/mod.rs:364`) |
| Affordances ("what would each button do right now") | `crates/ambition_actors/src/affordances/mod.rs:75` (`PlayerAffordances`), `:89` (`compute_player_affordances`), `variants.rs`, `resolvers.rs`, `devices.rs:76` (`ActiveInputMethod`), `:142` (`glyph_for`) |
| Hint consumers today | `crates/ambition_touch_input/src/layout.rs:114-135` (hardcoded "Blink"/"Dash"/"Jump" buttons), `bevy_plugin.rs:791` (`ButtonVerb`), `update_button_verb_from_affordances` |
| Character identity worn by the controlled body | `crates/ambition_characters/src/actor/worn.rs:33` (`WornCharacter`), catalog row `crates/ambition_characters/src/actor/character_catalog/entry.rs:375-377` (`abilities: Option<AbilityKitSpec>`) |
| Umbrella surface | `crates/ambition/src/lib.rs` re-exports `game_shell` (:24), `menu` (:33), `ui_nav` (:48), `audio` (:15) — every type this plan touches is already reachable from demos |
| Standalone demo hosts (must also gain the pause menu by composition) | `game/ambition_demo_smb1_app/src/lib.rs:38` (`compose_smb1_shell`), `game/ambition_demo_sanic_app/src/lib.rs` (same pattern) |

Design stance for all four parts: **the host offers ONE primitive; providers
opt in by registration, hosts opt in by composition.** No per-game match arms,
no bespoke menus per demo, no `ambition_app`-only features.

---

## Part 1 — The shell pause-menu primitive

### Problem

Each hosted experience (Ambition, Sanic, Mary-O, Pocket) needs a minimal
in-session menu: **Resume / Quit to Title / Quit to Desktop**. Today only
Ambition has any pause surface (its app-local grid/kaleidoscope mega-menu,
`game/ambition_app/src/menu/grid_backend.rs:312` `grid_menu_open_routing`,
which sets `GameMode::Paused` at `:418`), and the demos have nothing. The
comment at `shell_host.rs:266` already anticipates this: *"The in-game pause
menu can grow a 'Quit to Home' entry on top of the same command."*

### The seam

Everything needed already exists in `ambition_game_shell` + its deps:

- **Pause semantics**: `GameMode::Paused` is engine vocabulary
  (`ambition_platformer_primitives/src/schedule.rs:310`), and the sim-clock
  zeroing that makes pause REAL is wired engine-side in
  `ambition_runtime/src/player_schedule.rs:53-77` — so a Sanic session pauses
  correctly the moment anything sets the state. No app code involved.
- **Quit semantics**: `ShellCommand::QuitToHome` and
  `ShellCommand::ExitProcess` (`router.rs:187-193`) are already
  provider-agnostic; the host maps `ExitRequested` → `AppExit`
  (`shell_host.rs:255`).
- **Menu rendering**: the launcher already proves the pattern — build a
  `MenuPageModel`, render with `spawn_bevy_ui_menu_with_assets`, mark the root
  with an identity component (`basic_presentation.rs:220-343`,
  `BasicShellUiRoot` at `:36`). The pause menu is the SAME pattern with three
  rows.
- **Input**: `shell_action_edges` (`input.rs`) already unifies kb/pad
  navigation edges and carries a `quit_to_home` edge (F10/Start).

### The smallest elegant change

New module `crates/ambition_game_shell/src/pause.rs` + `ShellPauseMenuPlugin`,
exported from `game_shell` and therefore from `ambition::game_shell`.

```rust
/// Host-composed in-session pause surface. One primitive for every provider.
pub struct ShellPauseMenuPlugin;

/// Cursor + open state. Session-relative: force-closed on session retire.
#[derive(Resource, Default)]
pub struct ShellPauseMenu {
    pub open: bool,
    pub selected: usize,          // over the enabled entries
}

/// Host presentation knobs; labels only, no routes, no game names.
#[derive(Resource)]
pub struct ShellPauseMenuSpec {
    pub resume_label: String,          // "Resume"
    pub quit_to_title_label: String,   // "Quit to Title"
    pub quit_to_desktop_label: Option<String>, // None on platforms that forbid self-exit
}

/// Semantic entries — the menu dispatches SHELL commands, never routes.
enum ShellPauseEntry { Resume, QuitToTitle, QuitToDesktop }
```

Three systems (mirroring the launcher's decomposition, all in `Update`):

1. `shell_pause_toggle` — runs only while `ActiveGameplaySession.0.is_some()`.
   Reads a new `pause` edge added to `ShellActionEdges` (Esc keyboard + Start
   pad; see arbitration below). Toggling open sets
   `NextState<GameMode>::Paused`; closing sets `Playing`. Also force-closes
   (and restores `Playing`) when a `GameplaySessionEvent::Retiring` arrives, so
   quit-to-title never leaves the host stuck in `Paused` — add this reset to the
   activation path in `translate_shell_session_lifecycle`
   (`session.rs`) as a belt-and-braces invariant: **a fresh activation always
   begins in `GameMode::Playing`.**
2. `shell_pause_nav` — while open: up/down move `selected` (reusing
   `shell_action_edges`), confirm dispatches:
   `Resume` → close; `QuitToTitle` → `ShellCommand::QuitToHome` + close;
   `QuitToDesktop` → `ShellCommand::ExitProcess`. Menu SFX exactly as the
   launcher does (`ids::UI_MENU_MOVE` / `UI_MENU_ACCEPT`,
   `basic_presentation.rs:75-92`).
3. `render_shell_pause_menu` — frame-key respawn pattern copied from
   `render_basic_shell` (`basic_presentation.rs:117-146`): a 3-row
   `MenuPageModel` (title = the active experience's registered label, read
   from `ShellExperienceRegistry` via
   `ActiveGameplaySession` — free flavor, zero config), rendered with
   `spawn_bevy_ui_menu_with_assets`, root marked with a new
   `ShellPauseUiRoot` identity component (menu-root identity lesson: teardown
   queries `(With<BevyUiMenuRoot>, With<ShellPauseUiRoot>)` ONLY — never claim
   another producer's roots).

**Opt-in shape.** The HOST composes `ShellPauseMenuPlugin` (one line in
`compose_ambition_shell_host` after `MinimalShellPlugins`, and one line in each
standalone demo host, e.g. `compose_smb1_shell` in
`game/ambition_demo_smb1_app/src/lib.rs:38`). Experiences that own a bespoke
pause surface opt OUT at registration:
`ExperienceRegistration::with_own_pause_surface()`
(`crates/ambition_game_shell/src/experience.rs:54`) — the toggle system skips
sessions whose experience set that flag. Default is opted-in, so Sanic, Mary-O,
Pocket, and Ambition all get the menu with zero provider code.

### Arbitration with Ambition's existing menu + the raw Start binding

Two existing claims on the same buttons must yield by identity, not by system
order:

- `quit_to_home_on_key` (`shell_host.rs:268`) currently maps **Start/F10 →
  instant QuitToHome**. When the host composes `ShellPauseMenuPlugin`, Start
  must open the pause menu instead. Change: keep F10 as the dev
  instant-quit chord; remove Start from `ShellActionEdges::quit_to_home` and
  give it to the new `pause` edge. (`quit_to_home_on_key` keeps working via
  F10; the player-facing path becomes pause → Quit to Title.)
- Ambition's grid menu opens on the same pause button
  (`grid_backend.rs:312`, target `MenuPage::System` via `pause_entry_target`
  `:187`). Decision: **in the shell-hosted composition, Esc/Start belongs to
  the shell pause menu**; Ambition's grid menu remains bound to its
  inventory/map entries (Tab/I/M) and stays fully reachable. The System face's
  own Quit row is already slated for removal
  (`game/ambition_app/src/menu/dispatch.rs:126` — "the old pause-menu Quit row
  (which is removed in a later phase)"); this plan is that later phase:
  `SystemMenuAction::Quit` → delete the row, the shell menu owns quitting.
  Direct entry (`--direct`, no `AmbitionShellHosted`) keeps today's behavior —
  gate Ambition's pause-entry routing on `direct_entry` (`shell_host.rs:47`)
  and the shell toggle on session presence, so exactly one owner exists per
  composition.

### Why it stays generic

The plugin names no experience, no route, no verbs: entries are semantic
`ShellCommand`s; pause is `GameMode` (engine vocabulary the demos already
run); rendering is `ambition_menu`'s neutral model; the title string comes
from the experience registration the provider already wrote. A hypothetical
fifth game composed from the umbrella gets Resume/Quit-to-Title/Quit-to-Desktop
by adding one plugin line to its host.

---

## Part 2 — "Exit to Title Screen" from a live session

This is deliberately the smallest part: **the path already exists end-to-end;
it just has no menu entry.**

- Retire path: `ShellCommand::QuitToHome` → `router.rs:275` resolves the
  host's `home_route` (`ShellHostSpec`, set at `shell_host.rs:104-109`) →
  route deactivation → `GameplaySessionEvent::Retiring` → generic
  session-scope sweep (leak-free launch/quit/relaunch is already tested) →
  home/launcher route activates → `select_shell_audio_context`
  (`session.rs:420`) flips audio authority back to the frontend profile.
- The menu entry is Part 1's `ShellPauseEntry::QuitToTitle` writing exactly
  `ShellCommand::QuitToHome` — the same message the F10 binding writes today
  (`shell_host.rs:280`). No new lifecycle code. The only genuinely new
  invariant is the `GameMode::Playing` reset on retire/activate described in
  Part 1 (today nothing pauses, so nothing ever needed to unpause).

Acceptance: from a paused Sanic session, Quit to Title lands on the launcher
with zero session entities remaining (reuse the existing leak assertions from
the session-scope tests, `crates/ambition_game_shell/src/session/tests.rs`),
and relaunching Sanic starts unpaused.

---

## Part 3 — Title-screen & vanity-card presentation timing

Three defects, one seam each:

### 3a. Vanity card too short, no fade

- **Seam:** `ShellSegmentPolicy` (`sequence.rs:45-58`) — the "powered by
  Ambition" card uses the default 2 s auto-advance
  (`ShellSegmentSpec::text` at `shell_host.rs:147`). The card is drawn by
  `render_basic_shell` (`basic_presentation.rs:164-217`), which only mutates
  UI on frame-key change — so cards snap in and out.
- **Change (data):** extend `ShellSegmentPolicy` with
  `fade_in: Duration` and `fade_out: Duration` (both default `ZERO` — every
  existing segment behaves identically). Add a **pure** envelope function next
  to the runtime, unit-tested in `sequence.rs`:

  ```rust
  /// Presentation alpha for a segment at `elapsed`. Fade-out anchors to the
  /// auto-advance deadline so the card finishes fading exactly when it leaves;
  /// segments without auto-advance never fade out (they end by skip/ack).
  pub fn segment_alpha(elapsed: Duration, policy: &ShellSegmentPolicy) -> f32
  ```

  (ramp 0→1 over `fade_in`; hold; ramp 1→0 over the last `fade_out` before
  `auto_advance_after`; clamp; `fade_out` asserted ≤ the hold window).
- **Change (presentation):** one new small system in `basic_presentation.rs`,
  `animate_basic_sequence_alpha`, running every frame (not gated by the
  frame key): reads `ActiveShellSequence.runtime` elapsed + current policy,
  writes alpha onto the `BasicSequenceRoot` tree's `TextColor` /
  `ImageNode.color` (the root's near-black `BackgroundColor` stays opaque —
  fading the CONTENT against the held backdrop is the classic vanity look and
  avoids flashing the desktop through). Skip still despawns instantly —
  acceptable; a skip-fade is polish, not architecture.
- **Host tuning** (`shell_host.rs:146-152`): the startup card gets
  `.with_policy(ShellSegmentPolicy { auto_advance_after: Some(4 s),
  fade_in: 600 ms, fade_out: 900 ms, ..default() })`. Numbers are start
  points; ship them blind (draw-blind rule) and mark the commit.

### 3b. Title menu snaps in after the dramatic pause

- **Seam:** the launcher tree is spawned wholesale by `spawn_launcher_menu`
  (`basic_presentation.rs:220`) the frame the launcher route activates.
- **Change:** a **fade shroud**, not per-node alpha plumbing: when the basic
  presentation spawns the launcher root, also spawn one full-screen
  black overlay (`GlobalZIndex` above the menu, marked
  `FrontendOwnedEntity::shell(activation_id, FrontendPresentationKind::…)` —
  add a `TransitionShroud` variant to `FrontendPresentationKind`,
  `frontend.rs:22`) whose alpha animates 1→0 over
  `ShellLauncherPresentation::fade_in` (new field, default ~500 ms;
  `launcher.rs`), then despawns itself. One `animate_frontend_shroud` system,
  ~20 lines. Because the shroud is activation-scoped, route teardown already
  sweeps it; a skipped vanity card fading into the shroud reads as one
  continuous transition.

### 3c. Soundtrack tied to BOOT, should be tied to TITLE

- **Seam today:** `select_shell_audio_context`
  (`crates/ambition_game_shell/src/session.rs:420-448`) selects the frontend
  profile — including its `title_track` — for **every** non-gameplay route
  activation, so `apply_frontend_music_policy`
  (`crates/ambition_actors/src/audio/plugin.rs:280,324-335`) starts
  `a_possible_morning` the moment the startup vanity card activates, then
  silences + restarts it when the launcher route (a new activation id)
  arrives.
- **Change (zero new config):** *the title theme is definitionally the HOME
  route's track.* In `select_shell_audio_context`, read
  `Res<ShellHostConfiguration>` (same crate) and pass the profile's
  `title_track` through to `ActiveAudioSelection::select_frontend`
  (`selection.rs:211`) **only when the activated route id equals
  `ShellHostSpec.home_route`**. Non-home frontend routes (startup card,
  loading, future credits) keep the menu-SFX allowlist but get no preferred
  track — deliberate silence under the vanity card, music lands exactly when
  the title screen does. Mechanically: add
  `FrontendAudioProfile::without_title_track(&self) -> FrontendAudioProfile`
  (a narrowed clone) rather than a boolean parameter.
- **Continuity guard (small, same commit):** in
  `apply_frontend_music_policy`, before `silence_music_backend`, if the new
  frontend context's `preferred_track` equals `music_state.active_track` and
  is still authorized, latch `applied_owner` and return — the theme survives
  home→home reactivations and future frontend sub-routes (settings page)
  without a restart hiccup.
- **Optional polish (blind-fix commit):** start the title theme with a kira
  fade-in (`.play(handle).looped().fade_in(...)`) so 3a/3b/3c compose into one
  soft boot.

Why generic: the rule is host-relative ("home route owns the theme"), so the
standalone Mary-O host — whose home is its own launcher
(`ambition_demo_smb1_app/src/lib.rs`) — gets identical semantics from the same
line of engine code; its profile simply has no track (explicit silence today,
`FrontendAudioProfile::new(MARY_O_EXPERIENCE)`).

---

## Part 4 — Context-sensitive control hints

### Problem

On-screen button labels are the Ambition protagonist's verbs, twice over:

1. The affordance table `PlayerAffordances`
   (`crates/ambition_actors/src/affordances/mod.rs:75`) is computed for the
   **`PrimaryPlayer`** (`compute_player_affordances`, `mod.rs:89`) with
   protagonist-specific inputs (portal gun, morph) — fine as far as it goes,
   and already the single source of truth ("HUD can never disagree with what
   fires").
2. The touch overlay's button SET is a static protagonist layout —
   `layout.rs:114-135` bakes "Blink", "Fly", "Shot", "Attack", "Dash", "Jump"
   buttons for every session, so a Mary-O run shows Blink/Fly/Shot buttons
   that do nothing.

Relativity principle: the prompt row is a function of the **active control
context** — the controlled body, its worn character's capability kit, and the
active UI focus — not a global constant.

### The seam

- Controlled body: `ControlledSubject`
  (`ambition_platformer_primitives::markers`, resolved in
  `crates/ambition_actors/src/control/queries.rs:64-69`) / `Brain::Player`
  (single control seam).
- Character identity + kit: `WornCharacter`
  (`crates/ambition_characters/src/actor/worn.rs:33`) and the catalog row's
  `abilities: Option<AbilityKitSpec>` → `ae::AbilitySet` (the engine bool set
  the body actually enforces). Demos flow through this same catalog
  (`AuthoredCatalogFragments::starting_character`,
  `crates/ambition_game_shell/src/experience.rs`).
- Contextual verb labels: the affordance variants + `glyph_for` /
  `ActiveInputMethod` (`devices.rs:76,142`) already carry label + device
  glyph.
- UI focus: `MenuFocusState` (`crates/ambition_ui_nav/src/pointer.rs`) and
  `GameMode` tell us when the active context is a MENU, not a body.

### The smallest elegant change: one read-model between producers and pixels

New module `crates/ambition_actors/src/affordances/hints.rs`:

```rust
/// The physical prompt slots a host can render (touch buttons, a desktop
/// hint row, a pause-menu footer). Mirrors action bindings, not verbs.
pub enum HintSlot { Jump, Attack, Shield, Dash, Special, Interact, Menu }

pub struct ControlHint {
    pub slot: HintSlot,
    pub label: Cow<'static, str>,   // "Jump", "Spin", "Talk", "Select"
    pub glyph: Cow<'static, str>,   // from glyph_for(ActiveInputMethod, slot)
    pub enabled: bool,              // slot exists for THIS control context
}

/// Frame read-model. Consumers render it; nobody else writes it.
#[derive(Resource, Default)]
pub struct ControlHintModel { pub hints: Vec<ControlHint>, pub context: HintContext }

pub enum HintContext { Body /* controlled subject */, Menu, Dialogue }
```

One producer system, `derive_control_hints`, after
`AffordancesSystemSet::Compute`, with an **explicit identity-ordered
arbitration** (arbitrate-by-identity rule — a priority match, not system-order
races):

1. **Menu focused** (`MenuFocusState` owner present, or launcher/pause menu
   open) → menu verbs: Navigate/Select/Back from the shell's neutral edges.
2. **Dialogue** (`GameMode::Dialogue`) → Advance/Choose.
3. **Body** → for the resolved controlled subject (via `ControlledSubject`,
   NOT `PrimaryPlayer` — one-line generalization of
   `compute_player_affordances`'s query filter): slot presence gated by the
   body's effective `AbilitySet` (`enabled = set.allows(slot)`), labels from
   the existing affordance variants (`VariantLabel::text()`), glyphs from
   `glyph_for`.

Consumer refits (each a thin diff, no new logic):

- **Touch overlay**: `update_button_verb_from_affordances`
  (`bevy_plugin.rs`) reads `ControlHintModel` instead of `PlayerAffordances`
  directly, and a sibling `sync_button_visibility_from_hints` hides buttons
  whose slot is `enabled: false`. The static `layout.rs` list stays (layout is
  geometry; the model decides presence/labels) — a Mary-O session then shows
  exactly Move/Jump/Menu.
- **Future desktop hint row** is just another consumer of the same resource —
  out of scope here, but the model is the hook the task asked for.

Per-character label overrides (e.g. Sanic's Jump reading "Spin") ride the
catalog: an optional `verb_labels: BTreeMap<HintSlot, String>` on
`CharacterCatalogEntry` (beside `abilities`, `entry.rs:375`), consulted by
`derive_control_hints` between the variant label and the glyph. Content stays
data; core stays named-content-free (agent-navigability north star). This
override map is a stretch slice — presence-gating by `AbilitySet` alone
already fixes the visible lie.

Why generic: the model's inputs are all engine vocabulary the demos already
have (controlled subject, ability set, input method, menu focus). Nothing
names Ambition; the protagonist just becomes the first character whose kit
happens to enable every slot.

---

## Design question — ability sets should COMPOSE, not enumerate

`AbilityKitSpec` (`crates/ambition_characters/src/actor/character_catalog/entry.rs:182`)
is a preset-picker: a catalog row names `Basic | SaneSubset | SandboxAll` and
hydrates to a fixed `ae::AbilitySet`. That's the right authoring ergonomics for
"one word in a RON row" but the wrong long-term algebra: it makes every new
loadout a new enum variant (a preset lattice that only grows), and it can't
express Ambition's actual progression model — the north star says upgrades are
theorems the player ACCUMULATES. The composition model I'd sketch: make
`AbilitySet` the **join-semilattice value it already secretly is** (a set of
verb capabilities, composition = union), and make everything that "has"
abilities a **grant source**: the character's base kit (catalog row), worn
equipment (`WornEquipment` already exists), collected upgrades/theorems, and
mount/possession transfers. The body's effective set is a derived read-model —
`Effective = (base ∪ gear ∪ upgrades) ∩ session_mask` — recomputed when any
grant changes, never mutated in place; the `session_mask` (meet) is how a
tutorial room or a demo restricts without destroying grants (Noether-room
gravity tutorials, Mary-O's classic kit). Presets then stop being primitives
and become **named grant bundles** — `Basic` is sugar for
`grants: [Run, JumpVariable, Reset]` — so `AbilityKitSpec` survives as a
serde-level convenience that lowers into the same grants list, and no code
matches on preset names. This also closes the loop with Part 4: control hints
and the movement kernel read the SAME effective set, so a picked-up theorem
instantly grows both the body's verbs and the prompt row — every upgrade a
theorem, every hint a corollary.

---

## Task cards

Order: **T1 → T2 → T3 → T4** (T1 unlocks the player-visible pause+quit loop;
T2 is independent polish that can also run first; T3 rides T1's menu; T4 is
separable). Estimates are focused-model wall-clock.

### T1 — `ShellPauseMenuPlugin` (pause + quit entries) — ~3 h
- `crates/ambition_game_shell/src/pause.rs` (new): resource, spec, 3 systems
  per Part 1 sketch; export from `lib.rs`.
- `crates/ambition_game_shell/src/input.rs`: add `pause` edge (Esc + Start);
  move Start OUT of `quit_to_home` (F10 stays).
- `crates/ambition_game_shell/src/experience.rs:54`:
  `ExperienceRegistration::with_own_pause_surface()` opt-out flag.
- `session.rs` `translate_shell_session_lifecycle`: reset
  `NextState<GameMode>` to `Playing` on activation; pause.rs force-closes on
  `Retiring`.
- Hosts: one `add_plugins(ShellPauseMenuPlugin)` line each in
  `compose_ambition_shell_host` (`shell_host.rs:79-83` block),
  `compose_smb1_shell`, the sanic app's equivalent, and the pocket demo host.
- Ambition arbitration: gate the grid menu's `PauseEntrySource::Pause`
  path (`grid_backend.rs:312-350`) on `direct_entry`; delete the System-face
  Quit row (`dispatch.rs:124-127`).
- Tests (minimal-plugin App pattern): toggle sets `GameMode::Paused` and a
  tick moves no `Transform` in a live demo session; confirm-on-QuitToTitle
  writes `ShellCommand::QuitToHome`; menu root teardown touches only
  `ShellPauseUiRoot` (poison: spawn a foreign `BevyUiMenuRoot` first and
  assert it survives); relaunch starts `Playing`.

### T2 — Presentation timing (fades + music-to-title) — ~2.5 h
- `sequence.rs`: `fade_in`/`fade_out` on `ShellSegmentPolicy` (default ZERO) +
  pure `segment_alpha` + unit tests (including fade-out anchored to
  auto-advance and the no-auto-advance case).
- `basic_presentation.rs`: `animate_basic_sequence_alpha`;
  launcher `TransitionShroud` + `animate_frontend_shroud`;
  `ShellLauncherPresentation::fade_in` field (`launcher.rs`).
- `frontend.rs:22`: `FrontendPresentationKind::TransitionShroud` variant.
- `session.rs:420-448`: home-route-only title track via
  `ShellHostConfiguration` + `FrontendAudioProfile::without_title_track`
  (`crates/ambition_audio/src/selection.rs:122`).
- `crates/ambition_actors/src/audio/plugin.rs:280`: same-track continuity
  guard before `silence_music_backend`.
- `shell_host.rs:146-152`: startup card policy (4 s / 600 ms / 900 ms).
- Tests: `segment_alpha` envelope; select-frontend for a NON-home route
  carries no preferred track while the home route does (poison-test the shape:
  assert the startup activation's authority `preferred_track` is None);
  continuity guard leaves `active_track` untouched across two home
  activations. Timing values ship as a "blind fix:" commit.

### T3 — Control-hint read-model + touch refit — ~3 h
- `crates/ambition_actors/src/affordances/hints.rs` (new): `HintSlot`,
  `ControlHint`, `ControlHintModel`, `derive_control_hints` (identity-ordered
  Menu > Dialogue > Body), registered in `AffordancesPlugin` after Compute.
- Generalize `compute_player_affordances` query (`mod.rs:89`) from
  `PrimaryPlayer` to the controlled-subject resolution used by
  `control/queries.rs` (keep `PrimaryPlayer` fallback).
- Touch refit (`crates/ambition_touch_input/src/bevy_plugin.rs`):
  `update_button_verb_from_affordances` reads the model;
  new `sync_button_visibility_from_hints` (Display::None for disabled slots).
- Tests: Basic-kit body yields `enabled:false` for Special/Shield/Dash slots;
  menu-open frames produce `HintContext::Menu` regardless of body state
  (pre-poison the model with body hints before the call); touch buttons for
  disabled slots are hidden in a Mary-O-kit session.
- Stretch (separate commit): `verb_labels` catalog field + lexicon lookup.

### T4 — (deferred, tracked only) ability grant composition
Not scheduled here — the Part-5 paragraph is the design seed. When picked up:
grants list on catalog rows lowering `AbilityKitSpec` to sugar, effective-set
read-model, session mask. Blocked on nothing; sequenced after T3 so hints
already read the effective set.

## Anti-god-structure note

Explicitly rejected shapes, so the executing model doesn't "helpfully" build
them:

- **No `ShellUxPlugin` umbrella** bundling pause/fades/hints — each is a small
  plugin/system in the crate that owns its seam; hosts compose lines, not a
  monolith.
- **No generic tweening framework** — `segment_alpha` is one pure function;
  the shroud is one component + one system. If a third fade consumer appears,
  THEN extract.
- **The pause menu does not absorb Ambition's grid menu** (settings,
  inventory, map stay app-side); it is three semantic rows. A "Settings" row
  that opens the game's own menu is a later, separate seam.
- **`ControlHintModel` is a read-model**: consumers render it, nothing
  downstream writes it, and no gameplay system reads hints to decide behavior
  (the affordance resolvers remain the shared truth). No state accumulates on
  it (moveset-dedup lesson).
- **`FrontendAudioProfile` stays a profile**, not a music director — the
  home-route rule lives in the shell bridge, which already owns
  route-to-authority translation.

— Fable (claude-fable-5), 2026-07-15
