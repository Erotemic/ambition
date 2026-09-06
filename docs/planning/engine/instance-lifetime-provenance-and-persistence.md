# Instance lifetime, provenance and persistence

**State:** DISTILLED — the provenance/lifetime foundation is implemented; the
remaining product semantics are owned by open-world, custody, and reconstitution
plans.

## Current model

Authoritative instances have explicit provenance/lifetime semantics rather than
being classified by which constructor happened to spawn them. Important current
vocabulary includes session/room scope, authored placement provenance, runtime
spawn identity, occurrence/disposition facts, and persistent ledgers used by a
fresh construction/restore path.

The durable lesson is:

> existence, residency, rollback history, durable occurrence identity, and
> presentation are different lifetimes.

A relationship may cross a durable save/load horizon only when the durable road
can restore the authority for that relationship. For example, item custody may
be persisted because item inventory/custody has a durable reconstruction path;
transient possession of an actor is not made durable merely because both happen
to project through a generic live relationship component.

Do not infer a universal `InstanceId` requirement from shared vocabulary.
Domain-specific actor/item/world-object IDs remain acceptable until common
operations demonstrate a real reusable core.

## Remaining owners

- [`construction-and-reconstitution.md`](construction-and-reconstitution.md) —
  which populations are retained/reconstructed at session/room/replay/restore
  boundaries.
- [`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md) —
  resident versus nonresident world state.
- [`item-custody-and-accounting.md`](item-custody-and-accounting.md) — item
  occurrence, inventory and physical custody.

## Still-open questions

- terminal versus resettable occurrence/tombstone semantics;
- stable identity required across a fresh process versus identity that may be
  deterministically regenerated;
- persistent relocation of actors/items away from authored home placement;
- world/per-owner uniqueness without conflating identity with definition;
- how much provenance is product state versus diagnostics.

The implementation campaign and dated measurements that established this model
remain recoverable through git history; they are no longer active planning.

## ⭐⭐⭐ A CLAIM RELEASED ONLY BY THE PATH THAT TOOK IT — three player-visible instances in one day (2026-09-06)

Three subsystems, three owners, one shape, all three reported or found the same
day. Every one had a `release` a reader would tick off as present.

| owner | claimed by | release reachable when? |
|---|---|---|
| portal dependent hide | the body loop's hide | ⛔ never — no marker, no release branch at all |
| `CUT_ROPE_MUSIC_OWNER` | `reset_cut_rope_attempt_on_replay` (a DEATH is a replay) | ⛔ only inside that same one-shot |
| `SCRIPT_MUSIC_OWNER` | an `EncounterEffect::SetMusic` beat | ⛔ only while a live script emits `SetMusic(None)` |
| `BOSS_MUSIC_OWNER` | the boss-music system | ✔ every frame — it says so in its own comment |
| `DEATH_MUSIC_OWNER` | mary-o's death window | ✔ if/else over a live query |

⇒ **AN EFFECT FIRES ONCE; A DESPAWN FIRES NOTHING.** The presence of a `release`
call proves nothing. The question is **on which frames it is REACHABLE**, and
specifically whether it is reachable on a frame where nobody claims.

⛔ **WHAT MADE THEM PLAYER-VISIBLE RATHER THAN UNTIDY:** the portal one latched a
hit-flash mesh hidden *for the rest of the session* (its update path states
visibility "stays `Visible` permanently", so nothing ever put it back); the two
music ones win over room music, because `EncounterMusicRequest::desired_track` puts
the priority tier above it — so a stale claim does not linger quietly, it plays a
boss's intro in every room the player visits.

⭐ **THE CORRECT SHAPE WAS ALREADY WRITTEN IN THE SAME CRATE, for a different
owner**: *"This system has no run condition, so it reaches the 'no boss is
fighting' arm on every frame of every game."* ⇒ When one member of a family is
right, read its comment and apply it to the others rather than re-deriving. Two of
the three fixes are that sentence, moved.

⚠ **A SYNTACTIC GUARD CANNOT CATCH THIS — checked before building one.** All five
owners *have* a matching release; the defect is REACHABILITY, not presence, so a
"every claim has a release" grep is green on all three bugs. That is why this is a
recorded RULE and not a new checker.

⚠ **And the release must be OWNER-SCOPED.** The obvious wrong fix — clearing the
tier outright — silences whoever legitimately holds it, and that crate's comment
records having shipped exactly that once: *"a demo with no bosses at all could not
hold priority music for a single frame."*
