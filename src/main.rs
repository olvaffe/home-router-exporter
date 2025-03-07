// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

#![warn(missing_docs)]

//! Home Router Exporter is a Prometheus exporter designed for home routers.

mod procfs;

fn main() {
    let stat = procfs::parse_stat().expect("failed to parse /proc/stat");
    println!("{} {} {}", stat.user_ms, stat.system_ms, stat.idle_ms);
}
