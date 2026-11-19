# Parallel body steps need shared inputs and ordered effects

A body step can run beside another body step when it reads shared entry state
and changes only its own components. Shared contact must use the poses from
before integration. Do not read a neighbour's changing pose during the batch.

Prepare room geometry before the batch. Keep each body's gate passes, barriers,
portal admission, room pace, and tuning in the body step. Shared geometry does
not mean shared permission to pass through a wall.

Give each worker its own reusable contact and effect buffers. Publish global
effects after the workers finish. Use stable session, room, and simulation
identity to order those effects. Keep each body's local effect order, and use
the native writers that attach credit and source data.

These buffers are derived scratch data. They are not rollback state. A build
without threads must use the same body kernel and effect publication rules.

Measure moving bodies with distinct goals, bodies that converge with contact,
and moving geometry. Use settled bodies as a control. Compare the complete body
state and movement output with the serial reference. Measure the whole tested
update as well as the body phase, because targeting and perception can still
limit the gain. A system benchmark does not establish device frame rate.

The ECS fixture is `body::actors::active_crowd_tests::measure_active_crowds` in
`ambition_platformer2d_actor_monolith`. The fixture runs real patrol brains,
target selection, contact snapshots, and integration. It does not run the full
tactical perception schedule, navigation graph construction, or rendering.
