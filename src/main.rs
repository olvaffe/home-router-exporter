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
    let stat = procfs::parse_stat().expect("failed to parse /proc/stat");
    println!("cpu: {} {} {}", stat.user_ms, stat.system_ms, stat.idle_ms);

    let meminfo = procfs::parse_meminfo().expect("failed to parse /proc/meminfo");
    println!(
        "mem: {}MB {}MB",
        meminfo.mem_total_kb / 1024,
        meminfo.mem_avail_kb / 1024
    );
    println!(
        "swap: {}MB {}MB",
        meminfo.swap_total_kb / 1024,
        meminfo.swap_free_kb / 1024
    );

    let diskstats = procfs::parse_diskstats().expect("failed to parse /proc/diskstats");
    for stat in diskstats {
        println!(
            "{}: read {}KB write {}KB",
            stat.name,
            stat.read_bytes / 1024,
            stat.write_bytes / 1024
        );
    }

    let mountinfos = procfs::parse_self_mountinfo().expect("failed to parse /proc/self/mountinfo");
    for info in mountinfos {
        println!(
            "{}: {} {} {} {}",
            info.mount_source,
            info.mount_point,
            info.total / 1024,
            info.free / 1024,
            info.avail / 1024
        );
    }

    let zones = sysfs::parse_class_thermal().expect("failed to parse /sys/class/thermal");
    for zone in zones {
        println!("thermal zone {}: {} {}", zone.zone, zone.name, zone.temp);
    }

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
