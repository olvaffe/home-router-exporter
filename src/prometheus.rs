// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use prometheus::{Encoder, IntCounter, Opts, TextEncoder, register_int_counter};

pub struct Prom {
    counter: IntCounter,
}

impl Prom {
    pub fn new() -> Self {
        let test_opts = Opts::new("counter", "test counter")
            .namespace("home_router")
            .subsystem("test");
        let test_counter = register_int_counter!(test_opts).unwrap();

        Prom {
            counter: test_counter,
        }
    }

    pub fn collect(self: &Self) {
        self.counter.inc();

        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer).unwrap();

        println!("{:?}", std::str::from_utf8(&buffer).unwrap());
    }
}
