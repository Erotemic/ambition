---
status: current
last_verified: 2026-10-24
related_docs:
  - docs/systems/actors-brains-and-character-content.md
  - docs/recipes/writing-a-procedural-module.md
---

# Worn gear

Worn gear is an item that a body wears on an anchor of its art. The first gear
is a space helmet. GNU-ton, the giant gnu and the player robot can each wear
it. The gear has its own pictures. It is not painted into the art of one
character, so one gear can go on different characters.

## The contract

- **The anchor.** A gear attaches to an anchor of the wearer. The only anchor
  now is `Head`.
- **The fit.** Each wearer sheet has a fit, keyed by its sheet target
  (`CharacterAnimator::spec.base_sheet_key()`, for example `player_robot_v3`).
  The fit names the track of the sheet's part flipbook that is the anchor part
  (`head` when it names none). It also gives a `scale`, an `offset` (world
  units, in the frame of the body: +x is the way it faces, +y is up) and a
  `turn` (degrees). A wearer sheet with no fit does not show the gear.
- **The pictures.** `back` is drawn under the whole body. `front` is drawn
  just over the anchor part, so a part drawn after the anchor (a near arm) is
  in front of it. Both are at the pivot of the anchor part. They turn with the
  part and squash with it. Their `size` is in world units at a fit scale of 1,
  so the gear does not grow with the body. `socket` is the point of the
  pictures that is on the pivot (0 to 1 on each axis, from the top left). A
  body that faces left wears the mirror image, also when it draws the mirror
  row of its sheet.
- **The slot.** A gear has an exclusive equipment slot (`head`). A body wears
  one gear in a slot.

## How content declares a gear

A game declares its gear in a `worn_gear` file of its pack (schema
`worn_gear`, owned by the items capability). Ambition's file is
`game/ambition_content/assets/data/worn_gear.ron`:

```ron
"space_helmet": (
    anchor: Head,
    slot: "head",
    back: Some("sprites/space_helmet_back.png"),
    front: Some("sprites/space_helmet_front.png"),
    size: (64.0, 64.0),
    socket: (0.5, 0.58),
    fits: {
        "player_robot_v3": (track: "head", scale: 1.0),
    },
),
```

The pack compiler refuses a row with no picture, a size that is not more
than 0, a socket off the picture, or a fit with no track or no scale. The rows
fill the `WornGearManifest` (`ambition_platformer2d_shared_tangle::worn_gear`).
A content reload restates them.

## How a game puts gear on a body

To wear a gear, a body has the gear's equipment row in its `WornEquipment`:
the gear id as the row's `id`, in the gear's slot. `WornEquipment` is rollback
state, so the gear a body wears rolls back with it. A gear row has no
modifiers and no grants, but a change of the worn set folds the body's
repertoire again, as any equipment change does.

A boss conductor uses the request port `ambition.boss.wear`
(`ambition_boss_special_port::Wear`). It puts the gear on, or takes it off,
the boss, the mount the boss rides, or the body the boss targets. The adapter
refuses a gear id that no content declares, and a body that is not there,
with a warning. A request that changes nothing does not write the worn set.

## How the gear is drawn

The simulation publishes the gear of each body in `WornGearView`
(`ambition_sim_view`): one fact for each worn row whose id is a gear. The
render reads only that view and the manifest
(`ambition_render::rendering::worn_gear_visuals`). It finds the rigged root of
the body (the feature visual of an actor or a boss, else the body), asks its
parts for the fit's track under the reader `worn_gear:<track>`, and draws the
two pictures on the last draw of that track. When a body wears no gear, the
render forgets its ask. A picture that is not published draws nothing.

## A body drawn baked

A body drawn from a baked sheet has no parts, so it has no anchor part. A
boss drawn baked (`BossAnimator`) wears its gear at the middle of its
picture, moved by the fit's `offset` (world units, +x the way it faces, +y
up), and mirrored with the picture. The fit is the fit of the boss sheet's
key. The offset is one place for all the frames of the sheet, so a fit for a
baked boss is the place of its head in its rest frame, and the gear is big
enough to cover the head as it moves. GNU-ton is drawn this way. Another
body drawn baked does not show its gear; the render logs one warning for
each gear and sheet.
