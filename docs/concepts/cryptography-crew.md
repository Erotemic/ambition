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
| **Bob** | Workshop engineer with visible legs + workboots, vest open over a tee, tool belt with three hanging tools (key ring + wrench + hammer); ships in 3/4 (idle), side (walk), and front (talk/interact) views | Carabiner ring with 3 keys held in hand + the three belt tools | Warm tan vest + slate-blue tee + safety yellow |
| **Alice** | Lean academic with a knee-length cinched traveling coat over the OTP-checker tabard, ankle-cuffed leggings + ankle boots, layered long hair (back mass + cheek curtains + bangs + forward braid) | Ribbon-tied cipher scroll held in hand | Deep teal coat + cream OTP tabard + amber sash |
| **Eve** | Tall hooded cloak, brass ear-trumpet cupped to listen | Listening horn | Aubergine + slate + brass |
| **Mallory** | Rigid tactical field jacket with chrome zip + diagonal strap, red mohawk-style hair | Tablet | Black + oxblood red |
| **Trent** | Robe-first silhouette (no stick-figure peeking through), elongated head with jaw extension, full white flowing beard, gold chain of office with medallion | Handheld balance scales | Forest green + brushed gold |
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

## Phenotype variation across the cast

Skin tones, hair textures, and facial features are deliberately
spread across the human phenotype range so the cast actually
looks like a crew rather than seven shades of the same person.
The batch-2 sketches are where the spread became explicit; the
guideline for any future addition is to look at the existing
palette set and pick a tone that's not already represented.

Current spread (skin → hair):

- Olivia: very pale `#F2DDC6` → platinum `#F4EEDC`
- Alice: light peach `#E5C5A6` → blue-black `#15131C`
- Craig: pale freckled `#ECCBB0` → faded auburn `#7A4630`
- Eve / Judy: muted tan range
- Trudy: warm tan `#D9B190` → jet black `#0E0B12`
- Walter: medium cool `#C2A48B` → silver `#B4B0B0`
- Trent: mid-tan `#C9A78B` → white beard
- Mallory: warm tan `#D6A38B` → oxblood-red hair
- Victor: olive `#B58B6E` → dark `#1A1820`
- Bob: warm-olive `#B58968` → dark brown `#3D2B22`
- Peggy: rich brown `#97694A` → black ponytail
- Sybil: deep brown `#6B4530` → black braids

## Batch 2 — landed (sketches)

Seven first-pass sketches on the toon template. Each may be
promoted to a bespoke template (like trent_elder / bob_engineer /
alice_cryptographer) when a story room calls for it. The sketches
fix the canonical names + phenotype + prop + outfit in place so
later refinements can iterate on geometry without re-deriving
identity.

| Name | Silhouette read | Prop | Palette signal |
|---|---|---|---|
| **Trudy** | Lean intruder; visible ponytail flowing back, dark green field jacket, lockpick + tension wrench in hand | Lockpick set | Slate green + warm tan skin (East Asian phenotype) |
| **Craig** | Tall + gangly safe-cracker; wide-brim hat over faded auburn hair, denim apron, stethoscope at the safe (NOT a script-kiddie hacker) | Stethoscope | Pale freckled European + denim blue |
| **Sybil** | Layered patchwork poncho over many small braids, holding a fan of small masks (three colors) | Stack of masks | Plum + marigold + deep brown skin |
| **Victor** | Sharp slate blazer, square-fringe haircut, holding a magnifier — precise and analytical | Magnifier | Slate blue + chrome accent + olive skin |
| **Peggy** | Athletic prover stance with a long pointer/wand; high ponytail, warm orange jacket | Long pointer/wand | Warm orange + cream + rich brown skin |
| **Walter** | Stern warden; tricorn hat over silver hair, deep navy long coat, brass-trimmed handheld lantern | Lantern | Deep navy + brass + silver hair |
| **Olivia** | Veiled oracle in a lavender layered robe; hands hidden in long sleeves, no held prop | none (hands in sleeves) | Lavender veil + silver-pale skin + platinum hair |

## Promotion criteria

A batch-2 sketch should be promoted out of toon (i.e. given a
bespoke target like trent_elder / bob_engineer / alice_cryptographer)
when:

- The story room needs multi-view rendering (front pose for dialog,
  side profile for walking).
- The character's silhouette would be hard to reach by tweaking
  toon primitives — e.g. Olivia's veiled drape, Sybil's many-braid
  textured hair, or characters who'd carry both hands forward.
- A specific gesture or animation can't be expressed in toon's
  `idle / walk / talk / interact / hit / death` vocabulary.

Promote into a new file `targets/<name>_<role>.py` + new adapter,
following the trent_elder / bob_engineer / alice_cryptographer
file conventions.

## Templates

Two adapters serve the crew today:

- **`toon`** (`targets/toon_side.py`) — the shared cartoon
  template that ships Bob, Alice, Eve, Mallory, Judy. Stick-figure
  construction underneath: capsule limbs + oval head + applied
  hair/outfit/prop dispatch tables. Easy to add new characters by
  authoring a preset; harder to escape the "everyone is basically
  the same body under the costume" read.
- **`trent_elder`** (`targets/trent_elder.py`) — bespoke template
  for Trent. Robe-first composition (no capsule limbs poking out),
  elongated head with a jaw extension that gives the beard a real
  triangular base, two-tone draped fabric, no shared dispatch
  tables. Single archetype today; the geometry vocabulary (head +
  jaw + beard + robe + side-fringe + chain-of-office) is documented
  so a future "council batch" character can be added by overriding
  the palette and a few proportions.
- **`bob_engineer`** (`targets/bob_engineer.py`) — bespoke template
  for Bob, builds on the trent_elder lessons and adds **multi-view
  rendering**: each animation locks one of three views (3/4 / side /
  front) via the `ANIMATION_VIEWS` table, with per-view draw
  functions for head, body, and arms. Walking uses the side
  profile (one eye, ear visible, nose forward, leg swing); talking
  and interact use the front view (symmetric face + ears + bangs);
  idle stays at 3/4 for canonical-preview continuity. Also lands a
  visible-legs construction (no robe hiding them), workshop boots,
  and a leather tool belt with three hanging tools (key ring /
  wrench / hammer). Single archetype today.
- **`alice_cryptographer`** (`targets/alice_cryptographer.py`) —
  bespoke template for Alice, third scaffold after Trent and Bob.
  Silhouette philosophy: a knee-length cinched traveling coat over
  an OTP-checker tabard with ankle-cuffed leggings + ankle boots
  showing beneath the flared coat hem — distinct from Trent
  (fully-draped robe) and Bob (workshop vest with belt-tools).
  Layered hair construction is the key new pattern: hair is a
  stack of FOUR primitives (back mass + cheek curtains + bangs +
  forward braid), each its own polygon so the front fringe doesn't
  fight the back mass. Multi-view: 3/4 (idle/talk/interact) +
  side (walk/idle_side); front is queued. Cleaner face geometry:
  smaller pupils with iris + sclera + outer-corner eyelash tick,
  no chin shadow (per the feminine-archetype rule). Carries a
  ribbon-tied cipher scroll. Single archetype today; the hair
  primitive vocabulary (back / curtain / bangs / braid) is
  documented so future long-haired characters can reuse it.

The expectation going forward: any character whose silhouette
genuinely doesn't fit the toon template gets a bespoke template
file. Don't keep stretching the toon target to handle every
silhouette — the per-character template cost is real (~500
lines), but each one delivers a quality jump that's hard to get
from preset tuning alone.

## Pipeline notes

- The toon PRESETS live in
  `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/toon_side.py`.
  They mix existing primitives (body plans, the `hood` hair, the
  `tablet`/`medals`/`scarf`/`satchel` props) with new ones added in
  the same patch: hairs `tousled_crop`, `long_side_braid`,
  `forward_braid`, `clean_bald`, `barrister_wig` (and the original
  `chignon` / `undercut_braid` kept for opt-in androgynous reads);
  outfits `vest_over_shirt`, `tabard`, `cinched_tabard`,
  `eavesdrop_cloak`, `field_jacket`, `cinched_field_jacket`,
  `formal_robe`, `judicial_robe`; props `key_ring`,
  `cipher_scroll`, `listening_horn`, `balance_scales`, `gavel`;
  accessory `jabot_collar`.
- Trent's bespoke template lives in `targets/trent_elder.py` with a
  one-line registration in `adapters.py`. His review config sets
  `target: trent_elder` instead of `target: toon`.
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
