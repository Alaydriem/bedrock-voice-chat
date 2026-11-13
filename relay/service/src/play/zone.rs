use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, RwLock};

use hickory_proto::rr::rdata::{A, AAAA, NS, SOA, SRV};
use hickory_proto::rr::{Name, RData, Record};
use tokio::sync::watch;

use crate::config::PlayDnsConfig;

use super::zone_entry::PlayZoneEntry;
use super::zone_location::ZoneLocation;

// The records the play zone serves, held in memory.
//
// The registry fills it from the database at start and updates it in the same call
// that writes the database. The serial always comes from `PlayNameService`: this
// type never invents one, because a secondary ignores a zone whose serial does not
// grow, and a second counter would drift from the stored one.
pub struct PlayZone {
    // Lowercase, no trailing dot.
    zone: String,
    origin: Name,
    dns: PlayDnsConfig,
    primary: Name,
    hostmaster: Name,
    nameservers: Vec<Name>,
    // Serial and records under one lock, so a transfer never mixes two versions.
    state: RwLock<(u32, HashMap<String, PlayZoneEntry>)>,
    serial_tx: watch::Sender<u32>,
}

impl PlayZone {
    const SOA_REFRESH: i32 = 300;
    const SOA_RETRY: i32 = 60;
    // A secondary that cannot reach the registry for longer than one certificate
    // lifetime stops answering rather than serve addresses nobody has checked.
    const SOA_EXPIRE: i32 = 1_209_600;
    const NS_TTL: u32 = 300;
    pub const SERVICE: &'static str = "_bvc._tcp";

    // Names in `dns` are validated when the configuration is parsed.
    pub fn new(zone: String, dns: PlayDnsConfig) -> Self {
        let zone = zone.trim_end_matches('.').to_ascii_lowercase();
        let origin = Self::name(&zone);
        let primary = Self::name(&dns.primary);
        let hostmaster = Self::name(&dns.hostmaster);
        let nameservers = dns.nameservers.iter().map(|n| Self::name(n)).collect();
        let (serial_tx, _) = watch::channel(0);

        Self {
            zone,
            origin,
            dns,
            primary,
            hostmaster,
            nameservers,
            state: RwLock::new((0, HashMap::new())),
            serial_tx,
        }
    }

    pub fn new_shared(zone: String, dns: PlayDnsConfig) -> Arc<Self> {
        Arc::new(Self::new(zone, dns))
    }

    // Replaces every record. Ignored when `serial` is lower than the one held.
    pub fn load(&self, records: Vec<(String, PlayZoneEntry)>, serial: u32) {
        self.apply(serial, |map| {
            map.clear();
            for (label, entry) in records {
                map.insert(label.to_ascii_lowercase(), entry);
            }
        });
    }

    pub fn publish(&self, label: &str, entry: PlayZoneEntry, serial: u32) {
        self.apply(serial, |map| {
            map.insert(label.to_ascii_lowercase(), entry);
        });
    }

    pub fn withdraw(&self, label: &str, serial: u32) {
        self.apply(serial, |map| {
            map.remove(&label.to_ascii_lowercase());
        });
    }

    pub fn lookup(&self, label: &str) -> Option<PlayZoneEntry> {
        let state = self.state.read().expect("the zone lock is not poisoned");
        state.1.get(&label.to_ascii_lowercase()).copied()
    }

    pub fn serial(&self) -> u32 {
        self.state.read().expect("the zone lock is not poisoned").0
    }

    // The serial and the records, read under one lock.
    pub fn snapshot(&self) -> (u32, Vec<(String, PlayZoneEntry)>) {
        let state = self.state.read().expect("the zone lock is not poisoned");
        let mut records: Vec<_> = state.1.iter().map(|(k, v)| (k.clone(), *v)).collect();
        records.sort_by(|a, b| a.0.cmp(&b.0));
        (state.0, records)
    }

    // Fires when the serial grows.
    pub fn subscribe(&self) -> watch::Receiver<u32> {
        self.serial_tx.subscribe()
    }

    pub fn origin(&self) -> &Name {
        &self.origin
    }

    pub fn ttl(&self) -> u32 {
        self.dns.ttl
    }

    pub fn dns(&self) -> &PlayDnsConfig {
        &self.dns
    }

    // Classifies a lowercase query name, with or without a trailing dot.
    pub fn locate(&self, queried: &str) -> ZoneLocation {
        let queried = queried.trim_end_matches('.').to_ascii_lowercase();

        if queried == self.zone {
            return ZoneLocation::Apex;
        }

        let Some(host) = queried.strip_suffix(&format!(".{}", self.zone)) else {
            return ZoneLocation::Outside;
        };
        if host.is_empty() {
            return ZoneLocation::Outside;
        }

        let single_label = |rest: &str| !rest.is_empty() && !rest.contains('.');

        if let Some(label) = host.strip_prefix(&format!("{}.", Self::SERVICE)) {
            if single_label(label) {
                return ZoneLocation::Service(label.to_string());
            }
        }
        if let Some(label) = host.strip_prefix("_tcp.") {
            if single_label(label) {
                return ZoneLocation::EmptyNonTerminal(label.to_string());
            }
        }

        ZoneLocation::Host(host.to_string())
    }

    pub fn soa_record(&self) -> Record {
        self.soa_record_at(self.serial())
    }

    pub fn soa_record_at(&self, serial: u32) -> Record {
        Record::from_rdata(
            self.origin.clone(),
            self.dns.ttl,
            RData::SOA(SOA::new(
                self.primary.clone(),
                self.hostmaster.clone(),
                serial,
                Self::SOA_REFRESH,
                Self::SOA_RETRY,
                Self::SOA_EXPIRE,
                self.dns.ttl,
            )),
        )
    }

    pub fn ns_records(&self) -> Vec<Record> {
        self.nameservers
            .iter()
            .map(|ns| Record::from_rdata(self.origin.clone(), Self::NS_TTL, RData::NS(NS(ns.clone()))))
            .collect()
    }

    pub fn address_record(&self, label: &str, address: IpAddr) -> Record {
        let owner = Self::name(&format!("{label}.{}", self.zone));
        let rdata = match address {
            IpAddr::V4(v4) => RData::A(A(v4)),
            IpAddr::V6(v6) => RData::AAAA(AAAA(v6)),
        };

        Record::from_rdata(owner, self.dns.ttl, rdata)
    }

    // The SRV record that names the port a label's server answers on.
    pub fn service_record(&self, label: &str, port: u16) -> Record {
        let owner = Self::name(&format!("{}.{label}.{}", Self::SERVICE, self.zone));
        let target = Self::name(&format!("{label}.{}", self.zone));

        Record::from_rdata(owner, self.dns.ttl, RData::SRV(SRV::new(0, 0, port, target)))
    }

    // Applies a change at `serial`, or drops it when the serial is behind.
    fn apply(&self, serial: u32, change: impl FnOnce(&mut HashMap<String, PlayZoneEntry>)) {
        let mut state = self.state.write().expect("the zone lock is not poisoned");
        if serial < state.0 {
            return;
        }

        change(&mut state.1);
        let grew = serial > state.0;
        state.0 = serial;
        drop(state);

        if grew {
            self.serial_tx.send_replace(serial);
        }
    }

    fn name(text: &str) -> Name {
        let mut name = Name::from_ascii(text).expect("a valid DNS name; the configuration was validated");
        name.set_fqdn(true);
        name
    }
}
