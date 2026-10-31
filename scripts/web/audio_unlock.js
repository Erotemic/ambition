(function () {
            const Orig = window.AudioContext || window.webkitAudioContext;
            if (!Orig) {
                console.warn("[ambition-audio] no AudioContext on this browser; audio will be silent");
                return;
            }
            const tracked = [];
            const Patched = function (...args) {
                const ctx = new Orig(...args);
                tracked.push(ctx);
                console.info(
                    "[ambition-audio] AudioContext created (state=" + ctx.state +
                    ", sampleRate=" + ctx.sampleRate + "). " +
                    (ctx.state === "suspended"
                        ? "Waiting for first user gesture to resume."
                        : "Already running.")
                );
                return ctx;
            };
            Patched.prototype = Orig.prototype;
            window.AudioContext = Patched;
            if (window.webkitAudioContext) {
                window.webkitAudioContext = Patched;
            }

            const status = () =>
                tracked.map((c) => c.state).join(", ") || "<none>";

            const resumeAll = (evtType) => {
                if (tracked.length === 0) {
                    return;
                }
                let attempted = 0;
                for (const ctx of tracked) {
                    if (ctx.state === "suspended") {
                        attempted++;
                        ctx.resume()
                            .then(() =>
                                console.info(
                                    "[ambition-audio] resume() succeeded via " + evtType +
                                    " (now state=" + ctx.state + ")"
                                )
                            )
                            .catch((err) =>
                                console.warn(
                                    "[ambition-audio] resume() failed via " + evtType + ":",
                                    err
                                )
                            );
                    }
                }
                if (attempted > 0) {
                    console.info(
                        "[ambition-audio] attempted resume on " + attempted +
                        " suspended context(s); current states=[" + status() + "]"
                    );
                }
            };

            const events = ["pointerdown", "keydown", "touchstart", "click"];
            for (const evt of events) {
                document.addEventListener(evt, () => resumeAll(evt), {
                    capture: true,
                    passive: true,
                });
            }
            console.info(
                "[ambition-audio] AudioContext unlock hook installed (listeners: " +
                events.join(", ") + ")"
            );
        })();
