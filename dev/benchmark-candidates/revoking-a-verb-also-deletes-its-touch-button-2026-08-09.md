# Revoking a verb also deletes its TOUCH BUTTON — and only on a phone

**Tags:** `architecture-invariant`, `input`, `touch`, `platform-asymmetry`,
`latent-defect`, `held-items`

## What happened

D51: holding the gun-sword fired the laser bolt *and* the wearer's normal jab.
Two claimants on one Attack press. The named precedent was
`revoke_host_owned_ranged`, which fixes exactly this for `ranged` by removing the
whole verb FAMILY from a moveset — so the obvious fix was to do the same for
`attack` at equip time, and restore on unequip.

That fix is wrong, and its failure mode is invisible where you would test it.

The Attack slot in `derive_action_scheme` is a **union** of two authorities:

```rust
push(
    has_directional_verb(ids::ATTACK) || action_set.is_some_and(|a| a.melee.is_some()),
    ControlSlot::Attack, ids::ATTACK,
);
```

Equipping the gun-sword already clears `action_set.melee`. Revoking the moveset's
`attack` family clears the other half — so the body has **no Attack slot at all**.
And two independent consumers read that:

- `gate_worn_player_control` → `resolve_control_slots(.., holds_item)`. An absent
  Attack slot strips `melee_pressed` — **except** that `holds_item` exists
  precisely to keep it alive for the item's own systems. So on a desktop the
  gun-sword still fires. Everything looks fixed.
- `ambition_touch_input::touch_action_available` →
  `prompt.label_for(ControlSlot::Attack).is_some()`. That single expression gates
  both whether the on-screen Attack button is DRAWN and whether it is TOUCHABLE
  (`touch_action_live` / `mask_unavailable`). No slot, no button.

⇒ the "clean" fix ships a weapon that is unusable on a phone, verified green by
every desktop test, and the two consumers disagree *by design* — one has a
held-item exception and the other does not.

## The transferable invariant

**Removing an authority to stop a behaviour also removes everything else derived
from that authority.** A verb binding is not just "what the press does"; here it
is also "does this control exist", which is what a touch overlay draws from.

So when arbitrating between two claimants of one input:

- **Change the RESOLUTION, not the DECLARATION.** `trigger_moveset_moves` now
  asks `HeldItem` who owns the Attack press and answers from the item; the
  wearer's `attack` verbs and timelines are untouched. The slot survives, the
  button stays drawn, and there is no equip/unequip/rewind state to keep
  coherent — the routing reverts the instant the hand is empty.
- **A prep-time precedent does not transfer to a runtime fact unexamined.**
  `revoke_host_owned_ranged` runs at character-definition prep, where the host
  kit is a permanent property of the body. A held item is transient, and the same
  edit at runtime turns a declaration into mutable state.

## The tell to look for

Before deleting a capability declaration to suppress a behaviour, grep for the
declaration's OTHER readers — not the one you are fixing:

```
grep -rn "has_slot\|action_for_slot\|label_for" --include=*.rs crates/ game/
```

If a UI/affordance layer reads it, deletion is a platform-asymmetric bug: the
gameplay path usually has a compensating exception (here `holds_item`) and the
presentation path does not. This is the same family as
`two-tables-decide-a-touch-buttons-life` — drawn by one table, bound by another.

## Second finding: a press spent across a SCHEDULE PHASE

The item systems run in `PlayerSimulation`; `trigger_moveset_moves` runs in
`Combat`, one phase later. Every item action *keeps* the item — except the throw,
which removes `HeldItem`. So on the throw tick the trigger found an empty hand
and handed the very same press to the wearer's jab: the fix looked complete and
throwables still double-fired.

**An arbiter keyed on a component cannot see a press spent by removing that
component earlier in the tick.** The consumer marks the press spent where it
spends it (`control.0.melee_pressed = false`), exactly as
`pickup_held_item_system` already did — not an ordering edge between two phases
that exist to be independent.

## Where it is enforced now

`crates/ambition_combat/src/moveset/mod.rs` — `held_weapon_attack_move` +
the `HeldItem` arm of `trigger_moveset_moves`;
`crates/ambition_platformer2d_actor_monolith/src/items/pickup/mod.rs` —
`throw_held_item_system` consumes its press.
Tests: `the_gunsword_owns_the_attack_press_on_the_ground` / `..._in_the_air`
(the airborne one is the poison — a base-verb-only guard hands the jab back the
moment you leave the ground), `a_thrown_item_owns_the_attack_press_too`,
`a_melee_item_answers_the_attack_press_with_its_own_swing`.
