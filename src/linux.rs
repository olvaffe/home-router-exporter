// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

mod ethtool;
mod procfs;
mod rtnetlink;
mod sysfs;

use anyhow::{Context, Result};
use neli::{consts::socket::NlFamily, router::synchronous::NlRouter};
use std::{fs, io, path};

pub struct Linux {
    procfs_path: path::PathBuf,
    sysfs_path: path::PathBuf,

    rt_sock: NlRouter,
    genl_sock: NlRouter,

    ethtool_id: u16,

    sysconf_nproc: u64,
    sysconf_user_hz: u64,
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

        // SAFETY: valid sysconf call with validation
        let mut sysconf_nproc = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) };
        if sysconf_nproc <= 0 {
            sysconf_nproc = 1;
        }

        // SAFETY: valid sysconf call with validation
        let mut sysconf_user_hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
        if sysconf_user_hz <= 0 {
            sysconf_user_hz = 100;
        }

        Linux {
            procfs_path: procfs_path.as_ref().to_path_buf(),
            sysfs_path: sysfs_path.as_ref().to_path_buf(),
            rt_sock,
            genl_sock,
            ethtool_id,
            sysconf_nproc: sysconf_nproc as _,
            sysconf_user_hz: sysconf_user_hz as _,
        }
    }

    fn procfs_open(&self, file: &str) -> Result<impl io::BufRead> {
        let path = self.procfs_path.join(file);
        let fp = fs::File::open(&path).with_context(|| format!("failed to open {:?}", path))?;
        Ok(io::BufReader::new(fp))
    }
}
