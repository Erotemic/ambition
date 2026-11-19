# Native constructors and linker choice

A profiling binary linked with mold 1.0.3 had two `.init_array` sections. The
loader's `DT_INIT_ARRAY` covered only the second section. Tracy's native
constructors were in the first section and did not run. The binary contained
Tracy symbols and Bevy printed its profiling warning, but the runtime had no
Tracy threads or listening socket. Its native profiler instance was null at
`main`.

The default Rust linker merged the tables and registered their full size.
Relinking the same game libraries restored the Tracy handshake. Use the Rust
toolchain's native linker instead of forcing a system mold installation.

For this failure class, compare `readelf -W -S` with `readelf -d`. A section
containing constructor addresses is insufficient if the loader does not visit
it. Check the runtime instance and listener as well as symbols. Keep diagnostic
fixtures and binary experiments in the ignored measurement artifacts.
