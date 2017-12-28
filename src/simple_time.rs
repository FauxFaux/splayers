use std::time;

use crates_time;

pub fn simple_time(dur: time::Duration) -> u64 {
    dur.as_secs()
        .checked_mul(1_000_000_000)
        .map_or(0, |nanos| nanos + u64::from(dur.subsec_nanos()))
}

pub fn simple_time_tm(val: crates_time::Tm) -> u64 {
    let timespec = val.to_timespec();
    simple_time(time::Duration::new(
        timespec.sec as u64,
        timespec.nsec as u32,
    ))
}

pub fn simple_time_epoch_seconds(seconds: u64) -> u64 {
    seconds.checked_mul(1_000_000_000).unwrap_or(0)
}

#[cfg(never)]
pub fn simple_time_ctime(val: &stat::Stat) -> u64 {
    if val.ctime <= 0 {
        0
    } else {
        (val.ctime as u64).checked_mul(1_000_000_000).unwrap_or(0) + (val.ctime_nano as u64)
    }
}
