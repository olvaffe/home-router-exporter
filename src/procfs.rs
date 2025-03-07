// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind};

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

pub fn parse_stat() -> std::io::Result<ProcStat> {
    let f = File::open("/proc/stat")?;
    let mut reader = BufReader::new(f);

    let mut line = String::new();
    reader.read_line(&mut line)?;

    let mut cols = line.split_whitespace();
    let cpu = cols
        .next()
        .ok_or(Error::new(ErrorKind::InvalidData, "bad"))?;
    let user_ticks = cols
        .next()
        .ok_or(Error::new(ErrorKind::InvalidData, "bad"))?;
    cols.next();
    let system_ticks = cols
        .next()
        .ok_or(Error::new(ErrorKind::InvalidData, "bad"))?;
    let idle_ticks = cols
        .next()
        .ok_or(Error::new(ErrorKind::InvalidData, "bad"))?;

    if cpu != "cpu" {
        return Err(Error::new(ErrorKind::InvalidData, "bad"));
    }

    let user_ticks = user_ticks
        .parse::<u64>()
        .map_err(|_| Error::new(ErrorKind::InvalidData, "bad"))?;
    let system_ticks = system_ticks
        .parse::<u64>()
        .map_err(|_| Error::new(ErrorKind::InvalidData, "bad"))?;
    let idle_ticks = idle_ticks
        .parse::<u64>()
        .map_err(|_| Error::new(ErrorKind::InvalidData, "bad"))?;

    let nrproc = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) } as u64;
    let clk_tck = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as u64;

    let stat = ProcStat {
        user_ms: user_ticks * 1000 / clk_tck / nrproc,
        system_ms: system_ticks * 1000 / clk_tck / nrproc,
        idle_ms: idle_ticks * 1000 / clk_tck / nrproc,
    };

    Ok(stat)
}

pub fn parse_meminfo() -> std::io::Result<ProcMemInfo> {
    let f = File::open("/proc/meminfo")?;
    let reader = BufReader::new(f);

    let mut info = ProcMemInfo {
        mem_total_kb: 0,
        mem_avail_kb: 0,
        swap_total_kb: 0,
        swap_free_kb: 0,
    };
    for line in reader.lines() {
        let line = line?;

        let get_u64 = |line: &str| {
            let col1 = line
                .split_whitespace()
                .nth(1)
                .ok_or(Error::new(ErrorKind::InvalidData, "bad"))?;
            col1.parse::<u64>()
                .map_err(|_| Error::new(ErrorKind::InvalidData, "bad"))
        };

        if line.starts_with("MemTotal:") {
            info.mem_total_kb = get_u64(&line)?;
        } else if line.starts_with("MemAvailable:") {
            info.mem_avail_kb = get_u64(&line)?;
        } else if line.starts_with("SwapTotal:") {
            info.swap_total_kb = get_u64(&line)?;
        } else if line.starts_with("SwapFree:") {
            info.swap_free_kb = get_u64(&line)?;
            break;
        }
    }

    Ok(info)
}

pub fn parse_diskstats() -> std::io::Result<Vec<ProcDiskStat>> {
    let mut stats = Vec::new();

    let f = File::open("/proc/diskstats")?;
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
