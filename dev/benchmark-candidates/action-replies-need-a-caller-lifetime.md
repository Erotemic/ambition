# Action replies need a caller lifetime

A semantic identity does not identify one live lifetime. A room can recreate a
body with the same authored identity. If both bodies start their local request
counter at zero, a delayed result for the first body can match the second body.
Native entity handles do not solve this across rollback or room reconstruction.

Use a caller generation that its live room assigns. Keep the room ordinal,
generation, and local occurrence in the action ticket. Keep the room's generation
counter and the recipient's mailbox in canonical snapshots. A surviving body
keeps its mailbox during travel. A recreated body receives a new generation.

Keep the request high-water mark after bounded reply history expires. Otherwise,
a retry of an old request can repeat a reward after its terminal reply leaves
the history. Retire pending authority when its source entry or recipient leaves.
The domain must check this authority before each deferred change.

The behavioral checks belong at two boundaries: the mailbox must reject old
occurrences, and the native consumer must publish only the result it applies.
A codec round trip cannot establish either rule.
