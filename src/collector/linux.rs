// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

mod ethtool;
mod nfnetlink;
mod procfs;
mod rtnetlink;
mod sysfs;

use crate::{collector, config, metric};
use anyhow::{Context, Result};
use log::error;
use neli::{consts::socket::NlFamily, router::synchronous::NlRouter};
use std::{fs, io, path};

pub(super) struct Linux {
    procfs_path: &'static path::Path,
    sysfs_path: &'static path::Path,

    rt_sock: NlRouter,
    nf_sock: NlRouter,
    genl_sock: NlRouter,

    ethtool_id: u16,

    sysconf_page_size: u64,
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
            sysconf_page_size: crate::libc::sysconf_page_size(),
            sysconf_user_hz: crate::libc::sysconf_user_hz(),
        };

        Ok(lin)
    }

    pub fn collect(&self, metrics: &collector::Metrics, enc: &mut metric::Encoder) {
        if let Err(err) = self.collect_cpu(metrics, enc) {
            error!("failed to collect cpu metrics: {err:?}");
        }

        if let Err(err) = self.collect_mem_info(metrics, enc) {
            error!("failed to collect mem info metrics: {err:?}");
        }

        if let Err(err) = self.collect_mem_vm(metrics, enc) {
            error!("failed to collect mem vm metrics: {err:?}");
        }

        if let Err(err) = self.collect_fs(metrics, enc) {
            error!("failed to collect fs metrics: {err:?}");
        }

        if let Err(err) = self.collect_thermal(metrics, enc) {
            error!("failed to collect thermal metrics: {err:?}");
        }

        if let Err(err) = self.collect_net_link_speed(metrics, enc) {
            error!("failed to collect net link speed: {err:?}");
        }

        if let Err(err) = self.collect_net_link_state(metrics, enc) {
            error!("failed to collect net link state: {err:?}");
        }

        if let Err(err) = self.collect_net_route(metrics, enc) {
            error!("failed to collect net route: {err:?}");
        }

        if let Err(err) = self.collect_net_nft(metrics, enc) {
            let mut level = log::Level::Error;
            if let Some(err) = err.downcast_ref::<io::Error>() {
                if err.kind() == io::ErrorKind::PermissionDenied {
                    level = log::Level::Debug;
                }
            }

            log::log!(level, "failed to collect net nft: {err:?}");
        }
    }

    fn collect_cpu(&self, metrics: &collector::Metrics, enc: &mut metric::Encoder) -> Result<()> {
        let stats = self.parse_stat()?;

        let mut menc = enc.with_info(&metrics.cpu.idle, None);
        for stat in stats {
            let stat = stat?;

            let idle_s = stat.idle_ticks as f64 / self.sysconf_user_hz as f64;
            menc.write(&[&stat.cpu], idle_s);
        }

        Ok(())
    }

    fn collect_mem_info(
        &self,
        metrics: &collector::Metrics,
        enc: &mut metric::Encoder,
    ) -> Result<()> {
        let meminfo = self.parse_meminfo().unwrap_or_default();

        enc.write(&metrics.mem.size, meminfo.mem_total_kb * 1024, None);
        enc.write(&metrics.mem.available, meminfo.mem_avail_kb * 1024, None);
        enc.write(&metrics.mem.swap_size, meminfo.swap_total_kb * 1024, None);
        enc.write(&metrics.mem.swap_free, meminfo.swap_free_kb * 1024, None);

        Ok(())
    }

    fn collect_mem_vm(
        &self,
        metrics: &collector::Metrics,
        enc: &mut metric::Encoder,
    ) -> Result<()> {
        let vmstat = self.parse_vmstat().unwrap_or_default();

        enc.write(
            &metrics.mem.swap_in,
            vmstat.pswpin * self.sysconf_page_size,
            None,
        );
        enc.write(
            &metrics.mem.swap_out,
            vmstat.pswpout * self.sysconf_page_size,
            None,
        );

        Ok(())
    }

    fn collect_fs(&self, metrics: &collector::Metrics, enc: &mut metric::Encoder) -> Result<()> {
        let mountinfos = self
            .parse_self_mountinfo()?
            .filter_map(|info| info.ok())
            .map(|info| {
                let iostats = self.parse_dev_block(&info.major_minor).unwrap_or_default();
                (info, iostats)
            })
            .collect::<Vec<_>>();

        let mut menc = enc.with_info(&metrics.fs.size, None);
        for (info, _) in mountinfos.iter() {
            menc.write(&[&info.mount_source, &info.mount_point], info.total);
        }

        menc = enc.with_info(&metrics.fs.available, None);
        for (info, _) in mountinfos.iter() {
            menc.write(&[&info.mount_source, &info.mount_point], info.avail);
        }

        menc = enc.with_info(&metrics.fs.read, None);
        for (info, iostats) in mountinfos.iter() {
            menc.write(&[&info.mount_source, &info.mount_point], iostats.read_bytes);
        }

        menc = enc.with_info(&metrics.fs.write, None);
        for (info, iostats) in mountinfos.iter() {
            menc.write(
                &[&info.mount_source, &info.mount_point],
                iostats.write_bytes,
            );
        }

        Ok(())
    }

    fn collect_thermal(
        &self,
        metrics: &collector::Metrics,
        enc: &mut metric::Encoder,
    ) -> Result<()> {
        let zones = self.parse_class_thermal()?;

        let mut menc = enc.with_info(&metrics.thermal.temperature, None);
        for zone in zones {
            let zone = zone?;

            menc.write(&[&zone.name], zone.temp as f64 / 1000.0);
        }

        Ok(())
    }

    fn collect_net_link_speed(
        &self,
        metrics: &collector::Metrics,
        enc: &mut metric::Encoder,
    ) -> Result<()> {
        let speeds = self.parse_ethtool()?;

        let mut menc = enc.with_info(&metrics.net.link_speed, None);
        for speed in speeds {
            let speed = speed?;

            menc.write(&[&speed.name], speed.speed as f64 * 1000.0 * 1000.0 / 8.0);
        }

        Ok(())
    }

    fn collect_net_link_state(
        &self,
        metrics: &collector::Metrics,
        enc: &mut metric::Encoder,
    ) -> Result<()> {
        let links = self
            .parse_links()?
            .filter_map(|link| link.ok())
            .collect::<Vec<_>>();

        let mut menc = enc.with_info(&metrics.net.link_up, None);
        for link in &links {
            menc.write(&[&link.name], link.admin_up as u8);
        }

        menc = enc.with_info(&metrics.net.link_operstate, None);
        for link in &links {
            menc.write(&[&link.name], link.operstate);
        }

        menc = enc.with_info(&metrics.net.link_rx, None);
        for link in &links {
            menc.write(&[&link.name], link.rx);
        }

        menc = enc.with_info(&metrics.net.link_tx, None);
        for link in &links {
            menc.write(&[&link.name], link.tx);
        }

        Ok(())
    }

    fn collect_net_route(
        &self,
        metrics: &collector::Metrics,
        enc: &mut metric::Encoder,
    ) -> Result<()> {
        let routes = self.parse_routes()?;

        let mut menc = enc.with_info(&metrics.net.route_default, None);
        for route in routes {
            let route = route?;

            menc.write(&[&route.ip().to_string()], 1);
        }

        Ok(())
    }

    fn collect_net_nft(
        &self,
        metrics: &collector::Metrics,
        enc: &mut metric::Encoder,
    ) -> Result<()> {
        let sets = self.parse_nfnetlink()?;

        let mut menc = enc.with_info(&metrics.net.nft_set_counter, None);
        for set in sets {
            let set = set?;
            let counters = self.parse_nft_set(&set)?;
            for counter in counters {
                let counter = counter?;

                menc.write(
                    &[
                        &set.family.to_string(),
                        &set.table,
                        &set.name,
                        &counter.addr,
                    ],
                    counter.bytes,
                );
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
