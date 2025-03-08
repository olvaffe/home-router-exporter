// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

#![warn(missing_docs)]

//! Home Router Exporter is a Prometheus exporter designed for home routers.

mod hyper;
mod linux;
mod procfs;
mod prometheus;
mod rtnetlink;
mod sysfs;

fn main() {
    let lin = linux::Linux::new("/proc", "/sysfs");
    let prom = prometheus::Prom::new(lin);
    let _ = hyper::run(prom);
}
