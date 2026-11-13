use hickory_proto::op::{Message, OpCode, ResponseCode};
use hickory_proto::rr::rdata::SRV;
use hickory_proto::rr::{RData, Record};

use super::DnsScript;

/// Turns a script and a request into the bytes a fixture sends back.
pub struct DnsAnswerBuilder;

impl DnsAnswerBuilder {
    pub fn response(request: &Message, script: &DnsScript) -> Option<Vec<u8>> {
        let mut message = Message::response(request.metadata.id, OpCode::Query);
        message.metadata.recursion_desired = request.metadata.recursion_desired;
        message.queries = request.queries.clone();
        let owner = request.queries.first().map(|query| query.name().clone())?;

        match script {
            DnsScript::NxDomain => {
                message.metadata.response_code = ResponseCode::NXDomain;
            }
            DnsScript::Answer { port, ttl } => {
                // The queried host: the owner name without its `_bvc._tcp` service labels.
                let target = owner.trim_to(owner.num_labels().saturating_sub(2) as usize);
                message.add_answer(Record::from_rdata(
                    owner,
                    *ttl,
                    RData::SRV(SRV::new(0, 0, *port, target)),
                ));
            }
        }

        message.to_vec().ok()
    }
}
