// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

mod kea;
mod linux;
mod unbound;

use crate::metric;
use anyhow::Result;
use log::debug;
use std::sync;

const NAMESPACE: &str = "homerouter";
const SUBSYS_CPU: &str = "cpu";
const SUBSYS_MEMORY: &str = "memory";
const SUBSYS_FILESYSTEM: &str = "filesystem";
const SUBSYS_THERMAL: &str = "thermal";
const SUBSYS_NETWORK: &str = "network";

struct CpuMetrics {
    idle: metric::Info<1>,
}

struct MemoryMetrics {
    size: metric::Info<0>,
    available: metric::Info<0>,
    swap_size: metric::Info<0>,
    swap_free: metric::Info<0>,
}

struct FilesystemMetrics {
    size: metric::Info<2>,
    available: metric::Info<2>,
    read: metric::Info<2>,
    write: metric::Info<2>,
}

struct ThermalMetrics {
    temperature: metric::Info<1>,
}

struct NetworkMetrics {
    link_speed: metric::Info<1>,

    link_up: metric::Info<1>,
    link_operstate: metric::Info<1>,
    link_rx: metric::Info<1>,
    link_tx: metric::Info<1>,

    route_default: metric::Info<1>,

    nft_set_counter: metric::Info<4>,

    dhcp_received: metric::Info<0>,
    dhcp_sent: metric::Info<0>,
    dhcp_addr_fail: metric::Info<0>,

    dns_query: metric::Info<0>,
    dns_timeout: metric::Info<0>,
}

struct Metrics {
    cpu: CpuMetrics,
    mem: MemoryMetrics,
    fs: FilesystemMetrics,
    thermal: ThermalMetrics,
    net: NetworkMetrics,
}

impl Metrics {
    fn new() -> Self {
        let cpu = CpuMetrics {
            idle: metric::Info {
                subsys: SUBSYS_CPU,
                name: "idle",
                help: "CPU idle time",
                unit: metric::Unit::Seconds,
                ty: metric::Type::Counter,
                label_keys: ["cpu"],
            },
        };

        let mem = MemoryMetrics {
            size: metric::Info {
                subsys: SUBSYS_MEMORY,
                name: "size",
                help: "Total memory size",
                unit: metric::Unit::Bytes,
                ty: metric::Type::Gauge,
                label_keys: [],
            },
            available: metric::Info {
                subsys: SUBSYS_MEMORY,
                name: "available",
                help: "Estimated available memory size",
                unit: metric::Unit::Bytes,
                ty: metric::Type::Gauge,
                label_keys: [],
            },
            swap_size: metric::Info {
                subsys: SUBSYS_MEMORY,
                name: "swap_size",
                help: "Total swap size",
                unit: metric::Unit::Bytes,
                ty: metric::Type::Gauge,
                label_keys: [],
            },
            swap_free: metric::Info {
                subsys: SUBSYS_MEMORY,
                name: "swap_free",
                help: "Free swap size",
                unit: metric::Unit::Bytes,
                ty: metric::Type::Gauge,
                label_keys: [],
            },
        };

        let fs = FilesystemMetrics {
            size: metric::Info {
                subsys: SUBSYS_FILESYSTEM,
                name: "size",
                help: "Total filesystem size",
                unit: metric::Unit::Bytes,
                ty: metric::Type::Gauge,
                label_keys: ["device", "mountpoint"],
            },
            available: metric::Info {
                subsys: SUBSYS_FILESYSTEM,
                name: "available",
                help: "Available filesystem size",
                unit: metric::Unit::Bytes,
                ty: metric::Type::Gauge,
                label_keys: ["device", "mountpoint"],
            },
            read: metric::Info {
                subsys: SUBSYS_FILESYSTEM,
                name: "read",
                help: "Total read size",
                unit: metric::Unit::Bytes,
                ty: metric::Type::Counter,
                label_keys: ["device", "mountpoint"],
            },
            write: metric::Info {
                subsys: SUBSYS_FILESYSTEM,
                name: "write",
                help: "Total write size",
                unit: metric::Unit::Bytes,
                ty: metric::Type::Counter,
                label_keys: ["device", "mountpoint"],
            },
        };

        let thermal = ThermalMetrics {
            temperature: metric::Info {
                subsys: SUBSYS_THERMAL,
                name: "temperature",
                help: "Current temperature",
                unit: metric::Unit::Celsius,
                ty: metric::Type::Gauge,
                label_keys: ["device"],
            },
        };

        let net = NetworkMetrics {
            link_speed: metric::Info {
                subsys: SUBSYS_NETWORK,
                name: "link_speed",
                help: "Link speed",
                unit: metric::Unit::Bytes,
                ty: metric::Type::Gauge,
                label_keys: ["device"],
            },

            link_up: metric::Info {
                subsys: SUBSYS_NETWORK,
                name: "link_up",
                help: "Link administrative state",
                unit: metric::Unit::None,
                ty: metric::Type::Gauge,
                label_keys: ["device"],
            },
            link_operstate: metric::Info {
                subsys: SUBSYS_NETWORK,
                name: "link_operstate",
                help: "Link operational state",
                unit: metric::Unit::None,
                ty: metric::Type::Gauge,
                label_keys: ["device"],
            },
            link_rx: metric::Info {
                subsys: SUBSYS_NETWORK,
                name: "link_rx",
                help: "Total rx size",
                unit: metric::Unit::Bytes,
                ty: metric::Type::Counter,
                label_keys: ["device"],
            },
            link_tx: metric::Info {
                subsys: SUBSYS_NETWORK,
                name: "link_tx",
                help: "Total tx size",
                unit: metric::Unit::Bytes,
                ty: metric::Type::Counter,
                label_keys: ["device"],
            },

            route_default: metric::Info {
                subsys: SUBSYS_NETWORK,
                name: "route_default",
                help: "Default route",
                unit: metric::Unit::Info,
                ty: metric::Type::Gauge,
                label_keys: ["gateway"],
            },

            nft_set_counter: metric::Info {
                subsys: SUBSYS_NETWORK,
                name: "nft_set_counter",
                help: "Nftables set counter",
                unit: metric::Unit::Bytes,
                ty: metric::Type::Counter,
                label_keys: ["family", "table", "set", "key"],
            },

            dhcp_received: metric::Info {
                subsys: SUBSYS_NETWORK,
                name: "dhcp_received",
                help: "DHCP total packet received",
                unit: metric::Unit::Packets,
                ty: metric::Type::Counter,
                label_keys: [],
            },
            dhcp_sent: metric::Info {
                subsys: SUBSYS_NETWORK,
                name: "dhcp_sent",
                help: "DHCP total packet sent",
                unit: metric::Unit::Packets,
                ty: metric::Type::Counter,
                label_keys: [],
            },
            dhcp_addr_fail: metric::Info {
                subsys: SUBSYS_NETWORK,
                name: "dhcp_addr_fail",
                help: "DHCP total failed address allocation",
                unit: metric::Unit::None,
                ty: metric::Type::Counter,
                label_keys: [],
            },

            dns_query: metric::Info {
                subsys: SUBSYS_NETWORK,
                name: "dns_query",
                help: "DHCP total query count",
                unit: metric::Unit::None,
                ty: metric::Type::Counter,
                label_keys: [],
            },
            dns_timeout: metric::Info {
                subsys: SUBSYS_NETWORK,
                name: "dns_timeout",
                help: "DHCP total query timeout",
                unit: metric::Unit::None,
                ty: metric::Type::Counter,
                label_keys: [],
            },
        };

        Metrics {
            cpu,
            mem,
            fs,
            thermal,
            net,
        }
    }
}

pub struct Collector {
    lin: linux::Linux,
    kea: sync::Arc<kea::Kea>,
    unbound: sync::Arc<unbound::Unbound>,

    metrics: Metrics,
}

impl Collector {
    pub fn new() -> Result<Self> {
        debug!("creating collector");

        let lin = linux::Linux::new()?;
        let kea = kea::Kea::new()?;
        let unbound = unbound::Unbound::new();

        let metrics = Metrics::new();

        Ok(Collector {
            lin,
            kea,
            unbound,
            metrics,
        })
    }

    pub fn content_type() -> &'static str {
        "text/plain; version=0.0.4"
    }

    pub fn collect(&self) -> String {
        debug!("collecting metrics");

        let mut buf = String::with_capacity(4096);
        let mut enc = metric::Encoder::new(&mut buf, NAMESPACE);

        self.lin.collect(&self.metrics, &mut enc);
        self.kea.collect(&self.metrics, &mut enc);
        self.unbound.collect(&self.metrics, &mut enc);

        buf
    }
}
