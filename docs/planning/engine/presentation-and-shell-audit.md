# Presentation and shell — remaining capability gaps

> **Verified against `cecd01ca` (2026-08-13).** The original thirteen-domain
> presentation/shell audit is closed for rendering, VFX, audio, shell, menus,
> UI navigation, dialogue, inventory UI, settings/persistence, diagnostics and
> load presentation. Its full evidence is archived at
> [`../../archive/planning-superseded/2026-08-13/engine/presentation-and-shell-audit.md`](../../archive/planning-superseded/2026-08-13/engine/presentation-and-shell-audit.md).

Only the capability gaps below remain useful as forward planning.

## Localization — trigger-based

There is still no translation catalog/runtime locale system. Some engine-facing
vocabularies already carry stable ids or reserved i18n keys, but user-facing
presentation is not resolved through a locale-aware message catalog.

**Trigger:** the first non-English shipping target or another concrete consumer
of translated UI/dialogue.

When triggered:

- introduce stable message identity and locale selection at the presentation
  boundary rather than making simulation/content identity depend on display text;
- migrate user-facing literals incrementally through the one translation path;
- keep authored IDs stable and language-independent;
- provide missing-key diagnostics that identify provider/source provenance.

Do not build an i18n framework merely because the old audit named the absence.

## Accessibility — remaining product gaps

Several controls from the old audit have landed, including player-facing input
rebinding. Existing video/gameplay settings also cover flash intensity,
colorblind mode selection, camera/framing controls and damage assist.

The remaining concrete gaps are:

- ▢ make the colorblind setting affect a real presentation/palette transform;
- ▢ add user-controlled text/UI scaling if a shipping target needs it;
- ▢ add screen-reader / Bevy accessibility-tree integration when a target needs
  non-visual menu navigation;
- ▢ add caption/subtitle presentation for non-dialogue audio cues when required.

Treat each as a capability with a real consumer and acceptance case; do not grow
a parallel UI stack to implement it.
