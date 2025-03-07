// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

#![warn(missing_docs)]

//! Home Router Exporter is a Prometheus exporter designed for home routers.

mod procfs;

fn main() {
    let stat = procfs::parse_stat().expect("failed to parse /proc/stat");
    println!("cpu: {} {} {}", stat.user_ms, stat.system_ms, stat.idle_ms);

    let diskstats = procfs::parse_diskstats().expect("failed to parse /proc/diskstats");
    for stat in diskstats {
        println!(
            "{}: read {}KB write {}KB",
            stat.name,
            stat.read_bytes / 1024,
            stat.write_bytes / 1024
        );
    }
}
