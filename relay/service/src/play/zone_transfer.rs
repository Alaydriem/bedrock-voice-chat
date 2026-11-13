use std::net::IpAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use common::curia;
use hickory_proto::op::{Edns, Header, HeaderCounts, Metadata, ResponseCode};
use hickory_proto::rr::rdata::TSIG;
use hickory_proto::rr::rdata::tsig::{TsigAlgorithm, make_tsig_record};
use hickory_proto::rr::{Record, TSigResponseContext, TSigner};
use hickory_proto::serialize::binary::{BinEncodable, BinEncoder};
use hickory_server::net::xfer::Protocol;
use hickory_server::server::{Request, ResponseHandler, ResponseInfo};
use hickory_server::zone_handler::MessageResponseBuilder;

use crate::config::PlayDnsConfig;

use super::error::PlayError;
use super::zone::PlayZone;

// Zone transfers, and the TSIG signing every response to a signed request needs.
//
// The stock catalog is not used for transfers: it sends the whole zone as one message,
// which breaks past 64 KiB, and its in-memory zone does not implement signed-only
// transfers. This answers AXFR and IXFR itself, with the full zone for both, which
// RFC 1995 permits.
pub struct PlayZoneTransfer {
    zone: Arc<PlayZone>,
    dns: PlayDnsConfig,
    signer: Option<TSigner>,
}

impl PlayZoneTransfer {
    // Record bytes per message. Well under a 16 KiB message, which
    // leaves room for the header, the question and the TSIG record.
    const MESSAGE_BUDGET: usize = 15 * 1024;
    // Advertised for responses to a request that carried EDNS.
    const EDNS_PAYLOAD: u16 = 1232;

    pub fn new(zone: Arc<PlayZone>, dns: PlayDnsConfig) -> Self {
        let signer = dns.tsig.as_ref().and_then(|tsig| match tsig.signer() {
            Ok(signer) => Some(signer),
            Err(e) => {
                curia::error!("Play TSIG key is unusable. Zone transfers are refused.", { "error": e.to_string() });
                None
            }
        });

        Self { zone, dns, signer }
    }

    // `Ok(None)` for an unsigned request, `Ok(Some(mac))` for one signed with the
    // configured key inside the time window, and an error for anything else.
    pub fn verify(&self, request: &Request) -> Result<Option<Vec<u8>>, PlayError> {
        if request.signature.is_none() {
            return Ok(None);
        }

        let Some(signer) = &self.signer else {
            return Err(PlayError::Dns("no TSIG key is configured".to_string()));
        };

        match signer.verify_message_byte(request.as_slice(), None, true) {
            Ok((mac, _time, window)) if window.contains(&Self::now()) => Ok(Some(mac)),
            Ok(_) => Err(PlayError::Dns("the TSIG time is outside the window".to_string())),
            Err(e) => Err(PlayError::Dns(format!("TSIG verification failed: {e}"))),
        }
    }

    // Whether this request may pull the zone: over TCP, from a secondary's address, and
    // with a verified signature.
    pub fn permits(&self, request: &Request, signed: bool) -> bool {
        signed && request.protocol() == Protocol::Tcp && self.is_secondary(request.src().ip())
    }

    // The records of a full transfer, split into messages: the SOA, the apex NS set, every
    // address record, then the SOA again, all from one snapshot.
    pub fn stream(&self) -> Vec<(Vec<Record>, Vec<Record>)> {
        let (serial, records) = self.zone.snapshot();

        let mut all = Vec::with_capacity(records.len() + 4);
        all.push(self.zone.soa_record_at(serial));
        all.extend(self.zone.ns_records());
        all.extend(
            records
                .into_iter()
                .map(|(label, address)| self.zone.address_record(&label, address)),
        );
        all.push(self.zone.soa_record_at(serial));

        let mut messages = Vec::new();
        let mut current = Vec::new();
        let mut size = 0usize;
        for record in all {
            let weight = record.to_bytes().map(|bytes| bytes.len()).unwrap_or(64);
            if !current.is_empty() && size + weight > Self::MESSAGE_BUDGET {
                messages.push((std::mem::take(&mut current), Vec::new()));
                size = 0;
            }
            size += weight;
            current.push(record);
        }
        messages.push((current, Vec::new()));

        messages
    }

    // Sends one response message per entry of `messages`, each as `(answers, authority)`.
    // Signs every one when the request was signed: the first against the request's MAC,
    // later ones chained to the message before (RFC 8945 section 5.3.1).
    pub async fn respond<R: ResponseHandler>(
        &self,
        request: &Request,
        handle: &mut R,
        request_mac: Option<&[u8]>,
        authoritative: bool,
        code: ResponseCode,
        messages: Vec<(Vec<Record>, Vec<Record>)>,
    ) -> ResponseInfo {
        let edns = request.edns.as_ref().map(|_| {
            let mut edns = Edns::new();
            edns.set_max_payload(Self::EDNS_PAYLOAD);
            edns.set_version(0);
            edns
        });

        let signer = request_mac.and(self.signer.as_ref());
        let mut previous_mac = request_mac.map(<[u8]>::to_vec).unwrap_or_default();
        let mut last = None;

        for (index, (answers, authority)) in messages.iter().enumerate() {
            let mut metadata = Metadata::response_from_request(&request.metadata);
            metadata.authoritative = authoritative;
            metadata.response_code = code;

            let build = |signature| {
                let mut response = MessageResponseBuilder::new(&request.queries, edns.as_ref())
                    .build(
                        metadata,
                        answers.iter(),
                        std::iter::empty(),
                        authority.iter(),
                        std::iter::empty(),
                    );
                if let Some(signature) = signature {
                    response.set_signature(signature);
                }
                response
            };

            let result = match signer {
                None => handle.send_response(build(None)).await,
                Some(signer) => {
                    let mut encoded = Vec::new();
                    let signed = build(None)
                        .destructive_emit(&mut BinEncoder::new(&mut encoded))
                        .map_err(|e| e.to_string())
                        .and_then(|_| {
                            self.sign(request.metadata.id, signer, &previous_mac, &encoded, index == 0)
                        });

                    match signed {
                        Ok((signature, mac)) => {
                            previous_mac = mac;
                            handle.send_response(build(Some(signature))).await
                        }
                        Err(e) => {
                            curia::error!("Play DNS response signing failed", { "error": e.to_string() });
                            return Self::failed(request);
                        }
                    }
                }
            };

            match result {
                Ok(info) => last = Some(info),
                Err(e) => {
                    curia::warn!("Play DNS response send failed", { "error": e.to_string() });
                    return Self::failed(request);
                }
            }
        }

        last.unwrap_or_else(|| Self::failed(request))
    }

    // The TSIG record for one message and the MAC the next message chains to.
    fn sign(
        &self,
        id: u16,
        signer: &TSigner,
        previous_mac: &[u8],
        encoded: &[u8],
        first: bool,
    ) -> Result<(Box<hickory_proto::rr::Record<TSIG>>, Vec<u8>), String> {
        let time = Self::now();

        if first {
            let record = TSigResponseContext::new(id, time, signer.clone(), previous_mac.to_vec(), None)
                .sign(encoded)
                .map_err(|e| e.to_string())?;
            let mac = record.data.mac.clone();
            return Ok((record, mac));
        }

        // Prior MAC, the message, then only the time and fudge fields.
        let mut input = Vec::with_capacity(previous_mac.len() + encoded.len() + 12);
        input.extend_from_slice(&(previous_mac.len() as u16).to_be_bytes());
        input.extend_from_slice(previous_mac);
        input.extend_from_slice(encoded);
        input.extend_from_slice(&time.to_be_bytes()[2..]);
        input.extend_from_slice(&signer.fudge().to_be_bytes());

        let mac = signer.sign(&input).map_err(|e| e.to_string())?;
        let tsig = TSIG::new(
            TsigAlgorithm::HmacSha256,
            time,
            signer.fudge(),
            mac.clone(),
            id,
            None,
            Vec::new(),
        );

        Ok((Box::new(make_tsig_record(signer.signer_name().clone(), tsig)), mac))
    }

    fn is_secondary(&self, source: IpAddr) -> bool {
        self.dns
            .secondaries
            .iter()
            .any(|secondary| secondary.transfer_from.contains(&source))
    }

    fn failed(request: &Request) -> ResponseInfo {
        let mut metadata = Metadata::response_from_request(&request.metadata);
        metadata.response_code = ResponseCode::ServFail;

        ResponseInfo::from(Header {
            metadata,
            counts: HeaderCounts::default(),
        })
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default()
    }
}
