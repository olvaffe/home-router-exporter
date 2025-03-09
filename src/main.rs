// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

#![warn(missing_docs)]

//! Home Router Exporter is a Prometheus exporter designed for home routers.

mod hyper;
mod libc;
mod linux;
mod prometheus;

fn main() {
    let lin = linux::Linux::new("/proc", "/sys");
    let prom = prometheus::Prom::new(lin);
    let _ = hyper::run(prom);
}
