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
