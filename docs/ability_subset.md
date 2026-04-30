# Sane maximalist subset

Ambition will eventually have a wide optional ability vocabulary, but the first
sandbox should stay focused enough that each verb can be tested and tuned.

The current "sane subset" is:

- core movement: run, variable jump, double jump, fast fall
- wall movement: wall jump, wall cling, wall climb
- burst movement: dash, double dash
- special movement: blink, precision blink
- combat/platforming interaction: primary attack, directional primary, pogo
- world interaction: rebound surfaces

Every item is represented as a flag on `ambition_engine::AbilitySet`. The
sandbox enables the full endgame set by default with `AbilitySet::sandbox_all()`,
but tests and later story states can construct smaller ability sets.

## Blink

Blink is the first concrete "direction + special" verb.

- Tap/release blink quickly to teleport a short distance in the input direction,
  or facing direction if no direction is held.
- Hold blink long enough to enter precision aim mode.
- Precision aim mode is intended to run in bullet time in the Bevy sandbox.
- Releasing while precision aiming teleports farther and with finer destination
  control.
- Blink clamps against solid geometry by sampling along the blink segment and
  returning the last safe player AABB.

Current default keyboard bindings use the generic secondary/special key:

- classic action preset: `A`
- WASD custom preset: `L`
- QWER chirality preset: `R`
- UIPO chirality preset: `O`

## Direction + action/special grammar

The engine now reserves ability flags for:

- `directional_primary`
- `directional_special`

The first implementation only uses blink as a directional special and keeps
primary attacks mostly as the existing slash/pogo hitboxes. Future work can add
neutral/forward/up/down variants without changing the top-level input model.

## Testing implications

Blink is deliberately implemented in `ambition_engine` rather than Bevy so it can
be tested headlessly. Tests should cover:

- ability gating: blink disabled means no teleport;
- quick release: short blink changes position and emits a `BlinkEvent`;
- hold past ~0.1s: player enters `blink_aiming`, release emits a precision blink;
- safety: blink destination does not overlap solid geometry;
- compatibility: precision blink warns if blink is disabled.

## 2026-04 precision blink / fast-fall adjustment

Current sandbox behavior:

- Blink is still part of the sane subset.
- Quick blink remains a tap/release movement verb.
- Long-hold blink enters a gradual, very deep bullet-time ramp and exposes a controllable precision destination cursor. The current sandbox targets roughly 0.35% normal speed during full precision aim.
- Soft and hard blink walls are represented as engine geometry tiers. The current sandbox enables both tiers and makes all interior blink-walls passable except the central hard-solid pillar and outer room boundaries.
- Fast-fall is explicit double-tap-down input, not `hold down`, so down+attack remains reserved for pogo/downward attack intent.
