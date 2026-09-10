# A moved directory is a citation broken in silence

**Status:** open row, diagnosis complete. **Not a proposal to wire anything.**
Filed 2026-09-10.

## The mechanism

`scripts/check_planning_citations.py` resolves **symbols**. A doc comment that
names a **directory** in prose —

    /// `abilities/traversal/{blink,dive,mark_recall}.rs`

— resolves nothing, gates nothing, and reddens nothing when those files move.
The files moved to `crates/ambition_abilities/src/traversal/`. The table in
`ambition_combat::moveset::verdict_belongs_to` kept pointing at the old
location, and **the tree stayed green the whole time**.

⇒ **The citation checker sees symbols, not the directory a sentence names.**
A carve or a crate extraction breaks these silently and in bulk, because the
prose that describes *where* a population lives is exactly the prose a carve
invalidates.

## Why it is filed rather than fixed

⛔ **A gate for this is not obviously correct.** Prose paths are also written
about deleted files on purpose, about directories in other repositories, and
about shapes rather than locations (`crates/*/src/rollback_registration.rs`).
A checker that reddens on all of them buys a suppression list, and an amnesty
list is how you hide what it exempts.

⇒ **The row is the deliverable.** If this ever costs someone real time, the
diagnosis is already done and the decision about whether to gate it can be made
with the cost in hand rather than in advance.

## What it cost this time

The table it broke was the `attacker_move_instance: None` population on
`verdict_belongs_to` — the input to sizing A12's remaining work. Re-deriving it
found the count was wrong in **both** directions (13 code sites, not 15: two of
the fifteen were prose) and that its load-bearing column had never been
measured at all.
