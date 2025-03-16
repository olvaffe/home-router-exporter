// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use anyhow::{Context, Result, anyhow};
use std::{
    fs,
    io::{self, BufRead},
};

#[derive(Default)]
pub(super) struct MemInfo {
    pub mem_total_kb: u64,
    pub mem_avail_kb: u64,
    pub swap_total_kb: u64,
    pub swap_free_kb: u64,
}

pub(super) struct Stat {
    pub cpu: String,
    pub idle_ticks: u64,
}

pub(super) struct PidMountInfo {
    pub major_minor: String,
    pub mount_source: String,
    pub mount_point: String,
    pub total: u64,
    pub avail: u64,
}

fn parse_stat_line(line: &str) -> Result<Stat> {
    // 0:cpu 1:user 2:nice 3:system 4:idle 5:iowait
    let cols: Vec<&str> = line.split_ascii_whitespace().collect();
    if cols.len() < 5 {
        return Err(anyhow!("failed to parse stat"));
    }

    let cpu = cols[0].to_string();
    let [_user_ticks, _system_ticks, idle_ticks] =
        [cols[1], cols[3], cols[4]].map(|col| col.parse().unwrap_or(0));

    Ok(Stat { cpu, idle_ticks })
}

pub(super) struct StatIter {
    reader: io::BufReader<fs::File>,
}

impl Iterator for StatIter {
    type Item = Result<Stat>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => return None,
                Ok(_) => (),
                Err(err) => return Some(Err(err).context("failed to read stat")),
            };

            match line.strip_prefix("cpu") {
                Some(line) => {
                    if line.starts_with(" ") {
                        continue;
                    }
                }
                None => return None,
            };

            return Some(parse_stat_line(&line));
        }
    }
}

fn parse_pid_mountinfo_line(line: &str) -> Result<(&str, &str, &str)> {
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

    let major_minor = cols[2];
    let dst = cols[4];
    let src = cols[sep + 2];

    Ok((major_minor, src, dst))
}

pub(super) struct PidMountInfoIter {
    reader: io::BufReader<fs::File>,
}

impl Iterator for PidMountInfoIter {
    type Item = Result<PidMountInfo>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => return None,
                Ok(_) => (),
                Err(err) => return Some(Err(err).context("failed to read mountinfo")),
            };

            let res = parse_pid_mountinfo_line(&line);
            if let Ok((_, src, _)) = res {
                if !src.starts_with("/") {
                    continue;
                }
            }

            let info = res.and_then(|(major_minor, src, dst)| {
                let [total, _free, avail] = crate::libc::statvfs_size(dst)?;

                Ok(PidMountInfo {
                    major_minor: major_minor.to_string(),
                    mount_source: src.to_string(),
                    mount_point: dst.to_string(),
                    total,
                    avail,
                })
            });

            return Some(info);
        }
    }
}

impl super::Linux {
    pub(super) fn parse_meminfo(&self) -> Result<MemInfo> {
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

        Ok(MemInfo {
            mem_total_kb,
            mem_avail_kb,
            swap_total_kb,
            swap_free_kb,
        })
    }

    pub(super) fn parse_stat(&self) -> Result<StatIter> {
        let reader = self.procfs_open("stat")?;
        Ok(StatIter { reader })
    }

    pub(super) fn parse_self_mountinfo(&self) -> Result<PidMountInfoIter> {
        let reader = self.procfs_open("self/mountinfo")?;
        Ok(PidMountInfoIter { reader })
    }
}
