// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use crate::collector::{self, kea, linux, unbound};
use crate::metric;
use anyhow::Result;
use log::debug;
use prometheus::{
    Encoder, IntGauge, Opts, Registry, TextEncoder, register_int_gauge_with_registry,
};
use std::sync;

const NAMESPACE: &str = "home_router";
const SUBSYS_NETWORK: &str = "network";

pub struct NetworkMetrics {
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
    pub metrics: collector::Metrics,
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
        let metrics = collector::Metrics::new();
        let mutex = sync::Mutex::new(());

        let net = NetworkMetrics {
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
            metrics,
            mutex,
            net,
        };

        Ok(prom)
    }

    fn reset(&self) {
        self.net.dhcp_rx_pkt.set(0);
        self.net.dhcp_tx_pkt.set(0);
        self.net.dhcp_addr_fail.set(0);

        self.net.dns_rx_pkt.set(0);
        self.net.dns_rx_timeout.set(0);
    }

    pub fn collect(&self) {
        let _lock = self.mutex.lock();
        debug!("collecting metrics");

        let mut enc = metric::Encoder::new(self.metrics.namespace);

        self.reset();
        self.lin.collect(self, &mut enc);
        self.kea.collect(self);
        self.unbound.collect(self);

        println!("{}", enc.take());
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
