// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

#![warn(missing_docs)]

//! Home Router Exporter is a Prometheus exporter designed for home routers.

mod collector;
mod config;
mod hyper;
mod libc;
mod metric;

use log::error;

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

#[tokio::main]
async fn main() {
    config::get();
    init_logger();

    let collector = match collector::Collector::new() {
        Ok(collector) => collector,
        Err(err) => {
            error!("failed to initialize collector: {err:?}");
            return;
        }
    };

    if let Err(err) = hyper::run(collector).await {
        error!("failed to start web server: {err:?}");
    }
}
