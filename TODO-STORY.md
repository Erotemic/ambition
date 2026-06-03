# TODO-STORY.md — narrative & worldbuilding tasks

Story / cutscene / cast / town content tasks, kept separate from the engineering
[`TODO.md`](TODO.md). Mechanics go in TODO.md; *what the world says and does*
goes here.

**Authority:** world facts + principles are governed by
[`docs/storylines/cannon.md`](docs/storylines/cannon.md) (Jon's verbatim words
only, changes only when Jon says so). Proposals live in
[`docs/storylines/cannon_expansions.md`](docs/storylines/cannon_expansions.md).
Items here should not contradict canon; where a beat needs a new world fact,
stage it in `cannon_expansions.md` and let Jon promote it.

## Status legend
- `[ ]` not started · `[~]` in progress / partial · `[x]` done (move durable history to `FEATURES.md` / `docs/history/`)
- `[V#/D#]` rough V(alue)/D(ifficulty) 1–5, like TODO.md.

---

## S — Jon's active wants (top priority)

- [ ] **Galwah cutscene plays** `[V4/D3]` — Jon wants the Galwah cutscene to
  actually trigger and play. Galwah already exists (`npc_galwah` in
  `content/character_catalog`, `assets/sprites/galwah_*`), and the intro backlog
  notes a **"Galwah duel"**, but there is **no Galwah cutscene script yet**.
  Build: author a `galwah_*` `CutsceneScript` (beats: Fade / Banner / Dialogue,
  see `intro/cutscene.rs` for the pattern), register it in
  `install_intro_cutscenes`, and bind it to a room via
  `intro_room_cutscene_bindings()` (room_id → cutscene id) so
  `auto_trigger_room_cutscenes` fires it. Use a `with_seen_flag` so it plays
  once. **Open:** which room/trigger does it play in (intro flow vs the hall),
  and is it a *duel* (gameplay) or a pure cutscene? Needs Jon's steer + canon
  for who Galwah is.

- [ ] **A town that feels alive** `[V5/D4]` — build out the post-lab town
  (the drain market / gate-stack area; `drain_alley → drain_market_arrival`
  already exists) so it reads as lived-in, not a corridor. Density of NPCs with
  ambient lines (`intro/banter.rs`), funny mundane signage (canon-adjacent draft
  tone: `GATE 6 DELAYED: SHARK TRAFFIC`, `No swordguns inside the food court`),
  vendors / a merchant, idle animations, foot traffic, the "the street is
  annoyed" tone. Leans on existing NPC + dialog + banter systems; mostly
  authoring + a few small systems (crowd ambience). Break into a vertical slice:
  one packed market screen first.

- [ ] **Alice + Bob quest — a real journey, not adjacent** `[V4/D3]` — the
  cartography route already exists (`content/quest.rs`: Alice's sealed note →
  Bob's field survey; `intro/route_state.rs` flips route/map flags on pickup),
  but Alice and Bob currently sit ~one room apart. Spread them across the intro
  so the quest is an actual traversal: **Alice gives you an item**, you carry it
  through several rooms to **Bob**, and YarnSpinner does interesting things based
  on whether you have it (and on the Alice relationship). Reuse the existing
  inventory↔Yarn bridge (`<<give_item>>` / `inventory_has(...)`) and
  `route_state` flags. Decide Alice/Bob room placements — coordinate with the
  "town that feels alive" and Galwah beats.

- [ ] **Build out the internal inventory (the give→carry→use data layer)**
  `[V4/D3]` — `crate::inventory` already has `ItemKind` + `PlayerInventory` and a
  Yarn bridge (`cmd_give_item`, `inventory_has`, `inventory_counts`,
  `ItemKind::from_dialog_id` in `dialog/yarn_bindings.rs`). Build it out enough
  that Alice can hand over a story item and Bob's (and Alice's) Yarn branches on
  it end-to-end in **real dialogue**, not just unit tests: add the item kind(s)
  the quest needs, make give / has / consume work through the live flow, and a
  minimal "you have X" read. This is the **internal** inventory — the precursor
  the full **OoT-style inventory menu** (`submodules/ambition_inventory_ui`,
  `DESIGN-OOT-DEMO.md`) will later present; keep the data model UI-agnostic so
  both front-ends share it.

- [ ] **Revisit existing intro dialogue/beats with YarnSpinner** `[V3/D3]` — the
  last intro story pass predated YarnSpinner, so beats were hardcoded. Now that
  Yarn + the inventory/state bridge are wired (`dialog/yarn_bindings.rs`,
  `YarnStateMirror`), upgrade static intro lines / cutscenes to Yarn-driven
  branching + state (relationships, route flags, item checks) where it adds
  life. Audit `intro/dialog.rs` + `intro/cutscene.rs` for beats that only existed
  because Yarn wasn't available, and improve existing areas as we pass through.

- [ ] **Gridvania world map: town (left) ↔ post-intro hub ↔ dark forest (right)**
  `[V5/D4]` — Jon: after you rise up out of the intro zone, the world opens into
  a **gridvania** layout (grid-aligned rooms in world space). The **town is on
  the left**; a **dark forest** is to the **right**. Placement is flexible
  ("reposition things as needed") and the **scale of the world should be
  interesting** — but it can be **somewhat coarse right now**. Build: lay the
  macro map out as a coarse gridvania (rooms snapped to a world grid); add the
  **dark forest** as its **own `.ldtk` zone** (multi-file LDtk: `world init`
  scaffolds defs, loader merges, `validate --secondary-world` cross-resolves —
  never hand-edit JSON, use `ambition_ldtk_tools`); wire the rise-out-of-intro
  exit into the hub, town west, forest east. Refine scale/feel later. See
  `docs/concepts/ldtk-world-composition.md` + `[[feedback_ldtk_multifile]]`.
  (Layout facts staged as candidate canon in `cannon_expansions.md` — promote if
  canonical.)

---

## A — Cutscenes & cast

- [ ] Define **Galwah** (personality, role, why a duel) — needs canon. Sprites
  exist; the character is currently just a hall NPC.
- [ ] Erdish appearances + the over-namer bit ("Tangent preservation." / "No.").
- [ ] Oiler first-contact on the street (teaches a movement move; see
  `after_intro_brainstorm.md`).
- [ ] Wire the cart visual (intro wake room still uses a `DebugLabel`
  placeholder per `project-intro-slice`).

## B — Town / world texture

- [ ] Mundane gate signage as authored props/labels (the funny infra signs).
- [ ] Ambient crowd banter pool + a light system to surface lines near the player.
- [ ] A merchant/vendor in the market (ties to the engineering "Merchant +
  buy/sell" TODO — story side: who they are, what they sell, banter).
- [ ] Gate-stack interchange set dressing (gates as infrastructure: queues,
  delays, "SHARK TRAFFIC").

## C — Intro arc depth (from the brainstorm)

- [ ] Each intro room teaches one idea via one toy (wake→raid→escape→drain→gate),
  reusing built mechanics as story beats (gravity ripples in the lab, portal-gun
  prototype as the stolen tool, gun-sword raiders, the creator's diagnostic
  altar = the heal/save shrine). See `cannon_expansions.md` for the mapping.
- [ ] The "you were not the main target" reveal — how much to plant in the intro.

---

## Notes / open questions for Jon
- Where does the Galwah cutscene live, and is it a duel or a talk?
- Town scope: deepen drain_market, or a new dedicated town level?
- How much canon do you want to write for Galwah / Oiler / Erdish before I author
  their cutscenes (so I'm not inventing personality that becomes load-bearing)?
