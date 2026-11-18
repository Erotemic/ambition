# Gate frontiers, the armistice, and the tone of Ambition

**Status:** Working brainstorm, 2026-11-17. Not accepted canon or an implementation task.
**Source:** Jon's statements from the design conversation, plus short assistant suggestions.

## How to read this file

- **USER (verbatim)** blocks reproduce Jon's words exactly, including tentative ideas, false starts, spelling, and capitalization. Give them more weight than the assistant bullets.
- **Assistant synthesis (provisional)** means a suggestion, not an accepted decision. Do not silently promote it to canon.
- Later user statements can revise earlier user statements. A question, "maybe," or "perhaps" is still undecided.
- `docs/storylines/cannon-elevated.md` is accepted canon. This brainstorm does not edit or override it. Current code states implemented behavior, even when these proposals differ from it.
- Do not invent fixed answers for open questions. In particular, **the General has no accepted personal name**, and **Hobford is a working name**.

## 1. A recently connected rural dimension and the survey robot

**USER (verbatim):**

```text
So gates have been established in some dimensions for decades. But not all dimension. E.g. in hobford the creator has recently moved there as a recently connected dimension and they are building robots that work well in the new environment, scout is one of them. Maybe scout was made to help alice or was going to be offered to help alice - perhaps she doesn't know that yet.
```

**Assistant synthesis (provisional):**

- Gate travel is old infrastructure in some dimensions. Other dimensions are recent additions.
- The Creator is a newcomer to the newly connected rural area. Scout is one of the robots made to operate in its environment.
- Scout's intended role can inform its design without fixing Scout's future or making it a chosen one.

**USER (verbatim; later refinement):**

```text
The creator was probably briefed that Alice and Bob were coming to do map work, maybe he was directed to make Scout to help her and Bob, or maybe he just did it because he builds things, maybe we don't find out. But I don't think they knew each other. Err actually that doesn't make sense because Alice needs a reason to go to the mountain workshop. So maybe he was directed to build a scouting robot and Alice knew that's where she should go.

Hobford is a fairly primitive town, which is rapidly adapting since the gate first opened maybe 6 years ago now. Time enough where it's not new, but residents remember life before, and displacement and disruption is still a major factor.
```

**Assistant synthesis (provisional):**

- Alice and Bob come for map work. Alice knows to visit the mountain workshop for a scouting robot. The Creator expects the survey team, but they have not met.
- Whether the Creator was commissioned, directed, or built Scout on his own is open. The story need not answer it.
- The settlement had a life before its gate opened about six years ago. Locals recall that life. New trade and newcomers bring opportunity, displacement, and disruption.
- Avoid treating the town as an empty frontier. It has its own residents, work, history, and government before contact.

## 2. Secret addresses, illicit arrivals, and the opening raid

**USER (verbatim):**

```text
Hmm, that's a good question. Getting the the politics of opening a gate to a new world is tricky. I think the gate authority generally doesn't let anyone know any gate address except for the location to the central hub. The gate address information is top secret. Yet... sometimes there are breaches of security. That's how the goblins came to Hobford. Sometimes an unauthorized gate access happens, especially when a new world is connected to. There might also be hidden gates that even the authority doesn't know about that allow for illicit black market travel, although we will need a mechanism to make these gates more portable, because if they were in one place, I imagine the gate authority would be able to track them down well enough. The nazis are certainly behind some of this, although we probably don't see real nazis until act 2.
```

**Assistant synthesis (provisional):**

- The official public network routes through a central hub. The Authority restricts destination-address knowledge.
- Secret, stolen, and unregistered routes still exist. The Authority has an incomplete map and imperfect enforcement.
- Some goblins entered the rural dimension through illicit access. Do not imply all goblins are criminals or Nazi agents.
- Nazi involvement in clandestine travel may be felt in Act I without showing actual Nazis before Act II.

**USER (verbatim):**

```text
There's a problem though, because if the nazis hire goblin thugs to go ransack a lab with a particularly valuable technology (maybe the unsaid truth that we don't fine out about until much later is they want to capture one of the bossofthegame's agents to work for them?) - back to the point: you can't give a thug a real address. They can tell it to anyone. But maybe you can give them a key. A zero knowledge proof that works in the gate to open a specific location. Ugggg, but there's a problem. The address lights up on the gate, and it looks super cool. I don't want to lose that.
```

**Assistant synthesis (provisional):**

- A contractor should be able to travel without receiving a reusable secret address. A destination-bound key is one possible answer.
- Zero-knowledge methods are worth exploring, but a working mathematical protocol is not yet selected. Single-use enforcement and what the gate reveals remain real design problems.
- Preserve the visually readable, illuminated address-glyph dialing sequence. Do not erase this feature to solve credential secrecy.
- The idea that the raid sought one of the bossofthegame's agents is a **possible late reveal**, not an established motive. Keep the actual employer and target open.

## 3. The early shortcut and the dead traveler's key

**USER (verbatim; identifies a constraint):**

```text
But that also breaks the speedrun route where the robot runs to the first gate and dials the address to the bossofthegame...
```

**USER (verbatim; subsequent revision):**

```text
Hmm, maybe we do this. We can make the speedrun route a bit more convoluted. We can have a one time access key be a secret item that exists and is accessible in the early game if you do a trick. (maybe a tight pogo). That gives you the creds to have one shot to dial any address you want. That item might even be a real item you want to get later in the game in a potential story arc, so players learn about it much later.
```

**USER (verbatim; further refinement):**

```text
Nah, I think it's a key from a dead illicit traveler tucked away somewhere.
```

**Assistant synthesis (provisional):**

- An illicit traveler died in a hidden early-game location. The body holds a **one-use key that permits one manual dial to any known address**.
- A difficult movement trick, possibly a tight pogo, can make the item accessible from the start. A knowledgeable speedrunner can use it to dial the final destination.
- A normal playthrough may introduce this kind of key much later, possibly through a quest. Do not require that quest if the player already has a valid key.
- Do **not** replace the dead traveler with a forgotten official maintenance key. The corpse's history need not connect to the main plot.
- Preserve a hard, skill-and-knowledge-based shortcut. Whether specific gates require this credential is an open future rule, not current implementation behavior.

## 4. The Nazi war, armistice, the General, and the embassy debate

**USER (verbatim):**

```text
what can we do about making the gate keys interesting? it'd be good if they were mathematically plausible. better if they were mathematically implementable. some zero knowledge system might be really interesting. but actually maybe that's not the most important detail to decide. I think what I'm interested in is having a conflict between the army led by the general and the Nazis but I'm not sure how to work that in correctly. one of the ideas that I liked was people being uncomfortable about allowing Nazi embassies in their dimension and they're being some sort of debate about that. so maybe there was a war between what is now the gate authority and the Nazi dimension but they're at some sort of truce? but maybe the Nazi dimension had some sort of control over the gate which is why there's the elicit gate keys that are available sometimes?
```

**USER (verbatim; strong choice):**

```text
stalemate for sure. maybe gates by their nature are unlocked but the gate authority maps then out and installs restrictions. but we still haven't solved the problem of portable hard to find gates. or maybe bootleg gates. could have them of different sizes to allow ships maybe. what's the total number of addresses possible which the search space that we're dealing with? 12 possible symbols I think maybe 13 and you can use them up to four times?
```

**USER (verbatim; later refinement):**

```text
I think the Nazis did and the war in a stale made sort of but obviously the gate authority or the force that became the gate authority ended up in a slightly stronger position. just not strong enough to fully break the fascists. also note that Hobford might not be the right name. it's fine but it's also very llmy.

we are going to have flying pirate ships so ships moving through a gate needs to be a thing. there's also a very real possibility I'm going to want to work in a time traveling Castle, which also functions as a ship.

as for the purpose for what they were constructed: they were constructed to make gameplay. I think the idea of me treating me as a villain of some sorts is something I want to move away from. we're probably going to end up breaking the fourth wall at a few points. I do want a somewhat serious fantasy sci-fi universe but also something that's completely ridiculous. think: adventure Time. very concrete rules but also entirely ridiculous and fever dreamy
```

**Assistant synthesis (provisional):**

- The war ends in a **stalemate**, with the coalition or predecessor of the Gate Authority in a somewhat stronger position. It could not defeat the Nazi regime outright. Avoid retelling this as a decisive coalition victory.
- The General's army has a real reason to resist Nazi expansion. The General can be pompous or reckless without being wrong that the threat is real.
- The Authority could grow out of a wartime coalition or an older transit institution. Its exact origin, treaty powers, and military relationship remain open.
- A proposed Nazi embassy should provoke a concrete political debate. Distinguish limited contact for prisoner exchange or family reunification from an embassy that could support spying, intimidation, or access to refugee records.
- Nazi regime involvement in illicit credentials or old gate infrastructure is plausible but not yet a fixed historical mechanism.
- **The General's proposed personal name is rejected. Do not use “Percival Brass” as a selected name.** Keep the character unnamed for now.
- **Hobford is provisional**, for the town and possibly for the wider dimension. Do not entrench it as a final name.

## 5. Ancient gates, unreliable replicas, and smuggling infrastructure

**USER (verbatim):**

```text
it might be that the portable gates are not even something that operates for very long. they might have a high chance of not even working, or break themselves in the process they're not very stable. that could be your dude shoddy craftsmanship, but also even the best crafted portable Gates probably don't work very well. the gates were always there but nobody really knew how to use them. and they were already there because the boss of the game created the world that way. they made of an odd material the good Gates. the best science has replicated it somewhat and they could build a gate that stable for maybe a few cycles. oh but that introduces a problem though because you sort of need stable illicit Gates Don't you. it could just be that they're not mapped out very well. you have some of these gates with 16 digit addresses that are in sort of strange places. and maybe it's also possible to move a gate if you need to.
```

**Assistant synthesis (provisional):**

- The bossofthegame placed original gates when constructing the world. Their material is hard to reproduce. Civilizations discovered how to operate them long after the gates existed.
- Original gates are highly reliable. Modern replicas, even well made ones, may survive only a few uses. Cheap bootleg versions may fail to open or destroy themselves.
- A **stable illicit route** need not rely on a stable portable replica. It can use an undiscovered, stolen, or relocated original gate.
- An original gate may be movable without being easy to carry. A candidate rule is that an address follows the ring when it moves. This is **not decided**.
- The Authority cannot monitor all gates it has not found, especially dormant rings with long, hard-to-guess addresses. Secret sites and limited operating times can also hinder discovery.
- Gates can have different aperture sizes. Some must fit flying pirate ships, and very large gates may be needed for a castle that also acts as a ship.
- The larger the sustained flow of cargo and troops, the harder an illicit connection is to conceal. This can help maintain the military stalemate while allowing raids and smuggling.

**Current address math (implementation fact, not a new lore decision):**

- `LEGALLYDISTINCT` defines **15 distinct selectable chevrons** (11 distinct letter labels). An address has at most **16 chevrons**, and each chevron can occur at most **4 times**.
- This permits **6,719,427,824,596,046,550** nonempty sequences of lengths 1 through 16, or **6,265,680,743,637,315,000** at exactly length 16.
- These are possible sequences, not a count of physical gates. A short known address does not gain security from the huge total search space.
- Source: `docs/planning/game/gate-network.md` and `crates/ambition_platformer2d_world/src/rooms/gate_network.rs`.

## 6. Gameplay purpose, creator role, and tone

**USER (verbatim; repeated here because it constrains these proposals):**

```text
as for the purpose for what they were constructed: they were constructed to make gameplay. I think the idea of me treating me as a villain of some sorts is something I want to move away from. we're probably going to end up breaking the fourth wall at a few points. I do want a somewhat serious fantasy sci-fi universe but also something that's completely ridiculous. think: adventure Time. very concrete rules but also entirely ridiculous and fever dreamy
```

**USER (verbatim excerpt; naming direction):**

```text
Also I don't like the general's name.
```

**Assistant synthesis (provisional):**

- The gates' ultimate purpose is to **make gameplay**. Do not replace this with a hidden utilitarian or sinister master plan.
- Move away from casting the developer / world creator as a villain. The bossofthegame can still be a character and an encounter without making all suffering a deliberate act of the creator.
- Fourth-wall breaks may occur. They need not erase the autonomy or stakes of ordinary inhabitants.
- Aim for an Adventure Time-like coexistence of coherent local rules, serious fantasy/science fiction, and fever-dream absurdity.
- A flying pirate ship, a ship-sized ancient gate, and a time-traveling castle can coexist with a serious war and daily life. Do not force a single solemn explanation for every ridiculous detail.
- If time travel enters the game, its constraints can be developed when gameplay needs them. Do not lock in a theory now.

## 7. Open matters and code discrepancies

- **Name:** Choose a better name than Hobford later. Choose a name for the General later. Do not reuse rejected assistant proposals.
- **Politics:** Decide the armistice terms, who the Gate Authority answers to, and how it relates to the General's army. No final war dates or commander biography are set here.
- **Security:** Decide how official controllers restrict dialing, how credentials can hide destinations while preserving the lit glyph sequence, and how one-use keys work. A mathematical or zero-knowledge design is desirable but not required yet.
- **Gate logistics:** Decide whether old gates can be moved, what it costs, how ships cross, and what fabricated gates can safely do. Stable illicit routes via unregistered original gates are a working proposal.
- **Story:** The raid's true employer and target, the Creator's exact commission, and the Nazi connection to any particular smuggler are open.
- **Engine reality:** `docs/planning/game/gate-network.md` currently says any valid address can be dialed and no faction rejects a dial. This is not yet a key-gated system. In the content catalog, `start_known: All` is a development setting, `LEGACY` is the hidden final-boss address, and `TILLAGE` is the current Hobford address. The catalog still says Hobford's hill country joined last year; this brainstorm suggests about six years for the town's connection. Reconcile later; do not silently change implementation to match this note.

**Related documents:** `docs/storylines/cannon-elevated.md`, `docs/brainstorms/good_llm_ideas/gates_morality_and_the_nazi_embassy.md`, `docs/planning/game/gate-network.md`, `game/ambition_content/assets/data/gate_network.ron`.
