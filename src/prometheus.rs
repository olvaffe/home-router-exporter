// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use prometheus::{Encoder, IntGauge, Opts, TextEncoder, register_int_gauge};

const NAMESPACE: &str = "home_router";

pub struct Prom {
    encoder: TextEncoder,

    cpu_idle_ms: IntGauge,
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

        Prom {
            encoder,
            cpu_idle_ms,
        }
    }

    pub fn format_type(&self) -> &str {
        self.encoder.format_type()
    }

    pub fn update(&self) {
        self.update_cpu();
    }

    fn update_cpu(&self) {
        let stat = crate::procfs::parse_stat().expect("failed to parse /proc/stat");
        self.cpu_idle_ms.set(stat.idle_ms.try_into().unwrap());
    }

    pub fn gather(&self) -> Vec<u8> {
        let metrics = prometheus::gather();

        let mut buf = Vec::new();
        self.encoder.encode(&metrics, &mut buf).unwrap();

        buf
    }
}
