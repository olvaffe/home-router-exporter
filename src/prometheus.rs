// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use prometheus::{Encoder, IntCounter, Opts, TextEncoder, register_int_counter};

pub struct Prom {
    encoder: TextEncoder,
    counter: IntCounter,
}

impl Prom {
    pub fn new() -> Self {
        let test_opts = Opts::new("counter", "test counter")
            .namespace("home_router")
            .subsystem("test");
        let test_counter = register_int_counter!(test_opts).unwrap();

        Prom {
            encoder: TextEncoder::new(),
            counter: test_counter,
        }
    }

    pub fn format_type(&self) -> &str {
        self.encoder.format_type()
    }

    pub fn update(&self) {
        self.counter.inc();
    }

    pub fn gather(&self) -> Vec<u8> {
        let metrics = prometheus::gather();

        let mut buf = Vec::new();
        self.encoder.encode(&metrics, &mut buf).unwrap();

        buf
    }
}
