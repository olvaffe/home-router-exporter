// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

#![warn(missing_docs)]

//! Home Router Exporter is a Prometheus exporter designed for home routers.

mod collector;
mod hyper;
mod libc;
mod prometheus;

use collector::{linux, ping, unbound};
use std::net;

#[tokio::main]
async fn main() {
    let procfs_path = "/proc";
    let sysfs_path = "/sys";
    let unbound_path = "/tmp/unbound.sock";
    let hyper_addr = net::SocketAddr::from(([0, 0, 0, 0], 3000));

    let lin = linux::Linux::new(procfs_path, sysfs_path);
    let unbound = unbound::Unbound::new(unbound_path);
    let ping = ping::Ping::new();
    let prom = prometheus::Prom::new(lin, unbound, ping);

    hyper::run(hyper_addr, prom).await.unwrap();
}
