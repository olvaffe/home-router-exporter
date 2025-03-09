// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

pub fn sysconf_nproc() -> u64 {
    // SAFETY: valid sysconf call with validation
    let mut nproc = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) };
    if nproc <= 0 {
        nproc = 1;
    }

    nproc as _
}

pub fn sysconf_user_hz() -> u64 {
    // SAFETY: valid sysconf call with validation
    let mut user_hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if user_hz <= 0 {
        user_hz = 100;
    }

    user_hz as _
}
