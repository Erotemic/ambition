# Decode a rig publication once per consumer

The Android capture `android-all-castle_godwin-auto-20261124T040732Z`
attributes 1.79% of process CPU samples to `bind_rigged_presentations`.
Its part-binding path parsed the same RON document three times: for the
body rig, the part bindings, and the limb chains. Most of the document is
animation poses. Read those products from one decoded publication.

The capture's actual library build ID is
`0b087a9e23e7f802f8b074f0657b183c7bbe068b`. The candidate list names an older
build from before the profile build. Use the build ID in `perf.data` when
selecting symbols. The unpacked capture retains the matching library under
`symfs/`; the archive excludes it.

On the VM, the `profiling` build of `measure_body_rig_parse` measured complete
parse and preparation of a 2,399,792-byte robot publication. Nine samples of
five iterations each gave these times per iteration:

| reader | median | range |
| --- | ---: | ---: |
| three parses | 94.298 ms | 93.207–95.294 ms |
| one parse | 32.237 ms | 31.752–33.494 ms |

Both produced 15 joints, 294 clips, 58 part bindings, and 4 limbs. The input
SHA-256 is `a3f3856b43d16b2749049da32a034c9df6126dbc8926bde657d4c3c4f6abb6fc`.
The baseline was `c61fa2396` with the measurement example added. Run the
example with a publication path to repeat the measurement. Keep joint and
limb validation, schema checks, and the published origin in the shared reader.

This measures CPU preparation on the VM. It does not establish Android frame
time or FPS after the change. Repeat the device capture to measure that effect.
