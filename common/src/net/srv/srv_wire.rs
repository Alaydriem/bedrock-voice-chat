use hickory_proto::op::{Message, Query, ResponseCode};
use hickory_proto::rr::{Name, RData, RecordType};

use super::{PlayZoneDns, SrvAnswer, SrvLookup};

/// SRV queries and answers in DNS wire format.
pub struct SrvWire;

impl SrvWire {
    // The owner name `_bvc._tcp.<host>.`, fully qualified so no search list applies.
    pub fn owner_name(host: &str) -> String {
        format!("{}.{}.", PlayZoneDns::SERVICE, host.trim_end_matches('.'))
    }

    pub fn query(host: &str, recursion_desired: bool) -> Result<Message, String> {
        let name = Name::from_ascii(Self::owner_name(host)).map_err(|error| error.to_string())?;
        let mut message = Message::query();
        message.metadata.recursion_desired = recursion_desired;
        message.add_query(Query::query(name, RecordType::SRV));
        Ok(message)
    }

    pub fn encode(message: &Message) -> Result<Vec<u8>, String> {
        message.to_vec().map_err(|error| error.to_string())
    }

    pub fn decode(bytes: &[u8]) -> Result<Message, String> {
        Message::from_vec(bytes).map_err(|error| error.to_string())
    }

    pub fn answers(records: &[hickory_proto::rr::Record]) -> Vec<SrvAnswer> {
        records
            .iter()
            .filter_map(|record| match &record.data {
                RData::SRV(srv) => Some(SrvAnswer {
                    priority: srv.priority,
                    weight: srv.weight,
                    port: srv.port,
                    target: srv.target.to_ascii(),
                    ttl: record.ttl,
                }),
                _ => None,
            })
            .collect()
    }

    // NXDOMAIN and an empty NOERROR are both "no record"; any other response code is a failure.
    pub fn classify(message: &Message) -> SrvLookup {
        match message.metadata.response_code {
            ResponseCode::NXDomain => SrvLookup::NoRecord,
            ResponseCode::NoError => {
                let answers = Self::answers(&message.answers);
                if answers.is_empty() {
                    SrvLookup::NoRecord
                } else {
                    SrvLookup::Found(answers)
                }
            }
            other => SrvLookup::Failed(format!("response code {other:?}")),
        }
    }
}
