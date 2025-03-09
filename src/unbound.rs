// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use crate::prometheus::Prom;
use anyhow::{Context, Result};
use std::{
    io::{self, BufRead, Write},
    os::unix,
    path,
};

pub struct Unbound {
    path: path::PathBuf,
}

struct Stats {
    total_num_queries: u64,
}

impl Unbound {
    pub fn new(path: impl AsRef<path::Path>) -> Self {
        Unbound {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn collect(&self, prom: &Prom) {
        if let Ok(stats) = self.parse_stats() {
            println!("query count: {}", stats.total_num_queries);
        }
    }

    // TODO: async
    fn parse_stats(&self) -> Result<Stats> {
        let mut sock = unix::net::UnixStream::connect(&self.path)
            .with_context(|| format!("failed to connect to {:?}", self.path))?;
        sock.write_all("UBCT1 stats_noreset\n".as_bytes())
            .context("failed to write to unbound")?;

        let mut total_num_queries = 0;

        let reader = io::BufReader::new(sock);
        for line in reader.lines() {
            let line = line.context("failed to read from unbound")?;

            if let Some(val) = line.strip_prefix("total.num.queries=") {
                total_num_queries = val.parse().unwrap();
            }
        }

        Ok(Stats { total_num_queries })
    }
}
