# Inhabited-world dialogue: story threads across places

**Status:** Working content pass, open to revision. Story proposals remain
proposals until explicitly adopted in `docs/storylines/cannon.md` or
`cannon-elevated.md`; this document does not change either canon file.

**First pass:** 2026-10-24, source snapshot `057721102dc3`. Companion to the
Math Town dialogue work, rather than a replacement for it.

## Purpose

Conversations in different places should connect because the residents share
routes, trades, jobs, relatives, or problems. A player may hear about the same
subject in several places, but the accounts should not all explain it or agree.
The early game needs normal lives alongside plot clues. We can revise specific
claims as the main story develops.

The source of truth for playable lines is
`game/ambition_content/assets/dialogue/sandbox/*.yarn`. The guide for writing
and reviewing them is `docs/concepts/anti-llmism-style-guide.md`.

## Threads seeded in this pass

| Thread | Locations / characters | Established or proposed relationship | Follow-up opportunities |
| --- | --- | --- | --- |
| Freight and gate economics | Ines, Bram, Jory, Agnes, Bo Lindqvist, Reyes, Pulse | Rings move goods quickly but capacity, power, and customs constrain travel; ships still work | A delayed delivery someone genuinely depends on; a choice between gate and ship |
| Passenger records and Castle Godwin | Ines, Okafor, the General, Fretjaw, Alice and the manifest clerk | Requests for names are surfacing from different offices; characters differ in how they handle them | Identify the requester, the person at risk, and the legal limits of sharing a list before any embassy storyline |
| Gnu-ton's gravity works | Wen, Rahimi, Reyes | Work in the Science dimension can be dangerous; residents have partial, occupation-specific evidence | Connect field anomalies and power budgets to a real authored area, without declaring the station incident Gnu-ton's fault |
| The Mockingbird and the ninja-pirate agreement | Admiral, Bess, Shadow Oni, the Duelist | The groups have a narrow common objective, different stakes, and little reason to trust each other | Resolve differences after the boss while preserving the cove treasure quest and dojo access |
| Kagero and Castle Godwin | Sato, Daichi, Hayashi, Duelist, Shadow Oni | Kagero is spoken of as independent of the village dojo; a few people have heard where she went | Optional recognition after her actual boss-fight appearance, keyed to a reliable event if one is authored |
| The wrong-list raid and safer maps | Alice, Bob, Oiler, the clerk, the gate janitor | The wrong address has visible paperwork and human consequences; the cartographers protect routes | Trace one document through the institutions; keep the creator's unfinished sentence unresolved |
| A future friendship | Oiler | Mentions Faith "Fia" Knoss as a fellow mechanic, without starting or resolving her future plot | Introduce Fia in person through repairs and trust, rather than an explanatory speech |
| Local character relationships | Wren, Bram, Jory, Ume, Ren, Daichi, Gus Pell | Residents know and annoy one another; a town has routines when the player is absent | Add small stateful callbacks if those interactions become playable |

The Castle Godwin list requests, gravity-test paperwork, and individual bits
of personal history in the Yarn lines are **provisional playable seeds**. They
should not be taken as completed missions, documented historical events beyond
what the characters say, or automatic promotion into locked canon.

## Scope of this pass

Expanded optional player choices, with existing entry node IDs intact, in:

- `pirate_port.yarn` — the freight office, shop, wharf, inn, and a visitor from
  the cove;
- `ninja_village.yarn` — couriers, tea house, students, and rank examination;
- `orbital.yarn` — customs, engineering, survey work, and habitation;
- `cove.yarn` and `dojo.yarn` — the two sides of the Mockingbird dispute;
- `intro.yarn` — cartographers, Oiler, the janitor and manifest clerk;
- `factions.yarn` — the General, Fretjaw and Captain Pulse.

Math Town's `math.yarn` and the large hall/gallery dialogue are deliberately
untouched. The dialogue additions do not create quests, items, world flags,
new interactables, or unlock addresses. They do not change existing Yarn
commands or conditions. Some extra Yarn nodes are added for conversations
whose original layout uses jump-to-subnode menus.

## Continuity rules for further passes

1. Keep **the actual current quest transitions**: Ines's manifest return;
   Okafor's crate clearance and spur pass; Hayashi's rank award; Alice/Bob's
   note and survey; the General's clearance; the pirate Mockingbird result.
2. Never present a **rumor as objective narration**. An NPC can be mistaken.
   Reconcile their claims with future authored events before those events
   become release content.
3. Avoid revealing the creator's unfinished sentence. Give the player clues
   about the raid and its records, not an answer that cancels the mystery.
4. The Ninja dojo does **not** own Kagero. GNU-ton's gameplay works are in the
   **Science dimension**, with the orbital station, per the current gate-network
   and boss-fight plans.
5. Distinguish a claimed total of seventeen dimensions from **seventeen known
   dimensions**. Gate-address counts and dimensional counts are not the same.
6. Do not turn every resident into a gate-policy expert. Some should care
   about dinner, repairs, a rival, the price of fish, or a family letter.
7. Keep the main Nazi faction plainly villainous, while allowing residents
   genuine disagreement over concrete humanitarian contact and document access.
8. Maintain different voices. Preserve the theatrical pirates, aspirational
   ninja student, harried station workers, and Oiler's practical manner.

## Validation / future work

- Run the existing all-sources Yarn compiler test after editing:
  `cargo test -p ambition_content --features ui --test content_it yarn_compile`.
- Playtest **each pre-quest and post-quest branch** in Pirate Port, Ninja
  Village and Orbital Station; verify that new optional choices never set
  flags or delay the expected hand-in paths.
- Visit the cove before and after defeating the Mockingbird. A defeated boss
  must still expose the Admiral's reward dialogue.
- Check portrait, speaker attribution, option paging and controller navigation
  with the new option counts (also with split-screen co-op).
- Review every added line against `anti-llmism-style-guide.md`: remove tidy
  universal morals, thematic metaphor chains, and interchangeable punchlines.
- Before adding reactive callbacks, verify the exact fact or occurrence key
  rather than branching on an invented flag. Treat future story revisions as
  normal content work; keep the source of truth in Yarn.

Related story sources: `docs/storylines/cannon.md`,
`docs/storylines/cannon-elevated.md`, `docs/storylines/cannon_expansions.md`,
`docs/brainstorms/the_fia_arc.md`,
`docs/brainstorms/good_llm_ideas/gates_morality_and_the_nazi_embassy.md`,
`docs/planning/game/gate-network.md`, and `docs/planning/game/bosses.md`.
