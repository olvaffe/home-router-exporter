// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

mod ethtool;
mod procfs;
mod rtnetlink;
mod sysfs;

use crate::prometheus::Prom;
use anyhow::{Context, Result};
use neli::{consts::socket::NlFamily, router::synchronous::NlRouter};
use std::{fs, io, net, path};

pub struct Linux {
    procfs_path: path::PathBuf,
    sysfs_path: path::PathBuf,

    // TODO: mutex?
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

    pub fn get_gateways(&self) -> Vec<net::SocketAddr> {
        self.parse_routes().unwrap_or_default()
    }

    pub fn collect(&self, prom: &Prom) {
        self.collect_cpu(prom);
        self.collect_mem(prom);
        self.collect_fs(prom);
        self.collect_thermal(prom);
        self.collect_io(prom);
        self.collect_net(prom);
    }

    fn collect_cpu(&self, prom: &Prom) {
        if let Ok(stat) = self.parse_stat() {
            prom.cpu.idle_ms.set(stat.idle_ms.try_into().unwrap());
        }
    }

    fn collect_mem(&self, prom: &Prom) {
        if let Ok(meminfo) = self.parse_meminfo() {
            prom.mem.total_kb
                .set(meminfo.mem_total_kb.try_into().unwrap());
            prom.mem.available_kb
                .set(meminfo.mem_avail_kb.try_into().unwrap());
            prom.mem.swap_total_kb
                .set(meminfo.swap_total_kb.try_into().unwrap());
            prom.mem.swap_free_kb
                .set(meminfo.swap_free_kb.try_into().unwrap());
        }
    }

    fn collect_fs(&self, prom: &Prom) {
        // TODO iterator
        if let Ok(mountinfos) = self.parse_self_mountinfo() {
            for info in mountinfos {
                prom.fs_total_kb
                    .with_label_values(&[&info.mount_source, &info.mount_point])
                    .set((info.total / 1024).try_into().unwrap());
                prom.fs_available_kb
                    .with_label_values(&[&info.mount_source, &info.mount_point])
                    .set((info.avail / 1024).try_into().unwrap());
            }
        }
    }

    fn collect_thermal(&self, prom: &Prom) {
        // TODO iterator
        if let Ok(zones) = self.parse_class_thermal() {
            for zone in zones {
                prom.thermal_current_mc
                    .with_label_values(&[&zone.name])
                    .set((zone.temp).try_into().unwrap());
            }
        }
    }

    fn collect_io(&self, prom: &Prom) {
        // TODO iterator
        if let Ok(diskstats) = self.parse_diskstats() {
            for stat in diskstats {
                prom.io_read_kb
                    .with_label_values(&[&stat.name])
                    .set((stat.read_bytes / 1024).try_into().unwrap());
                prom.io_write_kb
                    .with_label_values(&[&stat.name])
                    .set((stat.write_bytes / 1024).try_into().unwrap());
            }
        }
    }

    fn collect_net(&self, prom: &Prom) {
        // TODO iterators

        if let Ok(links) = self.parse_links() {
            for link in links {
                prom.net_rx_kb
                    .with_label_values(&[&link.name])
                    .set((link.rx / 1024).try_into().unwrap());
                prom.net_tx_kb
                    .with_label_values(&[&link.name])
                    .set((link.tx / 1024).try_into().unwrap());
            }
        }

        if let Ok(speeds) = self.parse_ethtool() {
            for speed in speeds {
                prom.net_link_speed
                    .with_label_values(&[&speed.name])
                    .set(speed.speed as _);
            }
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
