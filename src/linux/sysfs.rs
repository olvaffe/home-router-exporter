// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use std::io::{Error, ErrorKind};
use std::path::Path;

pub struct SysThermalZone {
    pub zone: u64,
    pub name: String,
    pub temp: u64,
}

fn read_str<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    let mut s = std::fs::read_to_string(path)?;
    s.truncate(s.len() - 1);
    Ok(s)
}

fn read_u64<P: AsRef<Path>>(path: P) -> std::io::Result<u64> {
    let s = read_str(path)?;
    s.parse::<u64>()
        .map_err(|_| Error::new(ErrorKind::InvalidData, "bad"))
}

pub fn parse_class_thermal(sysfs: &Path) -> std::io::Result<Vec<SysThermalZone>> {
    let mut zones = Vec::new();

    let zone_entries = std::fs::read_dir("/sys/class/thermal")?;
    for zone_entry in zone_entries {
        let zone_entry = zone_entry?;

        let zone = zone_entry.file_name().to_str().map_or(-1, |name| {
            if !name.starts_with("thermal_zone") {
                return -1;
            }
            name[12..].parse::<i32>().unwrap_or(-1)
        });
        if zone == -1 {
            continue;
        }

        let type_path = zone_entry.path().join("type");
        let temp_path = zone_entry.path().join("temp");

        let name = read_str(type_path)?;
        let temp = read_u64(temp_path)?;

        zones.push(SysThermalZone {
            zone: zone as u64,
            name,
            temp,
        });
    }

    Ok(zones)
}

impl super::Linux {
    pub fn parse_class_thermal(&self) -> std::io::Result<Vec<SysThermalZone>> {
        parse_class_thermal(&self.sysfs_path)
    }
}
