// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use prometheus::{
    Encoder, IntGauge, IntGaugeVec, Opts, TextEncoder, register_int_gauge, register_int_gauge_vec,
};

const NAMESPACE: &str = "home_router";

pub struct Prom {
    encoder: TextEncoder,

    /* cpu */
    cpu_idle_ms: IntGauge,

    /* memory */
    memory_total_kb: IntGauge,
    memory_available_kb: IntGauge,
    swap_total_kb: IntGauge,
    swap_free_kb: IntGauge,

    /* filesystem */
    fs_total_kb: IntGaugeVec,
    fs_available_kb: IntGaugeVec,

    /* thermal */
    thermal_current_mc: IntGaugeVec,

    /* io */
    io_read_kb: IntGaugeVec,
    io_write_kb: IntGaugeVec,

    /* net */
    net_rx_kb: IntGaugeVec,
    net_tx_kb: IntGaugeVec,
    net_link_speed: IntGaugeVec,
}

impl Prom {
    pub fn new() -> Self {
        let encoder = TextEncoder::new();

        /* cpu */
        let cpu_idle_ms = register_int_gauge!(
            Opts::new("idle_ms", "CPU idle time")
                .namespace(NAMESPACE)
                .subsystem("cpu")
        )
        .unwrap();

        /* memory */
        let memory_total_kb = register_int_gauge!(
            Opts::new("total_kb", "Total memory size")
                .namespace(NAMESPACE)
                .subsystem("memory")
        )
        .unwrap();
        let memory_available_kb = register_int_gauge!(
            Opts::new("available_kb", "Available memory size")
                .namespace(NAMESPACE)
                .subsystem("memory")
        )
        .unwrap();
        let swap_total_kb = register_int_gauge!(
            Opts::new("swap_total_kb", "Total swap size")
                .namespace(NAMESPACE)
                .subsystem("memory")
        )
        .unwrap();
        let swap_free_kb = register_int_gauge!(
            Opts::new("swap_free_kb", "Free swap size")
                .namespace(NAMESPACE)
                .subsystem("memory")
        )
        .unwrap();

        /* filesystem */
        let fs_total_kb = register_int_gauge_vec!(
            Opts::new("total_kb", "Total filesystem size")
                .namespace(NAMESPACE)
                .subsystem("filesystem"),
            &["src", "dst"]
        )
        .unwrap();
        let fs_available_kb = register_int_gauge_vec!(
            Opts::new("available_kb", "Available filesystem size")
                .namespace(NAMESPACE)
                .subsystem("filesystem"),
            &["src", "dst"]
        )
        .unwrap();

        /* thermal */
        let thermal_current_mc = register_int_gauge_vec!(
            Opts::new("current_mc", "Current temperature")
                .namespace(NAMESPACE)
                .subsystem("thermal"),
            &["type"]
        )
        .unwrap();

        /* io */
        let io_read_kb = register_int_gauge_vec!(
            Opts::new("read_kb", "Total read size")
                .namespace(NAMESPACE)
                .subsystem("io"),
            &["block"]
        )
        .unwrap();
        let io_write_kb = register_int_gauge_vec!(
            Opts::new("write_kb", "Total write size")
                .namespace(NAMESPACE)
                .subsystem("io"),
            &["block"]
        )
        .unwrap();

        /* net */
        let net_rx_kb = register_int_gauge_vec!(
            Opts::new("rx_kb", "Total rx size")
                .namespace(NAMESPACE)
                .subsystem("net"),
            &["netdev"]
        )
        .unwrap();
        let net_tx_kb = register_int_gauge_vec!(
            Opts::new("tx_kb", "Total tx size")
                .namespace(NAMESPACE)
                .subsystem("net"),
            &["netdev"]
        )
        .unwrap();
        let net_link_speed = register_int_gauge_vec!(
            Opts::new("link_speed", "Link speed")
                .namespace(NAMESPACE)
                .subsystem("net"),
            &["netdev"]
        )
        .unwrap();

        Prom {
            encoder,
            cpu_idle_ms,
            memory_total_kb,
            memory_available_kb,
            swap_total_kb,
            swap_free_kb,
            fs_total_kb,
            fs_available_kb,
            thermal_current_mc,
            io_read_kb,
            io_write_kb,
            net_rx_kb,
            net_tx_kb,
            net_link_speed,
        }
    }

    pub fn format_type(&self) -> &str {
        self.encoder.format_type()
    }

    pub fn update(&self) {
        self.update_cpu();
        self.update_memory();
        self.update_fs();
        self.update_thermal();
        self.update_io();
        self.update_net();
    }

    fn update_cpu(&self) {
        let stat = crate::procfs::parse_stat().expect("failed to parse /proc/stat");
        self.cpu_idle_ms.set(stat.idle_ms.try_into().unwrap());
    }

    fn update_memory(&self) {
        let meminfo = crate::procfs::parse_meminfo().expect("failed to parse /proc/meminfo");
        self.memory_total_kb
            .set(meminfo.mem_total_kb.try_into().unwrap());
        self.memory_available_kb
            .set(meminfo.mem_avail_kb.try_into().unwrap());
        self.swap_total_kb
            .set(meminfo.swap_total_kb.try_into().unwrap());
        self.swap_free_kb
            .set(meminfo.swap_free_kb.try_into().unwrap());
    }

    fn update_fs(&self) {
        let mountinfos =
            crate::procfs::parse_self_mountinfo().expect("failed to parse /proc/self/mountinfo");
        for info in mountinfos {
            self.fs_total_kb
                .with_label_values(&[&info.mount_source, &info.mount_point])
                .set((info.total / 1024).try_into().unwrap());
            self.fs_available_kb
                .with_label_values(&[&info.mount_source, &info.mount_point])
                .set((info.avail / 1024).try_into().unwrap());
        }
    }

    fn update_thermal(&self) {
        let zones =
            crate::sysfs::parse_class_thermal().expect("failed to parse /sys/class/thermal");
        for zone in zones {
            self.thermal_current_mc
                .with_label_values(&[&zone.name])
                .set((zone.temp).try_into().unwrap());
        }
    }

    fn update_io(&self) {
        let diskstats = crate::procfs::parse_diskstats().expect("failed to parse /proc/diskstats");
        for stat in diskstats {
            self.io_read_kb
                .with_label_values(&[&stat.name])
                .set((stat.read_bytes / 1024).try_into().unwrap());
            self.io_write_kb
                .with_label_values(&[&stat.name])
                .set((stat.write_bytes / 1024).try_into().unwrap());
        }
    }

    fn update_net(&self) {
        let ifaces = crate::rtnetlink::parse_links().expect("failed to parse rtnetlink");
        for iface in ifaces {
            self.net_rx_kb
                .with_label_values(&[&iface.name])
                .set((iface.rx / 1024).try_into().unwrap());
            self.net_tx_kb
                .with_label_values(&[&iface.name])
                .set((iface.tx / 1024).try_into().unwrap());
        }

        let speeds = crate::ethtool::parse_ethtool().expect("failed to parse ethtool");
        for speed in speeds {
            self.net_link_speed
                .with_label_values(&[&speed.name])
                .set((speed.speed).try_into().unwrap());
        }

        let routes = crate::rtnetlink::parse_routes().expect("failed to parse routes");
        for route in routes {
            println!("gateway: {:?} oif {}", route.gateway, route.oif);
        }
    }

    pub fn gather(&self) -> Vec<u8> {
        let metrics = prometheus::gather();

        let mut buf = Vec::new();
        self.encoder.encode(&metrics, &mut buf).unwrap();

        buf
    }
}
