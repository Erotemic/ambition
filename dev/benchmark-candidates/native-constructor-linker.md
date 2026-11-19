# Native constructors and linker choice

A profiling binary linked with mold 1.0.3 had two `.init_array` sections. The
loader's `DT_INIT_ARRAY` covered only the second section. Tracy's native
constructors were in the first section and did not run. The binary contained
Tracy symbols and Bevy printed its profiling warning, but the runtime had no
Tracy threads or listening socket. Its native profiler instance was null at
`main`.

The default Rust linker merged the tables and registered their full size.
Relinking the same game libraries restored the Tracy handshake. Modern mold
also passes the native constructor probe. The Linux build now selects pinned
mold 2.42.1 through `scripts/setup/native_linker.sh`. The driver uses an absolute
path: clang can select `/usr/bin/ld.mold` before a newer copy on `PATH`.

Do not infer linker performance from a dependency rebuild after a configuration
change. That change invalidates Cargo artifacts. No full game link comparison
between bundled LLD and modern mold was measured in this investigation.

For this failure class, compare `readelf -W -S` with `readelf -d`. A section
containing constructor addresses is insufficient if the loader does not visit
it. Check the runtime instance and listener as well as symbols. Keep diagnostic
fixtures and binary experiments in the ignored measurement artifacts.
