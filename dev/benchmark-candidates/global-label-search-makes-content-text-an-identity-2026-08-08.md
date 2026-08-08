# A global label search makes CONTENT TEXT an ENTITY IDENTITY, and archetype order decides the answer

**Tags:** `bevy-ecs`, `query-order`, `test-construction`, `false-dependency`,
`agent-verification`, `bisect-proves-correlation`

## What happened

`game/ambition_app/src/app/plugins.rs` installed two `bevy_material_ui` plugins
and carried a warning that one of them, `dialog::DialogPlugin`, was
**load-bearing**: remove it and
`the_title_screen_says_choose_game_and_is_readable` reads the launcher title at
20.0px, *"because menu typography stops resolving"*. The claim was written after
a **six-step bisect** that genuinely reproduced. Nothing in the tree named a type
from the crate — `git grep bevy_material_ui` returned exactly the one file — so
the note also explained why no grep could have found the dependency: *"a plugin
contributes SYSTEMS AND RESOURCES, not names."*

Every sentence of that is true except the causal one.

The test:

```rust
let mut texts = app.world_mut().query::<(&Text, &TextFont)>();   // GLOBAL
let rendered: Vec<(String, f32)> = texts.iter(app.world())
    .map(|(text, font)| (text.0.clone(), font.font_size)).collect();

let size_of = |wanted: &str| rendered.iter()
    .find(|(label, _)| label == wanted)                          // FIRST MATCH
    .map(|(_, size)| *size).unwrap();

let title = size_of("Ambition");
assert!(title >= 32.0);
```

`"Ambition"` is on that screen **twice**, and always was. It is the launcher's
title *and* it is the game-select row for the game called Ambition — the roster's
first entry. They are different entities in different archetypes:

| | title | row label |
|---|---|---|
| spawner | `spawn_node`, `MenuNode::Text` | `spawn_control`, a control's child |
| `font_size` | `MenuTextHeightFraction(5.6).reference_pixels()` = **60.48** | `..default()` = **20.0** |
| `MenuTextHeightFraction` | yes | **no** |

`spawn_control` sets the font HANDLE and leaves the SIZE at `..default()`, and
`TextFont::default().font_size` is 20.0 — so "20.0px, Bevy's default" was a real
entity all along, not a failed resolve.

Dumping every `Text` entity with and without the plugins settles it: **the two
worlds are identical.** 29 text entities either way, same labels, and the title
measures 60.48px in **both**. What changed was the order a global query walked
its archetypes.

```
WITH    Material — archetypes 133, 118, 89, 120, …   title at index  3, row at 12  → PASS
WITHOUT Material — archetypes  88,  86, 108, 117, …  row   at index  2, title at 12 → FAIL
```

The plugins register components and spawn a `Startup` entity. That shifts
archetype/table creation order. That is the entire "dependency" — 160 s of
clean-build rustc work retained by a coincidence of iteration order. (CPU work,
not build time: the crate was not on the critical path, so the wall-clock effect
is ~20 s of a 540 s build. Do not restate it as a compile win.)

## The transferable invariant

**Never identify an entity by the text it displays.** A label is CONTENT: it is
authored, it repeats, and a game whose roster contains an entry named after the
product will collide with its own title screen on day one. Select by ROLE —
a marker component, a scoped subtree root, a semantic marker — and **assert the
match is unique**, so a duplicate fails loudly instead of being silently resolved
by `find()`.

The second half, which is what let a wrong answer look like a measurement:
**a bisect proves that removing X changes the outcome. It does not tell you
why.** Nothing in the bisect distinguishes "X provides a behaviour the test
needs" from "X perturbs an unspecified ordering the test accidentally depends
on". The moment a bisect result is written down as a mechanism, it stops being
evidence and starts being a load-bearing comment that forbids the next person
from rechecking it.

## What should have caught it, before any bisect

Arithmetic. The launcher spawns its title at
`MenuTextHeightFraction(5.6).reference_pixels()` = 60.0, and
`resolve_menu_text_size` only *corrects* that against a live window:

```rust
let height = windows.iter().next().map(|w| w.resolution.height())
    .filter(|h| *h > 0.0)
    .unwrap_or(crate::MENU_REFERENCE_VIEWPORT_HEIGHT);   // 1080.0
```

The test's App is `VisibleRenderMode::NoWindow` and has **zero** primary windows,
so the resolver takes the fallback and writes back exactly what the spawner
already wrote. **A resolver that never ran would leave the title at 60px and the
test would pass.** The stated mechanism therefore predicted the *opposite* of the
observed failure, and 20.0px could only ever have been a different entity. That
contradiction was visible from reading two functions, without building anything.

## The question for a model

Given the test above, the two spawn sites, `resolve_menu_text_size`, and the
report *"removing `DialogPlugin` makes this assert see 20.0px"* — say what is
actually wrong, and fix it so the test cannot pass or fail based on which entity
a global search reaches first.

**Expected answer.** The test is wrong, not the renderer. Scope the query to the
launcher UI root (`BasicShellUiRoot`) and select the typographic roles by role —
launcher text nodes carrying `MenuTextHeightFraction`, which a control's label
never has — asserting exactly one match. `bevy_material_ui` is then removable.

**Validation.** The fixed test must pass **with the plugins removed**, i.e. in
the configuration where the archetype walk still reaches the 20px row label
first. That is the poison: the hazardous ordering is still present and the test
no longer cares.

## Why this is a good candidate

- The wrong answer is **documented, confident, and cites a real experiment**. A
  model that trusts the comment stops immediately; the comment even pre-empts the
  obvious objection ("no grep could have shown this").
- The correct route is **arithmetic on two functions**, available before any
  build.
- The fix is not "delete the dependency" — it is noticing that a test let content
  act as identity, which the dependency removal then follows from.
- The failure is **invisible to every static check**: it compiles, the grep is
  clean, and the suite is green. Only asking the world what is in it answers it.
