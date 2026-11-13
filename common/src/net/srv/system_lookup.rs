use std::time::Duration;

use hickory_resolver::TokioResolver;

use super::{SrvLookup, SrvWire};

/// Looks up an SRV record through the operating system's resolver configuration.
pub struct SystemLookup {
    timeout: Duration,
}

impl SystemLookup {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    pub async fn lookup(&self, host: &str) -> SrvLookup {
        match tokio::time::timeout(self.timeout, self.query(host)).await {
            Ok(lookup) => lookup,
            Err(_) => SrvLookup::Failed("the system resolver did not answer in time".to_string()),
        }
    }

    async fn query(&self, host: &str) -> SrvLookup {
        let mut builder = match TokioResolver::builder_tokio() {
            Ok(builder) => builder,
            Err(error) => return SrvLookup::Failed(error.to_string()),
        };
        builder.options_mut().timeout = self.timeout;
        builder.options_mut().attempts = 1;
        let resolver = match builder.build() {
            Ok(resolver) => resolver,
            Err(error) => return SrvLookup::Failed(error.to_string()),
        };

        match resolver.srv_lookup(SrvWire::owner_name(host)).await {
            Ok(lookup) => {
                let answers = SrvWire::answers(lookup.answers());
                if answers.is_empty() {
                    SrvLookup::NoRecord
                } else {
                    SrvLookup::Found(answers)
                }
            }
            Err(error) if error.is_no_records_found() => SrvLookup::NoRecord,
            Err(error) => SrvLookup::Failed(error.to_string()),
        }
    }
}
