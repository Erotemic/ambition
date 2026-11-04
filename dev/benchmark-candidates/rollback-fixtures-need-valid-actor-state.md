# Rollback fixtures need a valid actor state

A post-simulation value can change because a normal system writes it. A source
label alone does not establish a valid provoked actor. Live provocation also
sets hostile aggression and disposition. With a provoked label on a peaceful,
retaliatory body, rally cleanup can restore the default mind. That is not a
snapshot failure.

Stage the complete policy before the rollback baseline. Require actual
snapshot loads, then check the selected source, brain, and wounded health.
Keep persistent aggression separate from target-derived standing: disposition
can become peaceful when no foe is present while hostile aggression remains.
Do not freeze that projection to make a snapshot test pass.

Sources: `body/aggression.rs` and `body/rally.rs` in the actor kernel,
`ambition_combat::targeting`, and the app's `rollback_provoked_actor` tests.

Fix: `72419da35`.
