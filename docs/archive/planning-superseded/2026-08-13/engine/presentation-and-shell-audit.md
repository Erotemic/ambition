# Presentation and game-shell completeness audit

**Task 10** of [`competitive-2d-platformer-engine-roadmap.md`](competitive-2d-platformer-engine-roadmap.md).
Written 2026-07-27.

**Exit criterion:** *"every reviewed domain has an explicit ruling: use Bevy core,
use a selected plugin, retain a justified Ambition contract, refactor duplication,
or defer because no current customer requires it."*

The criterion is about COVERAGE, which is why it needs a document and not a set of
good decisions scattered across crate docs. Individually correct rulings already
existed everywhere; what did not exist was a list that can say whether a domain
was skipped. This is that list.

**Method.** For each domain: what does the crate actually contain, what does Bevy
or a selected plugin already supply, and what is left that is genuinely
platformer- or Ambition-specific. Sizes are non-comment source lines under
`src/`, counted 2026-07-27, and are a rough scale marker only — a big number is a
question, not a verdict.

⚠ **This audit was written against the code, not against crate names.** The same
sweep that produced it twice reported a capability absent because the type was
named something else (see D15 in the 24h queue). Every "defer" below names what
would have to exist for the answer to change.

---

## Ruling vocabulary

| Ruling | Means |
| --- | --- |
| **Bevy core** | Bevy already supplies this; Ambition adds nothing and should not. |
| **Selected plugin** | An ecosystem crate supplies it and is in the dependency graph. |
| **Ambition contract** | Retained deliberately: it encodes platformer or determinism semantics Bevy has no opinion about. |
| **Refactor** | Real duplication or leakage; a named follow-up. |
| **Defer** | No current customer. The trigger that would change the answer is named. |

---

## 1. Rendering — `ambition_render` (10.7k lines)

**Ruling: Ambition contract, with one refactor.**

Bevy supplies the renderer, sprites, atlases, cameras and text; none of that is
reimplemented. What this crate holds is *policy over* those primitives, and it is
policy Bevy cannot have an opinion about:

- **presentation reads a read model, never sim components** — `platformer_presentation`
  and `rendering/` consume `SimView` facts by id. That rule is the reason a
  rollback resimulation cannot be corrupted by a renderer, and it is the single
  most load-bearing contract in the crate.
- **platformer sorting, parallax, nameplates, hit flash, surround** — layering and
  framing decisions with gameplay meaning.
- **`asset_census`** — always-on decode reporting. Named an Ambition contract
  rather than diagnostics because it is how the 627MP/2.5GB boot decode was found;
  see Task 12.

**Refactor (already ruled, 2026-07-16 decision §1, not yet complete):** named-content
modules inside the renderer become content-owned presentation plugins on a public
render seam, the `portal_presentation` pattern. Any module in `rendering/` named
after a specific piece of content is on that list.

## 2. VFX — `ambition_vfx` (504 lines)

**Ruling: Ambition contract. Small and correctly small.**

A semantic cue vocabulary (`VfxMessage::Burst`, slash arcs and poses) rather than a
particle system: content names WHAT happened, presentation decides what it looks
like. This is the same shape as the SFX seam and for the same reason — a cue
emitted during a predicted frame must be deferrable, which a direct spawn is not.

Bevy has no particle system in core, and a plugin (`bevy_hanabi` and friends) would
sit *underneath* this vocabulary rather than replace it. Not needed at this scale.

## 3. Audio — `ambition_audio` (5.2k), `ambition_sfx` (898), `ambition_sfx_bank` (470)

**Ruling: selected plugin for output, Ambition contract for everything above it.**

`bevy_kira_audio` is the mixer. Above it:

- **`ambition_sfx`** — the emission vocabulary. Deliberately Bevy-light (`bevy_ecs`
  + `bevy_math` only, no full Bevy) so the sim can emit without depending on a
  renderer. Owns the three attribution cases: body-owned, session-global, and
  provider-owned (`write_for` / `write_global` / `write_from`).
- **`ambition_sfx_bank`** — no Bevy at all. A cue table and a synth spec.
- **`ambition_audio`** — provider-relative selection and authorization
  (`ActiveAudioSelection`), the music registry, and the confirmed-frame boundary.

The confirmed-frame half is the part no mixer can supply: a sound emitted during a
predicted frame must be deferred rather than suppressed, because suppressing at
emit time destroys the corrected sound before anything can decide whether the
prediction it replaces was ever heard.

**Open, and tracked:** `ActiveAudioSelection::authorize_sfx_source` panics on
re-authorization with a different definition and has no revoke (A15 in the 24h
queue).

## 4. Game shell — `ambition_game_shell` (5.4k lines)

**Ruling: Ambition contract. No Bevy or ecosystem equivalent exists.**

Routes, launcher, session preparation/activation lifecycle, pause semantics,
startup sequences. Bevy has `States`, which this uses; everything else here is the
multi-game host, and a multi-game host is the thing the engine is FOR. The
external-consumer fixture assembles a complete game through this surface with no
engine edits, which is the strongest available evidence that the shape is right.

**Explicitly ruled and not revisited** (2026-07-16 decision §5): provider
registration stays two explicit lines in `ambition_app`. No plugin discovery.

## 5. Menus — `ambition_menu` (2.2k), `ambition_settings_menu` (1.9k), `ambition_menu_kaleidoscope`

**Ruling: selected plugin for presentation, Ambition contract for the model.**

`bevy_lunex` and `bevy_material_ui` are the selected UI plugins, both feature-gated.
`ambition_settings_menu` depends on NO external crate at all — it is a pure model
(pages, entries, actions), which is the right split: the model is testable
headlessly and the backend is swappable, and there are already two backends (grid
and kaleidoscope) proving that.

**Refactor candidate, low priority:** two menu backends is one more than a shipped
game needs. Not urgent — the second exists to keep the model honest.

## 6. UI navigation — `ambition_ui_nav` (656 lines)

**Ruling: Ambition contract.**

Directional focus movement over a declared grid. Bevy 0.18 has no focus-navigation
core, and the alternative — every menu implementing its own arrow-key handling — is
what this replaced.

**Revisit trigger:** Bevy's `bevy_input_focus` maturing into directional navigation.

## 7. Dialogue — `ambition_dialog` (1.8k lines)

**Ruling: selected plugin.**

`bevy_yarnspinner` runs the dialogue; this crate is the binding layer (visit
counters, the save-backed `visit_count` function, the runner's interface to
gameplay state). Correct shape and small for what it does.

## 8. Inventory UI — `ambition_inventory_ui` (115 lines)

**Ruling: Ambition contract, trivially.**

115 lines. A read model for the inventory surface. Nothing to consolidate.

## 9. Settings and persistence — `ambition_persistence` (3.8k lines)

**Ruling: Ambition contract.**

Save I/O, migration, the autosave confirmation gate, user settings, platform data
paths. The confirmation gate is the Ambition-specific part: a rollback host must
not write a predicted world to disk, because unlike a sound — heard once and wrong
— a save file outlives the session that produced it.

Bevy has no save system. `serde` + `ron` are the selected serializers.

## 10. Diagnostics and dev tools — `ambition_dev_tools` (1.6k lines)

**Ruling: Bevy core plus a thin Ambition contract.**

`bevy_diagnostic` supplies the frame/system diagnostics; `bevy_egui` (app-gated)
supplies the inspector. What is Ambition's is the always-on census reporting
(`[schedule-census]`, `[frame-spike]`, `[image]`) that prints on every boot and the
developer-tools settings model.

**Gap, tracked as D13:** these numbers are MEASURED and never enforced. Nothing
fails when one regresses.

## 11. Load presentation — `ambition_load_presentation` (1.9k lines)

**Ruling: Ambition contract.**

Loading surfaces for room transitions and shell routes, contributor-neutral so a
provider gets one without writing one. Tied to the load-plan transaction, which is
Ambition's.

✔ **Composition hazard closed 2026-07-27** (was a Phase-6 leak): whether the
engine group or the host owed `AmbitionLoadPlugin` was an undocumented rule
enforced by a hard Bevy panic. The plugin is idempotent now — a host may add it,
omit it, or add it twice — and Outlander adds it itself as the proof.

## 12. Localization — **no crate**

**Ruling: DEFER. No customer.**

There is no i18n dependency (`fluent`, `gettext`, anything) and no string table:
every user-facing string is a Rust literal or authored content. Deferred because no
shipped game targets a second language.

**Trigger that changes this:** the first non-English target. It is a large change —
every literal becomes a key — and it gets much more expensive per month of content
authored, so the decision to defer should be re-examined deliberately rather than
by drift.

## 13. Accessibility — settings model only

**Ruling: PARTIAL — retained contract, with named gaps.**

What exists is real and shipped: `Colorblind` mode, `Flashes` intensity (with a
multiplier that clamps), screen-shader strength, camera zoom and framing presets,
`ShowFps`. Flash intensity in particular is a genuine photosensitivity control, not
a cosmetic one.

**Named gaps, none with a customer yet:** no input remapping surfaced in the
settings menu (bindings exist in `ambition_input`; the menu does not expose them),
no text scaling, no screen-reader or `AccessKit` integration (Bevy has
`bevy_a11y`; nothing uses it), no subtitle track for the audio cues.

**Trigger:** a shipping target with an accessibility requirement, or a platform
certification that mandates remapping — the most likely first one.

---

## Summary

| Domain | Ruling |
| --- | --- |
| Rendering | Ambition contract + one named refactor |
| VFX | Ambition contract |
| Audio | Selected plugin (kira) + Ambition contract |
| Game shell | Ambition contract |
| Menus | Selected plugins + Ambition model |
| UI navigation | Ambition contract (revisit on `bevy_input_focus`) |
| Dialogue | Selected plugin (yarnspinner) |
| Inventory UI | Ambition contract (trivial) |
| Settings / persistence | Ambition contract |
| Diagnostics | Bevy core + thin contract; **enforcement gap (D13)** |
| Load presentation | Ambition contract; **composition hazard recorded** |
| Localization | **Defer** — no customer, trigger named |
| Accessibility | **Partial** — real controls shipped, gaps named |

Thirteen domains, thirteen rulings, and the two that are not "done" say what would
have to be true to change them. Nothing here authorizes building a renderer, a
mixer, or a UI framework — the audit's own instruction, and the finding is that
nobody has.
