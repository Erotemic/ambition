# The defect is the SIBLING that disagrees, and the fix is already written next door

**Tags:** `fork-detection`, `diagnostic-method`, `agent-verification`,
`silent-degradation`

## The shape

N call sites answer one question. **N−1 agree, and they usually carry a comment
explaining why.** The one that differs is the bug — and the correct behaviour,
including its reasoning, is already written a few lines away.

⇒ **when you find a defect, do not design a fix. Enumerate its siblings and diff
them.**

```text
find the defect      →  "the prop's quality stamp is wrong"
enumerate siblings   →  every BoundSpriteQuality write in the file
diff them            →  two use `asset.tier`, one uses the requested setting
the fix              →  the majority, and its comment says why
```

## Eleven instances, one session (2026-08-09)

| the odd one out | its siblings | distance |
|---|---|---|
| the prop path stamped `BoundSpriteQuality` from the REQUESTED tier | two actor paths stamp `asset.tier`, the resident one | **40 lines, same file** |
| the shield ring read the SIM pose | two render overlays read the presented pose **and say why** | same crate |
| the death-drop `GroundItem` spawned session-scoped | three other `GroundItem` sites are room-scoped | same subsystem |
| one prompt exit never consulted `ActiveUiCues` | the other exit does | **20 lines apart** |
| `body_pixel_extent` implemented twice, disagreeing up to **1.30×** | — | two crates |
| the held item did not arbitrate the Attack press | two systems already arbitrate their slot | same crate |
| two catalog-join fns sat above the types they use | every other consumer sits below | crate boundary |
| `player_robot_v3`'s `block` row paints a **detached** shield ring, 1.77× wider and 1.45× taller than its idle | **36 of 37** sheets with a `block` row draw in front of the body at near-idle size | the asset tree |
| `drop_ability_pickup` never marks its drop `RoomScopedEntity` | the coin and health drops both do, and the coin's marker carries a **19-line comment** naming the black screen it caused | **same file** |
| `mary_o_1_2` authors a `goal_pole` block and nothing else | `mary_o_1_1` authors the shaft **and** the finial **and** the banner | one `.ldtk` |
| Mary-O's audio fragment never declares `world.coin.pickup` | Sanic declares that exact id **and pins it with a test**; the engine emits it for both | two demo crates |

⭐ **In five of the first seven the sibling carried a comment stating the rule**, e.g.
*"The DRAWN position, not the simulated one … a stand-in placed on the sim pose
shudders against a body drawn from the presented one."* The knowledge was
present, adjacent, and explained. One caller had not received it.

## Why it is worth a method rather than a habit

⛔ **The odd one out is invisible to a reader who starts from the symptom.** You
arrive at the broken site because that is where the bug is; nothing in that site
says *"forty lines up, this same question is answered differently."* Both look
locally reasonable.

⚠ **and every one of these degraded silently** — a prop still got a quality
stamp, a ring still got a position, a drop still got a lifetime. The wrong answer
is well-formed, so nothing errors and the symptom surfaces as a content complaint
months later.

## How to apply

**Two greps, at the moment you understand the defect and before you design:**

```sh
# 1. what is the QUESTION this site answers?  (which field it writes,
#    which resource it reads, which slot it claims)
# 2. who else answers it?
grep -rn "<the field or call>" --include=*.rs crates/ game/
```

Then diff the answers. ⭐ **prefer joining the majority to inventing a third
way** — a new mechanism makes it three sites disagreeing, which is how these got
here.

⚠ **and when the sibling has a comment, read it before overriding it.** Twice
this session the majority's comment named a constraint the obvious fix would have
broken: revoking a moveset verb would have removed the touch button, and moving a
`SpritePosedBody` constructor up would have split a grant from its retraction.

### ⭐ The eighth instance generalised the method past code

The shield-bubble bug survived **six** wrong mechanisms, every one of them an
expression in the renderer, because the investigation never left the code. What
killed it was a **population query over the assets**: *of the 37 sheets that have
both an `idle` and a `block` row, how does each one's `block` differ from its
`idle`?* One answer was 1.77× wider and 1.45× taller; the other 36 clustered near
1.0 or grew in one axis only.

⇒ **the siblings do not have to be call sites.** Anything the project has many of
— spritesheet rows, `.ron` archetypes, LDtk entity instances, manifest entries,
config blocks — supports the same query, and the outlier is found by *ratio to
its own baseline*, not by absolute value. A sheet being large means nothing; a
sheet being large **only in the row that misbehaves** is the finding.

⚠ **and this is the case where reading the code cannot work at all.** The engine
was correct to the last decimal — `pos == kin.pos`, offset `(+0.00, +0.00)`,
one entity, no duplicate texture draw. There was no wrong expression to find.
⛔ **when a measurement says every code path is clean and the artefact is still on
screen, the population to enumerate is the DATA, not the callers.**

### ⛔ The sweep produces FALSE accusations by the same mechanism — check both ways

Running the sibling query repo-wide found one real defect (a drop function
missing its lifetime marker) **and one innocent file with an identical
signature**. The innocent one spawned a component declared
`#[require(RoomScopedEntity)]`, so the marker was on the entity without the name
appearing anywhere in the file.

⇒ **a grep answers "is this name written here", never "is this true of the
entity".** In an ECS with required components, inherited fields, defaulted
config, or `Deref` re-exports, those two questions come apart — and here they
disagreed for **2 of 5** candidates in a hand-narrowed population.

⚠ **the second step is not optional and it is cheap**: for every component in the
suspect bundle, check whether anything `#[require]`s the thing you claim is
missing. Confirming the *real* finding needs it too — the ability drop's absence
is only a defect because none of its four components pulls the marker in.

⭐ **the strongest version of this is deletion.** After fixing the prop stamp, its
helper `active_sprite_scale` had no callers left — because its ONLY caller was
the site doing it wrong. Deleting it made the rule structural: there is now no
way to reach for the requested setting from a presentation binder.

## Related

* [`a-capability-with-no-adopters-2026-08-09.md`](a-capability-with-no-adopters-2026-08-09.md)
  — the same disease with N = 1: the correct mechanism exists and has no callers
  at all.
* [`one-question-two-checkers-only-the-first-runs-2026-08-08.md`](one-question-two-checkers-only-the-first-runs-2026-08-08.md)
  — two siblings where one is unreachable, so fixing it repairs nothing.
