// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use crate::collector::{kea, linux, unbound};
use anyhow::Result;
use log::debug;
use prometheus::{
    Encoder, IntGauge, IntGaugeVec, Opts, Registry, TextEncoder,
    register_int_gauge_vec_with_registry, register_int_gauge_with_registry,
};
use std::sync;

const NAMESPACE: &str = "home_router";
const SUBSYS_CPU: &str = "cpu";
const SUBSYS_MEMORY: &str = "memory";
const SUBSYS_FILESYSTEM: &str = "filesystem";
const SUBSYS_THERMAL: &str = "thermal";
const SUBSYS_NETWORK: &str = "network";

pub struct CpuMetrics {
    pub idle_ms: IntGaugeVec,
}

pub struct MemoryMetrics {
    pub total_kb: IntGauge,
    pub available_kb: IntGauge,
    pub swap_total_kb: IntGauge,
    pub swap_free_kb: IntGauge,
}

pub struct FilesystemMetrics {
    pub total_kb: IntGaugeVec,
    pub available_kb: IntGaugeVec,
    pub read_kb: IntGaugeVec,
    pub write_kb: IntGaugeVec,
}

pub struct ThermalMetrics {
    pub temp_mc: IntGaugeVec,
}

pub struct NetworkMetrics {
    pub link_speed_mbps: IntGaugeVec,
    pub link_up: IntGaugeVec,
    pub link_operstate: IntGaugeVec,
    pub link_rx_kb: IntGaugeVec,
    pub link_tx_kb: IntGaugeVec,

    pub route_default: IntGaugeVec,

    pub nft_set_counter_kb: IntGaugeVec,

    pub dhcp_rx_pkt: IntGauge,
    pub dhcp_tx_pkt: IntGauge,
    pub dhcp_addr_fail: IntGauge,

    pub dns_rx_pkt: IntGauge,
    pub dns_rx_timeout: IntGauge,
}

pub struct Prom {
    lin: linux::Linux,
    kea: sync::Arc<kea::Kea>,
    unbound: sync::Arc<unbound::Unbound>,

    registry: Registry,
    encoder: TextEncoder,

    mutex: sync::Mutex<()>,
    pub cpu: CpuMetrics,
    pub mem: MemoryMetrics,
    pub fs: FilesystemMetrics,
    pub thermal: ThermalMetrics,
    pub net: NetworkMetrics,
}

impl Prom {
    pub fn new(
        lin: linux::Linux,
        kea: sync::Arc<kea::Kea>,
        unbound: sync::Arc<unbound::Unbound>,
    ) -> Result<Self> {
        let registry = Registry::new();
        let encoder = TextEncoder::new();
        let mutex = sync::Mutex::new(());

        let cpu = CpuMetrics {
            idle_ms: register_int_gauge_vec_with_registry!(
                Opts::new("idle_ms", "CPU idle time")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_CPU),
                &["cpu"],
                registry,
            )?,
        };

        let mem = MemoryMetrics {
            total_kb: register_int_gauge_with_registry!(
                Opts::new("total_kb", "Total memory size")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_MEMORY),
                registry,
            )?,
            available_kb: register_int_gauge_with_registry!(
                Opts::new("available_kb", "Estimated available memory size")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_MEMORY),
                registry,
            )?,
            swap_total_kb: register_int_gauge_with_registry!(
                Opts::new("swap_total_kb", "Total swap size")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_MEMORY),
                registry,
            )?,
            swap_free_kb: register_int_gauge_with_registry!(
                Opts::new("swap_free_kb", "Free swap size")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_MEMORY),
                registry,
            )?,
        };

        let fs = FilesystemMetrics {
            total_kb: register_int_gauge_vec_with_registry!(
                Opts::new("total_kb", "Total filesystem size")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_FILESYSTEM),
                &["device", "mountpoint"],
                registry,
            )?,
            available_kb: register_int_gauge_vec_with_registry!(
                Opts::new("available_kb", "Available filesystem size")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_FILESYSTEM),
                &["device", "mountpoint"],
                registry,
            )?,

            read_kb: register_int_gauge_vec_with_registry!(
                Opts::new("read_kb", "Total read size")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_FILESYSTEM),
                &["device", "mountpoint"],
                registry,
            )?,
            write_kb: register_int_gauge_vec_with_registry!(
                Opts::new("write_kb", "Total write size")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_FILESYSTEM),
                &["device", "mountpoint"],
                registry,
            )?,
        };

        let thermal = ThermalMetrics {
            temp_mc: register_int_gauge_vec_with_registry!(
                Opts::new("temp_mc", "Current temperature")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_THERMAL),
                &["type"],
                registry,
            )?,
        };

        let net = NetworkMetrics {
            link_speed_mbps: register_int_gauge_vec_with_registry!(
                Opts::new("link_speed_mbps", "Link speed")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_NETWORK),
                &["device"],
                registry,
            )?,
            link_up: register_int_gauge_vec_with_registry!(
                Opts::new("link_up", "Link administrative state")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_NETWORK),
                &["device"],
                registry,
            )?,
            link_operstate: register_int_gauge_vec_with_registry!(
                Opts::new("link_operstate", "Link operational state")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_NETWORK),
                &["device"],
                registry,
            )?,
            link_rx_kb: register_int_gauge_vec_with_registry!(
                Opts::new("link_rx_kb", "Total rx size")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_NETWORK),
                &["device"],
                registry,
            )?,
            link_tx_kb: register_int_gauge_vec_with_registry!(
                Opts::new("link_tx_kb", "Total tx size")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_NETWORK),
                &["device"],
                registry,
            )?,

            route_default: register_int_gauge_vec_with_registry!(
                Opts::new("route_default", "Default route")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_NETWORK),
                &["gateway"],
                registry,
            )?,

            nft_set_counter_kb: register_int_gauge_vec_with_registry!(
                Opts::new("nft_set_counter_kb", "Nftables counter")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_NETWORK),
                &["family", "table", "set", "addr"],
                registry,
            )?,

            dhcp_rx_pkt: register_int_gauge_with_registry!(
                Opts::new("dhcp_rx_pkt", "DHCP total packet received")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_NETWORK),
                registry,
            )?,
            dhcp_tx_pkt: register_int_gauge_with_registry!(
                Opts::new("dhcp_tx_pkt", "DHCP total packet sent")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_NETWORK),
                registry,
            )?,
            dhcp_addr_fail: register_int_gauge_with_registry!(
                Opts::new("dhcp_addr_fail", "DHCP total failed address allocation")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_NETWORK),
                registry,
            )?,

            dns_rx_pkt: register_int_gauge_with_registry!(
                Opts::new("dns_rx_pkt", "DNS total query received")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_NETWORK),
                registry,
            )?,
            dns_rx_timeout: register_int_gauge_with_registry!(
                Opts::new("dns_rx_timeout", "DNS total query timeout")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_NETWORK),
                registry,
            )?,
        };

        let prom = Prom {
            lin,
            kea,
            unbound,
            registry,
            encoder,
            mutex,
            cpu,
            mem,
            fs,
            thermal,
            net,
        };

        Ok(prom)
    }

    fn reset(&self) {
        self.cpu.idle_ms.reset();

        self.mem.total_kb.set(0);
        self.mem.available_kb.set(0);
        self.mem.swap_total_kb.set(0);
        self.mem.swap_free_kb.set(0);

        self.fs.total_kb.reset();
        self.fs.available_kb.reset();
        self.fs.read_kb.reset();
        self.fs.write_kb.reset();

        self.thermal.temp_mc.reset();

        self.net.link_speed_mbps.reset();
        self.net.link_up.reset();
        self.net.link_operstate.reset();
        self.net.link_rx_kb.reset();
        self.net.link_tx_kb.reset();

        self.net.route_default.reset();

        self.net.nft_set_counter_kb.reset();

        self.net.dhcp_rx_pkt.set(0);
        self.net.dhcp_tx_pkt.set(0);
        self.net.dhcp_addr_fail.set(0);

        self.net.dns_rx_pkt.set(0);
        self.net.dns_rx_timeout.set(0);
    }

    pub fn collect(&self) {
        let _lock = self.mutex.lock();
        debug!("collecting metrics");

        self.reset();
        self.lin.collect(self);
        self.kea.collect(self);
        self.unbound.collect(self);
    }

    pub fn format_type(&self) -> &str {
        self.encoder.format_type()
    }

    pub fn encode(&self) -> Vec<u8> {
        let metrics = self.registry.gather();

        let mut buf = Vec::new();
        self.encoder.encode(&metrics, &mut buf).unwrap();

        buf
    }
}
