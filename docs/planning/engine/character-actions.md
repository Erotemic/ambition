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

- ▢ **Give authored moves useful presentation metadata where still missing.** A
  stable move/action id remains the machine identity; a display label and
  optional icon/prompt presentation are authoring metadata, with a deterministic
  fallback when absent.

  ⚠ **VERIFIED OPEN 2026-08-20, and the code says so itself.**
  `MoveSpec::display()` title-cases the id (`"sandbag_swat"` → `"Sandbag
  Swat"`), and its own doc comment reads *"P6 adds an authored `display_name:
  Option<String>` field that this reads first"* — that field does not exist on
  `MoveSpec`. The deterministic fallback is built; the authored override is not.
  ⭐ the CONSUMER is live: `action_scheme.rs:295` fills each slot's
  `display_name` from `mv.display()`, so a label authored on the move would
  reach the control prompts with no new plumbing. Today there is no production
  way to author a player-facing move label at all — `"tilt_up"` can only ever
  read "Tilt Up".

  ⚠ **PRICED 2026-08-20, and the price is the reason nobody has done it.**
  `MoveSpec` is built by **100 exhaustive struct literals** and exactly ONE of
  them uses `..Default::default()`, so a new required field is a 100-site
  mechanical edit whose entire content is `display_name: None`. That is a
  carry list with no judgement in it — the opposite of the exhaustive
  destructures worth keeping, which exist to force an author to think.

  ⇒ **do not add the field first.** The cheap enabling move is a `Default` impl
  or a constructor helper that the 100 sites can adopt incrementally; then the
  field costs one line. ⭐ serde is already fine either way (`MoveSpec` derives
  `Serialize`/`Deserialize`, so `#[serde(default)]` covers the saved shape), and
  `MoveSpec` is NOT in any rollback registration, so this does not touch the
  schema baseline.

- ▢ **Decide layout behavior only when a real repertoire exceeds prompt capacity.**
  Do not pre-generalize the control surface for hypothetical >8-slot schemes.

- ▢ **Decide whether availability exposes only repertoire presence or also
  cooldown/charge state** when a concrete UI consumer needs the distinction.

## Exit

Possessing/controlling any authored body yields prompts and available actions
from that body's prepared repertoire, with no game-specific prompt table and no
fallback combat identity hidden in controller code.
