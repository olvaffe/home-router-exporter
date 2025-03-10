// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use crate::collector::{linux, ping, unbound};
use prometheus::{
    Encoder, IntGauge, IntGaugeVec, Opts, TextEncoder, register_int_gauge, register_int_gauge_vec,
};
use std::sync;

const NAMESPACE: &str = "home_router";
const SUBSYS_CPU: &str = "cpu";
const SUBSYS_MEMORY: &str = "memory";
const SUBSYS_FILESYSTEM: &str = "filesystem";
const SUBSYS_THERMAL: &str = "thermal";
const SUBSYS_IO: &str = "io";
const SUBSYS_NET: &str = "net";

pub struct CpuMetrics {
    pub idle_ms: IntGauge,
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
}

pub struct ThermalMetrics {
    pub current_mc: IntGaugeVec,
}

pub struct IoMetrics {
    pub read_kb: IntGaugeVec,
    pub write_kb: IntGaugeVec,
}

pub struct Prom {
    lin: linux::Linux,
    unbound: sync::Arc<unbound::Unbound>,
    ping: sync::Arc<ping::Ping>,

    encoder: TextEncoder,

    pub cpu: CpuMetrics,
    pub mem: MemoryMetrics,
    pub fs: FilesystemMetrics,
    pub thermal: ThermalMetrics,
    pub io: IoMetrics,

    /* net */
    pub net_rx_kb: IntGaugeVec,
    pub net_tx_kb: IntGaugeVec,
    pub net_link_speed: IntGaugeVec,
    pub net_gateway_latency: IntGaugeVec,
    pub net_dns_query_count: IntGauge,
}

impl Prom {
    pub fn new(
        lin: linux::Linux,
        unbound: sync::Arc<unbound::Unbound>,
        ping: sync::Arc<ping::Ping>,
    ) -> Self {
        let encoder = TextEncoder::new();

        let cpu = CpuMetrics {
            idle_ms: register_int_gauge!(
                Opts::new("idle_ms", "CPU idle time")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_CPU)
            )
            .unwrap(),
        };

        let mem = MemoryMetrics {
            total_kb: register_int_gauge!(
                Opts::new("total_kb", "Total memory size")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_MEMORY)
            )
            .unwrap(),
            available_kb: register_int_gauge!(
                Opts::new("available_kb", "Available memory size")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_MEMORY)
            )
            .unwrap(),
            swap_total_kb: register_int_gauge!(
                Opts::new("swap_total_kb", "Total swap size")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_MEMORY)
            )
            .unwrap(),
            swap_free_kb: register_int_gauge!(
                Opts::new("swap_free_kb", "Free swap size")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_MEMORY)
            )
            .unwrap(),
        };

        let fs = FilesystemMetrics {
            total_kb: register_int_gauge_vec!(
                Opts::new("total_kb", "Total filesystem size")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_FILESYSTEM),
                &["src", "dst"]
            )
            .unwrap(),
            available_kb: register_int_gauge_vec!(
                Opts::new("available_kb", "Available filesystem size")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_FILESYSTEM),
                &["src", "dst"]
            )
            .unwrap(),
        };

        let thermal = ThermalMetrics {
            current_mc: register_int_gauge_vec!(
                Opts::new("current_mc", "Current temperature")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_THERMAL),
                &["type"]
            )
            .unwrap(),
        };

        let io = IoMetrics {
            read_kb: register_int_gauge_vec!(
                Opts::new("read_kb", "Total read size")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_IO),
                &["block"]
            )
            .unwrap(),
            write_kb: register_int_gauge_vec!(
                Opts::new("write_kb", "Total write size")
                    .namespace(NAMESPACE)
                    .subsystem(SUBSYS_IO),
                &["block"]
            )
            .unwrap(),
        };

        /* net */
        let net_rx_kb = register_int_gauge_vec!(
            Opts::new("rx_kb", "Total rx size")
                .namespace(NAMESPACE)
                .subsystem(SUBSYS_NET),
            &["netdev"]
        )
        .unwrap();
        let net_tx_kb = register_int_gauge_vec!(
            Opts::new("tx_kb", "Total tx size")
                .namespace(NAMESPACE)
                .subsystem(SUBSYS_NET),
            &["netdev"]
        )
        .unwrap();
        let net_link_speed = register_int_gauge_vec!(
            Opts::new("link_speed", "Link speed")
                .namespace(NAMESPACE)
                .subsystem(SUBSYS_NET),
            &["netdev"]
        )
        .unwrap();
        let net_gateway_latency = register_int_gauge_vec!(
            Opts::new("gateway_latency", "Gateway latency")
                .namespace(NAMESPACE)
                .subsystem(SUBSYS_NET),
            &["gateway"]
        )
        .unwrap();
        let net_dns_query_count = register_int_gauge!(
            Opts::new("dns_query_count", "DNS total query count")
                .namespace(NAMESPACE)
                .subsystem(SUBSYS_NET)
        )
        .unwrap();

        Prom {
            lin,
            unbound,
            ping,
            encoder,
            cpu,
            mem,
            fs,
            thermal,
            io,
            net_rx_kb,
            net_tx_kb,
            net_link_speed,
            net_gateway_latency,
            net_dns_query_count,
        }
    }

    pub fn collect(&self) {
        self.ping.set_hosts(self.lin.get_gateways());

        self.lin.collect(self);
        self.unbound.collect(self);
        self.ping.collect(self);
    }

    pub fn format_type(&self) -> &str {
        self.encoder.format_type()
    }

    pub fn encode(&self) -> Vec<u8> {
        let metrics = prometheus::gather();

        let mut buf = Vec::new();
        self.encoder.encode(&metrics, &mut buf).unwrap();

        buf
    }
}
