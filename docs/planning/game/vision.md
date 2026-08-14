# Game vision

Ambition the *game* is the **flagship and primary product driver**. It is built
on the reusable engine rather than privileged engine shortcuts, but the engine
program is judged partly by whether Ambition becomes better to author, extend,
ship and play. Product needs should expose reusable capability; they are not
subordinate to an abstract framework.

---

## North star

> Every upgrade a theorem, every boss a failed objective function, every biome a math
> world model.

The player is a small embodied **AI** without a clear purpose. Its abilities are
**mathematical theorems**; the world's pressure is **uncertainty**, not a tidy moral.

> The player is not told they are special. The player is only told to move.

> Do not let the player being an AI solve ethics. The world should treat uncertainty
> as the central pressure.

## The spine (intro arc)

Wake in a basement lab → escape a "wrong-list" raid through maintenance tunnels →
learn that humans travel through **stable gates** while AIs exploit **ripples** →
enter **The Kernel**, a transit hub where rooms are *both* gameplay tests *and* lore
evidence. The first emotional arc is the **Alice / Bob** cryptography questline —
about trust and communication.

**One beat = one room, one verb, one lore fact, one state change.** Build the spine as
a chain of such rooms; defer everything not on it.

> Every lore reveal should correspond to a new movement route, quest state, room
> transform, boss unlock, or dialogue consequence.

## The three pillars, made concrete

- **Upgrade = theorem.** Cryptographic protocols become *traversal*: a Blink Key, a
  Public-Key Door, a Commit-Reveal Platform, challenge-response movement. Learning the
  protocol *is* the unlock.
- **Boss = failed objective function.** A boss is a flawed optimizer the player
  exploits and out-learns — the Mockingbird (mimics/steals your moves), the Clockwork
  Warden (reads your patterns), the Perfect Cell-ular Automaton. See
  [`bosses.md`](bosses.md).
- **Biome = math world model.** The Kernel basement is *diegetic debug* — a
  maintenance layer where mechanics are inspectable; faction spaces teach different
  world models; the Gate Stack contrasts legal vs illegal routes.

## Strong ideas (the kept backlog)

Triaged from the idea index — each hooks to existing engine primitives (LDtk rooms,
quest steps, save flags, NPCs, portals), so they're content, not new systems:

- **Handshake quest** — Alice → Bob → verify (quest steps + NPC flags).
- **Eve observes / Mallory modifies** — interceptor contacts and route choices alter
  dialogue and message integrity (verification vs speed).
- **Ripple** — a small, AI-only portal distinct from stable gates, with its own
  audio/visual signature.
- **The hub as a playable table of contents** — doors invite experimentation; quest
  gates arrive later.
- **Courier chains across incompatible routes** — message integrity under different
  world models.
- **Commit-Reveal platform**, **Public/Private-Key doors** — crypto-as-traversal.

**Defer the faction bloat** (pirates, ninjas, GNU-ton, tech-bros, military tower):
commit to the Alice/Bob handshake + Eve/Mallory + the Kernel hub first. Add a new idea
here when it's still rough but should be findable; archive one only when it's
misleading or superseded.


## Multiplayer direction

Ambition should eventually support solo, local couch co-op, online co-op and
mixed local+remote parties through one body/control architecture. Presentation
may be shared-screen, fixed split-screen or adaptively split when participants
separate and merged when they regroup. When the game mode allows independent
exploration, participants may occupy different rooms rather than being forced
into one globally active room.

Story/dialogue/save/join policy remains Ambition product design; participant,
transport, world-residency and view-index machinery belongs to the reusable
engine. See [`multiplayer.md`](multiplayer.md) and
[`../engine/multiplayer-and-multiview.md`](../engine/multiplayer-and-multiview.md).
