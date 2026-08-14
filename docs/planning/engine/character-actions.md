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

- ▢ **Finish cast action authoring where a body still relies on implicit/default
  repertoire.** Keep the rule body-owned: the character authors abilities and
  move repertoire; a controller only supplies intent. Do not introduce a
  fighter/peaceful taxonomy to compensate for missing body data.

- ▢ **Give authored moves useful presentation metadata where still missing.** A
  stable move/action id remains the machine identity; a display label and
  optional icon/prompt presentation are authoring metadata, with a deterministic
  fallback when absent.

- ▢ **Decide layout behavior only when a real repertoire exceeds prompt capacity.**
  Do not pre-generalize the control surface for hypothetical >8-slot schemes.

- ▢ **Decide whether availability exposes only repertoire presence or also
  cooldown/charge state** when a concrete UI consumer needs the distinction.

## Exit

Possessing/controlling any authored body yields prompts and available actions
from that body's prepared repertoire, with no game-specific prompt table and no
fallback combat identity hidden in controller code.
