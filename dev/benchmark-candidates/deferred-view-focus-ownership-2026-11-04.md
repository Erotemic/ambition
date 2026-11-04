# View coverage must use the focus after a queued change

A Bevy command does not update a query inside the system that queues it.
A chained system can see the applied command, but later code in the same system
still sees the old component.

A leader-view pass can queue removal of its `ViewParticipant`, then decide
which seats need another view. Coverage must use the focus after that removal.
If it uses the old leader focus, it opens a view for the first seat. The next
pass sees two views for that seat and opens a third view for the leader.

Use the expected declaration for the coverage decision. Keep the last focus
written by the layout in an ownership marker. Remove the declaration only when
it still matches that marker. This lets another writer take ownership without
losing its declaration when the layout changes.

The guard is
`local_view::tests::a_leader_view_uses_the_declared_leader_across_rooms_and_releases_its_focus`
in [local_view.rs](../../crates/ambition_sim_view/src/local_view.rs).
It follows seat 1 across two rooms, returns to adaptive layout, and checks that
the two seats have two views. Change: `5ed9bb73a`.
