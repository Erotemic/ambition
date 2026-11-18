# Keep preparation work with its immutable inputs

The desktop captures from 2026-11-18 exposed two repeated work paths.

## File access can cause a scan loop

The sprite watcher reads `published.json` during each scan. Linux reports an
open event for that read. If the watcher treats access as a change, its own
scan requests another scan. A test with only loose files does not exercise
this path.

Reject access-only reports before they enter the scan queue. Keep errors and
rescan requests. Test an idle published library and an actual publication
replacement. A read must cause no scan; a replacement must still reload.

## A frozen catalog needs one data owner

Room construction retains a character catalog. Its public API permits reads
and whole-value replacement. A deep copy for each room adds no isolation.
Repeated identity calculation over the same catalog also adds no information.

Use the existing `PreparedValue` owner to share the immutable data and its
digest. Keep serialization of the catalog independent of its storage. A new
catalog gets a new owner. A retained plan keeps its old owner. Do not put the
digest in a process-global cache or key it only by a local generation counter.

Keep validated sprite tables with their generation too. Publication must not
decode a quality table and discard it, then require its source again when a
visible body changes quality. A realization must still use the admitted table
after its source file is removed. This also removes the need for a mutable
decode cache and background warm-up workers.

## Bound curves by their distance from a chord

An animation corner has linear translation, linear scale, and constant angular
velocity within one tween interval. Its distance from the endpoint chord is
bounded by `maximum_acceleration * interval_length² / 8`.

Use this bound to reject intervals already inside the known envelope. Linear
motion needs no interior samples. Keep the error term when a sampling limit
applies. Check combined translation, rotation, and mirrored scale. Check the
published corpus against the resulting envelope.

## Camera targets need no CPU pixel buffer

Use Bevy's `Image::new_uninit` for targets whose cameras supply the pixels.
Set the texture usages that their consumers need. A capture target also needs
`COPY_SRC`. This avoids allocating, copying, and uploading a blank CPU image.
Verify the result through the real renderer and GPU readback.
