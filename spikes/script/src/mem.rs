//! Process memory on Windows, read from the OS (the spike is the only place that needs `unsafe`).

#[repr(C)]
#[allow(non_snake_case)]
struct ProcessMemoryCounters {
    cb: u32,
    PageFaultCount: u32,
    PeakWorkingSetSize: usize,
    WorkingSetSize: usize,
    QuotaPeakPagedPoolUsage: usize,
    QuotaPagedPoolUsage: usize,
    QuotaPeakNonPagedPoolUsage: usize,
    QuotaNonPagedPoolUsage: usize,
    PagefileUsage: usize,
    PeakPagefileUsage: usize,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> isize;
    fn K32GetProcessMemoryInfo(p: isize, c: *mut ProcessMemoryCounters, cb: u32) -> i32;
}

/// (working set, peak working set, private bytes) in bytes.
pub fn now() -> (usize, usize, usize) {
    unsafe {
        let mut c: ProcessMemoryCounters = std::mem::zeroed();
        c.cb = size_of::<ProcessMemoryCounters>() as u32;
        K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb);
        (c.WorkingSetSize, c.PeakWorkingSetSize, c.PagefileUsage)
    }
}
