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
and reviewing them is `docs/authoring/anti-llmism-style-guide.md`.

## Threads seeded in this pass

| Thread | Locations / characters | Established or proposed relationship | Follow-up opportunities |
| --- | --- | --- | --- |
| Freight and gate economics | Ines, Bram, Jory, Agnes, Bo Lindqvist, Reyes, Pulse | Rings move goods quickly but capacity, power, and customs constrain travel; ships still work | A delayed delivery someone genuinely depends on; a choice between gate and ship |
| Passenger records and Castle Godwin | Ines, Okafor, the General, Fretjaw, Alice and the manifest clerk | Requests for names are surfacing from different offices; characters differ in how they handle them | Identify the requester, the person at risk, and the legal limits of sharing a list before any embassy storyline |
| Gnu-ton's gravity works | Wen, Rahimi, Reyes | Work in the Science dimension can be dangerous; residents have partial, occupation-specific evidence | Connect field anomalies and power budgets to a real authored area, without declaring the station incident Gnu-ton's fault |
| Shipment security and piracy | Admiral, Bess, Zorlack 65536, the Duelist | The old wars are over; Roo raids freight arriving in PVN, and Zorlack is guarding a shipment while awaiting news of jobs in the Business Dimension | Develop their own ambitions and the commercial rivalry independently of the Mockingbird |
| Kagero and Castle Godwin | Sato, Daichi, Hayashi, Duelist, Shadow Oni | Kagero is spoken of as independent of the village dojo; a few people have heard where she went | Optional recognition after her actual boss-fight appearance, keyed to a reliable event if one is authored |
| The wrong-list raid and safer maps | Alice, Bob, Oiler, the clerk, the gate janitor | The wrong address has visible paperwork and human consequences; the cartographers protect routes ([level 1](level-1.md)) | Trace one document through the institutions; keep the creator's unfinished sentence unresolved |
| A future friendship | (none in level 1) | Fia is not named in the intro: her full name is a later reveal, and the character may be reworked or cut ([level 1](level-1.md)) | Introduce Fia in person through repairs and trust, rather than an explanatory speech |
| Local character relationships | Lark, Bram, Jory, Ume, Ren, Daichi, Gus Pell | Residents know and annoy one another; a town has routines when the player is absent | Add small stateful callbacks if those interactions become playable |

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
- `cove.yarn` and `dojo.yarn` — the two sides of the shipment rivalry (originally the Mockingbird dispute, superseded November 27);
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
   reading and survey; the General's clearance; the interim cove encounter result.
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

## PVN revision — 2026-11-27

The current shipment is an interim contract for Zorlack 65536, the final
numbered leader of the Shadow Clan, while he waits for responses to job
applications in the Business Dimension. Roo is a former Gate Authority
recruit who settled in PVN after a severe crossing reaction and now raids
freight arriving there. The exact origin, succession event, and potential
future recovery remain open; the main-world dialogue retains the R. Badely
name rather than revealing Roo's first name.

The stolen song chest, shared pirate–ninja hunt and reconciliation after the
Mockingbird are retired as plot premises. The boss sequence is to be redesigned
rather than following *How to Kill a Mockingbird*. Do not invent a replacement
plot or use the shipment as a mandatory boss quest. For now the existing boss,
reward, and arena access remain mechanically unchanged; the journal and
post-fight dialogue describe only the interim encounter. The old
`pirate_treasure` quest key and `after_treasure` Yarn node IDs are retained for
compatibility, not as evidence of a stolen chest story.

## Expanded PVN and Authority pass — 2026-11-27

The Iron Bastion is planned for the Gate Authority dimension, with the annex
retained as its adjacent military facility. See [gate-network](gate-network.md#the-military-gate).
The dialogue anticipates the move without specifying the new entrance. The
General and Faction of Steel retain their established names. Their exact legal
relationship to the civilian Authority and the wartime chain of command are
open. Lines about cooperation between the garrison, Concourse and Manifest
Office are provisional characterization, not new clearance rules. The actual
map move remains a separate task.

The dialogue pass extends the shipment thread into Pirate Port and Zorlack's
Business College conversations. Roo remains in PVN; Zorlack can travel and
continue the existing contract while applying elsewhere. The local Ninja
Village has its own rank examination; its relationship to Zorlack's clan is
left open. Ordinary NPCs have local routines rather than universal knowledge
of the shipment. The contents, destination, raid result, job outcome,
and replacement cove boss remain undecided.

The existing `business_zorlack_met` branch provides another opportunity to
ask about the contract. No new facts or save flags track Zorlack's travels;
his existing appearances are retained. Reconcile placement and scheduling
when travel becomes an actual simulated event.

The Hall entries for these characters are updated for continuity. Existing
quest commands, node IDs, gate-known flags, and clearance conditions remain.
New anecdotes and local relationships are editable dialogue proposals.

## Validation / future work

- Run the existing all-sources Yarn compiler test after editing:
  `cargo test -p ambition_content --features ui --test content_it yarn_compile`.
- Playtest **each pre-quest and post-quest branch** in Pirate Port, Ninja
  Village and Orbital Station; verify that new optional choices never set
  flags or delay the expected hand-in paths.
- Visit the cove before and after defeating the Mockingbird. Roo's personal
  and shipment conversations must be available in either state; the defeated
  boss still exposes the interim reward dialogue.
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
