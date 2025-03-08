// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use prometheus::{Encoder, IntGauge, Opts, TextEncoder, register_int_gauge};

const NAMESPACE: &str = "home_router";

pub struct Prom {
    encoder: TextEncoder,

    cpu_idle_ms: IntGauge,

    memory_total_kb: IntGauge,
    memory_available_kb: IntGauge,
    swap_total_kb: IntGauge,
    swap_free_kb: IntGauge,
}

impl Prom {
    pub fn new() -> Self {
        let encoder = TextEncoder::new();
        let cpu_idle_ms = register_int_gauge!(
            Opts::new("idle_ms", "CPU idle time")
                .namespace(NAMESPACE)
                .subsystem("cpu")
        )
        .unwrap();
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

        Prom {
            encoder,
            cpu_idle_ms,
            memory_total_kb,
            memory_available_kb,
            swap_total_kb,
            swap_free_kb,
        }
    }

    pub fn format_type(&self) -> &str {
        self.encoder.format_type()
    }

    pub fn update(&self) {
        self.update_cpu();
        self.update_memory();
    }

    fn update_cpu(&self) {
        let stat = crate::procfs::parse_stat().expect("failed to parse /proc/stat");
        self.cpu_idle_ms.set(stat.idle_ms.try_into().unwrap());
    }

    fn update_memory(&self) {
        let meminfo = crate::procfs::parse_meminfo().expect("failed to parse /proc/meminfo");
        self.memory_total_kb.set(meminfo.mem_total_kb.try_into().unwrap());
        self.memory_available_kb.set(meminfo.mem_avail_kb.try_into().unwrap());
        self.swap_total_kb.set(meminfo.swap_total_kb.try_into().unwrap());
        self.swap_free_kb.set(meminfo.swap_free_kb.try_into().unwrap());
    }

    pub fn gather(&self) -> Vec<u8> {
        let metrics = prometheus::gather();

        let mut buf = Vec::new();
        self.encoder.encode(&metrics, &mut buf).unwrap();

        buf
    }
}
