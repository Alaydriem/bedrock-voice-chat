use async_mutex::Mutex;
use common::structs::packet::QuicNetworkPacket;

use std::{ thread, time::Duration, collections::HashMap, sync::Arc };

use moka::future::Cache;
use rand::distributions::{ Alphanumeric, DistString };
use anyhow::anyhow;

use async_once_cell::OnceCell;
use tauri::State;

pub(crate) type QuicNetworkPacketConsumer = Arc<
    Mutex<
        async_ringbuf::AsyncConsumer<
            QuicNetworkPacket,
            Arc<
                async_ringbuf::AsyncRb<
                    QuicNetworkPacket,
                    ringbuf::SharedRb<
                        QuicNetworkPacket,
                        Vec<std::mem::MaybeUninit<QuicNetworkPacket>>
                    >
                >
            >
        >
    >
>;

pub(crate) type QuicNetworkPacketProducer = Arc<
    Mutex<
        async_ringbuf::AsyncProducer<
            QuicNetworkPacket,
            Arc<
                async_ringbuf::AsyncRb<
                    QuicNetworkPacket,
                    ringbuf::SharedRb<
                        QuicNetworkPacket,
                        Vec<std::mem::MaybeUninit<QuicNetworkPacket>>
                    >
                >
            >
        >
    >
>;

pub(crate) static NETWORK_STATE_CACHE: OnceCell<
    Option<Arc<Cache<String, String, std::collections::hash_map::RandomState>>>
> = OnceCell::new();

const SENDER: &str = "send_stream";
const RECEIVER: &str = "receive_stream";
