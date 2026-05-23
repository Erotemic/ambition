## Control affordances

Status (2026-05-23): Phase 1 + 2 + 3 landed end-to-end. The on-screen
touch HUD now reads as a context-sensitive control overlay even on
desktop builds — verbs swap by player state (Shield → Roll on a ledge,
Attack → D-Air mid-air with the stick held down), per-device glyphs
render under each verb (Z / X / C for keyboard, A / X / RB for gamepad
face buttons), and held buttons highlight as a streamer-style input
display. The architecture is the centerpiece: a single
`PlayerAffordances` resource computed each frame is the *only* source
of truth for "what would each press do right now," consumed by both
the HUD today and (future) gameplay code so the prompt the player sees
can never disagree with what the simulation actually fires.

## Why this exists

Two adjacent failure modes the previous design slid into:

1. **HUD-as-strings drift.** The old `contextual_actions.rs` had one
   growing `match` returning `Cow<'static, str>` per (verb, state)
   combination. Each new context rule grew both a flat-bool
   `PlayerActionContext` struct (`is_aerial`, `on_ledge`,
   `aim_down`, `aim_up`, `aim_back`, …) and the `label_for` match.
   The label was a string the HUD believed; gameplay had its own
   parallel `if`-chain to decide which attack actually fired when
   the player pressed the button. They could drift, and the HUD
   would lie about what was about to happen.

2. **Touch-only context.** The Phase-0 wiring only ran on the
   `mobile_touch` HUD path. Desktop / gamepad players got no
   prompt feedback even though the same data was already computed.

Both fixes share a shape: name the **outcome** (the typed variant of
the action that would fire), not the label, and put it in a typed
resource. The HUD displays the variant; gameplay reads the same
resource (or, for performance, calls the same resolver) to decide
the real outcome. There's nothing left to drift.

## Architecture

```text
   ControlFrame                PlayerMovementAuthority + components
        |                                      |
        v                                      v
  compute_player_intent              (built into compute_player_affordances)
        |
        v
   PlayerIntent { aim: Aim }                NearestInteractable
        \                                       /
         \           PogoTargetBelow           /
          \                |                  /
           +-------+-------+--------+--------+
                   |                |
                   v                v
            compute_player_affordances
                   |
                   v
   +-----------------------------------------------+
   | PlayerAffordances {                           |
   |   jump:     JumpVariant,                      |
   |   attack:   AttackVariant,                    |
   |   shield:   ShieldVariant,                    |
   |   dash:     DashVariant,                      |
   |   interact: InteractVariant,                  |
   |   special:  SpecialVariant,                   |
   | }                                             |
   +-----------------------------------------------+
                   |
   +---------------+--------------+--------------+
   |               |              |              |
   v               v              v              v
 Touch HUD     Future        Future AI      Future tutorial
 (verb +       gameplay      hint system    overlay
  glyph +      consumers     (read for
  pressed)                    "what can I
                              still do?")
```

Files (all under `crates/ambition_sandbox/src/player/affordances/`):

| File | What it owns |
|---|---|
| `intent.rs` | `Aim` 9-way enum, `compute_aim`, `PlayerIntent` resource, `compute_player_intent` system, `Aim::arrow_glyph` pure helper |
| `variants.rs` | `AttackVariant` / `JumpVariant` / `ShieldVariant` / `DashVariant` / `InteractVariant` / `SpecialVariant` + `VariantLabel` trait |
| `resolvers.rs` | Pure `resolve_attack / _jump / _shield / _dash / _interact / _special(intent, body, world)` |
| `interactable_proximity.rs` | `NearestInteractable` resource (Talk / Open / Activate / Custom) |
| `pogo_proximity.rs` | `PogoTargetBelow` resource (downward AABB scan for `PogoTargetContributor`) |
| `devices.rs` | `InputMethod` + `GamepadKind` + `ActiveInputMethod` resource + `glyph_for(action, &KeyboardPreset, InputMethod) -> Cow<str>` |
| `mod.rs` | `PlayerAffordances` resource, `compute_player_affordances` system, `AffordancesPlugin`, `AffordancesSystemSet::Compute` |

The HUD consumer lives in `crates/ambition_sandbox/src/host/mobile_input/bevy_plugin.rs`:

| Component | Owner | Updated by |
|---|---|---|
| `ButtonVerb` | Touch button Text child | `update_button_verb_from_affordances` (reads `PlayerAffordances`) |
| `ButtonGlyph` | Touch button Text child | `update_button_glyph_from_active_input` (reads `ActiveInputMethod`) |
| `ButtonPressed` | Touch button entity | `update_button_pressed_from_actions` (reads `ActionState<SandboxAction>` from primary player) |
| `JoystickPromptText` | L-stick overlay Text | `update_joystick_prompt_text` (reads `PlayerIntent.aim.arrow_glyph()`) |

Each derived state has its own narrow update system; `render_touch_button_text` folds verb + glyph; `sync_button_pressed_visual` flips the `BackgroundColor` on press. Adding a new derived value (charge level, cooldown timer, etc.) means one new component + one new update system, not editing existing logic.

## Design principles

The arrangement here follows three rules; they're more important than
the specific variant names.

1. **The label is just the `Display` of the action variant that would
   fire.** Variant enums are typed and closed (`AttackVariant::DAir`
   not `"D-Air"`); the HUD calls `VariantLabel::text()`. When
   gameplay refactors to use the resolvers, the HUD and the sim can
   never disagree by construction.

2. **Each derived value is its own first-class object.** Not a flat
   god-struct of bools; not a god-system that does everything. Verb,
   glyph, pressed-state, charge, etc. each get their own component +
   update system. Independent concerns compose at render time.

3. **Closed sets over open booleans.** `Aim` is a 9-way enum, not
   three booleans (`aim_up`/`aim_down`/`aim_back`). The compiler
   enforces exhaustive `match`; impossible combinations don't exist.

## Adding a new contextual rule

Walk through one example: "Pogo upgrades to a vampiric pogo when the
player is below a fixed HP threshold."

1. Add `AttackVariant::VampPogo` + `VariantLabel` impls.
2. Add a field to `PlayerBodyView`: `pub is_low_hp: bool`.
3. Branch on it in `resolve_attack`:
   ```rust
   } else if body.is_aerial && aim.is_down() && world.pogo_target_below {
       if body.is_low_hp { AttackVariant::VampPogo } else { AttackVariant::Pogo }
   }
   ```
4. Populate the new field in `compute_player_affordances`. Done.

The HUD updates for free because it just renders the variant. No
mobile / desktop / accessibility consumer changes.

## Adding a new HUD consumer

Walk through: "a tutorial overlay that highlights one specific button
when its variant matches what the tutorial step is teaching."

1. Add a marker component (`TutorialHighlight { target_action: AttackVariant::DTilt }`).
2. Add a system that reads `PlayerAffordances` and the marker, and
   sets a glow color when `affordances.attack == target_action`.

No new ECS plumbing for the contextual machinery; the affordance
table is already the shared shape between every consumer.

## Per-device glyph rendering

`InputMethod` is `Keyboard` / `Gamepad(GamepadKind)` / `Touch`.
`ActiveInputMethod` updates each frame ("last input wins"):

- Touch wins when any active touch is present.
- Keyboard wins on the next `KeyCode::just_pressed`.
- Gamepad detection is a TODO — Bevy 0.18's gamepad API changed shape
  and the detection wiring is left for a follow-up. The keyboard
  glyphs are conservatively correct in the interim.

`KeyboardPreset` (the player's chosen binding preset) is the source
of truth for keyboard glyphs. The HUD reads it from a hardcoded
default today (`KeyboardPreset::arrows_zxc()`) because the active
preset isn't exposed as a Resource yet; threading the actual selected
preset is a one-line follow-up once the preset surface is split out.

## Variant vocabulary

The current variants (the on-disk labels follow Smash conventions):

| Verb | Variants | Notes |
|---|---|---|
| `JumpVariant` | `Jump` / `Climb` / `Unmorph` / `Stroke` | |
| `AttackVariant` | `Jab` / `DTilt` / `UTilt` / `FSmash` / `DSmash` / `USmash` / `NAir` / `BAir` / `UAir` / `DAir` / `Pogo` | Smash attacks reserved (`F-Smash` / `D-Smash` / `U-Smash` exist but the resolver doesn't pick them yet — they're held inputs that need an input-buffer signal); `Pogo` requires `pogo_target_below`. |
| `ShieldVariant` | `Shield` / `Roll` | |
| `DashVariant` | `Dash` / `Dodge` | |
| `InteractVariant` | `None` / `Talk` / `Open` / `Use` / `Activate` / `Custom(Cow)` | `Custom` carries the authored interactable's prompt string |
| `SpecialVariant` | `Special` (fallback) / `NeutralSpecial` / `SideSpecial` / `UpSpecial` / `DownSpecial` / `Hadouken` / `Blink` | All variants fire the same gameplay outcome (fireball) today; the HUD distinguishes them so the player gets feedback for stick direction. `Hadouken` is the QCF seam — wires up once an input-history buffer detects the motion. |

The variant enums each implement `VariantLabel { text, icon, i18n_key }`. The HUD calls `text()` today; `icon()` is reserved for future symbolic rendering, and `i18n_key()` for future localization (key shape: `"<verb>.<variant>"` snake_case, e.g. `attack.d_air`).

## Schedule

`AffordancesPlugin` registers three systems chained in `Update` under `AffordancesSystemSet::Compute`:

1. `compute_player_intent` — reads `ControlFrame` + primary player's `PlayerMovementAuthority` for facing; writes `PlayerIntent`.
2. `update_nearest_interactable` + `update_pogo_target_below` — walk the feature world (peaceful actors, switches, intact chests; `PogoTargetContributor` entities below the player); write proximity resources.
3. `compute_player_affordances` — pulls intent + proximity + the primary player's body view; calls each resolver; writes `PlayerAffordances`.

`detect_active_input_method` runs unchained in `Update` (keyboard + touch input edges only; gamepad detection deferred).

HUD systems in `mobile_input/bevy_plugin.rs` pin `.after(AffordancesSystemSet::Compute)` so they see this frame's values.

## Testing anchors

```bash
cargo test -p ambition_sandbox --lib affordances
```

Unit tests cover:

- `intent.rs::tests` — `compute_aim` for every cardinal + diagonal +
  neutral combination, facing-relative behavior, `Aim::arrow_glyph`.
- `variants.rs::tests` — every variant's `text()` and i18n-key prefix.
- `resolvers.rs::tests` — each verb's resolver for every aim/state
  combination; pogo-target-below promotion; interact prompt forwarding.
- `mod.rs::tests` — full-app integration via `AffordancesPlugin`:
  baseline labels, aerial-down → `DAir`, ledge → `Climb` + `Roll`,
  back-air detection, special direction dispatch.
- `devices.rs::tests` — keyboard glyph from preset; gamepad glyph by
  `GamepadKind`; touch glyph is empty.

## Migration target (future)

The eventual shape: gameplay subsystems (attack, jump, shield, …)
call the same `resolve_*` functions to decide what their tick should
do. The affordance table becomes the *cached* answer, not just the
HUD's answer. At that point a regression test can assert that for
every (intent, body, world) combination, the HUD's displayed variant
and the gameplay system's executed variant are identical.

Until then, gameplay code keeps its own branching but the HUD's
labels are still authoritative documentation of the contextual rules
— a `git grep AttackVariant::DAir` finds every place the system
treats down-air specially.
