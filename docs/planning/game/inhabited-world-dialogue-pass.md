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

## World dialogue pass (2026-11-27)

This pass follows the PVN and Authority dialogue changes. It keeps the
current encounter and quest logic. It does not move rooms or add quests.

- Fantasy: the Barrow King now uses imported junk. Residents report the
  change in the ghosts' behavior. The king's earlier character and the
  method he uses to animate the objects remain open. Sir Cumference came
  from Math Town. The local wizard uses imported goods as well as spells.
- Science: Nicholas and Coulomba Tezla replace the old display names. Keep
  actor IDs, Yarn node IDs and asset paths. The mine and town have a
  repair-work connection. Farah gives a hypothesis about the crusher;
  her explanation is not a confirmed cause.
- Noodle: Ramen Nujan came from the Math Dimension and follows the
  Noodle faith. His scarf is made of noodles. His visit to Brothlehem
  and his notebooks are dialogue proposals.
- Frontier: Gallus was a lawman and is now an outlaw. Residents still
  have to deal with his men and his toll. No change to the corral fight.
- Castle Godwin: the castle is a local command under the Supreme Leader.
  The capital is elsewhere. Staff, prisoners and townspeople have their
  own concerns. Do not assume the castle staff all oppose the regime.
- Art: keep disagreements about authorship and give residents other
  subjects to discuss, such as painting, music, work and visitors.
- Automaton, Math, Neural and Magic: correct misleading explanations
  and remove repeated rhetorical formulas. Keep the current puzzles,
  routes and character jokes. Erdish starts his conversations with
  "My mind is open."
- Primeval, Orbital and Unfinished: revise routine work and travel
  dialogue. Preserve the existing quests and local routes.

New personal anecdotes, dates, minor trade relationships and occupations
are proposals in playable dialogue. No canon file is changed. The Hall
historical gallery and History Dimension are not a complete fact audit.
The old Mockingbird encounter and military tower placement remain work
for later passes.

Full Stack plays bass, and the bossofthegame stands on his shoulders.
Hylbert is working to increase the spectral gap in the haunted hotel.
These lines do not change the band animations, the climbable instrument,
or the hotel encounter.

## Follow-up: Hall conversations and cartography

The Hall's ordinary residents should talk about their work, interests, and
relationships, not give interchangeable complaints about their exhibits.
Keep its explicitly meta characters, transformation demonstrations, and
debug interactions. Hall dialogue is not a canonical timeline of completed
quests; avoid declaring quest outcomes or new geography there.

Capital restricts cartography and the distribution of directions; sharing
a map without permission can lead to arrest. The dialogue describes that
restriction without implementing enforcement or the later Alice/Bob story.

The new anecdotes are provisional; the pirate shipment's contents and the
Mockingbird replacement remain undecided.

## Follow-up: encounter-independent dialogue and technical precision

Hall conversations should not assume that a visitor is Scout, has already
fought a character, or has completed a quest. Keep intentional game and
gallery meta jokes, but give other characters specific questions about their
work and surroundings. Match Hall and field dialogue where characters recur.
Technical concepts may be personified; avoid making the personification a
false explanation of the actual concept (for example, soft attention is not
necessarily exclusive to one item, and a runtime Busy Beaver maximizes steps
rather than output). This pass changes speech only; the new small anecdotes
are provisional and no mechanics or locations are changed.

## Follow-up: recurring cast and continuity

Keep the existing rural opening's tasks and player choice order. Alice and
Bob know each other's surveying habits and work together; do not turn their
relationship into a player-selected or discussed romance. Field conversations
should not assume the later cryptography arc has already happened. Eve may
listen without necessarily intending harm; Mallory changes messages, and
signature checks depend on verifying the intended sender's public key.
Dennis builds Scout, and the locals know him as Dennis. Oiler's repair response
must make sense after either offered answer. Oiler teaches rather than belittles
an inexperienced repair customer. Fia's early kindness should be sincere; her
later strategic behavior should develop in the story rather than be announced
in idle barks. The Hall transformation remains a preview, not an explanation
of the origin of her void forms. The Business Dimension's distribution rules
foreshadow the cartography conflict without asserting an arrest already happened.
New small anecdotes are provisional; no progression, map, or character mechanics
are added in this pass.

## Follow-up: boss personalities and aftermath

PCA is a cultured, complacent duelist who enjoys being surprised; his combat
voice should sound like the character in his arena, not a malfunctioning
machine. GNU-ton is an accomplished scientist whose judgement can fail. Give
GNU and GNU2 their own concerns; distinguish the scholar from his mounts and
leave their exact origin, GNU2's eventual freedom, and GNU-ton's proposed
stand-down or redemption unresolved. Keep the existing prelude and fight
choreography. Do not interpret an ordinary Hall conversation as a fight result.

The existing encounter-cleared branches in Dry Dock and the Elemental Mine
can show local consequences without claiming an entire political or industrial
problem is solved. Gallus's defeat leaves the toll office and work schedule
in question; no successor or post-fight survival is established. Mother Lode's
shell breaks, but the listening core survives. Its request to leave the living
seam alone is not an implemented agreement; Inez's temporary suspension of the
crusher does not decide the machine's final fate. The Barrow King is a menace
with newly effective animated junk; his earlier moral character is not settled.

Keep the Mockingbird replacement open and preserve the Void Lizard's truce.
The Hall boss conversations and catalog barks are available now; additional
in-world introductions, outcomes, and persistent reactions require encounter
and placement work. In particular, do not add an unreferenced GNU-ton
redemption scene or Gallus lookout node and call it implemented. New anecdotes,
individual reactions, and incidental personal details are provisional. No
encounter IDs, commands, progression gates, or arena geometry changed here.

## Follow-up: revisitable topics and meaningful choices

A first-meeting flag should normally change an NPC's greeting, not consume
all of the questions the player did not ask on that visit. Move ordinary
informational topics outside the first-meeting conditional, keeping the
existing flag write in its original arm and preserving the unanswerable
stop. Do not apply this blindly to quest hand-ins, purchases, rewards,
confessions, encounter transitions, or choices that represent one-time
story decisions. Keep the conditional route hints available while the
relevant quest or encounter state allows them.

For the Authority information desk, gatekeeper and registrar, Neighbour
Moore, the Turing machine Head, Marguerite, Zorlack at the college and the
Frontier station agent, use a dedicated repeatable topic node with an
explicit exit. Return to the topic menu after an answer, rather than
replaying the greeting and setting the met flag again. The shorter
conversations retain their one-answer structure but offer the same
informational topics on subsequent visits. These patterns follow the
existing conversation style of PCA and Roo. Marguerite now asks whether
the player intends to keep or distribute a map; her answers describe
existing story restrictions and do not implement any licensing mechanic.
Bob can explain his survey after handing it over, the valley gatekeeper
can repeat the freight-track clue, and Ruth and Pearl can repeat their
route hints. Keep the Mother Lode agreement and the shipment raid open.
New personal details are provisional.

The dojo leader currently offers "Open the arena door" as the only
non-question exit in several nodes. Determine whether the door is opened
by the selected choice or by completion of the conversation/placement
before adding a distinct leave-without-opening option. This pass leaves
those nodes unchanged: there is no explicit door command in the Yarn
body and the map submodule is not present in the supplied source archive.

## Follow-up: regional relationships and differing perspectives

Use existing events as shared points of contact, with each NPC speaking from
what they have actually seen or done. A player should be able to hear the
conversations in either order. Avoid introducing mandatory fetch steps,
requiring a preceding conversation for an unguarded line, or treating a
character's opinion as a settled world fact.

- Hobford: imported grain affects both the mill and the inn; the trader and
  ranger have different evidence of animated scrap on the moor. The wizard
  and knight have their own approaches to the barrow. Preserve the opening's
  existing Dennis and goblin encounter conditions.
- PVN: Jory objects to berth charges, Bram has maintenance expenses, and Lark
  sees them together at supper. Tomas supplies Agnes, who in turn supplies
  Lamplugh's lamp oil. Nell trades salvage with the lighthouse keeper. None
  of this decides the current shipment's contents or the raid's outcome.
- Ninja Village: rooftop training affects Ume; Daichi and Ren have different
  approaches to practice. Hayashi knows about Zorlack's shipboard training
  needs. No exam requirements or ranks change.
- Science: Teodora's logs support Priya's clock measurements; Pell and Mina
  share a supply chain. Inez is responsible for production, Ellis for the
  carts, and Farah and Tamsin for tracing the vibration through samples.
  The deep seam remains a problem to investigate, not a solved agreement.
- Frontier: after the Gallus encounter, Dolores is reluctant to spend the
  toll money without authority, Ruth has to face her workers, and Mae sees
  the night crew as customers. These lines remain in existing cleared
  branches. No succession, wages, or toll changes are implemented. Augusta
  and Hettie discuss delayed freight; Ida and Magda share a laundry routine.
- Authority: a passenger scheduling request affects station freight, and
  civilian manifest staff need information from both signals and sentries.
  The tower relocation and gate scheduling mechanics remain separate work.
- Art and Math: let Frida, Peckasso, and the Curator disagree about credits
  and exhibition practice. Furrier wants measurements while Hylbert is
  responsible for safety. Emmy and Erdish have a small recurring cafe habit.

The new anecdotes, supply arrangements, fee structures, and personnel
interactions are provisional dialogue-level worldbuilding. No new flags,
rewards, placements, encounter outcomes, or progression rules are added.

## Follow-up: voice, comic timing, and incidental dialogue

Preserve jokes that already work, especially character names, deliberate
repetition, and game-specific references. Avoid adding a second explanation
of a punchline. Do not give every NPC the same mock-officious voice. Let
Roo speak like an experienced captain, Salt Annet like a working sailor,
Zorlack like a disciplined clan leader who wants a new career, and Bess
like a gunner concerned with her next shot. Keep their different rhythms
rather than intensifying the pirate dialect.

Business satire can use a concrete transaction or administrative problem
rather than an extended list of buzzwords. PCA enjoys his superiority and
his challengers; keep his self-introduction consistent across arenas.
Shorten the puppy slug stage directions to visible actions, without
assigning them human inner monologues. In technical conversations, give
the speaker a reason to explain the particular idea, such as Donna's
measurement or Curry's glass of soda.

These are line-level revisions. Choice labels, commands, conditionals,
node names, existing encounter outcomes and progression are unchanged.
Minor details about people, equipment and business practices are
provisional, not implemented gameplay or settled canon.

## Follow-up: information pacing and discovery

Hobford can be explored in any order. Alice saw the goblins pass her survey
station; she can infer where they intended to go, but should not assert
that they are still waiting at the siding. Bob's camp remains worth moving
whether or not the siding fight has already happened. Keep the raid's
actual work order (the old mill, not the barn) at the freight siding, and
let the Drain Market clerk describe the contents and discrepancies of the
paperwork rather than claim to know the cargo or the true employer. The
Registrar's missing entry is an observation about the copy at hand, not
proof that a person never crossed a ring. Keep the denial and correction
clues, and do not establish who commissioned the raid in this pass.

Gate information should match `gate-network.md` and `gate_network.ron`:
connections are bidirectional, remain available for further travellers
until idle timeout, and addresses need not be listed to work. LEGAL is the
Authority capital; TESTING is the sandbox hub. Ring Seven's breaker
supplies power, not a restriction to TANGENT. The General's clearance
still controls the physical military checkpoint, not the ability to dial.
Distinguish the Authority's schedules and paperwork from engine constraints.

A few existing optional route conversations now teach addresses using the
existing `gate.known.*` flags. Okafor gives TANGENT when directing the player
to Canter; Teodora gives SCIENCE and GALACTIC when explaining the return
journey; Augusta gives LEADENSADDLE when describing Dry Dock; Roo gives
ALDENTE when discussing his desired visit; Zorlack gives DEALINGS when
discussing the business college. These are existing catalog addresses;
knowledge adds directory entries, not travel permissions. Development's
`All` start policy means the extra commands are redundant on a fresh dev
save. They become useful if `Discovered` is selected for release. Neither
policy nor the catalog is changed. No hidden address, including the final
boss address, is newly exposed. Keep the actual final encounter reveal out
of ordinary town conversation.

The Ninja Village's undelivered letters and the Authority's armistice are
reported from local perspectives. Frontier's reunion does not assume the
player fought the bear when asking about the mechanic. New minor wording
and address-sharing choices are provisional. This pass changes two Bob
response labels but retains their targets, all existing commands, all
conditionals and all quest transitions. No placements or encounter
outcomes are added.
