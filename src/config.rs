// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use clap::{Arg, ArgAction, Command};
use std::{path, sync};

pub struct Config {
    pub debug: bool,
    pub procfs_path: &'static path::Path,
    pub sysfs_path: &'static path::Path,
    pub kea_socket: path::PathBuf,
    pub unbound_socket: path::PathBuf,
    pub hyper_addr: String,
}

fn parse_args() -> Config {
    let matches = Command::new("home-router-exporter")
        .arg(
            Arg::new("debug")
                .long("debug")
                .short('d')
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("addr")
                .long("web.listen-address")
                .default_value("0.0.0.0:9527"),
        )
        .arg(
            Arg::new("kea_socket")
                .long("collector.kea.socket")
                .default_value("/run/kea/kea4-ctrl-socket"),
        )
        .arg(
            Arg::new("unbound_socket")
                .long("collector.unbound.socket")
                .default_value("/run/unbound.ctl"),
        )
        .get_matches();

    let debug = matches.get_flag("debug");
    let procfs_path = path::Path::new("/proc");
    let sysfs_path = path::Path::new("/sys");
    let kea_socket = path::PathBuf::from(matches.get_one::<String>("kea_socket").unwrap());
    let unbound_socket = path::PathBuf::from(matches.get_one::<String>("unbound_socket").unwrap());
    let hyper_addr = matches.get_one::<String>("addr").unwrap().clone();

    Config {
        debug,
        procfs_path,
        sysfs_path,
        kea_socket,
        unbound_socket,
        hyper_addr,
    }
}

pub fn get() -> &'static Config {
    static CONFIG: sync::LazyLock<Config> = sync::LazyLock::new(parse_args);
    &CONFIG
}
