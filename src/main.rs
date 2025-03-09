// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

#![warn(missing_docs)]

//! Home Router Exporter is a Prometheus exporter designed for home routers.

mod hyper;
mod libc;
mod linux;
mod prometheus;
mod unbound;

use std::net;

fn main() {
    let procfs_path = "/proc";
    let sysfs_path = "/sys";
    let unbound_path = "/tmp/unbound.sock";
    let hyper_addr = net::SocketAddr::from(([0, 0, 0, 0], 3000));

    let lin = linux::Linux::new(procfs_path, sysfs_path);
    let unbound = unbound::Unbound::new(unbound_path);
    let prom = prometheus::Prom::new(lin, unbound);

    hyper::run(hyper_addr, prom).unwrap();
}
