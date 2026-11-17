# Menu edges must survive gameplay suppression

A cutscene can capture gameplay input while its confirm button remains active.
Do not clear the whole input latch on each captured frame. At a display rate
above the simulation rate, a press and release can occur before the next
simulation tick. A full reset on the release frame deletes the confirm edge.
Waiting between attempts does not prevent this failure.

Filter the buffered frame through `ControlFrame::menu_only`. Keep menu edges
until a simulation tick takes them. Clear gameplay edges and held movement at
the capture boundary. A full reset belongs to the end of input authority.

`the_creators_cutscene_uses_one_press_per_shown_line` drives the shipped host
with keyboard presses at 30, 60 and 144 Hz. Before this correction, the 144 Hz
case kept the first Creator line after one F press. The core latch test checks
that the retained edge reaches one tick and does not repeat on the next tick.
