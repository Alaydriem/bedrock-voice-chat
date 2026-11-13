use std::net::IpAddr;
use std::sync::Arc;

use common::curia;
use hickory_proto::op::{OpCode, ResponseCode};
use hickory_proto::rr::{Record, RecordType};
use hickory_server::net::runtime::Time;
use hickory_server::server::{Request, RequestHandler, ResponseHandler, ResponseInfo};

use crate::config::PlayDnsConfig;

use super::zone::PlayZone;
use super::zone_entry::PlayZoneEntry;
use super::zone_location::ZoneLocation;
use super::zone_transfer::PlayZoneTransfer;

// Answers for the play zone, from the in-memory records.
//
// Authoritative only. Names under the zone that are not published are NXDOMAIN with the
// SOA; names outside it are REFUSED, because this server is not a resolver.
pub struct PlayDnsHandler {
    zone: Arc<PlayZone>,
    transfer: PlayZoneTransfer,
}

impl PlayDnsHandler {
    pub fn new(zone: Arc<PlayZone>, dns: PlayDnsConfig) -> Self {
        let transfer = PlayZoneTransfer::new(zone.clone(), dns);
        Self { zone, transfer }
    }

    async fn answer<R: ResponseHandler>(&self, request: &Request, mut handle: R) -> ResponseInfo {
        let mac = match self.transfer.verify(request) {
            Ok(mac) => mac,
            Err(e) => {
                curia::warn!("Play DNS request refused: signature rejected", { "source": request.src().to_string(), "error": e.to_string() });
                return self.reply(request, &mut handle, None, true, ResponseCode::Refused, Vec::new(), Vec::new()).await;
            }
        };
        let mac = mac.as_deref();

        if request.metadata.op_code != OpCode::Query {
            return self.reply(request, &mut handle, mac, true, ResponseCode::NotImp, Vec::new(), Vec::new()).await;
        }

        let Ok(info) = request.request_info() else {
            return self.reply(request, &mut handle, mac, true, ResponseCode::FormErr, Vec::new(), Vec::new()).await;
        };
        let kind = info.query.query_type();
        let location = self.zone.locate(&info.query.name().to_string());

        match location {
            ZoneLocation::Outside => {
                self.reply(request, &mut handle, mac, true, ResponseCode::Refused, Vec::new(), Vec::new()).await
            }
            ZoneLocation::Apex if matches!(kind, RecordType::AXFR | RecordType::IXFR) => {
                self.transfer_zone(request, &mut handle, mac).await
            }
            ZoneLocation::Apex => {
                let answers = match kind {
                    RecordType::SOA => vec![self.zone.soa_record()],
                    RecordType::NS => self.zone.ns_records(),
                    RecordType::ANY => {
                        let mut all = vec![self.zone.soa_record()];
                        all.extend(self.zone.ns_records());
                        all
                    }
                    _ => Vec::new(),
                };
                self.positive(request, &mut handle, mac, answers).await
            }
            ZoneLocation::Service(_) | ZoneLocation::EmptyNonTerminal(_)
                if matches!(kind, RecordType::AXFR | RecordType::IXFR) =>
            {
                self.reply(request, &mut handle, mac, true, ResponseCode::Refused, Vec::new(), Vec::new()).await
            }
            ZoneLocation::Service(label) => match self.zone.lookup(&label) {
                Some(PlayZoneEntry { port: Some(port), .. })
                    if matches!(kind, RecordType::SRV | RecordType::ANY) =>
                {
                    let answers = vec![self.zone.service_record(&label, port)];
                    self.positive(request, &mut handle, mac, answers).await
                }
                Some(_) => self.positive(request, &mut handle, mac, Vec::new()).await,
                None => {
                    self.reply(request, &mut handle, mac, true, ResponseCode::NXDomain, Vec::new(), vec![self.zone.soa_record()]).await
                }
            },
            ZoneLocation::EmptyNonTerminal(label) => match self.zone.lookup(&label) {
                Some(_) => self.positive(request, &mut handle, mac, Vec::new()).await,
                None => {
                    self.reply(request, &mut handle, mac, true, ResponseCode::NXDomain, Vec::new(), vec![self.zone.soa_record()]).await
                }
            },
            ZoneLocation::Host(_) if matches!(kind, RecordType::AXFR | RecordType::IXFR) => {
                self.reply(request, &mut handle, mac, true, ResponseCode::Refused, Vec::new(), Vec::new()).await
            }
            ZoneLocation::Host(label) => match self.zone.lookup(&label) {
                Some(PlayZoneEntry { address, .. }) => {
                    let wanted = match (kind, address) {
                        (RecordType::A, IpAddr::V4(_)) => true,
                        (RecordType::AAAA, IpAddr::V6(_)) => true,
                        (RecordType::ANY, _) => true,
                        _ => false,
                    };
                    let answers = if wanted {
                        vec![self.zone.address_record(&label, address)]
                    } else {
                        Vec::new()
                    };
                    self.positive(request, &mut handle, mac, answers).await
                }
                None => {
                    self.reply(request, &mut handle, mac, true, ResponseCode::NXDomain, Vec::new(), vec![self.zone.soa_record()]).await
                }
            },
        }
    }

    // NOERROR with the answers, or with the SOA in authority when there are none.
    async fn positive<R: ResponseHandler>(
        &self,
        request: &Request,
        handle: &mut R,
        mac: Option<&[u8]>,
        answers: Vec<Record>,
    ) -> ResponseInfo {
        let authority = if answers.is_empty() {
            vec![self.zone.soa_record()]
        } else {
            Vec::new()
        };

        self.reply(request, handle, mac, true, ResponseCode::NoError, answers, authority).await
    }

    async fn transfer_zone<R: ResponseHandler>(
        &self,
        request: &Request,
        handle: &mut R,
        mac: Option<&[u8]>,
    ) -> ResponseInfo {
        if !self.transfer.permits(request, mac.is_some()) {
            curia::warn!("Play zone transfer refused", { "source": request.src().to_string(), "signed": mac.is_some().to_string() });
            return self.reply(request, handle, mac, true, ResponseCode::Refused, Vec::new(), Vec::new()).await;
        }

        self.transfer
            .respond(request, handle, mac, true, ResponseCode::NoError, self.transfer.stream())
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn reply<R: ResponseHandler>(
        &self,
        request: &Request,
        handle: &mut R,
        mac: Option<&[u8]>,
        authoritative: bool,
        code: ResponseCode,
        answers: Vec<Record>,
        authority: Vec<Record>,
    ) -> ResponseInfo {
        self.transfer
            .respond(request, handle, mac, authoritative, code, vec![(answers, authority)])
            .await
    }
}

#[async_trait::async_trait]
impl RequestHandler for PlayDnsHandler {
    async fn handle_request<R: ResponseHandler, T: Time>(
        &self,
        request: &Request,
        handle: R,
    ) -> ResponseInfo {
        self.answer(request, handle).await
    }
}
