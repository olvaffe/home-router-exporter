// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use std::ffi::CString;
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

pub struct ProcMountInfo {
    pub mount_source: String,
    pub mount_point: String,
    pub total: u64,
    pub free: u64,
    pub avail: u64,
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

    // SAFETY:
    let nrproc = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) } as u64;
    // SAFETY:
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

pub fn parse_self_mountinfo() -> std::io::Result<Vec<ProcMountInfo>> {
    let mut infos: Vec<ProcMountInfo> = Vec::new();

    let f = File::open("/proc/self/mountinfo")?;
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
