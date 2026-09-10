# Item occurrence, custody, inventory and checkpoint — writer inventory

**A7's hold, verbatim: "after A1, enumerate item occurrence, holder, inventory
and checkpoint baseline writers."** This page is that enumeration. A1 closed
2026-09-09, so the hold is spent; the packet says the census comes before any
type moves, so the census is the deliverable and no type has moved here.

Measured 2026-09-10 against `13021f0bf`. Re-derive before acting:

```bash
python3 scripts/measure_state_writers.py --domain item --sites
```

⛔⛔ **EVERY NUMBER BELOW IS A LOWER BOUND.** A text scan cannot see a rollback
codec, a `serde` restore, or a helper that takes `&mut T` and is called from
elsewhere. The reliable enumeration is to SEAL the type — make its fields
private and let `rustc` list the callers that stop compiling. Use this to know
where to look and to see the shape; seal the one you are going to act on.

## The four families

| family | owning crate | sites | outside the owner |
|---|---|---|---|
| occurrence (`GroundItem`, `SettledItem`, `WorldItem`, `ItemMotion`, `ItemEmerge`) | `ambition_held_items`, `ambition_world_items` | 25 | 9 |
| custody (`ItemCustody`, `ItemStruckBody`, `ReleasedAs`) | `ambition_held_items` | 10 | 1 |
| inventory (`OwnedItems`) | `ambition_items` | 16 | **13** |
| checkpoint baseline (`MintedItemBaseline`, `OwnedItemsBaseline`, `MintedItemDescription`, `ItemCheckpointRestoreInputs`) | `ambition_platformer2d_actor_monolith` | 9 | **0** |

**60 write-capable sites in total.**

## What the inventory says

⭐ **THE SHAPE IS THE OPPOSITE OF THE PACKET'S FRAMING, AND THAT IS THE FINDING.**
A7 is written as "separate item custody/accounting from lifecycle
orchestration", which reads as *the monolith's session code has grown into the
item domain*. Measured, the monolith's **checkpoint baseline family is 9 of 9
inside its own crate** — the one family the packet names as session-adjacent has
no foreign writer at all. Meanwhile `OwnedItems`, which the packet barely
mentions, is written from four crates and only 3 of its 16 sites are in
`ambition_items`. ⇒ The separation A7 is looking for is not between session and
item; it is between **the crate that defines inventory and the crates that
schedule writes to it**.

⛔⛔ **OCCURRENCE HAS NO CONSTRUCTOR, SO IT HAS SEVEN.** `GroundItem` has four
`pub` fields and no `impl GroundItem` anywhere. Every minting site builds the
struct literal:

| site | what mints |
|---|---|
| `crates/ambition_held_items/src/lib.rs:1355` | the domain's own release path |
| `crates/ambition_platformer2d_actor_monolith/src/construction/mod.rs:549` | world construction |
| `crates/ambition_platformer2d_actor_monolith/src/features/ecs/damage_drops.rs:332` | death drop |
| `crates/ambition_platformer2d_actor_monolith/src/items/match_spawn.rs:127` | match spawn |
| `crates/ambition_platformer2d_actor_monolith/src/items/pickup/mod.rs:416` | pickup/throw |
| `game/ambition_demo_smash/src/bomb.rs:121` | a ruleset's bomb |
| `game/ambition_demo_smash/src/mine.rs:222` | a ruleset's mine |

A7's acceptance says *"reward policy receives accepted outcomes; it does not
become an alternative item minting path."* `damage_drops.rs:332` is a drop
policy minting an occurrence directly, and the reason it can is that there is no
narrower road to take. ⇒ **This is the "make it impossible, not checked" edge in
the packet.** A constructor that takes what an occurrence needs, with the fields
sealed, turns seven minting authorities into one and leaves the six callers
stating what they are minting instead of assembling it. That also produces the
exact upper-bound enumeration this page cannot: sealing is how you find the
codec and `serde` writers a text scan misses.

⛔ **AND THE STARTER ROSTER IS INSERTED BY TWO CRATES IN THE SAME COMPOSITION.**
`OwnedItems::starter()` is `insert_resource`d at
`game/ambition_app/src/app/resources.rs:361` (in `init_sandbox_resources`, whose
own comment explains that headless `Platformer2dSimHarness` runs quest reward
systems without the presentation plugins) and again at
`game/ambition_content/src/items/mod.rs:36` (in `AmbitionItemRosterPlugin`,
installed from `plugins.rs:526` inside `install_menu_setup_and_hotkeys`). The
windowed app runs both.

⚠ **NO VALUE DIVERGES TODAY and the page will not overstate it.** Both spell the
same `ambition_items::OwnedItems::starter()` — `ambition_platformer2d::items` is
a re-export of `ambition_items` (`crates/ambition_platformer2d/src/lib.rs:200`)
and there is exactly one `fn starter` (`crates/ambition_items/src/lib.rs:632`).
Both run at plugin-build time, so the later insert wins with an identical value.
The defect is structural: **two build-time authorities for "what the game starts
owning", one of them reachable only through a menu plugin.** The sim-side insert
exists *because* the other is presentation-gated, which is the honest reason and
also the reason the duplication is invisible — each composition sees one.

⭐ **CUSTODY IS ALREADY WHERE A7 WANTS IT.** 9 of 10 writes are in
`ambition_held_items`, and the single foreign row
(`items/pickup/mod.rs:161`) is one `&mut ItemCustody` in the pickup query beside
the `&mut GroundItem` at `:160` — the same system, not a second authority. The
custody vocabulary does not need separating; it needs the occurrence minting
above to stop bypassing it.

## What this page does not settle

- **Whether a `ResMut<OwnedItems>` row ever mutates.** Every foreign row was read
  by hand and does write; the owning-crate rows were not exhaustively read.
- **The caller graph behind `mut_param`.** A chain of `&mut T` helpers is ONE
  authority, not N writers, and this counts the declaration.
- **Rollback and save roads.** `register_checkpoint_rollback_state` and the save
  codecs write all four families without naming a type at the write site.
