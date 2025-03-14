// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

pub mod kea;
pub mod linux;
pub mod unbound;

use crate::metric;

const NAMESPACE: &str = "homerouter";
const SUBSYS_CPU: &str = "cpu";
const SUBSYS_MEMORY: &str = "memory";
const SUBSYS_FILESYSTEM: &str = "filesystem";
const SUBSYS_THERMAL: &str = "thermal";
const SUBSYS_NETWORK: &str = "network";

pub struct CpuMetrics {
    pub idle: metric::Info<1>,
}

pub struct MemoryMetrics {
    pub size: metric::Info<0>,
    pub available: metric::Info<0>,
    pub swap_size: metric::Info<0>,
    pub swap_free: metric::Info<0>,
}

pub struct FilesystemMetrics {
    pub size: metric::Info<2>,
    pub available: metric::Info<2>,
    pub read: metric::Info<2>,
    pub write: metric::Info<2>,
}

pub struct ThermalMetrics {
    pub temperature: metric::Info<1>,
}

pub struct NetworkMetrics {
    pub link_speed: metric::Info<1>,

    pub link_up: metric::Info<1>,
    pub link_operstate: metric::Info<1>,
    pub link_rx: metric::Info<1>,
    pub link_tx: metric::Info<1>,

    pub route_default: metric::Info<1>,

    pub nft_set_counter: metric::Info<4>,
}

pub struct Metrics {
    pub namespace: &'static str,
    pub cpu: CpuMetrics,
    pub mem: MemoryMetrics,
    pub fs: FilesystemMetrics,
    pub thermal: ThermalMetrics,
    pub net: NetworkMetrics,
}

impl Metrics {
    pub fn new() -> Self {
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
        };

        Metrics {
            namespace: NAMESPACE,
            cpu,
            mem,
            fs,
            thermal,
            net,
        }
    }
}
