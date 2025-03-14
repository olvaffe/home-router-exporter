// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use crate::config;
use crate::prometheus::Prom;
use anyhow::{Context, Result, anyhow};
use serde_json::{self, Value, json};
use std::{io, path, sync};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct Kea {
    path: &'static path::Path,
    req: Vec<u8>,
    stats: sync::Mutex<Option<Stats>>,
    notify: tokio::sync::Notify,
}

struct Stats {
    pkt4_received: u64,
    pkt4_sent: u64,
    v4_allocation_fail: u64,
}

impl Kea {
    pub fn new() -> Result<sync::Arc<Self>> {
        let req = json!({
            "command": "statistic-get-all"
        });
        let req = serde_json::to_vec(&req)?;

        let kea = Kea {
            path: &config::get().kea_socket,
            req,
            stats: sync::Mutex::new(None),
            notify: tokio::sync::Notify::new(),
        };
        let kea = sync::Arc::new(kea);

        let clone = kea.clone();
        tokio::task::spawn(async move {
            clone.task().await;
        });

        Ok(kea)
    }

    pub fn collect(&self, prom: &Prom) {
        if let Some(stats) = self.stats.lock().unwrap().take() {
            prom.net.dhcp_rx_pkt.set(stats.pkt4_received as i64);
            prom.net.dhcp_tx_pkt.set(stats.pkt4_sent as i64);
            prom.net.dhcp_addr_fail.set(stats.v4_allocation_fail as i64);
        }

        self.notify.notify_one();
    }

    async fn task(&self) {
        loop {
            self.notify.notified().await;

            match self.parse_stats().await {
                Ok(stats) => *self.stats.lock().unwrap() = Some(stats),
                Err(err) => {
                    let mut level = log::Level::Error;
                    if let Some(err) = err.downcast_ref::<io::Error>() {
                        if err.kind() == io::ErrorKind::NotFound {
                            level = log::Level::Debug;
                        }
                    }

                    log::log!(level, "failed to collect kea stats: {err:?}");
                }
            }
        }
    }

    async fn parse_stats(&self) -> Result<Stats> {
        let mut sock = tokio::net::UnixStream::connect(&self.path)
            .await
            .with_context(|| format!("failed to connect to {:?}", self.path))?;

        sock.write_all(&self.req)
            .await
            .context("failed to write to kea")?;

        let mut buf = Vec::new();
        sock.read_to_end(&mut buf)
            .await
            .context("failed to read from kea")?;
        let resp: Value = serde_json::from_slice(&buf).context("failed to parse kea response")?;

        let result = resp
            .pointer("/result")
            .and_then(Value::as_u64)
            .unwrap_or(100);
        if result != 0 {
            return Err(anyhow!("kea responded result {result}"));
        }

        let pkt4_received = resp
            .pointer("/arguments/pkt4-received/0/0")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let pkt4_sent = resp
            .pointer("/arguments/pkt4-sent/0/0")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let v4_allocation_fail = resp
            .pointer("/arguments/v4-allocation-fail/0/0")
            .and_then(Value::as_u64)
            .unwrap_or_default();

        Ok(Stats {
            pkt4_received,
            pkt4_sent,
            v4_allocation_fail,
        })
    }
}
