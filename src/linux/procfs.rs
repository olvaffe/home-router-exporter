// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use anyhow::{Context, Result, anyhow};
use std::ffi::CString;
use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind};
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

pub fn parse_diskstats(procfs: &Path) -> std::io::Result<Vec<ProcDiskStat>> {
    let mut stats = Vec::new();

    let f = File::open(procfs.join("diskstats"))?;
    let reader = BufReader::new(f);

    for line in reader.lines() {
        let line = line?;
        let cols = line.split_whitespace();

        let mut cols = cols.skip(2);
        let col2 = cols
            .next()
            .ok_or(Error::new(ErrorKind::InvalidData, "bad"))?;
        let mut cols = cols.skip(2);
        let col5 = cols
            .next()
            .ok_or(Error::new(ErrorKind::InvalidData, "bad"))?;
        let mut cols = cols.skip(3);
        let col9 = cols
            .next()
            .ok_or(Error::new(ErrorKind::InvalidData, "bad"))?;

        let name = col2.to_string();
        let read_secs = col5
            .parse::<u64>()
            .map_err(|_| Error::new(ErrorKind::InvalidData, "bad"))?;
        let write_secs = col9
            .parse::<u64>()
            .map_err(|_| Error::new(ErrorKind::InvalidData, "bad"))?;

        if read_secs == 0 && write_secs == 0 {
            continue;
        }

        stats.push(ProcDiskStat {
            name: name.to_string(),
            read_bytes: read_secs * 512,
            write_bytes: write_secs * 512,
        })
    }

    Ok(stats)
}

pub fn parse_self_mountinfo(procfs: &Path) -> std::io::Result<Vec<ProcMountInfo>> {
    let mut infos: Vec<ProcMountInfo> = Vec::new();

    let f = File::open(procfs.join("self/mountinfo"))?;
    let reader = BufReader::new(f);

    for line in reader.lines() {
        let line = line?;
        let cols = line.split_whitespace();

        let mut cols = cols.skip(4);
        let col4 = cols
            .next()
            .ok_or(Error::new(ErrorKind::InvalidData, "bad"))?;

        let mut cols = cols.skip(1);
        while let Some(col) = cols.next() {
            if col == "-" {
                break;
            }
        }

        let src = cols
            .nth(1)
            .ok_or(Error::new(ErrorKind::InvalidData, "bad"))?;
        if !src.starts_with("/") {
            continue;
        }

        let mut skip = false;
        for info in &mut infos {
            if info.mount_source == src {
                skip = true;
                if info.mount_point.starts_with(col4) {
                    info.mount_point = col4.to_string();
                }
                break;
            }
        }
        if skip {
            continue;
        }

        let info = ProcMountInfo {
            mount_source: src.to_string(),
            mount_point: col4.to_string(),
            total: 0,
            free: 0,
            avail: 0,
        };

        infos.push(info);
    }

    for info in &mut infos {
        let path = CString::new(&*info.mount_point).unwrap();

        let mut stat = std::mem::MaybeUninit::<libc::statfs64>::uninit();
        // SAFETY:
        let ret = unsafe { libc::statfs64(path.as_ptr(), stat.as_mut_ptr()) };
        if ret != 0 {
            println!("nonono");
            return Err(Error::new(ErrorKind::InvalidData, "bad"));
        }
        // SAFETY:
        let stat = unsafe { stat.assume_init() };

        info.total = stat.f_blocks * stat.f_bsize as u64;
        info.free = stat.f_bfree * stat.f_bsize as u64;
        info.avail = stat.f_bavail * stat.f_bsize as u64;
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

    pub fn parse_diskstats(&self) -> std::io::Result<Vec<ProcDiskStat>> {
        parse_diskstats(&self.procfs_path)
    }

    pub fn parse_self_mountinfo(&self) -> std::io::Result<Vec<ProcMountInfo>> {
        parse_self_mountinfo(&self.procfs_path)
    }
}
