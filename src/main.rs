// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

#![warn(missing_docs)]

//! Home Router Exporter is a Prometheus exporter designed for home routers.

mod collector;
mod config;
mod hyper;
mod libc;
mod prometheus;

use anyhow::Result;
use collector::{kea, linux, unbound};
use log::error;
use prometheus::Prom;

fn init_logger() {
    let module = "home_router_exporter";
    let module_filter = if config::get().debug {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    env_logger::Builder::from_default_env()
        .filter_module(module, module_filter)
        .init();
}

fn init_prometheus() -> Result<Prom> {
    let lin = linux::Linux::new()?;
    let kea = kea::Kea::new()?;
    let unbound = unbound::Unbound::new();

    prometheus::Prom::new(lin, kea, unbound)
}

#[tokio::main]
async fn main() {
    init_logger();

    let prom = init_prometheus();
    if let Err(err) = &prom {
        error!("failed to initialize prometheus: {err:?}");
        return;
    }
    let prom = prom.unwrap();

    if let Err(err) = hyper::run(prom).await {
        error!("failed to start web server: {err:?}");
    }
}
