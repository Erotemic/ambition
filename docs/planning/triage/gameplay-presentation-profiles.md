# Gameplay presentation profiles — remaining work

> **Verified against `cecd01ca` (2026-08-13).** GP1–GP5 are implemented: profile
> resolution, fixed/aspect viewport policy, surround layout, provider profile
> declaration, occupancy/control regions, touch placement, and the player HUD's
> first surround-region consumer exist. The original design/review history is
> archived at
> [`../../archive/planning-superseded/2026-08-13/triage/gameplay-presentation-profiles.md`](../../archive/planning-superseded/2026-08-13/triage/gameplay-presentation-profiles.md).

## Remaining

- ▢ **Bridge real platform safe-area insets.** `DisplaySafeAreaInsets` exists and
  is consumed, but the runtime still lacks a production writer that publishes
  non-zero platform insets where appropriate.
  ✔ **RE-MEASURED 2026-09-03 — accurate, and it is a total absence rather than a
  partial one.** The type has exactly FOUR references in the entire repository:
  the definition
  (`crates/ambition_platformer2d_shared_tangle/src/gameplay_presentation/mod.rs:262`),
  an import, one `init_resource`
  (`crates/ambition_platformer2d_host/src/gameplay_presentation.rs:62`) and one
  `Res` read (`:235`). No `ResMut`, no `insert_resource` with a value, anywhere.
  ⇒ **The resource is `Default` — zero on every edge — for the entire life of
  every process**, and the reader is not a leaf: it is
  `resolve_host_gameplay_presentation`, which feeds `safe_area_insets` into the
  whole layout resolve. So the safe-area path is wired end to end except for its
  producer, and every layout the game has ever computed used zero insets.
  ⚠ The consequence is device-shaped and cannot be seen here: on a display with
  a notch, a cutout or rounded corners, gameplay is laid out into the unsafe
  region and looks correct on every desktop and in every headless test. Same
  family as the Android font path in
  [`../../recipes/checks-that-did-not-run.md`](../../recipes/checks-that-did-not-run.md)
  — closing it needs a device, not a build.

- ▢ **Finish overlap fallbacks only as real cases demand them.** The current
  layout can reserve control/HUD regions; remaining escalation steps are
  repositioning contextual controls, fading presentation near the controlled
  subject when necessary, and strengthening silhouette/readability. Implement
  from observed overlap cases, not as a speculative framework.

- ▢ **Participant-facing layout preference.** Add an actual user-facing profile
  or layout preference only when the settings/UI owner is clear.

- ▢ **Authored surround art.** `GameAuthored` and
  `DecorativeWorldExtension` modes still need a content path that can supply the
  authored surround rather than only the base fill.

- ▢ **Move remaining overlays onto computed layout where useful.** Quest, debug,
  map, dialogue, and other HUD/menu surfaces should consume resolved regions when
  they materially compete with gameplay/control space. Do not build one giant
  responsive-HUD manager.
