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
    let speeds = ethtool::parse_ethtool().expect("failed to parse ethtool");
    for speed in speeds {
        println!("nic {}: {}", speed.name, speed.speed);
    }

    let ifaces = rtnetlink::parse_rtnetlink().expect("failed to parse rtnetlink");
    for iface in ifaces {
        println!(
            "nic {}: rx {}KB tx {}KB",
            iface.name,
            iface.rx / 1024,
            iface.tx / 1024
        );
    }

    let prom = crate::prometheus::Prom::new();
    let _ = hyper::run(prom);
}
