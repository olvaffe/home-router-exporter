// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use anyhow::Result;
use std::fs;

pub struct SysThermalZone {
    pub id: u64,
    pub name: String,
    pub temp: u64,
}

fn parse_class_thermal_device(dir: fs::DirEntry, id: u64) -> Result<SysThermalZone> {
    let dir_path = dir.path();
    let type_path = dir_path.join("type");
    let temp_path = dir_path.join("temp");

    let name = super::read_string(type_path)?;
    let temp = super::read_u64(temp_path)?;

    Ok(SysThermalZone { id, name, temp })
}

impl super::Linux {
    pub fn parse_class_thermal(&self) -> Result<Vec<SysThermalZone>> {
        let dirs = self.sysfs_read_dir("class/thermal")?;

        let mut zones = Vec::new();
        for dir in dirs {
            let dir = dir?;

            if let Some(name) = dir.file_name().to_str() {
                if name.starts_with("thermal_zone") {
                    let id = name[12..].parse()?;
                    let zone = parse_class_thermal_device(dir, id)?;
                    zones.push(zone);
                }
            }
        }

        Ok(zones)
    }
}
