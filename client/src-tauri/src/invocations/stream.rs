use std::{ thread, time::Duration, collections::HashMap, sync::Arc };

use moka::future::Cache;
use rand::distributions::{ Alphanumeric, DistString };
use anyhow::anyhow;
use common::structs::config::StreamType;

const INPUT_STREAM: &str = "input_stream";
const OUTPUT_STREAM: &str = "output_stream";

#[tauri::command(async)]
pub(crate) async fn input_stream(s: String) -> Result<bool, bool> {
    // Create a new job for the thread worker to execute in
    let task = tokio::task::spawn(async move {
        // Create a new task ID and retrieve the cache
        let (id, cache) = match setup_task_cache(INPUT_STREAM).await {
            Ok((id, cache)) => (id, cache),
            Err(e) => {
                tracing::error!("{}", e.to_string());
                // If the cache isn't found, then we have no way to manage state and should self terminate
                return;
            }
        };

        loop {
            //
            if should_self_terminate(&id, cache, INPUT_STREAM).await {
                return;
            }

            // Check something external to determine if this thread should be terminated
            thread::sleep(Duration::from_millis(1000));
            println!("Input stream is doing work {}", s);
        }
    });

    _ = task.await;

    Ok(true)
}

#[tauri::command(async)]
pub(crate) async fn output_stream(s: String) -> Result<bool, bool> {
    // Create a new job for the thread worker to execute in
    let task = tokio::task::spawn(async move {
        let (id, cache) = match setup_task_cache(OUTPUT_STREAM).await {
            Ok((id, cache)) => (id, cache),
            Err(e) => {
                tracing::error!("{}", e.to_string());
                return;
            }
        };

        // Mock of an event loop
        loop {
            if should_self_terminate(&id, cache, OUTPUT_STREAM).await {
                return;
            }

            // Check something external to determine if this thread should be terminated
            thread::sleep(Duration::from_millis(1000));
            println!("Output stream is doing work: {}", s);
        }
    });

    _ = task.await;

    Ok(true)
}

#[tauri::command(async)]
pub(crate) async fn stop_stream(st: StreamType) -> bool {
    let cache_key = match st {
        StreamType::InputStream => INPUT_STREAM,
        StreamType::OutputStream => OUTPUT_STREAM,
    };

    match crate::STREAM_STATE_CACHE.get() {
        Some(cache) =>
            match cache {
                Some(cache) => {
                    let jobs: HashMap<String, i8> = HashMap::<String, i8>::new();
                    cache.insert(
                        cache_key.to_string(),
                        serde_json::to_string(&jobs).unwrap()
                    ).await;
                    return true;
                }
                None => {
                    return false;
                }
            }
        None => {
            return false;
        }
    }
}

/// Determines if a thread should self terminate or not by looking up the job ID inside of the named cache
/// Lookup the jobs inside of cache state
/// If the ID of this job no longer exists in it (or other states have changed) then self terminate
/// We should also terminate if the cache fails
/// This lookup takes 80-150.6µs, which shouldn't interfere with any audio playback buffering checks
async fn should_self_terminate(
    id: &str,
    cache: &Arc<Cache<String, String>>,
    cache_key: &str
) -> bool {
    match cache.get(cache_key).await {
        Some(result) => {
            let jobs: HashMap<String, i8> = serde_json::from_str(&result).unwrap();
            match jobs.get(id) {
                Some(_) => {
                    return false;
                }
                None => {
                    return true;
                }
            }
        }
        None => {
            return true;
        }
    }
}

/// Sets up the task cache with the correct values
/// We're storing the current job inside of the cache as a single value
/// When this task launches, we replace the entire cache key with single element containing only this id
/// We're using a hashmap to make a single lookup with a HashMap::<String, id>::new() value
/// Where the String is the self identifier of _this_ thread, and the id is the job running status
/// When this thread launches, we consider all other threads invalid, and burn the entire cache
/// If for some reason we can't access the cache, then this thread self terminates
async fn setup_task_cache(
    cache_key: &str
) -> Result<(String, &Arc<Cache<String, String>>), anyhow::Error> {
    // Self assign an ID for this job
    let id = Alphanumeric.sample_string(&mut rand::thread_rng(), 24);

    match crate::STREAM_STATE_CACHE.get() {
        Some(cache) =>
            match cache {
                Some(cache) => {
                    let mut jobs: HashMap<String, i8> = HashMap::<String, i8>::new();
                    jobs.insert(id.clone(), 1);

                    cache.insert(
                        cache_key.to_string(),
                        serde_json::to_string(&jobs).unwrap()
                    ).await;
                    return Ok((id, cache));
                }
                None => {
                    return Err(anyhow!("Cache wasn't found."));
                }
            }
        None => {
            return Err(anyhow!("Cache doesn't exist."));
        }
    }
}
