# CPAL web buffer scheduling

This is CPAL 0.18.2 from crates.io, under its Apache 2.0 license.
Only `src/host/webaudio/mod.rs` differs from the published source.

The Web Audio callback must keep the next buffer ahead of the device clock.
The published code kept an old cursor after a main-thread stall. Buffers then
played together until that cursor caught up. This caused music bursts and
hid short sound effects. The callback now advances a late cursor without
changing the sample data or the order of the buffers.

Remove this patch when a CPAL release includes this correction.
