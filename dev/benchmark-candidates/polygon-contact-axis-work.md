# Polygon contact axis work

Static polygon contact projects both shapes onto each candidate separation
axis. Opposite cardinal edges give the same separation test. Test each of the
two cardinal axes once. Keep the first axis so equal penetration depths keep
the same contact order and minimum normal. Other axes keep their original path.

Measure with the production body kernel as well as contact tests. Compare body
state hashes and retain cases with rotated geometry, clipped material, and
portal mappings. Use unchanged cast construction as a timing control. Small
body-step changes near that control's variation do not establish a frame gain.
