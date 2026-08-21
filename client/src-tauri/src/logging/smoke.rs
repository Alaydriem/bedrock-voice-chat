use crate::logging::Defect;

// Fires one representative emission per invariant the logging pipeline is meant
// to hold, so a live Sentry run can be checked against a known list rather than
// against whatever the app happened to do.
//
// Every line carries smoke_test so it can be filtered out of real data, and so a
// stray production trigger is obvious in the dashboard.
pub struct LoggingSmokeTest;

impl LoggingSmokeTest {
    pub fn run() {
        Self::levels();
        Self::defect_routing();
        Self::tag_routing();
        Self::throttle();
    }

    // Each sink has its own registered level, so this shows which sinks see what.
    fn levels() {
        curia::error!("smoke: error level", { smoke_test: true });
        curia::warn!("smoke: warn level", { smoke_test: true });
        curia::info!("smoke: info level", { smoke_test: true });
        curia::debug!("smoke: debug level", { smoke_test: true });
        curia::trace!("smoke: trace level", { smoke_test: true });
    }

    fn defect_routing() {
        // Expect exactly one Sentry Issue, fingerprinted on the variant
        curia::error!("smoke: declared defect", {
            smoke_test: true,
            defect: Defect::AudioDeviceLost,
            io: "input",
            error: "synthetic",
        });

        // Expect a Sentry Log and a breadcrumb, and NO Issue. This is the
        // property the July 2026 storm fix bought.
        curia::error!("smoke: ordinary error without a defect", {
            smoke_test: true,
            error: "synthetic",
        });
    }

    fn tag_routing() {
        // Declared variant: expect a tag
        curia::warn!("smoke: valid tag variant", {
            smoke_test: true,
            transport: "quic",
        });

        // Undeclared value: expect an attribute, never an unbounded tag
        curia::warn!("smoke: invalid tag variant", {
            smoke_test: true,
            transport: "carrier-pigeon",
        });

        // Unbounded by nature: expect attributes, and no tags at all
        curia::warn!("smoke: unbounded fields", {
            smoke_test: true,
            player_hash: "9f2c1e",
            connected_server: "https://example.invalid",
            device_name: "Focusrite Scarlett 2i2",
        });
    }

    fn throttle() {
        // One fingerprint: expect one emit carrying
        // "[+19 identical suppressed in last 30s]" on the next window
        for _ in 0..20 {
            curia::warn!("smoke: throttled repeat", { smoke_test: true });
        }

        // Differ only by a tag field: expect both to emit, because Tag-destination
        // fields are folded into the throttle fingerprint
        curia::warn!("smoke: tag split", {
            smoke_test: true,
            device_host: "asio",
        });
        curia::warn!("smoke: tag split", {
            smoke_test: true,
            device_host: "wasapi",
        });
    }
}
