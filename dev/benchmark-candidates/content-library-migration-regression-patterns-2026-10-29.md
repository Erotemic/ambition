# Content library migration: lost declarations and early playback

Removing a runtime inference rule requires an authored replacement in every
affected source. The content library migration removed theme inference from
biome and music fields. Many maps still had no explicit `parallax_theme`, so
the Primeval Trail drew the hub city and had no terrain skin. Set the theme
through the LDtk field tool and update the authoring recipes. A valid asset
catalog does not prove that a room selects its art.

Inspection tools must read the accepted library selection. A loose generator
directory can hold older products, and the engine asset directory can be empty.
The LDtk sprite and portrait inspectors now use the shared library reader.
Their tests use fixtures instead of a minimum cast size or a fixed repository
layout. An author can remove a character without restoring it for a tool test.

An asset reference with an explicit source must keep that source. Relative
path joins in boss pages, sprite metadata, and adaptive music can corrupt a
`content://` or `game://` reference. Test the loaded handle's path, including
an explicit owner that differs from the metadata owner.

A requested audio handle is not decoded audio. Kira retries pending plays on
each channel. If layers finish loading at different times, they can start at
different times. A director that starts its section clock at request time can
also end an intro too early. Load the complete cue before starting its layers
and its clock. The audio owner test delays one decoded layer beyond the full
intro length, then checks the complete intro and loop transition.

Capture snapshots include camera framing, body positions, and music state.
Inspect these with the image. A successful render or freshness check alone
does not prove correct cue order or camera framing.
