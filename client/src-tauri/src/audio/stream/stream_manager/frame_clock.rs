// High-precision 20 ms cadence clock used to pace fake frames like a real
// audio device. On Windows it raises the system timer resolution and busy-waits
// against QueryPerformanceCounter; elsewhere it falls back to a monotonic
// Instant spin/sleep loop. target_time = start + n * interval keeps the cadence
// drift-free rather than accumulating per-frame sleep error.
#[cfg(target_os = "windows")]
pub(crate) struct FrameClock {
    frequency: i64,
    start: i64,
    interval_ticks: i64,
    tick: i64,
}

#[cfg(target_os = "windows")]
impl FrameClock {
    pub(crate) fn new(interval_ms: f64) -> Self {
        windows_targets::link!("winmm.dll" "system" fn timeBeginPeriod(uperiod: u32) -> u32);
        windows_targets::link!("kernel32.dll" "system" fn QueryPerformanceCounter(lpperformancecount: *mut i64) -> i32);
        windows_targets::link!("kernel32.dll" "system" fn QueryPerformanceFrequency(lpfrequency: *mut i64) -> i32);

        unsafe {
            timeBeginPeriod(1);
        }

        let mut frequency = 0i64;
        let mut start = 0i64;
        unsafe {
            QueryPerformanceFrequency(&mut frequency);
            QueryPerformanceCounter(&mut start);
        }

        let interval_ticks = (frequency as f64 * interval_ms / 1000.0) as i64;

        Self {
            frequency,
            start,
            interval_ticks,
            tick: 0,
        }
    }

    // Block until the deadline for the next slot, then advance the clock.
    pub(crate) fn wait_next(&mut self) {
        windows_targets::link!("kernel32.dll" "system" fn QueryPerformanceCounter(lpperformancecount: *mut i64) -> i32);

        self.tick += 1;
        let target_time = self.start + (self.tick * self.interval_ticks);

        loop {
            let mut current_time = 0i64;
            unsafe {
                QueryPerformanceCounter(&mut current_time);
            }

            // Re-anchor after an idle gap. If the source blocked waiting for the
            // next chunk, the fixed epoch leaves target_time many intervals in
            // the past; replaying those missed slots would emit a burst with no
            // cadence and overrun the bounded QUIC datagram queue. Resync the
            // epoch to now and emit this frame immediately instead. A burst
            // within a continuous feed never falls this far behind.
            if current_time - target_time > 2 * self.interval_ticks {
                self.start = current_time;
                self.tick = 0;
                return;
            }

            if current_time >= target_time {
                break;
            }

            let remaining_ticks = target_time - current_time;
            let remaining_ms = remaining_ticks as f64 * 1000.0 / self.frequency as f64;

            if remaining_ms > 2.0 {
                std::thread::sleep(std::time::Duration::from_millis((remaining_ms - 1.0) as u64));
            } else {
                std::thread::yield_now();
            }
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for FrameClock {
    fn drop(&mut self) {
        windows_targets::link!("winmm.dll" "system" fn timeEndPeriod(uperiod: u32) -> u32);

        unsafe {
            timeEndPeriod(1);
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) struct FrameClock {
    start: std::time::Instant,
    interval: std::time::Duration,
    tick: u32,
}

#[cfg(not(target_os = "windows"))]
impl FrameClock {
    pub(crate) fn new(interval_ms: f64) -> Self {
        Self {
            start: std::time::Instant::now(),
            interval: std::time::Duration::from_micros((interval_ms * 1000.0) as u64),
            tick: 0,
        }
    }

    // Block until the deadline for the next slot, then advance the clock.
    pub(crate) fn wait_next(&mut self) {
        self.tick += 1;
        let target = self.start + self.interval * self.tick;

        loop {
            let now = std::time::Instant::now();

            // Re-anchor after an idle gap (see the Windows impl): replaying
            // missed slots as a burst would overrun the bounded QUIC datagram
            // queue, so resync the epoch to now and emit this frame immediately.
            if now > target + self.interval * 2 {
                self.start = now;
                self.tick = 0;
                return;
            }

            if now >= target {
                break;
            }

            let remaining = target - now;
            if remaining > std::time::Duration::from_millis(2) {
                std::thread::sleep(remaining - std::time::Duration::from_millis(1));
            } else {
                std::thread::yield_now();
            }
        }
    }
}
