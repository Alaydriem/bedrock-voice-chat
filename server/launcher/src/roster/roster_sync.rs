use std::collections::HashSet;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use reqwest::blocking::Client;

use super::{RosterEndpoint, RosterPayload, RosterTick};
use crate::Console;
use crate::setup::AllowlistFile;

/// Registers the players in `allowlist.json` with BVC before they first join.
///
/// Only adds: a name removed from the file is ignored, and nothing is sent until BVC reports
/// ready. `sent` is per process, so a restart sends the allowlist again, which registers
/// nobody twice.
pub struct RosterSync {
    variables: PathBuf,
    allowlist: PathBuf,
    interval: Duration,
    sent: HashSet<String>,
    // Logged only when it changes, so a long outage is reported once rather than every tick.
    not_ready: Option<String>,
    ticks: u64,
}

impl RosterSync {
    pub const INTERVAL: Duration = Duration::from_secs(60);
    const TIMEOUT: Duration = Duration::from_secs(5);
    // `/api/position` answers 200 even when registering a player failed, so `sent` is only
    // trusted for this long.
    const RESEND_AFTER: Duration = Duration::from_secs(60 * 60);

    pub fn new(variables: PathBuf, allowlist: PathBuf, interval: Duration) -> Self {
        Self {
            variables,
            allowlist,
            interval,
            sent: HashSet::new(),
            not_ready: None,
            ticks: 0,
        }
    }

    pub fn spawn(mut self) {
        thread::spawn(move || {
            loop {
                self.tick();
                thread::sleep(self.interval);
            }
        });
    }

    pub fn tick(&mut self) -> RosterTick {
        self.forget_sent_when_due();
        let (endpoint, client) = match Self::connect(&self.variables) {
            Ok(connected) => connected,
            Err(e) => {
                Console::warn(&format!("Allowlist not sent to BVC: {e:#}"));
                return RosterTick::NoEndpoint;
            }
        };
        if let Err(reason) = Self::ready(&client, &endpoint) {
            if self.not_ready.as_deref() != Some(reason.as_str()) {
                Console::warn(&format!("Allowlist waiting for BVC: {reason}"));
                self.not_ready = Some(reason.clone());
            }
            return RosterTick::NotReady(reason);
        }
        if self.not_ready.take().is_some() {
            Console::info("BVC is ready; sending the allowlist");
        }

        let allowlist = match AllowlistFile::load(&self.allowlist) {
            Ok(allowlist) => allowlist,
            Err(e) => {
                Console::warn(&format!("Allowlist not read: {e:#}"));
                return RosterTick::NoAllowlist;
            }
        };
        if allowlist.skipped() > 0 {
            Console::warn(&format!(
                "{} allowlist entries have no name and were skipped",
                allowlist.skipped()
            ));
        }

        let pending: Vec<String> = allowlist
            .names()
            .iter()
            .filter(|name| !self.sent.contains(*name))
            .cloned()
            .collect();
        if pending.is_empty() {
            return RosterTick::NothingPending;
        }
        self.post(&client, &endpoint, pending)
    }

    fn forget_sent_when_due(&mut self) {
        let interval = self.interval.as_secs().max(1);
        let period = (Self::RESEND_AFTER.as_secs() / interval).max(1);
        if self.ticks > 0 && self.ticks % period == 0 {
            self.sent.clear();
        }
        self.ticks += 1;
    }

    fn connect(variables: &std::path::Path) -> anyhow::Result<(RosterEndpoint, Client)> {
        let endpoint = RosterEndpoint::from_variables(variables)?;
        let client = endpoint.client(Self::TIMEOUT)?;
        Ok((endpoint, client))
    }

    fn ready(client: &Client, endpoint: &RosterEndpoint) -> Result<(), String> {
        let url = endpoint
            .url("/health/readiness")
            .map_err(|e| format!("{e:#}"))?;
        let response = client.get(url).send().map_err(|e| format!("{e:#}"))?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("BVC answered {}", response.status()))
        }
    }

    fn post(&mut self, client: &Client, endpoint: &RosterEndpoint, pending: Vec<String>) -> RosterTick {
        let result = endpoint.url("/api/position").and_then(|url| {
            Ok(client
                .post(url)
                .bearer_auth(endpoint.token())
                .json(&RosterPayload::body(&pending))
                .send()?)
        });
        match result {
            Ok(response) if response.status().is_success() => {
                let count = pending.len();
                self.sent.extend(pending);
                Console::info(&format!("Registered {count} allowlisted player(s) with BVC"));
                RosterTick::Registered(count)
            }
            Ok(response) => Self::rejected(format!("BVC answered {}", response.status())),
            Err(e) => Self::rejected(format!("{e:#}")),
        }
    }

    fn rejected(reason: String) -> RosterTick {
        Console::warn(&format!("Allowlist not registered: {reason}"));
        RosterTick::Rejected(reason)
    }
}
