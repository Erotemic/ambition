# Read body facts through read-only queries

Reading through `ActorMut` can mark body components changed.
`ActorClusterQueryDataItem::as_actor_mut` converts Bevy `Mut` values to mutable
references. This marks the components changed before the caller changes a value.
The query also declares write access to the whole cluster.

The contact-damage pass did this for each actor before it checked whether the
actor could cause contact damage. The provocation pass used the same mutable
view to read identity, position, abilities, and tuning.

Both passes now query the facts that they read. Contact attacks use
`ContactAttack::from_body`, a pure constructor. Speech placement uses the pure
`bark_anchor` function. Contact resolution reads attackers and victims together,
so it needs neither a `ParamSet` nor a temporary attack list.

The mutable body view remains with integration and damage application, which
write body state. Read-only borrows prevent these passes from marking body components changed.

The body geometry, contact, provocation, and Hall tests cover behavior. Component
change propagation and scheduler access are separate costs. No Android frame-rate
saving is claimed from this code inspection.
