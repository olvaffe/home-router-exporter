// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

mod ethtool;
mod nfnetlink;
mod procfs;
mod rtnetlink;
mod sysfs;

use crate::config;
use crate::prometheus::Prom;
use anyhow::{Context, Result};
use log::{debug, error};
use neli::{consts::socket::NlFamily, router::synchronous::NlRouter};
use std::{fs, io, path};

pub struct Linux {
    procfs_path: &'static path::Path,
    sysfs_path: &'static path::Path,

    rt_sock: NlRouter,
    nf_sock: NlRouter,
    genl_sock: NlRouter,

    ethtool_id: u16,

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

fn nl_socket(family: NlFamily) -> Result<NlRouter> {
    let (sock, _) = NlRouter::connect(family, None, neli::utils::Groups::empty())?;
    sock.enable_ext_ack(true)?;
    sock.enable_strict_checking(true)?;

    Ok(sock)
}

impl Linux {
    pub fn new() -> Result<Self> {
        let config = config::get();

        let rt_sock = nl_socket(NlFamily::Route)?;
        let nf_sock = nl_socket(NlFamily::Netfilter)?;
        let genl_sock = nl_socket(NlFamily::Generic)?;

        let ethtool_id = genl_sock.resolve_genl_family(ethtool::ETHTOOL_GENL_NAME)?;

        let lin = Linux {
            procfs_path: config.procfs_path,
            sysfs_path: config.sysfs_path,
            rt_sock,
            nf_sock,
            genl_sock,
            ethtool_id,
            sysconf_user_hz: crate::libc::sysconf_user_hz(),
        };

        Ok(lin)
    }

    pub fn collect(&self, prom: &Prom) {
        if let Err(err) = self.collect_cpu(prom) {
            error!("failed to collect cpu metrics: {err:?}");
        }

        if let Err(err) = self.collect_mem(prom) {
            error!("failed to collect mem metrics: {err:?}");
        }

        if let Err(err) = self.collect_fs(prom) {
            error!("failed to collect fs metrics: {err:?}");
        }

        if let Err(err) = self.collect_thermal(prom) {
            error!("failed to collect thermal metrics: {err:?}");
        }

        if let Err(err) = self.collect_net_link_speed(prom) {
            error!("failed to collect net link speed: {err:?}");
        }

        if let Err(err) = self.collect_net_link_state(prom) {
            error!("failed to collect net link state: {err:?}");
        }

        if let Err(err) = self.collect_net_route(prom) {
            error!("failed to collect net route: {err:?}");
        }

        if let Err(err) = self.collect_net_nft(prom) {
            let mut level = log::Level::Error;
            if let Some(err) = err.downcast_ref::<io::Error>() {
                if err.kind() == io::ErrorKind::PermissionDenied {
                    level = log::Level::Debug;
                }
            }

            log::log!(level, "failed to collect net nft: {err:?}");
        }
    }

    fn collect_cpu(&self, prom: &Prom) -> Result<()> {
        let stats = self.parse_stat()?;
        for stat in stats {
            let stat = stat?;

            let idle_ms = stat.idle_ticks * 1000 / self.sysconf_user_hz;
            prom.cpu
                .idle_ms
                .with_label_values(&[&stat.cpu])
                .set(idle_ms as _);
        }

        Ok(())
    }

    fn collect_mem(&self, prom: &Prom) -> Result<()> {
        let meminfo = self.parse_meminfo()?;

        prom.mem.total_kb.set(meminfo.mem_total_kb as _);
        prom.mem.available_kb.set(meminfo.mem_avail_kb as _);
        prom.mem.swap_total_kb.set(meminfo.swap_total_kb as _);
        prom.mem.swap_free_kb.set(meminfo.swap_free_kb as _);

        Ok(())
    }

    fn collect_fs(&self, prom: &Prom) -> Result<()> {
        let mountinfos = self.parse_self_mountinfo()?;
        for info in mountinfos {
            let info = info?;

            prom.fs
                .total_kb
                .with_label_values(&[&info.mount_source, &info.mount_point])
                .set((info.total / 1024) as _);
            prom.fs
                .available_kb
                .with_label_values(&[&info.mount_source, &info.mount_point])
                .set((info.avail / 1024) as _);

            match self.parse_dev_block(&info.major_minor) {
                Ok(iostats) => {
                    prom.fs
                        .read_kb
                        .with_label_values(&[&info.mount_source, &info.mount_point])
                        .set((iostats.read_bytes / 1024) as _);
                    prom.fs
                        .write_kb
                        .with_label_values(&[&info.mount_source, &info.mount_point])
                        .set((iostats.write_bytes / 1024) as _);
                }
                Err(err) => debug!("failed to collect iostats: {err:?}"),
            }
        }

        Ok(())
    }

    fn collect_thermal(&self, prom: &Prom) -> Result<()> {
        let zones = self.parse_class_thermal()?;
        for zone in zones {
            let zone = zone?;

            prom.thermal
                .temp_mc
                .with_label_values(&[&zone.name])
                .set(zone.temp as _);
        }

        Ok(())
    }

    fn collect_net_link_speed(&self, prom: &Prom) -> Result<()> {
        let speeds = self.parse_ethtool()?;
        for speed in speeds {
            let speed = speed?;

            prom.net
                .link_speed_mbps
                .with_label_values(&[&speed.name])
                .set(speed.speed as _);
        }

        Ok(())
    }

    fn collect_net_link_state(&self, prom: &Prom) -> Result<()> {
        let links = self.parse_links()?;
        for link in links {
            let link = link?;

            prom.net
                .link_up
                .with_label_values(&[&link.name])
                .set(link.admin_up as _);
            prom.net
                .link_operstate
                .with_label_values(&[&link.name])
                .set(link.operstate as _);
            prom.net
                .link_rx_kb
                .with_label_values(&[&link.name])
                .set((link.rx / 1024) as _);
            prom.net
                .link_tx_kb
                .with_label_values(&[&link.name])
                .set((link.tx / 1024) as _);
        }

        Ok(())
    }

    fn collect_net_route(&self, prom: &Prom) -> Result<()> {
        let routes = self.parse_routes()?;
        for route in routes {
            let route = route?;

            prom.net
                .route_default
                .with_label_values(&[&route.ip().to_string()])
                .set(1);
        }

        Ok(())
    }

    fn collect_net_nft(&self, prom: &Prom) -> Result<()> {
        let sets = self.parse_nfnetlink()?;
        for set in sets {
            let set = set?;

            let counters = self.parse_nft_set(&set)?;
            for counter in counters {
                let counter = counter?;

                prom.net
                    .nft_set_counter_kb
                    .with_label_values(&[
                        &set.family.to_string(),
                        &set.table,
                        &set.name,
                        &counter.addr,
                    ])
                    .set((counter.bytes / 1024) as _)
            }
        }

        Ok(())
    }

    fn procfs_open(&self, file: &str) -> Result<io::BufReader<fs::File>> {
        let path = self.procfs_path.join(file);
        let fp = fs::File::open(&path).with_context(|| format!("failed to open {:?}", path))?;
        Ok(io::BufReader::new(fp))
    }

    fn sysfs_open(&self, file: &str) -> Result<io::BufReader<fs::File>> {
        let path = self.sysfs_path.join(file);
        let fp = fs::File::open(&path).with_context(|| format!("failed to open {:?}", path))?;
        Ok(io::BufReader::new(fp))
    }

    fn sysfs_read_dir(&self, dir: &str) -> Result<fs::ReadDir> {
        let path = self.sysfs_path.join(dir);
        fs::read_dir(&path).with_context(|| format!("failed to open {:?}", path))
    }
}
