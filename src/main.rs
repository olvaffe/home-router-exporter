// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

#![warn(missing_docs)]

//! Home Router Exporter is a Prometheus exporter designed for home routers.

mod ethtool;
mod hyper;
mod procfs;
mod prometheus;
mod rtnetlink;
mod sysfs;

fn main() {
    let prom = crate::prometheus::Prom::new();
    let _ = hyper::run(prom);
}
