// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use anyhow::{Context, Result, anyhow};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub struct ProcStat {
    pub user_ms: u64,
    pub system_ms: u64,
    pub idle_ms: u64,
}

pub struct ProcMemInfo {
    pub mem_total_kb: u64,
    pub mem_avail_kb: u64,
    pub swap_total_kb: u64,
    pub swap_free_kb: u64,
}

pub struct ProcDiskStat {
    pub name: String,
    pub read_bytes: u64,
    pub write_bytes: u64,
}

pub struct ProcMountInfo {
    pub mount_source: String,
    pub mount_point: String,
    pub total: u64,
    pub free: u64,
    pub avail: u64,
}

pub fn parse_diskstats_line(line: &str) -> Result<ProcDiskStat> {
    // 0:major 1:minor 2:name
    // 3:r_completed 4:r_merged 5:r_sectors 6:r_time
    // 7:w_completed 8:w_merged 9:w_sectors 10:w_time
    // 11:io_count 12:io_time 13:io_weighted
    let cols: Vec<&str> = line.split_ascii_whitespace().collect();
    if cols.len() < 9 {
        return Err(anyhow!("failed to parse diskstats"));
    }
    let name = cols[2].to_string();
    let [read_bytes, write_bytes] = [cols[5], cols[9]].map(|col| {
        let sectors: u64 = col.parse().unwrap_or(0);
        sectors * 512
    });

    Ok(ProcDiskStat {
        name,
        read_bytes,
        write_bytes,
    })
}

pub fn parse_mountinfo_line(line: &str) -> Result<(&str, &str)> {
    // 0:id 1:parent_id 2:major:minor 3:root 4:mountpoint 5:options
    // optional fields... n:seperator
    // n+1:fs_type n+2:src n+3:super
    let cols: Vec<&str> = line.split_ascii_whitespace().collect();
    let sep_min = 6;
    let sep = cols[sep_min..]
        .iter()
        .position(|&col| col == "-")
        .map_or(0, |idx| sep_min + idx);
    if sep < sep_min || cols.len() < sep + 3 {
        return Err(anyhow!("failed to parse mountinfo"));
    }

    let dst = cols[4];
    let src = cols[sep + 2];

    Ok((src, dst))
}

pub fn parse_self_mountinfo(procfs: &Path) -> Result<Vec<ProcMountInfo>> {
    let mut infos: Vec<ProcMountInfo> = Vec::new();

    let f = File::open(procfs.join("self/mountinfo"))?;
    let reader = BufReader::new(f);

    for line in reader.lines() {
        let line = line?;
        let (src, dst) = parse_mountinfo_line(&line)?;
        if !src.starts_with("/") {
            continue;
        }

        let info = ProcMountInfo {
            mount_source: src.to_string(),
            mount_point: dst.to_string(),
            total: 0,
            free: 0,
            avail: 0,
        };
        infos.push(info);
    }

    for info in &mut infos {
        let [total, free, avail] = crate::libc::statvfs_size(&info.mount_point)?;
        info.total = total;
        info.free = free;
        info.avail = avail;
    }

    Ok(infos)
}

impl super::Linux {
    pub fn parse_stat(&self) -> Result<ProcStat> {
        let mut reader = self.procfs_open("stat")?;

        let mut line = String::new();
        reader.read_line(&mut line).context("failed to read stat")?;

        // 0:type 1:user 2:nice 3:system 4:idle 5:iowait
        let cols: Vec<&str> = line.split_ascii_whitespace().collect();
        if cols.len() < 5 || cols[0] != "cpu" {
            return Err(anyhow!("failed to parse stat"));
        }
        let [user_ms, system_ms, idle_ms] = [cols[1], cols[3], cols[4]].map(|col| {
            let ticks: u64 = col.parse().unwrap_or(0);
            ticks * 1000 / self.sysconf_user_hz / self.sysconf_nproc
        });

        Ok(ProcStat {
            user_ms,
            system_ms,
            idle_ms,
        })
    }

    pub fn parse_meminfo(&self) -> Result<ProcMemInfo> {
        let reader = self.procfs_open("meminfo")?;

        let mut mem_total_kb = 0;
        let mut mem_avail_kb = 0;
        let mut swap_total_kb = 0;
        let mut swap_free_kb = 0;
        for line in reader.lines() {
            let line = line.context("failed to read meminfo")?;

            // type: value [unit]
            let cols: Vec<&str> = line.split_ascii_whitespace().collect();
            if cols.len() < 2 {
                return Err(anyhow!("failed to parse meminfo"));
            }
            let ty = cols[0];
            let val: u64 = cols[1].parse().unwrap_or(0);

            match ty {
                "MemTotal:" => mem_total_kb = val,
                "MemAvailable:" => mem_avail_kb = val,
                "SwapTotal:" => swap_total_kb = val,
                "SwapFree:" => {
                    swap_free_kb = val;
                    // we've got them all
                    break;
                }
                _ => (),
            }
        }

        Ok(ProcMemInfo {
            mem_total_kb,
            mem_avail_kb,
            swap_total_kb,
            swap_free_kb,
        })
    }

    pub fn parse_diskstats(&self) -> Result<Vec<ProcDiskStat>> {
        let reader = self.procfs_open("diskstats")?;

        let mut stats = Vec::new();
        for line in reader.lines() {
            let stat = parse_diskstats_line(&line?)?;
            if stat.read_bytes != 0 || stat.write_bytes != 0 {
                stats.push(stat)
            }
        }

        Ok(stats)
    }

    pub fn parse_self_mountinfo(&self) -> Result<Vec<ProcMountInfo>> {
        parse_self_mountinfo(&self.procfs_path)
    }
}
