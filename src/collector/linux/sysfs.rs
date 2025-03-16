// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use anyhow::{Context, Result, anyhow};
use std::{fs, io::BufRead};

#[derive(Default)]
pub(super) struct IoStats {
    pub read_bytes: u64,
    pub write_bytes: u64,
}

pub(super) struct ThermalZone {
    pub name: String,
    pub temp: u64,
}

fn parse_io_stats_line(line: &str) -> Result<IoStats> {
    // 0:r_completed 1:r_merged 2:r_sectors 3:r_time
    // 4:w_completed 5:w_merged 6:w_sectors 7:w_time
    // 8:io_count 9:io_time 10:io_weighted
    // 11:d_completed 12:d_merged 13:d_sectors 14:d_time
    // 15:f_completed 16:f_time
    let cols: Vec<&str> = line.split_ascii_whitespace().collect();
    if cols.len() < 7 {
        return Err(anyhow!("failed to parse iostats"));
    }
    let [read_bytes, write_bytes] = [cols[2], cols[6]].map(|col| {
        let sectors: u64 = col.parse().unwrap_or(0);
        sectors * 512
    });

    Ok(IoStats {
        read_bytes,
        write_bytes,
    })
}

fn parse_thermal_zone_device(dir: fs::DirEntry, _id: &str) -> Result<ThermalZone> {
    let dir_path = dir.path();
    let type_path = dir_path.join("type");
    let temp_path = dir_path.join("temp");

    let name = super::read_string(type_path)?;
    let temp = super::read_u64(temp_path)?;

    Ok(ThermalZone { name, temp })
}

pub(super) struct ClassThermalIter {
    dir_iter: fs::ReadDir,
}

impl Iterator for ClassThermalIter {
    type Item = Result<ThermalZone>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let dir = match self.dir_iter.next() {
                Some(Ok(dir)) => dir,
                Some(Err(err)) => return Some(Err(err).context("failed to read class/thermal")),
                None => return None,
            };

            if let Some(id) = dir
                .file_name()
                .to_str()
                .and_then(|name| name.strip_prefix("thermal_zone"))
            {
                return Some(parse_thermal_zone_device(dir, id));
            }
        }
    }
}

impl super::Linux {
    pub(super) fn parse_class_thermal(&self) -> Result<ClassThermalIter> {
        let dir_iter = self.sysfs_read_dir("class/thermal")?;
        Ok(ClassThermalIter { dir_iter })
    }

    pub(super) fn parse_dev_block(&self, dev: &str) -> Result<IoStats> {
        let mut reader = self.sysfs_open(&format!("dev/block/{dev}/stat"))?;

        let mut line = String::new();
        reader
            .read_line(&mut line)
            .context("failed to read iostats")?;

        parse_io_stats_line(&line)
    }
}
