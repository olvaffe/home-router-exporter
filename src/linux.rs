// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

mod ethtool;
mod procfs;
mod rtnetlink;
mod sysfs;

use neli::{consts::socket::NlFamily, router::synchronous::NlRouter};
use std::path;

pub struct Linux {
    procfs_path: path::PathBuf,
    sysfs_path: path::PathBuf,

    rt_sock: NlRouter,
    genl_sock: NlRouter,

    ethtool_id: u16,
}

fn nl_socket(family: NlFamily) -> NlRouter {
    let (sock, _) = NlRouter::connect(family, None, neli::utils::Groups::empty()).unwrap();
    sock.enable_ext_ack(true).unwrap();
    sock.enable_strict_checking(true).unwrap();

    sock
}

impl Linux {
    pub fn new(procfs_path: impl AsRef<path::Path>, sysfs_path: impl AsRef<path::Path>) -> Self {
        let rt_sock = nl_socket(NlFamily::Route);
        let genl_sock = nl_socket(NlFamily::Generic);

        let ethtool_id = genl_sock.resolve_genl_family("ethtool").unwrap();

        Linux {
            procfs_path: procfs_path.as_ref().to_path_buf(),
            sysfs_path: sysfs_path.as_ref().to_path_buf(),
            rt_sock,
            genl_sock,
            ethtool_id,
        }
    }
}
