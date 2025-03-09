// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

#![warn(missing_docs)]

//! Home Router Exporter is a Prometheus exporter designed for home routers.

mod hyper;
mod libc;
mod linux;
mod prometheus;

use std::net;

fn main() {
    let procfs = "/proc";
    let sysfs = "/sys";
    let addr = net::SocketAddr::from(([0, 0, 0, 0], 3000));

    let lin = linux::Linux::new(procfs, sysfs);
    let prom = prometheus::Prom::new(lin);

    hyper::run(addr, prom).unwrap();
}
