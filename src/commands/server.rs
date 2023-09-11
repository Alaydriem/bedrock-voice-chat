use clap::Parser;
use std::sync::Arc;

use super::Command;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Config {}

impl Config {
    pub async fn run<'a>(&'a self, _cfg: &Arc<Command>) {
       
    }
}
