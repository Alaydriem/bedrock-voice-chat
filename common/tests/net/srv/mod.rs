#[cfg(all(debug_assertions, feature = "dev-trust"))]
mod dev_play_nameservers;
mod dns_answer_builder;
mod dns_responder;
mod dns_script;
mod doh_responder;
mod record_choice;
mod srv_resolver;

use dns_answer_builder::DnsAnswerBuilder;
use dns_responder::DnsResponder;
use dns_script::DnsScript;
use doh_responder::DohResponder;
