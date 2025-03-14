// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use anyhow::{Context, Result};
use std::{ffi, io, mem, path};

pub fn sysconf_user_hz() -> u64 {
    // SAFETY: valid sysconf call with validation
    let mut user_hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if user_hz <= 0 {
        user_hz = 100;
    }

    user_hz as _
}

pub fn statvfs_size(path: impl AsRef<path::Path>) -> Result<[u64; 3]> {
    let c_path = ffi::CString::new(path.as_ref().as_os_str().as_encoded_bytes())?;
    let mut stat = mem::MaybeUninit::<libc::statvfs>::uninit();

    // SAFETY: both pointers are valid
    let ret = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };
    if ret != 0 {
        return Err(io::Error::last_os_error())
            .context(format!("failed to statvfs {:?}", path.as_ref()));
    }
    // SAFETY: ret is 0
    let stat = unsafe { stat.assume_init() };

    let size = [stat.f_blocks, stat.f_bfree, stat.f_bavail].map(|blocks| blocks * stat.f_frsize);
    Ok(size)
}
