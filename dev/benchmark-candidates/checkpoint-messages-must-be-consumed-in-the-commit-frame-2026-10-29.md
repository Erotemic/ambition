# Checkpoint messages must be consumed in the commit frame

A rollback load clears transient message buffers. A state writer that reads a
checkpoint message on the next frame can produce different state on replay.
The original run can see a retained message. Replay starts with an empty buffer.

`513b20a1e` puts item-history readers in `CheckpointCapture`. Before the fix,
`BagSpendsSinceCheckpoint` was empty on the original run and retained a spend
after resimulation. The same phase gap existed for consumed-pickup history.

Use the published capture phase for each reader. Keep the message clear on
load. Do not add a second writer or a cache that retains an abandoned message.

A useful behavioral check banks non-empty history under forced rollback. An
empty baseline can hide a missed capture. The confirmed runtime-item case also
checks that deferred materialization is present in the new baseline and that
later loads retain one occurrence with its custody and provenance.
