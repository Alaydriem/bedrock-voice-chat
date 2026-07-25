use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use once_cell::sync::Lazy;

const FAILURE_THRESHOLD: u32 = 5;
const BASE_COOLDOWN: Duration = Duration::from_secs(15);
const MAX_COOLDOWN: Duration = Duration::from_secs(300);
const MAX_BACKOFF_SHIFT: u32 = 5;

static REGISTRY: Lazy<Mutex<HashMap<String, Arc<EndpointBreaker>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Outcome of a request routed through the endpoint breaker. `Open` means the
/// breaker short-circuited before touching the network; `Transport` carries a
/// genuine send failure that was already recorded against the breaker.
pub(crate) enum SendError {
    Open,
    Transport(common::reqwest::Error),
}

struct BreakerState {
    consecutive_failures: u32,
    open_until: Option<Instant>,
    open_streak: u32,
    half_open: bool,
}

pub(crate) struct EndpointBreaker {
    state: Mutex<BreakerState>,
}

pub(crate) struct CircuitBreaker;

impl CircuitBreaker {
    /// Return the shared breaker for an endpoint. Breakers live in a process-global
    /// registry keyed by endpoint so they survive the frequent re-creation of the
    /// `Api`/`Client` structs (each `api_initialize_client` builds fresh ones).
    pub(crate) fn for_endpoint(endpoint: &str) -> Arc<EndpointBreaker> {
        let mut registry = REGISTRY.lock().unwrap();
        registry
            .entry(endpoint.to_string())
            .or_insert_with(|| Arc::new(EndpointBreaker::new()))
            .clone()
    }
}

impl EndpointBreaker {
    fn new() -> Self {
        Self {
            state: Mutex::new(BreakerState {
                consecutive_failures: 0,
                open_until: None,
                open_streak: 0,
                half_open: false,
            }),
        }
    }

    /// Whether a request may proceed. While the breaker is open and its cooldown
    /// has not elapsed this returns false, so the caller short-circuits without
    /// touching the network or logging another transport error. Once the cooldown
    /// elapses a single half-open probe is admitted.
    pub(crate) fn allow(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        match state.open_until {
            Some(until) if Instant::now() < until => false,
            Some(_) => {
                state.open_until = None;
                state.half_open = true;
                true
            }
            None => true,
        }
    }

    /// Record a reachable server response (any HTTP status). The server answered,
    /// so the circuit closes.
    pub(crate) fn on_success(&self) {
        let mut state = self.state.lock().unwrap();
        state.consecutive_failures = 0;
        state.open_until = None;
        state.open_streak = 0;
        state.half_open = false;
    }

    /// Record a transport-level failure (connection refused, TLS handshake failure,
    /// timeout). Opens the breaker once the failure threshold is crossed, or
    /// immediately when a half-open probe fails. Returns true on the transition
    /// into the open state so the caller can log a single backoff notice.
    pub(crate) fn on_transport_failure(&self) -> bool {
        let mut state = self.state.lock().unwrap();

        if state.half_open {
            state.half_open = false;
            state.open_streak = state.open_streak.saturating_add(1);
            state.open_until = Some(Instant::now() + Self::cooldown(state.open_streak));
            return true;
        }

        state.consecutive_failures = state.consecutive_failures.saturating_add(1);
        if state.consecutive_failures < FAILURE_THRESHOLD {
            return false;
        }

        state.open_streak = state.open_streak.saturating_add(1);
        state.open_until = Some(Instant::now() + Self::cooldown(state.open_streak));
        state.consecutive_failures = 0;
        true
    }

    fn cooldown(open_streak: u32) -> Duration {
        let shift = open_streak.saturating_sub(1).min(MAX_BACKOFF_SHIFT);
        BASE_COOLDOWN
            .saturating_mul(1u32 << shift)
            .min(MAX_COOLDOWN)
    }
}
