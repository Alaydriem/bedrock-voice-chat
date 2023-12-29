use crate::commands::Config as StateConfig;
use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Config {}

impl Config {
    pub async fn run<'a>(&'a self, _cfg: &StateConfig) {
        println!("Banish user");
    }
}
