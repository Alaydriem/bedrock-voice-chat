use hickory_proto::op::{Message, OpCode, ResponseCode};
use hickory_proto::rr::rdata::SRV;
use hickory_proto::rr::{Name, RData, Record};

use super::DnsScript;

/// Turns a script and a request into the bytes a fixture sends back.
pub struct DnsAnswerBuilder;

impl DnsAnswerBuilder {
    // `None` means the script sends nothing.
    pub fn response(request: &Message, script: &DnsScript, over_tcp: bool) -> Option<Vec<u8>> {
        let mut message = Message::response(request.metadata.id, OpCode::Query);
        message.metadata.recursion_desired = request.metadata.recursion_desired;
        message.queries = request.queries.clone();
        let owner = request.queries.first().map(|query| query.name().clone());

        match script {
            DnsScript::Silent | DnsScript::Hang => return None,
            DnsScript::Empty => {}
            DnsScript::NxDomain => {
                message.metadata.response_code = ResponseCode::NXDomain;
            }
            DnsScript::Answer { port, ttl, target } => {
                Self::add_answer(&mut message, owner?, *port, *ttl, target.as_deref());
            }
            DnsScript::TruncatedUdp { port, ttl } => {
                if over_tcp {
                    Self::add_answer(&mut message, owner?, *port, *ttl, None);
                } else {
                    message.metadata.truncation = true;
                }
            }
        }

        message.to_vec().ok()
    }

    fn add_answer(message: &mut Message, owner: Name, port: u16, ttl: u32, target: Option<&str>) {
        let target = match target {
            Some(target) => Name::from_ascii(target).expect("a valid target name"),
            // The queried host: the owner name without its `_bvc._tcp` service labels.
            None => owner.trim_to(owner.num_labels().saturating_sub(2) as usize),
        };
        message.add_answer(Record::from_rdata(
            owner,
            ttl,
            RData::SRV(SRV::new(0, 0, port, target)),
        ));
    }
}
