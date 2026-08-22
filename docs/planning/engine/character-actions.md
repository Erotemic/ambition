# Character actions — remaining authoring work

> **Verified against `cecd01ca` (2026-08-13).** The old P0–P5 migration is no
> longer open: the slot→action seam, control prompts, live binding overrides,
> vendor-aware glyphs, and rebind capture UI exist. The pre-prune campaign is
> archived at
> [`../../archive/planning-superseded/2026-08-13/engine/character-actions.md`](../../archive/planning-superseded/2026-08-13/engine/character-actions.md).

Host/input followups belong to
[`participant-action-system.md`](participant-action-system.md). This file owns
only character/action authoring that still remains.

## Remaining

- ✔ **Cast action authoring no longer falls back to a shared kit** — verified
  2026-08-20 against HEAD, not counted. `smash_fighter_kit()` is DELETED
  (2026-08-12, `demo_smash/src/select.rs:862`); every remaining mention in the
  tree is a doc comment explaining its removal, and **eighteen** characters
  author their own `*_moveset() -> MovesetContract`. The rule the item asked for
  — body-owned repertoire, controller supplies intent only, no
  fighter/peaceful taxonomy — is the shipped shape.

- ✔ **Authored move labels — SHIPPED 2026-08-22.** `MoveSpec` carries
  `display_name: Option<String>` (`#[serde(default)]`), `MoveSpec::display()`
  reads it first and falls back to the title-cased id, and the consumer needed
  no plumbing: `combat_actions` already filled each slot's label from
  `mv.display()`. Pinned by
  `action_scheme::tests::an_authored_move_label_beats_the_title_cased_id`.

  ⭐ **and it is authored, not just enabled.** The directional prefab now names
  its variants the way the genre does — Up Tilt, Down Tilt, Forward/Up/Back/Down
  Air — through a `label` parameter on `directional_attack_variants`' `variant`
  helper. That prefab's own doc already called them *"up-/down-tilt + the four
  aerials"*, so the control prompt now says what the design language says instead
  of "Attack Air Down".

  ⛔⛔ **THE PRICE IN THIS ROW WAS WRONG BY MORE THAN 10×, and that is the lesson
  worth keeping.** It said *"100 exhaustive struct literals, so a new required
  field is a 100-site mechanical edit"* and concluded *"do not add the field
  first"*. There are 100 `MoveSpec {` literals, but a new required field broke
  **13 production sites and 19 test sites** — the rest construct through helpers
  or a `..base` spread. ⇒ **counting a TOKEN is not counting the EDIT.** The
  enabling `Default` this row asked for first was never needed, which is
  fortunate: an exhaustive literal is what forces an author to answer a new
  gameplay field, and defaulting it is how a field gets silently skipped.

- ▢ **Decide layout behavior only when a real repertoire exceeds prompt capacity.**
  Do not pre-generalize the control surface for hypothetical >8-slot schemes.

- ▢ **Decide whether availability exposes only repertoire presence or also
  cooldown/charge state** when a concrete UI consumer needs the distinction.

## Exit

Possessing/controlling any authored body yields prompts and available actions
from that body's prepared repertoire, with no game-specific prompt table and no
fallback combat identity hidden in controller code.
