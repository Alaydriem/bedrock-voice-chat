use crate::commands::Config as StateConfig;
use crate::rs::routes;
use clap::Parser;
use common::sea_orm_rocket::Database;
use faccess::PathExt;
use migration::{Migrator, MigratorTrait};
use rcgen::{Certificate, CertificateParams, DistinguishedName, IsCa, KeyPair, PKCS_ED25519};
use rocket::time::OffsetDateTime;
use std::{fs::File, io::Write, path::Path, process::exit};
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::fmt::SubscriberBuilder;

use common::{
    ncryptflib as ncryptf,
    ncryptflib::rocket::Fairing as NcryptfFairing,
    pool::{redis::RedisDb, seaorm::AppDb},
    rocket::{self, routes},
    rocket_db_pools,
};
use tracing::info;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Config {}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &StateConfig) {
        println!("Banish user");
    }
}
