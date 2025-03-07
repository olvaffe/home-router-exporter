// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use libc;
use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind};

pub struct ProcStat {
    pub user_ms: u64,
    pub system_ms: u64,
    pub idle_ms: u64,
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

    let clk_tck = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as u64;

    let stat = ProcStat {
        user_ms: user_ticks * 1000 / clk_tck,
        system_ms: system_ticks * 1000 / clk_tck,
        idle_ms: idle_ticks * 1000 / clk_tck,
    };

    Ok(stat)
}
