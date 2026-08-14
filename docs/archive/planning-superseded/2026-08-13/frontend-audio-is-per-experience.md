> ## ✅ LANDED 2026-08-07 — keyed by ROUTE
> `FrontendAudioProfile` is no longer a `Resource`; declarations live in
> `FrontendAudioRegistry` (`ambition_audio::selection`), keyed by route, with one
> host default as the fallback. Smash's character-select score now plays in the
> Ambition host, which is the case that could not be expressed before.
> **What this plan got wrong, and the outcome, are recorded at the bottom.**

# Frontend audio belongs to an experience, not to the process

Jon, 2026-08-07: *"Being able to use the same resource in different hosts is
unacceptable. … This current design is not elegant if games cant share
assets."*

## What prompted it

Smash got three scores written for it, one of them
`super_smash_siblings_character_select` — the character-select theme. It plays
in the standalone `ambition_demo_smash_app` and **it cannot play in the Ambition
host**, where the same select screen is the same route drawn by the same plugin.

That is not a missing feature. It is the shape of one type.

## The defect, exactly

`FrontendAudioProfile` is a `Resource` installed with `app.insert_resource` by
`ShellComposition::install`
(`crates/ambition_platformer2d_provider/src/composition.rs:138`) and again by the
multi-game host (`game/ambition_app/src/app/shell_host.rs:129`). One value per
**process**. The last composition to install wins.

So it is not "the frontend audio of an experience". It is "the frontend audio of
whoever booted", and every provider composed into a host shares it.

⭐ **The design already documents its own bug.** `ShellComposition::install`'s
comment says an app that skips the insert *"plays whichever provider's frontend
audio was cached last, which on a first run is nothing and after a route change
is somebody else's music."* That sentence describes a singleton being read as if
it were scoped.

⭐⭐ **And the line where it goes wrong is one line.**
`apply_audio_context_for_shell_lifecycle`
(`crates/ambition_game_shell/src/session.rs:444-472`) handles a frontend route:

```rust
ShellEvent::RouteActivated(activation)
    if !registry.contains(&activation.experience_id) =>
{
    let owner = AudioContextOwner::Frontend(activation.activation_id.0);
    if let Some(frontend) = frontend.as_deref() {          // ← the singleton
        let provider = frontend.provider_id();
```

The **owner** is taken from the activation. The **provider** is taken from a
global. One of those two facts is per-route and the other is per-process, and
they are used together in the same three statements. `activation.experience_id`
is right there, unread.

**The gameplay half solved this and the frontend half has nowhere to put the
answer.** `translate_shell_session_lifecycle` (`session.rs:567`) resolves the
provider ONCE, from the experience, at activation —
`registry.profile(&activation.experience_id).audio_provider`, defaulting to the
experience id — and stamps it onto the `GameplaySessionInstance`. The audio
function then reads it off the live session and scopes emission to
`AudioContextOwner::Gameplay(scope)`.

⭐ **A frontend route has no instance.** There is no per-activation record to
stamp, so the code reaches for the only thing in scope: a process global. That is
the actual root — not carelessness, an absence. Give frontend activations a place
to look up their provider and the singleton has no job left.

**Frontend audio is the half of a seam that never made the trip.**

## The class of bug this is

The same one that produced three defects in this repo on 2026-08-07 alone: a
fact about *one thing* stored where only one can exist.

- `GameMode` — a pause is a fact about a session, stored per process, so quitting
  to the title left the world stopped.
- `ClockState` / `RequestedClockScale` — a clock zeroed by a pause with nobody
  scoped to raise it again.
- `FrontendAudioProfile` — a provider's frontend sound, stored per process, so a
  host with two providers can only honour one.

⚠ so this is not a polish item. The engine's stated direction is that a provider
travels: compose it into any host and it brings its content, its cast, its rules.
Frontend audio is currently the one thing that does not travel with it.

## Target design

**Frontend audio is keyed by experience, exactly the way gameplay audio already
is, and the ROUTE selects it.**

```text
provider registers            AudioCatalogFragment(experience) …………… already true
                              FrontendAudioProfile(experience) ……………… NEW: keyed
route activates (frontend)    experience_id → profile → provider …… the lookup
                              AudioContextOwner::Frontend(activation) … already true
```

Three properties to hold it to:

1. **A provider declares its own frontend sound**, in its own registration,
   beside the audio fragment it already declares. Smash's select score ships with
   smash and plays wherever smash is composed.
2. **A host may still declare its own**, for the routes the HOST owns — its title
   screen, its launcher, its loading screen. That is a real and different thing
   and it keeps working.
3. **Nothing is keyed by "whoever installed last".** Resolution is a lookup on
   `activation.experience_id`, with the host's own profile as the answer for the
   host's own routes.

⚠ **`AudioContextOwner::Frontend(u64)` does not change.** The owner is the
activation and always was; only the provider resolution moves. Emission
ownership, staleness and the `AudioContextChanged` bookkeeping are correct
today and this must not disturb them.

## Phases

### Phase 0 — the probe, first and red

`game/ambition_app/tests/` — a host composing **two** providers, routed to a
frontend experience owned by the second one, asserting the selected music
authority is the SECOND provider's track.

Today that reports the first provider's (or the host's), which is the bug. The
existing `shell_host_lifecycle.rs:168` already reasons about
`FrontendAudioProfile::title_track` feeding the music authority, so the oracle is
established; what is new is that two providers exist and the answer must depend
on the route.

⚠ assert the **selected authority**, not the resource's contents. A test that
reads the profile back out passes on a singleton.

### Phase 1 — the registry

`FrontendAudioProfile` stops being a `Resource` in its own right and becomes an
entry in a registry keyed by `ShellExperienceId`, beside `AudioCatalogRegistry`
which is already keyed that way.

- `crates/ambition_audio/src/selection.rs` — the profile type keeps its shape
  (`provider_id`, `title_track`, `sfx_ids`); a `FrontendAudioRegistry` holds them
  by experience with one optional host default.
- The host default is what `shell_host.rs:129` installs today, and it stays the
  answer for the host's own frontend routes.

### Phase 2 — the lookup

`apply_audio_context_for_shell_lifecycle` (`session.rs:444`) resolves the profile
from `activation.experience_id`, falling back to the host default.

**This is the whole behavioural change.** The assert that follows
(`"frontend audio provider '{provider}' registered no audio fragment"`) becomes
more useful, not less: it now names the experience whose declaration is wrong.

### Phase 3 — the declaration sites

- `ShellComposition::with_frontend_audio` registers under the experience it
  already knows (`self.experience_id`) rather than inserting a global. Its
  callers (`ambition_demo_sanic_app`, `ambition_demo_smash_app`,
  `ambition_demo_pocket`) do not change shape.
- `ambition_app`'s host registers its default the same way.
- **Smash's select experience declares its own** — `SMASH_SELECT_EXPERIENCE` with
  `SMASH_SELECT_TRACK` — which is the thing that could not be said before, and
  the acceptance case.

### Phase 4 — the audit the singleton was hiding

`grep` for every reader of the resource and classify: host-frontend policy
(stays), provider-frontend policy (moves), test fixture (follow the production
shape). Known sites: `composition.rs:138,231,242,247`, `shell_host.rs:129`,
`session.rs:392`, `shell_host_rendered.rs:392`, and two demo `rollback_restore`
fixtures.

⚠ this is the step that has been skipped before on this kind of change. A
singleton has readers that do not look like readers.

## Verification

- **Phase 0 probe** red before, green after — that is the acceptance oracle.
- `ambition_app`'s `shell_host_lifecycle` and `shell_host_rendered` must stay
  green: they pin the host's OWN frontend audio, which must not move.
- The standalone smash demo must still play its select score — the case that
  works today and must not regress while the mechanism changes underneath it.
- **Listen to it**, or at least assert the selected track by name at both routes
  in the Ambition host. This is presentation, and presentation in this repo fails
  silently.
- Gate: `cargo check -p ambition_app`, then the app suite and
  `-p ambition_game_shell`.

## Not in scope

- **Per-route music WITHIN one experience** (a different track for the stage and
  for the winner card). That is a music-selection question and this change is
  about ownership. Once the registry is keyed, it becomes expressible; it is not
  this plan.
- **The `AudioContextOwner` enum.** It is right.
- **Asset sharing.** Already fine and worth separating from the ownership
  problem: music assets live in one tree, fragments name tracks by id, and 76
  cues are shared by every provider today. What is not shareable is the
  ownership ANSWER.

## Risks

- ⚠ **The frontend profile is read during route activation, which is ordering-
  sensitive.** A provider that registers its profile after the shell has already
  activated its first route gets silence on that route only. Registration happens
  at plugin build and activation at least a frame later, so this should hold —
  but it is the thing to check first if a route comes up silent.
- ⚠ **`selection.clear()` when no profile is found** is the current behaviour for
  a host with no profile at all, and it must stay reachable: a frontend route
  belonging to a provider that declares no frontend sound is SILENT, not
  inheriting somebody else's.
- Two demo `rollback_restore` fixtures insert the resource directly. They are
  asserting something else entirely and should take the least interesting path
  through the new API.

## DECIDED: per route (Jon, 2026-08-07)

Keyed by `ShellRouteId`, not `ShellExperienceId`. Everything above holds with the
key one notch narrower, and it is genuinely one notch: `ActiveShellExperience`
carries **both** (`router.rs:117-119`), so the lookup reads `activation.route_id`
and nothing else moves.

⭐ **and per-route is the more honest key anyway**, which the measurement below
makes plain: one experience is already reached by five different routes, so
"per experience" would have meant five screens sharing one answer — a smaller
version of the same singleton this plan exists to delete.

The host default stays the fallback for a route that declares nothing.

---

# Appendix: the vocabulary, because it does not mean what it looks like

Jon, 2026-08-07: *"I'm not sure I like the terms experience and route. The idea
is experience is the game itself, and route is a some screen or menu or system in
that game?"*

**That is not what the code means, and the gap is worth naming before the rename
is decided.** Measured 2026-08-07:

| term | what it actually is | evidence |
|---|---|---|
| **provider** | who AUTHORED the content | `AMBITION_CONTENT_PROVIDER`; audio fragments, characters and SFX are all keyed by it |
| **experience** | a runnable KIND the shell knows how to start | `BASIC_LAUNCHER_EXPERIENCE` is ONE experience reached by FIVE routes — Ambition's launcher, Sanic's, Mary-O's, the provider composition's, and a test's |
| **route** | a navigable ADDRESS that names an experience | `SMASH_SELECT_ROUTE` → `SMASH_SELECT_EXPERIENCE` |

So the three facts are: **who wrote it**, **what kind of thing it is**, and
**how you get there**. Under Jon's reading there are two, and "experience" is
carrying the first one.

Two concrete places the mismatch already shows:

* **Smash-the-game is not one experience.** It is two (`smash`, `smash.select`)
  plus a provider. A person asking "what is smash's frontend audio" is asking a
  question the vocabulary cannot phrase.
* **Provider and experience are the same string in most call sites.** Smash's
  audio fragment registers under `SMASH_EXPERIENCE`, and
  `translate_shell_session_lifecycle` (`session.rs:570`) even defaults the audio
  provider to `activation.experience_id` when a profile names none. They are
  distinct concepts that happen to agree today, which is exactly how a
  distinction gets quietly lost.

## ⛔ NO RENAME — decided 2026-08-07

Jon: *"We don't need to do the rename. Having a finer grained vocabulary is
probably the right way to do it. We want to find the path that is maximally
elegant. Let's not churn for now."*

⚠ **and the recommendation this section used to carry was WRONG**, which is why
it is written down rather than deleted. The draft advised `experience` →
`surface`, on the grounds that "experience" reads like "the game". Checking what
other engines do reversed it:

* **Unity and Godot have no word for the middle concept at all**, because they do
  not separate it — a Scene is both the kind and the instance. Having the concept
  IS the finer-grained position.
* **Unreal separates it, and Epic's Lyra sample calls it an `Experience`** —
  `ULyraExperienceDefinition`, a data asset defining what a session consists of,
  with each level naming the experience it wants. That is our meaning, under our
  word, arrived at independently.

Renaming away from the only prior art's term, on taste, would have been churn in
exactly the wrong direction.

Two things ARE recorded as weak, and neither is being acted on now:

* **`provider` is the poor term** — the one a player would never say, the one
  carrying the meaning "experience" was expected to have. Unreal calls it a *Game
  Feature*, Unity a *Package*.
* **Ambition uses the same STRING for provider and experience at most call
  sites.** Smash registers its audio fragment under `SMASH_EXPERIENCE`, and
  `session.rs:570` defaults the audio provider to `activation.experience_id`.
  Two distinct concepts that happen to agree today. Unreal keeps them as separate
  types that cannot be confused.

**Full comparison, with citations that were fetched rather than remembered:**
[`docs/related-work/shell-vocabulary-in-other-engines.md`](../related-work/shell-vocabulary-in-other-engines.md).

⭐ **None of this blocks the audio change.** It lands keyed by route whatever the
words are.

---

# Outcome, 2026-08-07

## What landed

| piece | where |
|---|---|
| `FrontendAudioProfile` loses `#[derive(Resource)]` | `ambition_audio/src/selection.rs` |
| `FrontendAudioRegistry` — declarations by route, one host default, plus the profile in effect | same |
| `FrontendAudioAppExt::{declare_route_frontend_audio, set_host_frontend_audio}` | same |
| the lookup on `activation.route_id` | `ambition_game_shell/src/session.rs` |
| smash's select route declares its own score, beside the route | `ambition_demo_smash/src/lib.rs` |
| the host default | `ambition_app/src/app/shell_host.rs`, `ShellComposition::install` |

Dropping the `Resource` derive is what made the audit exhaustive: **every reader
became a compile error**, including the two in the actor monolith's audio plugin
that the plan listed only as "known sites". Phase 4's warning — *"a singleton has
readers that do not look like readers"* — was right, and the way to answer it was
to let the compiler enumerate them rather than to grep.

## ⛔ Where this plan was WRONG

**The plan never mentioned `title_theme_keeps_playing`, and that predicate is the
one thing per-route keying could plausibly have broken.** Its doc block states the
property it relies on in as many words: the profile *"is composed once for the
whole host and does not blink out between two of its screens."* That is a
documented dependency on the singleton, written by the fix for a real audible
defect (Jon, 2026-08-03: the title music restarting on the startup→launcher
handoff). A plan that deletes a property some other code names as load-bearing
should say so; this one did not, because the search that produced it looked for
the TYPE and that call site reads a field off it.

Two further corrections, both from measuring rather than reasoning:

* **The continuity guard already existed.** `shell_host_startup::the_title_music_survives_the_handoff_from_the_cards_to_the_launcher`
  asserts exactly the property at risk, using `play_generation`. It was nearly
  written a second time, because it never names the predicate it defends — a
  grep for `title_theme` does not find it. *Ask whether a test asserts the
  property elsewhere before writing one.*
* **The gap I designed around does not exist.** `FrontendAudioRegistry::in_effect`
  is not cleared on deactivation, on the theory that a frontend→frontend handoff
  passes through a state with no route. Probed by force-clearing it and
  re-running: the guard stayed green, and a direct measurement showed
  `in_effect` absent on **0 of 24 frames** and the audio owner absent on 0 as
  well — `select_shell_audio_context` drains deactivation and activation in ONE
  run, so no reader observes anything between them. The field is kept as a cheap
  precaution and **its doc says explicitly that no test backs it**, rather than
  claiming the guard proves it.

## Verification as run

* `shell_host_rendered::a_providers_own_frontend_route_plays_the_score_written_for_it`
  — the acceptance oracle. Red before (`something_worth_building` played on
  smash's select route), green after. Asserts the base-channel PLAYBACK at both
  routes, so it cannot pass on a resource read back out of its own registry.
* `the_screen_decides::the_select_screen_plays_its_own_score_in_the_standalone_demo`
  — new. The half that already worked, so that making the score travel could not
  quietly pay for it by dropping it where it started.
* `composition::a_route_that_declares_its_own_sound_is_not_overruled_by_the_default`
  — new. Precedence, asserted where it is actually decidable.
* Green: the full `app_it` suite (318), `ambition_platformer2d_actor_monolith`
  (1196), `ambition_game_shell`, `ambition_audio`, `ambition_platformer2d_provider`,
  the four demo apps, 25/25 absence contracts, and `./run_tests.sh` 7/8.
* ⚠ the one red is **pre-existing and unrelated**:
  `engine.character-authority-is-app-local` vs
  `actor_monolith/src/conversation/rules.rs:65`.

## Still open

* **Per-route music WITHIN one experience** is now expressible and still not
  done — a stage theme and a winner-card theme are two routes' declarations.
  Deliberately out of scope; the key was the blocker and the key is gone.
* **`provider` and `experience` are still the same STRING at most call sites**,
  and `session.rs` still defaults the audio provider to `activation.experience_id`.
  Recorded in [`docs/related-work/shell-vocabulary-in-other-engines.md`](../related-work/shell-vocabulary-in-other-engines.md);
  no rename, by Jon's call.
