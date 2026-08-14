# Combat model — remaining engine work

> **Verified against `cecd01ca` (2026-08-13).** CM1–CM5, CM7 and CM8 from the
> original combat campaign are implemented. The full design/execution record is
> archived at
> [`../../archive/planning-superseded/2026-08-13/engine/combat-model.md`](../../archive/planning-superseded/2026-08-13/engine/combat-model.md).
>
> Current body-generic platform-fighter integration is owned by
> [`../smash-body-generic-combat-2026-08-09.md`](../smash-body-generic-combat-2026-08-09.md).

## Already landed — do not schedule again

The engine already has the shared damage/knockback axis, DI, smash-charge data,
attack hold/release gesture signals, cancel windows/chains, per-move
presentation validation, derived frame data, body-generic hit feedback, a shared
victim-side shield/parry seam, equipment modifiers/grants, touch-to-collect
`WorldItem` equipment, and Mary-O consumers of the equipment model.

## Remaining combat capabilities

### 1. Grab / hold / throw vocabulary

Platform-fighter grabs and throws are still absent as a body-generic combat
capability. Design them through existing control-authority/body seams rather than
creating a Smash-only grabbed-body state machine. A successful throw must feed
the ordinary damage/launch/DI pipeline and release the temporary hold authority.

This is **deferred behind the current D72 feel/body-generic campaign**; do not
rush it merely because the old CM6 sketch exists.

### 2. Shield-stun / shield durability, if the product still wants it

Current source has `BodyShieldState { active, parry_window_timer }`, directional
blocking, parry behavior, and shared damage resolution. The older CM6 proposal's
shield HP, shield stun, break stun, regeneration, and `OnBlock` cancel fact are
not implemented as that model.

Treat those as an optional next capability to justify from Smash feel. Extend the
one body shield/damage authority; do not introduce a second shield subsystem.

### 3. Finish body-scale equipment resolution

`ambition_characters::equipment::resolved_param` supports `BODY_SCALE`, and the
pickup/equipment path is live, but current actor/render/collision body-size reads
do not generally resolve `BODY_SCALE` through that fold. If equipment-authored
body scaling remains desired, make the authoritative size/collider/presentation
read path consume the resolved value once rather than adding demo-specific size
patches.

### 4. Content-level hurt identity

The engine can author attack strike cues and victim `HurtFeedback`; distinct
shipped hurt identities remain content work where characters need them. This is
not a new combat architecture campaign.

## Exit

New combat work is done when the capability is body-generic, driven through the
same control and victim-resolution seams for human/AI/possessed bodies, and does
not add mode-specific duplicate authority.
