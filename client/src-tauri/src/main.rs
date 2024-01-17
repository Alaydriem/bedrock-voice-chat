// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod invocations;
use std::path::Path;
use std::sync::Arc;
use common::structs::packet::AudioFramePacket;
use common::structs::packet::QuicNetworkPacket;
use faccess::PathExt;
use tracing::info;
use tracing::Level;
use tracing_appender::non_blocking::{ NonBlocking, WorkerGuard };
use tracing_subscriber::fmt::SubscriberBuilder;

use async_mutex::Mutex;

use crate::invocations::network::{ QuicNetworkPacketConsumer, QuicNetworkPacketProducer };
use crate::invocations::stream::{ AudioFramePacketProducer, AudioFramePacketConsumer };

use tokio::sync::mpsc::channel;

#[tokio::main]
async fn main() {
    tauri::async_runtime::set(tokio::runtime::Handle::current());

    let log_level: tracing::Level;

    #[cfg(debug_assertions)]
    {
        log_level = tracing::Level::INFO;
    }

    #[cfg(not(debug_assertions))]
    {
        log_level = tracing::Level::WARN;
    }

    // Setup and configure the application logger
    let out = "stdout";
    let subscriber: SubscriberBuilder = tracing_subscriber::fmt();
    let non_blocking: NonBlocking;
    let _guard: WorkerGuard;
    match out.to_lowercase().as_str() {
        "stdout" => {
            (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
        }
        _ => {
            let path = Path::new(out);
            if !path.exists() || !path.writable() {
                println!("{} doesn't exist or is not writable", out);
                return;
            }
            let file_appender = tracing_appender::rolling::daily(out, "homemaker.log");
            (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        }
    }

    subscriber
        .with_writer(non_blocking)
        .with_max_level(log_level)
        .with_level(true)
        .with_line_number(log_level.eq(&Level::DEBUG) || log_level.eq(&Level::TRACE))
        .with_file(log_level.eq(&Level::DEBUG) || log_level.eq(&Level::TRACE))
        .compact()
        .init();

    info!("Logger established!");

    // Audio cache for managing audio stream state
    crate::invocations::stream::STREAM_STATE_CACHE.get_or_init(async {
        return Some(Arc::new(moka::sync::Cache::builder().max_capacity(100).build()));
    }).await;

    // Network cache for managing network stream state
    crate::invocations::network::NETWORK_STATE_CACHE.get_or_init(async {
        return Some(Arc::new(moka::future::Cache::builder().max_capacity(100).build()));
    }).await;

    // The network consumer, sends audio packets to the audio producer, which is then consumed by the underlying stream
    let ring = async_ringbuf::AsyncHeapRb::<AudioFramePacket>::new(100000);
    let (ap, ac) = ring.split();
    let audio_producer: AudioFramePacketProducer = Arc::new(Mutex::new(ap));
    let audio_consumer: AudioFramePacketConsumer = Arc::new(Mutex::new(ac));

    let (quic_tx, quic_rx) = channel::<QuicNetworkPacket>(10000);

    let _tauri = tauri::Builder
        ::default()
        .manage(quic_tx)
        .manage(Arc::new(Mutex::new(quic_rx)))
        .manage(audio_producer)
        .manage(audio_consumer)
        .invoke_handler(
            tauri::generate_handler![
                // Authentication
                invocations::login::check_api_status,
                invocations::login::microsoft_auth,
                invocations::login::microsoft_auth_listener,
                invocations::login::microsoft_auth_login,
                invocations::login::logout,

                // Credential Management
                invocations::credentials::get_credential,
                invocations::credentials::set_credential,
                invocations::credentials::del_credential,

                // Quic
                invocations::network::stop_network_stream,
                invocations::network::network_stream,
                invocations::network::is_network_stream_active,

                // Audio
                invocations::stream::input_stream,
                invocations::stream::output_stream,
                invocations::stream::stop_stream,
                invocations::stream::get_devices,
                invocations::stream::is_audio_stream_active
            ]
        )
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
