# Control affordances

**Review date:** 2026-05-30. Reviewed against source archive `ambition-source-2026-05-30T104014-5-e721ea65c578`.

The control-affordance system computes “what would each button do right now?” as a typed table. The HUD uses that table for context-sensitive labels, glyphs, and pressed-state display.

This is a presentation/input-feedback system, not the canonical gameplay execution path. Gameplay has its own current consumers: player movement reads `ActorControl`, player melee start reads `ActorActionMessage::Melee`, projectiles consume `ActionRequest::PlayerProjectileTick`, and pogo start still has a player-specific path while target-surface policy is shared through `BlockKind::is_pogo_target()`. Future cleanup may reuse affordance resolvers directly in gameplay, but that is not true today.

## Current architecture

```text
ControlFrame + player cluster components + proximity resources
        ↓
compute_player_intent
update_nearest_interactable
update_pogo_target_below
        ↓
compute_player_affordances
        ↓
PlayerAffordances { jump, attack, shield, dash, interact, special }
        ↓
HUD verb/glyph/pressed-state systems
```

Important paths:

| File | What it owns |
|---|---|
| `crates/ambition_actors/src/player/affordances/intent.rs` | `Aim`, `PlayerIntent`, `compute_player_intent`. |
| `crates/ambition_actors/src/player/affordances/variants.rs` | `AttackVariant`, `JumpVariant`, `ShieldVariant`, `DashVariant`, `InteractVariant`, `SpecialVariant`, and `VariantLabel`. |
| `crates/ambition_actors/src/player/affordances/resolvers.rs` | Pure resolver functions from body/world/aim views to variants. |
| `crates/ambition_actors/src/player/affordances/interactable_proximity.rs` | Nearest interactable classification. |
| `crates/ambition_actors/src/player/affordances/pogo_proximity.rs` | Downward pogo-target proximity; aerial down-attack can label as `Pogo` when a pogo target is below. |
| `crates/ambition_actors/src/player/affordances/devices.rs` | Active input method and glyph lookup. |
| `crates/ambition_app/src/host/mobile_input/bevy_plugin.rs` | HUD consumer for touch/control prompt buttons. |

## Current variants

| Verb | Variants | Notes |
|---|---|---|
| `JumpVariant` | `Jump`, `Climb`, `Unmorph`, `Stroke` | Labels jump by ledge/body/swim context. |
| `AttackVariant` | `Jab`, `DTilt`, `UTilt`, `FSmash`, `DSmash`, `USmash`, `NAir`, `BAir`, `UAir`, `DAir`, `Pogo` | Smash variants exist as vocabulary; resolver currently picks jab/tilts/aerials/pogo. Held smash input is not implemented. |
| `ShieldVariant` | `Shield`, `Roll` | Ledge context maps shield to roll. |
| `DashVariant` | `Dash`, `Dodge` | Aerial dash label reads as dodge. |
| `InteractVariant` | `None`, `Talk`, `Open`, `Use`, `Activate`, `Custom(Cow)` | `Custom` carries authored prompt text. |
| `SpecialVariant` | `Special`, `NeutralSpecial`, `SideSpecial`, `UpSpecial`, `DownSpecial`, `Hadouken`, `Blink` | The resolver is aim-only today and does not read the projectile motion-input buffer. `Hadouken`/`Blink` are reserved vocabulary, not proof that the HUD is executing those outcomes. |

## Device glyph status

`ActiveInputMethod` is keyboard/touch first today. The glyph table contains gamepad glyphs by `GamepadKind`, but automatic gamepad detection is still deferred in `devices.rs`. Keyboard glyphs currently use the default `KeyboardPreset::arrows_zxc()` until the active preset is threaded into the HUD path.

## Design principles

1. **Closed variants over open strings.** The HUD renders variants such as `AttackVariant::DAir`, not arbitrary labels.
2. **Pure resolvers.** Resolver functions are unit-testable and can later be shared with gameplay code if/when the gameplay branch is ready.
3. **One derived value per system.** Verb, glyph, and pressed state are updated separately and composed by the HUD.
4. **Do not treat the HUD as execution authority.** The affordance table describes what should happen; current gameplay execution still lives in movement/combat/projectile systems.

## Adding a new contextual label

1. Add a variant to the appropriate enum in `variants.rs`.
2. Add `text()` and `i18n_key()` entries.
3. Add the branch in the pure resolver.
4. Add or update tests in `resolvers.rs` / `variants.rs`.
5. Only then wire gameplay execution if the new label represents a new real action.

## Adding a new HUD consumer

A consumer should read the already-computed `PlayerAffordances` table rather than duplicating contextual rules. Example shape:

1. Add a marker/resource for the tutorial, prompt, or overlay state.
2. Read `PlayerAffordances` after `AffordancesSystemSet::Compute`.
3. Match the typed variant, such as `AttackVariant::Pogo` or `InteractVariant::Open`.
4. Render the label/glyph/highlight without becoming gameplay authority.

This keeps desktop HUD, touch controls, tutorials, and accessibility prompts on the same vocabulary.

## Schedule and ordering

`AffordancesPlugin` computes intent, proximity, and final affordances in `Update` under `AffordancesSystemSet::Compute`. HUD and mobile-input presentation systems should run after that set so they read this frame's labels. The active input method detector is intentionally presentation-facing; it updates glyph choice, not the gameplay `ControlFrame`.

## Example contextual label

For a rule such as “a low-health aerial down attack labels as vampiric pogo when a pogo target is below,” keep the change localized:

1. Add `AttackVariant::VampPogo`.
2. Add label/i18n/icon entries in `variants.rs`.
3. Add the needed body/world field to the resolver input view.
4. Branch in `resolve_attack`.
5. Add resolver and HUD tests.
6. Wire gameplay only if the variant has a distinct real effect.

## Migration target

The long-term target is for gameplay systems and HUD systems to share the same pure resolver functions where possible. The affordance table would then be a cached answer for presentation, not a parallel guess. That migration is not complete today, so docs should continue to name direct gameplay consumers explicitly.

## Testing anchors

```bash
cargo test -p ambition_actors --lib affordances
```
