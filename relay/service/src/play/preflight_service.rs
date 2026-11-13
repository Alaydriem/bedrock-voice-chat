use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use common::curia;
use common::structs::network::AddressClass;
use common::structs::relay::enroll::PlayPreflightOutcome;
use moka::sync::Cache;
use tokio::sync::Semaphore;

use crate::validation::{AddressProbe, ProbeOutcome};

use super::name_service::PlayNameService;
use super::preflight_limiter::PlayPreflightLimiter;
use super::preflight_ticket::PreflightTicket;
use super::probe_slots::ProbeSlots;

// Proves, before a name or a certificate is issued, that the internet reaches the server.
//
// The registry mints the nonce, so a server cannot answer with one copied from a third
// party's public endpoint.
pub struct PlayPreflightService {
    probe: AddressProbe,
    delays: Vec<Duration>,
    limiter: PlayPreflightLimiter,
    allow_private_probe_targets: bool,
    passed: Cache<String, ()>,
    // One permit per probe in flight. A probe holds a task for several seconds, and the
    // target is the caller's choice.
    in_flight: Semaphore,
    // Probes in flight per requester prefix.
    requester_slots: ProbeSlots,
    // Addresses that reach the registry host itself. A preflight must not dial them.
    own_addresses: Vec<IpAddr>,
}

impl PlayPreflightService {
    pub const PASSED_TTL: Duration = Duration::from_secs(600);
    pub const PASSED_CAPACITY: u64 = 10_000;
    pub const PRODUCTION_DELAYS: [Duration; 2] =
        [Duration::from_millis(500), Duration::from_secs(2)];
    pub const PROBE_HOST: &'static str = "preflight.play.bedrockvc.stream";
    pub const MAX_CONCURRENT_PROBES: usize = 16;
    pub const MAX_PROBES_PER_REQUESTER: usize = 2;
    const BUSY_RETRY_SECS: u64 = 5;

    pub fn new(
        probe: AddressProbe,
        delays: Vec<Duration>,
        limiter: PlayPreflightLimiter,
        allow_private_probe_targets: bool,
        max_concurrent_probes: usize,
        own_addresses: Vec<IpAddr>,
    ) -> Self {
        Self {
            probe,
            delays,
            limiter,
            allow_private_probe_targets,
            passed: Cache::builder()
                .time_to_live(Self::PASSED_TTL)
                .max_capacity(Self::PASSED_CAPACITY)
                .build(),
            in_flight: Semaphore::new(max_concurrent_probes),
            requester_slots: ProbeSlots::new(Self::MAX_PROBES_PER_REQUESTER),
            own_addresses: own_addresses
                .into_iter()
                .map(|address| address.to_canonical())
                .collect(),
        }
    }

    pub fn new_shared(
        probe: AddressProbe,
        delays: Vec<Duration>,
        limiter: PlayPreflightLimiter,
        allow_private_probe_targets: bool,
        max_concurrent_probes: usize,
        own_addresses: Vec<IpAddr>,
    ) -> Arc<Self> {
        Arc::new(Self::new(
            probe,
            delays,
            limiter,
            allow_private_probe_targets,
            max_concurrent_probes,
            own_addresses,
        ))
    }

    // Whether this registry dials private and loopback targets, as a development deployment does.
    pub fn allows_private_targets(&self) -> bool {
        self.allow_private_probe_targets
    }

    // Classifies, then rate-limits, then mints a nonce. Nothing is dialled on `Err`.
    // `observed` is the address the request arrived from, when it is known.
    pub fn prepare(
        &self,
        node_id: &str,
        candidate: IpAddr,
        observed: Option<IpAddr>,
        port: u16,
    ) -> Result<PreflightTicket, PlayPreflightOutcome> {
        if !self.allow_private_probe_targets {
            if self.own_addresses.contains(&candidate.to_canonical()) {
                return Err(PlayPreflightOutcome::AddressNotPublic {
                    address: candidate.to_string(),
                });
            }

            match AddressClass::of(candidate) {
                AddressClass::Public => {}
                AddressClass::Cgnat => {
                    return Err(PlayPreflightOutcome::AddressCgnat {
                        address: candidate.to_string(),
                    });
                }
                _ => {
                    return Err(PlayPreflightOutcome::AddressNotPublic {
                        address: candidate.to_string(),
                    });
                }
            }
        }

        if let Err(wait) = self.limiter.admit(node_id, candidate, observed) {
            // Rounded up, so a retry at the stated time is admitted.
            let secs = wait.as_secs() + u64::from(wait.subsec_nanos() > 0);
            return Err(PlayPreflightOutcome::RateLimited {
                retry_after_secs: secs.max(1),
            });
        }

        Ok(PreflightTicket {
            address: candidate,
            port,
            nonce: PlayNameService::mint_nonce(),
            observed,
        })
    }

    // Probes after each delay. The first answer wins and records the pass. When the
    // probes in flight are at their cap, nothing is dialled and the answer is
    // `RateLimited`: a caller is refused, not queued.
    pub async fn run(&self, node_id: &str, ticket: &PreflightTicket) -> PlayPreflightOutcome {
        let address = ticket.address.to_string();
        let port = ticket.port;
        let _slot = match ticket.observed {
            Some(observed) => {
                let key = PlayPreflightLimiter::observed_key(observed);
                let Some(slot) = self.requester_slots.acquire(key) else {
                    curia::info!("Play preflight refused", { "node": node_id.to_string(), "outcome": "RateLimited", "reason": "too many probes in flight for this requester" });
                    return PlayPreflightOutcome::RateLimited {
                        retry_after_secs: Self::BUSY_RETRY_SECS,
                    };
                };
                Some(slot)
            }
            None => None,
        };
        let Ok(_permit) = self.in_flight.try_acquire() else {
            curia::info!("Play preflight refused", { "node": node_id.to_string(), "outcome": "RateLimited", "reason": "too many probes in flight" });
            return PlayPreflightOutcome::RateLimited {
                retry_after_secs: Self::BUSY_RETRY_SECS,
            };
        };
        let mut last = ProbeOutcome::TimedOut;

        for delay in &self.delays {
            tokio::time::sleep(*delay).await;

            last = self
                .probe
                .probe(Self::PROBE_HOST, ticket.address, ticket.port, &ticket.nonce)
                .await;
            if last == ProbeOutcome::Answered {
                self.passed.insert(node_id.to_string(), ());
                curia::info!("Play preflight passed", { "node": node_id.to_string(), "address": address.clone(), "tcp_port": port.to_string() });
                return PlayPreflightOutcome::Reachable { address, port };
            }
        }

        let outcome = match last {
            ProbeOutcome::Answered | ProbeOutcome::TimedOut => {
                PlayPreflightOutcome::TimedOut { address, port }
            }
            ProbeOutcome::Refused => PlayPreflightOutcome::Refused { address, port },
            ProbeOutcome::WrongService => PlayPreflightOutcome::WrongService { address, port },
        };
        curia::info!("Play preflight failed", { "node": node_id.to_string(), "outcome": outcome.kind(), "address": ticket.address.to_string(), "tcp_port": ticket.port.to_string() });
        outcome
    }

    pub fn has_passed(&self, node_id: &str) -> bool {
        self.passed.contains_key(node_id)
    }
}
