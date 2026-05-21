---
id: cryptography-crew
status: current
aliases:
  - crypto crew
  - Alice and Bob
  - cryptography NPCs
  - crypto roster
implemented_by:
  - tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/toon_side.py
  - tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/configs/review/
  - crates/ambition_sandbox/assets/sprites/
related_docs:
  - docs/concepts/llm-spatial-authoring-discipline.md
last_verified: 2026-05-21
---

# Cryptography crew

A 13-character canonical roster of cryptography-themed NPCs. The names
are the standard pedagogical cast (Alice, Bob, Eve, Mallory, Trudy,
Craig, Sybil, Trent, Victor, Peggy, Walter, Olivia, Judy); the *roles*
in the game world are deliberately fluid — the cryptographic vocabulary
is the through-line, not a 1:1 mapping. Bob is not necessarily "the
architect", Alice is not strictly "the cryptographer." Each character
gets a unique silhouette and a personality; the cryptographic
inspiration shows up in name, palette, prop, and dialog rather than in
literal job titles.

## Anti-stereotype rule

These characters are a *fun crew*, not a 1990s textbook diagram. Avoid:

- "Eve in a black mask" → cliché evil-hacker. Eve is a hooded
  eavesdropper with a brass listening horn; she reads as a Victorian
  detective overhearing things, not a stock-image cyber-villain.
- "Mallory with a pirate beard and an evil grin" → cartoon villain.
  Mallory is competent and professional in tactical streetwear; the
  threat is precision, not theatre.
- "Alice the cryptographer = lab coat + clipboard" → trope. Alice
  wears a tabard patterned like a one-time pad, hair in a chignon
  with a stick — looks like she could be a chess prodigy.
- "Trent the wise wizard." Trent's robe is *secular council*, not
  spellcaster. Forest green + chain of office + handheld balance
  scales. Civic, not magical.

Lean into the names as character ideas, not as job descriptions.

## Batch 1 — landed (2026-05-21)

These six are PRESETS in `toon_side.py`, with review configs in
`configs/review/` and rendered into `assets/sprites/` via
`draw-runtime-npcs`.

| Name | Silhouette read | Prop | Palette signal |
|---|---|---|---|
| **Bob** | Broad workshop engineer, hi-vis vest, short tousled hair | Carabiner ring with 3 keys | Warm tan + safety yellow |
| **Alice** | Hero-frame academic, tabard patterned like a one-time pad, chignon with hair-stick | Tightly-rolled ribbon-tied cipher scroll | Deep teal + cream |
| **Eve** | Tall hooded cloak, brass ear-trumpet cupped to listen | Listening horn | Aubergine + slate + brass |
| **Mallory** | Rigid tactical field jacket with chrome zip + diagonal strap, red mohawk-style hair | Tablet | Black + oxblood red |
| **Trent** | Broad formal robe with placket buttons + chain of office, clean bald with side fringe | Handheld balance scales | Forest green + brushed gold |
| **Judy** | Broad black judicial robe + crimson placket, full-bottom barrister wig, white jabot collar | Wooden gavel | Black + crimson + white |

The pair (Bob ↔ Alice), the eavesdropper (Eve), the attacker
(Mallory), the arbitrator (Trent), and the judge (Judy) cover the
core protocol vocabulary on their own — they're a self-sufficient
storytelling unit even before the second batch lands.

### Why these six first

Story-side: Alice and Bob are the foundational protocol pair, so they
need to land together. Eve and Mallory cover the two main attacker
archetypes (passive listener vs active interferer). Trent and Judy
cover the two flavors of authority the game's plot will lean on
(council-style arbitration vs adversarial-court judgment). The
remaining seven add nuance but the protocol scaffolding works without
them.

## Batch 2 — queued (see TODO.md)

The other seven canonical names are queued as a follow-up batch.
Sketched silhouette intent below; not yet implemented.

| Name | Role hint | Sketched silhouette |
|---|---|---|
| **Trudy** | Intruder | Lithe, parkour-ready; utility suit with many pockets, bandanna; carries a lockpick. |
| **Craig** | Cracker | Tall and gangly; wide-brim hat, suspenders; carries a stethoscope (safe-cracker, not script-kiddie). |
| **Sybil** | Pseudonymous attacker (many fake identities) | Soft-frame; layered patchwork vest of swapped badges, many small braids, mask stack. |
| **Victor** | Verifier | Sharp rigid frame; square-fringe haircut, precise blazer; carries a magnifier. |
| **Peggy** | Prover | Athletic; long flowing streamer; carries a long pointer; mid-stride dynamic pose. |
| **Walter** | Warden | Rigid; tricorn-style hat, long coat, brass buttons; carries a lantern. |
| **Olivia** | Oracle | Tall; veiled face, many-layered draped robe; carries nothing; stillness. |

When implementing each, follow the anti-stereotype rule and pick a
prop that subverts the cryptographic role rather than literalizing it.

## Pipeline notes

- All six PRESETS live in
  `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/toon_side.py`.
  They mix existing primitives (body plans, the `hood` hair, the
  `tablet`/`medals`/`scarf`/`satchel` props) with new ones added in
  the same patch: hairs `tousled_crop`, `chignon`, `undercut_braid`,
  `clean_bald`, `barrister_wig`; outfits `vest_over_shirt`, `tabard`,
  `eavesdrop_cloak`, `field_jacket`, `formal_robe`, `judicial_robe`;
  props `key_ring`, `cipher_scroll`, `listening_horn`,
  `balance_scales`, `gavel`; accessory `jabot_collar`.
- Review configs in `configs/review/{alice,bob,eve,judy,mallory,trent}.yaml`
  are added to `RUNTIME_REVIEW_NPCS` in `cli.py` so
  `draw-runtime-npcs` installs them with the rest of the toon roster.
- `python -m ambition_sprite2d_renderer draw-character configs/review/<name>.yaml --out-dir /tmp/preview`
  to iterate on one character without overwriting the live assets.
- Per the repo's "never commit binary/generated data" rule, the
  spritesheet + canonical PNGs are gitignored under
  `crates/ambition_sandbox/assets/sprites/.gitignore` (`*.png`). The
  manifest YAMLs (`*_spritesheet.yaml`) ARE committed because they
  carry the frame/animation metadata the runtime reads. PNGs
  regenerate deterministically from the seed in the review YAML, so
  a fresh checkout runs `python -m ambition_sprite2d_renderer
  draw-runtime-npcs` once to populate them.
