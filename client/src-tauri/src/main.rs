// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod invocations;
use std::path::Path;

use tauri::Manager;
use window_shadows::set_shadow;

use faccess::PathExt;
use tracing::info;
use tracing::Level;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::fmt::SubscriberBuilder;

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
    };

    subscriber
        .with_writer(non_blocking)
        .with_max_level(log_level)
        .with_level(true)
        .with_line_number(log_level.eq(&Level::DEBUG) || log_level.eq(&Level::TRACE))
        .with_file(log_level.eq(&Level::DEBUG) || log_level.eq(&Level::TRACE))
        .compact()
        .init();

    info!("Logger established!");

    let tauri = tauri::Builder::default()
        .setup(|app| {
            let window = app.get_window("main").unwrap();
            set_shadow(&window, true).expect("Unsupported platform!");
            window.set_always_on_top(true);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            invocations::login::check_api_status,
            invocations::login::microsoft_auth,
            invocations::login::microsoft_auth_listener,
            invocations::login::microsoft_auth_login,
            invocations::credentials::get_credential,
            invocations::credentials::set_credential,
            invocations::credentials::del_credential,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
