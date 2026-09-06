# Warning-cleanup benchmark candidates

These candidates come from cleanup patches where the code was already working,
but log-noise reductions introduced compile errors. The useful benchmark signal
is whether an agent can preserve the dependency surface while making a small
logging change.

## WQ-001: Downgrade a warning without adding an undeclared logging crate

### Context

A Bevy game crate already logs through Bevy's logging facade. A cleanup patch
changes two expected startup warnings into debug messages. The patch author uses
`tracing::debug!` directly in two existing modules, but the crate does not list
`tracing` as a direct dependency. The next build fails with:

```text
error[E0433]: failed to resolve: use of unresolved module or unlinked crate `tracing`
```

### Question

When downgrading warning-style messages to debug messages in a Rust crate that
already depends on Bevy but does not directly depend on `tracing`, what minimal
change should you make so the cleanup does not alter the crate's dependency
surface?

### Expected answer

Use the logging facade already available through Bevy, for example
`bevy::log::debug!(...)`, or import `bevy::log::debug` and call `debug!(...)`.
Do not introduce a direct `tracing` dependency just to fix a logging-level
cleanup unless the project has explicitly chosen to expose `tracing` as a crate
level dependency.

### Pitfall captured

A logging macro can compile in one module because a crate or dependency uses
`tracing` internally, but that does not make `tracing::debug!` available to the
current crate. Cleanup patches should prefer the project's existing logging API
instead of assuming the implementation crate is in scope.
