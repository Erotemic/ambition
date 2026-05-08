# SkelForm bridge

`tools/skelform_export.py` converts one assembled YAML rig pose into a SkelForm `.skf` package. It starts with one pose, extracts textures from the green-screen source using a threshold ramp for blended edges, creates `atlas0.png`, and writes a simple hierarchy with pelvis, torso, head, limbs, and hand/foot target bones.

## v51 topology correction

The first SkelForm export made visible hands and feet part of the IK chain. That
made the hand/foot texture act like an end-effector bone instead of an attached
sprite, which made it hard to reposition a hand along the wrist/arm and could
make target editing feel cross-wired.

The export now inserts explicit empty endpoint bones:

- `BackWrist` / `FrontWrist`
- `BackAnkle` / `FrontAnkle`

IK chains now target those empty endpoint bones:

- `BackArm -> BackWrist`
- `FrontArm -> FrontWrist`
- `BackLeg -> BackAnkle`
- `FrontLeg -> FrontAnkle`

The visible attachment sprites are children of the endpoint bones and are not in
the IK chains:

- `BackHand` parented to `BackWrist`
- `FrontHand` parented to `FrontWrist`
- `BackFoot` parented to `BackAnkle`
- `FrontFoot` parented to `FrontAnkle`

This means hand/foot targets move the limb endpoint, while the visible hand/foot
can still be moved/rotated relative to that endpoint for grip/contact placement.
