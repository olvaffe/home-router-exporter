// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use crate::prometheus::Prom;
use anyhow::{Context, Result};
use std::{path, sync};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct Unbound {
    path: path::PathBuf,
    stats: sync::Mutex<Option<Stats>>,
    notify: tokio::sync::Notify,
}

struct Stats {
    total_num_queries: u64,
}

async fn parse_stats(path: path::PathBuf) -> Result<Stats> {
    let mut sock = tokio::net::UnixStream::connect(&path)
        .await
        .with_context(|| format!("failed to connect to {:?}", path))?;

    sock.write_all("UBCT1 stats_noreset\n".as_bytes())
        .await
        .context("failed to write to unbound")?;

    let mut resp = String::new();
    sock.read_to_string(&mut resp)
        .await
        .context("failed to read from unbound")?;

    let mut total_num_queries = 0;

    for line in resp.lines() {
        if let Some(val) = line.strip_prefix("total.num.queries=") {
            total_num_queries = val.parse().unwrap();
        }
    }

    Ok(Stats { total_num_queries })
}

impl Unbound {
    pub fn new(path: impl AsRef<path::Path>) -> sync::Arc<Self> {
        let unbound = Unbound {
            path: path.as_ref().to_path_buf(),
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

    pub fn collect(&self, prom: &Prom) {
        if let Some(stats) = &*self.stats.lock().unwrap() {
            println!("query count: {}", stats.total_num_queries);
        } else {
            println!("no query count");
        }

        self.notify.notify_one();
    }

    async fn task(&self) {
        loop {
            self.notify.notified().await;
            *self.stats.lock().unwrap() = parse_stats(self.path.clone()).await.ok();
        }
    }
}
