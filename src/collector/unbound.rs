// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use crate::{collector, config, metric};
use anyhow::{Context, Result};
use std::{io, path, sync, time};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct Stats {
    timestamp: time::SystemTime,
    total_num_queries: u64,
    total_num_queries_timed_out: u64,
}

pub(super) struct Unbound {
    path: &'static path::Path,
    stats: sync::Mutex<Option<Stats>>,
    notify: tokio::sync::Notify,
}

impl Unbound {
    pub fn new() -> sync::Arc<Self> {
        let unbound = Unbound {
            path: &config::get().unbound_socket,
            stats: sync::Mutex::new(None),
            notify: tokio::sync::Notify::new(),
        };
        let unbound = sync::Arc::new(unbound);

        let clone = unbound.clone();
        tokio::task::spawn(async move {
            clone.task().await;
        });

        unbound
    }

    pub fn collect(&self, metrics: &collector::Metrics, enc: &mut metric::Encoder) {
        if let Some(stats) = &*self.stats.lock().unwrap() {
            enc.write(
                &metrics.net.dns_query,
                stats.total_num_queries,
                Some(stats.timestamp),
            );
            enc.write(
                &metrics.net.dns_timeout,
                stats.total_num_queries_timed_out,
                Some(stats.timestamp),
            );
        }

        self.notify.notify_one();
    }

    async fn task(&self) {
        loop {
            match self.parse_stats().await {
                Ok(stats) => *self.stats.lock().unwrap() = Some(stats),
                Err(err) => {
                    let mut level = log::Level::Error;
                    if let Some(err) = err.downcast_ref::<io::Error>() {
                        if err.kind() == io::ErrorKind::NotFound {
                            level = log::Level::Debug;
                        }
                    }

                    log::log!(level, "failed to collect unbound stats: {err:?}");
                }
            }

            self.notify.notified().await;
        }
    }

    async fn parse_stats(&self) -> Result<Stats> {
        let mut sock = tokio::net::UnixStream::connect(&self.path)
            .await
            .with_context(|| format!("failed to connect to {:?}", self.path))?;

        let timestamp = time::SystemTime::now();

        sock.write_all("UBCT1 stats_noreset\n".as_bytes())
            .await
            .context("failed to write to unbound")?;

        let mut resp = String::new();
        sock.read_to_string(&mut resp)
            .await
            .context("failed to read from unbound")?;

        let mut total_num_queries = 0;
        let mut total_num_queries_timed_out = 0;
        for line in resp.lines() {
            if let Some(val) = line.strip_prefix("total.num.queries=") {
                total_num_queries = val.parse()?;
            } else if let Some(val) = line.strip_prefix("total.num.queries_timed_out=") {
                total_num_queries_timed_out = val.parse()?;
            }
        }

        Ok(Stats {
            timestamp,
            total_num_queries,
            total_num_queries_timed_out,
        })
    }
}
