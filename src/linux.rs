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

fn read_string(path: impl AsRef<path::Path>) -> Result<String> {
    let mut s =
        fs::read_to_string(&path).with_context(|| format!("failed to read {:?}", path.as_ref()))?;
    s.truncate(s.trim_end().len());
    Ok(s)
}

fn read_u64(path: impl AsRef<path::Path>) -> Result<u64> {
    let s = read_string(path)?;
    Ok(s.parse::<u64>()?)
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
            sysconf_nproc: crate::libc::sysconf_nproc(),
            sysconf_user_hz: crate::libc::sysconf_user_hz(),
        }
    }

    fn procfs_open(&self, file: &str) -> Result<impl io::BufRead> {
        let path = self.procfs_path.join(file);
        let fp = fs::File::open(&path).with_context(|| format!("failed to open {:?}", path))?;
        Ok(io::BufReader::new(fp))
    }

    fn sysfs_read_dir(&self, dir: &str) -> Result<fs::ReadDir> {
        let path = self.sysfs_path.join(dir);
        fs::read_dir(&path).with_context(|| format!("failed to open {:?}", path))
    }
}
