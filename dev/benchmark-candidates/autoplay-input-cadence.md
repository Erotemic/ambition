# Navigation feedback must use the input tick

The navigation graph validates each leg with the movement kernel at 60 Hz.
An input source that follows those legs must read the body at each new input
tick. A rendered frame can contain many simulation ticks. A jump decision
made once per rendered frame does not reproduce the validated leg.

The desktop capture `desktop-autoplay-native-20261118T141710Z` shows this
failure. The body crossed the Hall, then fell 400 px on its return to the
door. It remained at `(1094.783, 6727.9995)` and reported `NoRoute`. The
surrounding frame windows had medians of about 150–200 ms.

Local input sampling belongs before input publication, outside historical
replay. Each new input tick gets one sample. A repeated request for that
same tick must get the same command. A later tick keeps held levels but must
not repeat a consumed press. Each source owns only its declared seats.

Keep the game itinerary separate from the movement feedback loop. The
itinerary must still run before a session starts, so it can launch the game.
Use the ordinary participant input path for the sampled controls.

Useful checks are the slow-frame Hall return and press catch-up tests in
`a_route_over_rooms`, plus the repeated input request test in the GGRS host.
Compare visited rooms before comparing two captures with equal durations.

The Hall return is a stress test. Run it explicitly:

```bash
cargo test -p ambition_app --test app_it autoplay_returns_from_the_hall_with_slow_render_frames -- --ignored --nocapture
```
